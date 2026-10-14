---
type: Project
title: umbriel-rs
description: A Sonic-first Rust Umbriel protocol library with a sans-I/O core and separate client, state, and columnar concerns.
resource: README.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Establish and review the protocol, session, and Polars-native implementation; integrate assessment, source evidence, and tickets
  - resource: design/architecture/architecture1.gpt6a.md
    title: Current library-wide architecture and human-selected direction
  - resource: design/assessment/brief0.gpt6a.md
    title: Commission for learning from tmux-interface-control followed by kubeless
  - resource: design/assessment/reuse0.gpt6a.md
    title: Completed Astra-medium assessment and reviewed reuse recommendations
  - resource: crates/umbriel-protocol/README.md
    title: Implemented protocol foundation and public codec, envelope, and request interfaces
  - resource: crates/umbriel-session/README.md
    title: Implemented sans-I/O session ownership and deterministic transcripts
  - resource: crates/umbriel-arrow/README.md
    title: Implemented Polars-native full-snapshot emission and schema invariants
  - resource: crates/umbriel-polars/README.md
    title: Implemented finite lazy-query composition and native output batches
---

# umbriel-rs

A high-performance Rust implementation of the **Umbriel compositor's client
protocol**, using **sonic-rs** and a **sans-I/O core**.

The direction is a multi-crate workspace: wire protocol and framing, Unix client
drivers, optional materialized state, and optional Arrow/Polars output. Typed
queries, actions, and subscriptions are the main surface. A stable-active-window
stream is one example consumer.

Four crates are implemented. The design is grounded in Umbriel source and a
completed assessment of existing Rust protocol/control libraries. Performance
has not yet been benchmarked.

## Implemented crates

| Crate | Responsibility |
|---|---|
| [`umbriel-protocol`](crates/umbriel-protocol/README.md) | Reusable NDJSON framing, Sonic envelopes, requests, complete query/event models |
| [`umbriel-session`](crates/umbriel-session/README.md) | Sans-I/O roles, partial writes, one command slot, cancellation-safe reply ownership |
| [`umbriel-arrow`](crates/umbriel-arrow/README.md) | Full windows/workspaces snapshots as owned Polars-native batches with Utf8View strings |
| [`umbriel-polars`](crates/umbriel-polars/README.md) | Finite lazy queries and native result batches retaining source provenance |

The columnar path uses matching **Polars 0.55.2** types throughout. No C Data
Interface bridge is involved. The batch emitter works directly from protocol
rows, without requiring a materialized state store or the query engine.

### Wire and session

[`umbriel-protocol`](crates/umbriel-protocol/README.md) provides:

- Reusable, bounded NDJSON framing with borrowed frame slices.
- Sonic decoding into caller-selected types and lazy wire payloads.
- Order-independent reply/event envelopes, including successful `ok:null`,
  unknown event names, and explicit blank subscription responses.
- Requests for all seven queries, action dispatch, subscriptions, and headless
  output operations, with correct JSON escaping and LF-inclusive request limits.
- Complete owned query and event payload models, opaque IDs, extensible enum
  spellings, and typed event routing that preserves unknown families.

The session crate drives one-shot requests and accepted subscriptions with
sequential commands. Events continue routing while a command is pending;
cancellation retains possibly-sent reply ownership until drainage or closure.
It handles the keyboard-only blank initial response and independent family
readiness without inventing a universal initialization barrier.

### Columnar queries

Complete windows/workspaces snapshots become schema-checked owned columns,
including a schema-bearing empty replacement. Caller-supplied observation
metadata lives outside the rows so it survives empty input. Lazy queries return
explicitly derived results rather than relabeling a filtered result as the
compositor's full family state.

Try the synthetic composition example:

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo run -p umbriel-polars --example snapshot_query -- windows
```

Collection is finite and blocking; production adapters should execute query
work outside socket draining. The [Polars crate guide](crates/umbriel-polars/README.md)
documents input modes, native expression composition, and output ownership.

The example also accepts real subscription envelopes:

```sh
umbriel subscribe windows | cargo run -p umbriel-polars --example snapshot_query -- windows --wire
```

This is a synchronous, backpressured composition example. Its window query uses
seat-global `active`, and its output retains local input positions through
skipped events and empty snapshots.

### Checks

The crates forbid unsafe code in their own implementation. For the full
workspace, the environment overrides below keep local Polars compilation memory
manageable:

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo nextest run --workspace
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Read first

- [Protocol commands and connection semantics](design/protocol/commands0.glm53m.md)
  — source-grounded command, event, and action research. Read with the
  [source-review corrections](design/protocol/review0.gpt6a.md), especially
  request limits, empty subscription responses, and dedup scope.
- [Current architecture](design/architecture/architecture1.gpt6a.md)
  — Sonic-first whole-protocol design, crate boundaries, subscriptions,
  materialized state, and the Arrow/Polars direction.
- [Tmux/Kubeless assessment](design/assessment/reuse0.gpt6a.md)
  — completed in that order; identifies Kubeless view reuse, tmux reply-ownership
  laws, independent bounded delivery, and a concrete Arrow/Polars boundary.
  The [assessment brief](design/assessment/brief0.gpt6a.md) records the commission.

- [Session handoff](design/handoff/handoff0.glm53m.md)
  — state, decisions, pitfalls, and next steps as of the founding session.

## Supporting research

- [JSON and buffer findings](design/performance/json0.gpt6a.md) — source-checked
  allocation/lifetime tradeoffs. Sonic has since been selected; comparative
  parser benchmarking is a future tool, not a development prerequisite.
- [Initial architecture](design/architecture/architecture0.gpt6a.md) — preserved
  predecessor, including detailed 750 ms stable-active-window semantics.
- [Earlier zooming-kittens Umbriel investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md)
  — protocol and Longcat research. zooming-kittens' existing compositor
  implementation is niri; this document is not an existing Umbriel client.
- [Shared ChatGPT discussion](https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09)
  — read in full; motivates selective decoding and reusable buffers.
