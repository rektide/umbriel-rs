use super::*;
use crate::{
    Payload, Record, emit,
    output::{Export, export_dispatch},
};
use std::{
    io,
    sync::{Arc, Mutex},
};
use umbriel_state::active::{ActiveWindow, StableActive};

#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn record(index: usize) -> Record {
    Record {
        run_id: "01234567-89ab-4cde-8123-456789abcdef".into(),
        seq: index as u64 + 1,
        event_time: "2026-10-31T00:00:05Z".into(),
        event_mono_ms: 5000,
        dwell_ms: 5000,
        payload: Payload::Accepted(StableActive {
            epoch: 1,
            since: Duration::ZERO,
            last_observed: Duration::from_millis(1000),
            emitted_at: Duration::from_secs(5),
            window: Some(ActiveWindow {
                id: "window".into(),
                title: "notes\n\"quote\"\u{1b}]2;inert\u{7}".into(),
                app_id: "app".into(),
                workspace: "ws".into(),
                scratchpad: String::new(),
            }),
        }),
    }
}
fn raw(record: &Record, index: usize) -> Raw {
    // Build the fixture from the actual producer's shared tracing emission.
    let buffer = Buffer::default();
    let writer = buffer.clone();
    emit(record, &export_dispatch(Export::Jsonl, move || writer.clone()));
    let value: serde_json::Value = serde_json::from_slice(&buffer.0.lock().unwrap()).unwrap();
    let mut fields: Vec<_> = value["fields"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.as_str() != "message")
        .map(|(name, value)| {
            (
                format!("UMBRIEL_{}", name.to_ascii_uppercase()).into_bytes(),
                value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()).into_bytes(),
            )
        })
        .collect();
    fields.extend([
        (b"TARGET".to_vec(), b"umbriel::activity".to_vec()),
        (b"_UID".to_vec(), Feed::default().uid.to_string().into_bytes()),
        (b"_SYSTEMD_USER_UNIT".to_vec(), b"umbriel-activity.service".to_vec()),
        (b"UNRELATED".to_vec(), vec![0, 255, b'\n']),
        (b"UNRELATED".to_vec(), b"second value".to_vec()),
    ]);
    Raw {
        cursor: format!("c{index}"),
        realtime_usec: (index as u64 + 1) * 1000,
        fields,
    }
}

enum Seek {
    Head,
    Tail,
    Cursor(String),
    Time(u64),
}
#[derive(Default)]
struct Fake {
    entries: Vec<Raw>,
    position: isize,
    seek: Option<Seek>,
    process_calls: usize,
    append_on_head: Option<Raw>,
    remove_on_finish: bool,
    invalidate_on_finish: bool,
    fail_current: bool,
    fail_time: bool,
}
impl Fake {
    fn target(&self, cursor: &str) -> Option<usize> {
        self.entries.iter().position(|r| r.cursor == cursor.strip_prefix("alias-").unwrap_or(cursor))
    }
    fn resolve(&mut self, forward: bool) {
        if let Some(seek) = self.seek.take() {
            self.position = match seek {
                Seek::Head => {
                    if forward {
                        0
                    } else {
                        -1
                    }
                }
                Seek::Tail => self.entries.len() as isize - if forward { 0 } else { 1 },
                Seek::Cursor(cursor) => self.target(&cursor).unwrap_or(0) as isize,
                Seek::Time(time) => {
                    if forward {
                        self.entries.partition_point(|e| e.realtime_usec < time) as isize
                    } else {
                        self.entries.partition_point(|e| e.realtime_usec <= time) as isize - 1
                    }
                }
            };
        } else {
            self.position += if forward { 1 } else { -1 };
        }
    }
}
impl Access for Fake {
    fn head(&mut self) -> Result<(), Error> {
        if let Some(raw) = self.append_on_head.take() {
            self.entries.push(raw);
        }
        self.seek = Some(Seek::Head);
        Ok(())
    }
    fn tail(&mut self) -> Result<(), Error> {
        self.seek = Some(Seek::Tail);
        Ok(())
    }
    fn seek(&mut self, cursor: &str) -> Result<(), Error> {
        self.seek = Some(Seek::Cursor(cursor.into()));
        Ok(())
    }
    fn seek_time(&mut self, time: u64) -> Result<(), Error> {
        self.seek = Some(Seek::Time(time));
        Ok(())
    }
    fn next(&mut self) -> Result<bool, Error> {
        self.resolve(true);
        Ok(self.position >= 0 && self.position < self.entries.len() as isize)
    }
    fn previous(&mut self) -> Result<bool, Error> {
        self.resolve(false);
        Ok(self.position >= 0 && self.position < self.entries.len() as isize)
    }
    fn test(&self, cursor: &str) -> Result<bool, Error> {
        Ok(self.target(cursor).is_some_and(|i| i as isize == self.position))
    }
    fn cursor(&self) -> Result<String, Error> {
        Ok(self.entries[self.position as usize].cursor.clone())
    }
    fn time(&self) -> Result<u64, Error> {
        if self.fail_time {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied).into());
        }
        Ok(self.entries[self.position as usize].realtime_usec)
    }
    fn current(&mut self, budget: &mut Budget<'_>) -> Result<Raw, Error> {
        if self.fail_current {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied).into());
        }
        let raw = self.entries[self.position as usize].clone();
        let bytes = raw.fields.iter().map(|(k, v)| k.len() + v.len() + 1).sum();
        budget.bytes(bytes)?;
        if bytes > budget.limits.max_record_bytes {
            return Err(Error::Budget("record bytes"));
        }
        Ok(raw)
    }
    fn process(&mut self) -> Result<bool, Error> {
        self.process_calls += 1;
        if self.process_calls == 2 {
            if self.remove_on_finish {
                self.entries.pop();
            }
            return Ok(self.invalidate_on_finish);
        }
        Ok(false)
    }
}
fn sample() -> Fake {
    Fake {
        entries: (0..3).map(|i| raw(&record(i), i)).collect(),
        ..Fake::default()
    }
}
fn cursor(name: &str) -> Cursor {
    Cursor {
        feed: Feed::default(),
        opaque: name.into(),
    }
}
fn read(j: &mut Fake, start: Start, after: Option<&Cursor>) -> Result<Replay, Error> {
    read::bounded(j, &Feed::default(), start, after, None, Limits::default(), &|| false)
}

#[test]
fn all_event_shapes_round_trip_original_fields_including_title_and_metadata_link() {
    let base = record(0);
    let Payload::Accepted(active) = &base.payload else { unreachable!() };
    let mut none = active.clone();
    none.window = None;
    for payload in [
        Payload::Accepted(active.clone()),
        Payload::Metadata {
            active: active.clone(),
            accepted_seq: 1,
        },
        Payload::Accepted(none),
        Payload::Epoch { epoch: 1 },
        Payload::Ready { epoch: 1 },
        Payload::Gap {
            epoch: 1,
            reason: "gap\nreason".into(),
        },
        Payload::Stopping,
    ] {
        let dwell_ms = if matches!(&payload, Payload::Accepted(_) | Payload::Metadata { .. }) {
            5000
        } else {
            0
        };
        let original = Record {
            payload,
            seq: 2,
            dwell_ms,
            ..base.clone()
        };
        let decoded = decode::decode(raw(&original, 0), &Feed::default()).unwrap();
        assert_eq!(decoded.record, original);
    }
}
#[test]
fn exact_resume_tail_and_empty_have_explicit_semantics() {
    assert_eq!(read(&mut sample(), Start::Head, None).unwrap().entries.len(), 3);
    assert_eq!(read(&mut sample(), Start::Tail(2), None).unwrap().entries[0].record.seq, 2);
    assert_eq!(read(&mut sample(), Start::Tail(9), None).unwrap().entries.len(), 3);
    assert_eq!(read(&mut sample(), Start::Head, Some(&cursor("alias-c0"))).unwrap().entries.len(), 2);
    assert!(read(&mut sample(), Start::Head, Some(&cursor("alias-c2"))).unwrap().entries.is_empty());
    let empty = read(&mut Fake::default(), Start::Head, None).unwrap();
    assert!(empty.entries.is_empty() && empty.upper.is_none());
    assert!(matches!(
        read(&mut sample(), Start::Head, Some(&cursor("expired"))),
        Err(Error::CursorUnavailable("lower"))
    ));
    assert!(matches!(
        read(&mut sample(), Start::Head, Some(&cursor(""))),
        Err(Error::CursorUnavailable("lower"))
    ));
    let mut foreign = cursor("c0");
    foreign.feed.uid += 1;
    assert!(matches!(read(&mut sample(), Start::Head, Some(&foreign)), Err(Error::CursorFeedMismatch)));
}
#[test]
fn finite_cut_then_suffix_does_not_chase_or_duplicate_handoff_append() {
    let mut j = sample();
    j.append_on_head = Some(raw(&record(3), 3));
    let first = read(&mut j, Start::Head, None).unwrap();
    assert_eq!(first.entries.len(), 3);
    let second = read(&mut j, Start::Head, first.upper.as_ref()).unwrap();
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].record.seq, 4);
    assert!(read(&mut j, Start::Head, second.upper.as_ref()).unwrap().entries.is_empty());
}
#[test]
fn time_selection_is_inclusive_since_exclusive_until_and_propagates_errors() {
    let run = |start, after: Option<&Cursor>, until| read::bounded(&mut sample(), &Feed::default(), start, after, until, Limits::default(), &|| false);
    assert_eq!(run(Start::Since(2000), None, Some(3000)).unwrap().entries[0].record.seq, 2);
    assert!(run(Start::Since(4000), None, None).unwrap().entries.is_empty());
    assert!(run(Start::Head, None, Some(1000)).unwrap().entries.is_empty());
    assert!(matches!(run(Start::Since(3000), None, Some(2000)), Err(Error::InvalidRange)));
    assert!(matches!(run(Start::Head, Some(&cursor("c2")), Some(2000)), Err(Error::InvalidRange)));
    let mut j = sample();
    j.fail_time = true;
    assert!(matches!(
        read::bounded(&mut j, &Feed::default(), Start::Head, None, Some(3000), Limits::default(), &|| false),
        Err(Error::Io(_))
    ));
}
#[test]
fn cut_loss_invalidation_native_failure_and_cancellation_are_not_partial_success() {
    let mut j = sample();
    j.remove_on_finish = true;
    assert!(matches!(read(&mut j, Start::Head, None), Err(Error::CursorUnavailable("upper"))));
    let mut j = sample();
    j.invalidate_on_finish = true;
    assert!(matches!(read(&mut j, Start::Head, None), Err(Error::Invalidated)));
    let mut j = sample();
    j.fail_current = true;
    assert!(matches!(read(&mut j, Start::Head, None), Err(Error::Io(_))));
    assert!(matches!(
        read::bounded(&mut sample(), &Feed::default(), Start::Head, None, None, Limits::default(), &|| true),
        Err(Error::Cancelled)
    ));
}
#[test]
fn entry_byte_record_time_and_tail_budgets_fail_without_truncation() {
    for limits in [
        Limits {
            max_entries: 1,
            ..Limits::default()
        },
        Limits {
            max_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_record_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_elapsed: Duration::ZERO,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            read::bounded(&mut sample(), &Feed::default(), Start::Head, None, None, limits, &|| false),
            Err(Error::Budget(_))
        ));
    }
    assert!(matches!(read(&mut sample(), Start::Tail(0), None), Err(Error::Budget(_))));
    assert!(matches!(read(&mut sample(), Start::Tail(10_001), None), Err(Error::Budget(_))));
}
#[test]
fn malformed_repeated_binary_unknown_and_untrusted_scalars_fail_explicitly() {
    for (name, value) in [
        ("UMBRIEL_TITLE", vec![255]),
        ("UMBRIEL_SCHEMA_VERSION", b"2".to_vec()),
        ("UMBRIEL_EVENT", b"future".to_vec()),
        ("UMBRIEL_SEQ", b"-1".to_vec()),
        ("_UID", b"9999999".to_vec()),
    ] {
        let mut entry = raw(&record(0), 0);
        entry.fields.iter_mut().find(|(k, _)| k == name.as_bytes()).unwrap().1 = value;
        assert!(decode::decode(entry, &Feed::default()).is_err());
    }
    let mut entry = raw(&record(0), 0);
    entry.fields.push((b"UMBRIEL_TITLE".to_vec(), b"second".to_vec()));
    assert!(matches!(decode::decode(entry, &Feed::default()), Err(Error::Field(_))));
    let mut entry = raw(&record(0), 0);
    entry.fields.retain(|(k, _)| k != b"UMBRIEL_TITLE");
    assert!(matches!(decode::decode(entry, &Feed::default()), Err(Error::Field(_))));
}
#[test]
fn conflicting_run_sequence_is_not_last_write_wins_but_equal_delivery_is_preserved() {
    let mut j = sample();
    j.entries[1] = raw(&record(0), 1);
    assert_eq!(read(&mut j, Start::Head, None).unwrap().entries.len(), 3);
    j.entries[1].fields.iter_mut().find(|(k, _)| k == b"UMBRIEL_TITLE").unwrap().1 = b"conflict".to_vec();
    assert!(matches!(read(&mut j, Start::Head, None), Err(Error::Conflict)));
}
