//! Application-owned field-first records. Reuse tracking without this output layer.
#![forbid(unsafe_code)]
pub mod output;
use std::time::Duration;
use tracing::Dispatch;
use umbriel_state::active::StableActive;
use umbriel_tracking::activity::Event;

#[derive(Debug, Clone)]
pub enum Payload {
    Epoch { epoch: u64 },
    Ready { epoch: u64 },
    Gap { epoch: u64, reason: String },
    Stopping,
    Accepted(StableActive),
    Metadata { active: StableActive, accepted_seq: u64 },
}

/// Original identity/time, independently re-emittable through an export Dispatch.
#[derive(Debug, Clone)]
pub struct Record {
    pub run_id: String,
    pub seq: u64,
    pub event_time: String,
    pub event_mono_ms: u64,
    pub dwell_ms: u64,
    pub payload: Payload,
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

pub struct Recorder {
    run_id: String,
    seq: u64,
    dwell: Duration,
    accepted_seq: Option<u64>,
    epoch: u64,
    gap_reported: bool,
}
impl Recorder {
    pub fn new(dwell: Duration) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            seq: 0,
            dwell,
            accepted_seq: None,
            epoch: 0,
            gap_reported: false,
        }
    }
    pub fn record(&mut self, event: Event, now: Duration) -> Record {
        let now = match &event {
            Event::Accepted(active) | Event::Metadata(active) => active.emitted_at,
            _ => now,
        };
        let payload = match event {
            Event::Epoch { epoch } => {
                self.epoch = epoch;
                self.accepted_seq = None;
                self.gap_reported = false;
                Payload::Epoch { epoch }
            }
            Event::Ready { epoch } => {
                self.epoch = epoch;
                self.gap_reported = false;
                Payload::Ready { epoch }
            }
            Event::Gap { epoch, reason } => {
                self.epoch = epoch;
                self.accepted_seq = None;
                self.gap_reported = true;
                Payload::Gap { epoch, reason }
            }
            Event::Accepted(active) => {
                self.accepted_seq = Some(self.seq + 1);
                Payload::Accepted(active)
            }
            Event::Metadata(active) => Payload::Metadata {
                active,
                accepted_seq: self.accepted_seq.expect("metadata follows acceptance in this recorder"),
            },
        };
        self.wrap(payload, now)
    }
    /// A received source gap already describes its terminal delivery error.
    pub fn terminal(&mut self, reason: String, now: Duration) -> Option<Record> {
        if self.gap_reported {
            return None;
        }
        Some(self.record(Event::Gap { epoch: self.epoch, reason }, now))
    }
    pub fn stopping(&mut self, now: Duration) -> Record {
        self.wrap(Payload::Stopping, now)
    }
    fn wrap(&mut self, payload: Payload, now: Duration) -> Record {
        self.seq = self.seq.checked_add(1).expect("activity sequence exhausted");
        Record {
            run_id: self.run_id.clone(),
            seq: self.seq,
            event_time: jiff::Timestamp::now().to_string(),
            event_mono_ms: millis(now),
            dwell_ms: millis(self.dwell),
            payload,
        }
    }
}

/// Emit flat scalar fields once. A private Dispatch isolates data from the host's
/// diagnostic subscriber; no JSON payload is embedded in MESSAGE.
pub fn emit(record: &Record, dispatch: &Dispatch) {
    let (event, epoch, stable, accepted_seq, reason) = match &record.payload {
        Payload::Epoch { epoch } => ("source.epoch", Some(*epoch), None, None, None),
        Payload::Ready { epoch } => ("source.ready", Some(*epoch), None, None, None),
        Payload::Gap { epoch, reason } => ("source.gap", Some(*epoch), None, None, Some(reason.as_str())),
        Payload::Stopping => ("source.stopping", None, None, None, None),
        Payload::Accepted(active) => ("activity.accepted", Some(active.epoch), Some(active), None, None),
        Payload::Metadata { active, accepted_seq } => ("activity.metadata", Some(active.epoch), Some(active), Some(*accepted_seq), None),
    };
    let window = stable.and_then(|active| active.window.as_ref());
    tracing::dispatcher::with_default(dispatch, || {
        tracing::info!(target: "umbriel::activity",
            schema_version = 1_u64, event, run_id = record.run_id.as_str(), seq = record.seq,
            event_time = record.event_time.as_str(), event_mono_ms = record.event_mono_ms,
            epoch, reason, accepted_seq,
            state = stable.map(|s| if s.window.is_some() { "active" } else { "none" }),
            since_mono_ms = stable.map(|s| millis(s.since)),
            last_observed_mono_ms = stable.map(|s| millis(s.last_observed)),
            dwell_ms = stable.map(|_| record.dwell_ms),
            window_id = window.map(|w| w.id.as_str()), app_id = window.map(|w| w.app_id.as_str()),
            title = window.map(|w| w.title.as_str()), workspace_id = window.map(|w| w.workspace.as_str()),
            scratchpad = window.map(|w| w.scratchpad.as_str()),
            "Umbriel activity observation"
        );
    });
}
