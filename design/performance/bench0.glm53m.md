---
type: Report
title: Sonic-only protocol performance baseline
description: First measured Criterion baseline of umbriel-protocol framing, envelope, typed-model, and selected-projection decode paths on sanitized live and scaled synthetic fixtures, with allocation accounting; revised after review corrections.
resource: design/performance/bench0.glm53m.md
status: draft
tags: [umbriel, performance, benchmarks, sonic, baseline]
generated:
  at: 2026-10-14
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent review corrected borrowing, SIMD and measurement interpretations; linked the implemented architecture and measured follow-up
  - resource: opencode://ses_ec7597a7affeFbugqOy35TD50A
    title: Live capture, sanitization, fixtures, benches, benchmark sessions, allocation probes, review corrections of serde-borrow, SIMD facts, ratios, copying, and statistics, and this report
  - resource: json0.gpt6a.md
    title: Sonic selection context, source-derived scratch facts, the serde(borrow) warning this baseline initially violated, and the measurement design
  - resource: ../protocol/commands0.glm53m.md
    title: Wire shapes, data-before-event key order, and live-session size reference
  - resource: ../architecture/architecture1.gpt6a.md
    title: Sonic-first direction and the measurement questions this baseline serves
  - resource: ../../crates/umbriel-protocol/README.md
    title: Benchmarked codec, envelope, and typed-event surfaces
  - resource: ../../crates/umbriel-protocol/benches/fixtures/README.md
    title: Committed fixtures, provenance, and exact sanitization rules
  - resource: https://docs.rs/crate/sonic-simd/0.1.4/source/src/lib.rs
    title: Resolved 0.1.4 source for the sse2/avx2/avx512 selection and v256/v512 pair-composition facts
  - resource: https://docs.rs/crate/sonic-rs/0.5.10/source/src/serde/de.rs
    title: Resolved 0.5.10 source checked for per-call scratch and string borrowing branches
  - resource: ../../crates/umbriel-protocol/src/model/vocabulary.rs
    title: string_enum Deserialize always allocating a transient String before spelling dispatch
---

# Sonic-only protocol performance baseline

## Situation and ask

First measured performance baseline of the implemented protocol crate under
ticket **umbrs-protocol-benchmarks**. Sonic is the selected backend
([json0](json0.gpt6a.md), [architecture1](../architecture/architecture1.gpt6a.md));
nothing here compares parsers or reopens that choice. Library behavior is
unchanged — all new code lives in `crates/umbriel-protocol/benches/`, its
fixtures, and this report. This revision corrects first-draft errors found in
parent review: a missing `#[serde(borrow)]` on the projection's `Cow` fields,
false scalar-on-x86 SIMD claims, wrong ratio arithmetic, a false
zero-copy framing claim, mislabeled statistics and throughput normalization,
and mislabeled allocation rows. The benchmarked objects:

1. **Reusable framing** (`codec::Framer`) under complete and chunked input,
   plus isolated drain and cold-growth shapes. No JSON is parsed.
2. **Direct full-model decoding** (`codec::decode::<Windows/Workspaces>`) on
   the payload bytes of real event frames.
3. **The real lazy envelope path** (`Message::decode`) and its typed
   (`model::decode_event`) and selected-projection extensions — exactly the
   paths a consumer uses.

Matched output semantics where paths are compared: for every fixture, the
startup code (never timed) asserts `envelope+typed` and `model_direct`
produce equal owned models, and that the selected projection agrees with the
full model on `id`/`active`/`title` **and** decodes `Cow::Borrowed` for
unescaped values / `Cow::Owned` for escaped ones. The paths do **not**
consume the same input: the envelope paths consume and validate the whole
frame; `model_direct` consumes only the extracted payload span. Throughput
below is therefore labeled per input. Fixture loading is excluded from all
timed loops.

## Build and environment identity

One clearly identified build; no portable/native split yet.

| Setting | Value |
|---|---|
| rustc / cargo | 1.101.0-nightly (5c543b0b8 / 3d7cf6e93, 2026-09-25/29) |
| CPU | AMD Ryzen 7 5800X, 8C/16T (Zen 3, AVX2-capable, no AVX-512); `amd_pstate=active`, governor `powersave`, boost enabled |
| System load during sessions | load average ≈ 3–4; no concurrent cargo builds during timed runs |
| Profile | `cargo bench` = release defaults (opt-level 3, no LTO, 16 codegen units, no debug info); no `[profile]` overrides anywhere in the workspace |
| Target flags | `RUSTFLAGS` unset → portable x86-64 baseline (no `-C target-cpu`) |
| Locked versions | sonic-rs 0.5.10, sonic-simd 0.1.4, memchr 2.8.3, criterion 0.8.2 (dev-dependency, added via `cargo add`) |

Source-derived build facts (read from the resolved registry sources; no
runtime ranking is claimed from them): sonic-simd selects its vector widths
at compile time (`src/lib.rs`). The 128-bit layer picks SSE2 first
(`target_feature = "sse2"`, lines 9–11) — and **x86-64 baseline guarantees
SSE2**, so this portable build compiles SSE2 SIMD structural paths, not
scalar code; NEON (aarch64) and wasm128 follow, with a scalar `v128` fallback
only for targets with none of them. The 256-bit layer uses the `avx2` module
when `target_feature = "avx2"` (lines 26–28); otherwise `v256.rs` composes
`Simd256` **as pairs of the selected 128-bit type** — on this portable build
that means 256-bit-wide algorithms execute as two SSE2 lanes. The 512-bit
layer requires both `target_feature = "avx512f"` and sonic-rs's opt-in
`avx512` cargo feature; otherwise `v512.rs` composes 512-bit ops as pairs of
the selected 256-bit type (itself possibly paired SSE2). Consequences: a
`-C target-cpu=native` build on this CPU would enable genuine 256-bit AVX2
for sonic's structural algorithms (not merely UTF-8 validation frequency
selection in simdutf8, which is runtime-dispatched and separate); AVX-512
paths would stay composed unless the opt-in feature is also enabled. This
native variant is **unmeasured** here.

sonic-rs's Serde deserializer allocates a fresh empty `scratch: Vec<u8>` per
`from_str`/`from_slice` call (de.rs:30,41), growing only when an escaped
string is decoded (de.rs:476,634). Both facts are visible in the measured
allocation counts below.

## Statistic and normalization conventions

All timing numbers below are the **median** from Criterion's saved
`estimates.json` (95 % confidence intervals preserved in the saved results
under `.test-agent/performance-baseline/`; scratch also holds the console
slope output). Criterion's console point estimate is a slope, not a median,
and differs slightly (e.g. `model_direct` on live-windows: console slope
23.66 µs vs median 21.80 µs). Throughput is labeled per input: envelope
paths are normalized to **full frame bytes** (envelope keys included);
`model_direct` is normalized to its actual input, the **payload span**, with
the frame-normalized figure in parentheses for cross-reference.

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

| Bench | Median | Throughput |
|---|---|---|
| receive live-windows×16, complete | 5.164 µs | 24.17 GiB/s |
| receive live-windows×16, 4096 B chunks | 4.317 µs | 28.91 GiB/s |
| receive live-windows×16, 64 B chunks | 28.54 µs | 4.37 GiB/s |
| receive live-workspaces×16, complete | 642 ns | 25.78 GiB/s |
| receive synthetic-200×4, complete | 9.711 µs | 23.69 GiB/s |
| receive synthetic-200×4, 4096 B chunks | 7.816 µs | 29.43 GiB/s |
| cold-complete live-windows×16 | 5.199 µs | 24.00 GiB/s |
| cold-complete synthetic-200×4 | 9.517 µs | 24.17 GiB/s |
| drain-only live-windows×16 | 1.586 µs | 78.69 GiB/s |
| drain-only synthetic-200×4 | 2.462 µs | 93.42 GiB/s |

Cold ≈ warm at these sizes (see allocations: one exact-size capacity growth).
4 KiB chunks are ~15–20 % cheaper per byte than one huge push (smaller
per-call scan ranges); 64 B chunks collapse to 4.37 GiB/s from per-chunk call
overhead — 131 chunks per live line ≈ 14 ns per push+drain turn. Per real
frame, framing a live windows line costs ≈ 0.32 µs. Note the copying model:
`push` always copies incoming bytes into the Framer's contiguous storage
(`extend_from_slice`), and compaction moves unconsumed bytes when triggered;
the reused-`Framer` result below shows **zero allocator calls, not zero
copying** — one input-copy per byte plus occasional compaction moves is the
designed cost of reusable borrowed-frame storage.

### Decode (whole wire frames, framing excluded)

`model_direct` consumes the payload span only (e.g. 8,347 of 8,374
live-windows bytes); the other paths consume and validate the full frame.

| Fixture | envelope (frame-norm) | envelope+typed (frame-norm) | model_direct (payload-norm) | envelope+selected (frame-norm) |
|---|---|---|---|---|
| live-windows (8,374 B) | 13.71 µs / 582.5 MiB/s | 35.84 µs / 222.8 MiB/s | 21.80 µs / 365.1 MiB/s (366.3 frame) | 28.89 µs / 276.4 MiB/s |
| synthetic-10 (3,125 B) | 5.49 µs / 542.9 MiB/s | 12.80 µs / 232.9 MiB/s | 7.59 µs / 389.5 MiB/s (392.8 frame) | 11.12 µs / 267.9 MiB/s |
| synthetic-50 (15,446 B) | 25.30 µs / 582.2 MiB/s | 63.58 µs / 231.7 MiB/s | 40.33 µs / 364.6 MiB/s (365.2 frame) | 53.33 µs / 276.2 MiB/s |
| synthetic-200 (61,749 B) | 101.27 µs / 581.5 MiB/s | 254.63 µs / 231.3 MiB/s | 156.90 µs / 375.2 MiB/s (375.3 frame) | 210.50 µs / 279.8 MiB/s |
| live-workspaces (1,110 B) | 2.10 µs / 503.2 MiB/s | 5.41 µs / 195.8 MiB/s | 3.26 µs / 315.8 MiB/s (324.5 frame) | — |

Ratios (medians): **total `envelope+typed` is 1.58–1.69 × the direct payload
decode** across fixtures (live 1.64×). The envelope stage is 39–43 % of the
typed total. The marginal typed second pass — `envelope+typed` minus
`envelope`, i.e. the re-parse of the payload span including its second UTF-8
check — costs **0.95–1.02 ×** a direct parse of the same payload: a full
parse, not a discounted re-scan. The selected projection's marginal second
pass costs 0.70–0.74 × a direct parse, and its total is 0.81–0.87 × the
typed total (13–19 % less).

### Allocations (separate counting-allocator binary; counting never timed)

Median over 200 armed calls; "steady" describes the measured path, not
allocator warmth — the process allocator is already warmed by fixture
loading before the first armed call. First call after process start was
identical in every row.

| Path | live-windows (29) | synthetic-50 |
|---|---|---|
| envelope (`Message::decode`) | **0 calls / 0 B** | 0 calls / 0 B |
| envelope+typed | 149 calls / 13,917 B | 269 calls / 29,282 B |
| model_direct | 149 calls / 13,917 B | 269 calls / 29,282 B |
| envelope+selected, `#[serde(borrow)]` | **4 calls / 3,360 B** | 21 calls / 7,534 B |
| envelope+selected, no `borrow` (**misconfiguration comparison**) | 62 calls / 4,795 B | 108 calls / 9,792 B |
| framing, cold/fresh `Framer`, first complete turn (134,000 B) | 1 call / 134,000 B | — |
| framing, **reused** `Framer`, second full turn | **0 calls / 0 B** | — |

The selected projection's `Cow` fields carry field-level `#[serde(borrow)]`;
startup verification (and a scratch probe) confirms 29/29 borrowed ids and
29/29 borrowed titles on live-windows and 50/50 borrowed ids with 37/50
borrowed titles (the 13 escaped ones) on synthetic-50. With the annotation,
live-windows decodes the whole projection with only the 4 `Vec`-growth
allocations; synthetic-50's 21 calls decompose as ≈ 5 `Vec` growth + 13
owned escaped titles + scratch growth during unescaping (split consistent
with counts; not individually instrumented). The **no-`borrow` rows are the
first draft's numbers, preserved deliberately**: plain `Cow<'a, str>` without
the annotation deserializes every field owned — exactly like `String` —
which was my bench misconfiguration, not a Sonic/Serde limitation. The
timing effect of the annotation was measured by re-running only the selected
rows: medians fell 5–11 % (live 31.17 → 28.89 µs; synthetic-200 232.5 →
210.5 µs).

The full-model rows also decompose cleanly by count. live-windows' 149 calls
= 29 windows × 5 + 4 `Vec` growth: four owned model strings per row (`id`,
`workspace`, `app_id`, `title`; `scratchpad`/`xdg_tag` are empty and do not
allocate) **plus one transient `String` per `content_type`** — every live
value is the known spelling `"none"`, yet the allocation happens because
[model/vocabulary.rs:101-105](../../crates/umbriel-protocol/src/model/vocabulary.rs)
deserializes every `string_enum` through `String::deserialize(...).map(Self::from)`,
allocating first and dispatching the spelling afterwards. Attribution is by
count fit (149 = 29×5 + 4 exactly), not per-field instrumentation; see
finding 6 and its experiment.

## Findings

1. **Framing is not the problem.** Complete-input framing runs at 24–29
   GiB/s; one live windows line frames in ≈ 0.32 µs — 2.4 % of even the
   cheapest full-frame decode (13.71 µs). The reused-`Framer` turn makes
   zero allocator calls but still copies every input byte into storage once
   (plus periodic compaction) — that single copy is the designed price of
   borrowed frame slices, not overhead to eliminate.
2. **The lazy envelope is one full validation pass at 503–583 MiB/s
   (frame-normalized) and zero allocations.** Throughput is nearly flat from
   10 to 200 windows (542.9 → 581.5 MiB/s); no superlinear behavior at these
   sizes. Whole-frame validation — UTF-8 check, structural skip of the
   payload, trailing-input enforcement — is the entire cost.
3. **The typed path pays for one extra full parse.** Total
   `envelope+typed` is 1.58–1.69 × the direct payload decode; its envelope
   stage (39–43 % of the total) validates the whole frame lazily, then
   `decode_event` re-parses the payload span through `codec::decode`
   (including a second UTF-8 check of that span) at 0.95–1.02 × the cost of
   a direct parse.
4. **The selected projection with `#[serde(borrow)]` is the cheapest
   consumer path measured: 13–19 % under the typed total, 0.70–0.74 × a
   direct parse for its second pass, and effectively allocation-free on
   unescaped input** (4 `Vec`-growth calls on live-windows). Escaped values
   correctly decode `Cow::Owned`. Forgetting the annotation silently pays
   one allocation per `Cow` field and 5–11 % more time — the exact trap
   [json0](json0.gpt6a.md) warned about; the fix is the standard serde
   attribute, not a new library helper.
5. **Direct model decoding runs at 315.8–389.5 MiB/s of payload** across
   fixtures. The live fixture's all-unescaped strings and the synthetic
   escape-bearing mixes land in the same band, so no escape throughput
   claim is made beyond the allocation counts.
6. **Known enum spellings still cost one transient allocation each.**
   Source ([model/vocabulary.rs:101-105](../../crates/umbriel-protocol/src/model/vocabulary.rs)):
   `string_enum`'s `Deserialize` runs `String::deserialize` and only then
   dispatches the spelling, so a known `content_type`/`layout`/`layer`
   value allocates a `String` and immediately discards it. The counting
   data fit this exactly on live-windows (149 = 29×5 + 4; the fifth
   per-row allocation is `content_type`, whose live values are all known).
   This is model-layer cost, separate from the projection's missing-borrow
   misconfiguration, and avoidable in principle — see the experiments.

## Limits and assumptions

- Single machine, single session, `powersave` governor, background load
  ≈ 3–4: absolute numbers are a baseline anchor, not a record; Criterion's
  95 % CIs carried ±2–7 %.
- Warm-state assumptions: decode benches decode the same preloaded bytes
  each iteration (Sonic input is immutable, so no restore is needed);
  framing warm benches reuse one `Framer`; sonic's per-call scratch `Vec`
  is constructed inside every measured call (source-derived), and grows
  only when escaped strings are decoded. Escaped and unescaped fixtures
  differ in entry count, so allocation totals are not comparable row-to-row
  across fixtures.
- One portable x86-64 build with SSE2 structural paths (see build facts);
  the native variant is unmeasured and no runtime ordering between them is
  claimed.
- The live fixture reflects one session state (29 windows, two output
  groups, no escaped titles, one multibyte title). Escape-bearing
  distributions come from the invented synthetic mix (¼ escaped, ¼
  multibyte, ½ plain).
- Allocation counts are allocator call counts and requested bytes (realloc
  counted once at new size), not peak RSS or copy volume; the Framer rows
  show allocator calls, which is not the same as zero byte copying.
- Chunked framing used size-aligned splits from byte 0; framing is LF-only,
  so split position cannot bias framing numbers.

## Actionable bottleneck and next experiments

**Decode dominates framing by ~40–90 × per byte; within decode, the typed
event path's second payload pass is the one measured, structural cost worth
attacking.** No core code was changed. Concrete next experiments, in order
of expected value:

1. **Single-pass typed envelope.** A fused path must still *build* the typed
   model, so it cannot inherit envelope-only cost; the realistic target is
   to absorb the separate validation pass. Measured opportunity estimate:
   `envelope+typed` minus `model_direct` ≈ 14.0 µs on live-windows (≈ 38 %
   of the typed total; 38–42 % across fixtures) — an upper-bound estimate of
   what a single traversal could save, **not a proven achievable ceiling**.
   Constraint: on this wire `data` precedes `event` (alphabetical server
   key order), so an event-first-seeded typed deserializer cannot assume
   family knowledge before the payload arrives; the visitor must retain the
   raw payload span and late-bind, or decode without family preselection.
   Whole-frame validation semantics (json0's finalization warning) must be
   preserved.
2. **Use `#[serde(borrow)]` in consumer Cow projections.** No library
   change needed; the misconfigured comparison above quantifies the cost of
   forgetting it. The benches' startup assertions are the template for
   verifying Borrowed/Owned branch behavior.
3. **Borrow-aware `string_enum` deserialization** (library-side, kept
   stable until this baseline lands): deserialize by matching the spelling
   first — e.g. a visitor over `&str`/borrowed input that constructs the
   known variant directly and falls back to `Unknown(String)` only for
   unrecognized spellings. Supported claims: the source shows the
   allocation is unconditional today, and the counting fit shows one such
   call per enum field per row, so removing it bounds the saving at one
   allocator call per known enum value per row (≈ 29 of 149 calls, ~20 %,
   on live-windows). No throughput improvement is claimed — that would need
   its own measurement after any change.
4. **Native-build comparison run** (same benches, `RUSTFLAGS=-C
   target-cpu=native`) if absolute x86 throughput becomes the target: per
   the source facts it switches sonic-simd's 256-bit structural algorithms
   from paired SSE2 lanes to AVX2 (and simdutf8's runtime dispatch picks
   wider validation); unmeasured here.

## Reproduction

```sh
cargo bench -p umbriel-protocol --bench framing
cargo bench -p umbriel-protocol --bench decode
cargo bench -p umbriel-protocol --bench allocations            # plain binary, prints table
cargo bench -p umbriel-protocol --bench decode -- selected     # selected rows only
cargo bench -p umbriel-protocol --bench allocations -- framing # allocation rows filtered
```

Verified alongside the sessions: `cargo fmt -p umbriel-protocol --check`,
`cargo clippy -p umbriel-protocol --all-targets -- -D warnings`,
`cargo nextest run -p umbriel-protocol` (58/58 pass; no timing gates added).
The `allocations` bench binary is the workspace's one deliberate `unsafe`
exception — a forwarding `GlobalAlloc` impl confined to benchmark
instrumentation; the production crates keep `#![forbid(unsafe_code)]`. Raw
outputs, medians, provenance, and the allocation probe live in gitignored
`.test-agent/performance-baseline/`.

## Cross-references

- [Known-enum allocation addendum](enum-allocations0.gpt61s.md) — subsequent
  measured cleanup; the baseline numbers above retain their original revision.
- [Implemented architecture](../architecture/architecture2.gpt6a.md) — current
  placement of framing, decoding, delivery, materialization and query work.
- [JSON research](json0.gpt6a.md) — Sonic selection, scratch/borrowing
  facts (including the `serde(borrow)` warning), and the three-layer
  measurement design this implements.
- [Commands research](../protocol/commands0.glm53m.md) — wire shapes, key
  order (`data` before `event`), and the earlier live probe this capture
  matches.
- [Architecture](../architecture/architecture1.gpt6a.md) — the Sonic-first,
  measure-when-questions-arise direction; this is the first such
  measurement.
- [Fixtures README](../../crates/umbriel-protocol/benches/fixtures/README.md)
  — provenance and sanitization rules for the committed fixtures.
