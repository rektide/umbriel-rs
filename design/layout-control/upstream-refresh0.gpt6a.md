---
type: Research
title: Current Umbriel layout control after archive refresh
description: Source-pinned October 22 API delta separating newly available layout observations from still-missing targeted tiled sizing and restoration.
resource: design/layout-control/upstream-refresh0.gpt6a.md
status: current
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9b39fedaffejzlE158Z9Oq5aK
    title: Refresh archived upstream without displacing local compositor work and reassess layout control against current sources
  - resource: longcat-needs0.gpt6a.md
    title: Earlier consumer requirements and native capability gaps
  - resource: ../../crates/umbriel-protocol/src/model/inventory.rs
    title: Rust inventory models predate native column and row fields
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/docs/user/ipc.md
    title: Current native observations and stream semantics
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/docs/user/actions.md
    title: Current action grammar including floating movement and focused extent setters
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/server/ipc_commands.cpp
    title: Actual scrolling membership serialization and geometry semantics
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/workspace/workspace.cpp
    title: Focused secondary sizing and visibility side effects
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/layout/scrolling.cpp
    title: Tab-unit weights, edge gaps and fraction mutation
verification:
  method: Fetched upstream and inspected current source, action documentation and existing Rust models.
  limits: Source research only; no running compositor version verification or live mutation.
---

# Current Umbriel layout control

**Umbriel has layout controls, but not the complete targeted, reversible tiled
allocation API Longcat needs.** Current upstream has also closed part of the
observation gap since the [earlier requirements](longcat-needs0.gpt6a.md).

## Archive update and evidence coordinate

Fetched `https://github.com/noctalia-dev/umbriel` on October 22. Both `origin/main`
and the archive's local `main` advanced from `5731190d` to `5d657581` (121 commits
ahead by the branch report). The existing checkout at
`/home/rektide/archive/noctalia-dev/umbriel` contains detached local development
history at `7666af0a43560658e7b966befcbe47c101968e1c`; it was clean and preserved.

Current upstream is checked out separately at
`/home/rektide/archive/noctalia-dev/umbriel-upstream`, detached at `5d657581`.
This is an archive worktree, not a deployment or a replacement of the locally
developed compositor. All upstream claims below use that fetched revision.

## What changed

| Concern | Current native API | Consequence for this library and Longcat |
|---|---|---|
| Scrolling membership | `windows` reports zero-based `column` and `row`; `-1` outside scrolling membership | Exact snapshot membership/order no longer requires geometry inference for scrolling. These are positional indices, not durable group identities. |
| Tabs | `tabbed`, `tab_index`, `tab_hidden` | A tab group shares an allocation unit; treating every window as an independent stacked row is wrong. |
| Window state | Output, parent, fullscreen, maximized, pinned, visible, snap and other fields | More native admission/context facts are available than the current Rust inventory models expose. |
| Outputs | Native `outputs` query and event include logical size and usable area, scale, modes, enablement and power | Current upstream no longer requires the separate Wayland inspection path merely to obtain output topology/workarea. |
| Layout modes | Scrolling, dwindle, master and floating | The library's earlier catalog is a versioned implementation, not a claim of complete current-upstream coverage. |
| Floating placement | `window-move-to:<anchor>/<x>/<y>[/<window-id>]` and `window-move-by:<dx>/<dy>[/<window-id>]` | By-ID position changes exist for floating windows; these do not size tiled groups. |
| Tiled sizing | `window-set-secondary-extent:<fraction>` and related primary/modify/cycle actions | Still focused-area actions, not ID-addressed group allocation. |
| Dwindle control | `layout-toggle-split` plus directional operations | Useful interactive layout control; not an exported tree with conditional restoration. |

The implementation evidence is `IpcCommands::windows` around lines 603–697:
`columnOf`/`rowOf` populate membership only for a scrolling layout, while tab
metadata is separately derived. Positions use layout targets (or drag positions),
and sizes use committed content geometry. A column index can change after
insertion, removal or reorder; it is not a stable incarnation or revision.

The current Rust `Window` model lacks these membership/tab/state additions;
`Workspace` likewise lacks the newer occupancy/focus details. Existing raw capture
preservation and typed consumer access are different capabilities. Updating typed
observations is useful work independently of authorizing any layout writes.

## What still blocks exact reversible allocation

`Workspace::setFocusedHeight` (lines 1927–1940) operates on `m_focusedView`.
It can clear maximize state and calls `ensureFocusedVisible`. Adding an ID to
a Rust method cannot change this native dispatch contract. Focusing a window
before resizing also creates a race and changes user-visible focus.

`ScrollingLayout::setHeightFraction` (lines 1365–1392) sizes an entire tab unit.
For a single unit it redistributes the remainder to top/bottom edge gaps; for
multiple units it sets a weight relative to the other units and gaps. Fractions
are clamped, and native weights have a floor. Sequential fraction commands are
therefore not a simple assignment of a saved vector of pixel heights.

The inspected IPC does not export native row-unit/edge-gap weights, a complete
layout tree, scroll offset, a sizing revision, or a conditional group-allocation
transaction. `column`/`row` and output usable areas help observation, but do not
recover the hidden sizing state or prove a restoration remains applicable.
The native snapshot is also not a cross-family atomic cut.

## Revised next-work boundary

1. **Rust/consumer observation update:** model current scrolling membership,
   tab units and relevant state; add native output query/events and floating-mode
   support. Distinguish absent old-server fields from known empty membership.
2. **Native control proposal:** explicitly targeted sizing without focus/reveal
   side effects, an observable native sizing basis, and a conditional group write
   or a deliberately specified restoration token.
3. **Consumer policy adaptation:** use the new facts with Longcat's existing
   identity and effect-evidence rules. Keep topology changes, tabs, axis and
   restoration conflicts explicit.

These are research conclusions, not implemented changes. The earlier
[Longcat requirements](longcat-needs0.gpt6a.md) still explain conditional writes
and restoration well; their blanket absence-of-column-membership statement is
historical and superseded by this source refresh.
