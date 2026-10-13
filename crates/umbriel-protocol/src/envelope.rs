//! Order-independent borrowed envelopes. No request/reply correlation lives here.
use std::{borrow::Cow, fmt};

use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use sonic_rs::LazyValue;

use crate::codec;

/// The six families observed in Umbriel 0.1.x, not a negotiated capability list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Theme,
    Overview,
    KeyboardLayout,
    Windows,
    Workspaces,
    Submap,
}

impl Family {
    pub const ALL: [Self; 6] = [
        Self::Theme,
        Self::Overview,
        Self::KeyboardLayout,
        Self::Windows,
        Self::Workspaces,
        Self::Submap,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Overview => "overview",
            Self::KeyboardLayout => "keyboard_layout",
            Self::Windows => "windows",
            Self::Workspaces => "workspaces",
            Self::Submap => "submap",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventName<'a> {
    Known(Family),
    Unknown(Cow<'a, str>),
}

impl<'a> EventName<'a> {
    pub fn new(name: Cow<'a, str>) -> Self {
        match Family::ALL
            .into_iter()
            .find(|family| family.as_str() == name)
        {
            Some(family) => Self::Known(family),
            None => Self::Unknown(name),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(family) => family.as_str(),
            Self::Unknown(name) => name,
        }
    }
}

#[derive(Debug)]
pub enum Message<'a> {
    /// Bare LF: contextual subscription evidence, never an ordinary reply.
    Blank,
    Ok(LazyValue<'a>),
    Err(Cow<'a, str>),
    Event {
        name: EventName<'a>,
        data: LazyValue<'a>,
    },
}

impl<'a> Message<'a> {
    /// Decode a frame without LF. Only an exactly empty frame is `Blank`.
    /// Significant keys must be unique and form exactly one envelope shape.
    /// Additional fields are accepted and validated, including their nested JSON.
    pub fn decode(frame: &'a [u8]) -> sonic_rs::Result<Self> {
        if frame.is_empty() {
            return Ok(Self::Blank);
        }
        codec::decode(frame)
    }
}

// Serde's ordinary Cow implementation owns strings. This visitor preserves
// Sonic's borrowed fast path, but takes ownership of decoded escaped scratch strings.
struct Text<'a>(Cow<'a, str>);
impl<'de> Deserialize<'de> for Text<'de> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct TextVisitor;
        impl<'de> Visitor<'de> for TextVisitor {
            type Value = Text<'de>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a string")
            }
            fn visit_borrowed_str<E: de::Error>(self, s: &'de str) -> Result<Self::Value, E> {
                Ok(Text(Cow::Borrowed(s)))
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(Text(Cow::Owned(s.to_owned())))
            }
            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(Text(Cow::Owned(s)))
            }
        }
        d.deserialize_string(TextVisitor)
    }
}

impl<'de> Deserialize<'de> for Message<'de> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct EnvelopeVisitor;
        impl<'de> Visitor<'de> for EnvelopeVisitor {
            type Value = Message<'de>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an Umbriel envelope object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut ok = None;
                let mut err = None;
                let mut event = None;
                let mut data = None;
                while let Some(Text(key)) = map.next_key::<Text<'de>>()? {
                    match key.as_ref() {
                        "ok" => {
                            if ok.is_some() {
                                return Err(de::Error::duplicate_field("ok"));
                            }
                            // next_value is LazyValue, not Option: present null stays Some.
                            ok = Some(map.next_value::<LazyValue<'de>>()?);
                        }
                        "err" => {
                            if err.is_some() {
                                return Err(de::Error::duplicate_field("err"));
                            }
                            err = Some(map.next_value::<Text<'de>>()?.0);
                        }
                        "event" => {
                            if event.is_some() {
                                return Err(de::Error::duplicate_field("event"));
                            }
                            event = Some(map.next_value::<Text<'de>>()?.0);
                        }
                        "data" => {
                            if data.is_some() {
                                return Err(de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value::<LazyValue<'de>>()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }
                match (ok, err, event, data) {
                    (Some(value), None, None, None) => Ok(Message::Ok(value)),
                    (None, Some(text), None, None) => Ok(Message::Err(text)),
                    (None, None, Some(name), Some(data)) => Ok(Message::Event {
                        name: EventName::new(name),
                        data,
                    }),
                    _ => Err(de::Error::custom("missing or conflicting envelope keys")),
                }
            }
        }
        d.deserialize_map(EnvelopeVisitor)
    }
}
