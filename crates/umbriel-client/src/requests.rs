//! One-shot sockets are never pooled; caller-selected decode follows wire retirement.
use crate::{Endpoint, io};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::{net::UnixStream, time::timeout};
use umbriel_protocol::{
    codec::{Framer, Limits},
    envelope::Message,
    request::Request,
};
use umbriel_session::{Effect, ProtocolFailure, Session, Terminal};

#[derive(Debug, Clone)]
pub struct Options {
    pub connect_timeout: Duration,
    /// Whole request deadline, including EOF after the reply.
    pub request_timeout: Duration,
    /// Maximum inbound payload bytes excluding LF.
    pub frame_limit: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
            frame_limit: 8 * 1024 * 1024,
        }
    }
}
impl Options {
    pub(crate) fn limits(&self) -> Result<Limits, Error> {
        let buffered = self
            .frame_limit
            .checked_add(1)
            .ok_or(Error::InvalidOptions("frame limit overflow"))?;
        Ok(Limits {
            frame: self.frame_limit,
            buffered,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid client options: {0}")]
    InvalidOptions(&'static str),
    #[error("connect failed: {0}")]
    Connect(#[source] std::io::Error),
    #[error("connect timed out (not transmitted)")]
    ConnectTimeout,
    #[error(transparent)]
    Admission(#[from] umbriel_session::Error),
    #[error("connection ended: {terminal:?}; native reply observed: {reply_observed}")]
    Ended {
        terminal: Terminal,
        reply_observed: bool,
    },
    #[error(
        "request deadline elapsed; possibly sent: {possibly_sent}; native reply observed: {reply_observed}"
    )]
    TimedOut {
        possibly_sent: bool,
        reply_observed: bool,
    },
    #[error("remote rejection: {0}")]
    Remote(String),
    #[error("local payload decode failed: {0}")]
    Decode(#[source] sonic_rs::Error),
    #[error("command ingress is full (not transmitted)")]
    QueueFull,
    #[error("subscription is closed (not transmitted)")]
    Closed,
    #[error("command belongs to epoch {requested}, current epoch is {current} (not transmitted)")]
    StaleEpoch { requested: u64, current: u64 },
}

/// One validated raw native reply. Its payload can be decoded into borrowed
/// projections without allocating a full owned model tree.
#[derive(Debug, Clone)]
pub struct RawReply {
    pub(crate) frame: Arc<[u8]>,
}
impl RawReply {
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }
    pub fn message(&self) -> Message<'_> {
        Message::decode(&self.frame).expect("validated reply")
    }
    pub fn decode<'a, T: Deserialize<'a>>(&'a self) -> Result<T, Error> {
        match self.message() {
            Message::Ok(_) => {
                #[derive(Deserialize)]
                struct Success<T> {
                    ok: T,
                }
                umbriel_protocol::codec::decode::<Success<T>>(&self.frame)
                    .map(|reply| reply.ok)
                    .map_err(Error::Decode)
            }
            Message::Err(text) => Err(Error::Remote(text.into_owned())),
            _ => unreachable!("reply validated by session"),
        }
    }
}

impl Endpoint {
    pub async fn request(
        &self,
        request: Request<'_>,
        options: &Options,
    ) -> Result<RawReply, Error> {
        let limits = options.limits()?;
        let mut session = Session::one_shot(options.frame_limit);
        session.admit(request)?;
        let socket = connect(self, options).await?;
        let mut framer = Framer::new(limits);
        let mut reply = None;
        let result = timeout(options.request_timeout, async {
            loop {
                while let Some(frame) = framer.next_frame() {
                    if let Effect::Reply { .. } = session.receive(frame)? {
                        reply = Some(RawReply {
                            frame: Arc::from(frame),
                        });
                    }
                }
                match io::step(&socket, &mut session, &mut framer, limits.buffered).await {
                    Ok(true) => {}
                    Ok(false) => {
                        session.eof();
                        break;
                    }
                    Err(reason) => {
                        io::end(&mut session, reason);
                        break;
                    }
                }
            }
            Ok::<(), Error>(())
        })
        .await;
        if result.is_err() {
            let terminal = session.close();
            return Err(Error::TimedOut {
                possibly_sent: terminal.unresolved.is_some_and(|r| r.possibly_sent),
                reply_observed: reply.is_some(),
            });
        }
        if let Ok(Err(error)) = result {
            if let Error::Admission(_) = error {
                return Err(ended(&session, reply.is_some()));
            }
            return Err(error);
        }
        let terminal = session.terminal().expect("finished socket");
        if terminal.reason == umbriel_session::EndReason::Eof {
            if let Some(reply) = reply {
                return Ok(reply);
            }
        }
        Err(ended(&session, reply.is_some()))
    }

    pub async fn query<T: serde::de::DeserializeOwned>(
        &self,
        request: Request<'_>,
        options: &Options,
    ) -> Result<T, Error> {
        self.request(request, options).await?.decode()
    }
}

pub(crate) async fn connect(endpoint: &Endpoint, options: &Options) -> Result<UnixStream, Error> {
    timeout(
        options.connect_timeout,
        UnixStream::connect(endpoint.path()),
    )
    .await
    .map_err(|_| Error::ConnectTimeout)?
    .map_err(Error::Connect)
}
pub(crate) fn ended(session: &Session, reply_observed: bool) -> Error {
    Error::Ended {
        terminal: session.terminal().cloned().unwrap_or(Terminal {
            reason: umbriel_session::EndReason::Protocol(ProtocolFailure::Framing(
                "driver ended without terminal".into(),
            )),
            unresolved: None,
        }),
        reply_observed,
    }
}
