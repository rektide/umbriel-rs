use super::*;
use crate::{Payload, Record};
use std::collections::BTreeMap;
use umbriel_state::active::{ActiveWindow, StableActive};

fn bad(name: &str) -> Error { Error::Field(name.into()) }
struct Fields<'a>(BTreeMap<&'a str, &'a str>);
impl Fields<'_> {
    fn optional(&mut self, name: &str) -> Option<&str> { self.0.remove(name) }
    fn take(&mut self, name: &str) -> Result<String, Error> { self.optional(name).map(str::to_owned).ok_or_else(|| bad(name)) }
    fn number(&mut self, name: &str) -> Result<u64, Error> {
        let value = self.take(name)?;
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) { return Err(bad(name)); }
        value.parse().map_err(|_| bad(name))
    }
}

pub(super) fn decode(raw: Raw, feed: &Feed) -> Result<Entry, Error> {
    let mut values = BTreeMap::new();
    // Unknown native fields may be binary/repeated. Never collapse them into a
    // string map or lossily decode them. Every selected schema/origin scalar is
    // checked for exact cardinality and valid UTF-8, even when its value is empty.
    for (name, value) in &raw.fields {
        let Ok(name) = std::str::from_utf8(name) else { continue };
        if !name.starts_with("UMBRIEL_") && !matches!(name, "TARGET" | "_UID" | "_SYSTEMD_USER_UNIT" | "_BOOT_ID" | "_SYSTEMD_INVOCATION_ID") { continue; }
        let value = std::str::from_utf8(value).map_err(|_| bad(name))?;
        if values.insert(name, value).is_some() { return Err(bad(name)); }
    }
    let mut f = Fields(values);
    if f.take("TARGET")? != "umbriel::activity" { return Err(Error::OutsideFeed); }
    let uid = u32::try_from(f.number("_UID")?).map_err(|_| bad("_UID"))?;
    let user_unit = f.optional("_SYSTEMD_USER_UNIT").map(str::to_owned);
    let boot_id = f.optional("_BOOT_ID").map(str::to_owned);
    let invocation_id = f.optional("_SYSTEMD_INVOCATION_ID").map(str::to_owned);
    if uid != feed.uid || feed.user_unit.as_ref().is_some_and(|v| Some(v) != user_unit.as_ref()) || feed.boot_id.as_ref().is_some_and(|v| Some(v) != boot_id.as_ref()) { return Err(Error::OutsideFeed); }
    if f.number("UMBRIEL_SCHEMA_VERSION")? != 1 { return Err(Error::Unsupported); }
    let kind = f.take("UMBRIEL_EVENT")?;
    let run_id = f.take("UMBRIEL_RUN_ID")?;
    uuid::Uuid::parse_str(&run_id).map_err(|_| bad("UMBRIEL_RUN_ID"))?;
    if feed.run_id.as_ref().is_some_and(|v| v != &run_id) { return Err(Error::OutsideFeed); }
    let seq = f.number("UMBRIEL_SEQ")?;
    if seq == 0 { return Err(bad("UMBRIEL_SEQ")); }
    let event_time = f.take("UMBRIEL_EVENT_TIME")?;
    event_time.parse::<jiff::Timestamp>().map_err(|_| bad("UMBRIEL_EVENT_TIME"))?;
    let event_mono_ms = f.number("UMBRIEL_EVENT_MONO_MS")?;
    let mut dwell_ms = 0;
    let payload = match kind.as_str() {
        "source.stopping" => Payload::Stopping,
        "source.epoch" => Payload::Epoch { epoch: f.number("UMBRIEL_EPOCH")? },
        "source.ready" => Payload::Ready { epoch: f.number("UMBRIEL_EPOCH")? },
        "source.gap" => Payload::Gap { epoch: f.number("UMBRIEL_EPOCH")?, reason: f.take("UMBRIEL_REASON")? },
        "activity.accepted" | "activity.metadata" => {
            let epoch = f.number("UMBRIEL_EPOCH")?;
            let since = f.number("UMBRIEL_SINCE_MONO_MS")?;
            let last = f.number("UMBRIEL_LAST_OBSERVED_MONO_MS")?;
            dwell_ms = f.number("UMBRIEL_DWELL_MS")?;
            if since > last || last > event_mono_ms || event_mono_ms.saturating_sub(since) < dwell_ms { return Err(bad("monotonic activity times")); }
            let window = match f.take("UMBRIEL_STATE")?.as_str() {
                "none" => None,
                "active" => {
                    let id = f.take("UMBRIEL_WINDOW_ID")?;
                    if id.is_empty() { return Err(bad("UMBRIEL_WINDOW_ID")); }
                    Some(ActiveWindow { id: id.into(), title: f.take("UMBRIEL_TITLE")?, app_id: f.take("UMBRIEL_APP_ID")?, workspace: f.take("UMBRIEL_WORKSPACE_ID")?.into(), scratchpad: f.take("UMBRIEL_SCRATCHPAD")? })
                }
                _ => return Err(bad("UMBRIEL_STATE")),
            };
            let active = StableActive { epoch, since: Duration::from_millis(since), last_observed: Duration::from_millis(last), emitted_at: Duration::from_millis(event_mono_ms), window };
            if kind == "activity.metadata" {
                let accepted_seq = f.number("UMBRIEL_ACCEPTED_SEQ")?;
                if accepted_seq == 0 || accepted_seq >= seq { return Err(bad("UMBRIEL_ACCEPTED_SEQ")); }
                Payload::Metadata { active, accepted_seq }
            } else { Payload::Accepted(active) }
        }
        _ => return Err(Error::Unsupported),
    };
    // Known fields in the wrong event shape must not silently disappear. Unknown
    // extension fields are tolerated within schema1 but never interpreted.
    for name in ["UMBRIEL_EPOCH", "UMBRIEL_REASON", "UMBRIEL_STATE", "UMBRIEL_SINCE_MONO_MS", "UMBRIEL_LAST_OBSERVED_MONO_MS", "UMBRIEL_DWELL_MS", "UMBRIEL_WINDOW_ID", "UMBRIEL_APP_ID", "UMBRIEL_TITLE", "UMBRIEL_WORKSPACE_ID", "UMBRIEL_SCRATCHPAD", "UMBRIEL_ACCEPTED_SEQ"] {
        if f.0.contains_key(name) { return Err(bad(name)); }
    }
    Ok(Entry {
        record: Record { run_id, seq, event_time, event_mono_ms, dwell_ms, payload },
        delivery: Delivery { cursor: Cursor { feed: feed.clone(), opaque: raw.cursor }, realtime_usec: raw.realtime_usec, uid, user_unit, boot_id, invocation_id },
    })
}
