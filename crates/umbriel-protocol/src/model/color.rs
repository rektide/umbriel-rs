use serde::{Deserialize, Serialize};

use super::{HdrMode, Primaries, TransferFunction, WindowId, required_nullable};

/// Complete `color` query result. It has no matching event family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub color_manager: bool,
    pub renderer: ColorRenderer,
    pub outputs: Vec<ColorOutput>,
    pub surfaces: Vec<ColorSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRenderer {
    pub input_color_transform: bool,
    pub output_color_transform: bool,
    pub timeline: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorOutput {
    pub name: String,
    pub hdr_mode: HdrMode,
    pub hdr_requested: bool,
    pub hdr_active: bool,
    pub fallback_reason: String,
    /// FourCC printable spelling or `invalid`; not a fixed enum vocabulary.
    pub render_format: String,
    pub transfer_function: TransferFunction,
    pub primaries: Primaries,
    pub sdr_white: f64,
    pub supported_transfer_functions: Vec<TransferFunction>,
    pub supported_primaries: Vec<Primaries>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorSurface {
    pub id: WindowId,
    pub app_id: String,
    pub title: String,
    pub transfer_function: TransferFunction,
    pub primaries: Primaries,
    /// Zero means unset in the pinned compositor.
    pub max_cll: u32,
    pub max_fall: u32,
    #[serde(deserialize_with = "required_nullable")]
    pub mastering_display_primaries: Option<MasteringDisplayPrimaries>,
    #[serde(deserialize_with = "required_nullable")]
    pub mastering_luminance: Option<MasteringLuminance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringDisplayPrimaries {
    pub red: Chromaticity,
    pub green: Chromaticity,
    pub blue: Chromaticity,
    pub white: Chromaticity,
}

/// CIE 1931 xy coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chromaticity {
    pub x: f64,
    pub y: f64,
}

/// Mastering luminance in cd/m².
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasteringLuminance {
    pub min: f64,
    pub max: f64,
}
