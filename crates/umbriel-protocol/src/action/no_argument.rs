//! Bare actions: no colon and no argument.
use super::catalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NoArgument {
    OverviewToggle,
}

impl NoArgument {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OverviewToggle => catalog::OVERVIEW_TOGGLE.name,
        }
    }
}
