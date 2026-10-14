---
type: Report
title: Known model enum allocation cleanup
description: Before-and-after counting-allocator measurements of borrow-aware enum classification, without a throughput claim.
resource: design/performance/enum-allocations0.gpt61s.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec7a72fc6ffedFA9NaqNKCxaP5
    title: Implement borrowed enum classification, verify owned unknown-buffer reuse, and measure identical allocation fixtures before and after
  - resource: bench0.glm53m.md
    title: Corrected baseline and attribution of the transient known ContentType allocations at bc7a3223
  - resource: ../../crates/umbriel-protocol/src/model/vocabulary.rs
    title: Changed string_enum conversion and Serde visitor at b3ff0980
  - resource: ../../crates/umbriel-protocol/tests/models.rs
    title: Known, escaped, borrowed, transient, owned, unknown, and wrong-type coverage
  - resource: ../../crates/umbriel-protocol/benches/allocations.rs
    title: Unchanged counting allocator, armed-call sampling, and corrected borrowed projections
---

# Known model enum allocation cleanup

This is an addendum to the [corrected baseline](bench0.glm53m.md), not a
replacement for its measurements. Before: **`bc7a3223`**. Focused code
correction and after measurement: **`b3ff0980`**.

The `string_enum` macro now classifies borrowed or transient text before
allocating. Known spellings construct the existing unit variant directly;
unknown spellings remain `Unknown(String)`. An owned `visit_string` reuses its
incoming buffer for unknown values. `From<&str>` likewise classifies first.
Public names, ownership, serialization, and wire semantics are unchanged.

## Matched measurements

Ran the unchanged binary both before and after:

```sh
cargo bench -p umbriel-protocol --bench allocations
```

Both runs used the same release/bench defaults and fixture files. `RUSTFLAGS`,
`CARGO_ENCODED_RUSTFLAGS`, and bench opt-level/LTO/codegen-unit environment
overrides were unset. Toolchain: `rustc 1.101.0-nightly (5c543b0b8 2026-09-29)`.
Fixture hashes match the baseline:

- `live-windows.json`: `8b3c231ba36799f635568d4e7e096ef9f21b27f373297b11fea550bed3399c60`
- `synthetic-windows-50.json`: `34861f427144d16fe0aa7017543d0457fdea5f464dccb54c44901381b5c49be0`

Allocator calls and requested bytes per decode; both full-model paths produced
the same counts:

| Fixture / paths | Before | After | Reduction |
|---|---|---|---|
| Live windows (29), `model_direct` and `envelope+typed` | 149 calls / 13,917 B | 120 calls / 13,801 B | 29 calls / 116 B |
| Synthetic-50, `model_direct` and `envelope+typed` | 269 calls / 29,282 B | 219 calls / 29,057 B | 50 calls / 225 B |

For every row, the first armed call and all 200 subsequent samples agreed
(median = minimum = maximum). This removes one transient allocation for each
known content type. No benchmark code or projection borrow annotations changed.

Control paths were unchanged: envelope-only **0 calls / 0 B**; borrowed selected
projection **4 / 3,360 B** live and **21 / 7,534 B** synthetic; deliberately
owned projection **62 / 4,795 B** live and **108 / 9,792 B** synthetic. Cold
framing remained **1 / 134,000 B** and reused framing **0 / 0 B**.

## Remaining costs and checks

The live model's remaining 120 calls fit the baseline's attribution: **116
owned model strings** (four nonempty strings per window) plus **four vector
growth calls**. Empty string fields do not allocate. This decomposition is
source/count attribution, not new per-field allocator instrumentation.
Synthetic input still pays for owned model text, vector growth, and Sonic's
unescaping scratch growth. Unknown enum text must still be owned; escaped known
text may grow parser scratch even though enum classification no longer allocates.

No timing benchmark was rerun, so **no throughput improvement is claimed**.
There were no live compositor queries.

Verification passed: **62/62 protocol tests**, `cargo fmt -p umbriel-protocol
--check`, and `cargo clippy -p umbriel-protocol --all-targets -- -D warnings`.
Model tests exercise all enum vocabularies through borrowed, transient, and
owned Serde inputs, escaped known spellings, exact unknown Unicode roundtrips,
owned unknown-buffer reuse, and wrong-type failures.
