---
type: Research
title: Practical relative sizing with the existing Umbriel API
description: Usable focused resizing, native sibling redistribution, directional boundary adjustments and a simpler application contract without exact restoration prerequisites.
resource: design/layout-control/practical-sizing0.gpt6a.md
status: proposed
generated: { at: 2026-10-22, by: 'model:openai/gpt-6-astra#medium' }
sources:
  - resource: opencode://ses_e9b39fedaffejzlE158Z9Oq5aK
    title: Human redirects research from exact Longcat restoration toward useful relative sizing; trace current actions and revise the adaptation boundary
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/docs/user/layout.md
    title: Layout-specific sizing semantics, presets and client constraints
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/docs/user/actions.md
    title: Runnable focused sizing and edge commands
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/workspace/workspace.cpp
    title: Native fraction deltas, focused resizing and edge-resize sessions
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/layout/scrolling.cpp
    title: Sibling-preserving weight changes, tab units, edge gaps and native column maximize toggle
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/layout/master.cpp
    title: Master/stack fractions and relative row allocation
  - resource: https://github.com/noctalia-dev/umbriel/blob/5d657581/src/layout/dwindle.cpp
    title: Fraction controls operate on horizontal or vertical split ancestry
  - resource: ../../crates/umbriel-protocol/src/action/extent.rs
    title: Existing typed fraction, delta and directional resizing APIs
  - resource: ../../crates/umbriel-client/examples/shell.rs
    title: Existing executable raw-action route for exercising resizing through Rust
  - resource: longcat-current0.glm53m.md
    title: Stronger episode and restoration contract that must not gate simpler focused sizing
verification:
  method: Inspected native implementation and documentation at fetched revision 5d657581 and existing Rust action builders.
  limits: No live desktop mutation or deployed-version check; allocation examples describe native weight algebra, not measured pixels.
---

# Practical relative sizing

**Useful relative resizing already works through the native API and already has
typed Rust builders.** The preceding research over-weighted Longcat's exact
observation, identity and restoration needs. Those are requirements for that
particular controller, not prerequisites for changing relative window sizes.

The smaller contract is: **resize whatever is focused when the compositor handles
the command, and let its layout engine redistribute the space.** No group query,
axis query, by-ID setter, solver, saved baseline or compositor patch is required
for that contract. Primary/secondary commands delegate axis interpretation to
the compositor itself.

## Things we can do now

Commands below are alternatives to bind to keys or invoke through a control UI.
They act on focus at execution. Invoking from a terminal normally targets that
terminal; use a keybinding to operate on another focused application.

```sh
# Scrolling: give the current row/unit roughly 60% of its column allocation.
umbriel msg window-set-secondary-extent:0.6

# Increase/decrease its native share by ten percentage points.
umbriel msg window-modify-secondary-extent:0.1
umbriel msg window-modify-secondary-extent:-0.1

# Scrolling: make the current column about two thirds of the viewport extent.
umbriel msg window-set-primary-extent:0.667

# Native configured presets, normally 0.333 / 0.5 / 0.667.
umbriel msg window-cycle-secondary-extent
umbriel msg window-cycle-primary-extent

# Grow outward from a physical edge; negative deltas shrink from that edge.
umbriel msg window-modify-height-down:0.05
umbriel msg window-modify-width-right:0.05
```

Set means an absolute **native fraction**, modify adds to the current native
fraction, and cycle selects a configured preset. A delta of `0.1` is ten
percentage points, not ten percent of the window's current pixel size.
The compositor owns clamps, arrangement and client configure requests.

### What does each layout do?

| Layout | Primary extent | Secondary extent |
|---|---|---|
| Scrolling | Current column's extent along the strip; other column widths remain their own | Current row/tab unit's share inside its column; other units retain relative weights |
| Master | Master/stack split; focusing a stack area changes the complementary master fraction | Current row's share within its area; siblings retain relative weights; unavailable for a solo or tabbed area |
| Dwindle | Adjust horizontal split ancestry to change the focused tile's width fraction | Adjust vertical split ancestry to change its height fraction |
| Floating | Width relative to output usable area | Height relative to output usable area |

For default scrolling (`workspace_axis = "vertical"`), primary is column width
and secondary is row height. With horizontal workspace movement, scrolling is
transposed. A generic primary/secondary control remains useful in either case;
it does not have to discover that axis first. Physical edge commands give the
alternative vocabulary of left/right/up/down.

## Native redistribution is the capability we were overlooking

For multiple scrolling units, `setHeightFraction` computes:

```text
others = total native weight - focused unit weight
new focused weight = requested_fraction * others / (1 - requested_fraction)
```

Other unit weights are unchanged. Consequently their **ratios to one another
are preserved**, while their shares of the whole change. Without edge gaps,
tabs or binding clamps/floors:

- Three equal rows, focus the first, set `0.6`: shares become **60/20/20**.
- Other rows have relative weights 2:1, focus a third, set `0.6`: the remaining
  40% divides **26.7/13.3**, preserving 2:1.
- A focused row at 40%, modify `0.1`: its native target becomes 50%.

This already supplies a simple emphasis operation. It does not require assigning
every sibling separately. Committed content pixels may differ through rounding,
gaps, decoration and client sizing. The multi-unit target clamps to 10–95%; a
minimum internal weight can further limit the requested result.

Tabs are also workable for this smaller contract: the compositor resizes the
whole tab unit. We need not reconstruct it to ask "give this focused area more
room." Longcat's independent-per-window solver is what makes tabs a modeling
problem. A solo scrolling unit can instead trade space with top/bottom edge gaps;
shrinking it need not enlarge a sibling that does not exist.

### Edge nudges are a second useful control

`Workspace::resizeFocusedEdge` opens the layout's native `ResizeGrab`, then
applies a delta as if one pointer-drag step occurred. Its magnitude is the given
fraction of tiled usable width/height, not a fraction of the focused window.
Positive deltas grow outward even on top/left edges; negative deltas shrink.

This provides boundary-oriented adjustment instead of a global target share.
The affected neighbor or split is chosen by the layout's edge-resolution rules.
Some boundaries are unavailable, and constraints can clamp movement. It is not
necessary to export the layout tree before using these native operations.

## How to adapt our client

Use a distinct **focused-sizing** contract alongside the stronger by-ID Longcat
actuator. The latter's `height_by_id` gate correctly refuses Umbriel; simply
turning that gate off would misrepresent what it guarantees.

The small feature can expose:

- `grow/shrink primary` and `grow/shrink secondary`, using native deltas;
- `set primary/secondary share`, for useful sizes such as 0.5 or 0.667;
- `cycle primary/secondary`, using native presets;
- directional edge nudges, for explicit spatial adjustments.

These Rust builders **already exist** in `action/extent.rs`:

```rust
use umbriel_protocol::action::{Action, ExtentDelta, ExtentFraction};

let emphasize = Action::window_set_secondary_extent(ExtentFraction::new(0.6)?);
let grow = Action::window_modify_secondary_extent(ExtentDelta::new(0.1)?);
let widen = Action::window_set_primary_extent(ExtentFraction::new(0.667)?);
let lower_edge = Action::window_modify_height_down(ExtentDelta::new(0.05)?);
```

Pass an action's `request()` through the existing endpoint request interface.
For an executable path today, run `cargo run -p umbriel-client --example shell`
and enter `raw window-modify-secondary-extent:0.1`. This uses our Rust transport,
not an additional `umbriel` CLI subprocess. Native keybindings are more convenient
when the target should be an application other than the command terminal.

### A modest automatic policy is also possible

If desired, a focus-dwell controller could issue **one absolute set**, such as
`secondary = 0.6`, after activation settles, and leave that allocation in place.
Avoid sending another command for every geometry update: those updates include
the results of our own resize. A native reply is not evidence that geometry has
settled, and an inapplicable action can be a no-op.

The policy must explicitly mean "emphasize current focus at handling time."
A last-moment focus check reduces stale commands but cannot turn this into
identity-targeted actuation. This is a limitation of automatic historical-target
work, not a blocker for deliberately current-focus controls.

Repeated focus emphasis can reshape the column over time. That is acceptable
only if the selected policy is persistent, approximate emphasis rather than
baseline-preserving Longcat. Incremental nudges should be user-driven or bounded;
replaying them on every observation would grow until clamped.

## Undo and normalization: useful, narrower meanings

- Opposite deltas are a convenient correction, not guaranteed exact undo after
  clamps, focus changes or intervening edits.
- Setting `0.5` means give **this** unit half, not reset the whole column.
- Sequentially setting every row to `1/N` does **not** generally equalize a
  column: each command changes the denominator for the following one.
- Scrolling has a native full-width toggle (`window-toggle-maximize`) that saves
  and restores the column width. This is a real, limited native restoration
  capability; unrelated width edits can clear that saved state. It is not a
  general secondary-size baseline restore.

Exact restoration remains a separate optional upgrade. The immediate route is
to use the existing native allocation operations and offer honest focused-size
controls, rather than make that upgrade a prerequisite for all resizing.

## Connections

- [Upstream refresh](upstream-refresh0.gpt6a.md): observation additions and version
  coordinates; useful for richer UI, not required for these commands.
- [Noctalia survey](noctalia0.glm53max.md): its companion plugin already uses
  focused primary-size presets; this extends that practical precedent to
  secondary shares and native boundary nudges.
- [Longcat research](longcat-current0.glm53m.md) and
  [exact-control requirements](longcat-needs0.gpt6a.md): stronger contracts for
  targeted, baseline-preserving automation. They should not gate this simpler use.
