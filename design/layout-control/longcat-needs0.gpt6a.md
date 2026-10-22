---
type: Design
title: Layout-control needs — Longcat, Umbriel and Kubeless
description: Consumer-grounded native protocol requirements, Rust-library opportunities and alternative compositions for observable, targeted and reversible layout control.
resource: design/layout-control/longcat-needs0.gpt6a.md
status: proposed
generated: { at: 2026-10-17, by: 'model:openai/gpt-6-astra#high' }
sources:
  - resource: opencode://ses_e9b39fedaffejzlE158Z9Oq5aK
    title: Add current-upstream correction pointer while preserving the earlier requirements argument
  - resource: opencode://ses_ec3316eefffejmDBXbZ28spBd7
    title: Longcat remake implementation, verified consumer findings, and human request to bring Umbriel requirements and cross-library compositions into umbriel-rs
  - resource: ../kubeless/handoff0.gpt6a.md
    title: Delivered inventory receipts and native change/impact example; repository ownership and publication boundaries
  - resource: ../kubeless/asks0.gpt61s.md
    title: Earlier reuse opportunities, including proposals superseded by the publication-first implementation
  - resource: ../../README.md
    title: Current subscriptions, captures, inferred shape and six-crate ownership
  - resource: ../../crates/umbriel-protocol/src/model/inventory.rs
    title: Actual geometry, activation and workspace fields
  - resource: ../../crates/umbriel-protocol/src/action/extent.rs
    title: Current fraction builders have no explicit window target
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/server/ipc_commands.cpp
    title: Native windows/workspaces payloads expose no exact group or sizing state
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/workspace/workspace.cpp
    title: Focused-height mutation also handles maximize state and visibility
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/layout/scrolling.cpp
    title: Row fractions, edge-gap weights, clamps and sibling-preserving weight updates
  - resource: file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md
    title: Implemented consumer contract, native input requirements, history-gated admission and retained effect evidence
  - resource: file:///home/rektide/src/zooming-kittens/design/remake/kubeless-asks0.gpt6a.md
    title: Actual Kubeless adoption, integration findings, completed checks and explicitly deferred fairness work
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Earlier source-grounded restoration algebra, focus race and axis/identity investigation
verification:
  method: Inspected existing design corpus and rechecked native/Rust models, action builders and sizing implementation; integrated prior consumer verification account.
  limits: Documentation only. Proposed protocol/API shapes are not implemented or compile-checked. No new live acquisition, mutation or test run.
---

# Layout-control needs

**October 22 source update:** current upstream now exports scrolling `column`
and `row`, tab metadata and output usable areas. The absence-of-membership claims
below describe the older inspected revision. Read the
[upstream refresh](upstream-refresh0.gpt6a.md) for the corrected observation
boundary; the focused sizing and conditional-restoration gaps remain.

## Why this document exists here

Longcat now has an implemented consumer in
[`zooming-kittens`](file:///home/rektide/src/zooming-kittens/ARCHITECTURE.md):
native observations, a Taffy solver, a pure baseline/intent/effect state machine,
direct Kubeless scheduling, and niri by-ID execution. Umbriel's stream works;
its current adapter deliberately reports unsupported grouping and refuses writes.

This document is the **Umbriel-side requirements and opportunity account** for
closing that gap. It brings together three threads:

1. **Native layout-control semantics**, owned by the C++ compositor (`umbrield`).
2. **Typed access and observed state**, owned by this Rust library.
3. **Change, impact and work composition**, using selected Kubeless mechanisms
   while Longcat retains its application policy.

The existing [Kubeless handoff](../kubeless/handoff0.gpt6a.md) explains delivered
receipt/publication work. Earlier Longcat requirements mostly lived in the
consumer repository; neither that handoff nor the whole-protocol library is a
specification for exact reversible layout control. This proposal supplies that
missing connection. It does not authorize changes in all three repositories.

### Evidence and standing

On 2026-10-17, umbriel-rs was clean at an empty working copy above `74cbff32`;
the inspected compositor checkout remained
`7666af0a43560658e7b966befcbe47c101968e1c`. These are source coordinates, not the
running daemon's verified build identity. The consumer's earlier integrated
verification reported 154 passing tests, strict Clippy/formatting, and a read-only
live capture of 31 windows/eight workspaces. Those tests do not verify the new
protocol proposals below or live native restoration.

## The behavior we need

A person focuses one window in a stacked scrolling group. Longcat enlarges it,
shares the remaining extent among its siblings, and can later restore the
pre-emphasis allocation without moving focus or overwriting an intervening edit.
Focus changes inside the group recompute from the original baseline, rather than
compounding previous enlargement. Closing, moving or reordering a member changes
the basis on which restoration was justified.

The application therefore needs to answer:

- Which exact windows share this allocation, in which order and along which axis?
- What sizing state existed before our effect, including edge gaps and constraints?
- Can we apply a change to those identities without selecting them as seat focus?
- Was the intended state accepted, and what later geometry actually became visible?
- Is our basis still applicable before another write or restoration?
- What evidence survives timeout, cancellation, disconnection or another actor's edit?

Taffy solves an allocation from supplied facts. It cannot discover hidden columns,
recover native weights from clamped pixels, or make a sequence of socket commands
atomic. Kubeless cannot manufacture those native semantics either.

## Current capabilities and the precise gaps

| Need | Existing capability | Remaining owner/work |
|---|---|---|
| Observe activity | Native full-family subscriptions, bounded observers, epoch/loss evidence | Already usable; no periodic polling prerequisite |
| Observe additions, changes and removals | `Inventory::install_windows_changes` and native replacement receipts | Already usable; consumer chooses relevant equality and impact |
| Exact groups/order/axis | Workspace layout mode and window geometry; capture tool infers spatial shape | Compositor must report actual topology; Rust then models it |
| Known sizing basis | Target x/y and committed content w/h | Native row/edge sizing state and target extents are missing |
| Targeted sizing | Rust `window_set_secondary_extent(ExtentFraction)` mirrors native focused-window action | By-ID native operation plus typed Rust API |
| Conditional application | Local capture/version checks | Server-side expected-state check and mutation in one native turn |
| Reversible group allocation | Client-side pixel baselines; native width restoration precedent in earlier research | Complete native sizing representation or explicitly designed restore token |
| Settlement | Native reply and later snapshots | Better target/revision evidence; client still observes committed geometry |
| Complete transition history | Coalesced native snapshots | Not offered; retain explicit continuity limits instead of promising an audit log |

The concrete source gap is visible in `model/inventory.rs` and
`IpcCommands::windows/workspaces`: no group identity, strip axis, weights or
layout revision. `Workspace::setFocusedHeight` reads `m_focusedView`, may clear
maximized state, calls `ensureFocusedVisible`, and marks arrangement. A Rust
overload taking an ID cannot bypass this native target/side-effect contract.

## Thread 1: native capabilities to propose

### A. Exact layout observation, with explicit sizing semantics

Start with scrolling groups; report other modes as unsupported for this operation
rather than forcing dwindle/master into a fictitious column model.

Proposed native facts:

- Workspace identity and layout mode; primary/secondary axis and orientation.
- Group identity scoped to a compositor instance, ordered member IDs, and an
  explicit identity/revision rule for insertion, merge, split and reorder.
- Target tile/content extents with coordinate units, decoration/gap treatment,
  and available allocation span. Committed content geometry stays separate.
- Native sizing representation sufficient for restoration: row weights and
  top/bottom edge-gap weights, or another canonical representation with equivalent
  fidelity. Include relevant constraints or the actual constrained target result.
- A control-relevant revision and scope. A positional index plus an ordered-member
  fingerprint and revision is a possible first contract if stable group IDs are
  expensive; the essential property is rejecting a changed basis, not a UUID.

Publish this through initial state **and subsequent updates**. Prefer a coherent
layout-family snapshot containing group/order/sizing facts at one native cut.
If split across families, provide a joinable native revision and specify when a
consumer has a usable set. Adjacent local receive positions are not such a join.
Expose explicit empty state, unsupported mode and missing/unready information as
different cases.

Avoid one revision counter for everything. Title changes and asynchronous client
commits should not necessarily invalidate a layout write. Define what advances
the topology/sizing precondition, and separately what establishes observed
geometry progress. Output/workarea or constraint changes must be accounted for
when they change operation validity. Native-instance identity prevents a revision
from a restarted compositor matching an old request.

### B. Explicit-target writes without focus/visibility side effects

Provide an operation that takes an explicit window or group target and sizing
argument. A dedicated structured request is a plausible fit because existing
action-string grammar is not already an ID-plus-fraction API. Reusing action
dispatch is also possible if its grammar and semantics are deliberately extended.

Required behavior:

- Select by native identity; do not call focus as an implementation workaround.
- Validate target existence, membership, mode and axis at execution.
- Keep seat focus and active workspace unchanged. Define scrolling/visibility and
  maximize/fullscreen behavior explicitly; do not inherit `ensureFocusedVisible`
  or maximize clearing accidentally from the keybinding action.
- Reject a missing target, incompatible mode or stale basis without partial writes.
  Preserve rejection information distinguishable from transport uncertainty.
- Return useful native handling evidence: affected identities, resulting relevant
  revision and accepted sizing state where possible. This is not a render-complete
  promise.

By-ID targeting prevents resizing the *wrong* focused window. If policy requires
the target to still be active, that is an additional expected-active precondition;
identity targeting alone does not guarantee the emphasis is still timely.

### C. Conditional group sizing and meaningful restoration

The preferred complete-control contract is **conditional group allocation**:
apply a declared sizing vector only if the native instance, group membership and
control revision still match. Check and mutate together on the compositor event
loop. A rejection changes nothing; success updates the whole group allocation
before arrangement. Visual client commits can still arrive independently.

This avoids interpreting a series of per-window fraction setters as a vector
assignment. In `setHeightFraction`, the selected row's weight is derived from
the current sum of *other* weights. Sequential setters can influence later
denominators; they are not inherently an atomic restore of an arbitrary baseline.

Restoration must cover edge gaps as well as rows. The inspected implementation
includes edge gaps in `heightFraction`, changes them for solo rows, clamps targets
to `[0.1,1.0]` or `[0.1,0.95]`, and floors internal row weights. Committed pixels
can additionally reflect rounding, client constraints and configure lag. Merely
remembering observed heights or one fraction cannot universally recover the
original internal state.

Two viable native restoration designs:

1. **Canonical sizing state + conditional vector assignment.** The client saves
   a native representation, applies emphasis, then restores it against the
   *resulting* revision from its latest accepted operation. The old baseline
   revision is no longer the expected revision after our own write. External
   changes cause a conflict instead of being overwritten.
2. **Opaque reversible token.** The compositor returns a token binding baseline,
   group incarnation and permitted restoration conditions. A later restore either
   succeeds under those conditions or reports conflict/expiry. This requires
   explicit token lifetime, bounds, restart and reuse semantics; it is a larger
   native responsibility and not the first requirement.

Restoring native weights means restoring those weights under current constraints,
not promising identical pixels after an output resize. Decide whether changed
workarea/constraints reject restoration or permit equivalent native state; make
the choice visible in capabilities and results.

### D. Discovery and outcome evidence

Add a way to discover the running server's relevant capabilities and schema
version. The Rust package version is not the daemon's capability set. An older
server's unsupported response must not silently trigger a focus-based fallback.

For timeouts, a possibly-sent request remains uncertain. A later matching snapshot
shows compatible state, not who caused it. A native operation identifier with
deduplication/outcome lookup could strengthen recovery, but only if the server
defines retention and replay semantics. A client-generated ID alone is not
idempotency. This is a possible later extension; the first client can retain
uncertainty and refuse blind retries.

Illustrative semantic transaction, **not an implemented wire format**:

```text
expect: native instance + group + ordered members + sizing revision
write:  canonical native sizing state (rows and edge gaps)
result: applied(new revision, accepted state) | conflict(current revision)
        | unsupported | target gone | invalid sizing
```

## Thread 2: umbriel-rs improvements

Place new wire models and validated requests in `umbriel-protocol`; retain one
reply-ownership mechanism in `umbriel-session`/`umbriel-client`. Existing sequential
subscribed commands can carry new operations; this does not require a second
correlation ledger, mandatory pipelining or a new socket framework.

Extend `umbriel-state` for layout-family installation and coherent captures when
the native family exists. Keep native order, health, old/new stamps and actual
replacement evidence with that owner. Reuse `kubeless-view-memory`; do not create
a competing map simply to expose a different consumer interface.

Useful library-only improvements can precede the protocol changes:

- A typed state-feed composition over the existing raw subscription and Inventory,
  if the native example and zk adapter demonstrate enough duplicate ownership
  code. Retain raw delivery as an independently usable API and preserve native
  frame/observer byte bounds, loss, explicit source lifetime and invalid snapshots.
- Public layout-shape inference for diagnostics/capture reuse if useful. Keep
  `Inferred`/unresolved evidence: making an algorithm public cannot promote its
  result into exact compositor topology.
- A documented admission recipe that combines current state with **processed
  continuity history**, based on the real consumer's lesson below.

New layout facts could later be emitted through Arrow/Polars with provenance,
but that is an inspection consumer. Longcat control should not acquire a Polars
dependency to get authoritative topology or revision checks.

## Thread 3: Kubeless composition and opportunities

### What already works

The existing [Umbriel handoff](../kubeless/handoff0.gpt6a.md) supplies the native
window receipt entrance. The [implemented zk account](file:///home/rektide/src/zooming-kittens/design/remake/kubeless-asks0.gpt6a.md)
shows its actual next consumer:

- Umbriel `Inventory::install_windows_changes` supplies real before/after evidence.
- `view-impact::affected_roots_with` visits former and current relations. Moving
  or removing a member must reconsider its former group as well as the current one.
- Niri uses the same memory leaf directly; no second Umbriel reflector was needed.
- The app uses direct `work-schedule` with bounded intake and one reconsideration
  key for its bounded episode set. It does not yet run independent group queues.
- Taffy and the pure episode core have no Kubeless dependency. They consume facts
  and produce decisions; native policy remains with the application.

### The important lesson: latest healthy state is insufficient for admission

A queue can contain healthy state → gap/invalidity → recovered healthy state,
while `current()` already returns the final state. A controller reading that
latest value and checking only one old event can carry baseline authority across
an unprocessed discontinuity. Local atomic captures do not fix this.

The implemented runner drains at most eight historical updates per turn, checks
every continuity marker, and only admits effects after catching up. A final
version/scope check and native precondition narrow different races. This is the
best concrete material for the proposed **non-resumable control composition
example** in Kubeless: actual receipts → impact → bounded reconsideration →
native write evidence → observed settlement, including recovery and unknown effects.

An optional future optimization is a continuity-generation witness attached
atomically to captures. Design it only if bounded history processing proves
awkward: source gaps and *observer-local* lag differ, and a shared generation
alone does not witness a particular consumer's delivery continuity. It must not
invent resumability or replace necessary per-consumer evidence. No new generic
primitive is justified merely by naming this possibility.

### Further reuse without compulsory framework adoption

- Independent lagging publication already exists for additional derived-state
  consumers. It is not a reason to replace native socket delivery, whose byte
  accounting and terminal semantics differ. The first zk JSONL consumer did not
  need another publication layer.
- Acknowledgment/settlement helpers are worth fitting to native group operations.
  Receipt ownership must survive cancellation of an individual wait; the current
  application retains effects in its episode owner. Do not hide unknown writes
  behind a generic retry loop or confuse local view anchors with remote CAS.
- A generic receipt/impact/scheduling example can improve API discoverability
  without moving Longcat behavior into Kubeless. Build the consumer, then lift
  the proved reusable composition—the human-selected order.

**Kubeless stream-scheduler fairness remains explicitly deferred.** Use bounded
caller-driven `Schedule` here. None of the native observation/control asks require
that upstream scheduler project or the full retained-action host.

## Potential weaves

### Weave 1 — observation first, useful independently

Compositor exact layout family → typed Rust models → native Inventory receipts
→ Kubeless impact → Longcat/Taffy previews and layout diagnostics.

Payoff: trustworthy grouping and sizing evidence, better capture/export, and a
real basis for comparing inferred shape with native topology. No mutation API
is required to deliver this step. It leaves focus-driven execution unavailable.
Recommended first implementation because it resolves the facts on which all
later contracts depend.

### Weave 2 — conditional native sizing, application-owned policy

Weave 1 + targeted conditional group allocation → Rust typed command/result
→ retained application effect → streamed target/committed geometry → conditional
native-state restoration.

Payoff: complete Longcat-style control without moving focus policy into the
compositor. Taffy expresses desired allocations; the adapter compiles to native
weights only where the mapping is sound, or uses native-share emphasis when
that better preserves semantics. Label strategies rather than claiming pixel
identity across compositors. This is the recommended destination.

A smaller by-ID single-row command may land first to prove targeting and side
effects, but is not by itself exact group restoration. Maintain that distinction
in capability discovery and tests.

### Weave 3 — compositor-owned reversible operations

Weave 1 + native reversible group operation/token → Rust typed receipts →
application focus policy and Kubeless reconsideration.

Payoff: the compositor can retain inaccessible sizing details and enforce restore
conditions directly. Cost: native token lifecycle and resource ownership become
part of the protocol. Compare it against canonical-state assignment before
selecting it; do not implement both restoration mechanisms by default.

These can share a read-only observation foundation. They are options for choosing
responsibility, not three parallel implementations we need to build.

## Landing and acceptance evidence by owner

| Step | Owner | Evidence that earns the next step |
|---|---|---|
| Agree native layout semantics | Compositor + Rust + zk reviewers | Concrete fixtures for multirow/solo edge gaps, both axes, constraints, merge/split/reorder and instance/revision rules |
| Publish exact layout state | Compositor, then Rust protocol/state | Initial and changed state, truthful unsupported modes, empty groups, source loss, and no false cross-family atomicity |
| Preview from reported topology | zk adapter + Taffy bridge | Live read-only capture, geometry meanings/budgets checked, native evidence distinguished from inferred shape |
| Add explicit-target sizing | Compositor, then Rust requests | Focus changes between observation/send never retarget; no focus/workspace/visibility changes; missing target rejects |
| Add conditional group restoration | Compositor, then Rust and zk | Concurrent edit rejects with no partial write; baseline row/edge state restored under declared conditions; own successful writes advance expected revisions |
| Lift generic composition | Kubeless, after consumer evidence | Fake-source example covers former/current impact, timer-only progress, queued gap then healthy current, unknown write and cancellation without replay |

For native control, also test constrained clients, pixel rounding, animation and
configure lag, output/workarea changes, window destruction mid-episode, late
replies, unknown outcome, and compositor restart. Success acknowledgments and
observed convergence must be asserted separately. No native mutation test should
be reported as completed on the basis of this requirements document.

Before cross-repository integration, record the compatible compositor and Rust
revisions plus discovered runtime capabilities. These repositories do not land
atomically. Old servers remain readable with explicit unsupported control.

## Decisions to make next

1. Choose the first layout-family scope and native revision/identity law.
2. Prefer conditional canonical sizing (Weave 2) or justify native restore tokens
   (Weave 3); retain observation-first as their shared foundation.
3. Specify targeted-write side effects, supported modes and truthful result codes.
4. Decide whether the native example/zk duplication warrants a typed state-feed
   helper now, or whether an example remains the better interface.

The separately ticketed inactive-window drift idea may later need native reorder
or move operations and axis-aware policy. It is not a requirement for this first
Longcat contract and does not justify expanding the present write API spec.

## Connections

- [Kubeless implementation handoff](../kubeless/handoff0.gpt6a.md): delivered
  state/publication work and its repository scope; this proposal is its new
  consumer-driven continuation, not a rewrite of that history.
- [Rust usage guide](../../README.md): existing subscriptions and captures that
  already make the read-only path possible.
- [Current consumer architecture](file:///home/rektide/src/zooming-kittens/ARCHITECTURE.md)
  and [runner contract](file:///home/rektide/src/zooming-kittens/app/zk/src/runner/README.md):
  implemented policy boundary and verification limits behind these requirements.
- [Earlier native investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md):
  detailed predecessor algebra and focus-race analysis. Its claim that no Rust
  client exists is historical; use the current Rust README for implementation.
