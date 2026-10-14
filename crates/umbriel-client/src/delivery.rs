//! Independent every-observation intervals; lag is terminal, never coalescing.
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use umbriel_protocol::envelope::Message;
use umbriel_session::{EndReason, ProtocolFailure, Terminal};

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
    /// Logical queued bytes: inline metadata plus raw payload/diagnostic bytes.
    /// Shared frame bytes are conservatively charged to each observer.
    fn retained_bytes(&self) -> Option<usize> {
        let payload = match self {
            Self::Frame { bytes, .. } => bytes.len(),
            Self::Gap { terminal, .. } => terminal_text_bytes(terminal),
            Self::Epoch { .. } => 0,
        };
        std::mem::size_of::<Self>().checked_add(payload)
    }
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
    state: Arc<Mutex<QueueState>>,
}
impl Observer {
    pub async fn recv(&mut self) -> Result<Observation, DeliveryEnd> {
        match self.receiver.recv().await {
            Some(item) => {
                self.state.lock().expect("delivery accounting lock").bytes -=
                    item.retained_bytes().expect("admitted byte weight");
                Ok(item)
            }
            None => Err(self
                .state
                .lock()
                .expect("delivery terminal lock")
                .end
                .clone()
                .expect("producer retains terminal before close")),
        }
    }
}

pub(crate) struct Producer {
    sender: mpsc::Sender<Observation>,
    state: Arc<Mutex<QueueState>>,
    byte_limit: usize,
}
#[derive(Default)]
struct QueueState {
    bytes: usize,
    end: Option<DeliveryEnd>,
}

pub(crate) fn channel(capacity: usize, byte_limit: usize) -> (Producer, Observer) {
    let (sender, receiver) = mpsc::channel(capacity);
    let state = Arc::new(Mutex::new(QueueState::default()));
    (
        Producer {
            sender,
            state: state.clone(),
            byte_limit,
        },
        Observer { receiver, state },
    )
}
impl Producer {
    pub(crate) fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
    pub(crate) fn push(&self, item: Observation) -> Result<(), ()> {
        let mut state = self.state.lock().expect("delivery accounting lock");
        let Some(total) = item
            .retained_bytes()
            .and_then(|weight| state.bytes.checked_add(weight))
            .filter(|total| *total <= self.byte_limit)
        else {
            return Err(());
        };
        // Reserve before publication, so a concurrently waking receiver cannot
        // release bytes before their reservation exists. No lock crosses await.
        if self.sender.try_send(item).is_err() {
            return Err(());
        }
        state.bytes = total;
        Ok(())
    }
    pub(crate) fn finish(self, reason: DeliveryEnd) {
        self.state.lock().expect("delivery terminal lock").end = Some(reason);
        // sender drops after terminal is installed, even when its queue is full.
    }
}

fn terminal_text_bytes(terminal: &Terminal) -> usize {
    match &terminal.reason {
        EndReason::TransportLost(text) | EndReason::SubscriptionRejected(text) => text.len(),
        EndReason::Protocol(ProtocolFailure::Decode(text) | ProtocolFailure::Framing(text)) => {
            text.len()
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn byte_budget_counts_payload_metadata_and_releases_on_receive() {
        let item = Observation::Frame {
            epoch: 1,
            position: 1,
            bytes: Arc::from(&b"abcd"[..]),
        };
        let weight = item.retained_bytes().unwrap();
        let (producer, mut observer) = channel(10, weight);
        producer.push(item.clone()).unwrap();
        assert!(
            producer.push(item.clone()).is_err(),
            "byte bound applies before message bound"
        );
        observer.recv().await.unwrap();
        producer.push(item).unwrap();
        let gap = Observation::Gap {
            epoch: 1,
            terminal: Terminal {
                reason: EndReason::TransportLost("x".repeat(weight)),
                unresolved: None,
            },
        };
        assert!(
            producer.push(gap).is_err(),
            "gap diagnostic bytes also count"
        );
        producer.finish(DeliveryEnd::Lagged {
            epoch: 1,
            position: 2,
        });
        observer.recv().await.unwrap();
        assert!(matches!(
            observer.recv().await,
            Err(DeliveryEnd::Lagged { .. })
        ));
    }
}
