---
type: Design
title: Sonic-first architecture for the whole Umbriel protocol
description: Revised library-wide design selecting Sonic and separating wire framing, typed commands and models, session state, transport, and optional observations.
resource: design/architecture/architecture1.gpt6a.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Human-selected Sonic and Polars-native direction, protocol and reuse reviews, and implementation integration
  - resource: architecture0.gpt6a.md
    title: Predecessor sans-I/O boundaries and the focused active-window example
  - resource: ../performance/json0.gpt6a.md
    title: Source-checked Sonic capabilities and buffer ownership tradeoffs
  - resource: https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09
    title: Shared Rust JSON Protocol Speed answer, read in full
  - resource: ../protocol/commands0.glm53m.md
    title: Returned protocol catalog and sequential subscriber-reuse evidence
  - resource: ../protocol/review0.gpt6a.md
    title: Source-reviewed size accounting, empty subscription response, and dedup qualifications
  - resource: ../assessment/reuse0.gpt6a.md
    title: Astra-medium assessment of tmux ownership, Kubeless views, and verified columnar API boundaries
  - resource: file:///home/rektide/src/kubeless/view/memory/src/lib.rs
    title: Parent checked atomic replacement, opaque local generation, shared row handles, and coherent snapshot implementation
  - resource: file:///home/rektide/src/tmux-interface-control/doc/library/ordering.md
    title: Parent checked cancellation and correlation-retirement ownership laws
---

# Sonic-first architecture for the whole Umbriel protocol

**Predecessor:** [architecture2](architecture2.gpt6a.md) now describes the
implemented system and its verified ownership/data paths. This document preserves
the decisions and design development that led to it.

## Direction selected by the user

**Use Sonic. Build a general Umbriel protocol library with a sans-I/O core.**
The user selected this direction after the first architecture draft: pursue
Sonic's apparent advantages directly and keep the whole protocol in view.

This supersedes the predecessor's backend-selection gate and example-led
implementation order. The [parser research](../performance/json0.gpt6a.md)
remains useful evidence about lifetimes and allocation, and simd-json remains a
possible future tool. A comparative parser project is not a prerequisite for
library development. Measurements should answer concrete implementation
questions as they arise.

The 750 ms active-window stream is one integration example among many. It
should fit the resulting library naturally, alongside a command client, a
workspace observer, a shell, and a geometry controller.

## The library's jobs

| Layer | Responsibility | Principal consumer |
|---|---|---|
| Codec | Bounded NDJSON framing, checked Sonic decode, reusable encoding | Any transport implementation |
| Protocol | Wire envelopes, event names, typed queries, action encoding, full data models | Commands, shells, controllers |
| Session | Role/lifecycle, request completion, subscription initialization, EOF interpretation | Runtime adapters and custom event loops |
| Transport | Socket discovery, actual I/O, scheduling, explicit connection policy | Ordinary Rust applications |
| Observe | Optional reductions over protocol snapshots | Active window, inventory, convergence helpers |

Use a **multi-crate Cargo workspace**, as subsequently requested by the user.
The core may use `std` and allocation without owning file descriptors, an
executor, a wall clock, or a timer. That is the useful sans-I/O boundary here.

### Proposed crate boundaries

The protocol, session, client, state, Arrow-emitter, and Polars-query crates are implemented;
the [root implementation inventory](../../README.md#implemented-crates) links
their current APIs and examples. Action families remain ordinary modules inside
the protocol crate; clock-free dwell and inventories share the state crate.
Small helpers and examples do not acquire separate packages.

| Crate | Owns | Dependencies flow toward |
|---|---|---|
| `umbriel-protocol` | Framing, Sonic wire envelopes, query/action grammar, payload models | Sonic, Serde, small codec utilities |
| `umbriel-session` | Sans-I/O role state, one reply slot, write progress, routing and terminal effects | Protocol |
| `umbriel-client` | Unix discovery/I/O, request and subscription drivers, independent bounded delivery, runtime integration | Protocol + session |
| `umbriel-state` | Optional family snapshots, identities, continuity/readiness, consumer-derived views | Protocol/session metadata + a Kubeless view adapter |
| `umbriel-arrow` | Optional column schemas and owned record-batch emission, direct from snapshots | Protocol + **polars-arrow (standalone, no query engine)** |
| `umbriel-polars` | Optional finite lazy-query execution over columnar snapshots | Arrow emitter + matching Polars facade |

These are responsibility boundaries, not a requirement to create empty crates
before each has code. The separate Polars package keeps the query engine out of
the simpler batch emitter. Neither Polars nor an executor belongs in
`umbriel-protocol`.
The initial codec/envelope implementation can land together in the protocol
crate while the assessment informs state/driver boundaries.

The commissioned Astra-medium assessment read **tmux-interface-control first,
kubeless second** after the protocol/design baseline was committed. Its
[returned assessment](../assessment/reuse0.gpt6a.md) motivated the explicit
session crate and direct columnar path above. The parent read the complete
assessment and checked consequential source seams before incorporating them.

### Reuse decisions guiding the next slices

- Use tmux's **ownership laws**: a sent request retains reply ownership after
  waiter cancellation until the reply drains or the connection closes. Transfer
  its bounded independent-observer contract. Umbriel's own single-slot session
  remains much smaller than tmux's fence-based correlation engine.
- Evaluate **`kubeless-view-memory`** through a family-snapshot adapter for
  materialized inventories. It already supplies atomic whole-map replacement
  and coherent reads without Kubernetes/resource traits or an async runtime.
  `kubeless-view` is an option when sole-writer closure/readiness is useful.
  These are concrete dependency candidates pending an integration proof.
- Keep **one replacement map per complete family scope**. Installing windows
  into a shared all-family map would incorrectly erase workspaces. Preserve
  separate family metadata and represent a known empty snapshot explicitly.
- Preserve **independent bounded delivery** on the receive path. Kubeless's
  blocking publication is useful for deliberately backpressured downstream
  work, but awaiting it inline could stall all socket input and command replies.
- Keep direct **snapshot-to-columns** available without an inventory store.
  Polars-native batches are the **decided representation** (user acceptance
  2026-10-13; see the [columnar assessment](../columnar/arrow-polars0.glm53m.md)
  and the decision rules in the columnar section below).

Kubeless's extra packages are currently unpublished. On 2026-10-14 the user
selected a **sibling path dependency** for development: the state adapter uses
the narrow view leaves from `../kubeless`. That checkout has since moved to
version `0.1.0` and newer facade organization; its current source governs the
adapter rather than the older assessment's `4.2.0` package inventory. Reuse
carries real costs: owned keys, shared row handles, locks,
replacement work, and optional retained before/after evidence. A direct borrowed
consumer should pay none of those materializer costs.

## Typed convenience and borrowed control

Expose two complementary levels rather than requiring every consumer to use a
fully owned event enum:

1. **Borrowed wire messages.** A decoded envelope borrows the frame and retains
   its payload as Sonic `LazyValue`. The caller inspects the kind, ignores it,
   parses a full model, or parses a custom projection. Complete envelope
   validation happens before it is exposed.
2. **Typed protocol operations.** Query descriptors associate a request with its
   response type. Typed events decode their matching model; typed actions
   validate and encode argument grammar. These are convenience operations over
   the same underlying wire contract.

This permits both:

```rust,ignore
// API sketches, not promises that these methods already exist.
let windows = client.query(Windows).await?; // complete owned Vec<Window>

// Custom transport, synchronous frame scope, selected borrowed fields.
match Message::decode(frame)? {
    Message::Event { name: EventName::Windows, data } => {
        let selected: MyWindowsProjection<'_> = data.deserialize()?;
        consumer.observe(selected);
    }
    other => route(other),
}
```

No backend plugin trait, object tree, or cross-task handoff is required on this
path. Sonic types may be visible in the raw API because Sonic is a deliberate
dependency; ergonomic typed APIs should speak in Umbriel terms.

### Envelope details worth getting right once

- Recognize `ok`, `err`, and event envelopes independently of JSON key order.
  Never assume `event` is first: `data` can precede it.
- Preserve **present `ok: null`** as successful unit output. A plain
  `Option<LazyValue>` using normal Serde null handling can erase the distinction
  between a missing `ok` field and successful null; use explicit presence
  tracking in the envelope decoder.
- Distinguish an unknown event name from a malformed envelope. Retain its raw
  data so applications can inspect protocol additions.
- Reject ambiguous envelope shapes and duplicate significant keys rather than
  routing them differently depending on backend or field order.
- Keep command failure, JSON/framing failure, unexpected response, and transport
  failure as distinct error layers. Preserve the server's actual error text.
- Do not partially publish payload-derived state if later decoding fails.

The [command investigation](../protocol/commands0.glm53m.md) supplies the exact
supported shapes. Read it with the [parent source review](../protocol/review0.gpt6a.md):
a keyboard-only subscription without a keyboard can produce a bare LF instead
of JSON. The session layer must handle this empty initial response without
completing an ordinary command. The implementation should use these findings
rather than extrapolating from a single `windows` example.

## Queries and data models

### Queries

Model each top-level query's request arguments and result. A query descriptor
can implement a trait with an associated owned response type for easy async
usage. The raw envelope API remains available for custom borrowed response
projections, avoiding a complicated lifetime-generic query trait until needed.

Cover the actual command surface: windows, workspaces, submap, layers, color,
tearing, keyboard layouts, headless output operations, and action dispatch.
Read-only queries and mutating output/action requests should be distinguishable
in types and docs, while sharing request serialization.

Requests are encoded directly into caller-owned byte storage. Fixed requests
may use static bytes; dynamic requests go through Sonic's serializer. A request
builder should not produce an intermediate `Value` or require concatenating
unescaped user input into JSON text.

### Models

Supply complete models of the inspected protocol, including all event-family
payloads and query-only results. Preserve meaning rather than merely naming
fields:

- IDs are opaque window/workspace string newtypes, not assumed numeric handles.
- `active` and `focused` remain distinct.
- Negative coordinates and sentinel PIDs must not be coerced to unsigned types.
- Geometry fields retain the distinction between target placement and committed
  client size documented in the prior research.
- Nullable fields are represented explicitly; absent/added fields are handled
  according to the actual wire evidence.
- Families with a related query do not automatically have identical wrappers.
  Share payload models only where the source confirms equivalence.

Owned models are convenient for application storage and normal streams. Borrowed
custom Serde projections are the narrow fast path. Do not introduce an owned
and borrowed copy of every protocol struct before actual consumers require
that maintenance burden. A reusable full-inventory reducer is optional; the
protocol layer itself does not need a global registry.

## Actions: the second grammar

Umbriel actions travel inside the JSON `msg` request but have their own text
grammar. Correct JSON escaping does not guarantee a correctly formed action.
Design the action API around the upstream parser's argument categories and
targeting rules, using the full catalog from the command research.

Useful structure:

- Typed names for no-argument actions.
- Validated direction, fraction, delta, workspace/output target, and optional
  window-ID argument types where the grammar actually uses them.
- Explicit variants for free-form command/text arguments.
- A raw-action escape hatch for protocol additions and uncommon commands.

Validate syntax and representable ranges locally. Do not pretend to validate
runtime state such as existence of a window, availability of a layout, or
whether an action will visibly do anything. Preserve upstream no-op versus
error behavior. In particular, a typed target option must not imply that every
window action can address an arbitrary window ID.

Use one central action catalog for names and argument categories. A table/macro
can generate repetitive enums/encoding internally if it remains easy to audit;
code generation from an invented external schema is not necessary. Every
supported name should have a source-backed entry, and catalog completeness
should be mechanically checkable against the pinned upstream table.

## Session roles and command outcomes

Represent roles explicitly:

```text
OneShot:       New -> Sending -> AwaitingReply -> Complete -> Closed
Subscription: New -> SendingSubscribe -> Initializing -> Streaming -> Closed
```

Errors and EOF can terminate either path. Initial event snapshots are useful
data, not disposable handshake traffic. Track which requested families have
arrived. A keyboard family may have no initial state, and there is no universal
initialization-complete marker, so expose per-family readiness rather than an
unconditional wait for every requested family. Initialization does not imply an
atomic multi-family snapshot or a shared revision number.

Ordinary command sockets are one-shot. Source and a read-only live probe confirm
that accepted subscriptions can carry further sequential commands and replies.
A persistent single-owner subscription/command driver is therefore a concrete
candidate, not merely an upstream wish. Its buffering, interleaving, initialization,
and error handling need a specified implementation; ordinary one-shot requests
remain useful independently.

With no wire correlation ID, allow at most one outstanding request on a reusable
subscribed command channel. Do not count event or blank initial frames as replies.
Multiple independent one-shot requests can use separate connections if the
application wants concurrency; bound that concurrency at the adapter layer.

Distinguish the caller's wait from the outstanding wire slot. If a caller cancels
or times out after any possible write, drain that command's eventual reply or
retire the connection before admitting the next command. Releasing the slot
when a future is dropped can assign its late reply to the wrong caller. An
unsent queued request can be removed without this drain obligation.

Expose command outcome uncertainty. If transport fails after sending an action,
the library may not know whether it executed. Cancellation of a waiting Rust
future does not undo an action. Automatic action retries would require an
application decision about idempotence, so keep retries explicit. A successful
`msg` reply acknowledges the protocol action, not completion of later rendering
or a client resize.

## Memory and scheduling contract

Use the predecessor's reusable contiguous buffer and scan cursor design:
consume complete LF-delimited frames, do not rescan an incomplete prefix on
every read, and compact only when necessary. Frame and buffered-input bounds
are separate, configurable client policies. The upstream request limit is not
a universal inbound-response limit.

Sonic reads an immutable frame slice. Borrowed fields stay within that frame's
synchronous scope. Consumers promote only data that must survive reuse. A
normal async stream yields owned records or deliberately retained raw frames;
retained raw slices can pin larger buffers and should be a conscious choice.

The receive owner performs framing, parsing, and immediate handling together.
It should drain available input in bounded work turns and resume promptly when
there is more. Request writes retain offsets across partial writes. Neither the
core nor a socket adapter should require one task per message.

For subscriptions, choose output semantics explicitly:

- A caller-driven connection exposes full control and requires continuous
  driving by the caller.
- A convenience driver can keep draining while a consumer waits, with bounded
  delivery storage.
- A latest-state interface may overwrite older states where that is the stated
  contract.
- An every-message interface must signal lag/overflow rather than silently
  collapse messages.

Raw snapshot coalescing is not universally correct: consumers that measure
continuity or react to transitions need to see each delivered transition. Keep
coalescing in a consumer-aware layer. Initial/repeated-subscribe responses can
duplicate broadcast state; broadcast dedup does not guarantee unique messages
per subscriber.

## Observations and examples

Build observations on the general event/projection APIs. Candidate examples:

1. List windows/workspaces with typed queries.
2. Execute a typed action and distinguish remote failure from transport failure.
3. Subscribe to multiple families and maintain a full latest inventory.
4. Report the active window after 750 ms of observed stable identity.
5. Run a controller that uses geometry snapshots and command acknowledgments
   without confusing acknowledgment with geometry convergence.

The [predecessor's dwell section](architecture0.gpt6a.md#stable-active-window)
preserves the detailed 750 ms semantics and traces. Keep that helper small and
runtime-free, with injected monotonic time. It must not dictate the protocol's
public model, transport topology, or internal storage for unrelated consumers.

## Materialized state and columnar output

Here **materializing** can mean two different things:

1. Maintaining a current, queryable view from incoming full snapshots. A windows
   update replaces the windows relation; missing IDs disappear from that view.
   This is observational state maintenance, not an instruction to close windows
   or reconcile the compositor toward a desired configuration.
2. Executing a lazy query and producing concrete column buffers/Arrow batches.
   A lazy plan describes computation; it is not itself a continuously updating
   subscription or an emitted record batch.

Keep an inexpensive direct path for consumers that do not need materialization:
borrowed event → projection → application. A materializer should be an optional
subscriber, not compulsory storage through which every message passes.

The columnar representation is **decided: Polars-native** (user acceptance,
2026-10-13). `umbriel-arrow` emits `polars_arrow::RecordBatch` — and depends
only on the standalone `polars-arrow` crate, not the query engine — while
`umbriel-polars` adds the facade. For our schema-matched primitive and Utf8View
columns, `From<RecordBatch> for DataFrame` can retain the existing data buffers.
Evidence and the historical comparison live in
[the columnar assessment](../columnar/arrow-polars0.glm53m.md). Its pointer and
value probes establish particular buffer-sharing/conversion properties; they
**do not measure equal runtime performance or zero runtime overhead**. We select
Polars-native representation for the direct data path and simpler maintenance.
The user's 2026-10-14 direction reaffirms it. Governing rules:

- **Utf8View for every string column, always.** Polars' DataFrame conversion
  moves view strings untouched but fully copies `Utf8`/`LargeUtf8` columns.
- **No C Data Interface bridge.** The user prefers no FFI bridging; if an
  Apache-arrow consumer ever appears, Arrow **IPC serialization** (pure safe
  Rust on both sides) is the sanctioned interop path, accepting its copy cost.
- **One resolved Polars type universe**: use matching pinned versions of
  `polars-arrow` and the `polars` facade and verify dependency unification.
  The implementation starts from the already-probed published **0.55.2**.
- **Schema/array agreement**: validate types as well as field count and height
  when constructing our batch envelope. Polars's `RecordBatch::try_new` checks
  counts and lengths, while `DataFrame::from` assumes matching types. Keep raw
  arbitrary-batch import distinct from the schema-bound emitter path.
- **Query results preserve source provenance** without pretending a filtered
  or aggregated result is the compositor's complete original family. Empty
  source replacements and empty query results both need their own envelope.

Polars's `collect_batches` returns a **blocking iterator of DataFrames**, even
though the API is behind an `async` feature. A dedicated query execution
boundary should own that work; the socket reader must remain free to drain.

Candidate flow:

```text
full family snapshot
  -> selected typed fields / column builders
  -> owned, schema-bound batch (or a current-state table)
  -> Polars lazy query plan
  -> explicit execution and output batches
```

A snapshot-per-batch design can avoid a per-row object inventory entirely. It
must still represent an **empty** snapshot as replacement, not silence. A
long-lived materialized table needs explicit replacement/deletion semantics;
merely appending every windows array would turn current state into duplicated
history. Historical append streams are useful too, but must carry snapshot
identity and their different contract.

Track family, connection epoch, local observation sequence/time, and schema
version at appropriate batch metadata/envelope boundaries. None of those local
coordinates is a server transaction ID. Cross-family joins can combine states
observed at different moments; initialization of all families does not make
them atomic. Columnar buffers must own their contents beyond receive-buffer
reuse, even when the initial Sonic parse borrows.

Batch by actual snapshots or an explicit rows/bytes/latency policy. Keep this
policy separate from the 750 ms stable-focus observation. Bound builder/output
storage and define what lag means. The
[completed assessment](../assessment/reuse0.gpt6a.md#arrow-and-polars-feasible-with-a-precise-rust-boundary)
identifies both direct and materialized input paths. An external snapshot-batch
envelope carries family, epoch, receive position, and complete-replacement
meaning even for zero rows. Chunked snapshots need an explicit final-chunk
boundary before replacing membership.

## Build order and verification

1. **Codec foundation:** reusable bounded framing, Sonic typed decoding, direct
   encoding, useful borrowing, and malformed/truncated frame errors.
2. **Whole wire surface:** robust envelopes, query descriptors, all current
   payload models, event families, and action argument categories/catalog.
3. **Session and transport:** explicit role state, initial subscription tracking,
   correct EOF/reply handling, a Unix adapter, and runtime conveniences that
   preserve the core's independence.
4. **Examples and useful observations:** demonstrate both command and event
   usage, with the stable-active-window stream as one case.

Each slice should be usable and checked. Tests should concentrate on protocol
properties: arbitrary frame chunking, bounds, escaping, key ordering, null
success, errors, all event families, action grammar, initial snapshots, reply
accounting, and partial writes. Synthetic peers make transport behavior
deterministic without mutating the user's compositor.

Profile when a complete path exists. Start by counting allocations and copies
on realistic snapshots, separating initial growth from warm steady state.
Measure Sonic's direct versus lazy paths if routing introduces a measurable
second scan. Revisit backends only if evidence presents a worthwhile problem.

## Cross-references

- [Tmux/Kubeless assessment](../assessment/reuse0.gpt6a.md): concrete reuse seams,
  session ownership laws, per-family materialization, and checked columnar APIs.
- [Command research](../protocol/commands0.glm53m.md): protocol inventory and
  source-grounded constraints for the typed surface.
- [JSON source research](../performance/json0.gpt6a.md): retained evidence about
  Sonic, scratch, escaping, lazy values, and future measurement.
- [Initial architecture](architecture0.gpt6a.md): preserved earlier argument,
  detailed buffer reasoning, and the stable-focus example semantics.
