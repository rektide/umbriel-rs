---
type: Report
title: Reusable tracking and live activity-service delivery
description: Implementation boundaries, tests, native field readback and dedicated user-service deployment evidence for the first tracing-native runtime slice.
resource: design/activity/runtime0.gpt6a.md
status: verified
generated:
  at: 2026-10-22
  by: model:openai/gpt-6-astra#medium
sources:
  - resource: opencode://ses_e9a6a958affehqIA6mCZZF154C
    title: Implemented and tested reusable tracking and tracing activity service; installed dedicated unit and inspected native journal and lifecycle
  - resource: records0.gpt6a.md
    title: Selected field-first tracing-journald and accepted-plus-metadata direction
  - resource: service0.gpt6a.md
    title: Original service scope and separately sequenced notify/history work
  - resource: ../../crates/tracking/README.md
    title: Delivered reusable API and freshness contracts
  - resource: ../../crates/activity/README.md
    title: Delivered journal field/filter contract, output bounds and deployment commands
  - resource: file:///home/rektide/src/zooming-kittens/design/longcat/umbriel-control-stream0.gpt6a.md
    title: Composition extraction and immediate versus delayed consumer distinction
  - resource: file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md
    title: Cyclrs and Phyllited precedents retained under native history ownership
verification:
  method: Scoped nextest and Clippy, opt-in native journald field test, systemd unit verification, actual install/start/stop/start and trusted-unit journal readback
  limits: No compositor restart or mutation by this implementation session, no native replay decoder, no notify/watchdog, no journal outage or retention injection, no durability or long-term soak claim
extensions:
  ticket: umbrs-stream-tracking
  related_work: [umbrs-activity, umbrs-activity-records, umbrs-activity-journal, umbrs-activity-cli, umbrs-activity-shutdown, umbrs-activity-unit, umbrs-activity-history]
---

# Delivered slice

The [tracking library](../../crates/tracking/README.md) now owns typed frame
classification, Inventory installation/freshness and a reusable queue-before-timer
activity loop. Inventory remains the sole native row owner. The client examples
use this library; immediate control consumers do not pass through the five-second
activity policy. `StableActiveReducer::observe_changes` owns metadata eligibility.

The [activity library/application](../../crates/activity/README.md) supplies
field-first tracing helpers, JSONL/NDJSON, defined logfmt, tracing-journald, native
reconnect configuration, signals and bounded output/shutdown. Synchronous tracing
I/O runs on a dedicated bounded worker, so a blocked journal/stdout writer cannot
hold the cancellation-aware runtime indefinitely. Timeout exits explicitly and
may leave output undelivered.

Root dependency verification initially failed because the state manifest required
the sibling memory leaf at `=0.1.0` while that checkout is `0.6.0`. The exact pin
was updated to `=0.6.0`, not widened. Existing `memory_view`, checked Replacement,
Writer replacement and receipt calls compiled unchanged; state and example tests
passed. Root Cargo.lock captures that local version and the activity dependencies.

## Verification results

- `cargo nextest run -p umbriel-activity -p umbriel-tracking -p umbriel-state
  -p umbriel-client --all-targets`: **76 passed**, one opt-in journal test skipped.
- `cargo nextest run -p umbriel-activity --test journal --run-ignored only`:
  **1 passed** against the real user journal. Four synthetic entries, exactly one
  each, preserved active/metadata/none/gap fields, original time, `accepted_seq`,
  newline/quotes/backslash and an 8 KiB title. INFO mapped to priority 5.
- `cargo clippy -p umbriel-tracking -p umbriel-activity --all-targets -- -D warnings`:
  passed.
- Paused-clock socket tests establish queued transitions and terminal observer lag
  before due acceptance, plus timer-only emission. Pure tests cover metadata/no-op/
  geometry/pre-acceptance churn, A–B–A, none and invalidation.
- Process tests establish SIGTERM while dwell is pending, source socket closure,
  no invented acceptance, explicit initial-connect failure and bounded nonzero exit
  when a 512 KiB record blocks an unread stdout pipe.
- Subsequently added configured reconnect/fresh-dwell/exhaustion and zero-attempts
  disabled-policy process tests. `cargo nextest run -p umbriel-activity --test process`
  passed all **5 process tests**, bringing scoped distinct coverage to **78 tests**
  plus the separately executed native-journal probe.
- `systemd-analyze --user verify crates/activity/systemd/umbriel-activity.service`
  returned success after binary installation. It emitted an existing unrelated
  duplicate `org.freedesktop.Notifications` bus-name warning for `dms.service`.

## Actual installation and observations

Commands are in the [operating guide](../../crates/activity/README.md#install-and-operate-the-dedicated-unit).
Executed `cargo install --path crates/activity --root /home/rektide/.local --locked`,
installed the checked-in unit, reloaded the user manager and enabled/started it.

- Binary: `/home/rektide/.local/bin/umbriel-activity`.
- Unit: `/home/rektide/.config/systemd/user/umbriel-activity.service`.
- Enabled link: `~/.config/systemd/user/umbriel-session.target.wants/umbriel-activity.service`.
- Before installation: activity unit `LoadState=not-found`; native compositor
  active PID **12558**, start **2026-10-19 17:10:55 UTC**.
- User manager already supplied `UMBRIEL_SOCKET=/run/user/1000/umbriel-wayland-0.sock`
  and active `umbriel-session.target`.
- First activity run: PID **1395121**, start **2026-10-22 21:57:30 UTC**,
  run ID `c21b8c62-2b1c-40a6-aab1-2e7fb6d3e80c`. Native journal showed epoch/ready
  at offset 0 and first acceptance at **5000 ms**, with `dwell_ms=5000`.
  Subsequent real focus observations produced further accepted episodes.
- Stopped only the dedicated activity unit: `Result=success`, `ExecMainStatus=0`,
  and `source.stopping` seq 8 at offset 176003 ms. Compositor PID remained 12558.
- Started activity again: PID **1434669**, start **2026-10-22 22:00:36 UTC**,
  run ID `e3570f81-ea7d-4942-9b87-bc687ec378d0`, epoch/ready then acceptance at
  **5002 ms**. Final inspected state: active/running, **NRestarts=0**.
- Trusted-unit journal selection contained no duplicate `(run_id, seq)` pairs.
  All application data used native `_TRANSPORT=journal`, `TARGET=umbriel::activity`
  and `UMBRIEL_*` fields. No stdout activity copy exists in this service mode.

This process is left running and enabled for the user to consume. Its accepted
metadata path has synthetic native readback coverage; the live deployment probe
did not deliberately change a window title. The compositor was not restarted.

## Remaining native work

The supplied unit is deliberately the running **Type=simple** slice.
`umbrs-activity-notify` retains AM design followed by Astra-xhigh implementation;
typed `source.ready`, `source.gap` and `source.stopping` provide lifecycle hooks.
`umbrs-activity-unit` remains open for full notify/history/lifetime acceptance.

`umbrs-activity-history` owns native schema decoding, trusted selection, exact
cursor validation, bounded finite/follow handoff and export-only re-emission.
The [survey](file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md)
connects Cyclrs cursor/budget mechanics and Phyllited repeated/binary-field oracle
ideas. No second ZK history undertaking or canonical JSON blob was introduced.
Record-level re-emission is tested; native journal-to-Record replay is not delivered.
