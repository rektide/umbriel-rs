use clap::Parser;
use std::{error::Error, path::PathBuf, time::Duration};
use umbriel_activity::{
    Recorder, emit,
    output::{self, Output},
};
use umbriel_client::{Discovery, Endpoint, Reconnect, SubscriptionOptions};
use umbriel_protocol::envelope::Family;
use umbriel_tracking::activity::Activity;

#[derive(Debug, Parser)]
#[command(version, about = "Observe stable Umbriel activity through tracing layers")]
struct Args {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let dispatch = output::dispatch(args.output)?;
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
            event = activity.next(&mut observer) => match event {
                Ok(event) => emit(&recorder.record(event, activity.elapsed()), &dispatch),
                Err(error) => {
                    if let Some(record) = recorder.terminal(error.to_string(), activity.elapsed()) { emit(&record, &dispatch); }
                    break Err(Box::<dyn Error + Send + Sync>::from(error));
                }
            }
        }
    };
    emit(&recorder.stopping(activity.elapsed()), &dispatch);
    tokio::time::timeout(Duration::from_millis(args.shutdown_timeout_ms), source.shutdown()).await?;
    result
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
}
