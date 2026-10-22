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
