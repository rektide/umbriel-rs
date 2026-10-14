//! Complete family replacement and explicit source health.
use kubeless_view_memory::{Reader, Replacement, Writer, memory_view};
use std::{collections::BTreeMap, fmt, hash::Hash, sync::Arc, time::Duration};
use umbriel_protocol::{envelope::Family, model::*};

/// Local observation coordinates, not a compositor revision or resume token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub epoch: u64,
    pub position: u64,
    pub observed_at: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleReason {
    EpochChanged,
    Gap,
    InvalidSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Unseen,
    Current,
    Stale(StaleReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// Evidence for the installed rows, including a known empty relation.
    pub stamp: Option<Stamp>,
    /// Most recent attempted typed snapshot; may have been rejected.
    pub last_attempt: Option<Stamp>,
    pub health: Health,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            stamp: None,
            last_attempt: None,
            health: Health::Unseen,
        }
    }
}

impl Status {
    /// Readiness in the current uninterrupted source interval, not historical readiness.
    pub fn is_ready(&self) -> bool {
        self.health == Health::Current
    }
    fn stale(&mut self, reason: StaleReason) {
        if self.stamp.is_some() {
            self.health = Health::Stale(reason);
        }
    }
}

/// Owned coherent capture. Key iteration is lexical, NOT compositor row order.
#[derive(Debug, Clone)]
pub struct FamilySnapshot<K, V> {
    pub status: Status,
    pub entries: BTreeMap<K, Arc<V>>,
    /// Exact row order of the last installed source array; singleton families use `()`.
    pub source_order: Vec<K>,
}

struct Relation<K, V> {
    reader: Reader<K, V>,
    writer: Writer<K, V>,
    status: Status,
    order: Vec<K>,
}

impl<K: Clone + Eq + Hash + Ord, V> Relation<K, V> {
    fn new() -> Self {
        let (reader, writer) = memory_view();
        Self {
            reader,
            writer,
            status: Status::default(),
            order: Vec::new(),
        }
    }
    fn snapshot(&self) -> FamilySnapshot<K, V> {
        FamilySnapshot {
            status: self.status,
            entries: self.reader.snapshot().entries.into_iter().collect(),
            source_order: self.order.clone(),
        }
    }
    fn replace(&mut self, stamp: Stamp, rows: Vec<(K, V)>) -> Result<(), Error> {
        self.status.last_attempt = Some(stamp);
        let mut replacement = Replacement::with_capacity(rows.len());
        let mut order = Vec::with_capacity(rows.len());
        for (key, value) in rows {
            order.push(key.clone());
            if replacement.upsert(key, value).is_some() {
                self.status.stale(StaleReason::InvalidSnapshot);
                return Err(Error::DuplicateId);
            }
        }
        // No public memory reader/writer can race the accompanying metadata.
        self.writer.replace_affected(replacement);
        self.order = order;
        self.status.stamp = Some(stamp);
        self.status.health = Health::Current;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    EmptyId,
    DuplicateId,
    WrongEpoch,
    SourceGap,
    NonIncreasingPosition,
    TimeWentBackwards,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid inventory input: {self:?}")
    }
}
impl std::error::Error for Error {}

/// One serialized owner; share under an application lock if concurrent access is
/// needed. Snapshots outlive it. No automatic acquisition, resume, or reconciliation.
pub struct Inventory {
    epoch: u64,
    connected: bool,
    last: Option<Stamp>,
    windows: Relation<WindowId, Window>,
    workspaces: Relation<WorkspaceId, Workspace>,
    theme: Relation<(), Theme>,
    overview: Relation<(), Overview>,
    keyboard: Relation<(), KeyboardLayouts>,
    submap: Relation<(), Submap>,
}

impl Inventory {
    pub fn new(epoch: u64) -> Self {
        Self {
            epoch,
            connected: true,
            last: None,
            windows: Relation::new(),
            workspaces: Relation::new(),
            theme: Relation::new(),
            overview: Relation::new(),
            keyboard: Relation::new(),
            submap: Relation::new(),
        }
    }

    fn statuses(&mut self) -> [&mut Status; 6] {
        [
            &mut self.windows.status,
            &mut self.workspaces.status,
            &mut self.theme.status,
            &mut self.overview.status,
            &mut self.keyboard.status,
            &mut self.submap.status,
        ]
    }

    /// A reconnect starts a strictly newer local epoch. Retained rows are stale.
    pub fn begin_epoch(&mut self, epoch: u64) -> Result<(), Error> {
        if epoch <= self.epoch {
            return Err(Error::WrongEpoch);
        }
        self.epoch = epoch;
        self.connected = true;
        self.last = None;
        for status in self.statuses() {
            status.stale(StaleReason::EpochChanged);
        }
        Ok(())
    }

    /// Transport gap or observer lag. Only a new epoch can restore this interval.
    pub fn gap(&mut self) {
        self.connected = false;
        for status in self.statuses() {
            status.stale(StaleReason::Gap);
        }
    }

    /// A decode/validation failure affecting a family leaves its prior rows stale.
    /// A later complete valid snapshot can restore that family in the same epoch.
    pub fn invalid_snapshot(&mut self, family: Family) {
        self.status_mut(family).stale(StaleReason::InvalidSnapshot);
    }

    fn status_mut(&mut self, family: Family) -> &mut Status {
        match family {
            Family::Windows => &mut self.windows.status,
            Family::Workspaces => &mut self.workspaces.status,
            Family::Theme => &mut self.theme.status,
            Family::Overview => &mut self.overview.status,
            Family::KeyboardLayout => &mut self.keyboard.status,
            Family::Submap => &mut self.submap.status,
        }
    }

    pub fn status(&self, family: Family) -> Status {
        match family {
            Family::Windows => self.windows.status,
            Family::Workspaces => self.workspaces.status,
            Family::Theme => self.theme.status,
            Family::Overview => self.overview.status,
            Family::KeyboardLayout => self.keyboard.status,
            Family::Submap => self.submap.status,
        }
    }

    /// Fully typed protocol data, with stricter keyed-ID policy than raw decoding.
    /// IDs are opaque: only an empty keyed ID is unusable here. Whitespace and
    /// unusual punctuation remain valid identity text. Empty workspace references
    /// on window rows remain valid and are represented explicitly by the join.
    pub fn install(&mut self, stamp: Stamp, event: Event) -> Result<(), Error> {
        if stamp.epoch != self.epoch {
            return Err(Error::WrongEpoch);
        }
        if !self.connected {
            return Err(Error::SourceGap);
        }
        if let Some(last) = self.last {
            if stamp.position <= last.position {
                return Err(Error::NonIncreasingPosition);
            }
            if stamp.observed_at < last.observed_at {
                return Err(Error::TimeWentBackwards);
            }
        }
        self.last = Some(stamp);
        let family = event.family();
        self.status_mut(family).last_attempt = Some(stamp);
        let unusable = match &event {
            Event::Windows(rows) => rows.iter().any(|row| row.id.as_str().is_empty()),
            Event::Workspaces(rows) => rows.iter().any(|row| row.id.as_str().is_empty()),
            _ => false,
        };
        if unusable {
            self.invalid_snapshot(family);
            return Err(Error::EmptyId);
        }
        match event {
            Event::Windows(rows) => self
                .windows
                .replace(stamp, rows.into_iter().map(|r| (r.id.clone(), r)).collect()),
            Event::Workspaces(rows) => self
                .workspaces
                .replace(stamp, rows.into_iter().map(|r| (r.id.clone(), r)).collect()),
            Event::Theme(value) => self.theme.replace(stamp, vec![((), *value)]),
            Event::Overview(value) => self.overview.replace(stamp, vec![((), value)]),
            Event::KeyboardLayout(value) => self.keyboard.replace(stamp, vec![((), value)]),
            Event::Submap(value) => self.submap.replace(stamp, vec![((), value)]),
        }
    }

    /// One local capture of all families and their evidence; NOT source atomicity.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            epoch: self.epoch,
            connected: self.connected,
            windows: self.windows.snapshot(),
            workspaces: self.workspaces.snapshot(),
            theme: self.theme.snapshot(),
            overview: self.overview.snapshot(),
            keyboard: self.keyboard.snapshot(),
            submap: self.submap.snapshot(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub epoch: u64,
    pub connected: bool,
    pub windows: FamilySnapshot<WindowId, Window>,
    pub workspaces: FamilySnapshot<WorkspaceId, Workspace>,
    pub theme: FamilySnapshot<(), Theme>,
    pub overview: FamilySnapshot<(), Overview>,
    pub keyboard: FamilySnapshot<(), KeyboardLayouts>,
    pub submap: FamilySnapshot<(), Submap>,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMatch {
    Matched(Arc<Workspace>),
    /// Empty workspace reference while stored in a scratchpad.
    Scratchpad {
        name: String,
    },
    /// Empty reference with no named scratchpad; not silently discarded.
    NoWorkspace,
    /// May reflect family observation skew, unseen data, or stale rows.
    Unmatched(WorkspaceId),
}

#[derive(Debug, Clone)]
pub struct JoinedWindow {
    pub window: Arc<Window>,
    pub workspace: WorkspaceMatch,
}

#[derive(Debug, Clone)]
pub struct JoinedWindows {
    pub windows_status: Status,
    pub workspaces_status: Status,
    /// Source windows-array order, not keyed-map iteration order.
    pub rows: Vec<JoinedWindow>,
}

impl Snapshot {
    /// Latest observed cross-family join. BOTH family stamps/health travel with it.
    pub fn join_windows(&self) -> JoinedWindows {
        let rows = self
            .windows
            .source_order
            .iter()
            .map(|id| {
                let window = self.windows.entries[id].clone();
                let workspace = if window.workspace.as_str().is_empty() {
                    if window.scratchpad.is_empty() {
                        WorkspaceMatch::NoWorkspace
                    } else {
                        WorkspaceMatch::Scratchpad {
                            name: window.scratchpad.clone(),
                        }
                    }
                } else if let Some(workspace) = self.workspaces.entries.get(&window.workspace) {
                    WorkspaceMatch::Matched(workspace.clone())
                } else {
                    WorkspaceMatch::Unmatched(window.workspace.clone())
                };
                JoinedWindow { window, workspace }
            })
            .collect();
        JoinedWindows {
            windows_status: self.windows.status,
            workspaces_status: self.workspaces.status,
            rows,
        }
    }
}
