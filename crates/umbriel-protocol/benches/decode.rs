//! Decode benchmarks on whole wire frames. Framing is not measured here.
//!
//! Four paths, all consuming the same frame bytes:
//!
//! - `envelope`: `Message::decode` — the real lazy envelope with whole-frame
//!   validation; payload stays a `LazyValue`.
//! - `envelope+typed`: `Message::decode` then `model::decode_event` — the
//!   typed payload path used by event consumers (payload parsed twice: once
//!   validated lazily, once into the owned model).
//! - `model_direct`: `codec::decode::<T>` on the frame's payload bytes,
//!   extracted once at startup — direct full-model decoding without envelope
//!   handling. Produces the same owned model as `envelope+typed` (asserted at
//!   startup: matched output semantics).
//! - `envelope+selected`: `Message::decode` then a narrow borrowed projection
//!   (`id`, `active`, `title`) decoded from the lazy payload — the selective
//!   fast path. Deliberately narrower output; not equivalent to the full-model
//!   paths, so it is reported, not compared as a drop-in.

use std::borrow::Cow;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use serde::Deserialize;
use umbriel_protocol::{
    codec,
    envelope::Message,
    model::{self, Event, EventDecode, Window, Windows, Workspaces},
};

mod common;

#[derive(Deserialize)]
struct SelectedWindow<'a> {
    id: Cow<'a, str>,
    active: bool,
    title: Cow<'a, str>,
}

struct Fixture {
    name: &'static str,
    frame: Vec<u8>, // wire line without LF
    /// Exact payload span of this frame, copied once at startup so the
    /// `model_direct` loop decodes identical bytes without lifetime gymnastics.
    data: Vec<u8>,
    expected_windows: Option<Windows>,
    expected_workspaces: Option<Workspaces>,
}

fn load_windows(name: &'static str) -> Fixture {
    let line = common::fixture(name);
    let frame = common::frame_without_lf(&line).to_vec();
    let message = Message::decode(&frame).expect("valid event frame");
    let umbriel_protocol::envelope::Message::Event { data, .. } = &message else {
        panic!("{name}: not an event frame");
    };
    let data = data.as_raw_str().as_bytes().to_vec();
    let expected: Windows = codec::decode(&data).expect("payload decodes");
    // Matched-output check between the two full-model paths.
    match model::decode_event(&message).expect("typed decode") {
        EventDecode::Known(Event::Windows(typed)) => assert_eq!(typed, expected),
        other => panic!("{name}: unexpected decode_event result {other:?}"),
    }
    Fixture {
        name,
        frame,
        data,
        expected_windows: Some(expected),
        expected_workspaces: None,
    }
}

fn load_workspaces(name: &'static str) -> Fixture {
    let line = common::fixture(name);
    let frame = common::frame_without_lf(&line).to_vec();
    let message = Message::decode(&frame).expect("valid event frame");
    let umbriel_protocol::envelope::Message::Event { data, .. } = &message else {
        panic!("{name}: not an event frame");
    };
    let data = data.as_raw_str().as_bytes().to_vec();
    let expected: Workspaces = codec::decode(&data).expect("payload decodes");
    match model::decode_event(&message).expect("typed decode") {
        EventDecode::Known(Event::Workspaces(typed)) => assert_eq!(typed, expected),
        other => panic!("{name}: unexpected decode_event result {other:?}"),
    }
    Fixture {
        name,
        frame,
        data,
        expected_windows: None,
        expected_workspaces: Some(expected),
    }
}

fn selected_matches_full(selected: &[SelectedWindow<'_>], full: &[Window]) {
    assert_eq!(selected.len(), full.len(), "projection length");
    for (s, w) in selected.iter().zip(full) {
        assert_eq!(s.id.as_ref(), w.id.as_str());
        assert_eq!(s.active, w.active);
        assert_eq!(s.title.as_ref(), w.title);
    }
}

fn bench_windows(c: &mut Criterion, fixtures: Vec<Fixture>) {
    let mut group = c.benchmark_group("decode/windows");
    group
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    for fixture in &fixtures {
        let expected = fixture.expected_windows.as_ref().expect("windows fixture");
        let frame = &fixture.frame[..];
        let data = &fixture.data[..];
        group.throughput(Throughput::Bytes(frame.len() as u64));

        group.bench_with_input(BenchmarkId::new("envelope", fixture.name), &(), |b, _| {
            b.iter(|| {
                let message = Message::decode(std::hint::black_box(frame)).expect("envelope");
                std::hint::black_box(&message);
            });
        });
        group.bench_with_input(
            BenchmarkId::new("envelope+typed", fixture.name),
            &(),
            |b, _| {
                b.iter(|| {
                    let message = Message::decode(std::hint::black_box(frame)).expect("envelope");
                    match model::decode_event(&message).expect("typed") {
                        EventDecode::Known(Event::Windows(windows)) => {
                            std::hint::black_box(&windows);
                        }
                        other => panic!("unexpected {other:?}"),
                    }
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("model_direct", fixture.name),
            &(),
            |b, _| {
                b.iter(|| {
                    let windows: Windows =
                        codec::decode(std::hint::black_box(data)).expect("model");
                    std::hint::black_box(&windows);
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("envelope+selected", fixture.name),
            &(),
            |b, _| {
                b.iter(|| {
                    let message = Message::decode(std::hint::black_box(frame)).expect("envelope");
                    let umbriel_protocol::envelope::Message::Event { data, .. } = &message else {
                        panic!("event");
                    };
                    let selected: Vec<SelectedWindow<'_>> =
                        codec::decode(data.as_raw_str().as_bytes()).expect("projection");
                    std::hint::black_box(&selected);
                });
            },
        );

        // Startup-only correctness cross-checks; never part of timed loops.
        let message = Message::decode(frame).expect("envelope");
        let Message::Event {
            data: lazy_payload, ..
        } = &message
        else {
            panic!("event");
        };
        let selected: Vec<SelectedWindow<'_>> =
            codec::decode(lazy_payload.as_raw_str().as_bytes()).expect("projection");
        selected_matches_full(&selected, expected);
        let direct: Windows = codec::decode(data).expect("model");
        assert_eq!(&direct, expected);
    }
    group.finish();
}

fn bench_workspaces(c: &mut Criterion, fixtures: Vec<Fixture>) {
    let mut group = c.benchmark_group("decode/workspaces");
    group
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    for fixture in &fixtures {
        let expected = fixture
            .expected_workspaces
            .as_ref()
            .expect("workspaces fixture");
        let frame = &fixture.frame[..];
        let data = &fixture.data[..];
        group.throughput(Throughput::Bytes(frame.len() as u64));

        group.bench_with_input(BenchmarkId::new("envelope", fixture.name), &(), |b, _| {
            b.iter(|| {
                let message = Message::decode(std::hint::black_box(frame)).expect("envelope");
                std::hint::black_box(&message);
            });
        });
        group.bench_with_input(
            BenchmarkId::new("envelope+typed", fixture.name),
            &(),
            |b, _| {
                b.iter(|| {
                    let message = Message::decode(std::hint::black_box(frame)).expect("envelope");
                    match model::decode_event(&message).expect("typed") {
                        EventDecode::Known(Event::Workspaces(workspaces)) => {
                            std::hint::black_box(&workspaces);
                        }
                        other => panic!("unexpected {other:?}"),
                    }
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("model_direct", fixture.name),
            &(),
            |b, _| {
                b.iter(|| {
                    let workspaces: Workspaces =
                        codec::decode(std::hint::black_box(data)).expect("model");
                    std::hint::black_box(&workspaces);
                });
            },
        );
        let direct: Workspaces = codec::decode(data).expect("model");
        assert_eq!(&direct, expected);
    }
    group.finish();
}

fn decode_benches(c: &mut Criterion) {
    bench_windows(
        c,
        vec![
            load_windows("live-windows.json"),
            load_windows("synthetic-windows-10.json"),
            load_windows("synthetic-windows-50.json"),
            load_windows("synthetic-windows-200.json"),
        ],
    );
    bench_workspaces(c, vec![load_workspaces("live-workspaces.json")]);
}

criterion_group!(benches, decode_benches);
criterion_main!(benches);
