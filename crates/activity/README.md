---
type: Reference
title: Umbriel activity application and journal contract
description: Runnable library-backed activity service, field-first tracing records, bounded output cancellation, installation and consumer filters.
resource: crates/activity/README.md
status: implemented
generated:
  at: 2026-10-22
  by: model:openai/gpt-6-astra#medium
sources:
  - resource: opencode://ses_e6e93b602ffe0iILE38OiivjPj
    title: Add a bounded JSON Lines journal dump to the activity command
  - resource: opencode://ses_e9a6a958affehqIA6mCZZF154C
    title: Implemented and tested tracing application and bounded output worker; installed dedicated service and verified native journal fields
  - resource: ../../design/activity/records0.gpt6a.md
    title: Selected field-first tracing-journald and accepted-plus-metadata design
  - resource: ../../design/activity/service0.gpt6a.md
    title: Service lifecycle and history predecessor requirements
  - resource: src/lib.rs
    title: Record, recorder and scalar tracing emission contract
  - resource: src/output.rs
    title: Projection configuration, filtering and bounded writer thread
  - resource: systemd/umbriel-activity.service
    title: Delivered dedicated simple user service
  - resource: file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md
    title: Journal reader precedents and native history follow-up ownership
extensions:
  ticket: umbrs-activity
---

# Activity application

This library-backed example application observes stable active identity and its
subsequent metadata. The reusable [tracking runtime](../tracking/README.md) owns
the observation composition; application time/run identity, tracing and lifecycle
live here. It requires neither Kopacetic nor Polars.

```sh
cargo run -p umbriel-activity -- --output jsonl
cargo run -p umbriel-activity -- --socket /run/user/1000/umbriel-wayland-0.sock --output logfmt
cargo run -p umbriel-activity -- --output journal --dwell-ms 5000
cargo run -p umbriel-activity -- dump > activity.jsonl
cargo run -p umbriel-activity -- dump --tail 100 > recent.jsonl
```

`dump` reads the retained local journal for the current UID and
`umbriel-activity.service`, without connecting to the compositor or writing back
to the journal. It emits one JSON object per line: `fields` holds decoded activity
fields and `delivery` holds journal cursor, receive time and origin metadata.
JSONL and NDJSON are names for the same format. The snapshot is bounded by
default to 10,000 entries, 32 MiB and five seconds; use `--max-entries`,
`--max-bytes` and `--timeout-secs` to raise these limits, or `--tail` to select
fewer entries. A budget or decoding failure exits nonzero rather than emitting a
partial snapshot. Retention and journal access determine what is available; the
dump is not a guarantee of complete history.

`--output ndjson` aliases `jsonl`. Explicit `--socket` overrides `UMBRIEL_SOCKET`,
then client discovery falls back to `XDG_RUNTIME_DIR` plus `WAYLAND_DISPLAY`.
Five seconds is the default, measured by monotonic time rather than wall time.
`--reconnect-attempts` defaults to 10, `--reconnect-delay-ms` to 1000; retries are
fixed-delay native transport/EOF recovery. Initial connection fails fast, protocol
errors are terminal, lag is terminal for that observer, and gaps reset dwell.
Set `--reconnect-attempts 0` to disable recovery.
Systemd can restart the application after failure. No old acceptance survives a run.

## Library and records

`Recorder` attaches UUID run identity, ordered sequence and UTC application time to
`umbriel_tracking::activity::Event`. `Record` retains original values;
`emit(&Record, &tracing::Dispatch)` re-emits them unchanged through any selected
projection. A future history decoder should use an export-only Dispatch, never
the live journal writer. The recorder expects one complete ordered runtime event
stream; metadata requires its preceding acceptance.

Every event uses tracing target `umbriel::activity`, INFO, and flat scalar fields:

| Fields | Meaning |
|---|---|
| `schema_version=1`, `event` | Schema and explicit kind |
| `run_id`, `seq` | Producer run and increasing data-event sequence |
| `event_time`, `event_mono_ms` | UTC RFC3339 application time and monotonic run offset |
| `epoch` | Local source lineage for source-bound events |
| `state` | `active` or `none`, only on accepted/metadata events |
| `since_mono_ms`, `last_observed_mono_ms`, `dwell_ms` | Candidate start, latest valid observation and acceptance policy |
| `window_id`, `app_id`, `title`, `workspace_id`, `scratchpad` | Full selected metadata, only when state is active |
| `accepted_seq` | Metadata's acceptance within the same run |
| `reason` | Diagnostic continuity-loss text on gaps; do not parse it as an enum |

Kinds: `source.epoch`, `source.ready`, `source.gap`, `source.stopping`,
`activity.accepted`, `activity.metadata`. A ready event is a valid typed census,
not a completed dwell or a systemd READY notification. A source gap followed by
its terminal delivery error is reported once. Stopping cancels pending dwell.

Selected metadata follows the same currently accepted candidate only. Geometry
churn emits nothing; identity changes/gaps end eligibility immediately. Acceptance
is not an interval-end or attention record, and brief intervening focus is not
represented as another accepted identity. Immediate control consumers use Tracker.

The JSONL projection uses tracing-subscriber's normal envelope; application fields
are under `fields`. Logfmt is a defined flat dialect: numeric/bool values unquoted,
strings/debug values JSON-quoted and escaped, one physical line plus LF per event.
There is no opaque JSON record in MESSAGE. Data selection survives `RUST_LOG=off`;
the private dispatch and per-layer target filter separate data from diagnostics.
Compile-time disabling of tracing INFO by a host build is outside this guarantee.

## Journal consumer contract

Tracing-journald 0.3.2 emits `UMBRIEL_<UPPERCASE_FIELD>` for application fields,
`TARGET=umbriel::activity`, `SYSLOG_IDENTIFIER=umbriel-activity`, INFO mapped to
`PRIORITY=5`. Native values are text/bytes; parse integers by schema. Fields absent
from an event are omitted, not encoded as an Option debug string. Empty scratchpad
and active-window strings preserve source meaning. MESSAGE is descriptive only.

```sh
journalctl --user _SYSTEMD_USER_UNIT=umbriel-activity.service \
  UMBRIEL_SCHEMA_VERSION=1 TARGET=umbriel::activity --all -o json --no-pager
journalctl --user _SYSTEMD_USER_UNIT=umbriel-activity.service \
  UMBRIEL_EVENT=activity.accepted --all -o json --follow
```

`--user` scopes the user's journal; trusted `_SYSTEMD_USER_UNIT` identifies the
service. Consumers crossing user scope should also require trusted `_UID`.
Identifier alone is not trusted provenance. Add `UMBRIEL_RUN_ID=...` for one run.
Use `--all` for long values; journal JSON can represent duplicate/binary fields
as arrays. Do not equate native opaque `__CURSOR`, run/seq, source epoch and ZK's
synthetic counters. Follow's default tail policy is journalctl's, not a complete
history contract.

Layer initialization probes the journal socket and fails explicitly. Stock
`on_event` ignores per-send errors; neither emission nor readback acknowledges
durable persistence. Rate limiting/retention can lose data without an application
gap. The [native smoke test](tests/journal.rs) checks readback visibility and
field fidelity, not storage durability.

## Cancellation and bounded output

A dedicated OS thread owns synchronous tracing I/O. The main runtime sends through
a 64-record bounded channel; each record allows at most 1 MiB of owned string data.
Oversized records, queue overload and closed workers fail explicitly. This bounds
the queue independently of the native observer's existing byte/count budgets.
No implicit dropped-event continuation or automatic queue pruning is used.

SIGTERM/SIGINT select ahead of observation. The application emits stopping, closes
the source and drains the output worker within `--shutdown-timeout-ms` (3000 by
default). A stuck writer thread is detached when that deadline expires and the
process exits nonzero; pending output is not guaranteed delivered. It cannot block
the async cancellation path. Tests cover idle/pending dwell and a deliberately
full stdout pipe. Normal synchronous formatter writes still have the stock layer's
error-reporting limitations.

## Install and operate the dedicated unit

```sh
cargo install --path crates/activity --root "$HOME/.local" --locked
install -Dm644 crates/activity/systemd/umbriel-activity.service \
  "$HOME/.config/systemd/user/umbriel-activity.service"
systemd-analyze --user verify "$HOME/.config/systemd/user/umbriel-activity.service"
systemctl --user daemon-reload
systemctl --user enable --now umbriel-activity.service
systemctl --user status umbriel-activity.service
```

The unit uses the existing `umbriel-session.target` stop/start lifetime and orders
after `umbriel.service`. Ordering does not establish socket readiness. The user
manager must already know `UMBRIEL_SOCKET` (or discovery's two fallback variables);
import the correct session value with `systemctl --user import-environment
UMBRIEL_SOCKET`, or set an explicit socket in a unit override. This named unit is
for one compositor session per user. Inspect existing units before installation.

Journal mode has no stdout activity copy (`StandardOutput=null`); pre-initialization
and process-exit errors can still reach journal via stderr. The service can be
stopped with `systemctl --user stop umbriel-activity.service` and disabled with
`systemctl --user disable umbriel-activity.service`.

This delivered slice uses **Type=simple**, with no READY/watchdog protocol yet.
`umbrs-activity-notify` retains the selected AM-design/Astra-xhigh-build sequence;
`umbrs-activity-unit` remains open for the complete notify/history acceptance.
Native history/follow remains `umbrs-activity-history`; the bounded dump uses the
in-progress native reader. OSC, session-idle/suspend inference and live reload are
not delivered.

## Verification and deployment evidence

See [implementation and deployment record](../../design/activity/runtime0.gpt6a.md)
for commands, actual unit/process identity and limits of the smoke test.
