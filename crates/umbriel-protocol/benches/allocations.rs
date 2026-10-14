//! Allocation accounting for the benchmarked decode and framing paths.
//!
//! Not a Criterion target: a counting global allocator perturbs timing, so it
//! runs in its own binary and reports per-call allocation counts (calls and
//! requested bytes; `realloc` counts as one call at its new size). Counters
//! are armed only around the measured closure, so process/criterion-style
//! noise is excluded.
//!
//! Run: cargo bench -p umbriel-protocol --bench allocations --no-run &&
//!      cargo bench -p umbriel-protocol --bench allocations
//! (plain binary; prints one table and exits).

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

// Fields are decoded-and-dropped in the allocation runs; only decode.rs
// cross-checks their contents.
#[allow(dead_code)]
#[derive(Deserialize)]
struct SelectedWindow<'a> {
    id: Cow<'a, str>,
    active: bool,
    title: Cow<'a, str>,
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
/// counts over `iterations` armed calls (median / min / max).
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

    let limits: Limits = common::bench_limits();
    let live_stream = common::repeat_frames(&windows_line, 16);

    println!(
        "allocation accounting (first call after process start; steady = median of 200 armed calls)\n"
    );
    for (label, frame) in [
        ("live-windows(29)", &live_frame),
        ("synthetic-50", &synthetic_frame),
    ] {
        measure(&format!("envelope {label}"), 200, || {
            Message::decode(frame).expect("envelope")
        });
        measure(&format!("envelope+typed {label}"), 200, || {
            let message = Message::decode(frame).expect("envelope");
            match model::decode_event(&message).expect("typed") {
                EventDecode::Known(Event::Windows(windows)) => windows,
                other => panic!("{other:?}"),
            }
        });
        let data = if label.starts_with("live") {
            &live_data
        } else {
            &synthetic_data
        };
        measure(&format!("model_direct {label}"), 200, || {
            codec::decode::<Windows>(data).expect("model")
        });
        measure(&format!("envelope+selected {label}"), 200, || {
            let message = Message::decode(frame).expect("envelope");
            let Message::Event { data, .. } = &message else {
                panic!("event");
            };
            codec::decode::<Vec<SelectedWindow<'_>>>(data.as_raw_str().as_bytes())
                .expect("projection")
        });
    }
    measure("framing warm receive live-windows×16", 200, || {
        let mut framer = Framer::new(limits);
        framer.push(&live_stream).expect("push");
        while let Some(frame) = framer.next_frame() {
            std::hint::black_box(frame);
        }
    });
    // Steady-state receive on one reused Framer: allocations only appear as
    // growth/compaction events, so report the delta over a second turn.
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
        "framing reused-Framer second turn             steady {:>5} calls {:>8} B",
        ALLOCATOR.calls.load(Ordering::Relaxed),
        ALLOCATOR.bytes.load(Ordering::Relaxed)
    );
}
