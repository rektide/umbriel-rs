use clap::{Parser, Subcommand};
use std::{error::Error, path::PathBuf, time::Duration};
use umbriel_activity::{
    Recorder,
    output::{Output, Worker},
};
use umbriel_client::{Discovery, Endpoint, Reconnect, SubscriptionOptions};
use umbriel_protocol::envelope::Family;
use umbriel_tracking::activity::Activity;
mod history_cli;

#[derive(Debug, Parser)]
#[command(version, about = "Observe stable Umbriel activity through tracing layers")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    /// Explicit socket; otherwise use the client's environment discovery.
    #[arg(long)]
    socket: Option<PathBuf>,
    #[arg(long, default_value_t = 5000)]
    dwell_ms: u64,
    #[arg(long, value_enum, default_value_t = Output::Jsonl)]
    output: Output,
    /// Fixed-delay retries after transport loss; initial connection still fails fast.
    #[arg(long, default_value_t = 10)]
    reconnect_attempts: usize,
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(1..))]
    reconnect_delay_ms: u64,
    #[arg(long, default_value_t = 3000, value_parser = clap::value_parser!(u64).range(1..))]
    shutdown_timeout_ms: u64,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show recent window titles; optionally replay then follow the running service.
    History(history_cli::History),
    /// Export retained activity journal records as JSON Lines.
    Dump(history_cli::Dump),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let shutdown_timeout = Duration::from_millis(args.shutdown_timeout_ms);
    match args.command {
        Some(Command::Dump(range)) => return history_cli::dump(range, shutdown_timeout).await,
        Some(Command::History(history)) => return history_cli::history(history, shutdown_timeout).await,
        None => {}
    }
    let worker = tokio::time::timeout(shutdown_timeout, Worker::start(args.output)).await??;
    let endpoint = match args.socket {
        Some(path) => Endpoint::new(path),
        None => Discovery::from_env().resolve()?,
    };
    // Register signal handling before opening the source.
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let options = SubscriptionOptions {
        reconnect: Reconnect::Retry {
            delay: Duration::from_millis(args.reconnect_delay_ms),
            max_attempts: args.reconnect_attempts,
        },
        ..Default::default()
    };
    let (source, mut observer) = endpoint.subscribe(&[Family::Windows], options).await?;
    let dwell = Duration::from_millis(args.dwell_ms);
    let mut activity = Activity::new(dwell);
    let mut recorder = Recorder::new(dwell);
    let result = loop {
        tokio::select! {
            biased;
            _ = terminate.recv() => break Ok(()),
            _ = interrupt.recv() => break Ok(()),
            _ = worker.closed() => break Err("activity output worker ended".into()),
            event = activity.next(&mut observer) => match event {
                Ok(event) => if let Err(error) = worker.send(recorder.record(event, activity.elapsed())) { break Err(error.into()); },
                Err(error) => {
                    if let Some(record) = recorder.terminal(error.to_string(), activity.elapsed()) { let _ = worker.send(record); }
                    break Err(Box::<dyn Error + Send + Sync>::from(error));
                }
            }
        }
    };
    let stopping = worker.send(recorder.stopping(activity.elapsed()));
    tokio::time::timeout(shutdown_timeout, async {
        source.shutdown().await;
        worker.drain().await
    })
    .await??;
    stopping?;
    result
}

fn dump_record(entry: &umbriel_activity::history::Entry) -> serde_json::Value {
    use serde_json::{Map, Value, json};
    use umbriel_activity::Payload;
    let r = &entry.record;
    let mut fields = Map::new();
    fields.insert("schema_version".into(), json!(1));
    fields.insert("run_id".into(), json!(r.run_id));
    fields.insert("seq".into(), json!(r.seq));
    fields.insert("event_time".into(), json!(r.event_time));
    fields.insert("event_mono_ms".into(), json!(r.event_mono_ms));
    let (event, epoch, active) = match &r.payload {
        Payload::Epoch { epoch } => ("source.epoch", Some(*epoch), None),
        Payload::Ready { epoch } => ("source.ready", Some(*epoch), None),
        Payload::Gap { epoch, reason } => {
            fields.insert("reason".into(), json!(reason));
            ("source.gap", Some(*epoch), None)
        }
        Payload::Stopping => ("source.stopping", None, None),
        Payload::Accepted(active) => ("activity.accepted", Some(active.epoch), Some(active)),
        Payload::Metadata { active, accepted_seq } => {
            fields.insert("accepted_seq".into(), json!(accepted_seq));
            ("activity.metadata", Some(active.epoch), Some(active))
        }
    };
    fields.insert("event".into(), json!(event));
    if let Some(epoch) = epoch {
        fields.insert("epoch".into(), json!(epoch));
    }
    if let Some(active) = active {
        fields.insert("state".into(), json!(if active.window.is_some() { "active" } else { "none" }));
        fields.insert("since_mono_ms".into(), json!(active.since.as_millis()));
        fields.insert("last_observed_mono_ms".into(), json!(active.last_observed.as_millis()));
        fields.insert("dwell_ms".into(), json!(r.dwell_ms));
        if let Some(w) = &active.window {
            fields.insert("window_id".into(), json!(w.id.as_str()));
            fields.insert("app_id".into(), json!(w.app_id));
            fields.insert("title".into(), json!(w.title));
            fields.insert("workspace_id".into(), json!(w.workspace.as_str()));
            fields.insert("scratchpad".into(), json!(w.scratchpad));
        }
    }
    json!({ "fields": Value::Object(fields), "delivery": entry.delivery })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_alias_and_invalid_retry_delay() {
        let args = Args::try_parse_from(["activity"]).unwrap();
        assert_eq!(args.dwell_ms, 5000);
        assert_eq!(Args::try_parse_from(["activity", "--output", "ndjson"]).unwrap().output, Output::Jsonl);
        assert!(Args::try_parse_from(["activity", "--reconnect-delay-ms", "0"]).is_err());
        assert_eq!(
            Args::try_parse_from(["activity", "--socket", "/explicit"]).unwrap().socket,
            Some("/explicit".into())
        );
    }

    #[test]
    fn dump_arguments_and_json_lines_fields() {
        use umbriel_activity::{
            Payload, Record,
            history::{Cursor, Delivery, Entry, Feed},
        };
        let args = Args::try_parse_from(["activity", "dump", "--tail", "3"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Dump(history_cli::Dump {
                range: history_cli::Range { tail: Some(3), .. },
                follow: false
            }))
        ));
        assert!(matches!(
            Args::try_parse_from(["activity", "dump", "-f"]).unwrap().command,
            Some(Command::Dump(history_cli::Dump { follow: true, .. }))
        ));
        assert!(Args::try_parse_from(["activity", "dump", "--follow", "--until", "2026-10-31T00:00:00Z"]).is_err());
        let feed = Feed::default();
        let entry = Entry {
            record: Record {
                run_id: "run".into(),
                seq: 1,
                event_time: "time".into(),
                event_mono_ms: 2,
                dwell_ms: 0,
                payload: Payload::Gap {
                    epoch: 3,
                    reason: "line\nnext".into(),
                },
            },
            delivery: Delivery {
                cursor: Cursor {
                    feed: feed.clone(),
                    opaque: "cursor".into(),
                },
                realtime_usec: 42,
                uid: feed.uid,
                user_unit: feed.user_unit.clone(),
                boot_id: None,
                invocation_id: None,
            },
        };
        let text = serde_json::to_string(&dump_record(&entry)).unwrap();
        assert_eq!(text.lines().count(), 1);
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["fields"]["reason"], "line\nnext");
        assert_eq!(value["fields"]["event"], "source.gap");
        assert_eq!(value["delivery"]["realtime_usec"], 42);
    }

    #[test]
    fn history_policy_is_explicit_and_journal_reemission_is_unavailable() {
        assert!(Args::try_parse_from(["activity", "history"]).is_ok());
        assert!(Args::try_parse_from(["activity", "history", "--follow", "--until", "2026-10-31T00:00:00Z"]).is_err());
        assert!(Args::try_parse_from(["activity", "history", "--tail", "0"]).is_err());
        assert!(Args::try_parse_from(["activity", "history", "--all", "--tail", "1"]).is_err());
        assert!(Args::try_parse_from(["activity", "history", "--after", "{}", "--since", "2026-10-31T00:00:00Z"]).is_err());
        assert!(Args::try_parse_from(["activity", "history", "--output", "journal"]).is_err());
        assert!(Args::try_parse_from(["activity", "history", "--output", "ndjson"]).is_ok());
    }
}
