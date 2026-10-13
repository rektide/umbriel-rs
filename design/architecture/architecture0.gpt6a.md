---
type: Design
title: A high-performance sans-I/O Umbriel client
description: Proposed Rust boundaries for framing, selective decoding, connection state, owned projections, and a 750 ms stable-active-window stream.
resource: design/architecture/architecture0.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: User's performance and sans-I/O direction, full shared-answer reading, parser source inspection, and architecture proposal
  - resource: https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09
    title: Rust JSON Protocol Speed — full shared answer recovered from embedded page data
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Prior Umbriel IPC, active/focused semantics, and Longcat integration research
  - resource: file:///home/rektide/a/noctalia-dev/umbriel/src/server/ipc.h
    title: Local IPC connection structure and six event-family definitions
  - resource: ../performance/json0.gpt6a.md
    title: Source-checked JSON backend and allocation findings from this session
---

# A high-performance sans-I/O Umbriel client

## Direction

**One connection owner, reusable byte storage, borrowed decoding within a frame,
and small owned state across frames.** Make transport and time inputs explicit.
Start with Sonic typed projections, retain a fair simd-json comparison, and
measure complete workflows before adding concurrency or specialized machinery.

This is a proposed architecture, not an implemented crate or a performance
claim. The repository's existing package stub does not yet provide Rust APIs.
The requested [command investigation](../protocol/commands0.glm53m.md) owns the
detailed wire contract. The [JSON investigation](../performance/json0.gpt6a.md)
owns implementation evidence and the benchmark matrix.

The working goal is an efficient **Rust client implementation of Umbriel's
existing protocol**. Server-side improvements can later remove costs that no
client decoder can eliminate.

## What the prior material contributes

The full [shared ChatGPT answer](https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09)
was read, including its NDJSON framing, connection-reuse limitation, borrowed
projection example, lazy-envelope suggestion, snapshot coalescing, and benchmark
recommendations. The URL file was actually at `~/umb.chat.url`.

The [zooming-kittens investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md)
establishes the important application distinction: `active` is seat-global
activation, while `focused` is remembered per-workspace focus. It also explains
why a geometry controller needs substantially more state than an active-window
reporter. Its proposed zk-internal module was appropriate to that earlier ask;
this effort now explicitly explores a reusable Rust library.

Treat the shared answer as design input, with the inspected local compositor
as the protocol authority. In particular, the chat mentions a screencast event
family, but the local `ipc.h` read in this session enumerates only theme,
overview, keyboard_layout, windows, workspaces, and submap. Do not quietly add
an event from another source revision to this client's supported contract.

## Architectural boundaries

Initially use one Rust crate with domain modules; separate crates only when
dependency or consumer boundaries justify them:

```text
src/
  protocol/       commands, replies, event names, typed wire projections
  codec/          NDJSON boundaries, encoding, decoder implementations
  session/        one-shot/subscription lifecycle and response accounting
  observe/        derived state machines, including stable active window
  transport/     optional Unix socket/runtime adapters and path discovery
benches/
  codec/          matched decoding and framing workloads
  observe/        timed trace replay and output equivalence
examples/
  active_window/  subscription plus 750 ms dwell example
```

### 1. Codec: bytes and values

The codec frames bytes, enforces its configured frame bound, decodes a complete
JSON value, and encodes a request. It neither connects nor sleeps. Serialization
uses static newline-terminated bytes for fixed requests and a typed serializer
writing dynamic requests into reused storage. Keep JSON escaping and action
argument grammar as separate concerns.

Provide a useful low-level complete-frame interface so callers with an existing
transport/framer need not copy their data through ours. A built-in reusable
framer supplies the common path. Do not make every projection pass through a
generic JSON DOM.

### 2. Session: protocol lifecycle

The session tracks connection role, expected response, requested subscription
families, initial-state readiness, and EOF/error disposition. It emits protocol
results and outbound bytes; the caller supplies bytes received, bytes written,
EOF, and transport failures. It does not own a Unix socket or choose retry timing.

Ordinary queries/actions and persistent subscriptions are different roles.
One-shot connections close after replying; a pool of indefinitely reusable
ordinary command sockets is not the starting architecture. Subscribed-command
reuse is an implementation-dependent possibility requiring a verified transport
schedule, not an unconditional fast path. No wire request ID means a lost
connection after sending an action can leave its outcome unknown. A local
request token helps the application track work but does not solve that ambiguity.

Default integration: a long-lived subscription and separate one-shot commands.
Keep any experimental subscribed-command mode isolated. Neither action success
nor a subsequent arbitrary snapshot establishes that a visual/client geometry
transition is complete. The command investigation supplies exact lifecycle and
ordering details.

### 3. Observe: application semantics

A consumer chooses a projection and a reducer. An active-only reducer does not
need a global window map; a layout controller can explicitly select one. These
reducers accept explicit monotonic times and return observations/deadlines.
They are deterministic and replayable without an executor.

Distinguish a valid no-active-window observation from a transport gap. A
disconnect invalidates continuity; reconnect plus fresh initial snapshots starts
a new epoch. Reconnection/backoff is adapter policy, with no automatic replay of
possibly executed actions.

### 4. Transport: runtime convenience

A Unix adapter resolves the socket path, performs connect/read/write, supplies
monotonic time, and schedules the reducer's next deadline. Optional Tokio support
can expose a conventional `Stream` of owned observations. A synchronous adapter
can use the same codec and reducers. Sans-I/O does not by itself require `no_std`.

The adapter owns partial-write offsets and preserves queued bytes until accepted
by the socket. It processes complete frames locally rather than spawning a task
per message. A work budget prevents continuous input from starving other tasks;
the core reports pending work so processing can resume promptly.

## Buffer and lifetime design

Use a contiguous receive allocation and three cursors:

```text
consumed <= scanned <= filled <= capacity
       complete processed region | pending bytes | free space
```

- Search only newly unscanned bytes for LF, using `memchr`.
- Determine a frame range before decoding, including before any in-place
  simd-json mutation. Exclude the LF from the JSON slice.
- Process all available complete frames up to the scheduling budget.
- Retain only an incomplete tail. Compact when room is needed or a meaningful
  prefix can be reclaimed, not once per frame.
- When completely consumed, clear logical length/cursors and keep capacity.
- Bound both frame length and buffered input. Check per-frame length rather
  than rejecting a read simply because it contains many short valid frames.
- Process complete frames before EOF; an unterminated nonempty tail is a
  truncated-frame error under the NDJSON contract.

The request-side 64 KiB limit and server-side 256 KiB outbound queue limit from
the prior research are different quantities. They are not a reason to hardcode
64 KiB as the maximum windows snapshot. Choose client bounds explicitly against
the verified server implementation and expected large snapshots.

The ergonomic input path can append caller bytes into reusable storage. A
lower-level receive-tail/commit interface can let an adapter fill storage
directly, avoiding that extra copy. Start with initialized spare space or an
established buffer API; bespoke unsafe uninitialized reads need demonstrated
benefit. A circular buffer is not automatically better: wrapped JSON frames
must often be linearized for either parser.

### Borrowed inside, owned at the boundary

A frame view borrows receive storage. Complete synchronous observation before
that storage is compacted, refilled, or mutated. A callback/lending frame API
expresses this naturally; an illustrative shape is:

```rust,ignore
// Design sketch; names and error types are not implemented APIs.
framer.with_next_frame(|frame: &mut [u8]| {
    let active = decoder.project_active(frame)?;
    focus.observe(active, now); // retain only data needed across calls
    Ok(())
})?;
```

Keep the decoder/reducer separate from the framer's borrow. A frame-level
`&mut [u8]` admits both Sonic's shared read and simd-json's exclusive in-place
read. Use concrete implementations/generics initially; a runtime plugin trait
with complex lifetime-associated types is not necessary to compare backends.

A normal asynchronous `Stream` cannot conveniently yield borrowed objects from
its own reusable internal buffer. Its items should own the selected record,
or own a retained raw frame when explicitly requested. `Bytes`/`Arc` sharing
can be useful for retained raw messages, but one tiny retained slice may keep
an entire large allocation alive. That tradeoff belongs in an opt-in interface.

For the active-window path, copy only ID and chosen metadata into reducer-owned
strings. Reuse string capacity and replace contents only when changed. Emitting
an independently owned stream item can require another copy or a buffer swap;
that is a per-output cost, not a per-input-snapshot cost. Do not claim a
zero-allocation public stream merely because the parser borrows.

## Decoding and protocol evolution

JSON object keys are unordered. Never classify a message from its first key or
search for an unstructured substring such as `"active":true`. A windows-only
subscription still needs to handle error replies and malformed input.

For the first implementation, compare:

1. A complete typed envelope with the selected windows projection.
2. A complete lazy envelope for mixed families, followed by selected payload
   decoding.
3. A custom visitor that returns only the active window and validates all
   remaining entries before committing state.

The lazy envelope solves payload-before-tag ordering but can add a second scan.
The direct visitor avoids a vector of windows. Neither should rely on ignoring
malformed suffixes or unknown-field validation to win a benchmark. For an
active-only projection, multiple `active:true` entries are a detectable
invariant violation, not a reason to pick an arbitrary winner.

Accept added fields within known payloads while enforcing the types/invariants
of fields used by the consumer. Preserve unknown event families as a deliberate
unsupported/opaque outcome. Keep backend-specific lazy values internal to the
decoder unless the caller explicitly selects a backend-specific API. Window
and workspace IDs should be opaque string newtypes at the public boundary;
the observed hex window spelling is not sufficient justification for permanently
restricting the API to `u128`.

Full snapshot means replace inventory state, not patch only members present in
the new list. Absence can mean a window closed or stopped being active. A
full-inventory reducer can use generation marking or a staged replacement;
the active-only reducer needs just its candidate. Publish updates only after
complete successful decoding.

## Stable active window

<a id="stable-active-window"></a>

### Proposed meaning of the example

“Wait 750 ms before reporting a window” is interpreted as **continuous observed
activation of the same window ID for 750 ms**. This is an explicit proposal:
not 750 ms of silence on the windows feed and not a fixed-rate sample.

The example subscribes to `windows`, selects `active`, and starts its first
dwell from receipt of the initial snapshot. It cannot know how long that window
was active before subscribing. Title, size, app metadata, workspace, and other
windows changing do not restart the identity dwell.

Minimal state:

```text
epoch                 changes when continuity is lost
candidate             Unknown | Known(None) | Known(Some(window record))
candidate_since       local monotonic observation time
generation            increments on every candidate identity transition
reported_generation   ensures one output per qualifying activation episode
deadline              candidate_since + 750 ms, or absent
```

Store the latest requested metadata for the candidate. At emission, report that
metadata, not a stale copy from the start of the dwell. This example emits once
per qualifying identity episode; continuous title-update reporting is a separate
option and does not change the dwell rule.

| Input | State transition |
|---|---|
| First valid snapshot | Establish candidate and a new deadline |
| Same active ID, changed metadata | Refresh selected fields; preserve deadline/generation |
| Different active ID | Replace candidate; increment generation; start new dwell |
| No active entry | Immediately cancel the old candidate; begin a no-active episode |
| Deadline reached with same valid epoch/generation | Emit once and clear deadline |
| Disconnect, parse failure, or known input loss | Invalidate candidate/deadline; emit a separate continuity-gap status |
| Fresh subscription snapshot after reconnect | Start a new epoch and fresh dwell |

For a stream of `Option<Window>`, the proposed default emits stable `None` after
the same 750 ms dwell. Internal cancellation of the old window is immediate.
Consumers that need an immediate clear can choose that policy explicitly.
Connection loss is never encoded as an ordinary valid `None` snapshot. A
windows-only output can filter valid `None` after the reducer has handled it.

### Timelines that must remain correct

| Observations | Expected report |
|---|---|
| A at 0; A title changes at 100 and 600 | A at 750, with latest title |
| A at 0; B at 500 | No A; B at 1250 if still active |
| A at 0; B at 400; A at 450 | A at 1200, not 750 |
| A reported; brief B; back to A | A may report again after its new 750 ms episode |
| A at 0; no-active at 700 | Cancel A; stable None at 1450 under the default policy |
| A at 0; disconnect at 700; A after reconnect at 1000 | Gap status; A at 1750 |
| A at 0; no more input | Timer alone emits A at 750 |

Use injected monotonic times, not wall-clock timestamps. Arrival timestamps and
an optional emission timestamp help consumers interpret lateness. If clock input
is user-supplied, define and enforce nondecreasing time within an epoch.

### Drain while waiting

The 750 ms is a publication policy, not a sleep in the read loop. Keep reading
and observing snapshots throughout it. The server's bounded outbound backlog
makes a slow or parked reader an actual connection problem.

Process every **delivered** active-ID transition before coalescing metadata or
expensive full-state work. Keeping only the latest raw windows snapshot can
erase A → B → A and falsely count a continuous A dwell. Generic latest-state UI
coalescing and continuity-aware automation must have different policies.

Within an adapter turn, process pending complete input before firing a due
deadline; a known backlog means the candidate may already be obsolete. If a
fairness budget is exhausted, reschedule processing and postpone publication
until caught up rather than certifying stale state. At equal supplied timestamp,
process the input transition before the timeout. These deterministic choices
can be exercised without real sleeps.

There is no server event timestamp or complete physical-focus history in this
design's evidence. The compositor itself coalesces snapshots, so even an exact
client cannot prove no physical focus flicker happened between snapshots.
The guarantee is continuity of **observed protocol state**. Socket/host scheduling
can delay both observation and emission; document that instead of pretending
the dwell measures compositor time exactly.

### Async delivery and backpressure

A pull-only `Stream` stops driving its socket when its consumer stops polling.
Therefore a convenience stream that promises continuous observation needs a
separately driven connection task or an explicit driver future that the caller
runs. This driver still performs framing, parsing, and reduction in one owner;
there is no reason to put a task/channel between those steps.

For a latest-active-window API, a bounded latest-value output can coalesce
**already reduced outputs** while the driver keeps observing input transitions.
Expose generation/epoch so skipped outputs are recognizable. For a consumer
requiring every report, use a bounded FIFO and an explicit lag/error policy;
never silently switch such a stream to latest-only semantics. An unbounded
queue or a blocked driver is not an answer to sustained slow consumption.

## What to optimize, and what evidence decides

The cost model is approximately:

```text
server state construction/serialization
  + socket/connect/wakeup cost
  + framing and buffer movement
  + JSON validation/projection
  + reducer work and retained-string updates
  + output delivery
```

The intentional 750 ms policy delay is a separate quantity. Measure deadline
overshoot and processing latency; do not present the deliberate dwell as a
decoder regression. Likewise a fast parser cannot eliminate the server's
construction of a full windows snapshot or the connect cost of one-shot commands.

The [performance investigation](../performance/json0.gpt6a.md#performance-experiment)
provides a concrete corpus and comparisons. Primary acceptance properties are
correct trace outputs, bounded memory, prompt input draining, and independently
measured decoding/allocation behavior. Warmed allocation-free framing is a
reasonable target; escaped text, retained strings, and owned emissions need
separate accounting rather than a blanket zero-allocation promise.

Potential later upstream work: a narrow active-window subscription, explicit
persistent command mode, correlation IDs and specified batching. These would
change the cost structure. They are proposals, not current protocol capabilities.

## First coherent implementation slices

1. Pin the protocol evidence and build representative fixtures. Implement
   NDJSON framing, fixed/dynamic request encoding, and checked minimal decoding.
   Verify arbitrary chunking, multiple frames/read, EOF tails, bounds, escaped
   titles, reordered keys, and initial/error replies.
2. Implement the runtime-free stable-focus reducer with injected time and the
   timelines above. Add direct frame projection and benchmark equivalent
   decoding shapes. A reference result must agree before timing is meaningful.
3. Add one Unix runtime adapter and the active-window example, exercising
   continuous input draining, partial writes, deadline wakeups, reconnect epochs,
   and explicit slow-consumer semantics.
4. Expand typed commands and full-inventory consumers against the protocol
   catalog. Promote measured decoder optimizations, including tape reuse, where
   their workload wins justify the API and maintenance cost.

This sequence makes the first example useful while preserving a clean protocol
library for heavier controllers. Architecture choices still needing evidence
are the default JSON backend, lazy versus direct event decoding, and the
best frame-buffer interface; the sans-I/O and lifetime boundaries support all
of them without a speculative transport framework.

## Cross-references

- [Protocol command research](../protocol/commands0.glm53m.md) constrains
  request construction, initial-state readiness, and connection roles.
- [JSON research](../performance/json0.gpt6a.md) grounds parser API and reuse
  claims and gives the matched benchmark plan.
- [Earlier Longcat research](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md)
  is the concrete whole-state controller use case, including target-versus-
  committed geometry and the limitations of focused-only actions.
