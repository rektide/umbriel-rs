---
type: ImplementationReport
title: Useful window titles and native activity history
description: Completed local native history/follow, preserved interrupted work and the JSON Lines dump entrance, title diagnosis, scoped tests and read-only native evidence.
resource: design/activity/history0.gpt61s.md
status: implemented
generated:
  at: 2026-10-31
  by: model:openai/gpt-6.1-sol#high
sources:
  - resource: opencode://ses_e6ea26b94ffeu233qr5mGPYsvz
    title: Commissioned history and first-class title usability; recovered interruption and partial commits, completed local reader/viewer, checked native title fidelity and bounded cancellation without restarting the service
  - resource: ../../crates/activity/README.md
    title: Maintained runnable commands and current history/journal contract
  - resource: records0.gpt6a.md
    title: Selected field-first tracing and accepted-plus-metadata policy
  - resource: service0.gpt6a.md
    title: Original history scope and remaining unit-level native fault verification
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/read.rs
    title: Adapted exact seek-next-test, captured upper cut and endpoint validation mechanics
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/lib.rs
    title: Native byte enumeration, thread-bound ownership and budget precedent
  - resource: file:///home/rektide/src/cyclrs/source/journald/src/read/tests.rs
    title: Inspected deterministic native-position, anchor-loss, invalidation and budget fixtures
  - resource: file:///home/rektide/src/cyclrs/design/kubeless-current/journald-followup0.gpt6a.md
    title: Real second-consumer opportunity and limits of extracting Cyclrs's schema-specific public reader
  - resource: file:///home/rektide/src/cyclrs/design/kubeless-current/journald-research0.gpt6a.md
    title: Ownership, binary/repeated-field, native waiting and completeness qualifications
  - resource: file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md
    title: Existing Umbriel history ownership and explicit historical-to-live/cancellation requirements
  - resource: ../../crates/activity/tests/history_native.rs
    title: New separately opt-in read-only title/resume/cancellation witnesses
verification:
  method: Scoped Nextest, all-targets warning-denying Clippy, formatting, release build, native read-only comparisons and explicit document checks
  limits: No service installation/restart, compositor changes, journal writes, vacuum/rotation/access-loss injection, downstream acknowledgment or durability claim; full workspace/Polars checks were not needed
extensions:
  ticket: umbrs-activity-history
  related_work: [umbrs-activity, umbrs-activity-unit]
---

# Delivered outcome

```sh
# Already built in this checkout; reads the existing service's journal.
./target/release/umbriel-activity history --follow
./target/release/umbriel-activity history --output jsonl
./target/release/umbriel-activity dump --tail 100 > recent.jsonl

# Rebuild/run from source instead:
cargo run -p umbriel-activity -- history --follow
```

Human output shows original time, accepted/metadata, **title first**, app ID,
window ID, workspace and scratchpad. Structured fields retain original titles,
identity, times, source epoch and metadata's `accepted_seq`. No extra compositor
subscription is needed to view the running service, and replay never journals
itself. The [application guide](../../crates/activity/README.md#native-history-follow-and-export)
owns the detailed head/tail, time, boot/run, origin, cursor and cancellation policy.

## Why the service looked unhelpful

The title path was already intact: compositor Windows census → tracking reducer's
selected `ActiveWindow` → `Record` → `emit` → native `UMBRIEL_TITLE`. Selected title
changes after acceptance already produced full `activity.metadata`; geometry did
not restart dwell and brief focus changes were intentionally filtered.

The real presentation gaps were **generic MESSAGE** (`Umbriel activity observation`),
service `StandardOutput=null`, and no completed history viewer. A private initial
100-record sample contained 51 acceptances and 49 metadata updates; all 99 active
records had nonempty title/app fields. Omitted titles in the parent session's
summary were not evidence of missing producer fields.

The fix is a shared terminal-safe title-first summary for new producer MESSAGE,
foreground `--output human`, and default human `history`. Structured values are
not sanitized or truncated: only human strings escape controls. Synthetic socket
and formatter tests exercise title propagation, immediate metadata, multiline,
quotes, ESC/OSC and C1 without writing private/synthetic records into journald.

An already-running old binary still has its old MESSAGE, but **the new viewer is
useful immediately against its existing structured records**. No install/restart
was needed for the delivered viewer. Updating the producer's ordinary journal
messages would be a separate deliberate install/restart, not performed here.

## Native reader and recovery boundaries

- Local `systemd::journal` reader, not a journalctl parsing subprocess. Native
  fields are enumerated as byte pairs with threshold disabled; repeated/binary
  unrelated fields are not collapsed into lossy string maps. Required scalar
  duplicates/invalid UTF-8, bad shapes and unknown versions/kinds fail explicitly.
- Default trusted selection is current effective UID plus the service user unit
  and activity target. Boot/run narrow it; including foreground writers explicitly
  weakens the unit-origin requirement.
- `history` defaults to initial recent 50; `dump` retains its head-by-default
  `fields` + `delivery` JSON Lines surface. Shared tracing export is available as
  history JSONL/NDJSON/logfmt. There is no journal export destination.
- Exact exclusive lower cursors are selection-bound JSON tokens. Capture the
  finite inclusive upper cut, traverse, then revalidate endpoints. Budgets fail
  before finite-range publication, rather than truncate successfully.
- Follow reads bounded exact suffixes, including appends during export. Native
  file-set invalidation outside a range is reported and followed by exact-anchor
  validation: **invalidation alone is not data loss**. Missing cuts and in-range
  invalidation fail. No automatic reseed, pruning or durable checkpointing.
- A worker creates/owns native reading and synchronous export. Signal handling
  remains async, with cooperative cancellation and a bounded process-exit timeout
  even for a blocked stdout writer. A token is reported only after full-range
  export/flush; that is not downstream acknowledgment. Incomplete output can be
  replayed from the prior token with duplicates rather than skipped records.

This is not a whole-history completeness, access, durability, attention-duration
or crash-consistent acceptance claim. Known source gaps remain source events;
journal loss need not have a producer gap. The full unit's genuine controlled
rotation/vacuum/access-loss fault sequence still belongs to `umbrs-activity-unit`.

## Interruption recovery and cleanup

The original start was clean at `1f8224bc`. On resumption, the repository was
still `/home/rektide/src/umbriel-rs`: **no moved layout was found**. Another
session/user had committed `4b87e3c6` (`Add JSON Lines activity journal dump`) and
added journal-message assertions; the initial native reader/title work remained
uncommitted. The dump depended on that incomplete reader.

Preserved those changes deliberately rather than blanket-reverting or rewriting
their history:

| Checkpoint | Meaning |
|---|---|
| `4b87e3c6` | Retained pre-existing JSON Lines dump entrance; not silently replaced |
| `ab732287` | Explicit incomplete predecessor checkpoint: native reader/dependencies/title summary plus the additional journal assertions |
| `27c6245a` | Complete local reader, historical-to-live CLI, title display and targeted tests; move dump I/O off signal runtime while preserving its output shape |

The completion adds cursor/time/origin controls, checked export writes and
bounded cancellation to dump rather than leaving a separate synchronous reader.
No obsolete parallel backend was retained. The root observer's existing JSONL
default is preserved. Documentation and durable native witnesses complete this
series separately; checkpoints are evidence, not an assertion that every partial
predecessor compiled. No squash, push, `jj edit`, foreign-file deletion or Cyclrs
edit/extraction was performed. A shared cross-repository native-reader extraction
remains a later architectural decision, not a prerequisite for this usable tool.

## Actual verification

```sh
cargo nextest run -p umbriel-activity -p umbriel-tracking -p umbriel-state \
  --lib --bins --tests --locked
cargo nextest run -p umbriel-activity --test history_native \
  --run-ignored ignored-only --locked
cargo clippy -p umbriel-activity --all-targets --locked -- -D warnings
cargo fmt -p umbriel-activity --check
cargo test -p umbriel-activity -p umbriel-tracking --doc --locked
cargo build --release -p umbriel-activity --locked
```

Results: **37 scoped tests passed**, four native tests skipped by default;
**three read-only native tests separately passed**. The fourth, older native
writer test was **not run**. Doc-test selection contained zero examples. Clippy,
formatting and release build passed. Deterministic cases cover exact equivalent
cursors/nearest-entry rejection, captured-cut append handoff, missing upper,
invalidation, native errors, cancellation, limits, malformed/unknown/repeated
schema values, unrelated binary/repeated fields and long/multiline title fidelity.

Private native checks on October 31 matched **99 active titles** byte-for-byte
from a retained 100-record sample through native decoding and shared tracing JSONL;
all 99 were visible in human output. A later sample contained 64 acceptances and
36 metadata updates. No unrelated title contents are retained in this receipt.
An actual finite since/until selection yielded nine records within the stated
receive-time bounds. Native caught-up resume yielded zero new records without
duplication; foreign/expired cursors and a one-entry budget were rejected with no
partial stdout. The positive new-suffix handoff proof is deterministic, not an
invented native append witness. Empty-follow and all three output modes shut down
on SIGTERM; unread full stdout caused an explicit bounded shutdown timeout.

Read-only manager inspection after the checks still reported active/running,
PID `1434669`, invocation `719a97e0b0ee42f3a9f519cd7a4ee479`, `NRestarts=0`.
Cancellation of the delegated agent did not cancel the live service. No journal
rotation, service stop/restart, privileged setup, permission changes or vacuum
was used to produce these checks.

## Connections and remaining decisions

This completes the concrete Umbriel consumer proposed by
[Cyclrs's follow-up](file:///home/rektide/src/cyclrs/design/kubeless-current/journald-followup0.gpt6a.md#3-share-native-mechanics-through-a-real-second-consumer)
and follows the older [ZK survey's ownership/handoff advice](file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md#recommended-next-use).
Their native cursor/budget/thread lessons are adapted locally; their domain codecs
and unbounded parser buffers are not imported. The result adds real cancellation
and viewer evidence without claiming the separate durable-reducer checkpoint or
shared-reader extraction opportunities are completed.

Keep or revert the coherent completed series after review; no blanket revert was
inferred from the user's tentative suggestion. Installation/restart of the producer,
notify/watchdog, full unit fault/soak evidence, OSC/session-awareness and automatic
checkpoint persistence remain separate decisions/work items.
