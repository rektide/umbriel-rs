//! Output, scratchpad, and window-id targets as opaque nonempty text.
//!
//! These standalone arguments have no grammar of their own, unlike workspace
//! output qualifiers: slashes, colons, and quotes are ordinary characters
//! here. Absence (`None`) is represented by the bare action name, distinct
//! from any text and from rejected empty text. Whether the named target
//! exists is a runtime decision.
use super::{Action, ValidationError, catalog};

// Every optional family pairs one wire name with one argument kind; the
// repeated small match keeps each action's empty-argument label explicit.
impl Action<'static> {
    /// Power on one output by any nonempty name, or every output when `None`.
    pub fn dpms_on(output: Option<&str>) -> Result<Self, ValidationError> {
        match output {
            Some("") => Err(ValidationError::Empty { argument: "output" }),
            Some(output) => Ok(Self::parameterized(catalog::DPMS_ON, output)),
            None => Ok(Self::bare(catalog::DPMS_ON)),
        }
    }

    /// Power off one output by any nonempty name, or every output when `None`.
    pub fn dpms_off(output: Option<&str>) -> Result<Self, ValidationError> {
        match output {
            Some("") => Err(ValidationError::Empty { argument: "output" }),
            Some(output) => Ok(Self::parameterized(catalog::DPMS_OFF, output)),
            None => Ok(Self::bare(catalog::DPMS_OFF)),
        }
    }

    /// Cycle visible scratchpad windows, optionally on one named scratchpad.
    pub fn scratchpad_focus_next(scratchpad: Option<&str>) -> Result<Self, ValidationError> {
        match scratchpad {
            Some("") => Err(ValidationError::Empty {
                argument: "scratchpad name",
            }),
            Some(name) => Ok(Self::parameterized(catalog::SCRATCHPAD_FOCUS_NEXT, name)),
            None => Ok(Self::bare(catalog::SCRATCHPAD_FOCUS_NEXT)),
        }
    }

    /// Show or hide scratchpad windows, optionally on one named scratchpad.
    pub fn scratchpad_toggle(scratchpad: Option<&str>) -> Result<Self, ValidationError> {
        match scratchpad {
            Some("") => Err(ValidationError::Empty {
                argument: "scratchpad name",
            }),
            Some(name) => Ok(Self::parameterized(catalog::SCRATCHPAD_TOGGLE, name)),
            None => Ok(Self::bare(catalog::SCRATCHPAD_TOGGLE)),
        }
    }

    /// Move the focused window into a scratchpad, the default one when `None`.
    pub fn window_move_to_scratchpad(scratchpad: Option<&str>) -> Result<Self, ValidationError> {
        match scratchpad {
            Some("") => Err(ValidationError::Empty {
                argument: "scratchpad name",
            }),
            Some(name) => Ok(Self::parameterized(
                catalog::WINDOW_MOVE_TO_SCRATCHPAD,
                name,
            )),
            None => Ok(Self::bare(catalog::WINDOW_MOVE_TO_SCRATCHPAD)),
        }
    }

    /// Return a scratchpad window to its saved workspace, from the default
    /// scratchpad when `None`.
    pub fn window_restore_from_scratchpad(
        scratchpad: Option<&str>,
    ) -> Result<Self, ValidationError> {
        match scratchpad {
            Some("") => Err(ValidationError::Empty {
                argument: "scratchpad name",
            }),
            Some(name) => Ok(Self::parameterized(
                catalog::WINDOW_RESTORE_FROM_SCRATCHPAD,
                name,
            )),
            None => Ok(Self::bare(catalog::WINDOW_RESTORE_FROM_SCRATCHPAD)),
        }
    }

    /// Move the focused window to or from a scratchpad, the default one when
    /// `None`.
    pub fn window_toggle_scratchpad(scratchpad: Option<&str>) -> Result<Self, ValidationError> {
        match scratchpad {
            Some("") => Err(ValidationError::Empty {
                argument: "scratchpad name",
            }),
            Some(name) => Ok(Self::parameterized(catalog::WINDOW_TOGGLE_SCRATCHPAD, name)),
            None => Ok(Self::bare(catalog::WINDOW_TOGGLE_SCRATCHPAD)),
        }
    }

    /// Focus the window with the given opaque map-lifetime id.
    pub fn window_focus(window_id: &str) -> Result<Self, ValidationError> {
        if window_id.is_empty() {
            return Err(ValidationError::Empty {
                argument: "window id",
            });
        }
        Ok(Self::parameterized(catalog::WINDOW_FOCUS, window_id))
    }

    /// Focus the window with the given id and warp the cursor to it.
    pub fn window_focus_warp(window_id: &str) -> Result<Self, ValidationError> {
        if window_id.is_empty() {
            return Err(ValidationError::Empty {
                argument: "window id",
            });
        }
        Ok(Self::parameterized(catalog::WINDOW_FOCUS_WARP, window_id))
    }

    /// Close the focused window, or the window with the given id.
    pub fn window_close(window_id: Option<&str>) -> Result<Self, ValidationError> {
        match window_id {
            Some("") => Err(ValidationError::Empty {
                argument: "window id",
            }),
            Some(window_id) => Ok(Self::parameterized(catalog::WINDOW_CLOSE, window_id)),
            None => Ok(Self::bare(catalog::WINDOW_CLOSE)),
        }
    }

    /// Float or tile the focused window, or the window with the given id.
    pub fn window_toggle_floating(window_id: Option<&str>) -> Result<Self, ValidationError> {
        match window_id {
            Some("") => Err(ValidationError::Empty {
                argument: "window id",
            }),
            Some(window_id) => Ok(Self::parameterized(
                catalog::WINDOW_TOGGLE_FLOATING,
                window_id,
            )),
            None => Ok(Self::bare(catalog::WINDOW_TOGGLE_FLOATING)),
        }
    }
}
