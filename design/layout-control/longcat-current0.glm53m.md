---
type: Research
title: Longcat current semantics and the Umbriel adapter boundary
description: Source-grounded account of the implemented zooming-kittens Longcat semantics, its exact native facts contract, and which gaps current Umbriel 5d657581 observation changes still leave for planning and writes.
resource: design/layout-control/longcat-current0.glm53m.md
status: draft
generated: { at: 2026-10-22, by: 'model:zai-coding-plan/glm-5.3#max' }
revisions:
  - at: 2026-10-22
    by: model:zai-coding-plan/glm-5.3#max
    note: revision 1 corrects the axis account after parent review — workspace axis vs strip direction naming, secondary-stacking parallelism, strip/stacking labels in the corroboration option, and the conditionality of dry-run episodes under the strict Reported bridge; original preserved in change c1911970
sources:
  - resource: opencode://ses_e9b37f9f4ffew5DLMbSfKMx1Fv
    title: Longcat implementation research against refreshed upstream Umbriel, bridge contract extraction and adapter gap assessment
  - resource: longcat-needs0.gpt6a.md
    title: Earlier Umbriel-side requirements whose observation-absence claims this account refines
  - resource: upstream-refresh0.gpt6a.md
    title: Parent refresh of native API delta at 5d657581, cross-referenced rather than duplicated here
  - resource: file:///home/rektide/src/zooming-kittens/layout/longcat/src/lib.rs
    title: Pure episode state machine read in full; baseline, phases, evidence and invalidation rules
  - resource: file:///home/rektide/src/zooming-kittens/layout/README.md
    title: Documented solver and episode contracts including restoration representation limits
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md
    title: Runner contract; history-gated admission, retained budgets and shutdown restoration
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/runner/bridge.rs
    title: Exact native facts the observation bridge requires from any backend
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/runner/control.rs
    title: Episode admission, capability guards and per-command fresh-basis requests
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/commands/longcat.rs
    title: CLI-level Umbriel write rejection and configuration composition
  - resource: file:///home/rektide/src/zooming-kittens/desktop/niri/src/source.rs
    title: By-ID niri height actuator with basis revalidation and transmission outcomes
  - resource: file:///home/rektide/src/zooming-kittens/desktop/niri/src/reducer.rs
    title: Reported scrolling groups from pos_in_scrolling_layout with unambiguity discipline
  - resource: file:///home/rektide/src/zooming-kittens/desktop/umbriel/src/reducer.rs
    title: Current read-only Umbriel adapter leaving groups unassigned
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/model/inventory.rs
    title: Rust window and workspace models predating native column, row, tab and state fields
  - resource: file:///home/rektide/src/umbriel-rs/crates/umbriel-protocol/src/envelope.rs
    title: Family enum still covering the pre-outputs/layers/screencast six
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/src/server/ipc_commands.cpp
    title: Windows, workspaces and outputs serialization at 5d657581 including column, row, tabs and usable_area
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/src/workspace/workspace.cpp
    title: Focused-only extent setters with maximize clearing and ensureFocusedVisible at 5d657581
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/src/layout/scrolling.cpp
    title: Tab-unit fraction algebra, edge gaps, weight floor and positional column and row lookup
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/docs/user/ipc.md
    title: Current documented window, workspace, output payload and event family semantics
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/docs/user/actions.md
    title: Trailing-window-id argument precedent on window-move-to and window-move-by
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/docs/user/workspaces.md
    title: Workspace axis setting, values and scrolling arrangement consequences used for the revision 1 correction
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/docs/user/outputs.md
    title: Output-level workspace_axis configuration key and default
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/src/config/resolve.cpp
    title: Strip direction resolved from the output workspace axis, not a user scrolling key
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel-upstream/src/config/config.h
    title: Resolved scrolling direction default and axis-agnostic state preservation comment
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Earlier source-grounded restoration algebra and focus-race analysis checked against current sources
verification:
  method: Read the implemented Longcat core, bridge, control, runner and CLI sources in full or in relevant part; read the refreshed upstream compositor sources and docs at 5d657581; compared against the current Rust protocol models and zk adapters. Revision 1 re-verified the axis chain against workspaces.md, outputs.md, config.h and resolve.cpp after parent review.
  limits: Source research only. No compositor was running or queried, no builds or tests were executed, and the cited zooming-kittens test counts are its documents' reported 2026-10-15 evidence, not re-verified here. No implementation is authorized by this document.
---

# Longcat current semantics and the Umbriel adapter boundary

The [native API refresh](upstream-refresh0.gpt6a.md) established what Umbriel
5d657581 exposes. This document is the consumer-side counterpart: what
zooming-kittens' implemented Longcat actually requires of a backend, what its
pure semantics are, and precisely which adapter gaps the new native facts close,
narrow, or leave open. It refines the [earlier requirements](longcat-needs0.gpt6a.md)
whose blanket absence-of-membership claims the refresh superseded, without
replacing its write and restoration analysis.

## Evidence coordinates

- zooming-kittens checkout at change `mlzwnoxo` (commit `3c2f12de`, above
  `80a2cfcf` "Link consumer findings to Umbriel-side layout-control
  requirements"); read-only, not modified by this session.
- Umbriel upstream sources at
  `~/archive/noctalia-dev/umbriel-upstream`, detached at
  `5d657581e27e1a3fe604693779ca1405610f4f21`. The preserved local compositor
  history at `~/archive/noctalia-dev/umbriel` (`7666af0a`) is **not** current
  upstream and its payload is the older shape the Rust models encode.
- umbriel-rs working tree above `94461dae`.

## What Longcat is now

The [architecture](file:///home/rektide/src/zooming-kittens/ARCHITECTURE.md)
separates three states: observed desktop captures, Longcat episodes, and the
Taffy solver tree. Only the first talks to a compositor.

### Episode core (zk-longcat)

`zk_longcat::Episode` (`layout/longcat/src/lib.rs`) is transport-free and
clock-injected. Its load-bearing semantics, all test-covered per the
[layout contracts](file:///home/rektide/src/zooming-kittens/layout/README.md):

- **Baseline immutability.** `Episode::new` fixes the first observation and a
  conserving integer `Allocation` computed from observed fractional extents.
  Every later emphasis is computed **from the baseline**, never from the
  previous expansion, so focus moves inside the group recompute rather than
  compound (`focus_inside_group_uses_baseline_not_the_previous_expansion`).
- **Identity and context invalidation.** Any change to source/epoch, workspace,
  topology token, axis, member order or membership, available span, gaps or
  member bounds invalidates (`GroupIdentityChanged`, `LayoutContextChanged`).
  A topology token is adapter-owned; ordered member identity is checked
  independently of it. None of these inherits restore authority.
- **Focus semantics.** `Focus::Unknown` suspends ordinary decisions (it is not
  blur); explicit `Focus::None` and known-outside focus request restoration —
  but only after a previous effect completed and observed geometry still
  matches it.
- **Execution evidence.** Per-command outcomes (`Unsent`, `Handled`,
  `Rejected`, `PossiblySent`) are retained through supersession, conflicts and
  loss; `Handled` is native acknowledgment, never convergence proof. A native
  sequence is not a transaction; an unsent tail stays unsent. `PossiblySent`
  is never blindly replayed.
- **Convergence.** Matching later geometry within tolerance yields
  `Convergence::Compatible`, not causal attribution. Deadlines are caller-clock
  milliseconds; command handling alone does not clear them, and the runner must
  arm `next_reconsideration()` even after the final native event.
- **Restoration representation.** The baseline allocation is a *conserving
  integer* representation of observed pixel allocation. It explicitly does not
  recover hidden native layout weights, and integer restoration is not
  pixel-exact recovery of fractional inputs.

### The bridge contract: native facts any backend must supply

`runner/bridge.rs::observation` is the precise admission contract. It rejects
unless:

1. the capture is ready (current windows **and** workspaces families);
2. the group's `membership` is `Evidence::Reported` — never inferred geometry;
3. the group's secondary axis is **reported** vertical;
4. the group has 2..=128 members, each tiled, on that workspace and group, in
   the same source scope;
5. every member has committed content geometry (finite height ≥ 0, width > 0) —
   observed extents, never the adapter's last request;
6. the content budget is captured **once** as the sum of baseline committed
   content heights (with `Gaps::AlreadyExcluded`), then retained while later
   observations carry actual extents, including partial-write sums;
7. focus derives from seat-global activation, with an explicit none case.

Independent context checks (runner `Context`): workspace output, per-member
content width, tile/content vertical decoration within two pixels (skipped when
the backend reports no tile size), and the group span only when its geometry
evidence is reported. Inferred tile bounding spans are deliberately not budget
inputs.

### Runner discipline and capability gates

The [runner](file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md)
enforces: bounded history catch-up (at most eight updates per turn, every
continuity marker checked, a healthy latest snapshot never jumps policy across
an unprocessed discontinuity); globally one command sequence with one owned
future; full `Plan::validate` (geometry included) before the first command and
`validate_identity` between commands; a fresh capture version as the native
basis immediately before each send; explicit retention bounds (32 episodes, 128
members, 64 effects) that stop rather than prune; latched conditional
`request_restore` at SIGINT/limit with a bounded shutdown deadline.

Two gates refuse Umbriel writes today. The CLI
(`app/zk/src/commands/longcat.rs:91-96`) rejects non-dry-run Umbriel runs
before opening a socket — native extent actions target the focused window, not
a chosen ID, and exact groups/axis/restorable layout state are absent. The
control layer (`runner/control.rs:171-176`) independently errors if a plan is
selected without `capabilities.height_by_id`. Umbriel observation still emits an
explicit unsupported planning result rather than fabricating groups
(`control.rs:113`).

### Restoration behavior in practice

Restoration triggers: known focus outside the group (including explicit none),
and shutdown via `request_restore`. Preconditions: unchanged
identity/context **and** observed compatibility with the possibly-sent
requested effect. A wholly unsent replacement attempt does not erase a prior
settled effect's restore duty. At the shutdown bound an outstanding command is
conservatively retained as possibly sent, its tail stays unsent, and authority
is revoked rather than writing stale baseline pixels. Source loss, rejection,
deadline expiry or settled unexpected geometry conflicts the episode without
automatic replay, learning or restoration.

## The niri reference path

Everything above runs today on niri because two facts exist there:

- **Reported scrolling groups**: `desktop/niri/src/reducer.rs:386-427` builds
  `GroupView`s from `pos_in_scrolling_layout` (column, row), ordering members
  by row and marking `Evidence::Reported` only when rows are unambiguous and
  every tiled window on the workspace is located. Note the niri group key is
  itself a positional column index (`NativeId::Number(col)`) — Longcat already
  tolerates positional topology tokens because episodes invalidate on any
  membership/order change rather than trusting the token as an incarnation.
- **By-ID pixel actuation**: `desktop/niri/src/source.rs:194-292` sends
  `SetWindowHeight { id, SetFixed(px) }` — explicit numeric target, one command
  at a time, basis revalidated against the current capture before and after
  connecting, with `Unsent`/`Rejected`/`Handled`/`PossiblySent` outcomes.

These two are the load-bearing precedents for judging Umbriel: the episode core
does not care how a backend reports groups or resizes rows, only that it does
so identifiably and without focus side effects.

## What current Umbriel changes for this consumer

Against the bridge contract, at 5d657581:

| Bridge requirement | Current native fact | Assessment |
|---|---|---|
| Reported membership and order | `windows` reports zero-based `column`/`row` for scrolling workspaces (`-1` otherwise) | **Closable now for membership alone**: group by column within workspace, order by row, with the same unambiguous-and-fully-located discipline niri uses. Indices are positional — acceptable for the same reason niri's are. Episodes remain gated by the axis row below |
| Reported vertical secondary axis | Not exposed. The axis chain is config-owned: per-output `workspace_axis` (`"vertical"` default / `"horizontal"`, `docs/user/workspaces.md#workspace-axis`) resolves to the strip `direction` (`src/config/resolve.cpp:499-501`) — strip perpendicular to the workspace axis, in-column stacking parallel to it. Longcat's vertical stacking therefore means `workspace_axis = "vertical"` (horizontal strip, columns left to right) | **Still open — the one remaining observation blocker**; the strict `Reported` bridge stays closed until a native field, an externally supplied axis, or a deliberate evidence-policy change lands (see below) |
| Committed content extents | `w`/`h` are `committedContentBox()`; `x`/`y` are now content-box positions from the **previewed pending arrangement** (`previewArrangedLayout` when `arrangePending`), not a stale animated target | Satisfied; positions additionally corroborate order/gaps without being an identity |
| Workspace output | `windows` entry `output` plus the workspaces entry | Satisfied directly (previously only inferrable) |
| Decoration/span context | No tile size or group bounds | Skipped by design (`None` compatible); acceptable, weaker context |
| Focus | `active` seat-global, `focused` workspace-local | Already implemented in the adapter |
| Workarea context (optional) | New `outputs` family/query with `usable_area`, `logical_size`, scale | **New capability**: output/workarea changes become independently observable, strengthening context invalidation the runner currently cannot prove |
| `capabilities.height_by_id` | Extent actions remain `window-set-secondary-extent:<fraction>` on the focused area; `Workspace::setFocusedHeight` (`workspace.cpp:1927-1941`) uses `m_focusedView`, clears maximized state and calls `ensureFocusedVisible` | **Unchanged blocker** for all writes |
| Sizing state / revision / group write | No unit weights, edge gaps, scroll offset, sizing revision, or conditional group allocation | **Unchanged blocker** for exact restoration and CAS-style admission |

Refinements against the earlier accounts:

- The [first investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md)
  argued id+fraction needs a designed grammar extension because actions take at
  most one argument. 5d657581 establishes an in-tree precedent:
  `window-move-to:<anchor>/<x>/<y>[/<window-id>]` and
  `window-move-by:<dx>/<dy>[/<window-id>]` are multi-token with a trailing
  optional window id (`docs/user/actions.md:32-33`). A
  `window-set-secondary-extent:<fraction>[/<window-id>]` extension now follows
  an existing pattern — though for floats only today, and a *pixel*-target
  variant would map more directly onto Longcat's integer plans.
- `ScrollingLayout::setHeightFraction` (`scrolling.cpp:1365-1393`) is now
  **tab-unit-aware**: a tab sizes its whole unit; a solo unit redistributes the
  remainder to top/bottom edge gaps (clamp `[0.1,1.0]`); multi-unit columns set
  `target·others/(1−target)` clamped `[0.1,0.95]` with the `0.05` weight floor.
  The denominator-interaction argument against sequential-setter restoration
  stands unchanged.

### Tabs

Tab groups are new facts Longcat has no policy for: members of a unit are
consecutive rows starting at the group's first row, hidden tabs report
`tab_hidden`, and sizing acts on the unit, not the window. Episode members are
independently-sized rows trading extent with siblings — wrong model for tabs.
Hidden tabs' committed geometry is also not a sound baseline input. The
conservative first adaptation is to **exclude tabbed columns from episodes**
(marking them unsupported, like non-scrolling modes), and only later decide
whether to collapse a unit to its shown tab as a single member.

### Axis

The exact chain at 5d657581: the user-facing setting is the **output's
workspace axis** (`workspace_axis`, `[output.<name>]`, `"vertical"` default /
`"horizontal"`; `docs/user/workspaces.md:72-87`, `docs/user/outputs.md:81`).
`ResolvedLayoutConfig.scrolling.direction` is not a documented user key — it is
**computed** from the output's axis at resolve time
(`src/config/resolve.cpp:499-501`; `src/config/config.h:159-160`), and it names
the **strip**: `ScrollingDirection::Vertical` means lanes running top to
bottom, so `ScrollingLayout::vertical()` means a vertical strip, not vertical
stacking. The strip runs perpendicular to the workspace axis and the
in-column/in-lane stacking (the secondary extent Longcat resizes) runs parallel
to it. Longcat's required vertical stacking is therefore the **default**
arrangement: `workspace_axis = "vertical"` → horizontal strip, columns left to
right, windows stacking top to bottom. Nothing in `src/server/ipc_commands.cpp`
exposes any of this.

Because the bridge demands a *reported* axis, dry-run planning stays closed
under current native facts. Three ways out, in rising consumer-side compromise:

1. **Upstream field**: expose the output's `workspace_axis` (or equivalently
   the strip direction / secondary axis) in the `outputs` payload — the axis is
   an output property, so the outputs family is its natural home. One field
   removes the ambiguity authoritatively and pairs naturally with any
   extent-write proposal. The cheap, correct ask.
2. **Externally supplied axis**: the adapter accepts an operator-declared
   per-output workspace axis as an explicit configuration input — a policy
   fact the consumer owns, never mistaken for native evidence. Admission then
   still rests on native column/row membership plus geometry.
3. **Geometry corroboration, as an evidence-policy change**: with reported
   column/row plus content positions, members of a column sharing `x`
   corroborate vertical **stacking** (horizontal strip — the Longcat case);
   sharing `y` corroborates horizontal stacking (vertical strip — reject).
   Sharing is evidence of this snapshot's **arrangement**, not proof of the
   configured axis. A policy accepting it must be recorded as a distinct
   evidence level in the desktop model (never conflated with
   `Evidence::Reported`), and must **reject rather than assume** the uncertain
   cases: solo columns have no sibling sharing to check, and columns with
   missing or inconsistent positions cannot corroborate anything. The bridge
   currently refuses inferred facts outright; using corroboration is a
   deliberate weakening of that rule, not an automatic consequence of the new
   fields.

### Old servers and absent fields

The Rust `Window`/`Workspace` models (`model/inventory.rs`) predate all new
fields, and `Family` (`envelope.rs`) covers the pre-`outputs`/`layers`/
`screencast` six. Two modeling requirements follow: new fields must be
optional or defaulted so old-server payloads decode, and a **missing**
`column` field must stay distinguishable from `-1` (not in a scrolling
column) — absence means old server / unknown capability, `-1` means current
server asserting non-membership. Capabilities should be derived from observed
payload shape, not the library version.

## What is now possible; next coherent slices

1. **Observation upgrade (Rust-only, no upstream work) — conditional for
   episodes.** Extend `umbriel-protocol` models with the new windows/workspaces
   fields and outputs family; extend `zk-umbriel`'s reducer from `group: None`
   to reported scrolling groups built from column/row with niri's
   unambiguity discipline, excluding tabbed columns. Membership reporting alone
   is unconditional Rust work. **Dry-run episodes are not**: the existing
   bridge admits a group only on a *reported* vertical secondary axis, so
   `zk longcat --backend umbriel --dry-run` keeps emitting its unsupported
   planning result until one of the three axis paths above lands — a native
   `workspace_axis` field, an externally supplied axis, or an explicit
   evidence-policy change accepting corroborated arrangement. Only then does
   dry-run produce real episodes with real baselines and preview decisions.
   Writes remain correctly refused either way. Evidence: scripted-peer replay
   tests plus a read-only live capture.
2. **Upstream asks, re-prioritized.** (a) the output's `workspace_axis` in the
   outputs payload — tiny, unblocks authoritative admission; (b) by-ID secondary
   extent, ideally pixel-targeted, following the `window-move-to` trailing-id
   precedent — proves targeting without focus/reveal side effects but is
   explicitly *not* group restoration (the clamps, floor, edge-gap and
   denominator interactions above still apply); (c) native sizing-state
   observability plus a conditional group allocation — the full
   [Weave 2](longcat-needs0.gpt6a.md) contract, unchanged by the refresh.
3. **Umbriel actuator in zk** only after (b): a `zk_umbriel::HeightActuator`
   mirroring the niri one's basis-revalidation and transmission-outcome
   contract.

The [runner's lesson](longcat-needs0.gpt6a.md) that latest-healthy-state is
insufficient for admission is unaffected by any of this: local history gating,
basis checks and effect retention remain the consumer's obligations even with
perfect native facts.

## Connections

- [Upstream refresh](upstream-refresh0.gpt6a.md): the native API delta this
  account consumes; its table remains the catalog of record.
- [Layout-control needs](longcat-needs0.gpt6a.md): the write/restoration
  thread analysis stands; this document updates its observation-gap row.
- [Runner contract](file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md)
  and [layout contracts](file:///home/rektide/src/zooming-kittens/layout/README.md):
  the implemented consumer behavior summarized above.
- [First Umbriel investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md):
  its restoration preconditions and focus-race analysis remain valid; its
  no-Rust-client and no-membership claims are historical.
