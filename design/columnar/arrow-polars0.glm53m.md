---
type: Research
title: Polars and Apache Arrow columnar targets — overhead, conversion, and one emitter
description: Historical comparison of Polars and Apache Arrow batch types and conversion probes; the Apache-native bridge recommendation was overruled in favor of the now-implemented Polars-native path.
resource: design/columnar/arrow-polars0.glm53m.md
status: draft
generated:
  at: 2026-10-13
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent clarified the overruled recommendation and the limits of the unmeasured performance claims during implementation
  - resource: opencode://ses_ec8ca68ccffeDXLmRjrvmin5KH
    title: Investigated polars-arrow standalone use, measured overhead deltas, proved both C Data Interface bridges and the IPC fallback in a compile probe, and wrote this recommendation
  - resource: ../../README.md
    title: Project orientation and the current columnar direction
  - resource: ../handoff/handoff0.glm53m.md
    title: Governing pitfalls including the polars_arrow RecordBatch type warning and crate-isolation rule
  - resource: ../architecture/architecture1.gpt6a.md
    title: Current crate plan whose Polars-native batch direction this research re-examines
  - resource: ../assessment/reuse0.gpt6a.md
    title: Completed assessment whose Arrow/Polars boundary and deferred interop probe this work completes
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/Cargo.toml
    title: polars-arrow standalone dependency set, empty default features, and feature catalog
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/record_batch.rs
    title: RecordBatchT/RecordBatch definition and checked constructor
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/array/mod.rs
    title: ArrayRef = Box<dyn Array>, the exact type export_array_to_c consumes
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/ffi/mod.rs
    title: C Data Interface export/import entry points
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/ffi/array.rs
    title: Import dispatch including Utf8View, release-callback ownership, and PrivateData
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/datatypes/field.rs
    title: Field shape, metadata, and the Polars-specific metadata keys
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/frame/chunks.rs
    title: From<polars_arrow RecordBatch> for DataFrame zero-copy conversion
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/series/from.rs
    title: Utf8View no-cast branch versus Utf8/LargeUtf8 cast_chunks branch
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/datatypes/dtype.rs
    title: CompatLevel and String to Utf8View/LargeUtf8 mapping
  - resource: file:///home/rektide/archive/apache/arrow-rs/arrow-array/src/ffi.rs
    title: to_ffi/from_ffi/from_ffi_and_data_type and the align_buffers no-op path
  - resource: file:///home/rektide/archive/apache/arrow-rs/arrow-data/src/ffi.rs
    title: FFI_ArrowArray ptr::replace move semantics and release ownership
  - resource: file:///home/rektide/archive/apache/arrow-rs/arrow-schema/src/ffi.rs
    title: FFI_ArrowSchema layout, from_raw, and Field/DataType TryFrom imports
  - resource: file:///home/rektide/archive/apache/arrow-rs/arrow-array/Cargo.toml
    title: arrow-array dependency set and the non-default ffi feature
  - resource: file:///home/rektide/src/umbriel-rs/.test-agent/arrow-probe/
    title: Compile-and-run probe proving both bridges, DataFrame conversion, and the IPC fallback at pinned versions
verification:
  method: Read the archived Polars 0.55.1 and arrow-rs 59.2.0 sources for every claimed symbol; compiled and ran the probe against crates.io polars-arrow/polars/polars-utils 0.55.2 and arrow-array/arrow-schema/arrow-ipc 59.3.0 with value and buffer-pointer assertions.
  limits: No benchmarks were run; overhead claims are structural (copies versus shared buffers, dependency counts), not timing measurements. FFI buffer-pointer identity was asserted for view and primitive columns in the forward direction only; the reverse direction was verified by value equality. Nested types were covered only through LargeList<Utf8View>.
---

# Polars and Apache Arrow columnar targets

> **Ruling (user acceptance, 2026-10-13): Polars-native only — option A.**
> No C Data Interface bridge; Utf8View strings everywhere; Arrow IPC is the
> sanctioned interop fallback. This document is the evidence, not the ruling;
> the decision and its governing rules live in
> [architecture1's columnar section](../architecture/architecture1.gpt6a.md#materialized-state-and-columnar-output).
> The probe results below remain valid and are what makes the decision cheap
> to revisit: switching to an arrow-array emitter later costs zero copied data.

## Recommendation

**Performance qualification (parent review, 2026-10-14):** the probes below
check values and particular shared-buffer identities. No timing benchmark was
run, so statements of equal runtime cost or zero runtime overhead are not
established. The historical recommendation below remains overruled; the
[implemented crates](../../README.md#implemented-crates) use Polars-native types
and no C Data Interface bridge.

**Make `umbriel-arrow` emit `arrow_array::RecordBatch` natively, and put the
Polars conversion in a small adapter crate that owns a centralized, audited C
Data Interface bridge.** The bridge is proven zero-copy by the probe; the
runtime overhead difference between building batches on `polars-arrow` versus
`apache arrow-array` is **zero for our use case** — the same builder patterns,
the same view strings, the same bit-packed validity. What actually differs is
dependency weight, version coupling, and ecosystem reach. Targeting
arrow-array native buys version isolation (Polars bumps stop touching the
emitter), the whole Apache ecosystem (IPC, Parquet, DataFusion) without
crossing any bridge, and a public type the wider Rust world already knows.
Polars stays zero-copy through ~100 lines of bridge code confined to one crate.

The Polars-native option (what the current architecture leans toward) remains
**viable and performance-equal**; it is the right choice only if we accept that
apache-arrow consumers always cross a bridge and that every Polars release
moves the emitter crate. Dual emitters sharing a schema core is the worst
profile: two builder stacks kept in lockstep for no runtime gain.

The one hard rule either way: **standardize on `Utf8View` for every string
column.** That single dtype decision is worth more than the crate choice — see
§2.

This revises the "Polars-native batches are the leading representation" note in
[architecture1](../architecture/architecture1.gpt6a.md#materialized-state-and-columnar-output)
and completes the interop probe the
[assessment](../assessment/reuse0.gpt6a.md#arrow-and-polars-feasible-with-a-precise-rust-boundary)
deferred. Human acceptance of the revision is still required; this document is
the evidence, not the ruling.

## 1. polars-arrow standalone: yes, and it converts to DataFrame without copying

**polars-arrow is usable without any of the Polars query engine.** Its
[Cargo.toml](file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/Cargo.toml)
(lines 15–59) depends only on `polars-buffer`, `polars-error`, `polars-schema`,
`polars-utils`, and small utility crates (`bytemuck`, `bytes`, `chrono`,
`either`, `ethnum`, `half`, `hashbrown`, `simdutf8`, `streaming-iterator`,
`strum_macros`). Nothing pulls `polars-core`, `polars-lazy`, or rayon. It is
the arrow2 fork (its own description, line 13), published standalone on
crates.io in lockstep with the facade — the probe used it at 0.55.2. Default
features are **empty** (line 79); `compute` kernels are opt-in with the note
"Disabling this significantly reduces compile time" (line 102), and `io_ipc`
pulls `polars-arrow-format` only when requested (line 92).

The batch type is
[`RecordBatchT<A>`](file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/record_batch.rs)
(lines 12–18): a height, an `ArrowSchemaRef`, and `Vec<A>` where
`RecordBatch = RecordBatchT<ArrayRef>` and
[`ArrayRef = Box<dyn Array>`](file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/array/mod.rs)
(array/mod.rs line 789). `try_new(height, schema, arrays)` (record_batch.rs
lines 35–50) checks field/array counts and equal lengths. Builders for what
our snapshots need are all present and ungated: `MutableBinaryViewArray<str>`
plus `Utf8ViewArray::from_slice`, `MutablePrimitiveArray`, `BooleanArray`,
`MutableListArray` (with `new_with_field`, `mut_values`, `try_push_valid`,
`push_null` via the `MutableArray` trait).

**The DataFrame conversion is zero-copy by construction.**
[`impl From<RecordBatch> for DataFrame`](file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/frame/chunks.rs)
(chunks.rs lines 8–35) moves each array into a single-chunk `Series` via
`Series::_try_from_arrow_unchecked_with_md(name, vec![arr], field.dtype(), …)`
and builds the frame with `DataFrame::new_unchecked`. No buffer is touched;
the `unsafe` blocks assert an invariant we must honor — **the schema dtype
must match each array** — which is exactly what a schema-bound emitter
guarantees at construction time. The probe compiled and ran this path with
`polars 0.55.2` (`default-features = false`) and confirmed string columns land
as dtype `String` with no cast.

One lockstep caveat the probe also proves: type identity requires that our
`polars-arrow` and the facade's resolve to the same crate instance, so pin the
same minor (0.55.x) in both places. The reverse direction,
`DataFrame::iter_chunks(CompatLevel::newest(), parallel)` →
`polars_arrow::RecordBatch` (polars-core frame/mod.rs lines 2118–2122), is
also zero-copy for already-chunked frames.

## 2. Overhead deltas for emitting window/workspace snapshots

Our shape: tens-to-hundreds of rows per snapshot, columns of opaque string
IDs, titles, signed geometry/PID, booleans, occasional list-of-strings; full
replacement per family event. Per dimension:

- **String representation.** The decisive one. Polars stores strings as views
  (`Utf8ViewArray`); `ArrowDataType::Utf8View` imports move chunks untouched
  ([from.rs line 208](file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/series/from.rs)),
  while `Utf8`/`LargeUtf8` imports run
  `cast_chunks(&chunks, &DataType::String, …)` — **a full string copy**
  (from.rs lines 209–212). `CompatLevel::newest()` emits `Utf8View` from
  Polars and `oldest()` emits `LargeUtf8`
  (dtype.rs lines 1287–1292; `CompatLevel` at 1770–1780). arrow-array 59 has
  the native counterpart (`StringViewArray`, `StringViewBuilder`), and both
  FFI layers map the same `"vu"` format string. **Deltas vanish if and only if
  every emitter standardizes on `Utf8View`;** otherwise Polars conversion
  silently pays a cast per string column. For short titles/IDs, views are also
  compact: ≤12-byte strings inline into the 16-byte view, no separate buffer
  touched.
- **Validity bitmaps.** Identical model, no delta. Both use bit-packed
  validity (`polars_arrow::bitmap::Bitmap` versus `NullBuffer`), and the C
  Data Interface carries the validity buffer natively.
- **Chunking.** No delta. A snapshot batch is one chunk per column;
  `DataFrame::from` wraps it as `vec![arr]` (chunks.rs line 21) with no
  rechunking.
- **Schema versus dtype metadata.** Negligible. polars
  `Field { name: PlSmallStr, dtype, is_nullable, metadata: Option<Arc<Metadata>> }`
  with `Metadata = BTreeMap<PlSmallStr, PlSmallStr>`
  ([field.rs lines 28–40](file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/datatypes/field.rs);
  datatypes/mod.rs line 24) versus arrow's `Field { name, data_type,
  nullable, metadata: HashMap }`. The FFI layer converts between them
  losslessly for our types.
- **Polars-specific extension metadata.** Not our problem. The special keys —
  `_PL_ENUM_VALUES`, `_PL_CATEGORICAL`, `maintain_type`
  (field.rs lines 7–21) — only annotate Categorical/Enum dtypes and
  Binary-type-preservation
  ([dtype.rs `to_arrow_field_metadata`](file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/datatypes/dtype.rs)
  lines 1163+). Our schemas use none of those dtypes, so fields carry no
  metadata. Standard `ARROW:extension:name` handling exists on both sides
  (polars datatypes/mod.rs lines 593–602) if we ever want it.
- **Dependency weight.** The honest overhead, and it is compile-time only. As
  resolved in the probe: the `arrow-array` subtree is 40 unique crates; the
  `polars-arrow` subtree is 88 (inflated by facade feature-unification of
  `chrono-tz` and by `io_ipc`'s flatbuffers; a bare default-features
  polars-arrow is materially less). Note both ecosystems pull `chrono`
  non-optionally
  ([polars workspace line 42](file:///home/rektide/archive/pola-rs/polars/Cargo.toml),
  arrow-array Cargo.toml line 57), so chrono is not a delta.

**Bottom line for the user's question:** the runtime impact of polars-arrow
versus arrow-array on emitting schema-bound snapshot batches is effectively
zero once strings are `Utf8View`. "Small enough to just target Polars" is
therefore *true* on performance grounds — the reasons to prefer arrow-array
native are coupling and ecosystem, not speed.

## 3. Conversion paths between the two RecordBatch types

There is no safe direct conversion; the types are unrelated Rust types. Two
realistic paths, both probe-verified end to end:

### C Data Interface bridge — zero-copy, small audited unsafe

Both projects implement the Arrow C Data Interface:

- polars-arrow:
  [`export_array_to_c(array: Box<dyn Array>) -> ArrowArray`](file:///home/rektide/archive/pola-rs/polars/crates/polars-arrow/src/ffi/mod.rs)
  (line 22), `export_field_to_c` (27), `import_field_from_c` (35),
  `import_array_from_c` (43). The import dispatch covers `Utf8View`
  (ffi/array.rs lines 20–50). Ownership rides a release callback plus a
  boxed `PrivateData` holding the array (ffi/array.rs lines 59–95).
- arrow-rs: `to_ffi(&ArrayData)` and
  `from_ffi`/`from_ffi_and_data_type` ([ffi.rs lines 237–289](file:///home/rektide/archive/apache/arrow-rs/arrow-array/src/ffi.rs)),
  importing buffers as custom allocations owned by the foreign release — no
  copy. `data.align_buffers()` is called on import but is a no-op for aligned
  Rust allocations (lines 257–262). **The `ffi` module is behind arrow-array's
  non-default `ffi` feature** (Cargo.toml line 71).

The bridge between them moves the `#[repr(C)]` structs across type systems:

- **polars → apache:** `export_array_to_c(arr)` consumes exactly the
  `Box<dyn Array>` that `RecordBatchT::into_arrays()` yields;
  `FFI_ArrowArray::from_raw` / `FFI_ArrowSchema::from_raw` perform
  `ptr::replace` out of the (cast) polars struct, leaving it empty so no
  double release (arrow-data/src/ffi.rs lines 236–238; arrow-schema
  equivalent at lines 266–269); `Field::try_from(&FFI_ArrowSchema)` rebuilds
  the field; `from_ffi_and_data_type` + `make_array` produce the
  `arrow_array::ArrayRef`.
- **apache → polars:** `to_ffi` on the apache data, `ptr::write` the two C
  structs into freshly `empty()`-initialized polars structs, then
  `import_field_from_c` + `import_array_from_c`; the polars `Drop` fires the
  arrow-rs release exactly once.

**Unsafe budget:** two functions plus compile-time size/alignment assertions
(both projects' structs are field-for-field identical; verified against
generated.rs lines 6–16 and arrow-schema/src/ffi.rs lines 76–98). The
ownership contract is the spec's move semantics — the consumer's drop releases
the producer's private data. The probe asserts **data-buffer pointer
identity** between the polars `Utf8ViewArray`/`Int32Array` and the imported
apache `StringViewArray`/`Int32Array`: genuinely zero-copy, both buffers
shared. Nested `LargeList<Utf8View>` round-trips with values intact. This is
the same mechanism cross-language Arrow interop uses everywhere; the risk is
concentrated, auditable, and version-independent (a C ABI).

### IPC serialization fallback — safe, portable, copies

polars-arrow's `io_ipc` `StreamWriter` serializes a
`polars_arrow::RecordBatch` (its `write` signature takes exactly that type)
into bytes; apache `arrow-ipc StreamReader` reads them back. The probe
round-tripped the full five-column batch with values equal. Cost: **serialize
plus parse — roughly two full buffer copies and message framing per batch**
versus zero shared allocations; also a `flatbuffers` dependency on the
polars-arrow side via `polars-arrow-format`. Two probe footguns worth
recording: polars' `StreamWriter::write` panics unless `start(schema, None)`
was called first (stream.rs line 102 unwraps `ipc_fields`), and 59.x
`StreamReader::try_new` takes a second `projection` argument. For
tens-of-rows snapshots this cost is microseconds and irrelevant next to JSON
decoding; it is the right fallback for *persistence or cross-process* paths,
not the in-process hot path.

## 4. Architecture comparison

| | **A. Polars-native only** | **B. arrow-array core + Polars adapter** (recommended) | **C. dual emitters + schema core** |
|---|---|---|---|
| Crates | `umbriel-arrow` (polars-arrow) + `umbriel-polars` (facade) | `umbriel-arrow` (arrow-array) + `umbriel-arrow-bridge` + `umbriel-polars` | schema core + two emitters + polars crate |
| Emitter runtime cost | baseline (zero-copy to DataFrame via `From`) | identical builders; +per-column C-struct moves into/out of FFI (pointer-identity-proven zero-copy) | identical twice |
| Polars → DataFrame | `From` impl, no unsafe on our side | bridge + the same `From` impl | A-path per schema |
| Apache-arrow consumer | nothing native; bridge needed later | native `arrow_array::RecordBatch`, plus IPC/Parquet/DataFusion directly | native per schema |
| Version coupling | **every Polars release moves the emitter** (polars-arrow minor lockstep) | Polars bumps confined to bridge + polars crates; emitter tracks only arrow-rs | both couplings exist, in different crates |
| Unsafe surface | none today (bridge deferred) | ~100 lines, one audited crate, C ABI stable | none, but duplicated builders |
| Maintenance | one builder stack | one builder stack + one bridge | **two builder stacks in lockstep; schema drift risk** |
| Public clarity | "umbriel-arrow" returns a Polars-internal type (handoff already flagged "name it honestly") | returns the type the ecosystem knows | two emitted types to document |

**Recommendation: B.** It matches the user's preference for more, smaller
crates with each owning one sharp concern: the emitter owns schemas and
builders (safe, no Polars anywhere), the bridge owns all the unsafe in the
workspace, and `umbriel-polars` owns the facade plus the already-governed
`collect_batches` blocking boundary. C's shared-schema benefit does not
survive contact with the duplication cost — the schema definitions can live in
the emitter and be consumed by the adapter without a separate core crate. A is
not wrong; it is the fallback if the team vetoes the unsafe bridge before any
apache consumer exists, and the probe proves migrating A → B later costs
nothing in copied data.

### Concrete crate graph

```text
umbriel-protocol                      [exists today]
        ↑
umbriel-arrow ────→ protocol + arrow-array(+arrow-schema)   [safe, no unsafe,
        ↑            Utf8View-only string columns, family schemas,           ]
|              SnapshotBatch envelope, zero/empty replacement]
|
umbriel-arrow-bridge → umbriel-arrow + polars-arrow + arrow-array/ffi
|              [the two audited unsafe fns + layout asserts, both directions;
|               create when the bridge code lands, per the no-empty-crates rule]
|        ↑
umbriel-polars ──→ umbriel-arrow + umbriel-arrow-bridge + polars 0.55.x
                 [DataFrame/lazy boundary, collect_batches off the receive owner]
```

Rules that make it work: pin `polars`, `polars-arrow` (bridge dep) on the same
minor so the facade's `From` impl sees one crate instance; enable arrow-array's
non-default `ffi` feature only in the bridge crate; keep IPC (de)serialization
as a later optional consumer of `umbriel-arrow` rather than a core dependency.

## Residual risks and unresolved API questions

1. **Schema-dtype invariant on the Polars side.** `DataFrame::from` trusts
   that each schema field's dtype matches its array
   (chunks.rs lines 17–26). The emitter guarantees this by construction; the
   bridge must guarantee it by mapping dtypes itself. Any future "bring your
   own schema" API needs an explicit validity check before this conversion.
2. **Reverse-direction pointer identity asserted only by value.** The
   apache → polars import is zero-copy by construction (foreign-allocation
   buffers), but the probe asserted pointer identity only for
   polars → apache. If we ship the bridge, add the mirrored assertion.
3. **Nested-type coverage.** Only `LargeList<Utf8View>` was exercised.
   Struct columns (not in current snapshot schemas) and dictionary encoding
   would need their own probe cases before being allowed in schemas.
4. **arrow-rs minor drift.** The probe compiled against 59.3.0 while the
   archived checkout is 59.2.0 — harmless (C ABI), but the FFI structs should
   keep their compile-time size/alignment asserts so drift cannot pass
   silently.
5. **`align_buffers` semantics.** Import calls `data.align_buffers()`, a
   documented no-op for aligned Rust buffers (arrow #10034/#10028 tracking).
   The probe's pointer identity confirms it copied nothing here; if we ever
   import from non-Rust producers, re-check alignment.
6. **Unresolved upstream question:** whether `polars_arrow::ffi` will keep
   `import_array_from_c`'s `ArrowDataType`-parameterized signature across the
   facade's 0.56+ releases. It is stable across 0.55.1–0.55.2 (checked), but
   the bridge crate should pin and test per bump since it is the one crate
   that sees both ecosystems' types.

## Connections

- [architecture1](../architecture/architecture1.gpt6a.md#proposed-crate-boundaries)
  owns the crate table this proposes to revise at the `umbriel-arrow` /
  `umbriel-polars` rows; its "explicitly selected Arrow representation" cell
  becomes "arrow-array native" under this recommendation.
- [reuse0's Arrow/Polars section](../assessment/reuse0.gpt6a.md#arrow-and-polars-feasible-with-a-precise-rust-boundary)
  deferred exactly this interop probe; its warnings (blocking
  `collect_batches`, distinct Rust types, "not an end-to-end zero-copy
  bridge" — now proven one) are carried forward unchanged.
- [handoff0](../handoff/handoff0.glm53m.md#known-pitfalls) pinned "no Rust
  Polars 2.0" and `polars = "0.55"`; this adds the same-minor pinning rule
  for `polars-arrow` and records the 59.3.0 arrow-rs observation.
- The probe lives in `.test-agent/arrow-probe/` (gitignored scratch; see its
  README). If the bridge is accepted, its two unsafe functions are the
  starting point for `umbriel-arrow-bridge`, promoted deliberately.
