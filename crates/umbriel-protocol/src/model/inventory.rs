use serde::{Deserialize, Serialize};

use super::{ContentType, Layer, LayoutMode, WindowId, WorkspaceId};

/// A mapped toplevel. All text fields are non-null, but may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    pub id: WindowId,
    /// Empty when stored in a scratchpad; do not parse as output/serial.
    pub workspace: WorkspaceId,
    pub scratchpad: String,
    /// Seat-global activation; a snapshot may have no active window.
    pub active: bool,
    pub app_id: String,
    pub title: String,
    pub xdg_tag: String,
    pub content_type: ContentType,
    pub floating: bool,
    /// Workspace-local remembered focus, distinct from seat activation.
    pub focused: bool,
    pub urgent: bool,
    pub xwayland: bool,
    /// Owning Wayland process; -1 is unknown, not an unsigned sentinel.
    pub pid: i32,
    /// Layout target position (not necessarily the currently animated position).
    pub x: i32,
    pub y: i32,
    /// Committed client content size, not necessarily the target resize size.
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    /// Explicit persistent inventory membership, not inferred from the name.
    pub named: bool,
    /// One-based display position.
    pub index: i32,
    pub output: String,
    /// Active workspace of its output group.
    pub active: bool,
    /// Active workspace on the cursor-preferred output.
    pub focused: bool,
    pub occupied: bool,
    pub layout: LayoutMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerSurface {
    pub layer: Layer,
    pub namespace: String,
    /// Empty when there is no assigned output.
    pub output: String,
    pub mapped: bool,
}

/// Shared by the `keyboard-layouts` query and `keyboard_layout` event.
/// A compositor with no keyboard reports a query error or omits the event;
/// it does not send this object with null fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardLayouts {
    pub names: Vec<String>,
    pub current_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overview {
    pub open: bool,
}
