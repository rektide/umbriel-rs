---
type: Project
title: umbriel-rs
description: Native Rust Umbriel protocol libraries, read-only desktop capture with inferred layout shape and optional monitor inspection, observations, and Polars-native batches.
resource: README.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#high
sources:
  - resource: opencode://ses_ec61d939affe5ZJDMcnGHXS8Xc
    title: Build and verify desktop capture, inferred layout shape, and opt-in existing Wayland output inspection without compositor modifications
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Implement and verify the six-crate library, review measured results and reuse boundaries, and rewrite the usage guide
  - resource: design/architecture/architecture1.gpt6a.md
    title: Predecessor design and human-selected direction
  - resource: design/architecture/architecture2.gpt6a.md
    title: Current implemented ownership and data paths
  - resource: design/kubeless/asks0.gpt61s.md
    title: Source-grounded reuse recommendations and small upstream lift proposals
  - resource: design/assessment/brief0.gpt6a.md
    title: Commission for learning from tmux-interface-control followed by kubeless
  - resource: design/assessment/reuse0.gpt6a.md
    title: Completed Astra-medium assessment and reviewed reuse recommendations
  - resource: crates/umbriel-protocol/README.md
    title: Implemented protocol foundation and public codec, envelope, and request interfaces
  - resource: crates/umbriel-session/README.md
    title: Implemented sans-I/O session ownership and deterministic transcripts
  - resource: crates/umbriel-client/README.md
    title: Native discovery, one-shot execution, independent subscriptions and reconnect policy
  - resource: crates/umbriel-state/README.md
    title: Kubeless-backed observed inventories, coherent joins, and pure active-identity dwell
  - resource: crates/umbriel-arrow/README.md
    title: Implemented Polars-native full-snapshot emission and schema invariants
  - resource: crates/umbriel-polars/README.md
    title: Implemented finite lazy-query composition and native output batches
---

# umbriel-rs

Native Rust access to the **Umbriel compositor**: queries, typed actions,
subscriptions, observed state, and **Polars-native batches**. Sonic handles JSON;
the protocol and session cores are sans-I/O, with a Tokio Unix adapter on top.

- All seven query models, six event families, and **121 typed action paths**.
- Continuously driven subscriptions with independent bounded observers and
  cancellation-safe command ownership.
- A read-only **desktop JSON dump** executable and example.
- Geometry-derived **layout shape** and optional existing Wayland monitor inspection,
  with **no compositor patches**.
- A native **750 ms stable-active-window** example—not the project's goal.
- Complete snapshot batches, lazy queries, and derived output with provenance.
  **No C Data Interface bridge.**

The goal is a reusable **whole-protocol Rust library**, not just a delayed focus
watcher. Small tools demonstrate concrete uses of that library; the desktop dump
is also shipped as a normal Cargo binary.

## Run from the checkout

The development workspace expects `../kubeless` beside this repository. The
state crate uses its unpublished `kubeless-view-memory` **0.1.0** leaf through
the user-selected sibling path dependency.

Discovery uses `UMBRIEL_SOCKET`, or `XDG_RUNTIME_DIR` and `WAYLAND_DISPLAY`.
Outside a graphical shell, set `UMBRIEL_SOCKET` to the compositor's socket path.
The examples below use the native Rust client.

### Build an executable and capture the desktop

```sh
cargo build --release
./target/release/umbriel-dump-desktop > desktop.json
```

For a smaller build that avoids the Polars packages:

```sh
cargo build --release -p umbriel-client --bin umbriel-dump-desktop
```

The dump contains **all mapped windows**, including scratchpad windows, with
IDs, application IDs, titles, PIDs, workspace membership, focus/activation,
floating state, and geometry. It also captures workspace names/order, output
assignments, effective layout modes, layers, color/HDR and tearing state
(including fullscreen state), keyboard layouts, submap, theme, and overview.
Unknown JSON fields are retained after validating the known payload models.

Use `--socket PATH` to override discovery or `--timeout-ms 10000` to set each
operation's deadline. As a second runnable example:

```sh
cargo run -p umbriel-client --example desktop_dump -- > desktop.json
```

**A capture, not a complete restore recipe.** The JSON declares `atomic: false`,
records per-payload local observation offsets, and lists missing information.
Separate queries/events can race. IPC does not expose exact layout trees/column
membership/split ratios/scroll offsets or application launch commands/documents.
Monitor configuration is available through the optional Wayland inspection below,
not IPC. Window positions are layout targets;
sizes are committed content geometry. IDs/PIDs are live-session identities,
not durable application identities. This tool neither relaunches nor moves
anything. Dumps can contain private window titles; store/share them accordingly.

Payloads live under `queries.<wire-query-name>.data` and
`events.<event-name>.data`; each has a `status`. A missing physical keyboard is
explicitly `unavailable` with reason `no keyboard`, not a known-empty snapshot.
Other query, decode, transport, or initial-event failures exit nonzero without
publishing partial JSON. Shell redirection itself may still create an empty file.

### General layout shape without compositor changes

The dump also includes `layout_shape`: a geometry-derived hierarchy of
`side_by_side` and `stacked` regions for each workspace, with bounds and leaf
references into `queries.windows.data`. This reveals apparent scrolling columns
and stacked rows, or spatial nesting in dwindle/master layouts, without requiring
any compositor patch. Workspace mode/output/membership and floating flags are
reported facts; the hierarchy is **inferred**, with `exact_layout_tree: false`.

The inference uses separating lines between rectangles, prefers side-by-side
grouping when both axes work, and retains off-screen windows. Overlaps or
non-separable arrangements remain `unresolved`, rather than inventing a tree.
Floating windows, scratchpads, invalid tiled geometry, and unmatched/ambiguous
workspace membership are listed separately. All references use source array
indices, so empty/duplicate window IDs do not lose windows. This provides the
general shape useful for approximate recreation, not hidden split ratios or
the unique original tiling history. It does not capture app contents (tabs,
documents, running terminal processes); those are separate from desktop layout.

### Include monitor topology using existing tooling

For the fullest capture without compositor changes, use the installed `umbriel`
CLI's **read-only** Wayland output-management inspection:

```sh
./target/release/umbriel-dump-desktop \
  --outputs-display "$WAYLAND_DISPLAY" > desktop.json
```

This adds `outputs.data` with monitor names/identity, enabled state, position,
scale, transform, physical size, available/current modes and refresh rates, and
adaptive-sync state. It runs `umbriel outputs --json` directly (no shell), using
the explicit Wayland display and inherited `XDG_RUNTIME_DIR`. Use
`--outputs-tool /path/to/umbriel` for a non-PATH CLI. The native IPC-only dump has
no external-command dependency and does not query Wayland automatically.

**Choose the same compositor for both sources.** `--socket` only selects IPC;
it does not select the Wayland display. The output section records its own source
and `same_compositor_verified: false`, because correspondence cannot be proved
from a socket filename. Output capture has a deadline and bounded stdout/stderr;
when requested, a missing/failing tool, invalid JSON, or timeout aborts the dump
without partial stdout. Unknown output fields are preserved. Monitor identity
can include serial numbers, so privacy cautions apply to these dumps too.

Cargo examples are **not** built by ordinary `cargo build --release`. Use
`cargo build --release -p umbriel-client --examples` for the client examples;
their executables appear under `target/release/examples/`, while the desktop-dump
binary appears directly under `target/release/`.

### Report a window after 750 ms of stable activation

```sh
cargo run -p umbriel-client --example active_window
```

Title and geometry changes do not restart the dwell. Focus changes do. The
example keeps reading during the wait and invalidates continuity on loss.

### Query real snapshots with Polars

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo run -p umbriel-polars --example native_query -- windows --count 1
```

Use `workspaces` for that family, or add `--socket PATH`. The example runs one
blocking query worker at a time, prints each derived native batch, and drops it.
Socket draining stays independent. For a compositor-free synthetic example:

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo run -p umbriel-polars --example snapshot_query -- windows
```

### Explore queries and actions

```sh
cargo run -p umbriel-client --example shell
```

Enter `help` for the REPL vocabulary. It supports all query names, typed
`overview-toggle` and `workspace-switch`, quoted names/output arguments, and a
`raw` action escape hatch. An optional socket path can be passed after `--`.

## Use the libraries

### A typed one-shot query

```rust,no_run
use umbriel_client::{Discovery, Options};
use umbriel_protocol::{model::Windows, request::{Query, Request}};

async fn windows() -> Result<Windows, Box<dyn std::error::Error>> {
    let endpoint = Discovery::from_env().resolve()?;
    Ok(endpoint.query(Request::Query(Query::Windows), &Options::default()).await?)
}
```

### Prepare an action once, reuse it

```rust
use umbriel_protocol::action::{Action, WorkspaceSelector, WorkspaceTarget};

fn action_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let target = WorkspaceTarget::new(WorkspaceSelector::name("2")?)
        .on_output("DP-1")?;
    let action = Action::workspace_switch(target); // literal name "2", not index 2
    let mut bytes = Vec::new();
    action.request().encode(&mut bytes)?;
    Ok(bytes)
}
```

Bare actions are available through `action::no_argument::NoArgument`; extent,
target, layout, and command builders validate their own argument grammar.
Runtime existence and applicability remain the compositor's decision.

### Turn a snapshot into a lazy query

```rust,ignore
use umbriel_polars::{SnapshotFrame, polars::prelude::col};

let snapshot = umbriel_arrow::windows(&rows, local_provenance)?;
let output = SnapshotFrame::from(snapshot)
    .lazy()
    .map(|plan| plan.filter(col("active")).select([col("id"), col("title")]))
    .collect()?;
let (source_metadata, native_batches) = output.into_parts();
```

The native namespace reexports expose the exact selected Polars types. Strings
use Utf8View, and Polars dependencies are pinned together at **0.55.2**. Empty
input retains complete-replacement metadata; filtered or aggregated output is
explicitly **derived**. Collection is finite and blocking, not a live incremental
query or a hard memory limit on arbitrary plans.

## Implemented crates

| Crate | Use it for |
|---|---|
| [`umbriel-protocol`](crates/umbriel-protocol/README.md) | Reusable framing, Sonic envelopes, models, requests and typed actions with your own transport. |
| [`umbriel-session`](crates/umbriel-session/README.md) | Clock-free connection roles, one wire slot, partial writes and reply ownership. |
| [`umbriel-client`](crates/umbriel-client/README.md) | Native Tokio Unix requests and independently driven subscriptions. |
| [`umbriel-state`](crates/umbriel-state/README.md) | Optional inventories, coherent captures/joins, freshness and stable-active reduction. |
| [`umbriel-arrow`](crates/umbriel-arrow/README.md) | Owned, schema-checked windows/workspaces batches without a state store or query engine. |
| [`umbriel-polars`](crates/umbriel-polars/README.md) | Native lazy expressions and provenance-preserving derived batches. |

Action families and small helpers remain modules inside these responsibilities.
Detailed usage and ownership contracts live in the crate guides.

## Behavior worth knowing

- Ordinary command sockets are **one-shot**. Accepted subscriptions can carry
  sequential commands; there is no pipelining or wire request ID.
- Cancelling a waiter does not undo a command or free a possibly-sent reply slot.
  Slow observers report explicit loss rather than blocking other consumers.
- Reconnect is opt-in, creates a new local epoch, and never replays actions.
- Events are complete family snapshots. Families initialize independently;
  a keyboard-only initial response may be blank. Local capture is not an atomic
  compositor-wide snapshot.
- Window `active` is seat-global; `focused` is remembered workspace focus.

## Verify and measure

The assembled implementation passed **152 tests, six doctests, formatting and
warning-denying Clippy**. The environment overrides keep local Polars builds
manageable; formatting names our packages to avoid traversing the sibling repo.
The desktop-dump, inferred-shape, and output-inspection additions passed all
**55 client tests** (including nine binary integration tests and nine shape
tests), client all-targets Clippy, and a read-only live release capture of 29
windows, eight workspaces, and two monitors. This verifies capture, not restoration.

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo nextest run --workspace --lib --tests --examples --locked
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test --workspace --doc --locked
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt -p umbriel-protocol -p umbriel-session -p umbriel-client \
  -p umbriel-state -p umbriel-arrow -p umbriel-polars --check
```

Protocol benchmarks are separate from tests:

```sh
cargo bench -p umbriel-protocol --bench framing
cargo bench -p umbriel-protocol --bench decode
cargo bench -p umbriel-protocol --bench allocations
```

The [corrected baseline](design/performance/bench0.glm53m.md) records fixture,
build and statistical limits. The [measured enum cleanup](design/performance/enum-allocations0.gpt61s.md)
reduced the 29-window owned-model paths from **149 allocations to 120** without
changing semantics; it makes no throughput claim. Library code forbids unsafe;
the allocation benchmark uses a forwarding allocator only for measurement.

## Design and next opportunities

- [Current architecture](design/architecture/architecture2.gpt6a.md) — implemented
  data paths, ownership, bounds and the reasons for the six-crate split.
- [Kubeless asks and possible lifts](design/kubeless/asks0.gpt61s.md) — prioritize
  an independent bounded delivery cell; retain current memory-view reuse and
  native protocol/state/query policies.
- [Protocol reference](design/protocol/commands0.glm53m.md) and
  [source-review corrections](design/protocol/review0.gpt6a.md).
- [Research and history index](design/index.md) — earlier designs, assessment,
  performance evidence and the founding handoff.
