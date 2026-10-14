//! Allocation accounting for the benchmarked decode and framing paths.
//!
//! Not a Criterion target: a counting global allocator perturbs timing, so it
//! runs in its own binary and reports per-call allocation counts (calls and
//! requested bytes; `realloc` counts as one call at its new size). Counters
//! are armed only around the measured closure, so process noise is excluded.
//!
//! The `unsafe` in this file is confined to benchmark instrumentation: the
//! forwarding `GlobalAlloc` impl. Production crates in this workspace keep
//! `#![forbid(unsafe_code)]`; this file is the deliberate, documented
//! exception and ships no library code.
//!
//! Run everything:
//!      cargo bench -p umbriel-protocol --bench allocations
//! or filter by substring:
//!      cargo bench -p umbriel-protocol --bench allocations -- selected

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::Deserialize;
use umbriel_protocol::{
    codec::{self, Framer, Limits},
    envelope::Message,
    model::{self, Event, EventDecode, Windows},
};

mod common;

// With field-level #[serde(borrow)]: unescaped values decode Cow::Borrowed;
// escaped values decode Cow::Owned (both branches verified in main()).
// `active` is decoded-and-dropped here; decode.rs cross-checks contents.
#[allow(dead_code)]
#[derive(Deserialize)]
struct SelectedWindow<'a> {
    #[serde(borrow)]
    id: Cow<'a, str>,
    active: bool,
    #[serde(borrow)]
    title: Cow<'a, str>,
}

// Deliberate misconfiguration comparison: the same projection without
// #[serde(borrow)] deserializes every field owned, exactly like String.
// Kept to quantify the cost of forgetting the annotation.
#[allow(dead_code)]
#[derive(Deserialize)]
struct SelectedOwnedWindow {
    id: String,
    active: bool,
    title: String,
}

struct CountingAllocator {
    armed: AtomicBool,
    calls: AtomicU64,
    bytes: AtomicU64,
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator {
    armed: AtomicBool::new(false),
    calls: AtomicU64::new(0),
    bytes: AtomicU64::new(0),
};

// # Safety: forwards every operation to `System` unchanged; counting is
// side-effect-free bookkeeping around the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        self.count(pointer, layout.size());
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let reallocated = unsafe { System.realloc(pointer, layout, new_size) };
        self.count(reallocated, new_size);
        reallocated
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

impl CountingAllocator {
    fn count(&self, pointer: *mut u8, size: usize) {
        if !pointer.is_null() && self.armed.load(Ordering::Relaxed) {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.bytes.fetch_add(size as u64, Ordering::Relaxed);
        }
    }
}

/// Warm the path, then report first-call and steady per-call allocation
/// counts over `iterations` armed calls (median / min / max). "Steady"
/// describes the measured path at its working set, not allocator warmth:
/// the process allocator is already warmed by fixture loading before the
/// first armed call.
fn measure<T>(name: &str, iterations: u64, mut path: impl FnMut() -> T) {
    let mut first: Option<(u64, u64)> = None;
    let mut samples: Vec<(u64, u64)> = Vec::new();
    for i in 0..=iterations {
        ALLOCATOR.calls.store(0, Ordering::Relaxed);
        ALLOCATOR.bytes.store(0, Ordering::Relaxed);
        ALLOCATOR.armed.store(true, Ordering::Relaxed);
        let result = path();
        ALLOCATOR.armed.store(false, Ordering::Relaxed);
        drop(result);
        let sample = (
            ALLOCATOR.calls.load(Ordering::Relaxed),
            ALLOCATOR.bytes.load(Ordering::Relaxed),
        );
        if i == 0 {
            first = Some(sample);
        } else {
            samples.push(sample);
        }
    }
    samples.sort();
    let (median_calls, median_bytes) = samples[samples.len() / 2];
    let (min_calls, min_bytes) = samples[0];
    let (max_calls, max_bytes) = samples[samples.len() - 1];
    let (first_calls, first_bytes) = first.expect("at least one call");
    println!(
        "{name:<42} first {:>5} calls {:>8} B | steady median {:>5} calls {:>8} B | min {:>5}/{:>8} max {:>5}/{:>8}",
        first_calls,
        first_bytes,
        median_calls,
        median_bytes,
        min_calls,
        min_bytes,
        max_calls,
        max_bytes,
    );
}

fn main() {
    // cargo bench passes its own flags (e.g. `--bench`) to the binary;
    // only a leading non-flag token acts as a label filter.
    let filter = std::env::args().nth(1).filter(|arg| !arg.starts_with('-'));
    let selected = |name: &str| filter.as_deref().is_none_or(|f| name.contains(f));

    let windows_line = common::fixture("live-windows.json");
    let live_frame = common::frame_without_lf(&windows_line).to_vec();
    let synthetic_line = common::fixture("synthetic-windows-50.json");
    let synthetic_frame = common::frame_without_lf(&synthetic_line).to_vec();

    let payload_of = |frame: &[u8]| -> Vec<u8> {
        let message = Message::decode(frame).expect("event frame");
        let Message::Event { data, .. } = &message else {
            panic!("event");
        };
        data.as_raw_str().as_bytes().to_vec()
    };
    let live_data = payload_of(&live_frame);
    let synthetic_data = payload_of(&synthetic_frame);

    // Verify both Cow branches before measuring: ids are unescaped in every
    // fixture (Borrowed); live titles are all unescaped (Borrowed);
    // synthetic titles include escapes (at least one Owned, one Borrowed).
    for (name, frame) in [("live", &live_frame), ("synthetic", &synthetic_frame)] {
        let message = Message::decode(frame).expect("envelope");
        let Message::Event { data, .. } = &message else {
            panic!("event");
        };
        let projection: Vec<SelectedWindow<'_>> =
            codec::decode(data.as_raw_str().as_bytes()).expect("projection");
        assert!(
            projection.iter().all(|w| matches!(w.id, Cow::Borrowed(_))),
            "{name} ids borrow"
        );
        let borrowed = projection
            .iter()
            .filter(|w| matches!(w.title, Cow::Borrowed(_)))
            .count();
        match name {
            "live" => assert_eq!(borrowed, projection.len(), "live titles borrow"),
            _ => assert!(
                borrowed > 0 && borrowed < projection.len(),
                "synthetic mixed"
            ),
        }
    }

    let limits: Limits = common::bench_limits();
    let live_stream = common::repeat_frames(&windows_line, 16);

    println!(
        "allocation accounting (first call after process start; steady = median of 200 armed calls)\n"
    );
    for (label, frame) in [
        ("live-windows(29)", &live_frame),
        ("synthetic-50", &synthetic_frame),
    ] {
        let data = if label.starts_with("live") {
            &live_data
        } else {
            &synthetic_data
        };
        if selected("envelope") {
            measure(&format!("envelope {label}"), 200, || {
                Message::decode(frame).expect("envelope")
            });
        }
        if selected("typed") {
            measure(&format!("envelope+typed {label}"), 200, || {
                let message = Message::decode(frame).expect("envelope");
                match model::decode_event(&message).expect("typed") {
                    EventDecode::Known(Event::Windows(windows)) => windows,
                    other => panic!("{other:?}"),
                }
            });
        }
        if selected("model_direct") {
            measure(&format!("model_direct {label}"), 200, || {
                codec::decode::<Windows>(data).expect("model")
            });
        }
        if selected("selected") {
            // The borrowed projection cannot outlive the frame's LazyValue,
            // so consume it inside the measured closure; allocation counts
            // are unaffected (only alloc/realloc are counted).
            measure(&format!("envelope+selected(borrow) {label}"), 200, || {
                let message = Message::decode(frame).expect("envelope");
                let Message::Event { data, .. } = &message else {
                    panic!("event");
                };
                let projection =
                    codec::decode::<Vec<SelectedWindow<'_>>>(data.as_raw_str().as_bytes())
                        .expect("projection");
                std::hint::black_box(&projection);
            });
            measure(
                &format!("envelope+selected(owned,no-borrow) {label}"),
                200,
                || {
                    let message = Message::decode(frame).expect("envelope");
                    let Message::Event { data, .. } = &message else {
                        panic!("event");
                    };
                    let projection =
                        codec::decode::<Vec<SelectedOwnedWindow>>(data.as_raw_str().as_bytes())
                            .expect("projection");
                    std::hint::black_box(&projection);
                },
            );
        }
    }
    if selected("framing") {
        // Fresh Framer per call: cold first-turn shape, including the single
        // exact-size capacity growth. Application storage is NOT reused here.
        measure("framing cold/fresh Framer first turn ×16", 200, || {
            let mut framer = Framer::new(limits);
            framer.push(&live_stream).expect("push");
            while let Some(frame) = framer.next_frame() {
                std::hint::black_box(frame);
            }
        });
        // Reused application storage: second full turn on one Framer whose
        // capacity was grown by the first turn. Allocator warmth is not the
        // variable here; storage reuse is.
        let mut framer = Framer::new(limits);
        framer.push(&live_stream).expect("push");
        while framer.next_frame().is_some() {}
        ALLOCATOR.calls.store(0, Ordering::Relaxed);
        ALLOCATOR.bytes.store(0, Ordering::Relaxed);
        ALLOCATOR.armed.store(true, Ordering::Relaxed);
        framer.push(&live_stream).expect("push");
        while let Some(frame) = framer.next_frame() {
            std::hint::black_box(frame);
        }
        ALLOCATOR.armed.store(false, Ordering::Relaxed);
        println!(
            "framing reused Framer, second full turn        {:>5} calls {:>8} B",
            ALLOCATOR.calls.load(Ordering::Relaxed),
            ALLOCATOR.bytes.load(Ordering::Relaxed)
        );
    }
}
