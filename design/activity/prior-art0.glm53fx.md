---
type: Research
title: Prior journald, tracing, and OSC session work across local projects
description: Hidden-aware survey of ~/src for reusable journald write/read code, tracing subscriber conventions, and OSC 3008/7501 session-awareness designs, distinguishing implemented code from proposals.
resource: design/activity/prior-art0.glm53fx.md
status: draft
author: model:zai-coding-plan/glm-5.3-flash#max
date: 2026-10-18
sources:
  - resource: opencode://ses_eaedd30e5ffeCrYXfObTLcucY5
    title: Commissioned read-only cross-project survey; discovered and read the cyclrs journald stack, systemd OSC 3008 implementation, term-preserver and opencode-status OSC work, and subscriber precedents; wrote and committed this note
  - resource: service0.gpt6a.md
    title: Commissioning activity-service design whose journald and re-emission shape this survey equips
  - resource: records0.gpt6a.md
    title: Field-first tracing-native record direction this survey must not contradict
  - resource: /home/rektide/src/cyclrs/telemetry/journald/README.md
    title: Implemented tracing-journald recorder with direct Dispatch and readback visibility
  - resource: /home/rektide/src/cyclrs/source/journald/README.md
    title: Implemented bounded native journal reader contract
  - resource: /home/rektide/src/cyclrs/design/architecture-refresh/journal-syn0.gpt6astra.md
    title: Prior cross-model journald research synthesis and corrected claims
  - resource: /home/rektide/src/systemd/src/shared/osc-context.c
    title: Upstream systemd OSC 3008 emitter reference implementation
  - resource: /home/rektide/src/term-preserver/design/tracking/osc-protocols.glm53m.md
    title: OSC 9;4 and OSC 7501 semantics, lifetimes, and limits brief
  - resource: /home/rektide/src/opencode-status/design/osc-status/README.md
    title: OSC 7501 client status registry research home, delegation paused
extensions:
  ticket: umbrs-activity
  related_work: [umbrs-activity-records]
scope: "Read-only research; no other repository was modified and no implementation is claimed."
---

# Prior art: journald export, journal reading, and OSC session awareness

Survey request: find reusable code, prior designs, crates, and conventions for
tracing/journald output, journal reading, and OSC 3008 / OSC 7501
session-awareness across `/home/rektide/src`, including hidden `.design` and
`.doc` directories. Everything below was read in place; each claim names its
file. Implemented code and proposals are labeled separately.

## Headline findings

1. **cyclrs already solved the hard journald problems** this project faces:
   stock `tracing-journald` export with mandatory visibility, and a bounded
   native journal reader for history. Its crates are the strongest reuse
   candidates, but its *record schema* conflicts with the field-first record
   direction chosen in [records0](records0.gpt6a.md) — reuse techniques, not
   the envelope.
2. **OSC 3008 is implemented upstream in systemd** (C, plus a shell prompt
   emitter); there is no local Rust emitter. OSC 7501 work exists in two
   places — term-preserver (proxy-observer semantics brief) and opencode-status
   (client-side reporting research, paused) — both proposals, no shipped code.
3. No project in the tree implements JSONL/logfmt/OSC as tracing *layers* yet;
   the closest implemented subscriber precedent is tracehouse
   (`subscriber-arrow-ch`), which owns the conventions for composing a custom
   subscriber crate.

## Implemented code

### cyclrs — journald write side (`/home/rektide/src/cyclrs`)

`telemetry/journald/` — **implemented, live-tested**. README declares "Stock
`tracing-journald` 0.3.2 … followed by bounded complete event read-back":

- `src/dispatch.rs` (83 lines): direct `tracing::Dispatch::event` submission via
  a private `DefaultCallsite`, bypassing both macro `STATIC_MAX_LEVEL` and the
  host's diagnostic subscriber; field values are raw strings, never
  Debug-rendered JSON. This is the local answer to "mandatory recording must not
  follow optional diagnostic filtering".
- `src/visibility.rs` (93 lines): `record_with` submits, then scans a bounded
  retained range for the exact event (whole-range validation, cumulative
  entry/byte budgets, deadline starting *after* the blocking submission). README
  is explicit that readback establishes **reader visibility, not durability**,
  and that readback failure is not a no-write receipt.
- Field codec lives in `domain/event/src/journal.rs`: nine canonical
  `CYCLRS_*` fields (`RECORD`, `EVENT_FORMAT`, `EVENT_ID`, `EVENT_SCHEMA`,
  `PROVIDER`, `SOURCE`, `TARGET`, `EVENT_TYPE`, `EVENT_JSON`) — a
  canonical-JSON-payload envelope with structured identity fields alongside.

`telemetry/event/README.md` and `telemetry/console/README.md` are **proposals
(sketch, unimplemented)**: the typed emission facade's acceptance API is called
"unresolved — ordinary `tracing` callbacks do not return per-event durable
append results", and console is kept a strictly optional view so that "a broken
diagnostic pipe must not be interpreted as a failed or safe-to-retry" operation,
with a warning against stdout-captured duplicate journal writes under systemd.

### cyclrs — journald read side

`source/journald/` — **implemented**. `src/lib.rs` (317 lines) + `src/read.rs` +
`src/record.rs` on the `systemd` crate 0.10.1 `journal` feature (libsystemd
bindings; the only local Cargo.toml using that feature):

- `Feed` selects by claimed provider/target plus trusted `_UID` and optional
  `_SYSTEMD_UNIT`/`_SYSTEMD_USER_UNIT`; README stresses claimed fields never
  authenticate origin — `SYSLOG_IDENTIFIER`-style names are not trusted fields
  (same correction recorded in the research synthesis below).
- `FeedCursor` = opaque native cursor + exact feed; exclusive resume validated
  through `seek_cursor` → `next` → `test_cursor`, never string comparison.
- `read_bounded` captures an inclusive finite **upper cut before traversal**;
  `ReadLimits` defaults 1,024 entries / 4 MiB copied bytes / 32 KiB events /
  2 s between native ops; budgets fail rather than return partial replay.
- Documented upstream pitfalls with working mitigations: `set_data_threshold(0)`
  plus `restart_data`/`enumerate_data` to avoid the convenience map's lossy
  UTF-8 and duplicate-key overwrites; `wait` overflow validation because
  systemd 0.10.1 converts durations via unchecked u64 microseconds and uses
  `u64::MAX` as its infinite-wait sentinel; `process()` invalidate detection.

`cmd/journal/` (`cyclrs-journal-probe`) — **implemented** small CLI verbs
(`record`, `read`, `init`, `advance`, `inspect`) exercising both sides across
real systemd service cycles; `deploy/systemd/probe/journal-waves.sh` is the
native-verification convention (three cycles of a transient user unit).

### cyclrs — prior journald research

`design/architecture-refresh/journal-research0.glm53fx.md`,
`journal-research0.gpt6luna.md`, `journal-research1.glm53fx.md`, and parent
synthesis `journal-syn0.gpt6astra.md` — source-grounded comparison with a
corrected-claims table. Consequences that transfer directly to umbriel:

- Stock `tracing-journald` discards per-event send results; its constructor
  socket probe is a useful *startup* failure only. Fine for observational
  activity records; do not promise durability from it.
- "Stock layer + typed full read-back" vs "strict sender + read-back" are
  alternative costs for the same visibility contract — and the synthesis
  warns "do not commission a custom native writer merely because the stock
  callback lacks a return value" when records are not effect-authorizing.
- Corrections worth carrying: `SyncIntervalSec=0` does not tighten durability;
  suppression markers and stored sequence numbers are not a completeness
  protocol; an older readable snapshot never certifies a lost suffix.

### tracehouse — custom tracing subscriber conventions (`/home/rektide/src/subscriber-arrow-ch`)

**Implemented** Rust tracing → ClickHouse via Arrow (`crates/{subscriber,
schema,encode,batch,writer}`). Not journald, but this is the family's one
shipped custom-subscriber codebase: startup configuration with environment
layering (`doc/configuration/README.md`), delivery accounting with explicit
outcome receipts, scoped runnable examples (`crates/subscriber/examples/
direct.rs`), and diagnostics kept distinct from delivery health. If umbriel
builds JSONL/logfmt/OSC as composable layers, these are the local structural
conventions to imitate.

### term-preserver — implemented OSC parsing (`/home/rektide/src/term-preserver`)

`src/osc.rs` — **implemented** byte-at-a-time functional-core OSC parser state
machine (`Emit`/`Hold` classification, typed `OscCapture` slots) for titles,
icon, palette, fg/bg/cursor colors. Relevant only if umbriel ever *reads*
terminal OSC traffic; umbriel's session-OSC ticket is an emitter. Its
design/tracking directory is where the protocol semantics live (below).

### Adjacent, smaller

- `rk8s-rfuse3-iou/project/Cargo.toml` declares `tracing-journald = "0.3.1"`
  (workspace dep for `rkl`), but the only `journald` hit in `rkl/src` is a
  comment in `src/cri/cri_api.rs:354`; no wired subscriber found.
- `exp2span` (`/home/rektide/src/exp2span`) — **implemented** CLI converting
  opencode logs to OTLP traces (`tracing-subscriber` json + OTLP stack); a
  log→trace conversion precedent, library-shaped reuse is explicitly absent
  (empty `lib.rs`, noted in tassle's o11y doc).

## Prior designs and proposals

### OSC 3008 — upstream systemd implementation

`/home/rektide/src/systemd/src/shared/osc-context.c` and
`profile.d/80-systemd-osc-context.sh` — **implemented upstream** (git history
shows upstream commits touching osc-context.c; the `systemd-*` variants are
workspaces of the same tree, not local patches). Generates OSC 3008
"Hierarchical Context Signalling" (UAPI.15 spec, uapi-group.org) open sequences
for types `boot`, `container`, `vm`, `elevate`, `chpriv`, `subcontext`,
`session`, `service` — including a `session` type with `user=`, `hostname=`,
`machineid=`, `bootid=`, `pid=`, `pidfdid=` identity pairs and escaped values.
The shell script is the prompt-side emitter convention; the C file is the
reference for payload shape and `xescape(';\\')` escaping. No Rust emitter
exists locally; umbriel would implement its own emission from this reference.

### OSC 7501 — two proposal threads

- **term-preserver** `design/tracking/osc-protocols.glm53m.md` (design only,
  `scope: "Design research only; no implementation"`): complete OSC 7501
  Program Status Protocol brief at spec revision 0.4 — key=value pairs,
  base64 free text, per-`id` record tree, states `idle|working|done|blocked|
  error` + `clear`, and **lifetimes as the load-bearing part**: terminals drop
  `working`/`blocked` at process exit or the next OSC 133 shell prompt;
  `done`/`error` survive. Also OSC 9;4 progress semantics and a support
  snapshot per terminal. Companion designs: `shell-tracking0.gpt6a.md`
  (shell-local tracking), `tmux-osc94.glm53fx.md` (per-pane propagation),
  `README.md` (ticket map under epic `tipre-tracking`).
- **opencode-status** `design/osc-status/` — research home `README.md` states
  delegation **paused at human direction**: a composable client-local registry
  (openproj entity registries) with an optional reference plugin reporting
  recently-active OpenCode sessions through OSC 7501. `research-brief0.gpt6a.md`
  owns the commission and subject/observation distinctions;
  `scout-lifecycle0.glm53fh.md` grounds status signals in OpenCode execution
  events (`session.execution.started|succeeded|failed|interrupted`), with the
  caveat that `succeeded` does not imply an empty inbox. No client code shipped.

### Other observability designs (context, not journald)

- `tassle/doc/o11y.md` — draft shape for a shared telemetry crate with "wide
  spans"; explicitly nothing built. Points at `exp2span`'s stack as prior art.
- `req-gov/doc/PLAN-tracing-decompose.md` — plan for per-limiter tracing spans
  (semaphore/governor/policy hierarchy); span-taxonomy example only.
- `arroyo-durable/doc/discovery/telemetry.md` — upstream-Arroyo telemetry study
  (third-party research), useful as a stack overview, not local convention.

## Strongest reuse candidates, ranked

1. **`cyclrs-source-journald`** for the `history` readback path: Feed/cursor/
   `ReadLimits` semantics, upper-cut capture, and the upstream-pitfall
   mitigations are exactly what `umbriel-activity history --follow
   --after-cursor` needs. Reuse as a pattern to port (crates are private-path
   deps inside cyclrs; check cyclrs workspace health before depending directly
   — its README records a Kubeless migration affecting builds).
2. **`cyclrs-telemetry-journald` techniques** — the direct-Dispatch pattern and
   the console/stdout duplication warning. For observational activity records,
   the plain stock `tracing-journald` Layer (already selected) is sufficient;
   reserve the readback visibility barrier for any future record whose loss
   would authorize an effect.
3. **tracehouse subscriber structure** for composing JSONL/logfmt/OSC as
   layers with explicit config, delivery accounting, and scoped examples.
4. **systemd OSC 3008 reference** (`osc-context.c` + profile.d script) for
   session-context emission payload shape and escaping.
5. **term-preserver OSC 7501 brief + opencode-status scouts** as the protocol
   and lifecycle grounding for `umbrs-activity-session-osc`; both are
   proposals — cite them as evidence, not as working code.

## Semantic conflicts and cautions

- **Record envelope vs fields.** cyclrs encodes a canonical JSON payload in one
  journal field plus structured identity fields; records0.gpt6a.md explicitly
  supersedes canonical-JSON payloads with a **field-first** shape. Port cyclrs's
  field-naming discipline (fixed canonical names, raw string values, no Debug
  JSON) without importing its `CYCLRS_*` envelope.
- **OSC 7501 lifetimes do not fit a detached service by default.** The spec's
  drop rules key on terminal process exit and OSC 133 prompt transitions; a
  systemd user service emitting activity status is not a shell child, so
  `working` records would be wiped at the next prompt unless umbriel targets
  `idle`/`done` semantics, its own `id` path, or renegotiates expectations.
  A standalone emitter also has no owner for the `OSC 7501 ; ?` detection
  reply — only a terminal may answer queries. Neither proposal thread resolved
  a service-side emitter.
- **Two different "session" subjects.** opencode-status sessions are OpenCode
  client sessions; OSC 3008 `session` is shell/login process context; umbriel
  activity is compositor window focus. Keep the subjects distinct in fields and
  records rather than overloading one session ID.
- **"All output through tracing" has a known cost boundary.** The cyclrs
  synthesis (journal-syn0, common-ground table) shows tracing callbacks discard
  send results and startup probes prove only socket presence. That is acceptable
  for activity observation, but the project should not describe journal export
  as acknowledged or durable without adopting the readback technique.
- **Version pinning.** cyclrs documents `systemd` 0.10.1 wait-overflow and
  lossy-convenience-map pitfalls; libsystemd 0.7.2 (pure Rust, cited in
  service0) has a different journal API surface. Whichever crate umbriel picks
  for reading, re-derive those mitigations rather than assuming they port.

## Search coverage note

Discovery used hidden-aware `fd`/`rg` over `/home/rektide/src` excluding
`target`, `node_modules`, `vendor`, `.git`, `.jj`, `archive`, and vendored
upstream trees (`systemd-import-btrfs` units, chromiumos archives). All
`design/`, `.design/`, `doc/`, `.doc/` directories (~400) were name-scanned for
journald/tracing/OSC terms; candidate paths above are the substantive reads.
 Vendored systemd/journald matches (units, tests) were excluded as upstream
code, with `/home/rektide/src/systemd` read deliberately as the OSC 3008
reference.
