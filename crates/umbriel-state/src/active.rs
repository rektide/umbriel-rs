//! Stable active identity, never workspace-local remembered focus.
use std::{fmt, time::Duration};
use umbriel_protocol::model::{Window, WindowId, WorkspaceId};

/// The only window metadata retained across frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWindow {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub workspace: WorkspaceId,
    pub scratchpad: String,
}

impl ActiveWindow {
    fn from_window(window: &Window) -> Self {
        Self {
            id: window.id.clone(),
            title: window.title.clone(),
            app_id: window.app_id.clone(),
            workspace: window.workspace.clone(),
            scratchpad: window.scratchpad.clone(),
        }
    }

    fn refresh(&mut self, window: &Window) {
        self.title.clone_from(&window.title);
        self.app_id.clone_from(&window.app_id);
        // The opaque ID wrapper derives Clone; its clone_from does not promise
        // reuse of the inner String allocation. Leave unchanged IDs untouched.
        if self.workspace != window.workspace {
            self.workspace.clone_from(&window.workspace);
        }
        self.scratchpad.clone_from(&window.scratchpad);
    }
}

/// One qualifying episode; `None` is a valid stable no-active observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableActive {
    pub epoch: u64,
    pub since: Duration,
    pub last_observed: Duration,
    pub emitted_at: Duration,
    pub window: Option<ActiveWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TimeWentBackwards,
    DeadlineOverflow,
    MultipleActive,
    EmptyActiveId,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid stable-active input: {self:?}")
    }
}
impl std::error::Error for Error {}

struct Candidate {
    window: Option<ActiveWindow>,
    since: Duration,
    last_observed: Duration,
    deadline: Option<Duration>,
}

/// A deterministic reducer. All calls use nondecreasing monotonic time, including
/// across epochs; equal times are allowed. Invalid input cancels continuity.
/// Input must be processed before `poll` at a simultaneous deadline.
pub struct StableActiveReducer {
    dwell: Duration,
    epoch: Option<u64>,
    last_time: Option<Duration>,
    candidate: Option<Candidate>,
}

impl Default for StableActiveReducer {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

impl StableActiveReducer {
    pub fn new(dwell: Duration) -> Self {
        Self {
            dwell,
            epoch: None,
            last_time: None,
            candidate: None,
        }
    }

    /// Gap, epoch announcement, decode failure, or independent observer loss.
    /// Never converts loss into a valid no-active episode.
    pub fn invalidate(&mut self) {
        self.candidate = None;
        self.epoch = None;
    }

    pub fn deadline(&self) -> Option<Duration> {
        self.candidate.as_ref().and_then(|c| c.deadline)
    }

    fn time(&mut self, now: Duration) -> Result<(), Error> {
        if self.last_time.is_some_and(|last| now < last) {
            self.invalidate();
            return Err(Error::TimeWentBackwards);
        }
        self.last_time = Some(now);
        Ok(())
    }

    /// A complete valid windows census. First data starts its dwell *now*.
    /// Same-ID metadata refreshes do not change the deadline, even after emission.
    pub fn observe(&mut self, epoch: u64, now: Duration, windows: &[Window]) -> Result<(), Error> {
        self.observe_changes(epoch, now, windows).map(|_| ())
    }

    /// Return full metadata only when the same already-accepted candidate changes.
    /// Geometry and pre-acceptance churn do not emit; invalidation ends eligibility.
    pub fn observe_changes<'a>(&mut self, epoch: u64, now: Duration, windows: impl IntoIterator<Item = &'a Window>) -> Result<Option<StableActive>, Error> {
        self.time(now)?;
        if self.epoch != Some(epoch) {
            self.invalidate();
            self.epoch = Some(epoch);
        }
        let mut active = windows.into_iter().filter(|window| window.active);
        let window = active.next();
        let error = if active.next().is_some() {
            Some(Error::MultipleActive)
        } else if window.is_some_and(|w| w.id.as_str().is_empty()) {
            Some(Error::EmptyActiveId)
        } else {
            None
        };
        if let Some(error) = error {
            self.invalidate();
            return Err(error);
        }
        if let Some(candidate) = &mut self.candidate
            && candidate.window.as_ref().map(|w| &w.id) == window.map(|w| &w.id)
        {
            let mut changed = false;
            if let (Some(record), Some(window)) = (&mut candidate.window, window) {
                changed = record.title != window.title
                    || record.app_id != window.app_id
                    || record.workspace != window.workspace
                    || record.scratchpad != window.scratchpad;
                record.refresh(window);
            }
            candidate.last_observed = now;
            return Ok((changed && candidate.deadline.is_none()).then(|| StableActive {
                epoch,
                since: candidate.since,
                last_observed: now,
                emitted_at: now,
                window: candidate.window.clone(),
            }));
        }
        let Some(deadline) = now.checked_add(self.dwell) else {
            self.invalidate();
            return Err(Error::DeadlineOverflow);
        };
        self.candidate = Some(Candidate {
            window: window.map(ActiveWindow::from_window),
            since: now,
            last_observed: now,
            deadline: Some(deadline),
        });
        Ok(None)
    }

    /// Timer-only progress is sufficient. Emits at most once per episode.
    pub fn poll(&mut self, now: Duration) -> Result<Option<StableActive>, Error> {
        self.time(now)?;
        let Some(candidate) = &mut self.candidate else {
            return Ok(None);
        };
        if !candidate.deadline.is_some_and(|deadline| now >= deadline) {
            return Ok(None);
        }
        candidate.deadline = None;
        Ok(Some(StableActive {
            epoch: self.epoch.expect("candidate has an epoch"),
            since: candidate.since,
            last_observed: candidate.last_observed,
            emitted_at: now,
            window: candidate.window.clone(),
        }))
    }
}
