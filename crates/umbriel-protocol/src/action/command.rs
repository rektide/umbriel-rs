//! Spawn commands and submap layer names: opaque text after the first colon.
//!
//! The compositor preserves everything after the first colon verbatim for
//! `spawn`; `submap` applies the same shape with a reserved-word and bracket
//! restriction. Whether the command runs or the layer exists is runtime.
use super::{Action, ValidationError, catalog};

impl Action<'static> {
    /// Any nonempty command text, preserved verbatim after the first colon,
    /// including further colons, whitespace, and quoting.
    pub fn spawn(command: &str) -> Result<Self, ValidationError> {
        if command.is_empty() {
            return Err(ValidationError::Empty {
                argument: "spawn command",
            });
        }
        Ok(Self::parameterized(catalog::SPAWN, command))
    }

    /// A submap layer name: nonempty, not the reserved `disable`, and without
    /// `]`. `reset` is ordinary valid text that pops the current layer.
    pub fn submap(name: &str) -> Result<Self, ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::Empty {
                argument: "submap name",
            });
        }
        if name == "disable" || name.contains(']') {
            return Err(ValidationError::InvalidSubmapName);
        }
        Ok(Self::parameterized(catalog::SUBMAP, name))
    }
}
