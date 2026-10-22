---
type: Handoff
title: ZK handoff — use existing Umbriel focused sizing
description: Consumer-side continuation for useful relative sizing through already implemented umbriel-rs actions, separate from Longcat's by-ID restoration contract.
resource: design/handoff/zk-focused-sizing0.gpt6a.md
status: proposed
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9b39fedaffejzlE158Z9Oq5aK
    title: Human requests a ZK handoff after clarifying that useful relative sizing already exists in Umbriel and umbriel-rs
  - resource: ../layout-control/practical-sizing0.gpt6a.md
    title: Governing practical guide with native semantics, commands, Rust builders and adaptation options
  - resource: ../layout-control/longcat-current0.glm53m.md
    title: Current ZK implementation and its stronger observation and actuation gates
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md
    title: Current Longcat runner rejects Umbriel writes and requires reported groups and vertical axis
  - resource: ../../crates/umbriel-protocol/src/action/extent.rs
    title: Existing typed relative-sizing methods
  - resource: file:///home/rektide/src/zooming-kittens/design/remake/README.md
    title: Consumer-side remake reading map
verification:
  method: Checked existing action builders, runner contract and ticket inventories; links and provenance checked with Rekon.
  limits: Handoff only; no ZK implementation or live desktop mutation. Acceptance checks below are proposed work, not completed tests.
---

# ZK handoff: existing Umbriel focused sizing

## Read this first

**[Practical relative sizing](../layout-control/practical-sizing0.gpt6a.md) is the
main guide.** It has runnable commands, native redistribution semantics, existing
Rust methods, layout-specific behavior and the smaller consumer contract.

The human's intent is useful control over relative sizes. Exact Longcat baseline
restoration must not be a prerequisite for that. Umbriel already implements the
operations, and umbriel-rs already exposes them. **The next implementation belongs
in ZK**, not in a new compositor API or Rust-library feature.

## What is ready

In `umbriel_protocol::action`:

- `Action::window_set_primary_extent(ExtentFraction)`
- `Action::window_set_secondary_extent(ExtentFraction)`
- `Action::window_modify_primary_extent(ExtentDelta)`
- `Action::window_modify_secondary_extent(ExtentDelta)`
- Directional `window_modify_width_left/right` and `window_modify_height_up/down`
  builders, plus bare cycle actions in the existing catalog.

Use `ExtentFraction::new` / `ExtentDelta::new`, then the existing endpoint request
path with `action.request()`. These are native socket commands; no subprocess is
required. The existing Rust shell example can already exercise them through `raw`.

The compositor performs allocation: setting a focused scrolling unit to 60%
preserves sibling weight ratios (three ordinary equal rows become approximately
60/20/20). This single operation does not need a solver or complete layout capture.

## Consumer contract to implement

Expose **focused sizing**: resize whatever is focused when Umbriel handles the
command. The proposed opening slice is one-shot, keybindable set/nudge/cycle and
edge controls. Choose CLI spelling to fit ZK; this handoff does not prescribe a
new command namespace.

The existing `zk longcat --backend umbriel` refusal is specific to its by-ID,
baseline-preserving contract. Do not make it claim `height_by_id` support or route
focused commands through an actuator that promises identity targeting. Add a
separate explicit operation/mode with the actual current-focus semantics.

Primary/secondary controls let the compositor resolve the axis. Native topology,
output and axis model upgrades can improve richer consumers later; they do not
block these operations. Tabs resize as native allocation units. Native clamps,
unavailable boundaries and client constraints can limit the result.

For a later automatic mode, the guide proposes one absolute secondary-share set
after focus dwell, leaving the allocation in place. That is a separate policy
choice: no implicit restoration, no command on every geometry update, and no
claim that a preflight focus check makes the eventual write by-ID.

## ZK entry points

- [Runner contract](file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md):
  explains current refusals and the stronger episode pipeline.
- `app/zk/src/commands/longcat.rs` and `app/zk/src/runner/control.rs`: current
  Longcat admission surfaces; investigate for reuse, not merely gate removal.
- `desktop/umbriel`: current native integration and socket discovery context.
- [Current consumer research](../layout-control/longcat-current0.glm53m.md):
  implementation map, niri comparison and exact-control limitations.

## Suggested acceptance evidence

- Scripted-peer verification that a valid operation emits the selected native
  action once; invalid fractions/deltas fail before transmission.
- Preserve native rejection and transport uncertainty. A handled command does
  not promise settled pixels; do not automatically replay an uncertain nudge.
- Keep existing niri Longcat behavior and Umbriel by-ID refusal intact.
- Document the target as current focus, fractional units and native constraints.
- Deliberate live trial on ordinary stacked windows: set 60%, nudge up/down,
  change primary size, and exercise an available edge. Compare observed behavior
  to the guide without requiring exact pixel ratios or baseline restoration.

## Tracking and delivery state

Ticket inventories checked on October 22:

- umbriel-rs has completed `umbrs-protocol-actions` and `umbrs-ep-research`;
  append research/consumer-handoff notes there without reopening completed work.
- ZK has existing Longcat and UI tasks, but no dedicated Umbriel integration or
  focused-sizing work item was found in its full ticket listing. The human will
  bring up the ZK-side tracking. No new ticket is created by this handoff.

Only research/docs and ticket notes are delivered here. ZK implementation remains
the receiving session's work. Exact conditional restoration remains separately
documented in [native control requirements](../layout-control/longcat-needs0.gpt6a.md).
