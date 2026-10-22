//! Separately driven subscription owner.
use crate::{
    Endpoint, Error, Options, PreparedCommand, RawReply,
    delivery::{self, DeliveryEnd, Observation, Observer, Producer},
    io, requests,
};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::{
    net::UnixStream,
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    time::{Instant, sleep_until},
};
use umbriel_protocol::{codec::Framer, envelope::Family, request::Request};
use umbriel_session::{
    Effect, EndReason, FamilyStatus, LocalRequestId, Phase, Session, StopDisposition, StopReason,
    Terminal,
};

#[derive(Debug, Clone, Default)]
pub enum Reconnect {
    #[default]
    Never,
    /// At most this many fresh connection attempts after each source loss.
    /// Only EOF/transport loss are retried, not protocol/rejection/local close.
    Retry {
        delay: Duration,
        max_attempts: usize,
    },
}

#[derive(Debug, Clone)]
pub struct SubscriptionOptions {
    pub io: Options,
    /// Includes active, queued, and ingress commands, not just channel storage.
    pub command_capacity: usize,
    pub observer_capacity: usize,
    /// Per-observer logical queued bytes, including metadata and gap diagnostics.
    /// One terminal reason is retained separately even when this budget is full.
    pub observer_byte_capacity: usize,
    pub max_observers: usize,
    /// Maximum time to finish/drain a cancelled possibly-sent command.
    pub cancellation_drain_timeout: Duration,
    pub reconnect: Reconnect,
}
impl Default for SubscriptionOptions {
    fn default() -> Self {
        Self {
            io: Options::default(),
            command_capacity: 16,
            observer_capacity: 16,
            observer_byte_capacity: 128 * 1024 * 1024,
            max_observers: 16,
            cancellation_drain_timeout: Duration::from_secs(10),
            reconnect: Reconnect::Never,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Initializing,
    Streaming,
    Disconnected,
    Closed,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub epoch: u64,
    pub health: Health,
    /// Readiness is historical envelope evidence in this epoch, not payload validation.
    pub families: [FamilyStatus; Family::ALL.len()],
    pub last_end: Option<Terminal>,
}
impl Status {
    pub fn family_status(&self, family: Family) -> FamilyStatus {
        self.families[Family::ALL
            .iter()
            .position(|f| *f == family)
            .expect("known family")]
    }
}

struct Owner {
    commands: mpsc::Sender<Command>,
    controls: mpsc::Sender<Register>,
    capacity: Arc<Semaphore>,
    close: watch::Sender<bool>,
    status: watch::Receiver<Status>,
    done: watch::Receiver<bool>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.close.send_replace(true);
    }
}

/// Cheap source owner. Dropping the last clone closes the socket; observers are
/// independent intervals and cannot prolong source ownership. Spawned task owns
/// all protocol and I/O progress, even while callers wait or stop polling.
#[derive(Clone)]
pub struct Subscription {
    owner: Arc<Owner>,
}

impl Endpoint {
    /// Starts the receive owner before returning. The initial observer is installed
    /// before any socket reads so initial snapshots are not lost to registration.
    pub async fn subscribe(
        &self,
        families: &[Family],
        options: SubscriptionOptions,
    ) -> Result<(Subscription, Observer), Error> {
        options.io.limits()?;
        if options.command_capacity == 0
            || options.observer_capacity == 0
            || options.max_observers == 0
        {
            return Err(Error::InvalidOptions("capacities must be nonzero"));
        }
        if options.observer_byte_capacity < std::mem::size_of::<Observation>() {
            return Err(Error::InvalidOptions(
                "observer byte capacity cannot hold epoch metadata",
            ));
        }
        options
            .observer_byte_capacity
            .checked_mul(options.max_observers)
            .ok_or(Error::InvalidOptions("observer byte budgets overflow"))?;
        if options.command_capacity > Semaphore::MAX_PERMITS
            || options.observer_capacity > Semaphore::MAX_PERMITS
            || options.max_observers > Semaphore::MAX_PERMITS
        {
            return Err(Error::InvalidOptions(
                "capacity exceeds Tokio semaphore maximum",
            ));
        }
        options
            .command_capacity
            .checked_mul(umbriel_protocol::REQUEST_LIMIT)
            .ok_or(Error::InvalidOptions("command byte bound overflow"))?;
        options
            .observer_capacity
            .checked_mul(options.io.frame_limit)
            .ok_or(Error::InvalidOptions("observer byte bound overflow"))?;
        options
            .observer_capacity
            .checked_mul(options.io.frame_limit)
            .and_then(|n| n.checked_mul(options.max_observers))
            .ok_or(Error::InvalidOptions("total observer byte bound overflow"))?;
        if Instant::now()
            .checked_add(options.cancellation_drain_timeout)
            .is_none()
            || matches!(options.reconnect, Reconnect::Retry { delay, .. } if Instant::now().checked_add(delay).is_none())
        {
            return Err(Error::InvalidOptions("subscription timing overflow"));
        }
        let session = Session::subscription(families, options.io.frame_limit)?;
        let socket = requests::connect(self, &options.io).await?;
        let (commands, command_rx) = mpsc::channel(options.command_capacity);
        let (controls, control_rx) = mpsc::channel(options.max_observers);
        let (close, close_rx) = watch::channel(false);
        let (done_tx, done) = watch::channel(false);
        let status_value = status(&session, 1, Health::Initializing, None);
        let (status_tx, status) = watch::channel(status_value);
        let (producer, observer) =
            delivery::channel(options.observer_capacity, options.observer_byte_capacity);
        let owner = Arc::new(Owner {
            commands,
            controls,
            capacity: Arc::new(Semaphore::new(options.command_capacity)),
            close,
            status,
            done,
        });
        let driver = Driver {
            endpoint: self.clone(),
            families: families.to_vec(),
            options,
            command_rx,
            control_rx,
            close: close_rx,
            done: done_tx,
            status: status_tx,
            producers: vec![producer],
            queue: VecDeque::new(),
            epoch: 1,
        };
        tokio::spawn(driver.run(socket, session));
        Ok((Subscription { owner }, observer))
    }
}

impl Subscription {
    /// Last owner-published state. After abrupt owner loss it is historical;
    /// status-channel closure and DriverLost outcomes report that local loss.
    pub fn status(&self) -> Status {
        self.owner.status.borrow().clone()
    }
    /// Latest health/readiness, not an every-transition stream. Use Observer for
    /// gaps; channel closure without normal completion signals driver loss.
    pub fn watch_status(&self) -> watch::Receiver<Status> {
        self.owner.status.clone()
    }

    /// Register an every-observation interval starting at owner processing time.
    /// No replay/seed of earlier events is invented.
    pub async fn observe(&self) -> Result<Observer, Error> {
        let (reply, response) = oneshot::channel();
        self.owner
            .controls
            .try_send(Register { reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::QueueFull,
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        receive_response(response).await
    }

    /// Explicit epoch binding prevents an old-state action from running on a new
    /// connection. Encoding happens before bounded enqueue, with no raw-byte bypass.
    /// Deadline includes queue time. Cancellation withdraws unsent commands or
    /// leaves the existing session slot occupied until drainage/close.
    pub async fn request(
        &self,
        epoch: u64,
        request: Request<'_>,
        deadline: Duration,
    ) -> Result<RawReply, Error> {
        {
            let status = self.owner.status.borrow();
            if matches!(status.health, Health::Disconnected | Health::Closed) {
                return Err(Error::Closed);
            }
            if epoch != status.epoch {
                return Err(Error::StaleEpoch {
                    requested: epoch,
                    current: status.epoch,
                });
            }
        }
        let permit = self
            .owner
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::QueueFull)?;
        let prepared = PreparedCommand::new(request)?;
        let (reply, response) = oneshot::channel();
        let deadline = Instant::now()
            .checked_add(deadline)
            .ok_or(Error::InvalidOptions("command deadline overflow"))?;
        let command = Command {
            epoch,
            prepared,
            reply,
            deadline,
            _permit: permit,
        };
        self.owner
            .commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::QueueFull,
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        receive_response(response).await
    }

    /// Closes all clones' source and waits for actual socket/task cleanup.
    pub async fn shutdown(&self) {
        self.owner.close.send_replace(true);
        let mut done = self.owner.done.clone();
        while !*done.borrow_and_update() {
            if done.changed().await.is_err() {
                break;
            }
        }
    }
}

struct Register {
    reply: oneshot::Sender<Result<Observer, Error>>,
}
struct Command {
    epoch: u64,
    prepared: PreparedCommand,
    reply: oneshot::Sender<Result<RawReply, Error>>,
    deadline: Instant,
    _permit: OwnedSemaphorePermit,
}
struct Pending {
    id: LocalRequestId,
    reply: oneshot::Sender<Result<RawReply, Error>>,
    deadline: Instant,
    cancelled: bool,
    _permit: OwnedSemaphorePermit,
}
struct Driver {
    endpoint: Endpoint,
    families: Vec<Family>,
    options: SubscriptionOptions,
    command_rx: mpsc::Receiver<Command>,
    control_rx: mpsc::Receiver<Register>,
    close: watch::Receiver<bool>,
    done: watch::Sender<bool>,
    status: watch::Sender<Status>,
    producers: Vec<Producer>,
    queue: VecDeque<Command>,
    epoch: u64,
}

impl Driver {
    fn publish(&mut self, item: Observation, position: u64) {
        let mut index = 0;
        while index < self.producers.len() {
            if self.producers[index].is_closed() {
                self.producers.swap_remove(index);
                continue;
            }
            match self.producers[index].push(item.clone()) {
                Ok(()) => index += 1,
                Err(_) => self.producers.swap_remove(index).finish(match &item {
                    Observation::Gap { terminal, .. } => DeliveryEnd::Source(terminal.clone()),
                    _ => DeliveryEnd::Lagged {
                        epoch: self.epoch,
                        position,
                    },
                }),
            }
        }
    }

    fn register(&mut self, register: Register) {
        self.producers.retain(|p| !p.is_closed());
        if self.producers.len() >= self.options.max_observers {
            let _ = register.reply.send(Err(Error::QueueFull));
            return;
        }
        let (producer, observer) = delivery::channel(
            self.options.observer_capacity,
            self.options.observer_byte_capacity,
        );
        let announcement = {
            let status = self.status.borrow();
            if status.health == Health::Disconnected {
                Observation::Gap {
                    epoch: self.epoch,
                    terminal: status
                        .last_end
                        .as_ref()
                        .expect("disconnected reason")
                        .clone(),
                }
            } else {
                Observation::Epoch { epoch: self.epoch }
            }
        };
        if producer.push(announcement.clone()).is_err() {
            producer.finish(match announcement {
                Observation::Gap { terminal, .. } => DeliveryEnd::Source(terminal),
                _ => DeliveryEnd::Lagged {
                    epoch: self.epoch,
                    position: 0,
                },
            });
            let _ = register.reply.send(Ok(observer));
        } else if register.reply.send(Ok(observer)).is_ok() {
            self.producers.push(producer);
        }
    }

    fn expire_queue(&mut self) {
        // Rotate exactly the bounded starting population. Surviving FIFO order
        // and the existing backing storage are retained; no suspension occurs.
        for _ in 0..self.queue.len() {
            let command = self.queue.pop_front().expect("starting queue population");
            if command.reply.is_closed() {
                continue;
            }
            if command.epoch != self.epoch {
                let _ = command.reply.send(Err(Error::StaleEpoch {
                    requested: command.epoch,
                    current: self.epoch,
                }));
            } else if command.deadline <= Instant::now() {
                let _ = command.reply.send(Err(Error::TimedOut {
                    possibly_sent: false,
                    reply_observed: false,
                }));
            } else {
                self.queue.push_back(command);
            }
        }
    }

    fn update_status(&self, session: &Session, health: Health) {
        let families = Family::ALL.map(|family| session.family_status(family));
        self.status.send_if_modified(|status| {
            if status.epoch == self.epoch && status.health == health && status.families == families
            {
                return false;
            }
            status.epoch = self.epoch;
            status.health = health;
            status.families = families;
            // Historical terminal evidence is unchanged, not cloned/compared.
            true
        });
    }

    async fn connection(&mut self, socket: &UnixStream, session: &mut Session) {
        let limits = self.options.io.limits().expect("checked options");
        let mut framer = Framer::new(limits);
        let initial_deadline = Instant::now() + self.options.io.request_timeout;
        let mut pending: Option<Pending> = None;
        self.publish(Observation::Epoch { epoch: self.epoch }, 0);
        loop {
            if *self.close.borrow() {
                session.close();
                break;
            }
            let mut frames = 0;
            while frames < 32 {
                let Some(frame) = framer.next_frame() else {
                    break;
                };
                frames += 1;
                match session.receive(frame) {
                    Ok(Effect::Reply { message, .. }) => {
                        if let Some(command) = pending.take()
                            && !command.reply.is_closed()
                        {
                            let _ = command
                                .reply
                                .send(Ok(RawReply::from_message(frame, message)));
                        }
                    }
                    Ok(
                        Effect::Event {
                            local_receive_position,
                            ..
                        }
                        | Effect::SubscriptionAcceptedEmpty {
                            local_receive_position,
                            ..
                        },
                    ) => {
                        if !self.producers.is_empty() {
                            self.publish(
                                Observation::Frame {
                                    epoch: self.epoch,
                                    position: local_receive_position,
                                    bytes: Arc::from(frame),
                                },
                                local_receive_position,
                            );
                        }
                    }
                    Ok(Effect::SubscriptionRejected { .. }) | Err(_) => break,
                }
                if session.terminal().is_some() {
                    break;
                }
            }
            if session.terminal().is_some() {
                break;
            }
            self.update_status(
                session,
                if session.phase() == Phase::Streaming {
                    Health::Streaming
                } else {
                    Health::Initializing
                },
            );
            self.expire_queue();
            if pending.is_none()
                && session.phase() == Phase::Streaming
                && let Some(command) = self.queue.pop_front()
            {
                // No suspension between cancellation check and admission.
                if !command.reply.is_closed() {
                    match session.admit_prepared(command.prepared) {
                        Ok(id) => {
                            pending = Some(Pending {
                                id,
                                reply: command.reply,
                                deadline: command.deadline,
                                cancelled: false,
                                _permit: command._permit,
                            })
                        }
                        Err(error) => {
                            let _ = command.reply.send(Err(error.into()));
                        }
                    }
                }
            }
            let deadline = self
                .queue
                .iter()
                .map(|c| c.deadline)
                .chain(pending.iter().map(|p| p.deadline))
                .chain((session.phase() == Phase::Subscribing).then_some(initial_deadline))
                .min();
            let cancelled = pending.as_ref().is_some_and(|p| !p.cancelled);
            tokio::select! {
                _ = self.close.changed() => { session.close(); break; },
                register = self.control_rx.recv() => { if let Some(register) = register { self.register(register); } },
                command = self.command_rx.recv() => { if let Some(command) = command { self.queue.push_back(command); } },
                _ = async { pending.as_mut().expect("guarded pending").reply.closed().await }, if cancelled => {
                    let command = pending.as_mut().expect("pending cancellation");
                    match session.stop_waiting(command.id, StopReason::Cancelled).expect("current session slot") {
                        StopDisposition::Withdrawn => { pending = None; },
                        StopDisposition::DrainRequired => { command.cancelled = true; command.deadline = Instant::now() + self.options.cancellation_drain_timeout; }
                    }
                },
                _ = async { sleep_until(deadline.expect("guarded deadline")).await }, if deadline.is_some() => {
                    if session.phase() == Phase::Subscribing || pending.as_ref().is_some_and(|p| p.deadline <= Instant::now()) {
                        if session.phase() == Phase::Subscribing {
                            session.stop_waiting(session.pending_request().expect("initial slot"), StopReason::TimedOut).expect("initial waiter");
                        } else if let Some(command) = pending.as_ref().filter(|p| !p.cancelled) {
                            session.stop_waiting(command.id, StopReason::TimedOut).expect("current waiter");
                        }
                        let terminal = session.close().clone();
                        if let Some(command) = pending.take() {
                            let _ = command.reply.send(Err(Error::TimedOut { possibly_sent: terminal.unresolved.is_some_and(|r| r.possibly_sent), reply_observed: false }));
                        }
                        break;
                    }
                },
                _ = tokio::task::yield_now(), if frames == 32 => {},
                result = io::step(socket, session, &mut framer, limits.buffered), if frames != 32 => {
                    match result { Ok(true) => {}, Ok(false) => { session.eof(); }, Err(reason) => io::end(session, reason) }
                }
            }
            if session.terminal().is_some() {
                break;
            }
            // A ready socket cannot monopolize the executor indefinitely.
            tokio::task::yield_now().await;
        }
        if let Some(command) = pending {
            let _ = command.reply.send(Err(requests::ended(session, false)));
        }
    }

    fn reject_queued(&mut self) {
        while let Ok(command) = self.command_rx.try_recv() {
            self.queue.push_back(command);
        }
        for command in self.queue.drain(..) {
            let _ = command.reply.send(Err(Error::Closed));
        }
    }

    async fn retry(&mut self) -> Option<(UnixStream, Session)> {
        let Reconnect::Retry {
            delay,
            max_attempts,
        } = self.options.reconnect
        else {
            return None;
        };
        for _ in 0..max_attempts {
            let wake = Instant::now() + delay;
            loop {
                if *self.close.borrow() {
                    return None;
                }
                tokio::select! {
                    _ = self.close.changed() => return None,
                    _ = sleep_until(wake) => break,
                    register = self.control_rx.recv() => { if let Some(register) = register { self.register(register); } },
                    command = self.command_rx.recv() => { if let Some(command) = command { let _ = command.reply.send(Err(Error::Closed)); } }
                }
            }
            let result = tokio::select! {
                _ = self.close.changed() => return None,
                result = requests::connect(&self.endpoint, &self.options.io) => result,
            };
            if let Ok(socket) = result {
                let session = Session::subscription(&self.families, self.options.io.frame_limit)
                    .expect("fixed validated families");
                self.epoch = self.epoch.checked_add(1)?;
                self.update_status(&session, Health::Initializing);
                return Some((socket, session));
            }
        }
        None
    }

    async fn run(mut self, mut socket: UnixStream, mut session: Session) {
        loop {
            self.connection(&socket, &mut session).await;
            drop(socket);
            let terminal = session.terminal().expect("connection terminal").clone();
            self.status.send_replace(status(
                &session,
                self.epoch,
                Health::Disconnected,
                Some(terminal.clone()),
            ));
            self.publish(
                Observation::Gap {
                    epoch: self.epoch,
                    terminal: terminal.clone(),
                },
                0,
            );
            self.reject_queued();
            if matches!(
                terminal.reason,
                EndReason::Eof | EndReason::TransportLost(_)
            ) && let Some((fresh_socket, fresh_session)) = self.retry().await
            {
                socket = fresh_socket;
                session = fresh_session;
                continue;
            }
            self.command_rx.close();
            self.control_rx.close();
            self.reject_queued();
            while let Ok(register) = self.control_rx.try_recv() {
                let _ = register.reply.send(Err(Error::Closed));
            }
            for producer in self.producers.drain(..) {
                producer.finish(DeliveryEnd::Source(terminal.clone()));
            }
            self.update_status(&session, Health::Closed);
            self.done.send_replace(true);
            break;
        }
    }
}

fn status(session: &Session, epoch: u64, health: Health, last_end: Option<Terminal>) -> Status {
    Status {
        epoch,
        health,
        families: Family::ALL.map(|f| session.family_status(f)),
        last_end,
    }
}

async fn receive_response<T>(response: oneshot::Receiver<Result<T, Error>>) -> Result<T, Error> {
    response.await.map_err(|_| Error::DriverLost)?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Driver {
        let (_, command_rx) = mpsc::channel(16);
        let (_, control_rx) = mpsc::channel(16);
        let (_, close) = watch::channel(false);
        let (done, _) = watch::channel(false);
        let (status, _) = watch::channel(status(
            &Session::one_shot(1024),
            1,
            Health::Initializing,
            None,
        ));
        Driver {
            endpoint: Endpoint::new("/not-connected"),
            families: vec![Family::Windows],
            options: SubscriptionOptions::default(),
            command_rx,
            control_rx,
            close,
            done,
            status,
            producers: Vec::with_capacity(8),
            queue: VecDeque::with_capacity(8),
            epoch: 1,
        }
    }

    #[tokio::test]
    async fn publication_retires_in_place_and_preserves_independent_delivery() {
        let mut driver = fixture();
        let (closed, dropped) = delivery::channel(1, 4096);
        drop(dropped);
        let (slow, mut stalled) = delivery::channel(1, 4096);
        slow.push(Observation::Epoch { epoch: 1 }).unwrap();
        let (fast, mut observer) = delivery::channel(1, 4096);
        driver.producers.extend([closed, slow, fast]);
        let pointer = driver.producers.as_ptr();
        let capacity = driver.producers.capacity();
        for position in 1..10 {
            driver.publish(Observation::Epoch { epoch: 2 }, position);
            assert!(matches!(
                observer.recv().await,
                Ok(Observation::Epoch { epoch: 2 })
            ));
            assert_eq!(driver.producers.as_ptr(), pointer);
            assert_eq!(driver.producers.capacity(), capacity);
        }
        assert_eq!(driver.producers.len(), 1);
        stalled.recv().await.unwrap();
        assert!(matches!(
            stalled.recv().await,
            Err(DeliveryEnd::Lagged { position: 1, .. })
        ));
    }

    #[tokio::test]
    async fn expiry_preserves_queue_storage_and_surviving_fifo_order() {
        let mut driver = fixture();
        let permits = Arc::new(Semaphore::new(8));
        let first = Instant::now() + Duration::from_secs(100);
        let last = first + Duration::from_secs(1);
        let mut responses = Vec::new();
        for (epoch, deadline) in [
            (1, first),
            (1, first),
            (0, first),
            (1, Instant::now()),
            (1, last),
        ] {
            let (reply, response) = oneshot::channel();
            driver.queue.push_back(Command {
                epoch,
                deadline,
                reply,
                prepared: PreparedCommand::new(Request::Msg("queued")).unwrap(),
                _permit: permits.clone().try_acquire_owned().unwrap(),
            });
            responses.push(response);
        }
        drop(responses.remove(1)); // second waiter cancels before transmission
        let capacity = driver.queue.capacity();
        driver.expire_queue();
        assert_eq!(driver.queue.capacity(), capacity);
        assert_eq!(
            driver
                .queue
                .iter()
                .map(|command| command.deadline)
                .collect::<Vec<_>>(),
            vec![first, last]
        );
        assert!(matches!(
            responses.remove(1).await.unwrap(),
            Err(Error::StaleEpoch {
                requested: 0,
                current: 1
            })
        ));
        assert!(matches!(
            responses.remove(1).await.unwrap(),
            Err(Error::TimedOut {
                possibly_sent: false,
                ..
            })
        ));
        for _ in 0..10 {
            driver.expire_queue();
        }
        assert_eq!(driver.queue.capacity(), capacity);
        assert_eq!(driver.queue.len(), 2);
    }

    #[test]
    fn status_only_notifies_changed_fields_and_keeps_historical_terminal_storage() {
        let driver = fixture();
        let mut receiver = driver.status.subscribe();
        let mut core =
            Session::subscription(&[Family::Windows, Family::KeyboardLayout], 1024).unwrap();
        driver.update_status(&core, Health::Initializing);
        driver.status.send_modify(|status| {
            status.last_end = Some(Terminal {
                reason: EndReason::TransportLost("historical detail".repeat(64)),
                unresolved: None,
            })
        });
        let old_pointer = match &receiver
            .borrow_and_update()
            .last_end
            .as_ref()
            .unwrap()
            .reason
        {
            EndReason::TransportLost(text) => text.as_ptr(),
            _ => unreachable!(),
        };
        for _ in 0..10 {
            driver.update_status(&core, Health::Initializing);
        }
        assert!(!receiver.has_changed().unwrap());
        let count = core.begin_write().unwrap().len();
        core.write_progress(count).unwrap();
        core.receive(br#"{"event":"windows","data":[]}"#).unwrap();
        driver.update_status(&core, Health::Streaming);
        assert!(receiver.has_changed().unwrap());
        receiver.borrow_and_update();
        core.receive(br#"{"event":"windows","data":[]}"#).unwrap();
        driver.update_status(&core, Health::Streaming);
        assert!(!receiver.has_changed().unwrap());
        match &receiver.borrow().last_end.as_ref().unwrap().reason {
            EndReason::TransportLost(text) => assert_eq!(text.as_ptr(), old_pointer),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn admitted_response_owner_loss_never_claims_nontransmission() {
        let (commands, mut ingress) = mpsc::channel(1);
        let (controls, _controls) = mpsc::channel(1);
        let (close, _close) = watch::channel(false);
        let (_done, done) = watch::channel(false);
        let session = Session::one_shot(1024);
        let (_status, status) = watch::channel(status(&session, 1, Health::Streaming, None));
        let source = Subscription {
            owner: Arc::new(Owner {
                commands,
                controls,
                capacity: Arc::new(Semaphore::new(1)),
                close,
                status,
                done,
            }),
        };
        let request = tokio::spawn(async move {
            source
                .request(1, Request::Msg("possibly-executed"), Duration::from_secs(5))
                .await
        });
        let command = ingress.recv().await.unwrap();
        let mut core = Session::one_shot(1024);
        core.admit_prepared(command.prepared).unwrap();
        let count = core.begin_write().unwrap().len();
        core.write_progress(count).unwrap();
        drop(command.reply); // local driver disappears after accounted transmission
        assert!(matches!(request.await.unwrap(), Err(Error::DriverLost)));
        assert!(core.close().unresolved.unwrap().possibly_sent);

        let (reply, response) = oneshot::channel::<Result<Observer, Error>>();
        drop(reply);
        assert!(matches!(
            receive_response(response).await,
            Err(Error::DriverLost)
        ));
    }
}
