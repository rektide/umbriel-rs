//! Explicitly enabled, in-memory Kopacetic materialization of live Umbriel state.
#![forbid(unsafe_code)]

#[cfg(feature = "active-cursor")]
mod cursor;
#[cfg(feature = "active-cursor")]
pub use cursor::*;
