use sonic_rs::LazyValue;

use crate::{
    codec,
    envelope::{EventName, Family, Message},
};

use super::{KeyboardLayouts, Overview, Submap, Theme, Windows, Workspaces};

/// Owned payloads for all six current event families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    // Keep the infrequent theme snapshot from inflating every window event.
    Theme(Box<Theme>),
    Overview(Overview),
    KeyboardLayout(KeyboardLayouts),
    Windows(Windows),
    Workspaces(Workspaces),
    Submap(Submap),
}

impl Event {
    pub const fn family(&self) -> Family {
        match self {
            Self::Theme(_) => Family::Theme,
            Self::Overview(_) => Family::Overview,
            Self::KeyboardLayout(_) => Family::KeyboardLayout,
            Self::Windows(_) => Family::Windows,
            Self::Workspaces(_) => Family::Workspaces,
            Self::Submap(_) => Family::Submap,
        }
    }
}

/// Typed routing without erasing unknown payloads or misclassifying replies.
#[derive(Debug)]
pub enum EventDecode<'message, 'frame> {
    Known(Event),
    Unsupported {
        name: &'message str,
        data: &'message LazyValue<'frame>,
    },
    /// Includes successful replies, server errors, and the blank initial response.
    NotEvent,
}

/// Decode a known payload through the checked generic codec. Leaves `message`
/// available for raw routing and caller-selected projections, including unknown
/// families. A malformed known payload is an error, never an unsupported event.
pub fn decode_event<'message, 'frame>(
    message: &'message Message<'frame>,
) -> sonic_rs::Result<EventDecode<'message, 'frame>> {
    let Message::Event { name, data } = message else {
        return Ok(EventDecode::NotEvent);
    };
    let EventName::Known(family) = name else {
        return Ok(EventDecode::Unsupported {
            name: name.as_str(),
            data,
        });
    };
    let bytes = data.as_raw_str().as_bytes();
    let event = match family {
        Family::Theme => Event::Theme(Box::new(codec::decode(bytes)?)),
        Family::Overview => Event::Overview(codec::decode(bytes)?),
        Family::KeyboardLayout => Event::KeyboardLayout(codec::decode(bytes)?),
        Family::Windows => Event::Windows(codec::decode(bytes)?),
        Family::Workspaces => Event::Workspaces(codec::decode(bytes)?),
        Family::Submap => Event::Submap(codec::decode(bytes)?),
    };
    Ok(EventDecode::Known(event))
}
