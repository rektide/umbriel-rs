//! Typed compositor action grammar, separate from JSON request escaping.
//!
//! Arguments are validated when constructed, not on each send. Runtime target
//! existence and applicability remain the compositor's responsibility.
use std::borrow::Cow;

use crate::request::Request;

pub mod catalog;
pub mod no_argument;

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

impl From<no_argument::NoArgument> for Action<'static> {
    fn from(action: no_argument::NoArgument) -> Self {
        Self {
            text: Cow::Borrowed(action.as_str()),
        }
    }
}
