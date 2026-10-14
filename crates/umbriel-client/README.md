---
type: Reference
title: Umbriel Tokio Unix client
description: On-demand one-shot sockets and separately driven bounded subscriptions.
resource: crates/umbriel-client/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec773a6beffeBs3acUzwNtvSoZ
    title: Implement Unix discovery, one-shot execution, and independently bounded subscription ownership
  - resource: ../umbriel-session/README.md
    title: Single wire slot and transmission uncertainty
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Runtime and protocol boundary
  - resource: ../../design/protocol/review0.gpt6a.md
    title: Initial keyboard blank, sequential commands, and framing boundaries
  - resource: ../../design/assessment/reuse0.gpt6a.md
    title: Independent bounded observers and explicit reconnect gaps
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/cli/ipc_client.cpp
    title: CLI environment precedence and byte-preserving socket name construction
---

# Umbriel client

Tokio Unix sockets, with no server launch, idle pool, state store, or Polars
dependency. `Discovery` accepts injected OS strings; `from_env` is the optional
process-environment boundary. Explicit path wins, followed by nonempty
`UMBRIEL_SOCKET`, then literal `XDG_RUNTIME_DIR + "/umbriel-" + WAYLAND_DISPLAY +
".sock"`, matching the CLI rather than assuming display names are basenames.

One-shot calls validate before connecting, account partial writes through the
session core, read the native reply, and wait for clean EOF within a deadline.
A truncated EOF tail remains a protocol error even after a native reply.
`RawReply::decode` selects a borrowed projection or owned result; remote rejection
and local decode failure are distinct. Connection failures are provably unsent;
terminal/deadline errors expose possible transmission and native-reply evidence.

```rust,no_run
use umbriel_client::{Discovery, Options};
use umbriel_protocol::request::{Query, Request};

async fn query() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = Discovery::from_env().resolve()?;
    let submap: Option<String> = endpoint
        .query(Request::Query(Query::Submap), &Options::default()).await?;
    println!("{submap:?}");
    Ok(())
}
```

## Subscriptions and commands

`Endpoint::subscribe` returns a source owner and an initial observer installed
**before** the task starts reading. A separately spawned Tokio task owns the
socket, framer, and sole session ledger. Waiting, cancellation, or a stalled
observer never takes over socket driving. `observe` registers additional bounded
intervals, starting when the owner processes registration; it does not seed or
replay earlier observations. Observer drop does not unsubscribe on the server.

The family set is fixed. Matching initial events or the special keyboard-only
blank establish acceptance. Readiness is independently available through
`Status::family_status`; it is envelope evidence, not full typed payload
validation or an atomic snapshot. Keyboard may remain unseen. Unknown/unselected
events and duplicates remain available in raw observations, without marking
undeclared families ready or completing commands.

```rust,no_run
use std::time::Duration;
use umbriel_client::{Endpoint, Observation, SubscriptionOptions};
use umbriel_protocol::{envelope::Family, request::{Query, Request}};

async fn observe(endpoint: Endpoint) -> Result<(), Box<dyn std::error::Error>> {
    let (source, mut events) = endpoint.subscribe(
        &[Family::Windows, Family::KeyboardLayout],
        SubscriptionOptions::default(),
    ).await?;
    while let Observation::Epoch { .. } = events.recv().await? {}
    // A matching initial frame established acceptance; there is no wait-for-all.
    let epoch = source.status().epoch;
    let reply = source.request(epoch, Request::Query(Query::Submap),
        Duration::from_secs(5)).await?;
    let submap: Option<String> = reply.decode()?;
    println!("{submap:?}");
    // Raw events permit selected borrowed projections in the caller's frame scope.
    if let Some(message) = events.recv().await?.message() {
        println!("{message:?}");
    }
    source.shutdown().await;
    Ok(())
}
```

Command ingress is bounded across the active slot, owner queue, and channel with
shared admission permits. `PreparedCommand` validates and encodes before enqueue;
the LF-inclusive 65536-byte protocol request limit applies. No raw-byte bypass
or second reply ledger exists. Commands execute sequentially only after acceptance.
Deadline includes queued time. Dropped queued waiters are removed without sending.
For a cancelled possibly-sent command, the core retains ownership and the owner
finishes writes and drains its late reply before admitting another command. If
the cancellation-drain deadline expires, the connection closes. Active command
deadlines also close the connection: no unbounded drain obligation is hidden.
Cancellation is not execution reversal. Native `ok` is not rendering convergence.

The I/O owner uses cancel-safe readiness waits and synchronous `try_read` /
`try_write`, resolving every write attempt through the core before routing input.
Each turn processes at most 32 frames and one readiness I/O operation, with
executor yields and independently selectable control/command/deadline work.
Inbound storage is limited to `frame_limit + 1`, and reads shrink to available
budget. Complete frames drain before EOF; unterminated tails fail.

## Independent delivery and source lifecycle

Each observer owns a bounded every-observation queue. Raw frames use shared
`Arc<[u8]>`, not mandatory owned JSON trees. With capacity **N** and maximum
frame bytes **F**, retained raw-frame bytes are at most **N × F** per observer;
metadata/epoch/gap items are also counted against N. Terminal records carry a
bounded frame-derived error description, separately from queue slots. Options
check byte-bound arithmetic and bound active observer count. Already queued
items drain before the retained terminal error, which remains repeatable. Slow
observers retire with explicit `DeliveryEnd::Lagged`, without blocking other
observers or replies. A full queue on source termination still retains the
source terminal reason separately. No latest-state coalescing is performed.
Caller-retained frames and retired intervals are caller-owned storage.

Dropping the last `Subscription` clone initiates source close; observers and
status watchers do not keep it alive. `shutdown` closes all clones' source and
waits for socket/task cleanup. Historical family evidence is retained with
`Disconnected`/`Closed` health, never advertised as current healthy state.

## Reconnect is a gap, not continuity

`Reconnect::Never` is the default. Opt-in `Retry { delay, max_attempts }` retries
only EOF/transport loss, with at most that many fresh connection attempts per
loss. Protocol failure, subscription rejection, poisoned request buffers,
explicit close, and deadline-driven close are not retried. Every successful
fresh connection re-subscribes and increments the checked local epoch; family
readiness resets to unseen. Observer intervals receive `Gap` then `Epoch`, or
explicit terminal loss if they cannot retain those transitions. The latest
status watch is intentionally not an every-transition log.

Commands carry a caller-selected epoch. Outstanding commands receive their old
terminal uncertainty; queued commands are rejected on disconnect. Commands
arriving after reconnection with an old epoch get `StaleEpoch`. **No command or
action is replayed automatically.** There is no server resume cursor.

## Checks

```sh
cargo nextest run -p umbriel-client -p umbriel-session
cargo test -p umbriel-client -p umbriel-session --doc
cargo fmt -p umbriel-client -p umbriel-session --check
cargo clippy -p umbriel-client -p umbriel-session --all-targets -- -D warnings
```

Scripted Unix peers and paused Tokio time cover framing, real exchanges,
independent delivery, cancellation, lifecycle, and reconnect epochs. No live
compositor mutation or performance claim is involved.
