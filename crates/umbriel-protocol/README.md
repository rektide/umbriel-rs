---
type: Reference
title: Umbriel protocol substrate
description: Implemented framing, checked Sonic envelopes, and the initial typed wire request surface.
resource: crates/umbriel-protocol/README.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ecbc463e2ffedTrt7LV5wGlDNq
    title: Fresh protocol implementation, source-checked selective Sonic validation, and malformed-Unicode token-boundary correction
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Selected Sonic and sans-I/O multi-crate responsibilities
  - resource: ../../design/protocol/commands0.glm53m.md
    title: Wire command and event catalog
  - resource: ../../design/protocol/review0.gpt6a.md
    title: Governing LF-inclusive request bound and blank subscription response
  - resource: ../../design/assessment/reuse0.gpt6a.md
    title: Session ownership and optional state and columnar boundaries
  - resource: https://docs.rs/crate/sonic-rs/0.5.10/source/src/serde/de.rs
    title: Resolved local source checked for validated lazy/discard paths and trailing-input enforcement
  - resource: https://docs.rs/crate/sonic-rs/0.5.10/source/src/parser.rs
    title: Resolved local source checked for string skipping and supplementary Unicode-escape validation needs
---

# Umbriel protocol substrate

This crate owns no socket, executor, request scheduler, or materialized state.

- `codec::Framer`: feed byte chunks with `push`, drain borrowed `next_frame`
  slices, and call `finish` at EOF. Frames exclude LF; empty frames survive.
  An incomplete EOF tail is an error even when complete frames remain queued.
  Input limit errors leave the stored input unchanged. Drain frames before
  appending a chunk that would exceed the buffered-input policy.
- `codec::Limits`: maximum frame bytes **excluding LF**, and maximum unconsumed
  buffered bytes **including LF**. Both are independent incoming client policy,
  not server response limits. Storage capacity survives consumption; compaction
  moves only unconsumed bytes. A scan cursor avoids repeated prefix scanning.
- `codec::decode<T>`: caller-selected Serde decoding with frame-bound borrowing.
  UTF-8 is checked before Sonic; typed parsing then checks structure, ordinary
  escapes and trailing input, including lazy/ignored values. Supplementary
  decoding checks only string tokens containing actual Unicode escapes, to reject
  invalid hex digits and surrogate pairs that Sonic's checked skip path accepts.
  Token selection checks all four hex digits before advancing, so malformed
  digits cannot hide quotes or backslashes; incomplete tokens return errors.
  Frames without `\u` byte sequences skip token selection entirely. This avoids a redundant
  full syntax pass and reparsing every key/string, without a generic JSON tree.
  Caller `Deserialize` code may run before supplementary validation fails:
  **stage application state until decoding returns success**.
- `envelope::Message`: order-independent `Ok(LazyValue)`, `Err(Cow<str>)`,
  `Event { name, data }`, or `Blank`. Present `ok:null` is success; `Blank` is
  **not** success. Significant duplicate/conflicting keys fail; additive fields
  are accepted only after validation. Unknown family names remain available.
  Unused additive values follow Sonic's checked discard path rather than retain
  lazy storage; escaped additive values no longer create owned lazy copies.
  Unescaped strings/payloads borrow where Sonic supports it; escaped strings may
  own storage. Promote needed data before reusing the receive buffer.
- `request::Request`: seven read-only queries, raw `msg` action strings,
  nonempty known-family subscriptions, output creation/destruction, and a raw
  command escape hatch. No complete action grammar or response models are claimed.
- `codec::encode` / `Request::encode`: replace reused caller byte storage with
  Sonic JSON plus LF. A bounded sink enforces 65536 bytes **including LF**.
  Failed encodes empty the destination while retaining capacity. Sonic's safe
  `BufferedWriter` adapter uses temporary serializer scratch, which is not an
  allocation ceiling for arbitrarily large source strings.

The future session crate must interpret blank frames in subscription context,
retain the one pending command's ownership across cancellation, handle partial
writes and EOF, and route events independently of ordinary replies. Full models,
typed action grammar, transport, state, Arrow, and Polars remain separate work.

Verification commands: `cargo fmt --all --check`, `cargo nextest run --workspace`,
and `cargo clippy --workspace --all-targets -- -D warnings`.
