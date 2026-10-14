//! Reusable-framing benchmarks: complete and chunked input, drain, cold growth.
//! Pure framing: nothing here decodes JSON. Fixture bytes are loaded and
//! chunked at startup; measured loops only push, scan, and drain.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use umbriel_protocol::codec::{Framer, Limits};

mod common;

/// One warm receive turn: feed every chunk, draining after each push exactly
/// as a receive loop would, so storage never grows without bound.
fn receive_turn(framer: &mut Framer, chunks: &[&[u8]]) {
    for chunk in chunks {
        framer.push(chunk).expect("push within limits");
        while let Some(frame) = framer.next_frame() {
            std::hint::black_box(frame);
        }
    }
    assert_eq!(framer.buffered_len(), 0, "all frames drained");
}

fn receive_benches(c: &mut Criterion) {
    let limits: Limits = common::bench_limits();
    let windows_line = common::fixture("live-windows.json");
    let workspaces_line = common::fixture("live-workspaces.json");
    let synthetic200_line = common::fixture("synthetic-windows-200.json");

    struct Input {
        name: &'static str,
        stream: Vec<u8>,
        chunking: &'static [usize],
    }
    // 16× live lines ≈ 134 KiB; 4× synthetic-200 ≈ 247 KiB.
    let inputs = [
        Input {
            name: "live-windows×16",
            stream: common::repeat_frames(&windows_line, 16),
            chunking: &[usize::MAX, 4096, 64],
        },
        Input {
            name: "live-workspaces×16",
            stream: common::repeat_frames(&workspaces_line, 16),
            chunking: &[usize::MAX],
        },
        Input {
            name: "synthetic-200×4",
            stream: common::repeat_frames(&synthetic200_line, 4),
            chunking: &[usize::MAX, 4096],
        },
    ];

    let mut group = c.benchmark_group("framing/receive");
    group
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    for input in &inputs {
        for size in input.chunking {
            let chunks: Vec<&[u8]> = if *size == usize::MAX {
                vec![input.stream.as_slice()]
            } else {
                common::chunks(&input.stream, *size)
            };
            // Warm-state measurement: one reused Framer across iterations,
            // matching a long-lived receive path (capacity retained, periodic
            // compaction included).
            let mut framer = Framer::new(limits);
            group.throughput(Throughput::Bytes(input.stream.len() as u64));
            group.bench_with_input(
                criterion::BenchmarkId::new(
                    input.name,
                    if *size == usize::MAX {
                        "complete".to_string()
                    } else {
                        format!("{size}B-chunks")
                    },
                ),
                &chunks,
                |b, chunks| {
                    b.iter(|| receive_turn(&mut framer, chunks));
                },
            );
        }
    }
    group.finish();
}

fn cold_benches(c: &mut Criterion) {
    let limits = common::bench_limits();
    let windows = common::repeat_frames(&common::fixture("live-windows.json"), 16);
    let synthetic = common::repeat_frames(&common::fixture("synthetic-windows-200.json"), 4);

    let mut group = c.benchmark_group("framing/cold-complete");
    group
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    for (name, stream) in [
        ("live-windows×16", &windows),
        ("synthetic-200×4", &synthetic),
    ] {
        group.throughput(Throughput::Bytes(stream.len() as u64));
        group.bench_with_input(name, stream, |b, stream| {
            // Fresh Framer per iteration: capacity growth from empty is part
            // of the measured work (first-connect receive shape).
            b.iter_batched(
                || Framer::new(limits),
                |mut framer| receive_turn(&mut framer, &[stream.as_slice()]),
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn drain_benches(c: &mut Criterion) {
    let limits = common::bench_limits();
    let windows = common::repeat_frames(&common::fixture("live-windows.json"), 16);
    let synthetic = common::repeat_frames(&common::fixture("synthetic-windows-200.json"), 4);

    let mut group = c.benchmark_group("framing/drain-only");
    group
        .warm_up_time(std::time::Duration::from_millis(500))
        .measurement_time(std::time::Duration::from_secs(2));
    for (name, stream) in [
        ("live-windows×16", &windows),
        ("synthetic-200×4", &synthetic),
    ] {
        group.throughput(Throughput::Bytes(stream.len() as u64));
        group.bench_with_input(name, stream, |b, stream| {
            // Isolate frame-boundary scanning/drain from byte ingestion: the
            // push happens in untimed setup.
            b.iter_batched(
                || {
                    let mut framer = Framer::new(limits);
                    framer.push(stream).expect("push within limits");
                    framer
                },
                |mut framer| {
                    while let Some(frame) = framer.next_frame() {
                        std::hint::black_box(frame);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, receive_benches, cold_benches, drain_benches);
criterion_main!(benches);
