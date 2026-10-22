//! Owned payloads for Umbriel at compositor revision `7666af0`.
//!
//! Query results describe the value inside `ok`, not the envelope. Known event
//! families are complete replacement snapshots, not deltas. Additional fields
//! are ignored; all source-supplied fields (including nullable keys) are required.
//! Caller-selected Serde projections through [`crate::codec`] remain available
//! when only a few fields or borrowed strings are needed.

use serde::Deserialize;

mod color;
mod event;
mod inventory;
mod layout;
mod tearing;
mod theme;
mod vocabulary;

pub use color::*;
pub use event::*;
pub use inventory::*;
pub use layout::*;
pub use tearing::*;
pub use theme::*;
pub use vocabulary::*;

/// `windows` query result and `windows` event snapshot.
pub type Windows = Vec<Window>;
/// `workspaces` query result and `workspaces` event snapshot.
pub type Workspaces = Vec<Workspace>;
/// `layers` query result (there is no layers event at the pinned revision).
pub type Layers = Vec<LayerSurface>;
/// `submap` query/event: null means the default keybinding context.
pub type Submap = Option<String>;
/// Successful `output-create` result: the assigned output name.
pub type OutputCreate = String;
/// Successful `output-destroy` result, JSON null.
pub type OutputDestroy = ();
/// Successful `msg` result, JSON null; not a rendering-completion signal.
pub type ActionResult = ();

// A deserialize_with field is required by Serde even when its target is Option.
// Ordinary Option fields would silently treat an absent key as null.
fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}
