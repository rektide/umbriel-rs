---
type: Library
title: umbriel-arrow
description: Complete owned Polars-native windows and workspaces snapshot emission.
resource: crates/umbriel-arrow/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec7a20df7ffeMkxNbf2TS1zhbi
    title: Implemented snapshot schemas, owned buffers, replacement metadata, invariant tests and native ecosystem reexport
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Polars-native decision and schema safety requirements
  - resource: ../umbriel-protocol/src/model/inventory.rs
    title: Complete wire-named protocol models
---

# umbriel-arrow

`windows(&[Window], provenance)` and `workspaces(&[Workspace], provenance)`
produce owned `SnapshotBatch<P>` values. No state store, session, executor or
query engine is required. The Arrow implementation is **polars-arrow**, pinned
to `=0.55.2` with default features disabled, not Apache arrow-array.
`umbriel_arrow::polars_arrow` transparently reexports this exact native ecosystem
for callers that need its array, schema or batch types.

All wire fields are retained: strings use `Utf8View`, numbers use signed `Int32`,
and booleans use `Boolean`. Schema version 1 has no nullable columns. String
construction copies into owned column storage, so rows and receive bytes can be
reused or dropped. No benchmark or universal zero-allocation claim is implied.

`SnapshotMetadata<P>` carries family, schema version, complete replacement and
caller-supplied provenance even for a schema-bearing zero-row snapshot. Use
local connection epoch, receive position and injected observation time as useful;
these are not server transactions, atomic cross-family snapshots or resume cursors.

`SnapshotBatch` exposes only read access and consuming `into_parts`. Its private
construction validates field counts, heights, names, dtypes and nullability.
There is deliberately no arbitrary raw-batch import or mutable-batch accessor.
`validate_batch` is a structural checker, not a DataFrame import capability.

See [umbriel-polars](../umbriel-polars/README.md) for explicit finite lazy queries.
