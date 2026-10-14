//! One receive owner, one wire request, and no runtime or delivery queues.
//!
//! Adapters drain framed input through [`Session::receive`], handle borrowed
//! effects synchronously, and retain this session independently of waiting futures.
#![forbid(unsafe_code)]

use thiserror::Error;
use umbriel_protocol::{
    codec::EncodeError,
    envelope::{EventName, Family, Message},
    request::Request,
};

/// Connection-local admission identity. Not a wire ID, cursor, or global ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalRequestId(u64);

impl LocalRequestId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    OneShot,
    Subscription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No request has been sent on a one-shot connection.
    Fresh,
    Subscribing,
    Streaming,
    /// The one-shot reply drained; only EOF is now expected.
    AwaitingEof,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waiter {
    Waiting,
    Stopped(StopReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopDisposition {
    /// No write was attempted or acknowledged; the request no longer owns a reply.
    Withdrawn,
    /// The wire slot remains occupied until a reply, acceptance, or close.
    DrainRequired,
}

/// Facts for a request that did not reach a native reply/acceptance boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnresolvedRequest {
    pub id: LocalRequestId,
    pub waiter: Waiter,
    /// False proves local non-transmission. True does not prove execution.
    pub possibly_sent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolFailure {
    Decode(String),
    FrameTooLong { limit: usize },
    Unexpected(Unexpected),
    LocalPositionExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unexpected {
    Blank,
    Event,
    Reply,
    ReplyBeforeWriteComplete,
    SubscriptionOk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    Eof,
    LocalClose,
    TransportLost(String),
    Protocol(ProtocolFailure),
    SubscriptionRejected(String),
    /// The server's oversize branch may leave its input buffer poisoned.
    RequestBufferPoisoned,
}

/// First terminal reason is retained. No implicit rollback or retry follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminal {
    pub reason: EndReason,
    pub unresolved: Option<UnresolvedRequest>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error("connection is terminal")]
    Terminal,
    #[error("the wire request slot is occupied")]
    Busy,
    #[error("ordinary requests are not supported in this phase")]
    NotReady,
    #[error("subscribe is only supported by the fresh subscription constructor")]
    UnsupportedSubscribe,
    #[error("there is no outbound write available")]
    NoWrite,
    #[error("a write attempt is already in flight")]
    WriteInFlight,
    #[error("no write attempt is in flight")]
    NoWriteInFlight,
    #[error("write progress {reported} exceeds the offered {remaining} bytes")]
    InvalidProgress { reported: usize, remaining: usize },
    #[error("request identity is not the current wire owner")]
    WrongRequest,
    #[error("the caller has already stopped waiting")]
    AlreadyStopped,
    #[error("connection-local request identities exhausted")]
    LocalRequestIdsExhausted,
    #[error("protocol failure: {0:?}")]
    Protocol(ProtocolFailure),
}

/// Borrowed effects never enter a core-owned queue. Payload decode belongs to
/// the caller: a typed decode error after `Ok` does not undo acknowledgement.
#[derive(Debug)]
pub enum Effect<'a> {
    Reply {
        id: LocalRequestId,
        waiter: Waiter,
        /// Exactly `Message::Ok` or `Message::Err`; events never complete a command.
        message: Message<'a>,
        local_receive_position: u64,
    },
    Event {
        message: Message<'a>,
        local_receive_position: u64,
        /// First matching event on a fresh connection is acceptance evidence.
        accepted_subscription: Option<(LocalRequestId, Waiter)>,
    },
    /// Only the exactly blank initial keyboard-only response justifies this.
    SubscriptionAcceptedEmpty {
        id: LocalRequestId,
        waiter: Waiter,
        local_receive_position: u64,
    },
    SubscriptionRejected {
        id: LocalRequestId,
        waiter: Waiter,
        message: Message<'a>,
        local_receive_position: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyStatus {
    NotDeclared,
    Unseen,
    /// Historical observation evidence, not current connection health.
    Observed,
}

#[derive(Debug)]
struct Slot {
    id: LocalRequestId,
    waiter: Waiter,
    written: usize,
    in_flight: bool,
}

/// A single connection's bounded ledger. No socket, clock, futures, reconnect,
/// materialized state, or per-observer delivery machinery is owned here.
#[derive(Debug)]
pub struct Session {
    role: Role,
    phase: Phase,
    declared: [bool; 6],
    observed: [bool; 6],
    outbound: Vec<u8>,
    slot: Option<Slot>,
    last_request_id: u64,
    local_receive_position: u64,
    frame_limit: usize,
    terminal: Option<Terminal>,
}

impl Session {
    /// The inbound frame limit excludes LF and is independent of request bounds.
    pub fn one_shot(frame_limit: usize) -> Self {
        Self {
            role: Role::OneShot,
            phase: Phase::Fresh,
            declared: [false; 6],
            observed: [false; 6],
            outbound: Vec::new(),
            slot: None,
            last_request_id: 0,
            local_receive_position: 0,
            frame_limit,
            terminal: None,
        }
    }

    /// Encode the sole initial subscribe. Duplicates fold in the declared set.
    /// No subsequent subscribe/unsubscribe requests are supported.
    pub fn subscription(families: &[Family], frame_limit: usize) -> Result<Self, Error> {
        let mut session = Self::one_shot(frame_limit);
        Request::Subscribe(families).encode(&mut session.outbound)?;
        for &family in families {
            session.declared[family_index(family)] = true;
        }
        session.role = Role::Subscription;
        session.phase = Phase::Subscribing;
        session.install_slot()?;
        Ok(session)
    }

    pub const fn role(&self) -> Role {
        self.role
    }
    pub const fn phase(&self) -> Phase {
        self.phase
    }
    pub fn terminal(&self) -> Option<&Terminal> {
        self.terminal.as_ref()
    }
    pub fn pending_request(&self) -> Option<LocalRequestId> {
        self.slot.as_ref().map(|slot| slot.id)
    }

    pub fn family_status(&self, family: Family) -> FamilyStatus {
        let index = family_index(family);
        if !self.declared[index] {
            FamilyStatus::NotDeclared
        } else if self.observed[index] {
            FamilyStatus::Observed
        } else {
            FamilyStatus::Unseen
        }
    }

    /// Admission succeeds only after the previous reply has actually drained.
    /// Raw `subscribe` is rejected too; it cannot bypass the fixed-set contract.
    pub fn admit(&mut self, request: Request<'_>) -> Result<LocalRequestId, Error> {
        self.ensure_live()?;
        if self.slot.is_some() {
            return Err(Error::Busy);
        }
        if !matches!(self.phase, Phase::Fresh | Phase::Streaming) {
            return Err(Error::NotReady);
        }
        if matches!(
            request,
            Request::Subscribe(_)
                | Request::Raw {
                    cmd: "subscribe",
                    ..
                }
        ) {
            return Err(Error::UnsupportedSubscribe);
        }
        self.last_request_id
            .checked_add(1)
            .ok_or(Error::LocalRequestIdsExhausted)?;
        request.encode(&mut self.outbound)?;
        self.install_slot()
    }

    /// Inspect remaining bytes without claiming that I/O has begun.
    pub fn outbound(&self) -> &[u8] {
        match &self.slot {
            Some(slot) if self.terminal.is_none() => &self.outbound[slot.written..],
            _ => &[],
        }
    }

    /// Call immediately before handing these bytes to a write operation.
    /// An unresolved attempt counts as possibly sent, even with no acknowledged
    /// progress. Report its result before routing input or starting another write.
    pub fn begin_write(&mut self) -> Result<&[u8], Error> {
        self.ensure_live()?;
        let slot = self.slot.as_mut().ok_or(Error::NoWrite)?;
        if slot.in_flight {
            return Err(Error::WriteInFlight);
        }
        if slot.written == self.outbound.len() {
            return Err(Error::NoWrite);
        }
        slot.in_flight = true;
        Ok(&self.outbound[slot.written..])
    }

    /// Acknowledge the completed attempt. Zero is valid and makes no progress;
    /// adapters decide whether it means WouldBlock or a fatal write-zero.
    /// Invalid progress leaves the in-flight obligation unchanged.
    pub fn write_progress(&mut self, bytes: usize) -> Result<(), Error> {
        self.ensure_live()?;
        let slot = self.slot.as_mut().ok_or(Error::NoWriteInFlight)?;
        if !slot.in_flight {
            return Err(Error::NoWriteInFlight);
        }
        let remaining = self.outbound.len() - slot.written;
        if bytes > remaining {
            return Err(Error::InvalidProgress {
                reported: bytes,
                remaining,
            });
        }
        slot.written += bytes;
        slot.in_flight = false;
        if slot.written == 0 && matches!(slot.waiter, Waiter::Stopped(_)) {
            self.withdraw_unsent();
        }
        Ok(())
    }

    /// Stops caller delivery, never execution. Partial requests must still finish
    /// writing (or close); suppressing their suffix could strand the server parser.
    pub fn stop_waiting(
        &mut self,
        id: LocalRequestId,
        reason: StopReason,
    ) -> Result<StopDisposition, Error> {
        self.ensure_live()?;
        let slot = self
            .slot
            .as_mut()
            .filter(|slot| slot.id == id)
            .ok_or(Error::WrongRequest)?;
        if slot.waiter != Waiter::Waiting {
            return Err(Error::AlreadyStopped);
        }
        slot.waiter = Waiter::Stopped(reason);
        if slot.written == 0 && !slot.in_flight {
            self.withdraw_unsent();
            Ok(StopDisposition::Withdrawn)
        } else {
            Ok(StopDisposition::DrainRequired)
        }
    }

    /// Route exactly one complete frame, excluding LF. No payload is retained.
    /// Invalid remote input terminates the connection; API misuse does not.
    pub fn receive<'a>(&mut self, frame: &'a [u8]) -> Result<Effect<'a>, Error> {
        self.ensure_live()?;
        if frame.len() > self.frame_limit {
            return self.fail(ProtocolFailure::FrameTooLong {
                limit: self.frame_limit,
            });
        }
        let Some(position) = self.local_receive_position.checked_add(1) else {
            return self.fail(ProtocolFailure::LocalPositionExhausted);
        };
        self.local_receive_position = position;
        let message = match Message::decode(frame) {
            Ok(message) => message,
            Err(error) => return self.fail(ProtocolFailure::Decode(error.to_string())),
        };
        match message {
            Message::Event { ref name, .. } => {
                let accepted_subscription = if self.phase == Phase::Subscribing {
                    if !matches!(name, EventName::Known(family) if self.declared[family_index(*family)])
                    {
                        return self.fail(ProtocolFailure::Unexpected(Unexpected::Event));
                    }
                    self.require_written()?;
                    let slot = self.retire_slot();
                    self.phase = Phase::Streaming;
                    Some((slot.id, slot.waiter))
                } else if self.phase == Phase::Streaming {
                    None
                } else {
                    return self.fail(ProtocolFailure::Unexpected(Unexpected::Event));
                };
                if let EventName::Known(family) = name {
                    let index = family_index(*family);
                    if self.declared[index] {
                        self.observed[index] = true;
                    }
                }
                Ok(Effect::Event {
                    message,
                    local_receive_position: position,
                    accepted_subscription,
                })
            }
            Message::Blank => {
                if self.phase != Phase::Subscribing
                    || self.declared != [false, false, true, false, false, false]
                {
                    return self.fail(ProtocolFailure::Unexpected(Unexpected::Blank));
                }
                self.require_written()?;
                let slot = self.retire_slot();
                self.phase = Phase::Streaming;
                Ok(Effect::SubscriptionAcceptedEmpty {
                    id: slot.id,
                    waiter: slot.waiter,
                    local_receive_position: position,
                })
            }
            Message::Ok(_) if self.phase == Phase::Subscribing => {
                self.fail(ProtocolFailure::Unexpected(Unexpected::SubscriptionOk))
            }
            Message::Ok(_) | Message::Err(_) => {
                self.require_written()?;
                let slot = self.retire_slot();
                if self.phase == Phase::Subscribing {
                    let Message::Err(ref text) = message else {
                        unreachable!()
                    };
                    self.end(EndReason::SubscriptionRejected(text.to_string()));
                    Ok(Effect::SubscriptionRejected {
                        id: slot.id,
                        waiter: slot.waiter,
                        message,
                        local_receive_position: position,
                    })
                } else {
                    if self.role == Role::OneShot {
                        self.phase = Phase::AwaitingEof;
                    }
                    if matches!(&message, Message::Err(text) if text == "request too long") {
                        self.end(EndReason::RequestBufferPoisoned);
                    }
                    Ok(Effect::Reply {
                        id: slot.id,
                        waiter: slot.waiter,
                        message,
                        local_receive_position: position,
                    })
                }
            }
        }
    }

    /// Drain all complete frames in the adapter's framer before reporting EOF.
    /// A truncated tail is a protocol failure, not a successful EOF reply.
    pub fn eof(&mut self) -> &Terminal {
        self.end(EndReason::Eof)
    }
    pub fn close(&mut self) -> &Terminal {
        self.end(EndReason::LocalClose)
    }

    /// Use when a write/read fails, including a dropped unresolved write future.
    pub fn transport_lost(&mut self, detail: impl Into<String>) -> &Terminal {
        if self.terminal.is_none() {
            self.end(EndReason::TransportLost(detail.into()));
        }
        self.terminal.as_ref().expect("terminal installed")
    }

    /// Report adapter framing failure without manufacturing a native rejection.
    pub fn protocol_failed(&mut self, failure: ProtocolFailure) -> &Terminal {
        self.end(EndReason::Protocol(failure))
    }

    fn ensure_live(&self) -> Result<(), Error> {
        if self.terminal.is_some() {
            Err(Error::Terminal)
        } else {
            Ok(())
        }
    }

    fn install_slot(&mut self) -> Result<LocalRequestId, Error> {
        let next = self
            .last_request_id
            .checked_add(1)
            .ok_or(Error::LocalRequestIdsExhausted)?;
        self.last_request_id = next;
        let id = LocalRequestId(next);
        self.slot = Some(Slot {
            id,
            waiter: Waiter::Waiting,
            written: 0,
            in_flight: false,
        });
        Ok(id)
    }

    fn require_written(&mut self) -> Result<(), Error> {
        match &self.slot {
            None => self.fail(ProtocolFailure::Unexpected(Unexpected::Reply)),
            Some(slot) if slot.in_flight || slot.written != self.outbound.len() => self.fail(
                ProtocolFailure::Unexpected(Unexpected::ReplyBeforeWriteComplete),
            ),
            Some(_) => Ok(()),
        }
    }

    fn retire_slot(&mut self) -> Slot {
        self.outbound.clear();
        self.slot.take().expect("validated slot")
    }

    fn withdraw_unsent(&mut self) {
        if self.phase == Phase::Subscribing {
            // There is no active subscription, nor an ordinary command lane.
            self.end(EndReason::LocalClose);
        } else {
            self.retire_slot();
        }
    }

    fn fail<T>(&mut self, failure: ProtocolFailure) -> Result<T, Error> {
        self.end(EndReason::Protocol(failure.clone()));
        Err(Error::Protocol(failure))
    }

    fn end(&mut self, reason: EndReason) -> &Terminal {
        if self.terminal.is_none() {
            let unresolved = self.slot.take().map(|slot| UnresolvedRequest {
                id: slot.id,
                waiter: slot.waiter,
                possibly_sent: slot.written > 0 || slot.in_flight,
            });
            self.outbound.clear();
            self.phase = Phase::Terminal;
            self.terminal = Some(Terminal { reason, unresolved });
        }
        self.terminal.as_ref().expect("terminal installed")
    }
}

const fn family_index(family: Family) -> usize {
    match family {
        Family::Theme => 0,
        Family::Overview => 1,
        Family::KeyboardLayout => 2,
        Family::Windows => 3,
        Family::Workspaces => 4,
        Family::Submap => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_request_ids_never_wrap_or_mutate_failed_admission() {
        let mut session = Session::one_shot(1024);
        session.last_request_id = u64::MAX;
        assert!(matches!(
            session.admit(Request::Msg("noop")),
            Err(Error::LocalRequestIdsExhausted)
        ));
        assert!(session.slot.is_none());
        assert!(session.outbound.is_empty());
        assert_eq!(session.phase, Phase::Fresh);
    }

    #[test]
    fn local_receive_positions_never_wrap() {
        let mut session = Session::one_shot(1024);
        session.admit(Request::Msg("noop")).unwrap();
        let count = session.begin_write().unwrap().len();
        session.write_progress(count).unwrap();
        session.local_receive_position = u64::MAX;
        assert!(matches!(
            session.receive(br#"{"ok":null}"#),
            Err(Error::Protocol(ProtocolFailure::LocalPositionExhausted))
        ));
        assert_eq!(session.local_receive_position, u64::MAX);
        assert!(
            session
                .terminal()
                .unwrap()
                .unresolved
                .unwrap()
                .possibly_sent
        );
    }
}
