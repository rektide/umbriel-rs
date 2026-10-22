use facet::Facet;
pub use kopacetic_core::Entity;
use tokio::sync::watch;
use umbriel_client::{DeliveryEnd, Observation, Observer};
use umbriel_protocol::{
    envelope::{EventName, Family, Message},
    model::{Window, Windows},
};

/// Integration-owned reflected projection, not a persistent window identity.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct ActiveWindow {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub workspace: String,
    pub scratchpad: String,
    pub pid: i32,
    pub floating: bool,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl From<&Window> for ActiveWindow {
    fn from(window: &Window) -> Self {
        Self {
            id: window.id.as_str().into(),
            app_id: window.app_id.clone(),
            title: window.title.clone(),
            workspace: window.workspace.as_str().into(),
            scratchpad: window.scratchpad.clone(),
            pid: window.pid,
            floating: window.floating,
            x: window.x,
            y: window.y,
            w: window.w,
            h: window.h,
        }
    }
}

/// Unavailability is never encoded as a valid no-active census.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(C)]
pub enum CursorState {
    AwaitingCensus,
    NoActive,
    Active(ActiveWindow),
    Gap(String),
    Invalid(String),
    Ended(String),
    Stopped,
}

/// Latest processed evidence; epoch and position are local, not server revisions.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct ActiveCursor {
    pub epoch: Option<u64>,
    /// Last relevant frame position; cleared on epoch, gap, or termination.
    pub position: Option<u64>,
    pub state: CursorState,
}

#[derive(Debug)]
pub struct EmptySource;
impl std::fmt::Display for EmptySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("active cursor requires a nonempty source identity")
    }
}
impl std::error::Error for EmptySource {}

/// Latest-value reader. Slow readers coalesce updates, never block the source.
#[derive(Clone)]
pub struct CursorReader {
    receiver: watch::Receiver<Entity<ActiveCursor>>,
}

impl CursorReader {
    /// Owned point-in-time capture. Previously returned captures do not mutate.
    /// Read again after `changed`, including its final channel-closed error.
    pub fn snapshot(&mut self) -> Entity<ActiveCursor> {
        self.receiver.borrow_and_update().clone()
    }

    pub async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.receiver.changed().await
    }
}

/// Sole publisher of a single source-qualified entity. Retain the native
/// Subscription separately: a feed does not own the socket's lifetime.
pub struct CursorFeed {
    sender: watch::Sender<Entity<ActiveCursor>>,
    ended: bool,
}

impl CursorFeed {
    pub fn channel(source: impl Into<String>) -> Result<(Self, CursorReader), EmptySource> {
        let source = source.into();
        if source.trim().is_empty() {
            return Err(EmptySource);
        }
        let (sender, receiver) = watch::channel(Entity::standalone(
            "UmbrielActiveCursor.v1",
            source,
            ActiveCursor {
                epoch: None,
                position: None,
                state: CursorState::AwaitingCensus,
            },
        ));
        Ok((Self { sender, ended: false }, CursorReader { receiver }))
    }

    fn publish(&self, epoch: Option<u64>, position: Option<u64>, state: CursorState) {
        self.sender.send_modify(|entity| {
            entity.payload = ActiveCursor { epoch, position, state };
        });
    }

    /// Apply ordered observations from one native observer. Unselected events
    /// leave a current census intact. A new epoch always clears old knowledge.
    /// Returns whether a publication occurred. Ended feeds cannot be revived.
    pub fn observe(&mut self, observation: &Observation) -> bool {
        if self.ended {
            return false;
        }
        match observation {
            Observation::Epoch { epoch } => {
                self.publish(Some(*epoch), None, CursorState::AwaitingCensus);
            }
            Observation::Gap { epoch, terminal } => {
                self.publish(Some(*epoch), None, CursorState::Gap(format!("{terminal:?}")));
            }
            Observation::Frame { epoch, position, .. } => {
                let new_epoch = self.sender.borrow().payload.epoch != Some(*epoch);
                if new_epoch {
                    self.publish(Some(*epoch), None, CursorState::AwaitingCensus);
                }
                let decoded = observation.message().and_then(|message| match message {
                    Some(Message::Event {
                        name: EventName::Known(Family::Windows),
                        data,
                    }) => umbriel_protocol::codec::decode::<Windows>(data.as_raw_str().as_bytes()).map(Some),
                    _ => Ok(None),
                });
                let state = match decoded {
                    Ok(Some(windows)) => {
                        let mut active = windows.iter().filter(|window| window.active);
                        let first = active.next();
                        if active.next().is_some() {
                            CursorState::Invalid("multiple active windows".into())
                        } else {
                            match first {
                                Some(window) if window.id.as_str().is_empty() => CursorState::Invalid("empty active window ID".into()),
                                Some(window) => CursorState::Active(window.into()),
                                None => CursorState::NoActive,
                            }
                        }
                    }
                    Ok(None) => return new_epoch,
                    Err(error) => CursorState::Invalid(error.to_string()),
                };
                self.publish(Some(*epoch), Some(*position), state);
            }
        }
        true
    }

    /// Terminal observer loss, including lag, clears the selected window.
    pub fn end(&mut self, error: &DeliveryEnd) {
        if !self.ended {
            let epoch = self.sender.borrow().payload.epoch;
            self.publish(epoch, None, CursorState::Ended(error.to_string()));
            self.ended = true;
        }
    }

    /// Own the feed for the entire loop. Cancelling/dropping this future clears
    /// its live value through Drop, even when the future was never polled.
    pub async fn follow(mut self, mut observer: Observer) -> DeliveryEnd {
        loop {
            match observer.recv().await {
                Ok(observation) => {
                    self.observe(&observation);
                }
                Err(error) => {
                    self.end(&error);
                    return error;
                }
            }
        }
    }
}

impl Drop for CursorFeed {
    fn drop(&mut self) {
        if !self.ended {
            let epoch = self.sender.borrow().payload.epoch;
            self.publish(epoch, None, CursorState::Stopped);
        }
    }
}
