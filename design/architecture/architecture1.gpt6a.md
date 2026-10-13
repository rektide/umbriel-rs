---
type: Design
title: Sonic-first architecture for the whole Umbriel protocol
description: Revised library-wide design selecting Sonic and separating wire framing, typed commands and models, session state, transport, and optional observations.
resource: design/architecture/architecture1.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Human selection of Sonic and correction toward designing and building the whole protocol library
  - resource: architecture0.gpt6a.md
    title: Predecessor sans-I/O boundaries and the focused active-window example
  - resource: ../performance/json0.gpt6a.md
    title: Source-checked Sonic capabilities and buffer ownership tradeoffs
  - resource: https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09
    title: Shared Rust JSON Protocol Speed answer, read in full
---

# Sonic-first architecture for the whole Umbriel protocol

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

Start with modules in a single crate. Keep dependency-bearing transport modules
feature-gated when introduced. The core may use `std` and allocation without
owning file descriptors, an executor, a wall clock, or a timer. That is the useful
sans-I/O boundary here.

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
supported shapes and any exceptions. The implementation should use those
findings rather than extrapolating from a single `windows` example.

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
arrived so a caller can know when its subscription is initialized. Initialization
does not imply an atomic multi-family snapshot or a shared revision number.

Ordinary command sockets are one-shot. The verified subscribed-command behavior
belongs in a separately specified mode, if offered, rather than making socket
reuse an undocumented universal assumption. Until its buffering and scheduling
contract is implemented and exercised, the normal adapter uses a subscription
connection plus one-shot request connections.

With no wire correlation ID, allow at most one outstanding request on any
experimental reusable command channel. Do not count event frames as replies.
Multiple independent one-shot requests can use separate connections if the
application wants concurrency; bound that concurrency at the adapter layer.

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
coalescing in a consumer-aware layer.

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

- [Command research](../protocol/commands0.glm53m.md): protocol inventory and
  source-grounded constraints for the typed surface.
- [JSON source research](../performance/json0.gpt6a.md): retained evidence about
  Sonic, scratch, escaping, lazy values, and future measurement.
- [Initial architecture](architecture0.gpt6a.md): preserved earlier argument,
  detailed buffer reasoning, and the stable-focus example semantics.
