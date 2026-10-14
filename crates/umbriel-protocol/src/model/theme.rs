use serde::{Deserialize, Serialize};

/// Source color spellings (`#RRGGBBAA` today), retained without reinterpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub background: String,
    pub text_primary: String,
    pub text_muted: String,
    pub accent_primary: String,
    pub accent_secondary: String,
    pub warning: String,
    pub error: String,
    pub insert_hint: String,
    pub backdrop: String,
    pub shadow: String,
    pub border: BorderColors,
    pub overview: OverviewColors,
    /// Logical pixels.
    pub corner_radius: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderColors {
    pub focused: String,
    pub unfocused: String,
    pub scratchpad_focused: String,
    pub scratchpad_unfocused: String,
    pub outer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewColors {
    pub background_tint: String,
    pub workspace_background: String,
    pub badge: String,
}
