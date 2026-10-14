---
type: Reference
title: Protocol benchmark fixtures
description: Sanitized live captures and deterministic scaled synthetic windows event frames for the protocol benchmarks, with provenance and exact sanitization rules.
resource: crates/umbriel-protocol/benches/fixtures/README.md
status: draft
generated:
  at: 2026-10-14
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ec7597a7affeFbugqOy35TD50A
    title: Single read-only live capture, sanitization, fixture generation, and the benchmark session
  - resource: ../../../design/performance/bench0.glm53m.md
    title: Baseline report consuming these fixtures
  - resource: ../../../design/protocol/commands0.glm53m.md
    title: Event frame shape, alphabetical key order, and query catalog
---

# Protocol benchmark fixtures

One-line NDJSON event frames, exactly as the compositor broadcasts them:
`{"data":<payload>,"event":"<family>"}` plus the terminating LF (keys in the
server's lexicographic order, so `"data"` is first). Decode benchmarks strip
the trailing LF, matching what `Framer::next_frame` hands a consumer.

## Live captures (sanitized)

`live-windows.json` and `live-workspaces.json` come from a single read-only
acquisition against the running compositor on 2026-10-14: two one-shot query
connections (`{"cmd":"windows"}`, `{"cmd":"workspaces"}`) on
`/run/user/1000/umbriel-wayland-0.sock`, each closed by the server after its
reply. No `subscribe`, no `msg`, no repeated connections. Raw replies are
preserved only in gitignored `.test-agent/performance-baseline/raw/` and are
never committed; their hashes are recorded in the
[baseline report](../../../design/performance/bench0.glm53m.md).

### What sanitization changes

- Rewritten, per character, content-blind: window `id`, `workspace`,
  `scratchpad`, `app_id`, `title`, `xdg_tag`; workspace `id`, `name`,
  `output`. ASCII lower → `x`, upper → `X`, digit → `d`; 2/3/4-byte
  codepoints → `α`/`雪`/`🦀`; punctuation, spaces, and control characters
  pass through, so byte lengths, escape sequences, and key structure survive
  exactly. The payload byte length is unchanged (8,347 B windows payload
  before and after).
- Nonzero `pid` values are remapped deterministically with the same digit
  count; `-1` stays `-1`.
- Kept verbatim: geometry (`x`/`y`/`w`/`h`), all booleans, `content_type`,
  `layout`, `index`, `named`, `occupied`, entry counts and ordering, and the
  active/focused census (windows: 1 active, 6 focused of 29; workspaces: 2
  active groups, 1 focused, of 8 across 2 output groups).
- Identical originals map to identical outputs, so window→workspace
  references stay consistent. Caveat: distinct originals with the same
  character-class shape collide (both real outputs sanitize to `XX-d`), so
  distinctness of same-shape identifiers is not preserved — irrelevant to
  decode cost, which depends on bytes, not identity.

## Scaled synthetic windows

`synthetic-windows-{10,50,200}.json` are deterministic, index-derived (no
RNG; stable output bytes) single-line event frames modeled on the live shape:
17 alphabetical keys, 32-character ids, one active window, one focused window
per workspace, negative/positive geometry, `pid: -1` every seventh entry, and
the escape/multibyte title mix the live session happened to lack — every
fourth title carries `"`/`\`/newline escapes, every fourth carries 2/3/4-byte
multibyte text.

## Inventory

| File | Bytes (incl. LF) | Entries | Escaped titles | Multibyte titles | SHA-256 |
|---|---|---|---|---|---|
| `live-windows.json` | 8,375 | 29 | 0 | 1 | `8b3c231ba36799f635568d4e7e096ef9f21b27f373297b11fea550bed3399c60` |
| `live-workspaces.json` | 1,111 | 8 | 0 | 0 | `88c8315a94b7bcf7ae5de7107959e39cba77dd8de9d66cba88133a63ea62f9db` |
| `synthetic-windows-10.json` | 3,126 | 10 | 3 | 2 | `e7516920e89df96a3cc8f1025ddc0b92488c310314c712dc310e3dfc6cbdf572` |
| `synthetic-windows-50.json` | 15,447 | 50 | 13 | 12 | `34861f427144d16fe0aa7017543d0457fdea5f464dccb54c44901381b5c49be0` |
| `synthetic-windows-200.json` | 61,750 | 200 | 50 | 50 | `2aac559c4b2139224706a11d0215e3910654b0dfdd3cf4b2f63ec8ca52d1752c` |

Full hashes, raw-capture hashes, and per-field length ranges live in the
baseline report and the gitignored scratch provenance record.
