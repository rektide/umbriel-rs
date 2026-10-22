//! Typed composition above the raw client. Inventory remains the sole row owner.
#![forbid(unsafe_code)]
pub mod activity;

use std::time::Duration;
use umbriel_client::Observation;
use umbriel_protocol::{
    envelope::{EventName, Family, Message},
    model::{Event, EventDecode, decode_event},
};
use umbriel_state::inventory::{Inventory, Stamp, Status, WindowChanges};

/// One observation outcome. Installed does not mean every subscribed family is ready.
#[derive(Debug)]
pub enum Update {
    Epoch {
        epoch: u64,
    },
    Gap {
        epoch: u64,
        reason: String,
    },
    Installed {
        family: Family,
        before: Status,
        after: Status,
        windows: Option<WindowChanges>,
    },
    Rejected {
        epoch: u64,
        family: Option<Family>,
        reason: String,
    },
    Ignored,
}

pub struct Tracker {
    inventory: Inventory,
    families: Vec<Family>,
    window_changes: bool,
}

impl Tracker {
    /// Native subscriptions announce epoch 1. Request exactly these families on
    /// the source too; an unclassifiable malformed frame invalidates all of them.
    pub fn new(families: &[Family], window_changes: bool) -> Self {
        Self {
            inventory: Inventory::new(0),
            families: families.to_vec(),
            window_changes,
        }
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    /// Call for terminal observer loss, including lag. Retained rows become stale.
    pub fn lost(&mut self) {
        self.inventory.gap();
    }

    fn rejected(&mut self, epoch: u64, family: Option<Family>, reason: String) -> Update {
        if let Some(family) = family {
            self.inventory.invalid_snapshot(family);
        } else {
            for family in &self.families {
                self.inventory.invalid_snapshot(*family);
            }
        }
        Update::Rejected { epoch, family, reason }
    }

    pub fn observe(&mut self, now: Duration, observation: Observation) -> Update {
        match observation {
            Observation::Epoch { epoch } => match self.inventory.begin_epoch(epoch) {
                Ok(()) => Update::Epoch { epoch },
                Err(error) => {
                    self.lost();
                    Update::Gap {
                        epoch,
                        reason: error.to_string(),
                    }
                }
            },
            Observation::Gap { epoch, terminal } => {
                self.lost();
                Update::Gap {
                    epoch,
                    reason: format!("{terminal:?}"),
                }
            }
            Observation::Frame { epoch, position, bytes } => {
                let message = match Message::decode(&bytes) {
                    Ok(message) => message,
                    Err(error) => return self.rejected(epoch, None, error.to_string()),
                };
                let family = match &message {
                    Message::Event {
                        name: EventName::Known(family),
                        ..
                    } if self.families.contains(family) => *family,
                    _ => return Update::Ignored,
                };
                let event = match decode_event(&message) {
                    Ok(EventDecode::Known(event)) => event,
                    Ok(_) => return Update::Ignored,
                    Err(error) => return self.rejected(epoch, Some(family), error.to_string()),
                };
                let stamp = Stamp {
                    epoch,
                    position,
                    observed_at: now,
                };
                let before = self.inventory.status(family);
                let result = match event {
                    Event::Windows(windows) if self.window_changes => self.inventory.install_windows_changes(stamp, windows).map(Some),
                    event => self.inventory.install(stamp, event).map(|()| None),
                };
                match result {
                    Ok(windows) => Update::Installed {
                        family,
                        before,
                        after: self.inventory.status(family),
                        windows,
                    },
                    Err(error) => self.rejected(epoch, Some(family), error.to_string()),
                }
            }
        }
    }
}
