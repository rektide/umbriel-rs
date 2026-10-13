---
type: Investigation
title: JSON decoding and allocation strategy for Umbriel
description: Source-checked Sonic and simd-json tradeoffs, allocation boundaries, and a workload-shaped performance experiment.
resource: design/performance/json0.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Read the shared performance discussion and inspect parser implementations to design a measurable Rust client
  - resource: https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09
    title: Rust JSON Protocol Speed — full shared answer recovered from the page's embedded data
  - resource: https://github.com/cloudwego/sonic-rs/tree/f63d1851de596fb9c53bf2a67ec84828c1d65c3c
    title: Sonic source, manifest version 0.5.10, README, performance and compatibility documentation
  - resource: https://github.com/simd-lite/simd-json/tree/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873
    title: Archived simd-json source, manifest version 0.17.0, README, buffer and tape APIs
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Prior snapshot-protocol and active-window research
---

# JSON decoding and allocation strategy

## Recommendation and evidence status

**Subsequent human direction:** Sonic is selected, and library development is
the priority. The [current architecture](../architecture/architecture1.gpt6a.md)
supersedes the comparative selection gate below. These source findings and the
experiment design remain available when a concrete performance question arises.

**Start with Sonic's checked, typed projection path. Compare it with equally
shaped serde_json and simd-json implementations before selecting a permanent
backend.** Preserve a path to simd-json's reusable tape for consumers that need
many views of the same message. The choice is a workload decision, not a library
leaderboard.

This is source research and an experiment design. No Umbriel decoder benchmarks
were run in this session. Manifest versions identify inspected checkouts; they
do not prove equivalence to the published crates with those version numbers.
The simd-json checkout was already archived locally; Sonic was fetched after
checking the archive. Exact revisions above bound the findings.

The [shared answer](https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09)
was read in full. Its central proposals survive inspection: typed projections,
one state owner, reusable byte framing, borrowed lifetimes, and independent
measurement of decoding versus socket round trips. Two refinements matter:
reusable parser buffers do **not** automatically mean a reusable tape, and lazy
field extraction does **not** automatically establish whole-frame validity.

## What each backend actually buys

| Candidate | Suitable job | Costs to measure |
|---|---|---|
| Sonic typed projection | One pass into a small known shape; most fields unwanted | String scratch, escaped selected strings, result containers |
| Sonic lazy envelope / array iteration | Route or defer payloads; select a few fields without constructing a DOM | Boundary scans, repeat scans, validation coverage, retained frame storage |
| simd-json buffered Serde | Typed decoding with reusable structural/string scratch and in-place unescaping | Full tape construction, fresh tape allocation in the inspected convenience path |
| simd-json `fill_tape` + tape traversal | Repeated projections of one frame; explicit reuse of tape and scratch | Full structural work even for a narrow query; lifetime/reset discipline |
| serde_json typed projection | Straightforward reference and small-message baseline | Escaped strings, parser scratch, result allocations |

### Sonic

The [README](https://github.com/cloudwego/sonic-rs/blob/f63d1851de596fb9c53bf2a67ec84828c1d65c3c/README.md#deserialize-struct)
describes direct typed deserialization rather than building a simd-json-style
tape first. This is compelling when a windows snapshot contains many fields but
the consumer wants only activation and identity. SIMD use differs by operation;
neither every instruction nor every field becomes SIMD work.

The inspected [deserializer](https://github.com/cloudwego/sonic-rs/blob/f63d1851de596fb9c53bf2a67ec84828c1d65c3c/src/serde/de.rs)
has private `scratch: Vec<u8>` initialized empty for a new deserializer
(lines 27–41). `from_slice` constructs one per call. Unescaped strings can use
`visit_borrowed_str`; escaped strings use `visit_str` over decoded scratch.
Thus `#[serde(borrow)] Cow<'a, str>` is useful, but escaped titles can still
allocate both parsing scratch and owned output. A reused receive vector alone
does not remove those costs. No caller-supplied scratch/reset API was found in
this inspected typed-deserializer surface.

`deserialize_ignored_any` calls `skip_one(true)` (lines 902–908): unused fields
are skipped with validation, not made costless. The convenience `from_slice`
path checks trailing input and final UTF-8 validity (lines 1300–1341). Preserve
that contract when replacing it with a custom visitor or seed; do not assume a
manually driven deserializer has the same finalization merely because its type
is the same.

`LazyValue` is useful for an envelope whose payload type is chosen after reading
`event`, including when `data` precedes `event`. A complete typed envelope parse
can establish the whole-frame contract before decoding selected payloads. That
may scan much of the payload twice. Direct windows-only decoding is a separate
candidate, not an inferior fallback.

[`get_many`](https://github.com/cloudwego/sonic-rs/blob/f63d1851de596fb9c53bf2a67ec84828c1d65c3c/src/lazyvalue/get.rs#L417-L468)
uses a reusable `PointerTree`, but returns a new
`Vec<Option<LazyValue>>`. It selects **known paths**, not “find the object whose
`active` field is true.” An array scan is still necessary for that predicate.
Build the pointer tree once when testing repeated fixed-path access.

The `get` implementation returns after finding the requested value and checks
UTF-8 only through the parser's current index (same file, lines 398–415). Do not
treat a successful selected-field lookup as proof that the entire frame,
including its suffix, was valid. Lazy iterator exhaustion and trailing input
need explicit examination too. Benchmark equivalent validation guarantees;
unchecked variants are not interchangeable with checked whole-frame decoding.

### simd-json: three reusable things, not one

1. **Receive storage:** caller-owned mutable bytes.
2. **Parser scratch:** `Buffers` holds string scratch, structural indexes,
   aligned input storage, and the stage-two stack.
3. **Tape storage:** the vector of parsed nodes.

[`serde::from_slice_with_buffers`](https://github.com/simd-lite/simd-json/blob/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873/src/serde.rs#L64-L79)
reuses item 2. Its
[`Deserializer::from_slice_with_buffers`](https://github.com/simd-lite/simd-json/blob/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873/src/lib.rs#L847-L859)
still constructs a fresh tape vector. Calling this “allocation-free buffered
Serde” would be incorrect for this checkout.

For deeper reuse, inspect
[`fill_tape`](https://github.com/simd-lite/simd-json/blob/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873/src/lib.rs#L139-L147)
and
[`Tape::reset`](https://github.com/simd-lite/simd-json/blob/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873/src/value/tape.rs#L14-L53).
`reset` clears borrowed nodes and returns the allocation with a new lifetime;
consume all views before resetting or reusing input. A tape-traversal projection
can then reuse both scratch and tape. The consuming `Tape::deserialize` helper
does not by itself provide a way to keep that tape allocation for the next
frame.

Input is rewritten during parsing. Determine the LF boundary **before** passing
exactly that frame's mutable slice to the decoder. Unescaping `\\n` can put a
literal newline into the consumed region: scanning modified bytes as though
they were original framing data is wrong. An immutable sibling observer or
original-wire diagnostic must run before mutation or use a deliberate copy.

“In-place” also does not mean “no internal input copy”: the inspected
[`fill_tape` implementation](https://github.com/simd-lite/simd-json/blob/0662a83a4da6b99dc60fd0fcb03d46d4ca73a873/src/lib.rs#L793-L844)
copies input into reusable aligned storage. Measure bytes moved as well as
allocation counts. SIMD acceleration may still outweigh the extra traffic.

## Project fewer objects before optimizing their representation

The obvious first baseline is a small `Vec<WindowProjection<'a>>` with borrowed
ID/title/app fields and `active`. It is easy to inspect but constructs a vector
and may allocate for escaped metadata on **inactive** windows.

A stronger active-only candidate is a sequence visitor returning zero or one
active projection. It visits every array member, validates the whole frame,
and constructs no window-list vector. Stage the candidate until decoding has
succeeded; a malformed later member must not partially update application
state. Treat multiple active members as an explicit invariant failure rather
than silently selecting whichever happened to occur first.

Avoid relying on object-key order to skip metadata. To decode metadata only for
the selected window, a Sonic alternative can retain raw per-window metadata or
the raw window object and decode it once activation is known. This trades
rescanning for fewer escaped-string allocations. Compare it with the simpler
borrowed projection; only real title distributions decide which pays.

For a consumer needing the complete window inventory, reusable application
storage can be worthwhile. It is a different workload: an ID-indexed model,
change-aware string updates, and full-snapshot replacement cost must be included.
Do not impose that inventory on an active-window-only consumer.

## Performance experiment

### Corpus and semantic equivalence

Build scrubbed real captures plus deterministic synthetic variants. Record
source revision, fixture hash, bytes, window count, title lengths/escape rate,
and event family. Every decoder must produce the same normalized result before
its timing is compared.

Cover empty, 1, 10, 50, and 200 windows; active entry first/middle/last/absent;
ordinary UTF-8, quotes, backslashes, escaped newlines, Unicode escapes and long
titles; metadata-only churn versus rapid active-ID changes; both envelope key
orders; unknown extra fields and nested values. Include mixed event/reply/error
traffic separately from windows-only traffic.

Malformed suffixes, duplicate relevant fields, invalid UTF-8, multiple active
windows, fragmented frames and oversized frames belong in a correctness corpus.
Different parser acceptance behavior must be reconciled or reported explicitly;
equal outputs on valid fixtures alone do not establish equal contracts.

### Three measurement layers

| Layer | Experiment | Useful measurements |
|---|---|---|
| Decode | Preloaded complete frames → same projection | ns/frame, cycles/byte, allocations and allocated bytes/frame, scratch/tape high-water capacity |
| Replay | Timed byte chunks → framer → decode → reducer | p50/p95/p99 processing latency, bytes copied, frames/wake, retained memory, timer error, output correctness |
| Socket | Synthetic Unix-socket peer, then read-only live subscription/query | wakeups, syscalls, catch-up time, command connect/write/reply breakdown, disconnects |

Use Criterion for decode/replay throughput and controlled timing comparisons;
record per-frame latency distributions separately instead of relabeling a
Criterion aggregate as p99. Use allocation instrumentation in a separate run
when its overhead would distort timings. Profiling can then explain a measured
hot path.

For mutable-input simd-json, restore pristine fixture bytes for every iteration.
Publish parse-only and copy-inclusive measurements separately. The production
reader already receives writable bytes, so charging only simd-json for a
fixture-cloning convenience is unfair; omitting its internal parser copy is
also incomplete accounting.

Separate cold connection/first allocation, warmed steady state, growth to a
larger message, and retention after a large message. Exercise split positions
inside UTF-8 and escapes, one-byte fragmentation, several frames per read, and
large incomplete tails. Distinguish time saved by skipping work from time saved
by parsing the same work faster.

### Comparison ladder

1. Matched borrowed `Vec` projections across Sonic, serde_json, buffered simd-json.
2. Active-only visitor versus that baseline; full-frame validation retained.
3. Sonic lazy selection versus direct projection; reusable simd-json tape
   traversal for both active-only and multi-projection workloads.
4. Reusable contiguous framer versus `read_until` into a reused vector.
5. Full pipeline with the stable-focus semantics in the
   [architecture proposal](../architecture/architecture0.gpt6a.md#stable-active-window).

Change one factor at a time. Record compiler, crate lockfile, CPU, allocator,
target features, build profile, and subscriber workload. Measure a normal
portable build and a host-native build separately; Sonic's README recommends
`-C target-cpu=native`, while simd-json defaults to runtime dispatch on x86.
Neither build arrangement should be silently given to only one candidate.

An optimization earns adoption when it improves its named workload with equal
semantics and acceptable memory/latency behavior. A parser win that disappears
inside socket scheduling is still useful evidence about where to work next.

## Cross-references

- [Current architecture](../architecture/architecture1.gpt6a.md): selects Sonic
  and owns the whole-protocol, multi-crate direction.
- [Initial sans-I/O architecture](../architecture/architecture0.gpt6a.md): preserves
  detailed frame lifetimes and dwell semantics from the first proposal.
- [Command research](../protocol/commands0.glm53m.md): owns actual wire shapes and
  connection constraints; parser benchmarks must use that protocol rather than
  generic large-JSON benchmark files.
- [Prior Longcat investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md):
  supplies the geometry-heavy, whole-inventory counterexample to active-only
  projection. Those consumers need different decoding economics.
