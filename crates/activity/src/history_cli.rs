//! Owning read/export worker. Cancellation never waits indefinitely for native
//! disk I/O or a full stdout pipe; the process may detach a stuck worker on exit.
use clap::Args;
use std::{
    error::Error,
    io::{self, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use umbriel_activity::{
    emit,
    history::{Cursor, Feed, Limits, Reader, Start},
    output::{Export, export_dispatch},
};

type Failure = Box<dyn Error + Send + Sync>;

#[derive(Debug, Args)]
pub(super) struct Range {
    /// Initial last N selected records (not just acceptances).
    #[arg(long, conflicts_with_all = ["all", "since", "after"], value_parser = positive)]
    pub tail: Option<usize>,
    /// Start at the retained readable head (history otherwise defaults to tail 50).
    #[arg(long, conflicts_with_all = ["tail", "since", "after"])]
    all: bool,
    /// Inclusive journal receive time, RFC3339 with offset (not application time).
    #[arg(long, value_parser = timestamp, conflicts_with_all = ["tail", "all", "after"])]
    since: Option<u64>,
    /// Exclusive journal receive time, RFC3339 with offset. Not usable with follow.
    #[arg(long, value_parser = timestamp)]
    until: Option<u64>,
    /// Exclusive resume cursor as the complete selection-bound JSON token.
    #[arg(long, conflicts_with_all = ["tail", "all", "since"])]
    after: Option<String>,
    /// Print the last completely exported range's cursor JSON to stderr.
    #[arg(long)]
    show_cursor: bool,
    /// Select a single producer run (UUID).
    #[arg(long)]
    run: Option<String>,
    /// Select a boot: current, or a 32-hex-digit boot ID.
    #[arg(long)]
    boot: Option<String>,
    /// Also include foreground journal writers from this UID; no unit authentication.
    #[arg(long)]
    include_foreground: bool,
    /// Maximum entries per captured range; overflow is failure, not truncation.
    #[arg(long, default_value = "10000", value_parser = positive)]
    max_entries: usize,
    /// Maximum copied native field bytes per captured range, including metadata.
    #[arg(long, default_value = "33554432", value_parser = positive)]
    max_bytes: usize,
    /// Maximum native bytes in one record, including journal metadata/MESSAGE.
    #[arg(long, default_value = "1048576", value_parser = positive)]
    max_record_bytes: usize,
    /// Cooperative native-read deadline per range; cannot interrupt a native call.
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..))]
    timeout_secs: u64,
}
#[derive(Debug, Args)]
pub(super) struct History {
    #[command(flatten)]
    range: Range,
    /// After the finite initial range, wait for exact-cursor suffixes.
    #[arg(short, long, conflicts_with = "until")]
    follow: bool,
    #[arg(long, value_enum, default_value_t = Export::Human)]
    output: Export,
}
#[derive(Debug, Args)]
pub(super) struct Dump {
    #[command(flatten)]
    pub range: Range,
    /// After the initial range, continue exporting new records.
    #[arg(short, long, conflicts_with = "until")]
    pub follow: bool,
}
fn positive(value: &str) -> Result<usize, String> {
    value.parse().ok().filter(|v| *v > 0).ok_or_else(|| "expected a positive integer".into())
}
fn timestamp(value: &str) -> Result<u64, String> {
    let time: jiff::Timestamp = value.parse().map_err(|e| format!("invalid RFC3339 time: {e}"))?;
    u64::try_from(time.as_microsecond()).map_err(|_| "time must not precede the Unix epoch".into())
}

impl Range {
    fn selection(&self) -> Result<Feed, Failure> {
        let mut feed = Feed::default();
        if self.include_foreground {
            feed.user_unit = None;
        }
        if let Some(run) = &self.run {
            // Preserve the exact producer spelling as a selector.
            uuid::Uuid::parse_str(run)?;
            feed.run_id = Some(run.clone());
        }
        if let Some(boot) = &self.boot {
            let boot = if boot == "current" {
                std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
            } else {
                boot.clone()
            };
            let boot = boot.trim().replace('-', "").to_ascii_lowercase();
            if boot.len() != 32 || !boot.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("boot must be current or a boot ID".into());
            }
            feed.boot_id = Some(boot);
        }
        Ok(feed)
    }
}

pub(super) async fn history(args: History, shutdown: Duration) -> Result<(), Failure> {
    run(args.range, args.follow, Some(args.output), shutdown).await
}
pub(super) async fn dump(args: Dump, shutdown: Duration) -> Result<(), Failure> {
    run(args.range, args.follow, None, shutdown).await
}

async fn run(range: Range, follow: bool, output: Option<Export>, shutdown: Duration) -> Result<(), Failure> {
    let feed = range.selection()?;
    let after: Option<Cursor> = range.after.as_deref().map(serde_json::from_str).transpose()?;
    if after.as_ref().is_some_and(|c| c.feed != feed) {
        return Err(umbriel_activity::history::Error::CursorFeedMismatch.into());
    }
    let start = match (range.since, range.tail) {
        (Some(time), _) => Start::Since(time),
        (_, Some(count)) => Start::Tail(count),
        _ if range.all || output.is_none() || after.is_some() => Start::Head,
        _ => Start::Tail(50),
    };
    let limits = Limits {
        max_entries: range.max_entries,
        max_bytes: range.max_bytes,
        max_record_bytes: range.max_record_bytes,
        max_elapsed: Duration::from_secs(range.timeout_secs),
    };
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    let (done, mut completed) = tokio::sync::oneshot::channel();
    std::thread::Builder::new().name("activity-history".into()).spawn(move || {
        let result = read_export(feed, start, after, range.until, limits, follow, output, range.show_cursor, &stop);
        let _ = done.send(result);
    })?;
    tokio::select! {
        biased;
        _ = terminate.recv() => {},
        _ = interrupt.recv() => {},
        result = &mut completed => return result?,
    }
    cancelled.store(true, Ordering::Relaxed);
    tokio::time::timeout(shutdown, completed).await???;
    Ok(())
}

#[derive(Clone, Default)]
struct CheckedWriter(Arc<Mutex<Option<io::Error>>>);
impl Write for CheckedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let result = io::stdout().write(bytes);
        if let Err(error) = &result {
            *self.0.lock().expect("output error mutex") = Some(io::Error::new(error.kind(), error.to_string()));
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()?;
        self.0.lock().expect("output error mutex").take().map_or(Ok(()), Err)
    }
}

#[allow(clippy::too_many_arguments)]
fn read_export(
    feed: Feed,
    mut start: Start,
    mut after: Option<Cursor>,
    until: Option<u64>,
    limits: Limits,
    follow: bool,
    output: Option<Export>,
    show_cursor: bool,
    cancelled: &AtomicBool,
) -> Result<(), Failure> {
    let mut reader = Reader::open(feed)?;
    let mut writer = CheckedWriter::default();
    let shared = writer.clone();
    let dispatch = output.map(|output| export_dispatch(output, move || shared.clone()));
    let is_cancelled = || cancelled.load(Ordering::Relaxed);
    loop {
        if is_cancelled() {
            return Ok(());
        }
        let replay = match reader.read(start, after.as_ref(), until, limits, &is_cancelled) {
            Err(umbriel_activity::history::Error::Cancelled) => return Ok(()),
            result => result?,
        };
        for entry in &replay.entries {
            // Complete the current range before publishing its resume token.
            // Cancellation in the middle intentionally leaves the prior token:
            // replay can duplicate output, but must not claim an unexported cut.
            if is_cancelled() {
                return Ok(());
            }
            if let Some(dispatch) = &dispatch {
                emit(&entry.record, dispatch);
            } else {
                serde_json::to_writer(&mut writer, &super::dump_record(entry))?;
                writeln!(writer)?;
            }
            writer.flush()?;
        }
        if let Some(upper) = replay.upper {
            // An empty suffix must not repeatedly print the same token.
            if show_cursor && (!replay.entries.is_empty() || after.is_none()) {
                eprintln!("{}", serde_json::to_string(&upper)?);
            }
            after = Some(upper);
        }
        if !follow {
            return Ok(());
        }
        // Tail is only the initial policy. If the initial feed was empty there
        // is no lower cursor yet: the first append must be read from head, not
        // silently truncated to another tail window.
        if matches!(start, Start::Tail(_)) {
            start = Start::Head;
        }
        if reader.wait()? && !is_cancelled() {
            eprintln!("journal files changed; revalidating the exact saved cursor before continuing");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn time_bounds_and_positive_budgets() {
        assert_eq!(timestamp("1970-01-01T00:00:01Z").unwrap(), 1_000_000);
        assert!(timestamp("yesterday").is_err());
        assert!(timestamp("1969-12-31T23:59:59Z").is_err());
        assert!(positive("0").is_err());
    }
}
