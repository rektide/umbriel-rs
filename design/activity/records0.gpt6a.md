---
type: Proposal
title: Tracing-native activity records and present capability inventory
description: A field-first activity event contract, tracing layer composition, current tracker completeness, and staged service work.
resource: design/activity/records0.gpt6a.md
status: proposed
generated:
  at: 2026-10-19
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_e9a6a958affehqIA6mCZZF154C
    title: Link delivered tracing/tracking runtime and native service verification while preserving this proposal as predecessor evidence
  - resource: opencode://ses_eaf7d6478ffe6aN53eIe8qdy8D
    title: Human selection of tracing-native output and accepted identities plus metadata updates; capability reassessment, record shape and reviewed FX reuse findings
  - resource: service0.gpt6a.md
    title: Predecessor service brainstorm and ticket decomposition, revised here toward tracing-first delivery
  - resource: ../../crates/umbriel-client/examples/active_window.rs
    title: Actual foreground loop, output, timing and cleanup behavior
  - resource: ../../crates/umbriel-state/src/active.rs
    title: Existing retained metadata, episode semantics and monotonic observation times
  - resource: ../../crates/umbriel-client/src/driver.rs
    title: Implemented optional reconnect, startup connection and explicit source shutdown
  - resource: ../../crates/umbriel-client/tests/subscription.rs
    title: Existing reconnect/gap/epoch and shutdown evidence
  - resource: file:///home/rektide/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tracing-journald-0.3.2/src/lib.rs
    title: Actual scalar field encoding, custom prefixes, initialization probe and ignored per-event send result
  - resource: file:///home/rektide/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tracing-subscriber-0.3.23/src/fmt/format/json.rs
    title: Existing newline-delimited JSON formatter and structured-value limitations
  - resource: prior-art0.glm53fx.md
    title: Commissioned hidden-aware survey of prior journald, tracing and OSC work
  - resource: file:///home/rektide/src/cyclrs/telemetry/journald/src/dispatch.rs
    title: Verified direct Dispatch submission and disabled-diagnostics field test
  - resource: file:///home/rektide/src/cyclrs/telemetry/journald/README.md
    title: Implemented stock journald recorder and readback visibility limits
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/read.rs
    title: Verified exact cursor positioning and bounded finite range traversal
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/lib.rs
    title: Domain-coupled feed types and thread-local native reader ownership
  - resource: file:///home/rektide/src/cyclrs/source/journald/README.md
    title: Byte enumeration, limits and native reader caveats
  - resource: file:///home/rektide/src/cyclrs/design/architecture-refresh/journal-syn0.gpt6astra.md
    title: Earlier journal comparison and corrected visibility/durability claims
  - resource: file:///home/rektide/src/systemd/src/shared/osc-context.c
    title: OSC 3008 identity/escaping and supported context types, including service
  - resource: file:///home/rektide/src/term-preserver/design/tracking/osc-protocols.glm53m.md
    title: OSC 7501 design research, status replacement and terminal-bound lifetimes
extensions:
  ticket: umbrs-activity-records
  related_work: [umbrs-activity, umbrs-activity-journal, umbrs-activity-session-osc, umbrs-activity-notify]
---

# Tracing-native activity records

**Delivered continuation (October 22):** [runtime and deployment evidence](runtime0.gpt6a.md)
and the [application contract](../../crates/activity/README.md) describe the now-running
accepted-plus-metadata service and reusable tracking library. Historical capability
statements below describe this proposal's earlier state; notify/history remain open.

## Direction selected; contract still being discussed

The user selected **tracing-journald for journal export**, and wants tracing
implementations to own application output, including JSON and session OSC.
That changes the most useful first delivery from broad service packaging to
**structured activity events plus composable tracing layers**.

This proposal refines [the service brainstorm](service0.gpt6a.md). Its separate
canonical-JSON-payload recommendation is superseded by the field-first proposal
below. The user subsequently selected **accepted identities plus metadata updates**;
the exact fields remain proposed.
No new implementation is claimed here. The five-second default is already done.

## How complete is the current tracker?

The runnable tracker is a Cargo example, `active_window`, not yet the proposed
packaged CLI with flags and history commands.

| Capability | Present behavior | Remaining work |
|---|---|---|
| Foreground observation | Continuous native subscription with independently driven socket and bounded observers | Reuse it |
| Socket discovery | Environment-based discovery through the client | Expose an explicit CLI override |
| Acceptance delay | Five-second default; library constructor accepts another duration | Expose configuration |
| Correct active identity | Seat-global `active`; same ID refreshes metadata without resetting dwell | Reuse it |
| Timer and race handling | Timer-only emission; queued transitions/gaps beat a due timer | Preserve existing tests |
| Known absence | Stable no-active episode is distinct from invalidation | Expose it structurally |
| Retained data | ID, app ID, title, workspace, scratchpad, epoch, since/last-observed/emitted times | Current text emits only ID/app/title |
| Continuity loss | Epoch/gap/decode/invalid census invalidate candidates | Emit machine-readable lifecycle events |
| Reconnect | Client has `Reconnect::Retry { delay, max_attempts }`; example uses `Never` | Enable/configure application policy, medium priority |
| Initial connection retry | Connect occurs before spawning the driver; initial failure returns | Decide startup retry separately |
| Source/task shutdown | Library awaits actual cleanup; example calls it on ordinary loop error exit | Signal handling and output drain, medium-low priority |
| Event output | `println!` and `eprintln!` | Tracing events/layers: highest-value start |
| History, journal, notify, OSC | Not implemented by the example | Ticketed integrations |
| Title changes after acceptance | Reducer refreshes internal metadata but emits no new episode | Add selected metadata-update events |

Reconnect is **bounded fixed-delay transport/EOF recovery**, not a general
exponential-backoff policy, initial-start retry, protocol-error recovery or observer
reattachment. Existing subscription tests exercise gap/new epoch/no command replay.
Graceful source cleanup is already implemented; SIGTERM/SIGINT routing is the
missing application behavior. These are integration tasks, not new foundations.

The observer tracks stable active identity well. It does not infer attention,
idle/lock state, exact time spent, or every compositor transition. Those should
remain explicit additions rather than conditions for calling this useful.

## Output architecture

```text
Windows observations -> existing reducer -> typed emission helpers
                                             |
                                         tracing events
                                             |
                  +--------------------------+-----------------------+
                  |                          |                       |
          tracing-journald           fmt JSON / logfmt          session OSC layer
          structured fields          newline records           protocol frames
```

The producer emits facts once. Subscriber/layer configuration selects destination
and representation. It never prints activity directly or serializes JSON as its
primary message. Ordinary CLI help/version remains ordinary CLI behavior.

Small typed helpers (for example `emit_accepted(&StableActive, &EventContext)`)
centralize tracing field names and event invariants. An owned event enum may help
history decoding; it is an internal convenience, not a second sink abstraction.
Reuse the reducer and its `StableActive` value. Application clock/run identity and
tracing belong at the consumer boundary, not inside the pure reducer.

Use explicit flat scalar fields rather than `?record` or a serialized record
string. Stable tracing visitors understand scalars; a Debug struct is not a
portable structured object. This also makes journald fields directly queryable.

## Initial field shape

Two data targets: `umbriel::activity` and `umbriel::session`. Diagnostics use a
separate target policy. Data events may all be INFO; **kind is an explicit field**,
not inferred from level or prose. A schema version describes the fields and
their meaning across projections, not the tracing library's entire JSON envelope.

Every data event:

| Field | Meaning |
|---|---|
| `schema_version` | Integer, initially 1 |
| `event` | Stable kind such as `activity.accepted`, `activity.metadata`, `source.gap`, `source.epoch`, `source.ready` |
| `run_id` | Unique producer run identity; preserved during history export |
| `seq` | Increasing sequence for data events in that run, across activity/session targets |
| `event_time` | Original application event time, UTC RFC 3339 |
| `event_mono_ms` | Original event offset from the run's monotonic origin |

Source-bound events also carry `epoch`. An accepted episode adds:

- `state = "active" | "none"`;
- `since_mono_ms`, `last_observed_mono_ms`, `dwell_ms`;
- when active: `window_id`, `app_id`, `title`, `workspace_id`, `scratchpad`.

The first shape keeps timestamps at millisecond precision, sufficient for the
five-second policy; units are in field names. These are local consumer observation
times, not compositor event timestamps. `event_mono_ms` maps to the reducer's
`emitted_at` for acceptance. Wall-clock jumps do not change the dwell computation.

For `state=none`, omit window-specific fields. Empty strings on an active window
retain their source meaning. Absence is therefore not inferred from empty strings
or a Debug-printed `Option`. A gap has a stable `reason` and optional diagnostic
`error`, never a fabricated `state=none`.

`source.epoch` announces new connection lineage, not readiness. `source.ready`
follows a valid typed census after acquisition/invalidation. `source.gap` marks
loss/invalid input immediately, independently of the five-second filter. Avoid
double-reporting one source terminal as both an observed gap and a second terminal
error. These lifecycle facts will give notify a useful interface later.

Conceptual accepted event fields (the exact API is still a proposal):

```text
schema_version=1 event=activity.accepted run_id=<run> seq=12 epoch=1
event_time=2026-10-18T15:00:05Z event_mono_ms=9200
state=active since_mono_ms=4200 last_observed_mono_ms=6100 dwell_ms=5000
window_id=<id> app_id=org.example.Terminal title="notes and code"
workspace_id=2 scratchpad=""
```

This is not an interval-end record. A briefly visited B that fails the dwell does
not establish continuous time spent on the last accepted A. If exact boundaries
matter, add an immediate transition stream deliberately.

### Selected addition: metadata after acceptance

The user selected accepted identity **and** metadata updates, rather than adding
the unfiltered identity-transition stream. Proposed semantics:

- `activity.accepted` is emitted once after the candidate satisfies dwell, with
  the latest complete selected metadata, as today.
- `activity.metadata` is emitted when title, app ID, workspace ID or scratchpad
  changes while the **same current candidate remains accepted**. It carries the
  full updated metadata projection and `accepted_seq`, referring to the acceptance
  event's `seq` within the same `run_id`. It has its own event sequence/time.
- Identical selected metadata, geometry changes and timer ticks produce no metadata
  event. Pre-acceptance metadata churn is folded into the eventual acceptance.
- Metadata emission has no additional five-second delay. It preserves the episode's
  original `since_mono_ms`; `last_observed_mono_ms` reflects the new valid census.
- Observing a different identity, known absence, gap or invalid input immediately
  ends eligibility to emit metadata for that accepted episode. Returning A after
  A-B-A needs a fresh dwell/acceptance even if B never qualified. Do not keep
  updating the last *reported* A across an unaccepted B or unknown period.
- History may start after the acceptance or lose it to retention. The full metadata
  event remains interpretable; the reference does not fabricate missing acceptance.

Prefer a typed metadata-change outcome from the existing reducer (or an equivalent
unified change interface) over a second consumer-owned continuity machine. The
application attaches `accepted_seq` and output context; the pure reducer decides
which candidate qualifies and whether selected metadata changed. This is a small
new capability, unlike the already implemented foreground loop and reconnect.
Verification should cover no-op/geometry-only changes, churn before acceptance,
one changed census, A-B-A, known absence and source invalidation.

## Projections are layers

**JSONL/NDJSON:** use tracing-subscriber's existing JSON formatter, with aliases
for the same mode. Start with its regular `timestamp`, `level`, `target`, `fields`
envelope. The application's contract is in `fields`; `event_time` remains the
original event time even when the envelope timestamp reflects a later export.
Keep essential fields directly on each event rather than relying on span context.

**Journal:** use tracing-journald, as selected. Recommend a fixed `UMBRIEL` field
prefix: `event` becomes `UMBRIEL_EVENT`, `window_id` becomes `UMBRIEL_WINDOW_ID`,
and so on; MESSAGE remains human-readable, TARGET is native metadata. Use only
snake_case ASCII names to avoid name-normalization collisions. Journal values
are bytes/text; history reconstructs the typed fields according to the schema.
There is no separate canonical JSON blob in this proposal.

**Logfmt:** another tracing formatter/layer over the same fields. Select an
existing implementation if the user's prior work supplies one; verify its
escaping before promising a dialect. One physical line per event, with quoted
values where necessary. The ordinary compact tracing formatter is not assumed
to be machine-compatible logfmt.

**OSC 3008/7501:** a session event target and dedicated tracing layer/formatter
for the actual terminal protocol. The same session event can also reach journal
and JSON as structured data. Exact payloads, session identifiers, framing,
terminators, destination and capabilities must follow the protocol references
[recovered by FX](prior-art0.glm53fx.md#osc-3008--upstream-systemd-implementation).
Do not invent wire syntax from the OSC numbers. JSON text
escaping an OSC string and a terminal writer transmitting control bytes are
different projections; decide which are wanted. Ticket: `umbrs-activity-session-osc`.

Configure per-layer filters: data selection must not disappear because a user
turns diagnostic verbosity down with RUST_LOG. Journal-only service mode enables
the native journal layer without duplicating its activity on captured stdout.
Interactive runs may choose JSON or logfmt and optionally a dedicated OSC writer.
Two unrelated formatters must not interleave bytes on the same writer.

Tracing-journald 0.3.2 probes availability during construction, but its `on_event`
discards the send result. The selected stock layer is therefore best-effort per
event. This is a concrete property for notify's health model: READY may follow
successful layer initialization, but watchdog health cannot mean acknowledged
journal persistence. Trace callbacks are synchronous; layer I/O/backpressure and
optional worker queues are still part of the process's responsiveness.

## History also uses tracing

Read journal native fields into the schema and re-emit through the **same emission
helpers** with original run/sequence/times. Select a history/export dispatcher with
JSON/logfmt (and OSC only if deliberately selected); it does not contain the live
journal layer. Keep export diagnostics distinct from replayed data. If dispatch is
scoped around async work, use tracing's async-aware dispatch propagation rather
than assuming a thread-local guard will cover futures across executor threads.

History metadata such as cursor, ingestion time and boot ID is optional export
context, not a replacement for the original event's identity/time. Native journal
reading versus journalctl streaming remains open pending FX's reuse research.

## Delivery order and open questions

1. Agree the minimal fields/event kinds; implement tracing-native emission and
   JSON/logfmt layer selection around the existing loop (`records`, P1).
2. Compose the selected tracing-journald layer (`journal`, P1), independently of
   reconnect/shutdown enhancements or installation polish.
3. Package CLI/configuration; add reconnect (P2), signal shutdown/drain (P3),
   history and session OSC as separately scoped work.
4. Once the tracing core exists, **AM designs notify, then Astra xhigh implements
   that design** (interpretation of the user's requested AM -> XH handoff). The
   notify ticket records the sequencing; no dependent agent has been dispatched yet.

Questions settled with the user:

- **OSC projection (settled 2026-10-27):** either/or/both. The same session
  events must support structured (journal/JSON) and terminal-byte (OSC 3008/7501)
  projections as **independently selectable layers**, not one fixed choice. The
  producer stays single-path; `umbrs-activity-session-osc` owns the wire formats.
- **JSON envelope:** resolved by the shipped implementation — the stock
  tracing-subscriber envelope with event fields under `fields` is the delivered
  default (see `crates/activity/README.md`); revisit only for a concrete consumer.

Delivery status (2026-10-27): steps 1–3 landed — records/journal/cli/reconnect/
shutdown are closed, the service runs as a Type=simple user unit, and remaining
work (notify design/build, history native readback, session-osc, decisions) is
tracked on open tickets under `umbrs-activity`.

## Verified local reuse from FX's survey

The [hidden-aware survey](prior-art0.glm53fx.md) searched normal and hidden design/
doc homes. The parent checked these particularly relevant findings against code:

- [cyclrs journal dispatch](file:///home/rektide/src/cyclrs/telemetry/journald/src/dispatch.rs)
  constructs raw tracing events and submits to an explicit private Dispatch,
  bypassing host diagnostic/macro gates. Its existing test checks fields with
  diagnostics disabled. This is an available technique if compile-time filtering
  independence is required; choose it deliberately alongside layer composition.
- [cyclrs reader](file:///home/rektide/src/cyclrs/source/journald/README.md) supplies
  working patterns for exact cursor validation, finite upper cuts and byte budgets.
  Its reader is a thread-local libsystemd handle; use an owning worker if composed
  with async tracking. The crate is coupled to Cyclrs event/feed types: reuse or
  extract the mechanisms deliberately rather than assume a drop-in dependency.
- [cyclrs recorder](file:///home/rektide/src/cyclrs/telemetry/journald/README.md)
  adds full-event readback visibility to stock tracing-journald. That is useful
  precedent, but mandatory per-event readback is not selected for this activity
  observer. Its canonical JSON envelope also need not become our field schema.
- [systemd OSC context](file:///home/rektide/src/systemd/src/shared/osc-context.c)
  implements 3008 and includes **service**, session and other context types.
  Choose context matching the actual producer/subject rather than equating a
  compositor window, shell session and application run.
- [term-preserver's protocol brief](file:///home/rektide/src/term-preserver/design/tracking/osc-protocols.glm53m.md#osc-7501--program-status-protocol)
  is research, not an emitter. Its 7501 replacement/lifetime semantics require a
  terminal destination and mapping of actual program status. Window activation
  alone does not establish `working`, `blocked`, or `done`. The detached-service
  projection still needs a concrete terminal-facing consumer; recording structured
  session facts can precede that transport choice.

The [design index](../index.md) connects the proposal and research. No cross-project
code was imported and no journal/OSC runtime integration was implemented here.
