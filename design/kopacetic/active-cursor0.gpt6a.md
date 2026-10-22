---
type: IntegrationDesign
title: Optional Kopacetic live active cursor
description: Source-grounded selection of a source-qualified live cursor and an isolated integration package.
resource: /design/kopacetic/active-cursor0.gpt6a.md
status: implemented
generated:
  by: model:openai/gpt-6-astra
  at: 2026-10-22
sources:
  - resource: opencode://ses_e9a6ff22fffeHaEytKrQvZz84Z
    title: Research and implement the user-commissioned optional Kopacetic integration for Umbriel
  - resource: /crates/umbriel-client/src/delivery.rs
    title: Independent bounded observer, terminal loss and queued-drain contract
  - resource: /crates/umbriel-client/src/driver.rs
    title: Source ownership and opt-in reconnect with new local epochs
  - resource: /crates/umbriel-state/src/active.rs
    title: Existing seat-global active selection and invalid census rules
  - resource: /crates/umbriel-protocol/src/model/inventory.rs
    title: Typed window observations and active versus focused distinction
  - resource: file:///home/rektide/src/kopacetic/crates/kopacetic-core/src/domain/mod.rs
    title: Faceted generic Entity, standalone identity and provenance
  - resource: file:///home/rektide/src/kopacetic/crates/kopacetic-rkyv/README.md
    title: Optional complete-snapshot backend and absence of liveness or watcher semantics
  - resource: file:///home/rektide/src/zooming-kittens/design/longcat/umbriel-control-stream0.gpt6a.md
    title: Coordinating research on stream composition and the separate immediate versus accepted activity contracts
---

# Optional Kopacetic live active cursor

Work anchors: **umbrs-kopacetic** → **umbrs-kopacetic-cursor**. The user explicitly
commissions an optional integration; ordinary Umbriel and ZK runtime paths must
remain independent. The upstream working home is `umbriel-rs`, whose typed native
client already supplies the required complete Windows events. `umbriel-working`
is the C compositor checkout; this integration needs no compositor modification.

## Research findings and choice

Kopacetic core's `Entity<T>` is a reflected value with identity and provenance,
not a subscription service. It has no built-in mutable-object lifecycle. Its
default Kubernetes feature must be disabled for this standalone consumer. Native
Umbriel types use Serde, not Facet; an integration-owned projection avoids adding
Facet derives or dependencies throughout the protocol model.

Two plausible materializations:

1. **One entity per window:** useful for querying all windows and joining workspace
   membership. It needs source/incarnation-qualified IDs, explicit deletion and
   collection completeness, and an ownership policy for absent entities. It would
   substantially overlap existing `umbriel-state` inventories and Kopacetic's
   evolving entity-collection work.
2. **One active-cursor entity per explicitly named source:** a stable object whose
   value is unavailable, known no-active, or the current active window. This
   directly serves a live active-window consumer with a small honest contract.
   Switching A → B replaces the value; A is not another retained entity.

Choose (2). This is immediate observed activation, **not the five-second dwell
tracker**. Metadata for the same active ID must update too. `focused` is remembered
workspace focus and must never substitute for seat-global `active`.

## Placement and data contract

Use an independent nested Cargo workspace at `integration/kopacetic-active/`,
outside the root workspace's `crates/*` membership. Gate its implementation and
example behind `active-cursor`, with an empty default feature set. Normal builds
do not even resolve Kopacetic through the root lockfile. This is an integration
package, not a new required layer of Umbriel's libraries.

The caller supplies a nonempty source name, unique among its publishers. Entity
kind is `UmbrielActiveCursor.v1`; entity name is that source name. A cursor has
local epoch and receive-position evidence plus an explicitly tagged state.
Window projection retains ID, title, app ID, workspace and scratchpad membership,
PID, floating flag and geometry. These IDs are local observation identities;
epochs and positions are not compositor revision tokens or durable identities.

A latest-value Tokio watch channel owns the materialized `Entity<ActiveCursor>`.
Readers get complete replacement snapshots and bounded coalescing notification;
this is not a transition journal. No storage backend or filesystem writes are
needed to have a real Kopacetic entity. Callers may explicitly persist captured
values with a selected backend, but a persisted record is historical evidence,
not a live lease. Kopacetic rkyv's atomic file replacement cannot solve writer
death or stream freshness; adding disk writes would create a misleading liveness
promise and extra I/O in this first slice.

## Loss, reconnection and ownership

- Initial state and every epoch announcement are unavailable, awaiting a census.
- Valid empty/no-active census is known no-active; it is distinct from loss.
- Gap, malformed event, multiple-active census, empty active ID, and terminal
  observer failure clear the selected window. No last-good active value is
  silently retained as current.
- Fresh complete census can recover after a decode gap; reconnect's new epoch
  requires fresh data. Independent observer lag is terminal, as in the client.
- The feed owns publication. Dropping it (including cancelling its owning future)
  publishes a stopped/unavailable state before closing the watch channel.
- The native source owner must remain alive separately. Latest-state readers do
  not keep the subscription source alive or apply backpressure to it.
- Queueing and polling latency remain possible; this is the latest processed
  observation, not a linearizable read of the compositor. An independently cloned
  snapshot is a historical value and cannot mutate itself when its producer dies.

## Verification plan

Test actual typed Entity projection, valid no-active versus unavailable, metadata
refresh, invalid census/decode, epoch/gap/recovery and terminal invalidation.
Script a real Unix subscription and reconnect; test cancellation and slow
latest-value readers. Check the integration with the feature disabled and enabled,
and the original client with no added defaults. Check dependency trees to prove
no Kubernetes dependency in the integration and no Kopacetic in the normal client.

## Connections

- [Native active-window example](/crates/umbriel-client/examples/active_window.rs)
  is the continuity-aware dwell precedent; it is deliberately not changed.
- [State API](/crates/umbriel-state/README.md) already owns full inventories and
  change evidence. Per-window publication can compose there if later commissioned.
- [Kopacetic backend guide](file:///home/rektide/src/kopacetic/README.md)
  explains selectable materialization backends; this slice uses its common Entity
  boundary without selecting persistence, discovery or a registry protocol.

## Implemented checkpoint and coordination

The [integration guide](/integration/kopacetic-active/README.md) owns usage.
Implementation is confined to `integration/kopacetic-active/`; no client/state,
protocol, compositor or ZK source file is changed. This account and its sibling
documents are the only owned research paths.

The feed directly reuses `umbriel_client::Observer` and `Observation`, native
epoch/gap handling and optional reconnect. It does not duplicate the private
`next_stable` timer/drain loop. Its small immediate Windows decoding/selection
adapter is needed because the stable reducer emits once per accepted identity
and does not publish same-ID metadata changes. Reusing that delayed event stream
would violate the cursor contract. A future common typed observation classifier
from **umbrs-stream-tracking** is a useful dependency once available; it should
carry decoded Windows data plus source/epoch/position and explicit invalidity,
without imposing dwell or a Kopacetic dependency.

The tracking/service implementer may own new runtime composition code and the
existing active-window example, client/state and activity docs as appropriate.
Avoid only this integration directory and `design/kopacetic/`. Keep immediate
state and accepted activity as distinct public projections. No service or tracing
work was taken by this integration.

### Verification evidence

On October 22, 2026:

- `cargo check --manifest-path integration/kopacetic-active/Cargo.toml
  --features active-cursor --all-targets`: passed.
- Same manifest, `cargo nextest run --features active-cursor --locked`: **4 tests
  passed**, covering actual Unix subscription/reconnect, cancellation, source
  shutdown, coalescing, metadata and invalid/no-active states.
- Same manifest, `cargo clippy --features active-cursor --all-targets --locked
  -- -D warnings`: passed.
- Same manifest, `cargo check --no-default-features --locked`: passed.
- `cargo tree --edges normal --no-default-features --locked`: only the integration
  package itself; zero normal dependencies.
- Enabled normal dependency tree: core with no Kubernetes/Kubeless, no backend,
  no Polars. Protocol/session/client compile as native dependencies.
- Root client-only check was attempted and **blocked at dependency resolution**:
  root `umbriel-state` pins `kubeless-view-memory = "=0.1.0"`, whereas the sibling
  checkout now provides `0.6.0`. No source compilation failed. This unrelated
  workspace pin was preserved and reported to the coordinating parent.

The independent integration lockfile is committed. Default root feature-disabled
verification cannot be called successful until that native dependency mismatch
is resolved. No live compositor activation trial or persistence backend test is
claimed; both are outside this in-memory, scripted-source slice.
