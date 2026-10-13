---
type: Handoff
title: umbriel-rs session handoff — state, decisions, and next steps
description: Where the project stands after the founding session: implemented protocol foundation, committed research corpus, beads tracking, governing decisions, known pitfalls, and the critical path forward.
resource: design/handoff/handoff0.glm53m.md
status: draft
generated:
  at: 2026-10-13
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Founding session — research, architecture, assessment, protocol implementation, and ticketing
  - resource: crates/umbriel-protocol/README.md
    title: Implemented foundation surface and verification commands
  - resource: ../protocol/commands0.glm53m.md
    title: Wire catalog the implementation is built against
  - resource: ../protocol/review0.gpt6a.md
    title: Source corrections that govern over the original catalog
  - resource: ../assessment/reuse0.gpt6a.md
    title: tmux-then-kubeless reuse assessment integrated into the architecture
  - resource: ../architecture/architecture1.gpt6a.md
    title: Current Sonic-first multi-crate direction
---

# Session handoff

## What this project is

A high-performance Rust client library for the **Umbriel compositor's IPC
protocol** (noctalia-dev, C++23/wlroots, v0.1.0). Direction selected by the
user: **Sonic-first** (sonic-rs, no backend comparison framework), **sans-I/O
core**, **whole protocol library** (not a single-use tool), **multi-crate
workspace**. The 750 ms stable-active-window stream is one example consumer,
not the organizing architecture.

## Repo map

| Path | Contents |
|---|---|
| `crates/umbriel-protocol/` | Implemented foundation: codec, envelope, request modules + tests + crate README |
| `design/protocol/` | `commands0.glm53m.md` (wire catalog) + `review0.gpt6a.md` (**corrections govern**) |
| `design/architecture/` | `architecture1.gpt6a.md` (current) + `architecture0.gpt6a.md` (predecessor; owns 750 ms dwell semantics) |
| `design/assessment/` | `brief0` (commission) + `reuse0.gpt6a.md` (completed tmux→kubeless assessment) |
| `design/performance/` | `json0.gpt6a.md` (Sonic/simd-json source findings; benchmarks deferred) |
| `design/handoff/` | This document |
| `.beads/issues.jsonl` | Ticket export — **dolt is source of truth**, export after changes |
| Root `README.md` | Navigation and implemented-foundation summary |

## What is built (commits a7269403 → cf065ea0)

`umbriel-protocol` provides:

- `codec::Framer` — bounded reusable NDJSON framing, scan cursor, compaction,
  blank frames preserved, strict truncated-EOF, atomic limit rejection.
- `codec::decode/encode` — checked Sonic decode (UTF-8 precheck, validated
  Unicode escapes incl. the malformed-digit boundary fixed in `cf065ea0`);
  encode into reused storage enforcing **65536 bytes including LF**.
- `envelope::Message` — order-independent `Ok/Err/Event/Blank`; `ok:null` is
  success; duplicate significant keys rejected; unknown families preserved.
- `request::Request` — 7 queries, `msg`, `subscribe`, output ops, raw hatch.

Checks green at `cf065ea0`: `cargo fmt --all --check`,
`cargo nextest run --workspace` (14 + regressions),
`cargo clippy --workspace --all-targets -- -D warnings`. Crate is
`#![forbid(unsafe_code)]`.

**Important semantic contract:** caller `Deserialize` code may run before
supplementary Unicode-escape validation fails — applications must stage state
until `decode` returns success (documented in the crate README).

## Governing protocol facts (the short list)

Full detail in `commands0` + `review0`. The ones that bite:

1. Non-subscribed connections are **strictly one-shot** (reply → server closes).
2. **Subscribed connections do support sequential command/reply reuse**
   (source + live-verified), but **no pipelining**, **no request IDs**, one
   outstanding request — ever.
3. Request limit is **65536 bytes including LF** (server checks input size
   before finding the newline). Oversize **poisons subscribers** (input never
   cleared) → reconnect after that error.
4. A `keyboard_layout`-only subscribe with no keyboard yields a **bare LF** —
   the framer exposes it; only the session layer may interpret it. It is never
   command success.
5. Every family payload is a **full snapshot** (replace, never merge). Initial
   and re-subscribe responses can **duplicate** broadcast state — don't assume
   byte-uniqueness per subscriber.
6. No version negotiation. Six families at 0.1.x. Tolerate added fields and
   unknown families.
7. `active` (seat-global, 0-or-1) ≠ `focused` (per-workspace memory).

## Decisions with authority (do not relitigate without the user)

- **Sonic selected**; simd-json remains a future tool, not a prerequisite.
- **Flash work is cancelled and deleted** — user explicitly removed it; do not
  recover those sessions' code.
- **Agent delegation pattern** (user-directed): **Sol medium = high-value
  implementation**, **g = routine/rest**, Astra medium was used only for the
  explicitly commissioned assessment. **No more Flash.**
- **Write agent prompts in normal spaced prose** — the user asked for readable
  instructions, not dense run-on prompts.
- tmux ownership law adopted: a sent request retains reply ownership across
  waiter cancellation until its reply drains or the connection closes.
- `kubeless-view-memory` is the reuse candidate for state (per-family
  replacement maps); never block socket draining on downstream publication.
- No automatic reconciliation/prune behavior; observation ≠ control.

## Outstanding work

Tracked in beads (see `bd list`, export committed at `8f494173`). Critical
path: **`umbrs-protocol-models` → session roles/command-slot → subscription
driver → 750 ms focus example**. Priorities and dependency edges live there;
open tickets carry context notes pointing at the governing design sections.

Immediate loose end: **`umbrs-protocol-review`** — the g verification never
ran because that provider hit its 5-hour usage limit (resets 2026-10-13
17:59). Re-dispatch when capacity returns.

## Known pitfalls

- **Polars**: Rust crate at the archived revision is **0.55.1** (not "2.0" —
  that's the user's shorthand for the lazy-frame workflow). Its `RecordBatch`
  is Polars-native, *not* Apache `arrow-array::RecordBatch`; `collect_batches`
  is a **blocking** `Iterator<DataFrame>` behind the `async` feature. Run
  queries off the receive owner. Pin versions; compile a proof first.
- **kubeless packages are unpublished** (workspace-local). A dependency needs
  a reviewed path/git pin, not a registry version.
- **sonic-rs 0.5.10 quirks** (found the hard way, now handled + tested): lazy
  path can inspect strings before final UTF-8 check (we precheck); checked
  skip path accepts invalid `\u` hex digits (we validate digits before
  selecting token boundaries — see `cf065ea0`).
- **zooming-kittens is niri-implemented.** Its Umbriel doc is research
  precedent only, not an existing client.
- **Live compositor** runs at `/run/user/1000/umbriel-wayland-0.sock`. Probes
  this session were read-only; keep it that way unless the user says otherwise.
- **jj discipline**: commit explicit owned paths, no squash, never push.
  Agent work should commit only its own files to avoid racing the parent.

## Environment pointers

- Compositor checkout: `~/a/noctalia-dev/umbriel` (= `~/archive/noctalia-dev/umbriel`), pinned `7666af0a`.
- Archived parsers: `~/archive/cloudwego/sonic-rs` (f63d185), `~/archive/simd-lite/simd-json` (0662a83).
- Archived Polars: `~/archive/pola-rs/polars` (31df0ae, Rust 0.55.1); Arrow: `~/archive/apache/arrow-rs` (00fadf9, 59.2.0).
- The shared ChatGPT answer that seeded the perf direction: `~/umb.chat.url` (read in full this session; its "screencast family" and blanket "reusable connections" claims are wrong for this revision).
- Comparison projects: `~/src/tmux-interface-control`, `~/src/kubeless` — do not edit them.

## For the next session

1. Start from `bd list --status open --sort priority` and the crate README.
2. Read `design/protocol/review0.gpt6a.md` before trusting `commands0`.
3. Pick up `umbrs-protocol-models` (P1) or re-dispatch `umbrs-protocol-review`.
4. Delegation: Sol medium builds, g reviews/does routine work, prompts in
   spaced prose, background dispatch, explicit-path commits.
