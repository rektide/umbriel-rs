---
type: Handoff
title: Umbriel change evidence and Kubeless integration handoff
description: What landed in each repository, the native window-change consumer, verification evidence, and boundaries for continuing the work.
resource: design/kubeless/handoff0.gpt6a.md
status: current
generated:
  at: 2026-10-14
  by: model:openai/gpt-6-astra#high
sources:
  - resource: opencode://ses_ec6abcf49ffeYEap9Wyro4UT12
    title: Assess the asks against Kubeless spirit, commission three sequential implementations, review results, and clarify the unexpected cross-repository scope
  - resource: opencode://ses_ec5a53b67ffeeyXe8Vd5hIrSva
    title: Umbriel implementation and reported scoped verification
  - resource: asks0.gpt61s.md
    title: Original reuse assessment, preceding the selected publication-first direction
  - resource: ../../crates/umbriel-state/src/inventory.rs
    title: Shared installation path and actual window replacement receipts
  - resource: ../../crates/umbriel-client/examples/window_changes.rs
    title: Native read-only change and relation-impact consumer
  - resource: ../../crates/umbriel-state/README.md
    title: Maintained state, health and receipt contracts
  - resource: file:///home/rektide/src/kube-kubeless/design/toolkit/publication/independent0.gpt61sol.md
    title: Implemented publication policy, generic seed cuts and verification account
verification:
  method: Parent source review and implementation-agent verification reports; checked repository state and documentation links during handoff.
  limits: No test suites rerun for this documentation change; no live compositor, benchmark, or whole-workspace verification claimed.
---

# Umbriel / Kubeless handoff

## Start here: scope and repository ownership

The session began in **kube-kubeless**. The user approved three ideas: checked
replacement construction, independent publication with less-coupled seeded
composition, and use of existing change/impact machinery. The parent interpreted
the third as an Umbriel consumer implementation and dispatched it there. On
return, the user expressed surprise: they had expected work in Kubeless. That
repository boundary should have been made explicit before implementation.

The code below is committed and verified within the stated scope. Do not treat
that as a blanket commission for further Umbriel work. State the destination
repository explicitly when proposing continuation. The present request is a
handoff and a pitch of further ideas, not further feature implementation.

| Repository | Delivered | Commits |
|---|---|---|
| `~/src/kube-kubeless` | Checked `Replacement::try_from_entries`, sharing duplicate validation | `156418e1` |
| `~/src/kube-kubeless` | `publication::lagging`, generic seeded stream/snapshot entrances, docs | `795b155b`, `629424e3` |
| `~/src/umbriel-rs` | Checked replacement adoption, opt-in window receipts, native change/impact example | `422de658`, `a7179117`, `8d83e554` |

At handoff start, both working copies were clean. Umbriel's implementation tip
was `8d83e554`; Kubeless's was `629424e3`. This handoff is a subsequent docs stage.

## What Umbriel now provides

### One owner, two evidence choices

[`Inventory`](../../crates/umbriel-state/src/inventory.rs) retains its existing
family ownership, native input order, source stamps and health rules.

- `install(stamp, event)` uses the existing cheaper key-only replacement path.
- `install_windows_changes(stamp, windows)` uses the **same validation/install
  path**, requesting the existing Kubeless writer's richer `replace_changes`
  receipt. It returns `WindowChanges { before, after, commit }`.
- `before` and `after` are family status. The commit contains its local generation
  and keyed before/after `Arc<Window>` handles. It is actual installation evidence,
  not a second snapshot-diff engine.
- Candidate preparation now uses `Replacement::try_from_entries`. Duplicates
  reject before installation; source order remains owned by Umbriel.

Equal values remain in raw replacement evidence. Consumers choose their equality
policy. The example filters full typed `Window` equality, so title/geometry changes
remain visible even if workspace membership does not change. Rich receipts retain
old and new handles while held; ordinary installation does not retain those receipts.

### Runnable consumer

From the Umbriel checkout:

```sh
cargo run -p umbriel-client --example window_changes
UMBRIEL_SOCKET=/path/to/umbriel.sock cargo run -p umbriel-client --example window_changes
```

[`window_changes.rs`](../../crates/umbriel-client/examples/window_changes.rs)
subscribes to native window censuses, retains the connection owner, installs
through the new API, and reports additions, removals and changed rows.

It uses existing `kubeless-view-impact::{visit_relations, affected_roots}`:

- A move from workspace A to B affects **both A and B**.
- An addition affects its new relation; a removal affects its former relation.
- Empty workspace references become explicit `Scratchpad(name)` or `NoWorkspace`
  relations. Nonempty references do not prove that a workspace currently exists.
- Relation traversal preserves window/side provenance; deduplicated affected roots
  intentionally omit that provenance.

This is a change/impact **reporter**. It does not maintain workspace summaries,
schedule recomputation, or perform compositor actions. Its stdout consumer can
fall behind and trigger the native observer's explicit loss behavior.

## Contracts to preserve

1. Rows, native order and status are captured through the single inventory owner;
   raw memory reader/writer handles do not escape.
2. Invalid input preserves installed rows/order/stamp, while last-attempt and
   stale-health facts still record the unsuccessful attempt.
3. A valid empty census is an installed empty relation, not missing data.
4. `WindowChanges::is_continuous()` distinguishes a current same-epoch comparison
   from baseline/stale reacquisition. Even uninterrupted observed censuses do not
   prove every physical compositor transition was delivered.
5. A source gap requires a newer epoch before installation. Decode/validation
   failure can be followed by a valid restoring census in the same epoch.
6. Transition iteration order is not source array order. Local generations and
   receive positions are not server revisions or resumable cursors.
7. The example ends on source termination or observer loss. It does not silently
   reconnect and relabel reacquisition as continuous history.

## Dependencies and the upstream distinction

`umbriel-state` continues to depend on the narrow `kubeless-view-memory` leaf.
`kubeless-view-impact` is a **client dev-dependency** used by the example. The
sibling paths resolve through `~/src/kubeless`, currently a link to
`~/src/kube-kubeless`; both workspaces use the current 0.1.0 Kubeless packages.
This is local checkout integration, not a published or immutable remote pin.

Kubeless's new publication policy is **not** installed into Umbriel's socket
delivery. Its per-subscriber item bounds and drain-then-`Lagged` stream are a
view-publication capability. Umbriel's existing delivery additionally owns byte
accounting and native source/local-loss terminal semantics.

The [original asks](asks0.gpt61s.md) proposed a generic delivery-cell extraction.
The user instead selected a closer extension of existing publication machinery.
Use the [upstream implementation account](file:///home/rektide/src/kube-kubeless/design/toolkit/publication/independent0.gpt61sol.md)
for what actually landed: existing `async-broadcast` queues, a separate lagging
policy, and generic seed/cut adapters over caller-owned streams and snapshot
closures. Blocking publication remains available.

## Verification and how to resume

The implementation agent reported **66 scoped tests and four doctests passed**,
plus scoped formatting and warning-denying all-targets Clippy. Parent review read
the inventory installation/evidence paths and native example. The scripted Unix
peer covers moves, equal censuses, scratchpads and removal. Inventory coverage
includes rejection preservation, empty replacement, gaps/epochs and release of
ordinary-install handles. These are local/scripted checks, not a live compositor
run or a performance result.

```sh
cargo nextest run -p umbriel-state -p umbriel-client --all-targets
cargo test -p umbriel-state -p umbriel-client --doc
cargo fmt -p umbriel-state -p umbriel-client --check
cargo clippy -p umbriel-state -p umbriel-client --all-targets -- -D warnings
```

For continuation, consult the [state guide](../../crates/umbriel-state/README.md#opt-in-window-change-evidence-and-workspace-impact)
and [usage guide](../../README.md). Useful possible follow-ons are a live read-only
exercise, an application that actually recomputes affected workspace summaries,
or a Kubeless-local composition probe demonstrating that same pipeline with native
generic values. These are opportunities, not selected work. Further expansion of
the window-only receipt entrance should follow a concrete family consumer.
