//! Independent every-observation intervals; lag is terminal, never coalescing.
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use umbriel_protocol::envelope::Message;
use umbriel_session::Terminal;

#[derive(Debug, Clone)]
pub enum Observation {
    /// Epoch announcement, never a server cursor or continuity promise.
    Epoch { epoch: u64 },
    /// Includes unknown/unselected events and the special initial blank.
    Frame {
        epoch: u64,
        position: u64,
        bytes: Arc<[u8]>,
    },
    /// Source stopped; a later Epoch (if reconnect was opted into) starts afresh.
    Gap { epoch: u64, terminal: Terminal },
}
impl Observation {
    pub fn frame(&self) -> Option<&[u8]> {
        match self {
            Self::Frame { bytes, .. } => Some(bytes),
            _ => None,
        }
    }
    pub fn message(&self) -> Option<Message<'_>> {
        self.frame()
            .map(|frame| Message::decode(frame).expect("session validated frame"))
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DeliveryEnd {
    #[error("observer lost observations at epoch {epoch}, receive position {position}")]
    Lagged { epoch: u64, position: u64 },
    #[error("source ended: {0:?}")]
    Source(Terminal),
}

/// Already queued observations drain before the first retained terminal error.
/// The error remains available on subsequent recv calls too. Drop retires only
/// this observer; observers do not keep the source socket alive.
pub struct Observer {
    receiver: mpsc::Receiver<Observation>,
    end: Arc<Mutex<Option<DeliveryEnd>>>,
}
impl Observer {
    pub async fn recv(&mut self) -> Result<Observation, DeliveryEnd> {
        match self.receiver.recv().await {
            Some(item) => Ok(item),
            None => Err(self
                .end
                .lock()
                .expect("delivery terminal lock")
                .clone()
                .expect("producer retains terminal before close")),
        }
    }
}

pub(crate) struct Producer {
    sender: mpsc::Sender<Observation>,
    end: Arc<Mutex<Option<DeliveryEnd>>>,
}
pub(crate) fn channel(capacity: usize) -> (Producer, Observer) {
    let (sender, receiver) = mpsc::channel(capacity);
    let end = Arc::new(Mutex::new(None));
    (
        Producer {
            sender,
            end: end.clone(),
        },
        Observer { receiver, end },
    )
}
impl Producer {
    pub(crate) fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
    pub(crate) fn push(
        &self,
        item: Observation,
    ) -> Result<(), mpsc::error::TrySendError<Observation>> {
        self.sender.try_send(item)
    }
    pub(crate) fn finish(self, reason: DeliveryEnd) {
        *self.end.lock().expect("delivery terminal lock") = Some(reason);
        // sender drops after terminal is installed, even when its queue is full.
    }
}
