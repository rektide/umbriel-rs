---
type: Reference
title: Reusable typed stream tracking
description: Inventory-owned observation composition and independently reusable stable activity runtime, with explicit freshness and queue-before-timer semantics.
resource: crates/tracking/README.md
status: implemented
generated:
  at: 2026-10-22
  by: model:openai/gpt-6-astra#medium
sources:
  - resource: opencode://ses_e9a6a958affehqIA6mCZZF154C
    title: Implemented and tested reusable tracking, reducer metadata outcomes, example migration and service composition
  - resource: src/lib.rs
    title: Typed observation adapter and Inventory ownership
  - resource: src/activity.rs
    title: Cancellation-safe queue-before-timer activity runtime
  - resource: ../umbriel-state/src/active.rs
    title: Reducer-owned accepted metadata semantics
  - resource: file:///home/rektide/src/zooming-kittens/design/longcat/umbriel-control-stream0.gpt6a.md
    title: Consumer requirements for immediate control versus delayed accepted activity
extensions:
  ticket: umbrs-stream-tracking
---

# Reusable typed tracking

`umbriel-tracking` composes client observations with the existing state owner.
It adds no tracing, journald, Kopacetic or Polars dependency. Raw client transport
retains its existing independently driven, count/byte-bounded observers.

## Immediate inventory consumers

```rust,ignore
use umbriel_tracking::{Tracker, Update};
use umbriel_protocol::envelope::Family;

let mut tracker = Tracker::new(&[Family::Windows, Family::Workspaces], true);
// Subscribe to the same families. Keep the Subscription owner alive.
let update = tracker.observe(origin.elapsed(), observer.recv().await?);
match update {
    Update::Installed { family, before, after, windows } => {
        let capture = tracker.inventory().snapshot();
        // Each family has its own status/stamp; this is not a source-atomic census.
        // `windows`, if requested, is the actual Inventory replacement receipt.
    }
    Update::Epoch { .. } | Update::Gap { .. } | Update::Rejected { .. } => {
        // Reconsider policy; retained rows are not current evidence.
    }
    Update::Ignored => {}
}
// On observer.recv/try_recv error, including lag:
tracker.lost();
```

`Inventory` alone owns native rows. `Tracker::inventory()` is read-only; captures
and optional boxed `WindowChanges` retain shared handles, not another mutable row
store. Receipt equality and `is_continuous()` keep their native semantics.

- A new epoch marks retained families stale; it does not announce typed readiness.
- A source gap or observer loss ends continuity; only a new epoch can restore it.
- A classified malformed family invalidates that family. An unclassifiable
  malformed envelope invalidates all requested families. Unknown/unrequested
  events and non-event frames are ignored.
- A rejected install retains old rows but invalidates the affected family. A later
  complete valid census can restore a decode-invalidated family in the same epoch.
- `Installed.after.is_ready()` means **this family** is current. `before` reveals
  baseline/reacquisition. Unseen families remain unseen; loss is also explicit in
  the returned outcome, even when no rows existed to mark stale.

Longcat should consume these immediate captures, including layout-state captures
when subscribed. It must not wait for activity acceptance to react to focus.

## Five-second activity policy

`activity::Activity::new(Duration)` composes a windows-only tracker and
`StableActiveReducer`. Its cancellation-safe `next(&mut Observer)` returns:

- `Epoch`, `Gap`, `Ready`: immediate source/typed-readiness facts;
- `Accepted(StableActive)`: one active/known-none episode after identity dwell;
- `Metadata(StableActive)`: full changed selected metadata for the same currently
  accepted candidate, without another dwell.

There is no private spawned task or extra delivery queue. The caller owns the
subscription, selects cancellation around `next`, and awaits source shutdown.
`next` checks queued observations before polling a due timer; after 64 consumed
observations it yields and starts another drain turn rather than publishing early.
Observer terminal errors invalidate retained state and return `Err`.

Title/app/workspace/scratchpad changes after acceptance emit metadata. Equal
selected metadata and geometry changes do not. Pre-acceptance churn folds into
acceptance. Different identity, none, gap, invalid census or epoch ends metadata
eligibility immediately. A–B–A requires fresh acceptance even when B was brief.
Times are local monotonic observation offsets, not compositor timestamps or proof
of human attention. Known none is never fabricated from loss.

The client [active-window example](../umbriel-client/examples/active_window.rs)
and [window-change example](../umbriel-client/examples/window_changes.rs) now use
this crate. The [activity application](../activity/README.md) supplies records,
output threading, signal handling and a dedicated user service.

## Verification

`cargo nextest run -p umbriel-tracking -p umbriel-state -p umbriel-client --all-targets`
covers native socket timer races, retained freshness, metadata eligibility, and
existing transport/reconnect/lag contracts. Runtime tests use paused monotonic
time; metadata tests exercise the pure reducer. No complete physical transition
history or automatic observer reattachment is claimed.
