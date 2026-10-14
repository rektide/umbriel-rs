//! Typed compositor action grammar, separate from JSON request escaping.
//!
//! Arguments are validated when constructed, not on each send. Runtime target
//! existence and applicability remain the compositor's responsibility.
use std::borrow::Cow;

use crate::request::Request;

pub mod catalog;
pub mod command;
pub mod control;
pub mod extent;
pub mod no_argument;
pub mod target;
pub mod workspace;

pub use control::{LayoutSelection, QuitConfirmation};
pub use extent::{ExtentDelta, ExtentFraction};
pub use workspace::{WorkspaceSelector, WorkspaceTarget};

/// Local syntax errors only; no compositor state is consulted. `Eq` is not
/// implemented because fraction and delta variants carry the rejected `f64`.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("{argument} must not be empty")]
    Empty { argument: &'static str },
    #[error("workspace index {index} is outside 1..=64")]
    WorkspaceIndex { index: usize },
    #[error("workspace name cannot be represented by the compositor grammar")]
    UnrepresentableWorkspaceName,
    #[error("workspace output must not contain a slash")]
    InvalidWorkspaceOutput,
    #[error("fraction {value} must be finite within 0.1..=1.0")]
    InvalidFraction { value: f64 },
    #[error("delta {value} must be finite, nonzero, and at most 0.9 in magnitude")]
    InvalidFractionDelta { value: f64 },
    #[error("submap name may not be \"disable\" or contain ']'")]
    InvalidSubmapName,
}

/// Immutable action text, reusable through the ordinary request encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action<'a> {
    text: Cow<'a, str>,
}

impl<'a> Action<'a> {
    pub const fn overview_toggle() -> Self {
        Self {
            text: Cow::Borrowed(catalog::OVERVIEW_TOGGLE.name),
        }
    }

    /// Deliberately bypass local action validation, including for future actions.
    /// JSON escaping and the final request-size limit still apply.
    pub fn raw(text: impl Into<Cow<'a, str>>) -> Self {
        Self { text: text.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Borrow already prepared text; no intermediate action allocation per send.
    pub fn request(&self) -> Request<'_> {
        Request::Msg(self.as_str())
    }
}

impl Action<'static> {
    // Families format validated arguments once. JSON escaping remains entirely
    // in Request/codec; do not pre-escape text here.
    fn parameterized(spec: catalog::Spec, argument: impl std::fmt::Display) -> Self {
        Self {
            text: Cow::Owned(format!("{}:{argument}", spec.name)),
        }
    }

    // Optional families use the bare borrowed name for absent arguments; no
    // colon with an empty suffix is valid on the wire.
    fn bare(spec: catalog::Spec) -> Self {
        Self {
            text: Cow::Borrowed(spec.name),
        }
    }
}

impl From<no_argument::NoArgument> for Action<'static> {
    fn from(action: no_argument::NoArgument) -> Self {
        Self {
            text: Cow::Borrowed(action.as_str()),
        }
    }
}
