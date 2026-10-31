//! Retained readable activity history, not a journal durability/completeness claim.
//! Native mechanics adapted locally from Cyclrs's exact-cut reader; no dependency
//! on its domain-specific event codec and no cross-repository extraction.
mod decode;
mod read;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use systemd::journal::{Journal, JournalWaitResult, OpenOptions};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feed {
    pub uid: u32,
    pub user_unit: Option<String>,
    pub run_id: Option<String>,
    pub boot_id: Option<String>,
}
impl Default for Feed {
    fn default() -> Self {
        Self {
            uid: rustix::process::geteuid().as_raw(),
            user_unit: Some("umbriel-activity.service".into()),
            run_id: None,
            boot_id: None,
        }
    }
}
impl Feed {
    fn validate(&self) -> Result<(), Error> {
        for value in [&self.user_unit, &self.run_id, &self.boot_id].into_iter().flatten() {
            if value.is_empty() || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
                return Err(Error::InvalidFeed);
            }
        }
        Ok(())
    }
}

/// Cursor identity includes the selection. Never compare opaque strings for order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub feed: Feed,
    pub opaque: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Delivery {
    pub cursor: Cursor,
    pub realtime_usec: u64,
    pub uid: u32,
    pub user_unit: Option<String>,
    pub boot_id: Option<String>,
    pub invocation_id: Option<String>,
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub record: crate::Record,
    pub delivery: Delivery,
}
#[derive(Debug)]
pub struct Replay {
    pub entries: Vec<Entry>,
    pub upper: Option<Cursor>,
}

/// Initial policy is explicit. Tail includes at most N recent selected records,
/// not N acceptances, and may begin with metadata whose acceptance was retained out.
#[derive(Debug, Clone, Copy)]
pub enum Start {
    Head,
    Tail(usize),
    Since(u64),
}
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_entries: usize,
    pub max_bytes: usize,
    pub max_record_bytes: usize,
    /// Cooperative between calls, not a hard deadline on libsystemd I/O.
    pub max_elapsed: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_bytes: 32 * 1024 * 1024,
            max_record_bytes: 1024 * 1024,
            max_elapsed: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native journal operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid journal selection")]
    InvalidFeed,
    #[error("invalid history range: since must precede until, and the after cursor must be earlier than until")]
    InvalidRange,
    #[error("cursor belongs to a different selection")]
    CursorFeedMismatch,
    #[error("{0} cursor is missing, malformed, expired, or outside this selection")]
    CursorUnavailable(&'static str),
    #[error("captured upper cut was not reached")]
    UpperUnreachable,
    #[error("journal files invalidated; restart explicitly after inspecting the gap")]
    Invalidated,
    #[error("history exhausted {0} budget; choose a smaller range or raise the explicit limit")]
    Budget(&'static str),
    #[error("malformed, repeated, missing, or unexpected scalar field {0}")]
    Field(String),
    #[error("unsupported activity schema or event kind")]
    Unsupported,
    #[error("record is outside the configured origin/selection")]
    OutsideFeed,
    #[error("conflicting content for the same run/sequence")]
    Conflict,
    #[error("history cancelled")]
    Cancelled,
}

/// Thread-local owning handle. Create and use on the history worker, never move
/// into an async executor. Read-only: exports cannot select a journal writer.
pub struct Reader {
    feed: Feed,
    journal: Journal,
}
impl Reader {
    pub fn open(feed: Feed) -> Result<Self, Error> {
        feed.validate()?;
        let mut journal = OpenOptions::default().local_only(true).open()?;
        journal.set_data_threshold(0)?;
        journal.match_add("TARGET", b"umbriel::activity".to_vec())?;
        journal.match_add("_UID", feed.uid.to_string().into_bytes())?;
        if let Some(unit) = &feed.user_unit {
            journal.match_add("_SYSTEMD_USER_UNIT", unit.as_bytes().to_vec())?;
        }
        if let Some(run) = &feed.run_id {
            journal.match_add("UMBRIEL_RUN_ID", run.as_bytes().to_vec())?;
        }
        if let Some(boot) = &feed.boot_id {
            journal.match_add("_BOOT_ID", boot.as_bytes().to_vec())?;
        }
        Ok(Self { feed, journal })
    }
    pub fn read(&mut self, start: Start, after: Option<&Cursor>, until: Option<u64>, limits: Limits, cancelled: &dyn Fn() -> bool) -> Result<Replay, Error> {
        read::bounded(&mut self.journal, &self.feed, start, after, until, limits, cancelled)
    }
    /// Short finite wait. `true` reports file-set invalidation, not necessarily
    /// lost data. The next read must revalidate its exact saved lower cursor.
    pub fn wait(&mut self) -> Result<bool, Error> {
        Ok(matches!(self.journal.wait(Some(Duration::from_millis(250)))?, JournalWaitResult::Invalidate))
    }
}

struct Budget<'a> {
    limits: Limits,
    started: Instant,
    entries: usize,
    bytes: usize,
    cancelled: &'a dyn Fn() -> bool,
}
impl Budget<'_> {
    fn check(&self) -> Result<(), Error> {
        if (self.cancelled)() {
            return Err(Error::Cancelled);
        }
        if self.started.elapsed() >= self.limits.max_elapsed {
            return Err(Error::Budget("elapsed time"));
        }
        Ok(())
    }
    fn entry(&mut self) -> Result<(), Error> {
        self.check()?;
        self.entries = self.entries.checked_add(1).ok_or(Error::Budget("entries"))?;
        if self.entries > self.limits.max_entries {
            return Err(Error::Budget("entries"));
        }
        Ok(())
    }
    fn bytes(&mut self, bytes: usize) -> Result<(), Error> {
        self.check()?;
        self.bytes = self.bytes.checked_add(bytes).ok_or(Error::Budget("bytes"))?;
        if self.bytes > self.limits.max_bytes {
            return Err(Error::Budget("bytes"));
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Raw {
    cursor: String,
    realtime_usec: u64,
    fields: Vec<(Vec<u8>, Vec<u8>)>,
}

// Private deterministic test seam, not a new generic journal framework.
trait Access {
    fn head(&mut self) -> Result<(), Error>;
    fn tail(&mut self) -> Result<(), Error>;
    fn seek(&mut self, cursor: &str) -> Result<(), Error>;
    fn seek_time(&mut self, usec: u64) -> Result<(), Error>;
    fn next(&mut self) -> Result<bool, Error>;
    fn previous(&mut self) -> Result<bool, Error>;
    fn test(&self, cursor: &str) -> Result<bool, Error>;
    fn cursor(&self) -> Result<String, Error>;
    fn time(&self) -> Result<u64, Error>;
    fn current(&mut self, budget: &mut Budget<'_>) -> Result<Raw, Error>;
    fn process(&mut self) -> Result<bool, Error>;
}
impl Access for Journal {
    fn head(&mut self) -> Result<(), Error> {
        Ok(self.seek_head()?)
    }
    fn tail(&mut self) -> Result<(), Error> {
        Ok(self.seek_tail()?)
    }
    fn seek(&mut self, cursor: &str) -> Result<(), Error> {
        Ok(self.seek_cursor(cursor)?)
    }
    fn seek_time(&mut self, usec: u64) -> Result<(), Error> {
        Ok(self.seek_realtime_usec(usec)?)
    }
    fn next(&mut self) -> Result<bool, Error> {
        Ok((**self).next()? != 0)
    }
    fn previous(&mut self) -> Result<bool, Error> {
        Ok((**self).previous()? != 0)
    }
    fn test(&self, cursor: &str) -> Result<bool, Error> {
        Ok(self.test_cursor(cursor)?)
    }
    fn cursor(&self) -> Result<String, Error> {
        Ok((**self).cursor()?)
    }
    fn time(&self) -> Result<u64, Error> {
        Ok(self.timestamp_usec()?)
    }
    fn current(&mut self, budget: &mut Budget<'_>) -> Result<Raw, Error> {
        let cursor = self.cursor()?;
        let realtime_usec = self.timestamp_usec()?;
        self.restart_data();
        let mut fields = Vec::new();
        let mut bytes = 0_usize;
        while let Some(field) = self.enumerate_data()? {
            let size = field.data().len();
            budget.bytes(size)?;
            bytes = bytes.checked_add(size).ok_or(Error::Budget("record bytes"))?;
            if bytes > budget.limits.max_record_bytes {
                return Err(Error::Budget("record bytes"));
            }
            if fields.len() >= 1024 {
                return Err(Error::Budget("fields per entry"));
            }
            fields.push((
                field.name().to_vec(),
                field.value().ok_or_else(|| Error::Field("native field".into()))?.to_vec(),
            ));
        }
        Ok(Raw { cursor, realtime_usec, fields })
    }
    fn process(&mut self) -> Result<bool, Error> {
        Ok(matches!((**self).process()?, JournalWaitResult::Invalidate))
    }
}

#[cfg(test)]
mod tests;
