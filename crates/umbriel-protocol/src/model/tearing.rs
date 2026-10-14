use serde::{Deserialize, Serialize};

use super::{PresentationHint, WindowId, required_nullable};

/// Complete `tearing` query result. It has no matching event family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tearing {
    pub protocol: bool,
    pub outputs: Vec<TearingOutput>,
    pub surfaces: Vec<TearingSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TearingOutput {
    pub name: String,
    pub allowed: bool,
    pub requested: bool,
    pub last_commit_tearing: bool,
    pub fallback_reason: String,
    /// Raw wlroots presentation flags; unknown bits are retained.
    #[serde(deserialize_with = "required_nullable")]
    pub last_presentation_flags: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_presentation_presented: Option<bool>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_presentation_vsync: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TearingSurface {
    pub id: WindowId,
    pub app_id: String,
    pub title: String,
    /// Empty if the surface has no current output.
    pub output: String,
    pub hint: PresentationHint,
    pub fullscreen: bool,
    pub eligible: bool,
    /// Null inherits policy, true forces tearing, false denies it.
    #[serde(deserialize_with = "required_nullable")]
    pub rule_override: Option<bool>,
}
