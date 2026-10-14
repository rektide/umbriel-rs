---
type: Reference
title: Umbriel Tokio Unix client
description: One-shot sockets, bounded subscriptions, and read-only desktop capture with inferred layout shape and opt-in existing Wayland output inspection.
resource: crates/umbriel-client/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#high
sources:
  - resource: opencode://ses_ec5a53b67ffeeyXe8Vd5hIrSva
    title: Add and verify a read-only native window-change and workspace-impact consumer while retaining existing byte/loss/source delivery semantics
  - resource: opencode://ses_ec61d939affe5ZJDMcnGHXS8Xc
    title: Build and verify desktop capture, inferred layout shape, and bounded opt-in Wayland output-tool reuse without changing the compositor
  - resource: opencode://ses_ec773a6beffeBs3acUzwNtvSoZ
    title: Implement and correct Unix discovery, request evidence, and bounded subscription ownership; remove source-obvious warm-path work
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
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/ipc_commands.cpp
    title: Checked available window/workspace/layer/color/tearing fields and the explicit no-keyboard query rejection
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/ipc.cpp
    title: Checked theme and overview payloads available through initial subscription events
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/cli/outputs.cpp
    title: Checked the existing read-only Wayland output-management client, display selection, and full JSON monitor fields
verification:
  method: Scripted Unix peer tests and paused Tokio time; all 55 client tests, two doctests, formatting, and all-targets Clippy; release desktop-dump build and read-only live capture of 29 windows, eight workspaces, and two monitors.
  limits: No live compositor mutation, production deployment, throughput benchmark, state store, or columnar execution.
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
Dropping a one-shot future closes that socket; it cannot roll back a possibly
executed request and does not trigger retry.

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

## Window changes and workspace impact

```sh
cargo run -p umbriel-client --example window_changes
```

The [read-only subscriber](examples/window_changes.rs) retains the native source
owner and installs window censuses through the existing optional state owner.
It filters full typed equal rows from actual replacement receipts, then uses
Kubeless relation traversal/impact to report both former and new workspaces on
moves. Scratchpad and empty references are explicit, not valid workspace IDs.
Reports carry before/after stamps and health, labeling first acquisition or stale
reacquisition separately from uninterrupted observations. Gaps, decode failures
and observer loss are not silent empty maps. Source end/lag terminates the
example; there is no automatic reconnect or complete-transition-history claim.
The impact crate is a dev-dependency only; native socket delivery is unchanged.
See the [state API guide](../umbriel-state/README.md#opt-in-window-change-evidence-and-workspace-impact)
for evidence/memory limits and the scripted peer verification (not a live run).

## Desktop capture tool and example

```sh
cargo build --release -p umbriel-client --bin umbriel-dump-desktop
./target/release/umbriel-dump-desktop > desktop.json
cargo run -p umbriel-client --example desktop_dump -- --socket /path/to/umbriel.sock
```

[`src/desktop_dump/mod.rs`](src/desktop_dump/mod.rs) is shared tool/example
composition, not an additional public library API. It uses all seven one-shot
queries and a short-lived theme/overview subscription, then shuts the source
down. No actions, application relaunch, state store, or Polars dependency.
`--timeout-ms` bounds each operation; default is 10000. Normal release builds
produce `target/release/umbriel-dump-desktop`; other examples require `--examples`
or `cargo run --example ...` and live under `target/release/examples/`.

The version-1 `umbriel-desktop-dump` JSON format stores query payloads under
`queries.<wire-name>` and initial event payloads under `events.<event-name>`.
Each entry has `status: "ok"` and `data`, with monotonic millisecond offsets from
the capture start. Unknown fields survive typed validation. `submap` can be
successful null and inventories successful empty arrays; `keyboard-layouts`
with a `no keyboard` rejection is instead `status: "unavailable"` plus `reason`.
Other failures abort without partial stdout. Serialization finishes before the
JSON is written. `protocol_model_revision` names the library model baseline,
**not** a discovered running-compositor version.

`layout_shape` adds a **derived spatial hierarchy**, not compositor topology.
Each workspace retains its reported mode/output and divides tiled rectangles
along separating X/Y lines into `side_by_side` or `stacked` regions. When both
axes work, X wins: the shape is deterministic but not necessarily the compositor's
internal tree. Leaves refer to source window indices and retain window IDs;
bounds include off-screen windows. Overlapping/non-separable geometry and the
32-level inference limit produce `unresolved` regions containing every remaining
window reference. Floats, scratchpads, invalid tiled sizes, and unassigned or
ambiguous workspace references remain separate. No hidden weights, split ratios,
master roles, or scroll offsets are fabricated. See the
[shape guide](../../README.md#general-layout-shape-without-compositor-changes).

`--outputs-display DISPLAY` optionally adds `outputs.data` by running the existing
`umbriel outputs --json` read-only Wayland client. The exact executable path can
be selected with `--outputs-tool PATH`; no shell is invoked, and `UMBRIEL_SOCKET`
is removed from the child's environment because output inspection is not IPC.
The selected `WAYLAND_DISPLAY` and inherited `XDG_RUNTIME_DIR` are recorded.
`same_compositor_verified: false` leaves source matching with the caller; an IPC
path does not establish Wayland-display identity. The monitor payload preserves
all JSON fields, including mode/position/scale/transform and unknown additions.
Collection uses the configured deadline, an 8 MiB default stdout bound (the IPC
frame-limit option internally), and a 64 KiB stderr bound, draining both streams
concurrently. Failed/timed-out/oversized subprocesses are terminated and reaped;
requested output inspection errors abort before stdout publication. Without this
flag, capture remains native IPC-only and does not execute any external tool.

The dump includes all exposed window and layout facts, with `atomic: false` and
explicit limitations. It is evidence for a future restore planner, **not** a
complete replay recipe. Output geometry/mode/scale/transform are absent from IPC
but available through the optional Wayland command. Internal layout trees/ratios/
column membership/scroll offsets, layer-surface geometry, launch commands and
application documents/session contents remain unavailable. IDs and
PIDs are not durable relaunch identities. Consult the
[root capture guide](../../README.md#build-an-executable-and-capture-the-desktop)
for geometry semantics and privacy cautions; the
[source catalog](../../design/protocol/commands0.glm53m.md#request-catalog) and
[governing review](../../design/protocol/review0.gpt6a.md) explain the wire boundary.

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
    if let Some(message) = events.recv().await?.message()? {
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
`Arc<[u8]>`, not mandatory owned JSON trees. Both `observer_capacity` and
`observer_byte_capacity` apply: logical queued bytes charge inline metadata,
raw frame bytes, and gap diagnostic text. Shared frame bytes are conservatively
charged in full to each observer. Channel/allocator overhead is additionally
bounded by slot count. With capacity **N** and maximum frame bytes **F**, raw
frame storage is also at most **N × F**. One final terminal record is retained
separately even at either budget, never pushed into an already-full queue;
source diagnostics derive from bounded inbound frames or local I/O/framing errors.
Options check byte-bound arithmetic and bound active observer count. Already queued
items drain before the retained terminal error, which remains repeatable. Slow
observers retire with explicit `DeliveryEnd::Lagged`, without blocking other
observers or replies. A full queue on source termination still retains the
source terminal reason separately. No latest-state coalescing is performed.
Caller-retained frames and retired intervals are caller-owned storage.

Publication retires failed observers in the existing bounded vector; observer
iteration order is not a cross-observer delivery barrier. Command expiry retains
queue storage and survivor FIFO order. Health/readiness watchers are notified
only when represented fields change, without copying unchanged historical
terminal text. `RawReply` privately retains its validated reply kind, so success
decoding need not reclassify the envelope before the caller-selected decode.
These are source-level work removals, not measured performance claims.

`Observation::message` returns `Result<Option<Message>, sonic_rs::Error>`:
non-frame observations are `Ok(None)`, and malformed caller-constructed frames
are decode errors rather than panics. Private validated `RawReply::message`
remains infallible. If the driver disappears without a recorded native terminal,
observers drain queued items then retain `DeliveryEnd::DriverLost`; admitted
command/registration response loss returns `Error::DriverLost`, not an unsent
`Closed` result. Such local loss cannot prove non-transmission or safe retry.
The status channel also closes on abrupt driver disappearance; its last retained
status is historical, not a new health or native completion receipt. No implicit
supervision/restart or synthetic native terminal is added.
For one-shot EOF deadlines, `reply_observed: true` also implies `possibly_sent:
true`, even though the session's wire slot already retired at the reply.

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
compositor mutation or performance claim is involved. Binary integration tests
add complete capture/raw-field retention, empty inventories, missing keyboard,
argument/connection failures, remote/decode errors, missing/invalid/lost initial
events, and subscription cleanup. Read-only live release capture also succeeded;
no desktop restoration was attempted. Shape tests cover scrolling rows/columns,
nested tiling, master and unknown modes, overlap ambiguity, touching edges,
empty/duplicate IDs, floating/scratchpad/unmatched windows, extreme coordinates,
and bounded deep inference with no lost window references.
Output-tool tests cover explicit selection, fixed read-only arguments,
space-containing executable/display names, raw-field retention, diagnostic
retention, spawn/exit/decode/shape failures, stdout/stderr limits, and timeout.
