---
type: Library
title: umbriel-polars
description: Explicit finite lazy queries from complete snapshots to owned native batches.
resource: crates/umbriel-polars/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec7a20df7ffeMkxNbf2TS1zhbi
    title: Implemented finite queries and synthetic, array and subscription-wire examples with local provenance
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Polars-native version, dtype and execution boundaries
  - resource: ../umbriel-arrow/README.md
    title: Invariant-preserving owned input snapshots
---

# umbriel-polars

```rust,ignore
use polars::prelude::col;
use umbriel_polars::SnapshotFrame;

let snapshot = umbriel_arrow::windows(&rows, local_provenance)?;
let output = SnapshotFrame::from(snapshot).lazy()
    .map(|plan| plan.filter(col("active")).select([col("id"), col("title")]))
    .collect()?;
let (source_metadata, native_batches) = output.into_parts();
```

`SnapshotFrame` accepts only the checked emitter's `SnapshotBatch`, never an
arbitrary externally assembled raw batch. It provides DataFrame inspection and
consuming access, then `lazy()` keeps metadata with the finite plan. `map()`
allows native caller-selected expressions; `collect()` explicitly executes them
and returns `QueryResult<P>`, an explicitly **derived** result, not a complete
compositor family replacement. Source metadata still describes the input.
Callers introducing extra query inputs must include those observations in their
own provenance; this library does not discover join lineage.

Execution is blocking and belongs outside socket draining. There is no async
stream, implicit live/incremental engine, append store, timer or compositor
mutation. The initial finite path rechunks into one owned Polars-native batch,
including a schema-bearing empty query result. Rechunking may allocate. The
result checks schema/array agreement before exposing batches. Native strings
use `CompatLevel::newest()` / Utf8View, without FFI or IPC conversion.

Dependencies are `umbriel-arrow` and exactly `polars = "=0.55.2"`, with default
features disabled and only `lazy` explicitly enabled. Polars' transitive feature
requirements are broader than this direct selection. No measured speedup or
universal zero-allocation guarantee is claimed.

## Composition example

Synthetic windows (or pass `workspaces`):

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo run -p umbriel-polars --example snapshot_query -- windows
```

Stdin is NDJSON with **one complete family array per line**, not individual rows.
For example, an empty complete windows replacement:

```sh
printf '[]\n' | cargo run -p umbriel-polars --example snapshot_query -- windows --stdin
```

The example decodes Sonic through protocol models, emits owned columns, filters
and projects lazily, then prints the returned native batches. Epoch/position
are synthetic local coordinates, not server transactions or resume tokens.
Windows select the seat-global `active` window, not workspace-local remembered
`focused` windows. Workspaces select `active` workspaces across output groups.

For actual subscription envelopes, use `--wire` instead of `--stdin`:

```sh
umbriel subscribe windows | cargo run -p umbriel-polars --example snapshot_query -- windows --wire
umbriel subscribe workspaces | cargo run -p umbriel-polars --example snapshot_query -- workspaces --wire
```

Wire mode is a subscription-event consumer, not a command/reply driver. It
validates each envelope, decodes only the selected family's payload using the
protocol event models, and skips unrelated/unknown events. Bare blank initial
frames and uncorrelated `ok` lines are skipped with diagnostics, never treated
as command success. Remote `err` lines, malformed envelopes and malformed
selected payloads terminate with explicit errors. Unrelated payloads receive
JSON/envelope validation, not family-specific model validation.

Provenance retains zero-based physical stdin line positions, including skipped
lines; it is not renumbered by selected event count. This example supplies local
epoch 0 and neither manages connections nor establishes subscription readiness.
The checked mixed fixture contains only synthetic data:

```sh
cargo run -p umbriel-polars --example snapshot_query -- windows --wire < crates/umbriel-polars/examples/data/mixed-wire.ndjson
```

Targeted checks (environment overrides keep local build memory manageable):

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo nextest run -p umbriel-arrow -p umbriel-polars
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy -p umbriel-arrow -p umbriel-polars --all-targets -- -D warnings
cargo fmt -p umbriel-arrow -p umbriel-polars --check
```
