---
type: Index
title: Umbriel Rust design and evidence map
description: Progressive navigation to current architecture, protocol evidence, measured performance, Kubeless asks, and preserved history.
resource: design/index.md
status: current
generated:
  at: 2026-10-14
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Integrated the implementation, source reviews, measurement corrections, and current documentation map
  - resource: architecture/architecture2.gpt6a.md
    title: Implemented ownership and data paths
  - resource: kubeless/asks0.gpt61s.md
    title: Current-source Kubeless reuse and lift assessment
  - resource: performance/bench0.glm53m.md
    title: Corrected measured protocol baseline
---

# Design and evidence map

## Current entry points

- [Usage guide](../README.md): run the native examples and choose a crate.
- [Implemented architecture](architecture/architecture2.gpt6a.md): six-crate
  boundaries, wire custody, independent observations, state and Polars data paths.
- [Kubeless asks](kubeless/asks0.gpt61s.md): actual current reuse, a prioritized
  delivery-cell lift, and the policies best retained in Umbriel.

## Protocol and implementation evidence

- [Command and action catalog](protocol/commands0.glm53m.md), read together with
  [governing source corrections](protocol/review0.gpt6a.md): source-pinned wire
  facts, connection reuse, initial state, limits and schema details.
- [Typed-action starter and handoff](protocol/actions-handoff0.gpt61s.md): why
  the simple/harder starter has its current API and how the remaining families
  were divided for implementation. The full catalog is now implemented.
- [Protocol crate](../crates/umbriel-protocol/README.md) and
  [session crate](../crates/umbriel-session/README.md): current wire APIs and
  runtime-free ownership contracts.
- [Client](../crates/umbriel-client/README.md),
  [state](../crates/umbriel-state/README.md),
  [Arrow emitter](../crates/umbriel-arrow/README.md), and
  [Polars](../crates/umbriel-polars/README.md): implemented composition surfaces.

## Performance

- [Corrected baseline](performance/bench0.glm53m.md): Criterion timings,
  allocation counts, fixture/build identity, and corrected interpretation.
- [Enum allocation addendum](performance/enum-allocations0.gpt61s.md): measured
  before/after reduction while preserving models and wire semantics.
- [JSON/source investigation](performance/json0.gpt6a.md): buffer, borrowing,
  lazy decoding and parser-library facts. Sonic has since been selected.
- [Benchmark fixtures](../crates/umbriel-protocol/benches/fixtures/README.md):
  sanitization and the boundary between decode fixtures and source identity.

## Comparison and preserved inquiry

- [Tmux then Kubeless assessment](assessment/reuse0.gpt6a.md) and its
  [commission](assessment/brief0.gpt6a.md): ownership precedents and original
  reuse candidates. The newer Kubeless asks inspect the evolved 0.1.0 source.
- [Architecture1](architecture/architecture1.gpt6a.md): the selected whole-
  protocol, multi-crate and Polars-native design before the implemented account.
- [Architecture0](architecture/architecture0.gpt6a.md): initial reasoning and
  detailed 750 ms identity-dwell examples, preserved as precedent.
- [Historical columnar comparison](columnar/arrow-polars0.glm53m.md): conversion
  probes and a bridge recommendation that the user overruled. Shared-buffer
  evidence is not a timing benchmark. Product code is Polars-native.
- [Founding handoff](handoff/handoff0.glm53m.md): a historical state snapshot;
  use the current README and architecture for implementation standing.

## External origins

- [zooming-kittens Umbriel investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md):
  the earlier protocol/Longcat inquiry. Its existing compositor implementation
  was niri; the document was research rather than an Umbriel client.
- [Rust JSON Protocol Speed](https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09):
  the shared answer read in full at the start of this effort. Its useful
  performance ideas were checked against source; the current protocol review
  records corrections to connection and event assumptions.
