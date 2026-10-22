---
type: IntegrationGuide
title: Optional Kopacetic active cursor
description: An isolated opt-in live Entity projection of immediate Umbriel activation with explicit loss and shutdown state.
resource: /integration/kopacetic-active/README.md
status: implemented
generated:
  by: model:openai/gpt-6-astra
  at: 2026-10-22
sources:
  - resource: opencode://ses_e9a6ff22fffeHaEytKrQvZz84Z
    title: Research, implementation and scripted-stream verification of the optional active-cursor integration
  - resource: /design/kopacetic/active-cursor0.gpt6a.md
    title: Entity-versus-cursor tradeoffs and ownership boundaries
  - resource: /integration/kopacetic-active/src/cursor.rs
    title: Implemented materialization and lifecycle contracts
  - resource: /integration/kopacetic-active/tests/live.rs
    title: Scripted native subscription, reconnect and cancellation verification
---

# Optional Kopacetic active cursor

One **live in-memory `kopacetic_core::Entity<ActiveCursor>`** per caller-named
Umbriel source. Its identity stays fixed while its value changes with immediate
activation and selected-window metadata. The integration is an independent Cargo
workspace outside `crates/*`, disabled by default. It adds no dependency or runtime
work to normal Umbriel or ZK builds.

Work: **umbrs-kopacetic-cursor** under **umbrs-kopacetic**; Kopacetic-side
qualification: **kop-umbriel-cursor**.

## Run explicitly

Development expects `../../../kopacetic` (normally `~/src/kopacetic`) alongside
the Umbriel checkout. From `umbriel-rs`:

```sh
cargo run --manifest-path integration/kopacetic-active/Cargo.toml \
  --locked --features active-cursor --example active_cursor -- desktop-main
```

The example uses native `UMBRIEL_SOCKET`/Wayland socket discovery, requests only
Windows observations, and opts into five reconnect attempts with a one-second
delay. It prints captured entities and stops on Ctrl-C or terminal observer
failure. It never mutates compositor state. The source name `desktop-main` is
caller-owned identity: choose distinct names for independent desktops/publishers.

## Compose the library

Select this package as an explicit dependency with `features = ["active-cursor"]`.
The implementation uses Kopacetic core with its Kubernetes defaults disabled.

```rust,ignore
use umbriel_kopacetic_active::{CursorFeed, CursorState};

// `source` and `observer` come from Endpoint::subscribe([Family::Windows], ...).
// Keep source alive for the intended subscription lifetime.
let (feed, mut reader) = CursorFeed::channel("desktop-main")?;
let task = tokio::spawn(feed.follow(observer));
loop {
    let entity = reader.snapshot();
    match entity.payload.state {
        CursorState::Active(window) => println!("{}", window.id),
        CursorState::NoActive => println!("known no-active"),
        state => println!("unavailable: {state:?}"),
    }
    if reader.changed().await.is_err() {
        // Read the final Ended/Stopped value even when channel closure wins.
        let final_capture = reader.snapshot();
        break;
    }
}
let terminal = task.await?;
source.shutdown().await;
```

Alternatively, retain the feed yourself and call `observe(&Observation)` and
`end(&DeliveryEnd)` from an existing **ordered, single-source** observer loop.
Every loss must reach the feed. Dropping the feed publishes `Stopped`, including
cancelling its consuming `follow` future. `end` permanently closes its publication
state; create another feed for a new observer interval after terminal lag.

## Semantics

- Entity kind: `UmbrielActiveCursor.v1`; name: explicit source name. Identity does
  not change on focus change or reconnect. Epoch/position evidence lives in the
  payload and comes from the local client, not a compositor revision token.
- States distinguish `AwaitingCensus`, `NoActive`, `Active`, `Gap`, `Invalid`,
  `Ended`, and `Stopped`. Every unavailable state carries **no active window**.
  Epoch, gap and termination clear the last relevant frame position too.
- Malformed frames, malformed Windows payloads, multiple active windows and an
  empty active ID invalidate the cursor. A fresh valid census recovers it.
- Same-ID title, app, membership, PID, floating and geometry updates replace the
  projection. Workspace-local `focused` never substitutes for seat `active`.
- Reader notifications coalesce, intentionally: latest materialized state, not
  every transition. The native observer retains its bounded queue/terminal-lag
  contract. A slow reader cannot backpressure the source or grow a history buffer.
- This is latest **processed** observation, subject to queueing/polling latency.
  Cloned snapshots are immutable historical captures; inspect the reader again
  for current state. The socket owner must be retained separately.
- This is an immediate cursor, not the stable-active five-second activity policy.
  No persistence, registry, discovery, systemd service or journal adapter is
  selected. The reflected Entity is usable by an explicitly chosen Kopacetic
  backend, but a saved value is not proof its producer remains alive.

The default feature-disabled package has **zero normal dependencies**. Its
independent lockfile still resolves optional dependencies for reproducibility;
that is not runtime activation. The root workspace and lockfile are unchanged.

## Verify

```sh
cargo check --manifest-path integration/kopacetic-active/Cargo.toml \
  --no-default-features --locked
cargo nextest run --manifest-path integration/kopacetic-active/Cargo.toml \
  --features active-cursor --locked
cargo clippy --manifest-path integration/kopacetic-active/Cargo.toml \
  --features active-cursor --all-targets --locked -- -D warnings
cargo fmt --manifest-path integration/kopacetic-active/Cargo.toml --check
```

Four tests passed: identity/projection/metadata/no-active; malformed/ambiguous
censuses and loss states; slow-reader coalescing/drop; real scripted Unix
subscription/reconnect/cancellation/source shutdown. No live-desktop run or
service deployment is claimed. Dependency-tree inspection confirms no Kubernetes,
Kubeless, persistence or Polars runtime dependency in this integration.

The attempted root `cargo check -p umbriel-client --no-default-features --locked`
is currently blocked before compilation by an existing `umbriel-state` sibling
pin: `kubeless-view-memory = "=0.1.0"` versus the sibling's `0.6.0`. The original
client compiles as a normal dependency of this isolated integration. This root
workspace resolution issue is independent of the integration and is recorded in
the [research/verification account](/design/kopacetic/active-cursor0.gpt6a.md).

## Related ownership

[Design and tradeoffs](/design/kopacetic/active-cursor0.gpt6a.md) explains why one
cursor was selected over per-window entities. The
[native client guide](/crates/umbriel-client/README.md) owns stream lifecycle.
The planned `umbrs-stream-tracking` composition library and `umbrs-activity`
service can remain independent of Kopacetic. If they expose a reusable typed
Windows classifier, this adapter can consume it; a delayed accepted-activity
stream must not replace the immediate cursor.
