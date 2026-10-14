---
type: Report
title: Sonic-only protocol performance baseline
description: First measured Criterion baseline of umbriel-protocol framing, envelope, typed-model, and selected-projection decode paths on sanitized live and scaled synthetic fixtures, with allocation accounting.
resource: design/performance/bench0.glm53m.md
status: draft
tags: [umbriel, performance, benchmarks, sonic, baseline]
generated:
  at: 2026-10-14
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ec7597a7affeFbugqOy35TD50A
    title: Live capture, sanitization, fixture generation, bench implementation, benchmark session, allocation probes, and this report
  - resource: json0.gpt6a.md
    title: Sonic selection context, source-derived scratch and lazy-path facts, and the measurement design this baseline follows
  - resource: ../protocol/commands0.glm53m.md
    title: Wire shapes, event frame key order, and live-session size reference
  - resource: ../architecture/architecture1.gpt6a.md
    title: Sonic-first direction and the measurement questions this baseline serves
  - resource: ../../crates/umbriel-protocol/README.md
    title: Benchmarked codec, envelope, and typed-event surfaces
  - resource: ../../crates/umbriel-protocol/benches/fixtures/README.md
    title: Committed fixtures, provenance, and exact sanitization rules
  - resource: https://docs.rs/crate/sonic-rs/0.5.10/source/src/serde/de.rs
    title: Resolved 0.5.10 source checked for per-call scratch and string borrowing branches
  - resource: https://docs.rs/crate/sonic-simd/0.1.4/source/src/lib.rs
    title: Resolved 0.1.4 source checked for compile-time target-feature SIMD guards
---

# Sonic-only protocol performance baseline

## Situation and ask

First measured performance baseline of the implemented protocol crate under
ticket **umbrs-protocol-benchmarks**. Sonic is the selected backend
([json0](json0.gpt6a.md), [architecture1](../architecture/architecture1.gpt6a.md));
nothing here compares parsers or reopens that choice. Library behavior is
unchanged — all new code lives in `crates/umbriel-protocol/benches/`, its
fixtures, and this report. The benchmarked objects:

1. **Reusable framing** (`codec::Framer`) under complete and chunked input,
   plus isolated drain and cold-growth shapes. No JSON is parsed.
2. **Direct full-model decoding** (`codec::decode::<Windows/Workspaces>`) on
   the payload bytes of real event frames.
3. **The real lazy envelope path** (`Message::decode`) and its typed
   (`model::decode_event`) and selected borrowed-projection extensions —
   exactly the paths a consumer uses.

Matched output semantics where paths are compared: for every fixture, the
startup code (never timed) asserts `envelope+typed` and `model_direct`
produce equal owned models, and that the selected projection agrees with the
full model on `id`/`active`/`title`. The selected projection is deliberately
narrower, so it is reported, not treated as equivalent. Fixture loading is
excluded from all timed loops.

## Build and environment identity

One clearly identified build; no portable/native split yet.

| Setting | Value |
|---|---|
| rustc / cargo | 1.101.0-nightly (5c543b0b8 / 3d7cf6e93, 2026-09-25/29) |
| CPU | AMD Ryzen 7 5800X, 8C/16T; `amd_pstate=active`, governor `powersave`, boost enabled |
| System load during session | load average ≈ 3–4; no concurrent cargo builds during timed runs |
| Profile | `cargo bench` = release defaults (opt-level 3, no LTO, 16 codegen units, no debug info); no `[profile]` overrides anywhere in the workspace |
| Target flags | `RUSTFLAGS` unset → portable x86-64 baseline (no `-C target-cpu`) |
| Locked versions | sonic-rs 0.5.10, sonic-simd 0.1.4, memchr 2.8.3, criterion 0.8.2 (dev-dependency, added via `cargo add`) |
| Statistics | Criterion: sample size 100, warm-up 500 ms, measurement 2 s per bench; tables below show medians with Criterion's slope CI in the source results files (spread was within ±2–7 % on this run) |

Source-derived build facts (resolved registry sources, not re-measured
here): sonic-simd compiles its SIMD structural helpers only behind
compile-time `target_feature`s (NEON on aarch64, simd128 on wasm32), so on
this portable x86-64 build those paths are scalar; UTF-8 validation through
simdutf8 is runtime-dispatched. sonic-rs's Serde deserializer allocates a
fresh empty `scratch: Vec<u8>` per `from_str`/`from_slice` call
(de.rs:30,41), growing only when an escaped string is decoded
(de.rs:476,634). Both facts are visible in the measured allocation counts
below.

## Fixture provenance

A single read-only live acquisition on 2026-10-14: two one-shot query
connections (`windows`, `workspaces`) to the running compositor at
`/run/user/1000/umbriel-wayland-0.sock`. No `subscribe`, no mutating
commands, no load. Raw replies (29 windows, 8,355 B; 8 workspaces, 1,088 B —
matching the 2026-10-13 probe in
[commands0](../protocol/commands0.glm53m.md#live-verification-read-only-probes-2026-10-13))
are preserved only in gitignored `.test-agent/performance-baseline/raw/`
and never committed.

Sanitization is content-blind and length-preserving (details in the
[fixtures README](../../crates/umbriel-protocol/benches/fixtures/README.md)):
personal string fields are rewritten character-class-wise (`x`/`X`/`d`/
multibyte placeholders), pids remapped with the same digit count, geometry
and flags kept, payload byte length unchanged. Distinct same-shape
identifiers can collide (both outputs became `XX-d`); identical originals
map identically, so cross-references survive.

| Fixture | Bytes (incl. LF) | Entries | Escaped titles | Multibyte | SHA-256 |
|---|---|---|---|---|---|
| live-windows | 8,375 | 29 (1 active, 6 focused) | 0 | 1 | `8b3c231ba36799f635568d4e7e096ef9f21b27f373297b11fea550bed3399c60` |
| live-workspaces | 1,111 | 8 (2 active groups, 1 focused) | 0 | 0 | `88c8315a94b7bcf7ae5de7107959e39cba77dd8de9d66cba88133a63ea62f9db` |
| synthetic-windows-10 | 3,126 | 10 | 3 | 2 | `e7516920e89df96a3cc8f1025ddc0b92488c310314c712dc310e3dfc6cbdf572` |
| synthetic-windows-50 | 15,447 | 50 | 13 | 12 | `34861f427144d16fe0aa7017543d0457fdea5f464dccb54c44901381b5c49be0` |
| synthetic-windows-200 | 61,750 | 200 | 50 | 50 | `2aac559c4b2139224706a11d0215e3910654b0dfdd3cf4b2f63ec8ca52d1752c` |

Raw captures (never committed): windows `a739251b…dbb4aa2`, workspaces
`de8cd2de…e4e6cfdde`. The synthetic fixtures are deterministic and
index-derived, and carry the escape/multibyte title mix the live session
happened to lack — important: **the live fixture exercises the borrowed fast
path only** (zero escaped titles), so escape costs appear in synthetic rows.

## Results

### Framing (reusable Framer; no JSON)

Streams: 16× live lines (134,000 B), 4× synthetic-200 (247,000 B). Warm =
one reused `Framer` across iterations (steady-state receive, capacity
retained, compaction included). Cold = fresh `Framer` per iteration.

| Bench | Median time | Throughput |
|---|---|---|
| receive live-windows×16, complete | 5.19 µs | 24.06 GiB/s |
| receive live-windows×16, 4096 B chunks | 4.38 µs | 28.48 GiB/s |
| receive live-windows×16, 64 B chunks | 29.20 µs | 4.27 GiB/s |
| receive live-workspaces×16, complete | 647 ns | 25.58 GiB/s |
| receive synthetic-200×4, complete | 9.69 µs | 23.75 GiB/s |
| receive synthetic-200×4, 4096 B chunks | 7.96 µs | 28.92 GiB/s |
| cold-complete live-windows×16 | 5.26 µs | 23.71 GiB/s |
| cold-complete synthetic-200×4 | 9.69 µs | 23.75 GiB/s |
| drain-only live-windows×16 | 1.68 µs | 74.46 GiB/s |
| drain-only synthetic-200×4 | 2.63 µs | 87.38 GiB/s |

Cold ≈ warm at these sizes (single exact-size capacity growth; see
allocations). 4 KiB chunks are ~15–20 % cheaper per byte than one huge push
(smaller per-call scan ranges); 64 B chunks collapse to 4.27 GiB/s from
per-chunk call overhead — 131 chunks per live line ≈ 14 ns per push+drain
turn. Per real frame, framing a live windows line costs ≈ 0.32 µs.

### Decode (whole wire frames, framing excluded)

| Fixture (frame B) | envelope | envelope+typed | model_direct | envelope+selected |
|---|---|---|---|---|
| live-windows (8,374) | 13.74 µs / 581 MiB/s | 37.33 µs / 214 MiB/s | 23.66 µs / 338 MiB/s | 32.24 µs / 248 MiB/s |
| synthetic-10 (3,125) | 5.47 µs / 545 MiB/s | 14.09 µs / 211 MiB/s | 7.99 µs / 373 MiB/s | 12.47 µs / 239 MiB/s |
| synthetic-50 (15,446) | 26.51 µs / 556 MiB/s | 64.83 µs / 227 MiB/s | 42.11 µs / 350 MiB/s | 57.67 µs / 255 MiB/s |
| synthetic-200 (61,749) | 102.92 µs / 572 MiB/s | 273.20 µs / 216 MiB/s | 161.23 µs / 365 MiB/s | 253.40 µs / 232 MiB/s |
| live-workspaces (1,110) | 2.19 µs / 482 MiB/s | 5.41 µs / 196 MiB/s | 3.45 µs / 307 MiB/s | — |

Marginal cost of the second pass (path minus `envelope`), against a direct
model parse on the same payload:

| Fixture | typed second pass | selected second pass |
|---|---|---|
| live-windows | 23.59 µs ≈ 1.00 × direct | 18.50 µs ≈ 0.78 × direct |
| synthetic-10 | 8.62 µs ≈ 1.08 × | 7.00 µs ≈ 0.87 × |
| synthetic-50 | 38.32 µs ≈ 0.91 × | 31.16 µs ≈ 0.74 × |
| synthetic-200 | 170.28 µs ≈ 1.06 × | 150.48 µs ≈ 0.93 × |

### Allocations (separate counting-allocator binary; counting never timed)

Median over 200 armed calls; first call after process start was identical in
every row (malloc arenas already warmed by fixture loading).

| Path | live-windows (29) | synthetic-50 |
|---|---|---|
| envelope (`Message::decode`) | **0 calls / 0 B** | 0 calls / 0 B |
| envelope+typed | 149 calls / 13,917 B | 269 calls / 29,282 B |
| model_direct | 149 calls / 13,917 B | 269 calls / 29,282 B |
| envelope+selected | 62 calls / 4,795 B | 108 calls / 9,792 B |
| framing, fresh `Framer`, complete push 134,000 B | 1 call / 134,000 B | — |
| framing, reused `Framer`, second full turn | **0 calls / 0 B** | — |

Follow-up probe (scratch, same counting method) decomposing the projection
count on live-windows: `Vec<{id: &str}>` = 4 calls / 960 B (pure `Vec`
growth); `Vec<{id: Cow<str>}>` = 33 calls / 2,368 B — **one allocation per
`Cow` field, byte-identical to a `String` field**, even though the live
fixture's strings are unescaped and sonic's parser provably returns borrowed
input slices for them (parser.rs:998–999). So on this path serde's `Cow`
glue pays the owned cost anyway; only a plain `&'a str` projection (or a
borrow-first visitor like the envelope's internal `Text`) is allocation-free.

## Findings

1. **Framing is not the problem.** Complete-input framing runs at 24–29
   GiB/s (memchr scanning plus compaction); one live windows line frames in
   ≈ 0.32 µs — 2.4 % of even the cheapest full-frame decode (13.74 µs). The
   reused-`Framer` steady turn allocates nothing and copies nothing but its
   own compaction, confirming the design goal of reusable receive storage.
2. **The lazy envelope is one full validation pass at 482–581 MiB/s and zero
   allocations.** Throughput is nearly flat from 10 to 200 windows (545 →
   572 MiB/s); no superlinear behavior at these sizes. Whole-frame
   validation — UTF-8 check, structural skip of the payload, trailing-input
   enforcement — is the entire cost.
3. **The typed path pays twice.** `envelope+typed` costs 2.2–2.7 × the
   direct model parse of the same payload: the envelope validates the whole
   frame lazily, then `decode_event` re-parses the payload span through
   `codec::decode` (including a second UTF-8 check of that span). The second
   pass costs 0.91–1.08 × a direct parse — a full parse, not a discount
   re-scan.
4. **The selected projection wins, but modestly, and its `Cow` fields
   allocate.** Total `envelope+selected` beats `envelope+typed` by 7–14 %
   across fixtures while decoding 3 of 17 fields; its marginal second pass
   is 0.74–0.93 × a direct parse. Measured allocations show 2 per row from
   the two `Cow` fields even on unescaped input; `&'a str` fields would be
   free (probe above).
5. **Model decoding scales linearly** at 307–373 MiB/s. The live fixture
   (all-unescaped strings) parses at 338 MiB/s and the escape-bearing
   synthetic fixtures at 350–373 MiB/s; the difference is small and
   confounded by fixture composition (title lengths, field mix), so no
   escape-cost claim is made beyond allocation counts.

## Limits and assumptions

- Single machine, single session, `powersave` governor, background load
  ≈ 3–4: absolute numbers are a baseline anchor, not a record; Criterion's
  CIs (in the saved results files) carried ±2–7 %.
- Warm-state assumptions: decode benches decode the same preloaded bytes
  each iteration (Sonic input is immutable, so no restore is needed);
  framing warm benches reuse one `Framer`; sonic's per-call scratch `Vec`
  is constructed inside every measured call (source-derived), and grows
  only when escaped strings are decoded — escaped and unescaped fixtures
  differ in entry count, so their allocation totals are not directly
  comparable row-to-row.
- One portable x86-64 build. A `-C target-cpu=native` variant was not run;
  per the source facts above it mainly affects simdutf8 validation
  frequency selection, not sonic's (scalar on x86) structural paths.
- The live fixture reflects one session state (29 windows, two output
  groups, no escaped titles, one multibyte title). Escape-bearing
  distributions come from the invented synthetic mix (¼ escaped, ¼
  multibyte, ½ plain).
- Allocation counts are allocator call counts and requested bytes (realloc
  counted once at new size), not peak RSS or copy volume; `first-call` rows
  describe a process already warmed by fixture loading.
- Chunked framing used size-aligned splits from byte 0; split positions
  inside escapes/UTF-8 are a decode-correctness concern already covered by
  tests, and framing itself is LF-only, so this does not bias framing
  numbers.

## Actionable bottleneck and next experiments

**Decode dominates framing by ~40–90 × per byte; within decode, the typed
event path's double pass is the one measured, structural cost worth
attacking.** No core code was changed. Concrete next experiments, in order
of expected value:

1. **Single-pass typed envelope**: let the envelope's map visitor deserialize
   known-family payloads directly (the `data` value drives a typed
   deserializer seeded by the already-seen `event` key, or the visitor
   retains the raw span and the typed decode replaces the lazy validation
   pass). Whole-frame validation semantics must be preserved (json0's
   warning about finalization stands). Measured ceiling for the win: the
   live `envelope+typed` 37.33 µs contains a ≈ 23.6 µs second parse that a
   fused path could largely absorb into the first.
2. **Borrowed-first projection fields**: the envelope's `Text` visitor
   pattern applied to narrow projections (try `visit_borrowed_str`, own only
   on escape) would remove the measured 1-alloc-per-`Cow`-field cost while
   keeping escape correctness. Requires a small projection helper in the
   crate, not a benchmark change.
3. **Native-build comparison run** (cheap, same benches, `RUSTFLAGS=-C
   target-cpu=native`) if absolute x86 throughput ever becomes the target.

## Reproduction

```sh
cargo bench -p umbriel-protocol --bench framing
cargo bench -p umbriel-protocol --bench decode
cargo bench -p umbriel-protocol --bench allocations   # plain binary, prints table
```

Verified alongside the session: `cargo fmt -p umbriel-protocol --check`,
`cargo clippy -p umbriel-protocol --all-targets -- -D warnings`,
`cargo nextest run -p umbriel-protocol` (58/58 pass; no timing gates added).
Raw outputs, provenance record, and the allocation probe live in gitignored
`.test-agent/performance-baseline/`.

## Cross-references

- [JSON research](json0.gpt6a.md) — Sonic selection, scratch/borrowing
  facts, and the three-layer measurement design this implements.
- [Commands research](../protocol/commands0.glm53m.md) — wire shapes, key
  order, and the earlier live probe this capture matches.
- [Architecture](../architecture/architecture1.gpt6a.md) — the Sonic-first,
  measure-when-questions-arise direction; this is the first such
  measurement.
- [Fixtures README](../../crates/umbriel-protocol/benches/fixtures/README.md)
  — provenance and sanitization rules for the committed fixtures.
