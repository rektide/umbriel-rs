---
type: AssessmentBrief
title: Learn from tmux-interface-control and kubeless for Umbriel
description: Prerequisites, user intent, comparison questions, and expected evidence for a sequential Astra-medium assessment of two local Rust projects.
resource: design/assessment/brief0.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Human commission for a baseline-first Astra-medium assessment, multi-crate library, and Arrow/Polars output possibilities
  - resource: ../architecture/architecture1.gpt6a.md
    title: Current Sonic-first whole-protocol and multi-crate direction
  - resource: ../architecture/architecture0.gpt6a.md
    title: Detailed frame ownership and one illustrative temporal observation
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Prior Umbriel investigation; research precedent rather than implemented Umbriel integration
  - resource: ../protocol/commands0.glm53m.md
    title: Returned wire catalog, schemas, argument categories, and reported live probes
  - resource: ../protocol/review0.gpt6a.md
    title: Parent-reviewed protocol qualifications required for the assessment
---

# Assessment brief

## Commission and readiness

The user requests **one Astra medium assessment, reading
`~/src/tmux-interface-control` first and `~/src/kubeless` second**. Before it is
launched, the parent must have a reasonably good Umbriel protocol definition and
a written direction for the library. Exhaustive action mapping is not required.

Read this brief, the [current architecture](../architecture/architecture1.gpt6a.md),
the [protocol investigation](../protocol/commands0.glm53m.md), and its
[source-review corrections](../protocol/review0.gpt6a.md) before inspecting the
comparison projects. The parent has read the returned investigation and reviewed
critical branches; this prerequisite is now met. Keep protocol facts, design
proposals, and working implementation clearly distinguishable.

## Situation

We are building a high-performance **Rust Umbriel client/protocol library**,
preferably sans-I/O at its core. Sonic is selected. The workspace should be
decomposed into crates with meaningful dependency and responsibility boundaries.
Typed commands, queries, all current event families, runtime integration,
stateful consumers, and useful output paths belong in scope.

The active-window stream with a 750 ms stable-identity dwell is an example, not
the organizing architecture. The prior zooming-kittens work investigates
Umbriel; its existing compositor integration is niri. The TypeScript project
stubs here were removed by the user; this is a Rust project.

Umbriel uses NDJSON over a Unix socket, full-state subscription snapshots, and
one-shot ordinary commands. Persistent subscriptions **do support sequential
command/reply reuse** at the inspected revision, confirmed by source and an
agent-reported live probe. They do not support general pipelining/multiplexing.
The baseline and review explain response correlation, missing keyboard initial
state, empty initial responses, request-size accounting, and dedup limits. Use
those facts to evaluate transfer from tmux or kubeless rather than assuming
their protocols behave alike.

## Questions to answer

### First: tmux-interface-control

Inspect maintained docs, architecture/design tips, implementation, and meaningful
tests. Recover the project's actual current shapes and their provenance:

- Endpoint discovery, transport/process ownership, session lifecycle, codec and
  command boundaries.
- Command correlation and cancellation; ordering and failure/unknown-outcome
  semantics; request builders and typed query results.
- Subscription/notification registration, routing, state accumulation,
  slow-consumer handling, reconnect or teardown.
- Where the design is genuinely reusable versus tmux-specific or still DIY.
- How a simple command consumer and a long-lived multi-observer consumer use it.

Record what transfers directly to Umbriel, what only transfers as a pattern,
and what relies on tmux capabilities Umbriel lacks. Do not equate a persistent
tmux control process with a reusable Umbriel one-shot command connection.

### Then: kubeless

Inspect kubeless after the tmux pass so its abstractions can be evaluated
against both a real preceding protocol library and Umbriel's constraints.

- Recover the current observation, operation, runtime, control, view, catalog,
  and work abstractions from code, not just directory names.
- Determine what is domain-neutral today, what is tied to Kubernetes, and what
  exists only in research/proposals.
- Evaluate subscription ownership, initial snapshot/readiness, continuity/loss,
  keyed materialized state, replay/resync, selection/projection, and fan-out.
- Separate observation from desired-state control and reconciliation. We want
  reusable mechanisms without automatically creating a reconciler or cleanup
  behavior for a read-only client.
- Assess actual crate reuse, a small adapter, extraction/shared infrastructure,
  or copying an idea. Give concrete API/type/trait references and the cost of
  each promising approach.

### Materialization, Arrow, and Polars

Explain concretely what materialization would mean for Umbriel's full snapshots:
current family state versus event history, replacement/removal, empty snapshots,
cross-family joins without transaction IDs, and epoch/observation metadata.

The user would like an emitter producing Arrow record batches, ideally through
Polars 2.0 lazy frames. Investigate an appropriate attachment point and realistic
data flow. Verify the exact Rust API/version/type ecosystem before promising
integration; the version wording is a user goal, not verified dependency
compatibility. A lazy query is not automatically a live incremental table.

Compare direct snapshot-to-columns with materialized rows-to-columns. Consider
batch ownership, selection pushdown, repeated full snapshots, empty replacements,
bounded delivery, and output schemas. Keep Polars/Arrow optional and out of the
wire crate. Scope any library-version research to what is needed to judge the
architecture; the main assessment remains tmux then kubeless.

## Expected return

Write and commit an independent model-suffixed document here under
`design/assessment/`. Include your actual native session ID in its OKF sources,
exact inspected repository revisions and working-tree caveats, sources actually
read, and justified evidence status. Do not edit the comparison repositories.

The return should provide:

1. A compact account of both projects, in the requested order.
2. A compatibility/wins matrix grounded in concrete source paths and symbols.
3. A few recommended reusable mechanisms, with integration seams and costs.
4. A proposed crate/dependency graph for umbriel-rs, identifying where it differs
   from the current architecture and why.
5. Subscription and materialization semantics that survive Umbriel's limitations.
6. A defensible Arrow/Polars path and explicit unresolved API questions.
7. Coherent next implementation slices, without time estimates.

Distinguish accepted user direction (Rust, Sonic, multiple crates, broad protocol
scope) from your proposals. Challenge existing draft architecture where the
evidence supports a better shape. Preserve ownership and lifecycle semantics;
do not force abstract uniformity onto different wire protocols.
