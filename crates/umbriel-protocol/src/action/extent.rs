//! Extent fractions and signed extent deltas in the compositor's decimal range.
//!
//! Validation happens on the `f64` input before any text is formatted, and the
//! emitted spelling is the shortest round-trip decimal, which the compositor's
//! `from_chars` branch re-parses to the same value. Whether a window actually
//! has that extent is a runtime decision.
use std::fmt;

use super::{Action, ValidationError, catalog};

/// A finite fraction within `0.1..=1.0`, for setting an extent outright.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtentFraction(f64);

impl ExtentFraction {
    /// Reject non-finite values and anything outside the accepted range before
    /// formatting; `NaN` and infinities never become action text.
    pub fn new(fraction: f64) -> Result<Self, ValidationError> {
        if !fraction.is_finite() || !(0.1..=1.0).contains(&fraction) {
            return Err(ValidationError::InvalidFraction { value: fraction });
        }
        Ok(Self(fraction))
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

/// A finite, nonzero extent delta with magnitude at most `0.9`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtentDelta(f64);

impl ExtentDelta {
    /// Reject non-finite values, zero (a no-op), and magnitudes above the cap
    /// before formatting. `-0.0` equals `0.0` and is rejected with it.
    pub fn new(delta: f64) -> Result<Self, ValidationError> {
        if !delta.is_finite() || delta == 0.0 || delta.abs() > 0.9 {
            return Err(ValidationError::InvalidFractionDelta { value: delta });
        }
        Ok(Self(delta))
    }

    pub const fn value(self) -> f64 {
        self.0
    }
}

// Display never emits exponent notation or a leading '+', so the compositor's
// optional-plus delta handling and fraction rejection of '+' stay untouched.
impl fmt::Display for ExtentFraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ExtentDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Action<'static> {
    pub fn window_set_primary_extent(fraction: ExtentFraction) -> Self {
        Self::parameterized(catalog::WINDOW_SET_PRIMARY_EXTENT, fraction)
    }

    pub fn window_set_secondary_extent(fraction: ExtentFraction) -> Self {
        Self::parameterized(catalog::WINDOW_SET_SECONDARY_EXTENT, fraction)
    }

    pub fn window_modify_primary_extent(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_PRIMARY_EXTENT, delta)
    }

    pub fn window_modify_secondary_extent(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_SECONDARY_EXTENT, delta)
    }

    pub fn window_modify_width_left(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_WIDTH_LEFT, delta)
    }

    pub fn window_modify_width_right(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_WIDTH_RIGHT, delta)
    }

    pub fn window_modify_height_up(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_HEIGHT_UP, delta)
    }

    pub fn window_modify_height_down(delta: ExtentDelta) -> Self {
        Self::parameterized(catalog::WINDOW_MODIFY_HEIGHT_DOWN, delta)
    }
}
