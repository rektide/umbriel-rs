---
type: Reference
title: Optional Umbriel observed state
description: Complete family inventories over Kubeless memory views and deterministic stable-active identity dwell.
resource: crates/umbriel-state/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_eaf7cba09ffe6roChmdCMdpIpd
    title: Raise the default stable-active dwell from 750 ms to 5 s; pin dwell-dependent traces to explicit thresholds
  - resource: opencode://ses_ec5a53b67ffeeyXe8Vd5hIrSva
    title: Adopt checked replacements and opt-in actual window receipts, verify native change/impact reporting and retained-snapshot continuity limits
  - resource: opencode://ses_ec75b4878ffekQ95N9mUgpOTl7
    title: Implement family replacement and health, coherent joins, stable-active reducer and native timer example; preserve capture membership/order and skip unchanged workspace copies
  - resource: ../../design/assessment/reuse0.gpt6a.md
    title: Family-scoped replacement, duplicate rejection, source evidence, and Kubeless reuse boundary
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Optional state and runtime-free reduction boundaries
  - resource: ../../design/architecture/architecture0.gpt6a.md#stable-active-window
    title: Continuous observed identity dwell and input-before-timer precedence
  - resource: ../../design/protocol/review0.gpt6a.md
    title: Duplicate snapshots and independent family readiness
  - resource: ../../design/protocol/commands0.glm53m.md
    title: Active versus focused, full snapshots, and scratchpad workspace references
  - resource: ../umbriel-client/README.md
    title: Separately driven source owner, epochs, gaps, and terminal observer lag
  - resource: file:///home/rektide/src/kubeless/view/memory/src/lib.rs
    title: Current 0.1.0 memory_view, Replacement, Reader, and Writer APIs
  - resource: file:///home/rektide/src/kubeless/view/impact/src/lib.rs
    title: Public before/after relation traversal and deduplicated impact projection used by the native consumer
  - resource: file:///home/rektide/src/kubeless/design/spirit/open0.gpt6astra.md
    title: Native meanings, non-prescriptive adoption and mechanical-versus-world evidence boundaries
verification:
  method: Deterministic traces, scripted Unix peer with paused Tokio time, scoped nextest and doctests, formatting, and owned-target Clippy with warnings denied.
  limits: No live compositor mutation, performance measurement, registry publication, or reproducibly pinned Kubeless dependency.
---

# Optional observed state

`umbriel-state` depends only on `umbriel-protocol` and the narrow
`kubeless-view-memory` leaf. It owns no Tokio executor, socket, Polars query,
source retry, action, or reconciliation policy. Applications need not use it to
consume raw or typed observations.

## Development dependency

The user approved a **sibling-checkout path dependency** at
`../../../kubeless/view/memory` relative to this crate: a checkout named
`kubeless` beside `umbriel-rs`. The current Kubeless workspace and its leaf
packages are version **0.1.0**. This checkout has no configured remote; this
is not a published-registry or reproducibly pinned Git dependency. Building
this crate or the client examples requires that sibling development checkout.
Nothing is vendored, published, or changed in Kubeless.

The actual current leaf provides `memory_view`, checked
`Replacement::try_from_entries`, atomic `Writer::replace_affected`, and opt-in
`Writer::replace_changes` receipts. Its anchoring/mutation facades are not needed
here. `kubeless-view-cache` readiness/sole-writer lifecycle would duplicate our
source-evidence policy, so it is not included. Inventory has no feature switch:
the whole crate is optional to applications, and the narrow leaf has no runtime
or Kubernetes dependency.

## Inventories and health

`Inventory::install(Stamp, Event)` installs one complete replacement map per
family. Windows and workspaces key by their opaque IDs; singleton families key
by `()`. Empty arrays become known empty relations. `Submap(None)` is a known
one-row nullable value, not an unseen family. The adapter rejects empty keyed
IDs and duplicates **before publishing any rows**; it neither parses ID spelling
nor restricts unusual nonempty text. Raw protocol decoding remains permissive.

Rows, source order, and `Status` are maintained by one serialized owner. Kubeless
writer/read handles never escape. A caller can share the owner under its own lock;
`snapshot()` captures all families and their evidence coherently through a shared
borrow and returns owned, immutable-by-convention values with shared row handles.
No slow reader can backpressure the socket through this interface.

`Status::stamp` describes the installed rows, even when empty or stale.
`last_attempt` distinguishes a rejected typed snapshot from that installed
evidence. `Health::Current` is current-interval readiness; `Unseen` has no installed
evidence, while `Stale` describes retained evidence after an epoch change, gap,
or invalid snapshot. Decode failures can be reported using `invalid_snapshot`.
Valid later family data restores readiness; a source `gap()` requires a newer
`begin_epoch()` before any new installation. Other families do not become ready
merely because windows arrived. Epoch announcements, terminal source failures,
and independent observer lag must be explicitly adapted by the application.

`Stamp` carries a local epoch, receive position, and injected observation time.
Positions increase across attempted installs in one epoch; observation times
cannot decrease there. These coordinates are **not** server revisions or resume
cursors. Reconnect starts a newer epoch with independent family reacquisition;
old rows stay explicitly stale until replaced. No source resume is invented.

```rust
use std::time::Duration;
use umbriel_protocol::{envelope::Family, model::Event};
use umbriel_state::inventory::{Inventory, Stamp};

let mut inventory = Inventory::new(1);
let stamp = Stamp { epoch: 1, position: 1, observed_at: Duration::ZERO };
inventory.install(stamp, Event::Windows(vec![])).unwrap();
let captured = inventory.snapshot();
assert!(captured.windows.entries().is_empty());
assert!(captured.windows.status.is_ready());
assert!(!inventory.status(Family::Workspaces).is_ready());
assert_eq!(captured.join_windows().windows_status.stamp, Some(stamp));
```

`Snapshot::join_windows` carries **both** windows/workspaces status and stamps.
It joins the latest separately observed families, never claiming atomic compositor
state. `WorkspaceMatch` explicitly distinguishes matched, unmatched, scratchpad,
and other empty workspace references; it retains every window. Snapshot
`entries()` iterate in lexical key order, **not compositor order**. `source_order()`
preserves the installed source array order, and joins follow windows source order.
Membership and source order are private with read-only accessors, preserving the
association consumed by the join. Captures cannot be manually fabricated or have
their membership/order edited independently. Callers can clone shared row handles
from `entries()` for their own output; stamps/status remain ordinary public data.

## Opt-in window change evidence and workspace impact

`Inventory::install_windows_changes(Stamp, Windows)` uses the **same** install
path as `install`, but asks the existing writer for its actual `replace_changes`
receipt. `WindowChanges` carries `before`/`after` family status and `commit`
(local generation and keyed before/after shared window handles). No second
inventory, snapshot-diff engine, publication cell, or transport adapter is added.
Ordinary `install` still uses the cheaper key-only replacement path and does not
retain former windows for reporting. Checked candidate construction rejects
duplicates before installing; native array order remains separately owned here.

The receipt includes **equal rows**, as actual replacement touches. Consumers
may use `commit.evidence.retain(|row| row.before != row.after)` for full typed
window equality; relation-only equality would miss title/geometry changes.
An empty replacement removes all prior rows, and empty-to-empty still commits a
new local generation. Receipt transition order is unspecified, not source order.
Retained receipt handles can keep both old and new windows alive until dropped.
Rejected data returns no receipt and leaves rows/order/stamp unchanged, with
existing last-attempt and stale-health semantics.

`is_continuous()` distinguishes comparisons inside one current observed interval
from first acquisition or replacement of stale retained rows after invalid data,
gaps, or epoch changes. Before and after status/stamps remain visible even when
the values are equal. Neither continuous observations nor local generations
prove every compositor transition was observed: complete censuses may coalesce
intermediate states. A gap still requires a newer epoch before installation.

```sh
cargo run -p umbriel-client --example window_changes
# Or select a native socket explicitly:
UMBRIEL_SOCKET=/path/to/umbriel.sock cargo run -p umbriel-client --example window_changes
```

The [read-only native consumer](../umbriel-client/examples/window_changes.rs)
subscribes only to windows, retains the source owner, decodes native frames,
installs through this API, and filters equal full rows before reporting changes.
It uses Kubeless's public `visit_relations` and `affected_roots` to project **both
before and after** handles: moving A→B affects A as well as B, deletion affects
the old workspace, and addition affects the new one. Impact is inspection, not
work admission, a compositor mutation, or proof of workspace existence. Nonempty
references stay workspace references; empty ones become `Scratchpad(name)` or
`NoWorkspace`, never a fabricated valid workspace ID. The impact leaf is only a
client **dev-dependency**, because this example alone needs it. Printed titles
may be private; output can lag, and a slow consumer can hit native observer loss.
The example reports gaps/stale comparisons and terminates on source end/lag;
it does not silently reconnect or claim a complete historical transition log.

## Stable active identity

`StableActiveReducer` is deterministic and clock-injected with monotonic
`Duration` values. The default dwell is **5 s**. `observe(epoch, now, windows)`
selects `active`, never `focused`; only ID, title, app ID, workspace, and scratchpad
metadata survive the frame. Same-ID updates skip unchanged workspace-ID copies
and reuse string storage where possible without resetting the deadline.
`poll(now)` emits once per qualifying episode,
even if no more input arrives. A later A episode after A–B–A may emit A again.

No-active data immediately cancels a pending window and begins a valid None
episode, which dwells for the same interval. Gap, epoch announcement, decode
failure, and observer lag call `invalidate()` instead: loss is never valid None.
An epoch change seen by `observe` also breaks continuity. Multiple active rows,
an empty active ID, deadline overflow, or backwards time return an error and
cancel continuity. Supplied time must be nondecreasing across **all** reducer
calls, including reconnect epochs; equal timestamps are allowed. Backwards time
does not lower the previous time floor. Initial data starts its dwell at local
observation time; no prior compositor duration is inferred.

Process input before polling a simultaneous deadline. This is continuity of
**observed protocol identity**, not proof that no physical transition was
coalesced by the compositor or delayed in socket scheduling.

## Native example and checks

```sh
cargo run -p umbriel-client --example active_window
cargo run -p umbriel-client --example window_changes
cargo nextest run -p umbriel-state -p umbriel-client --all-targets
cargo test -p umbriel-state -p umbriel-client --doc
cargo fmt -p umbriel-state -p umbriel-client --check
cargo clippy -p umbriel-state -p umbriel-client --all-targets -- -D warnings
```

The [native example](../umbriel-client/examples/active_window.rs) uses
`Endpoint::subscribe`, retains the source owner, and selects observer input with
a Tokio timer. `Observer::try_recv` preserves normal delivery accounting and
terminal-drain behavior; `Ok(None)` means momentarily empty, not completed.
Already queued observations (including A–B–A, gaps, and terminal lag) drain before
a due timer. Work turns are bounded; publication is postponed and the executor
yielded when that budget is exhausted. Reading continues during dwell, without
sleeping in the receive path or invoking a shell client. The default example
terminates on source end/lag rather than automatically hiding loss with retries.

Tests use deterministic reducer traces, inventory replacement/rejection/stale
captures and joins, and a scripted Unix peer with paused Tokio time. The peer
checks that a queued B cancels due A and B emits from timer-only progress. No
live compositor mutation or performance claim is involved.

The window-change addition passed **66 scoped client/state tests and four
doctests**, formatting, and all-targets warning-denying Clippy. Its scripted Unix
peer verifies initial additions, A→B impact on both sides, equal censuses,
scratchpad/detached references and empty replacement. Deterministic inventory
tests cover receipt handle identity, removals, duplicate preservation, invalid
data, stamps/order, source gaps/epochs, and ordinary-install handle release.
This is scripted/native adapter evidence, **not a live compositor run**.

## Connections

- [Reuse assessment](../../design/assessment/reuse0.gpt6a.md#materialization-details-that-affect-the-adapter):
  the direct memory adapter implements its cheaper replacement seam, not staged
  reflection or blocking publication.
- [Stable-active design](../../design/architecture/architecture0.gpt6a.md#stable-active-window):
  the reducer and queued-input precedence implement the documented episode traces.
- [Client ownership](../umbriel-client/README.md#independent-delivery-and-source-lifecycle):
  source health and observer loss remain independent of inventory readiness.
- [Kubeless renewed alignment](file:///home/rektide/src/kubeless/design/spirit/open0.gpt6astra.md#broad-in-availability-non-prescriptive-in-adoption):
  the consumer stops at replacement/change/impact without adopting publication or
  a controller; identities, equality, stamps and loss remain native.
