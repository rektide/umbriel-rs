//! Queue-before-timer composition; no detached producer and no unbounded queue.
use crate::{Tracker, Update};
use std::time::Duration;
use tokio::time::{Instant, sleep_until};
use umbriel_client::{DeliveryEnd, Observation, Observer};
use umbriel_protocol::envelope::Family;
use umbriel_state::active::{StableActive, StableActiveReducer};

#[derive(Debug)]
pub enum Event {
    Epoch { epoch: u64 },
    Gap { epoch: u64, reason: String },
    Ready { epoch: u64 },
    Accepted(StableActive),
    Metadata(StableActive),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("observer delivery ended: {0}")]
    Delivery(#[from] DeliveryEnd),
    #[error(transparent)]
    Reducer(#[from] umbriel_state::active::Error),
    #[error("runtime deadline overflow")]
    DeadlineOverflow,
}

pub struct Activity {
    tracker: Tracker,
    reducer: StableActiveReducer,
    origin: Instant,
    ready: bool,
}

impl Activity {
    pub fn new(dwell: Duration) -> Self {
        Self {
            tracker: Tracker::new(&[Family::Windows], false),
            reducer: StableActiveReducer::new(dwell),
            origin: Instant::now(),
            ready: false,
        }
    }
    pub fn elapsed(&self) -> Duration {
        self.origin.elapsed()
    }
    pub fn tracker(&self) -> &Tracker {
        &self.tracker
    }
    pub fn invalidate(&mut self) {
        self.tracker.lost();
        self.reducer.invalidate();
        self.ready = false;
    }
    pub fn observe(&mut self, observation: Observation) -> Option<Event> {
        let now = self.elapsed();
        match self.tracker.observe(now, observation) {
            Update::Epoch { epoch } => {
                self.reducer.invalidate();
                self.ready = false;
                Some(Event::Epoch { epoch })
            }
            Update::Gap { epoch, reason } | Update::Rejected { epoch, reason, .. } => {
                self.reducer.invalidate();
                self.ready = false;
                Some(Event::Gap { epoch, reason })
            }
            Update::Installed { after, .. } => {
                let epoch = after.stamp.expect("installed stamp").epoch;
                let snapshot = self.tracker.inventory().snapshot();
                match self
                    .reducer
                    .observe_changes(epoch, now, snapshot.windows.entries().values().map(|w| w.as_ref()))
                {
                    Ok(metadata) => {
                        if !self.ready {
                            self.ready = true;
                            Some(Event::Ready { epoch })
                        } else {
                            metadata.map(Event::Metadata)
                        }
                    }
                    Err(error) => {
                        self.tracker.inventory.invalid_snapshot(Family::Windows);
                        self.ready = false;
                        Some(Event::Gap {
                            epoch,
                            reason: error.to_string(),
                        })
                    }
                }
            }
            Update::Ignored => None,
        }
    }

    /// Cancellation-safe: all consumed input is applied before yielding. The
    /// caller retains the Subscription and selects cancellation around this call.
    /// Terminal loss invalidates state and returns Err (never accepted none).
    pub async fn next(&mut self, observer: &mut Observer) -> Result<Event, Error> {
        loop {
            let mut caught_up = false;
            for _ in 0..64 {
                match observer.try_recv() {
                    Ok(Some(item)) => {
                        if let Some(event) = self.observe(item) {
                            return Ok(event);
                        }
                    }
                    Ok(None) => {
                        caught_up = true;
                        break;
                    }
                    Err(error) => {
                        self.invalidate();
                        return Err(error.into());
                    }
                }
            }
            if !caught_up {
                tokio::task::yield_now().await;
                continue;
            }
            if let Some(stable) = self.reducer.poll(self.elapsed())? {
                return Ok(Event::Accepted(stable));
            }
            if let Some(deadline) = self.reducer.deadline() {
                let deadline = self.origin.checked_add(deadline).ok_or(Error::DeadlineOverflow)?;
                tokio::select! {
                    biased;
                    item = observer.recv() => match item {
                        Ok(item) => if let Some(event) = self.observe(item) { return Ok(event); },
                        Err(error) => { self.invalidate(); return Err(error.into()); }
                    },
                    _ = sleep_until(deadline) => {}
                }
            } else {
                match observer.recv().await {
                    Ok(item) => {
                        if let Some(event) = self.observe(item) {
                            return Ok(event);
                        }
                    }
                    Err(error) => {
                        self.invalidate();
                        return Err(error.into());
                    }
                }
            }
        }
    }
}
