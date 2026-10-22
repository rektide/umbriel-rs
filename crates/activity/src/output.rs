//! One tracing projection per destination. Journal callbacks are best-effort.
use std::{error::Error, fmt};
use tracing::{
    Dispatch, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    EnvFilter, Layer,
    filter::{FilterExt, filter_fn},
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    layer::SubscriberExt,
    registry::LookupSpan,
};

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum Output {
    #[value(alias = "ndjson")]
    Jsonl,
    Logfmt,
    Journal,
}

/// Synchronous tracing I/O lives on one dedicated thread. The bounded channel
/// fails explicitly on overload. A stuck writer can be detached at process exit
/// after a bounded drain timeout; delivery is not a persistence acknowledgement.
pub struct Worker {
    sender: tokio::sync::mpsc::Sender<crate::Record>,
    done: tokio::sync::oneshot::Receiver<()>,
}

impl Worker {
    pub async fn start(output: Output) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<crate::Record>(64);
        let (ready, initialized) = tokio::sync::oneshot::channel();
        let (finished, done) = tokio::sync::oneshot::channel();
        std::thread::Builder::new().name("activity-output".into()).spawn(move || {
            match dispatch(output) {
                Ok(dispatch) => {
                    if ready.send(Ok(())).is_ok() {
                        while let Some(record) = receiver.blocking_recv() {
                            crate::emit(&record, &dispatch);
                        }
                    }
                }
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
            }
            let _ = finished.send(());
        })?;
        initialized.await??;
        Ok(Self { sender, done })
    }
    /// At most 64 queued records, each with at most 1 MiB of owned string data.
    pub fn send(&self, record: crate::Record) -> std::io::Result<()> {
        use crate::Payload;
        let payload_bytes = match &record.payload {
            Payload::Accepted(active) | Payload::Metadata { active, .. } => active.window.as_ref().map_or(0, |w| {
                w.id.as_str().len() + w.app_id.len() + w.title.len() + w.workspace.as_str().len() + w.scratchpad.len()
            }),
            Payload::Gap { reason, .. } => reason.len(),
            _ => 0,
        };
        if payload_bytes + record.run_id.len() + record.event_time.len() > 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "activity record exceeds 1 MiB output budget",
            ));
        }
        self.sender.try_send(record).map_err(|error| match error {
            tokio::sync::mpsc::error::TrySendError::Full(_) => {
                std::io::Error::new(std::io::ErrorKind::WouldBlock, "activity output queue is full; continuity lost")
            }
            tokio::sync::mpsc::error::TrySendError::Closed(_) => std::io::Error::new(std::io::ErrorKind::BrokenPipe, "activity output worker ended"),
        })
    }
    pub async fn closed(&self) {
        self.sender.closed().await;
    }
    pub async fn drain(self) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        drop(self.sender);
        self.done.await
    }
}

pub fn dispatch(output: Output) -> Result<Dispatch, Box<dyn Error + Send + Sync>> {
    let filter =
        filter_fn(|metadata| metadata.target() == "umbriel::activity").or(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")));
    Ok(match output {
        Output::Jsonl => Dispatch::new(tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().json().with_filter(filter))),
        Output::Logfmt => Dispatch::new(tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().event_format(Logfmt).with_filter(filter))),
        Output::Journal => Dispatch::new(
            tracing_subscriber::registry().with(
                tracing_journald::layer()?
                    .with_field_prefix(Some("UMBRIEL".into()))
                    .with_syslog_identifier("umbriel-activity".into())
                    .with_filter(filter),
            ),
        ),
    })
}

/// Flat logfmt dialect: all string/debug values use JSON quoting/escaping;
/// numbers and booleans are unquoted. No spans are implicit record context.
pub struct Logfmt;
impl<S, N> FormatEvent<S, N> for Logfmt
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, _: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &tracing::Event<'_>) -> fmt::Result {
        write!(
            writer,
            "level={} target={}",
            event.metadata().level(),
            serde_json::to_string(event.metadata().target()).map_err(|_| fmt::Error)?
        )?;
        let mut visitor = LogfmtVisitor { writer, result: Ok(()) };
        event.record(&mut visitor);
        visitor.result?;
        writeln!(visitor.writer)
    }
}
struct LogfmtVisitor<'a> {
    writer: Writer<'a>,
    result: fmt::Result,
}
impl LogfmtVisitor<'_> {
    fn value(&mut self, field: &Field, value: impl fmt::Display) {
        if self.result.is_ok() {
            self.result = write!(self.writer, " {}={value}", field.name());
        }
    }
}
impl Visit for LogfmtVisitor<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.value(field, serde_json::to_string(value).expect("string serialization"));
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.value(field, value);
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.value(field, value);
    }
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.value(field, value);
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_str(field, &format!("{value:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn output_count_byte_and_closed_limits_are_explicit_failures() {
        let (sender, receiver) = tokio::sync::mpsc::channel(64);
        let (_finished, done) = tokio::sync::oneshot::channel();
        let worker = Worker { sender, done };
        let mut record = crate::Record {
            run_id: "test".into(),
            seq: 1,
            event_time: "now".into(),
            event_mono_ms: 0,
            dwell_ms: 5000,
            payload: crate::Payload::Stopping,
        };
        for _ in 0..64 {
            worker.send(record.clone()).unwrap();
        }
        assert_eq!(worker.send(record.clone()).unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        record.payload = crate::Payload::Gap {
            epoch: 1,
            reason: "x".repeat(1024 * 1024),
        };
        assert_eq!(worker.send(record.clone()).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        record.payload = crate::Payload::Stopping;
        drop(receiver);
        assert_eq!(worker.send(record).unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
    }
}
