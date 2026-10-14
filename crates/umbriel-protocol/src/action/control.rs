//! Layout selection and session quit: closed operation vocabularies.
//!
//! `workspace-set-layout` takes one of four lowercase words, where `toggle`
//! is an operation of the action rather than an observed layout mode, so this
//! selection type is deliberately separate from `model`'s `LayoutMode`.
//! `session-quit` takes an explicit confirmation choice, not arbitrary text.
use std::fmt;

use super::{Action, catalog};

/// The `workspace-set-layout` operation set. `Toggle` cycles the active
/// workspace's layout; it is not a fourth layout mode a workspace can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutSelection {
    Scrolling,
    Dwindle,
    Master,
    Toggle,
}

impl LayoutSelection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scrolling => "scrolling",
            Self::Dwindle => "dwindle",
            Self::Master => "master",
            Self::Toggle => "toggle",
        }
    }
}

impl fmt::Display for LayoutSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether `session-quit` confirms before quitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuitConfirmation {
    /// The bare action; the compositor confirms before quitting.
    ConfirmFirst,
    /// The `skip-confirmation` argument; quit without confirming.
    Skip,
}

impl Action<'static> {
    pub fn workspace_set_layout(selection: LayoutSelection) -> Self {
        Self::parameterized(catalog::WORKSPACE_SET_LAYOUT, selection)
    }

    pub fn session_quit(confirmation: QuitConfirmation) -> Self {
        match confirmation {
            QuitConfirmation::ConfirmFirst => Self::bare(catalog::SESSION_QUIT),
            QuitConfirmation::Skip => {
                Self::parameterized(catalog::SESSION_QUIT, "skip-confirmation")
            }
        }
    }
}
