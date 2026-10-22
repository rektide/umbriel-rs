//! Conditional scrolling allocation extension. Supported by the paired native
//! layout-control build, not by the original Umbriel protocol baseline.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutWeights {
    pub rows: Vec<f64>,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecondaryAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutConstraints {
    pub min_width: i32,
    pub min_height: i32,
    pub max_width: i32,
    pub max_height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutMember {
    pub id: String,
    pub title_bar_height: i32,
    pub constraints: LayoutConstraints,
    pub fullscreen: bool,
    pub maximized: bool,
    pub maximized_to_edges: bool,
    pub floating: bool,
}

/// Compare-current-state basis, not a monotonic revision. Includes native weights
/// even when their changes are invisible after pixel rounding. ABA is not detected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutBasis {
    pub instance: String,
    pub workspace: String,
    pub column: i32,
    pub axis: SecondaryAxis,
    pub members: Vec<LayoutMember>,
    pub weights: LayoutWeights,
    pub area: LayoutBox,
    pub edge_pad: i32,
    pub gap: i32,
    pub width_fraction: f64,
    pub primary_extent: i32,
    pub tabbed: bool,
    pub supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutGeometry {
    pub id: String,
    pub target_tile: LayoutBox,
    pub target_content: LayoutBox,
    pub committed_content: LayoutBox,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutColumn {
    pub basis: LayoutBasis,
    pub geometry: Vec<LayoutGeometry>,
    /// Secondary allocation span after inter-row gaps, including edge-gap allocation.
    pub cross_span: i32,
    /// Tile allocation span after edge gaps; title decorations still consume it.
    pub row_budget: i32,
    pub top_gap: i32,
    pub bottom_gap: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutState {
    pub version: u32,
    pub instance: String,
    #[serde(deserialize_with = "super::required_nullable")]
    pub active: Option<String>,
    pub columns: Vec<LayoutColumn>,
    pub unsupported_workspaces: Vec<String>,
}

/// Whole-group native handling receipt; committed pixels may still be pending.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutAllocationResult {
    pub version: u32,
    pub previous_weights: LayoutWeights,
    pub accepted: LayoutColumn,
}

/// Submit the last accepted basis for subsequent writes/restoration, not the old
/// pre-emphasis basis. For restoration retain the original LayoutWeights separately.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayoutAllocation {
    pub target: String,
    pub expected: LayoutBasis,
    pub weights: LayoutWeights,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_active: Option<String>,
    /// Optional exact secondary content extents in member order. Native preview
    /// rejects the entire write if rounding/constraints cannot represent them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_content_extents: Option<Vec<i32>>,
}

impl LayoutAllocation {
    pub fn request(&self) -> crate::request::Request<'_> {
        crate::request::Request::LayoutAllocate(self)
    }
}

pub fn layout_state_request() -> crate::request::Request<'static> {
    crate::request::Request::Raw {
        cmd: "layout-state",
        arg: None,
    }
}
