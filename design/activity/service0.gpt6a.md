---
type: Proposal
title: Active-window activity service and journal replay
description: Brainstorm and ticket handoff for a foreground systemd user service, structured activity records, journal storage, and CLI re-emission.
resource: design/activity/service0.gpt6a.md
status: proposed
generated:
  at: 2026-10-18
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_eaf7d6478ffe6aN53eIe8qdy8D
    title: Initial service commissioning and ticket decomposition, followed by tracing-first direction and a linked records refinement
  - resource: ../../crates/umbriel-client/examples/active_window.rs
    title: Existing continuous observer, identity dwell, queued-event precedence and invalidation on gaps
  - resource: ../../crates/umbriel-client/Cargo.toml
    title: Client dependencies and current example/binary packaging
  - resource: ../../crates/umbriel-client/README.md
    title: Discovery, subscription ownership and typed-readiness distinction
  - resource: ../architecture/architecture2.gpt6a.md
    title: Existing runtime/state boundaries and bounded independent observation
  - resource: ../kubeless/asks0.gpt61s.md#dwell-possible-pure-lift-not-snapshot-debounce
    title: Prior evidence distinguishing stable identity dwell from snapshot debounce and input idleness
  - resource: https://jsonlines.org/
    title: JSON Lines encoding, per-line values and recommended final newline
  - resource: https://github.com/ndjson/ndjson-spec/blob/master/README.md
    title: NDJSON serialization and parsing conventions
  - resource: https://docs.rs/sd-notify/0.5.0/sd_notify/
    title: Lightweight notifications and watchdog API
  - resource: https://docs.rs/tracing-journald/0.3.2/tracing_journald/struct.Layer.html
    title: Structured journal logging, field normalization, prefixes and priorities
  - resource: https://docs.rs/libsystemd/0.7.2/libsystemd/
    title: Pure-Rust broader notification/logging alternative
  - resource: https://docs.rs/systemd/0.10.1/systemd/journal/
    title: Native libsystemd-backed journal reading and writing
  - resource: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html
    title: Notify readiness, watchdog, reload protocol and restart semantics
  - resource: https://www.freedesktop.org/software/systemd/man/latest/journalctl.html
    title: Journal selection, cursor/follow behavior and JSON representation edge cases
  - resource: https://crates.io/api/v1/crates/sd-notify
    title: Download/adoption metadata inspected alongside the other candidate crates
extensions:
  ticket: umbrs-activity
---

# Activity service and journal replay

**Continuation:** [Tracing-native activity records](records0.gpt6a.md) records the
user's subsequent selection of tracing-journald for all journal export and tracing
layers for every activity output. Its field-first contract supersedes this draft's
canonical-JSON-payload recommendation, and its capability inventory/delivery order
refines the original broad CLI ticket. The initial brainstorm remains below as
context; current scope and dependencies live in the tickets.

## Situation and commissioning

The existing `active_window` Cargo example observes seat-global active identity,
waits for stability, and prints ID/application/title separated by tabs. Diagnostics
go to stderr. It is an example, not an installed daemon or a journal reader.
Queued changes beat a due dwell timer; gaps invalidate continuity. Title and
geometry updates do not restart identity dwell.

The user requested a production-oriented sample service, systemd notifications
through a popular crate, journal logging and CLI readback/re-emission, JSONL/NDJSON
and key=value output, and a **5-second default acceptance delay**. This session
commissions only that default change immediately, delegated to G. The remaining
tickets are a later implementation handoff. Names and detailed interfaces below
are recommendations for discussion, not implemented commands.

Work anchor: **`umbrs-activity`**. Navigation: [design index](../index.md).

## Recommended shape

An installable `umbriel-activity` executable with `watch` and `history` subcommands.
Keep it in the foreground: systemd owns process supervision. Use a **user unit**
because compositor discovery and socket access belong to the graphical user.

Prefer application composition under `crates/activity/` (package name may be
`umbriel-activity`), depending on client and state. Keep record, formatting, journal
and lifecycle modules together there initially. This preserves the existing
[library ownership boundaries](../architecture/architecture2.gpt6a.md#crate-graph)
and avoids making client transport or the pure reducer depend on systemd. The
[desktop dump](../../crates/umbriel-client/README.md#desktop-capture-tool-and-example)
is precedent for a normal binary, though its dependency needs differ.

Illustrative surface:

```sh
umbriel-activity watch --dwell 5s --format jsonl
umbriel-activity watch --sink journal
umbriel-activity history --since today --format logfmt
umbriel-activity history --follow --after-cursor CURSOR --format ndjson
```

`--format` is the rendering; `--sink` is the destination. Re-emission means
**stdout export of the original activity records**, retaining their original
times and identities. Journal metadata such as cursor belongs in an optional
export envelope. Do not automatically write history back into the journal: that
would create duplicate events and potentially a follow feedback loop. If journal
reinsertion is wanted later, it needs explicit replay provenance and selection.

## Record and output contract

Use one versioned typed activity record for live output, journal payloads and
history. Recommended fields: schema version, event kind, run ID, sequence,
connection epoch, candidate start/acceptance times, dwell, and window identity
plus application/title metadata. Decide the exact names in `umbrs-activity-records`.
Use wall-clock timestamps for navigation and monotonic values scoped to the run
for elapsed-time reasoning. Journal ingestion time is not focus onset time.

Represent stable active identity, stable **no active window**, and observation
**gap/reacquisition** distinctly. A new run/epoch must not quietly continue an old
window interval. Retain the reducer's latest metadata at acceptance; continuous
title-change tracking is a separate policy. Five seconds filters brief switches;
it does not measure human attention or provide exact time accounting. In
particular, switching away before another identity qualifies does not prove the
previous window stayed active. Duration summaries require a later explicit model.

**JSONL and NDJSON:** for this tool they should be aliases for identical UTF-8,
one-object-per-line output, with LF after every record and escaped embedded
newlines. JSON Lines is the broader convention; NDJSON documents a streaming
serialization/parsing convention. NDJSON requires the final newline; JSON Lines
strongly recommends it. Both support CRLF; NDJSON permits documented empty-line
skipping while JSON Lines treats a blank line as invalid. Both allow any JSON
value, but this application deliberately emits objects. Neither wraps the entire
stream in a JSON array or pretty-prints multiline records.
RFC 7464 JSON Text Sequences (`json-seq`, record-separator bytes) is different.

**key=value:** call it `logfmt`. Document a concrete dialect: stable keys,
quoting/escaping for spaces, equals, quotes, backslashes, tabs and newlines;
explicit representation of missing window and optional fields; exactly one
physical line per record. It is not shell syntax. Preserve field meanings between
formats, with schema-defined conversion of logfmt strings back to typed values.

## Crate choice and journal contract

Recommendation: **`sd-notify`** for lifecycle notifications and **`tracing` with
`tracing-journald`** for structured diagnostics. At research time docs.rs listed
sd-notify 0.5.0 and tracing-journald 0.3.2. Crates.io reported approximately 16.0M
and 9.8M total downloads respectively: established adoption, not a quality proof.
Use the installed release's actual API; old sd-notify examples differ.

`libsystemd` 0.7.2 is a broader pure-Rust alternative covering notifications and
journal submission. `systemd` 0.10.1 wraps native libsystemd and exposes a journal
reader. These similarly named crates have different deployment requirements.

For activity entries, preserve a canonical serialized record in a named journal
field, alongside queryable kind/schema/application/run fields and human MESSAGE.
Specify the actual journal names: tracing-journald normalizes names and defaults
to an `F` field prefix. Its INFO default maps to journal Notice, also configurable.
Do not reconstruct records by parsing prose MESSAGE or accidentally filter them
out with diagnostic log-level settings. Choose and test a fallible submission
path if the application needs to react to send failures; a tracing Layer callback
does not itself offer callers a per-event Result.

Emit each activity record once. In journal mode, native submission plus an stdout
copy captured by systemd would duplicate it. Diagnostics may use stderr capture
or a native layer, with one deliberate routing arrangement. Journal send success
is not a durable commit acknowledgement. Retention, vacuuming, rate limits and
storage configuration bound what can later be recovered.

For reading, choose between native `systemd::journal` and a supervised
`journalctl --output=json --all --no-pager` subprocess. The latter reuses the
host tool without linking libsystemd, but needs bounded streaming, cancellation,
child reaping and exit/error handling. It must account for journal JSON's arrays
for repeated/binary fields and avoid the default 4096-byte-to-null conversion.
Use one cursor-aware historical/follow stream; define initial-tail behavior
explicitly, since journalctl follow otherwise implies a tail limit.

Select activity records by marker/schema plus current user's trusted unit/UID
metadata, with explicit support for foreground runs if desired. Identifier alone
can also match other producers. Support time/boot/run filters and opaque
after-cursor resume. Distinguish no matches, unavailable journal/access, malformed
payloads and unsupported schemas. Cursor disappearance due to retention needs an
explicit error/gap policy, not a silent claim of complete replay. `--user` journal
availability depends on host storage configuration; document the tested setup.

## Systemd behavior worth demonstrating

1. **Readiness/status/stopping.** `Type=notify`, `NotifyAccess=main`; READY after
   sink initialization and a valid initial Windows census, not merely socket
   connection or the first five-second acceptance. STATUS describes starting,
   observing or reconnecting; STOPPING precedes bounded graceful shutdown.
   Missing notify environment is ordinary standalone operation. A configured
   broken notification endpoint is an actual error with a documented policy.
2. **Watchdog tied to useful progress.** Respect WATCHDOG_USEC/PID using the crate.
   A quiet desktop is healthy; no event-frequency timeout. Heartbeats must stop
   if the observation/output pipeline wedges, rather than come from an unrelated
   unconditional timer. Reconnect progress can be healthy while source status
   remains degraded. Fake-notify socket tests plus a real user-unit smoke test.
3. **Recovery and session lifetime.** Graceful SIGTERM/SIGINT; bounded buffering
   and shutdown; explicit observer-lag/output-failure policies. Coordinate native
   reconnect/backoff with `Restart=on-failure`, restart delay and start limits.
   Never carry accepted dwell across reconnect. Configuration failures should not
   produce a busy restart loop.
4. **Graphical user-unit integration.** Pair stop lifetime (`PartOf=`) and ordering
   with the actual compositor/graphical-session target; `After=` alone neither
   starts a target nor waits for its socket. Provide session environment import
   or an explicit socket override: the user manager need not inherit the shell's
   WAYLAND_DISPLAY/UMBRIEL_SOCKET. Decide single vs multi-session instance naming.
5. **Operational packaging.** Document install, enable/start, stop, status and
   journal inspection. Validate the unit with systemd-analyze and an actual user
   manager. Demonstrate conservative resource limits and hardening compatible
   with AF_UNIX compositor/notify/journal access. Add state/runtime directories
   only when a feature actually writes state; history cursors can remain explicit.
6. **Session semantics, a follow-up decision.** logind sleep/session state and
   compositor idle/lock signals could prevent attributing locked or suspended
   periods to a window. Scope observations to the correct session; logind hints
   may be absent or advisory. A user service does not automatically inherit system
   manager sleep-target behavior. Choose evidence, unknown states and dwell reset
   rules before implementing; monitoring should not inhibit suspend.
7. **Reload, a follow-up decision.** Decide whether changing dwell/output policy
   without restart is useful. If selected, define atomic validation/application,
   rollback on invalid config and candidate-reset rules; evaluate notify-reload
   (systemd 253+) with RELOADING, MONOTONIC_USEC and READY. Keep this out of the
   initial unit until the protocol exists.

Socket activation is a poor initial fit: this is an outbound compositor client
that must observe continuously. It becomes useful if a future query/control socket
is introduced. D-Bus control/log-level interfaces, journal catalogs and timer-based
exports are possible later examples, but each needs a concrete consumer.

The [earlier dwell assessment](../kubeless/asks0.gpt61s.md#dwell-possible-pure-lift-not-snapshot-debounce)
is a reuse constraint: snapshot debounce and raw-input idle timers cannot replace
stable-identity dwell. Session-idle awareness is additional evidence, not a new
interpretation of that existing reducer.

## Ticket handoff

All child IDs below begin with `umbrs-activity-`. The epic explains the complete
service; each ticket carries its acceptance criteria and dependencies in Beads.

| Suffix | Deliverable | Prerequisites |
|---|---|---|
| `dwell` | Change the existing default to five seconds; G's immediate task | None |
| `records` | Versioned records and shared JSONL/NDJSON/logfmt renderers | None |
| `cli` | Installable foreground watch CLI, config, reconnect and graceful lifecycle | records, dwell |
| `journal` | Structured, single-copy activity/diagnostic journal routing | cli |
| `notify` | Readiness/status/stopping and progress-aware watchdog | journal |
| `history` | Filtered journal readback, cursor-aware follow and shared re-emission | journal |
| `unit` | User unit, session environment, install guide and end-to-end smoke evidence | notify, history |
| `session-awareness` | Decide idle/lock/suspend evidence and tracking semantics | None; optional inquiry |
| `reload` | Decide and design live configuration reload | None; optional inquiry |

The decision tickets can inform the core without blocking its first delivery.
The unit ticket owns end-to-end verification: scripted compositor transitions
through journal readback in both formats, idle watchdog survival, reconnect gaps,
clean stop and restart, and genuine missing/rotated-journal behavior. Lower tickets
own targeted tests of their individual contracts. No production/soak verification
is claimed by this planning document.
