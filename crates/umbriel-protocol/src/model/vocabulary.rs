use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

identity!(
    WindowId,
    "Opaque map-lifetime window identity; empty is permitted by the wire."
);
identity!(
    WorkspaceId,
    "Opaque workspace identity; empty on scratchpad-stored windows."
);

// Keep the wire spelling rather than Serde's lossy `other` unit variant.
macro_rules! string_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant,)+
            /// A spelling added by a newer compositor (including literal `unknown`).
            Unknown(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Unknown(value) => value,
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                match value.as_str() {
                    $($wire => Self::$variant,)+
                    _ => Self::Unknown(value),
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::from(value.to_owned())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                String::deserialize(deserializer).map(Self::from)
            }
        }
    };
}

string_enum!(ContentType { None => "none", Photo => "photo", Video => "video", Game => "game" });
string_enum!(LayoutMode { Scrolling => "scrolling", Dwindle => "dwindle", Master => "master" });
string_enum!(Layer { Background => "background", Bottom => "bottom", Top => "top", Overlay => "overlay" });
string_enum!(HdrMode { Off => "off", On => "on", Auto => "auto", Fullscreen => "fullscreen" });
string_enum!(PresentationHint { Async => "async", Vsync => "vsync" });

// The output renderer exposes a subset of the protocol surface vocabulary.
string_enum!(TransferFunction {
    None => "none", Srgb => "sRGB", Pq => "PQ", ExtendedLinear => "extended linear",
    Gamma22 => "gamma 2.2", Bt1886 => "BT.1886", Gamma28 => "gamma 2.8",
    St240 => "ST 240", Log100 => "log 100", Log316 => "log 316", Xvycc => "xvYCC",
    ExtendedSrgb => "extended sRGB", St428 => "ST 428", Hlg => "HLG",
    CompoundPower24 => "compound power 2.4"
});
string_enum!(Primaries {
    None => "none", Srgb => "sRGB", Bt2020 => "BT.2020", PalM => "PAL-M",
    Pal => "PAL", Ntsc => "NTSC", GenericFilm => "generic film",
    Cie1931Xyz => "CIE 1931 XYZ", DciP3 => "DCI-P3", DisplayP3 => "Display P3",
    AdobeRgb => "Adobe RGB"
});
