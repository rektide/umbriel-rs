//! Small typed wire surface; full response models and action grammar are deferred.
use serde::{Serialize, Serializer, ser::SerializeStruct};

use crate::{
    codec::{self, EncodeError},
    envelope::Family,
};

/// Read-only commands observed in the pinned protocol baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Windows,
    Workspaces,
    Submap,
    Layers,
    Color,
    Tearing,
    KeyboardLayouts,
}
impl Query {
    pub const ALL: [Self; 7] = [
        Self::Windows,
        Self::Workspaces,
        Self::Submap,
        Self::Layers,
        Self::Color,
        Self::Tearing,
        Self::KeyboardLayouts,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Workspaces => "workspaces",
            Self::Submap => "submap",
            Self::Layers => "layers",
            Self::Color => "color",
            Self::Tearing => "tearing",
            Self::KeyboardLayouts => "keyboard-layouts",
        }
    }
}

/// Wire requests, without reply ownership, scheduling, or runtime validation.
/// `Msg` passes the action string through unchanged: JSON escaping is not action
/// grammar validation. Output mutations require a headless server at runtime.
#[derive(Debug, Clone, Copy)]
pub enum Request<'a> {
    LayoutAllocate(&'a crate::model::LayoutAllocation),
    Query(Query),
    Msg(&'a str),
    Subscribe(&'a [Family]),
    OutputCreate(&'a str),
    OutputDestroy(&'a str),
    /// Escape hatch for future commands; strings still go through the serializer.
    Raw {
        cmd: &'a str,
        arg: Option<&'a str>,
    },
}

impl Request<'_> {
    pub fn encode(&self, bytes: &mut Vec<u8>) -> Result<(), EncodeError> {
        codec::encode(self, bytes)
    }
}

impl Serialize for Request<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (cmd, arg) = match self {
            Self::LayoutAllocate(allocation) => {
                let weights = &allocation.weights;
                if allocation.target.is_empty()
                    || weights.rows.len() != allocation.expected.members.len()
                    || weights.rows.is_empty()
                    || !weights.top.is_finite()
                    || weights.top < 0.0
                    || !weights.bottom.is_finite()
                    || weights.bottom < 0.0
                    || weights.rows.iter().any(|w| !w.is_finite() || *w < 0.05)
                    || !(weights.rows.iter().sum::<f64>() + weights.top + weights.bottom)
                        .is_finite()
                    || allocation
                        .target_content_extents
                        .as_ref()
                        .is_some_and(|extents| {
                            extents.len() != weights.rows.len()
                                || extents.iter().any(|extent| *extent <= 0)
                        })
                {
                    return Err(serde::ser::Error::custom("invalid layout allocation"));
                }
                #[derive(Serialize)]
                struct Wire<'a> {
                    cmd: &'static str,
                    #[serde(flatten)]
                    allocation: &'a crate::model::LayoutAllocation,
                }
                return Wire {
                    cmd: "layout-allocate",
                    allocation,
                }
                .serialize(serializer);
            }
            Self::Query(query) => (query.as_str(), None),
            Self::Msg(action) => ("msg", Some(*action)),
            Self::Subscribe(families) => {
                if families.is_empty() {
                    return Err(serde::ser::Error::custom(
                        "subscription must name at least one family",
                    ));
                }
                struct Names<'a>(&'a [Family]);
                impl Serialize for Names<'_> {
                    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        s.collect_seq(self.0.iter().map(|family| family.as_str()))
                    }
                }
                let mut state = serializer.serialize_struct("Request", 2)?;
                state.serialize_field("cmd", "subscribe")?;
                state.serialize_field("events", &Names(families))?;
                return state.end();
            }
            Self::OutputCreate(name) => ("output-create", Some(*name)),
            Self::OutputDestroy(name) => ("output-destroy", Some(*name)),
            Self::Raw { cmd, arg } => (*cmd, *arg),
        };
        let mut state =
            serializer.serialize_struct("Request", if arg.is_some() { 2 } else { 1 })?;
        state.serialize_field("cmd", cmd)?;
        if let Some(arg) = arg {
            state.serialize_field("arg", arg)?;
        }
        state.end()
    }
}
