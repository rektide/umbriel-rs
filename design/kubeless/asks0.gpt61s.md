---
type: Assessment
title: Kubeless reuse and small lifts after Umbriel implementation
description: Source-grounded assessment of current Kubeless 0.1.0 delivery, capture and work mechanisms against Umbriel's implemented ownership boundaries.
resource: design/kubeless/asks0.gpt61s.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec73513c1ffeNKrefJ7ayTNQGo
    title: Inspected implemented Umbriel and current sibling Kubeless; separated existing reuse from small upstream asks and native policy
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent reviewed the proposed extraction boundary against implemented delivery and corrected symbol and locking details
  - resource: ../assessment/reuse0.gpt6a.md
    title: Historical reuse assessment, not authority for current package layout or capabilities
  - resource: ../../crates/umbriel-client/src/delivery.rs
    title: Independent bounded observation interval and terminal retention implementation
  - resource: ../../crates/umbriel-client/src/driver.rs
    title: Source owner, publication, admission permits, deadlines and cancelled request drainage
  - resource: ../../crates/umbriel-session/src/lib.rs
    title: Sans-I/O reply-slot and write-progress ownership
  - resource: ../../crates/umbriel-state/src/inventory.rs
    title: Encapsulated family membership, source order and evidence capture
  - resource: ../../crates/umbriel-state/src/active.rs
    title: Clock-injected stable-active identity reducer
  - resource: ../../crates/umbriel-arrow/src/lib.rs
    title: Checked native Arrow replacement envelopes and owned column buffers
  - resource: ../../crates/umbriel-polars/src/lib.rs
    title: Finite blocking lazy execution and derived output provenance
  - resource: file:///home/rektide/src/kubeless/Cargo.toml
    title: Current 0.1.0 workspace version and family members
  - resource: file:///home/rektide/src/kubeless/view/src/lib.rs
    title: View facade and view/cache split
  - resource: file:///home/rektide/src/kubeless/view/publication/src/blocking.rs
    title: Shared bounded publication implementation and backpressure policy
  - resource: file:///home/rektide/src/kubeless/observation/reflector/src/lib.rs
    title: Existing owner-coherent installed capture, cursor adjacency and pending publication
  - resource: file:///home/rektide/src/kubeless/work/custody/src/owner.rs
    title: Current attached-run custody kernel and unbounded retained history
  - resource: file:///home/rektide/src/kubeless/control/report/src/lib.rs
    title: Existing single-receiver backpressured advisory channel
  - resource: file:///home/rektide/src/kubeless/control/report-delivery/src/lib.rs
    title: Inline report transactions rather than observation delivery
  - resource: file:///home/rektide/src/tmux-interface-control/src/control_mode/delivery/cell.rs
    title: Independent item/byte delivery precedent and explicit producer-completion semantics
verification:
  method: Read concrete implementations, manifests, facade exports and test bodies; inspected jj revision and working-copy differences in each repository.
  limits: Source review only. No tests, benchmarks, live compositor probes or comparison-project builds were executed by this session. Proposed API sketches are not implemented or compile-checked.
---

# Kubeless reuse and small lifts after implementation

## Recommendation

**Keep the existing memory-view reuse. Propose one small delivery capability,
not a new controller or query platform.** A checked ordered-replacement builder
is a secondary convenience ask; the current native loop is perfectly defensible.
Keep source health, connection/reply ownership, focus policy and columnar schemas
in Umbriel. A pure stable-key dwell mechanism is a possible later lift, not an
immediate dependency recommendation.

There is no uncovered mandatory existing API migration in the inspected code.
Umbriel already uses the appropriate independently adoptable leaf. Richer current
Kubeless APIs do exist, but adopting them wholesale would add policies Umbriel
does not need and would not remove its essential native invariants.

These are review proposals, not implementation authorization, ticket creation,
automatic reconciliation, or evidence of measured performance gains.

## Evidence boundary and revision binding

| Repository | Inspected state |
| --- | --- |
| Umbriel | Implementation read while the parent advanced work from parent commit `b3ff0980` to working-copy revision `d1cdb9403efe3708e911e5320abf7030fdd17f4e`. Initial dirty paths were `.beads/issues.jsonl`, `Cargo.lock`, `README.md`, and `design/architecture/architecture1.gpt6a.md`; the final inspected diff contained the same paths. No claim that this was a frozen or clean whole-project baseline. |
| Kubeless | Working copy `7523451ec3eea927515b81c3d2b2c79957dcb64b`, empty over `9e7a1298`; `jj diff --stat` reported zero changes. Workspace packages are **0.1.0**, not the historical 4.2.0. |
| tmux-interface-control | Working copy `1ff71adc`, empty over `437544be`; `jj diff --stat` reported zero changes. Delivery implementation and relevant test bodies were inspected as a second-use precedent. |

Current files govern the recommendations. The [old assessment](../assessment/reuse0.gpt6a.md)
is historical reasoning; its package graph and API availability must not be
repeated as current facts. The user-approved sibling dependency is already
[`kubeless-view-memory = { path = "../../../kubeless/view/memory", version = "=0.1.0" }`](../../crates/umbriel-state/Cargo.toml).
It depends on a local sibling checkout; this assessment neither verifies an
available remote nor suggests replacing it with an old registry package.

## Existing APIs: use what exists, only where its contract fits

| Existing seam, checked in source | Recommendation for Umbriel |
| --- | --- |
| [`view/memory::memory_view`, `Writer::replace_affected`, `Reader::snapshot`](file:///home/rektide/src/kubeless/view/memory/src/lib.rs) | **Use today; already adopted.** Complete replacement and shared row handles are the right primitive. There is no need to request an atomic map replacement package. |
| [`view` facade](file:///home/rektide/src/kubeless/view/src/lib.rs), with lifecycle API owned by `view/cache` | Stay on the memory leaf. Historical cache readiness and mechanical writer closure are not Umbriel family freshness. Change import convenience only if another view-family capability is actually used. |
| [`observation-reflector::Writer::capture_installed`, `Installed`](file:///home/rektide/src/kubeless/observation/reflector/src/state.rs) | **Already exists; do not request generic “capture installed rows plus cursor” as new work.** Useful today for an adapter already consuming staged reflection facts. Not a drop-in for ordered complete-family frames; see below. |
| [`view-publication::blocking::bounded`](file:///home/rektide/src/kubeless/view/publication/src/blocking.rs) | Use for a deliberately backpressured downstream pipeline, not the socket drain path. Shared queue, item accounting only, and publication waits for responsible slow readers. |
| [`control-report::advisory::bounded`](file:///home/rektide/src/kubeless/control/report/src/lib.rs) | Already supplies a bounded single-receiver report channel. Use it for advisory work where awaiting capacity is intended. It does not supply independent lossy observation intervals, byte accounting or retained native terminals. |
| [`control-report-delivery::{SplitPlanCommit, ExclusiveSeries}`](file:///home/rektide/src/kubeless/control/report-delivery/src/lib.rs) | Report-series plan/transport/commit policies, not subscription delivery. `ExclusiveSeries` holds global exclusion across awaited transport; cancellation may leave remote effect without local commit. Do not use it to own wire correlation. |
| [`work::Executor`, `Capacity`, `ReadyWork`](file:///home/rektide/src/kubeless/work/execute/src/lib.rs) | Existing tools for a separately authorized application work pipeline. No new keyed executor ask is needed. They do not own task progress or source custody and do not bound the producer's queue. |
| [`observation-order::OrderDeclaration`, `EpochScope`](file:///home/rektide/src/kubeless/observation/order/src/lib.rs) | Use if an external consumer needs qualified comparison witnesses. Current `Inventory::install` needs only explicit monotonic local checks; wrapping every `Stamp` would add ceremony without strengthening compositor evidence. |

The facade families are convenience surfaces, not a requirement to import
`kubeless-all`. [`work/src/lib.rs`](file:///home/rektide/src/kubeless/work/src/lib.rs)
and [`control/src/lib.rs`](file:///home/rektide/src/kubeless/control/src/lib.rs)
explicitly expose independent leaves and feature-selected assemblies. Crate names
alone were not treated as capability evidence.

## Priority 1: independent bounded observation delivery

### Current friction and existing precedent

[`delivery::Producer::{push, finish}`, `Observer::{recv, try_recv}` and the producer-loss path](../../crates/umbriel-client/src/delivery.rs)
implement a small Tokio channel plus shared byte/terminal state. Each observer
has its own item and logical byte budget. The driver's
[`Driver::publish`](../../crates/umbriel-client/src/driver.rs) synchronously admits
to each interval, retiring a full interval without waiting for it. Queued items
drain before the first retained error; abrupt local producer loss yields
`DriverLost`, not invented transport evidence. An observer cannot prolong socket
ownership.

Current Kubeless publication is a different policy, not a missing option hidden
behind a facade. `blocking::Publisher::publish` awaits shared space;
`tests/blocking.rs::full_shared_queue_blocks_until_the_oldest_publication_advances`
and `dropping_a_slow_subscriber_releases_its_unread_responsibility` exercise that
law. The advisory report channel also waits at capacity, drains to ordinary EOF
after last reporter drop, and has no byte budget or separate terminal reason.

The credible second consumer already exists:
[`tmux DeliveryProducer<T,E>::push(item, bytes)`](file:///home/rektide/src/tmux-interface-control/src/control_mode/delivery/cell.rs)
has item/byte bounds, checked arithmetic and first-terminal drain. Its
`item_and_byte_limits_leave_accounting_unchanged`, `checked_byte_overflow_is_a_byte_limit`,
and `first_terminal_wins_and_items_drain_first` test bodies show the same mechanism.
It is private, includes sync/async consumer claims and receipts, and deliberately
does **not** synthesize a terminal on producer drop. Do not copy that entire
threading framework or silently change its owner-loss contract.

### Smallest seam and caller simplification

Propose an explicitly named independent-delivery capability under the
**view/publication concern**, alongside, not replacing, `blocking`. A directly
usable child may be appropriate if it has no view dependencies; avoid creating
another unrelated report/work package simply because it transports values.

The first API can be one sole-producer/single-receiver cell:

```text
channel<T, E>(limits, owner_loss_policy) -> (Producer<T, E>, Receiver<T, E>)
Producer::try_push(item, logical_bytes) -> Result<(), ReturnedPush<T>>
Producer::finish(first_terminal)
Receiver::try_recv() -> Item | EmptyNow | Terminal(E)
Receiver::recv() -> Item | Terminal(E)
```

`ReturnedPush` distinguishes item-full, byte-full/overflow, closed and terminal
without losing rejected input. Account the admitted weight with the queued item
and release exactly that weight on receive; do not recompute mutable payload
weights. Retain one terminal outside queue capacity. Reject further admission
after terminal and expose it repeatedly after drain. Construction validates
capacities. Wakers must not be invoked under the accounting lock.

That last property is a proposed extraction improvement, not a claim about the
current wrapper: Umbriel calls Tokio's `try_send` while holding its accounting
lock, then detects unexpected producer disappearance on the receiving side.
A generalized cell should reserve weight before publication, release the lock
before waking, and roll back reservation on refusal. Test immediate/reentrant
wake behavior rather than assuming every executor behaves like Tokio's scheduler.

The caller supplies logical weights: this is not allocator measurement or unique
resident-memory accounting. Shared frame storage can be conservatively charged
per interval, as Umbriel does. Terminal diagnostic storage also needs a stated
separate bound/policy; an out-of-band terminal is not inherently byte-bounded.

The simplest Umbriel profile installs caller-provided local-loss evidence on
sole producer drop. Tmux's explicit-completion profile should remain distinct;
supporting that profile does not license accidental indefinite waits in Umbriel.
Runtime-independent polling is attractive given tmux, but a Tokio-only profile
is a smaller first implementation if both consumers do not yet agree on a core.

Umbriel then deletes the accounting/terminal channel mechanics while retaining
`Observation`, `DeliveryEnd`, `retained_bytes` policy, registration limits and the
small fan-out loop. `Driver::publish` still chooses `Lagged` versus source-end
evidence. No generic cell should know about epochs, server replies or reconnect.
Start with the cell, not a feature-rich fan-out manager; receiver registration
and source ownership are already easy to understand locally.

### Acceptance evidence and cost

Required tests: exact item/byte boundaries, arithmetic overflow, unchanged
accounting after refusal, concurrent admission/receive, receive cancellation,
terminal on a full queue, first terminal across explicit finish/drop, repeated
terminal after drain, dropped receiver, and distinct producer-loss profiles.
Retain Umbriel's scripted peer test
[`two_observers_one_stalled_does_not_block_commands_or_other_observer`](../../crates/umbriel-client/tests/subscription.rs)
as integration evidence, plus its local-loss and byte-release delivery tests.
Tmux must demonstrate its explicit-completion behavior remains unchanged before
claiming a shared extraction succeeded.

Costs are synchronization, queue bookkeeping, a new contract to maintain and
possibly runtime adapters. Reusing it is worthwhile only if it remains smaller
than the native wrapper. There is no measured throughput or allocation claim
here. Until an upstream API is agreed and tested, keep Umbriel's current small
mechanism rather than forcing blocking publication onto it.

## Priority 2, optional: checked unique replacement preparation

[`inventory::Relation::replace`](../../crates/umbriel-state/src/inventory.rs)
builds `Replacement`, records exact input key order, and rejects duplicate IDs
before changing installed membership. Kubeless `Replacement::upsert` intentionally
uses last-write-wins; [`view/memory/tests/replacement.rs`](file:///home/rektide/src/kubeless/view/memory/tests/replacement.rs)
tests that policy. Changing it globally would be wrong.

A small opt-in builder could return `(Replacement<K,V>, Vec<K>)` from a finite
ordered `Vec<(K,V)>`, rejecting duplicates and returning the **entire original
input** on refusal. A less opinionated first extension is just
`Replacement::try_from_unique`; callers retain source order themselves. Choose
one only after a second adapter establishes which return shape is useful.
The owning concern is **view/memory** for replacement construction; reuse the
existing [`view/change::Duplicate`, `DuplicateKey`](file:///home/rektide/src/kubeless/view/change/src/unique.rs)
refusal vocabulary rather than introducing a second duplicate-error taxonomy.
Its current duplicate detection helpers are private, not a public builder we
can simply call today.

This removes a repeated preparation loop, not Umbriel validation policy. Empty
IDs, invalid-family health, installed stamp versus last attempt, and scratchpad
joins stay native. An ordered return must preserve input order even when lexical
key order differs. Empty replacement is valid and carries caller-owned metadata
despite having no entries.

The second-use candidate is Kubeless's own
[`observation-reflection::Reflection::apply_retaining`](file:///home/rektide/src/kubeless/observation/reflection/src/state.rs):
its staged candidate currently reports duplicate replacement with `replaced`
and permits last-write-wins. A separately selected strict acquisition adapter
could use the checked constructor without changing the existing reflection
contract. This is a plausible extension of a concrete consumer, **not evidence
that the current reflector requires strict duplicates**.

Tests should preserve non-Clone input and the rejected suffix, first/repeated
indices, unchanged installed generation after refusal, successful empty commit,
and source order distinct from map order. Costs include retained finite input,
duplicate-detection storage, key clones for an optional order vector and another
API. Savings are modest. If no second consumer selects strict preparation,
**keep the approximately eighteen-line native replacement loop**.

## Capture: an existing capability, but keep the ordered native owner

Current [`Writer::capture_installed`](file:///home/rektide/src/kubeless/observation/reflector/src/lib.rs)
captures an anchored map, committed cursor, separate progress, readiness,
candidate summary and publication backlog during one owner borrow.
`capture()` additionally refuses nonquiescent export. The inspected
[`tests/installed.rs`](file:///home/rektide/src/kubeless/observation/reflector/tests/installed.rs)
shows candidate rows excluded, progress distinct from commit cursor, and a
borrowed driver dropped/resumed without replacing its owner. This is substantial
existing capability, not an unimplemented generic-owner proposal.

It does not preserve source array order or Umbriel's family health/last-attempt
policy. `State` says entries have unspecified ordering and duplicates install
last-write-wins. Adapting each already-complete Umbriel frame into staged
begin/items/complete events adds work while still requiring a native wrapper.
If a future source actually has staged acquisitions, use reflection and its
capture API instead of rebuilding them. That future need is not present here.

Umbriel's private `Relation` reader/writer cannot escape; `Inventory::snapshot`
reads under one serialized owner borrow. That is why membership, order and
status remain coherent despite the underlying memory view having its own lock.
An outer application lock can share the owner; exporting raw reader handles
would lose this guarantee. Existing tests in
[`observations.rs`](../../crates/umbriel-state/tests/observations.rs) exercise
duplicate refusal, rejected-attempt versus installed stamps, empty ready
replacement, retained prior captures and cross-family skew.

No generic metadata transaction API is justified merely to avoid this wrapper.
Do not store a synthetic metadata row among domain rows or put a stamp solely on
individual rows: zero-row replacement still needs evidence. A local coherent
capture of windows and workspaces carries both stamps, not a source-atomic
compositor transaction. Connection epoch is local lineage, not a resume cursor;
local generation/anchor is a memory-view fence, not a server CAS condition.

## Work custody: preserve the wire slot, do not transplant a controller

[`Session::begin_write`, `write_progress`, `stop_waiting`, `receive`](../../crates/umbriel-session/src/lib.rs)
own a protocol obligation independent of the caller. Cancellation withdraws
only provably unsent work. Otherwise the partial request must finish writing and
its reply must drain, or the connection closes. The driver retains its admission
permit in `Pending` until that boundary, so capacity includes ingress, queue and
active/draining work. Ordinary deadlines include queue time; cancellation has a
separate bounded drain deadline. No possibly executed command is replayed.

The current [`work-custody::Owner`](file:///home/rektide/src/kubeless/work/custody/src/owner.rs)
is a real retained-duty/offer/run kernel; it can preserve return/accounting and
acceptance across disposable driving. Its core explicitly has no clock or task,
and memory/cardinality are unbounded until deliberate checked history transfer.
[`inspection`](file:///home/rektide/src/kubeless/work/custody/src/inspection.rs)
is a convenience over that same kernel, not another mandatory ledger. These are
valuable for maintained derivation duties, but do not own a socket, partial
write, receive owner or reply-correlation retirement. They do not replace our
small slot or provide our admission bound/deadline automatically.

[`work-execute::Executor`](file:///home/rektide/src/kubeless/work/execute/src/lib.rs)
and [`control-driver`](file:///home/rektide/src/kubeless/control/driver/src/lib.rs)
progress only while their output streams are polled; dropping them drops active
futures without rollback or synthetic receipts. Placing wire ownership inside
such a disposable operation future would weaken the driver guarantee. An outer
application executor can call `Subscription::request`, because the separately
spawned source owner still drains a cancelled call; the exclusion key for that
wire lane is the connection, not window ID.

No request-ledger lift is recommended now. Its supposedly generic flags would
still encode Umbriel framing and retirement policy. The scripted
[`cancellation_late_reply_cannot_complete_later_command`](../../crates/umbriel-client/tests/subscription.rs)
test and cancellation-drain deadline test are the relevant proof obligations,
not controller completion or report-series success. Caller cancellation is not
execution reversal; acknowledged native success is not observed convergence.

## Dwell: possible pure lift, not snapshot debounce

[`StableActiveReducer`](../../crates/umbriel-state/src/active.rs) compares only
active identity for continuity, refreshes metadata without moving its deadline,
emits once per episode, and accepts injected nondecreasing time. `None` is a
valid no-active episode; explicit loss invalidates continuity instead. The
[`title_churn_and_timer_only_progress_emit_latest_once`, `a_b_a_and_repeat_episodes`,
`none_cancels_immediately_and_gaps_restart_dwell`](../../crates/umbriel-state/tests/observations.rs)
test bodies establish those distinctions.

Current [`work-schedule::ScheduleOptions`](file:///home/rektide/src/kubeless/work/schedule/src/lib.rs)
describes trailing-edge debounce/coalescing with optional postponement cap, uses
Tokio `DelayQueue`, and has unbounded cardinality. A cap is not identity dwell.
[`observation-idle::next_or_idle`](file:///home/rektide/src/kubeless/observation/idle/src/lib.rs)
creates a fresh raw-read timeout; repeated raw input resets idleness, the opposite
of an unchanged-identity dwell deadline. Neither should replace this reducer.

If a second consumer needs it, the smallest lift is a clock-free
`StableKey<K>` episode core with `observe(key, now)`, `invalidate`, `deadline`
and one-shot `poll(now)` evidence. Keep payload refresh and epoch/loss validation
outside. A real absent key can be represented by `K = Option<Id>`; unknown input
is invalidation, not `None`. Own it in the **observation family**, not scheduling:
it judges observed continuity rather than creating work obligations.

A credible prospective second use is a stable presence/identity consumer over
the existing strict [`observation-singleton::Presence`](file:///home/rektide/src/kubeless/observation/singleton/src/lib.rs)
stream. That stream already distinguishes complete absence from source failure
and incomplete acquisition. No inspected consumer currently needs this dwell
policy, so this is a deferred lift, not an established cross-project demand.
Evidence needed includes equal-key refresh, A/B/A, absent episodes, timer-only
emission, invalidation at a deadline, overflow/backwards time and reset after
emission. The costs are adapter state and split responsibility for payload/time;
leave the compact native reducer intact until that division actually simplifies
two callers.

Latest-state coalescing cannot establish every identity episode: it may hide
A/B/A completely. The active example must process queued observations before a
simultaneous timer decision. A generic core must not claim that a socket-draining
queue captures every physical compositor transition; it only handles delivered
observations, with explicit loss invalidating its evidence.

## Polars/Arrow: native finite boundary, not a Kubeless subsystem

[`umbriel-arrow::SnapshotBatch`, `SnapshotMetadata`, `validate_batch`](../../crates/umbriel-arrow/src/lib.rs)
already keep replacement metadata outside rows, including empty relations, and
check field/array dtype, height and nullability before exposing the envelope.
The arrays are owned Polars-native Arrow, not Apache Arrow types or a C Data
Interface bridge. [`umbriel-polars`](../../crates/umbriel-polars/Cargo.toml)
selects Rust Polars **0.55.2** with `lazy`, not the archived 0.55.1 assessed earlier.

[`SnapshotQuery::collect`, `QueryResult::kind`](../../crates/umbriel-polars/src/lib.rs)
perform explicit finite blocking collection/rechunking and label output
`Derived`. Input provenance survives; it does not turn filtered output into a
complete family replacement. Caller-added query inputs need caller-added
provenance. The emitter is independent of inventory and the query engine.

[`native_query::consume_snapshots`, `query_off_thread`](../../crates/umbriel-polars/examples/native_query.rs)
await one blocking worker at a time, consume each output through a sink and
retain no result history. Source I/O runs separately; slow queries can lag their
bounded observer rather than blocking the socket driver. Cancellation of a
started blocking job should not be described as stopping its CPU work.

These are scheduling/retention bounds, **not a hard bound on query-engine output
bytes or temporary allocations**. Arbitrary native lazy transforms can amplify
output, and `collect` returns one rechunked owned batch. No output-byte limit is
exposed in this library. A concrete application needing bounded query memory
must restrict plans or supply an explicit output/execution policy; do not claim
the current observer byte budget bounds Polars computation.

Existing `work::Executor` could gate a future multi-query application pipeline,
but adds nothing to the current sequential example. Do not move Umbriel schemas,
Polars, batch validation or the query engine into Kubeless just for reuse. There
is no verified second consumer to justify that dependency expansion. A generic
provenance pair alone saves essentially no code and should remain ordinary Rust.

## Integration notes

The parent-owned README and architecture can link here for the upstream asks,
while continuing to describe the native wire/session, state and columnar
boundaries as implemented. This document deliberately does not modify those
homes or the historical assessment. Its source links identify concrete current
mechanisms; its revision table bounds their evidence.

The actionable sequence is: retain current memory-leaf reuse; review the small
independent-delivery cell with Kubeless and tmux owners; consider checked
replacement preparation only with a second strict consumer; leave the rest
native unless a commissioned caller creates a real reuse need.
