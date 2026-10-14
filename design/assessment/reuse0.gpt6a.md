---
type: Assessment
title: Reusing observation and ownership mechanisms for Umbriel
description: Sequential source assessment of tmux-interface-control then kubeless, with concrete integration seams, subscription semantics, crate boundaries, and a source-checked Rust Arrow/Polars path.
resource: design/assessment/reuse0.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra#medium
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent integrated the assessment and repaired navigation to moved Kubeless homes while preserving original revision-bound findings
  - resource: opencode://ses_ecbe07e52ffeiRoULSOml79Kjd
    title: Independently assessed tmux first and kubeless second, checked Rust columnar APIs, and proposed bounded reuse seams
  - resource: brief0.gpt6a.md
    title: Full commission and required comparison sequence
  - resource: ../architecture/architecture1.gpt6a.md
    title: Human-selected Rust, Sonic-first, sans-I/O, whole-protocol, multi-crate direction
  - resource: ../protocol/commands0.glm53m.md
    title: Protocol catalog and reported live subscription command reuse
  - resource: ../protocol/review0.gpt6a.md
    title: Governing source corrections for size bounds, blank replies, and duplicate snapshots
  - resource: file:///home/rektide/src/tmux-interface-control/README.md
    title: Current tmux library contract, followed into implementation and tests listed below
  - resource: file:///home/rektide/src/tmux-interface-control/doc/library/ordering.md
    title: Cooperative correlation, cancellation, and connection-local evidence contract
  - resource: file:///home/rektide/src/tmux-interface-control/design/information-lifecycle/README.md
    title: Distinguishes maintained information research from an implemented tracker
  - resource: file:///home/rektide/src/kubeless/README.md
    title: Independently adoptable capability map, followed into source and tests listed below
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-lazy/src/frame/mod.rs
    title: Rust lazy execution and actual collect_batches signature and iterator behavior
  - resource: file:///home/rektide/archive/pola-rs/polars/crates/polars-core/src/frame/chunks.rs
    title: Polars-native record batch to DataFrame conversion
  - resource: file:///home/rektide/archive/apache/arrow-rs/arrow-array/src/record_batch.rs
    title: Apache Arrow record batch type and checked constructor
  - resource: file:///home/rektide/archive/polars-inc/skills/polars/SKILL.md
    title: Python-oriented lazy-query guidance; Rust source governs API claims here
verification:
  method: Read maintained contracts, implementation branches, manifests, and representative test bodies; checked repository state and Rust type signatures.
  limits: No comparison-project test suite, live compositor probe, integration implementation, or columnar compilation was performed in this assessment.
---

# Reusing observation and ownership mechanisms for Umbriel

## Recommendation

**Reuse Kubeless's local view machinery through a small Umbriel snapshot adapter;
borrow tmux's ownership and bounded-delivery laws for the session driver.** Keep
Umbriel's actual wire session native. Neither comparison project's complete
session/controller stack is a drop-in replacement.

The strongest immediate package candidate is **`kubeless-view-memory`**, or
`kubeless-view` when sole-writer closure and historical readiness are useful.
They already implement atomic keyed replacement, coherent reads, cheap affected
keys, and optional before/after evidence. This is substantially more reusable
than rebuilding a concurrent inventory store. The package dependency is optional
at the application level because `umbriel-state` itself is optional.

The highest-risk attractive reuse is **`kubeless-view-publication` directly on
the socket-draining path**: one stalled subscriber can backpressure all source
progress. The best transfer from tmux is independently bounded subscribers with
explicit terminal loss, alongside a separately named latest-state interface.

These are proposals following the accepted direction, not authorization for an
automatic desired-state controller. Materialized omission removes a row from a
local observation; it does not close a window or destroy an output.

## Evidence and revision binding

Read order was the target README, brief, current architecture, command catalog
and governing review; then tmux docs/design/code/tests; then Kubeless
docs/code/tests; then supplementary columnar sources.

| Repository | Inspected revision / caveat |
|---|---|
| `umbriel-rs` | Commission baseline `f0b1f996f28c47cd6a8c03aced7a8cf00e5786e9`; concurrent parent/codec work was present (`Cargo.toml`, `src/lib.rs` at the initial check), and was not assessed as a completed client. |
| `tmux-interface-control` | jj working copy `df95c3e843c57bd11d9b0166c638af65a069b5f4`, empty over `2cee35d2530bc97d5e8e7427461f45561086750b`; `jj diff --stat` showed no changes. |
| `kubeless` | jj working copy `a7262d8e84d1c8b5e5d61e5b18ce0748788a49b8`, parent `7cca2eb236da74c571fd1cbd2e073357fdfba039`; one pre-existing three-line diff in `kube/runtime/src/events.rs`, outside the inspected reusable mechanisms. |
| archived Polars | Git `31df0ae24119bf23bcfb74b836fe114e130af9a8`, clean; workspace Rust version `0.55.1`. |
| archived arrow-rs | Git `00fadf9606b5e30ea96639aa15c402c0ab7d37cd`, clean; workspace version `59.2.0`, MSRV `1.88`. |

For tmux, raw Git HEAD was the older `7d5622b7b1627238e2a9aa134fb6541982b1b39d`
and Git described most current files as untracked. That is not the revision of
the inspected jj workspace. Use the explicitly selected jj `@` above, not raw
Git HEAD or the first item of an unrestricted log. Links below navigate local
homes; this table binds the source observations to their inspected versions.

Kubeless packages inherit version `4.2.0`, edition 2024, MSRV `1.89.0`; the
README explicitly says its extra crates are unpublished. A later dependency
needs a reviewed path/Git pin to this fork, not a registry `kube` dependency.

## 1. tmux-interface-control: mature native ownership, protocol-specific machinery

### What exists

[`endpoint::Tmux`](file:///home/rektide/src/tmux-interface-control/src/endpoint/mod.rs)
is immutable-by-convention endpoint configuration, not a process owner or live
socket. `query` uses `TypedOperation::prepare`, executes one native command, then
runs its fresh decoder. `session()` constructs a persistent client against an
existing server. Socket resolution and inherited context are native tmux rules;
Umbriel should transfer the endpoint/configuration distinction, not those rules.

[`operation`](file:///home/rektide/src/tmux-interface-control/src/operation/mod.rs)
keeps structured invocation, typed identities, decoders, and declaration-bound
preparation together. One query vocabulary works in direct and persistent modes.
`SnapshotOperation` is a narrow audited acquisition classification, not permission
to refresh or reconcile. For Umbriel, associated response types and typed action
arguments are useful; tmux row escaping, version declarations, and group decoders
are unnecessary for an NDJSON reply that is already one complete value.

[`session::Session`](file:///home/rektide/src/tmux-interface-control/src/session/mod.rs)
is a cheap shared handle over driver ingress, close handles, terminal receipts,
and limits. Last-owner drop initiates close; explicit shutdown observes cleanup.
The current path is native socket/control protocol, not a persistent subprocess.
Native server ownership is a separate optional facility. No implicit server
launch should enter Umbriel client construction either.

The internal
[`ProtocolCore` / `ProtocolEffect`](file:///home/rektide/src/tmux-interface-control/src/control_mode/protocol/core.rs)
and request ledger distinguish admission, dispatch, body routing, completion,
delivery retirement, and close. That input/effect split is a good sans-I/O
precedent, but these are `pub(crate)` types embedded in the tmux crate. Their
effects contain tmux frame/block/fence and tracked-line types. Direct linkage
would import the wrong protocol; extraction is a separate cross-project change.

### Subscription and failure laws worth transferring

[`Subscription`](file:///home/rektide/src/tmux-interface-control/src/session/subscription.rs)
owns one bounded observation interval. Queued events drain before a retained
terminal error. Wait timeout is distinct from cancellation; readiness is historical
activation evidence, not continued liveness. Nonblocking owner drop requests local
retirement. Independent subscriptions do not acquire each other's cancellation.

The internal
[`DeliveryProducer<T,E>`](file:///home/rektide/src/tmux-interface-control/src/control_mode/delivery/cell.rs)
has both item and byte bounds, checked byte arithmetic, first-terminal retention,
and explicit completion. Dropping a producer does not itself invent a terminal
reason. It also supplies synchronous/asynchronous consumer claims, but remains
private. Its mechanism is a plausible future shared extraction; for the first
Umbriel driver, implement its few needed laws over the chosen runtime's channels
and explicit terminal state rather than copy the entire threading framework.

[`Completion`, `CompletionProof`, `CompletionReceipt`](file:///home/rektide/src/tmux-interface-control/src/session/request.rs)
separate delivery outcome, native boundary, caller decoding, and lightweight
observation. Cancellation is not execution reversal. The *public request receipt*
is deliberately lightweight; do not assume every internal generic terminal
receipt is detached from its delivery cell.

Tmux can admit several requests because it sends a fresh private fence after
each application invocation and scans through that fence even after consumer
cancellation. Its command numbers alone are not caller correlation IDs. Umbriel
has neither those fences nor equivalent group semantics. Transfer this law:

> Losing the consumer must not release unresolved wire-reply ownership to the
> next request.

In Umbriel, a cancelled or timed-out sent request must retain the sole reply slot
until its `ok`/`err` drains, or the connection must close. A later command cannot
take that reply. A cancelled unsent request can be removed. No retry of a possibly
executed action follows from either path.

### What remains DIY or prospective

The bounded delivery, ingress, request ledger, native resource manager, and
threaded transport are extensive private infrastructure, not reusable public
crates. The README explicitly lists maintained projections (`track`), public
replay fixtures, async transport, and automatic reconnect as unimplemented.
The [information-lifecycle inquiry](file:///home/rektide/src/tmux-interface-control/design/information-lifecycle/README.md)
is useful reasoning about coverage and stale evidence, not an existing inventory
engine. Native format subscriptions are sampled resources: disappearance can be
silence, unlike Umbriel's complete family replacement.

Representative test evidence read:

- `tests/library/session.rs::accepted_credit_remains_occupied_after_native_terminal_until_fence`
  keeps admission blocked until actual correlation retirement.
- `tests/library/session.rs::independent_connections_stamp_requests_receipts_subscriptions_and_events`
  exercises provenance through all public handles.
- `src/control_mode/delivery/cell.rs` contains explicit byte-overflow and
  first-terminal/drain tests; the implementation's bounds were inspected.
- `tests/ordering_contract/` and `tests/operation_contract/` contain compile-time
  API restrictions, complementing the scripted peer and isolated live tests.

For Umbriel, prefer tiny deterministic peer transcripts covering wrong reply
attribution and event interleaving over reproducing tmux's whole test apparatus.

## 2. Kubeless: useful packages below the controller

Navigation update, 2026-10-14: the former `observation/view` implementation now
lives under `observation/reflection`, and `runtime/assembly` is now `runtime`.
The links below reach those current homes; the revision table still binds the
original assessment. See the [newer asks assessment](../kubeless/asks0.gpt61s.md)
for the current 0.1.0 API evaluation.

### Concrete reuse and its limits

| Capability | Actual source seam | Umbriel fit / cost |
|---|---|---|
| Atomic keyed current view | [`kubeless-view-memory::memory_view`, `Writer::replace_affected`, `replace_changes`, `Reader::snapshot`](file:///home/rektide/src/kubeless/view/memory/src/lib.rs) | **Direct package candidate.** A small family-snapshot-to-`Replacement` adapter; no Kubernetes resource trait. Costs owned keys, `Arc<V>` rows, locking, replacement work. |
| Writer lifetime/readiness | [`kubeless-view::{view, View, Writer, ViewStatus}`](file:///home/rektide/src/kubeless/view/src/lib.rs) | **Direct candidate when needed.** Adds explicit readiness and sole-writer closure; source health and connection epochs remain Umbriel data. Readiness stays true after closure. |
| Staged replacement | [`Reflection::apply`, `Fact::{SnapshotBegin,SnapshotItem,SnapshotComplete}`](file:///home/rektide/src/kubeless/observation/reflection/src/state.rs) | **Small adapter, optional.** Commit only on complete validated family snapshot. Usually more machinery than direct `Replacement`, because Umbriel already delivers the complete family in one frame. |
| Snapshot+resumable session | [`StreamRequest::{Initial,Resume}`, `Frame`, `Recovery`](file:///home/rektide/src/kubeless/observation/session/src/protocol.rs) | **Do not substitute for the wire session.** Umbriel has no server resume cursor, paged acquisition, global completion marker, or replay. A local sequence cannot implement `Resume`. |
| Seed then later commits | [`seeded::Subscribers::subscribe`](file:///home/rektide/src/kubeless/view/publication/src/seeded.rs) | **Strong local bootstrap pattern.** Registers before taking a coherent snapshot, then filters commits through the captured generation. Requires the documented single-writer commit/publication ordering. This is a local barrier, never an Umbriel initial-state barrier. |
| Bounded fan-out | [`blocking::bounded`, `Publisher::publish`](file:///home/rektide/src/kubeless/view/publication/src/blocking.rs) | **Policy mismatch on the drain path.** Shared queue counts publications, not bytes; all responsible subscribers must advance. Good for deliberate backpressured offline/post-ingest pipelines. |
| Reflection+publication owner | [`kubeless-observation-reflector::Writer`](file:///home/rektide/src/kubeless/observation/reflector/src/lib.rs) | **Conditional composition.** Owns pending publication across dropped borrowed drivers. Its serialize-apply/publish/next-input law still stalls input under pressure. `affected_unpublished`/manual routes can avoid publication. |
| Native evidence model | [`Evidence`, `SubjectRef`, `ObservationStamp`, `Observed`](file:///home/rektide/src/kubeless/observation/model/src/lib.rs) | **Optional vocabulary reuse.** `Known(None)` differs from missing; source incarnation/revision can be unsupported. An Umbriel connection epoch is observation lineage, not server incarnation. |
| Invocation vs rejection | [`control-attempt::attempt`, `control-pass::from_resolution`](file:///home/rektide/src/kubeless/control/pass/src/lib.rs) | **Optional application composition.** Native results pass through without equating completed polling with success. Do not replace the driver's reply correlation or uncertainty record. |
| Conditional operations | [`kubeless-operation`](file:///home/rektide/src/kubeless/operation/src/lib.rs) | **Weak direct fit today.** Its conflict retry needs source-checked conditions. Umbriel lacks CAS/version preconditions. A local view anchor cannot supply them. An `Infallible` conflict type could model one attempt, but adds little to a typed client call. |
| Keyed work | [`ReadyWork`](file:///home/rektide/src/kubeless/work/ready/src/lib.rs), [`KeyedFutures::try_insert`](file:///home/rektide/src/kubeless/work/keyed/src/lib.rs) | **Useful later for controllers.** Exclusion key for a subscribed command lane must be the connection, not window ID. Different windows still share one reply slot. `insert` replaces/drops an existing future; use deliberate admission instead. |
| Timed scheduling | [`work/schedule`](file:///home/rektide/src/kubeless/work/schedule/README.md) | **Optional Tokio consumer tool.** Trailing-edge debounce of every geometry snapshot is not active-identity dwell: repeated equal identity must not restart that timer. Keep the example reducer clock-injected. |
| Runtime assembly | [`DirectParts`, `Config`, `CloseOn`, `reflected_current`](file:///home/rektide/src/kubeless/runtime/src/lib.rs) | **Controller recipe, not transport driver.** Selects source/readiness/drain policy and spawns no work by itself. Forceful drop does not manufacture completion; graceful drain excludes pending source publication. |
| Catalog | [`Catalog`, `Declaration`, `reconcile`](file:///home/rektide/src/kubeless/catalog/src/lib.rs) | **Mostly a pattern for now.** Pure declared/discovered capability facts, not control reconciliation. Umbriel has no exhaustive discovery/version handshake; a pinned command catalog cannot be advertised as remotely confirmed support. |

These lower crates really are generic today. `view/memory/Cargo.toml` has normal
dependencies only on `hashbrown`, `parking_lot`, and `kubeless-view-change`.
The Kubernetes resource and watch conversion policy lives in the separate
`kube/` tree. Conversely, calling a mechanism runtime-free does not make its
assembly clock-free: `work/schedule` explicitly uses Tokio and DelayQueue.

### Materialization details that affect the adapter

`Replacement` is whole-map replacement. Therefore use **one map per complete
family scope**, rather than replacing a mixed map of all object families with
each windows frame. Windows and workspaces use their opaque string identities.
Singleton families can use a one-entry map or simpler owned snapshot cell.
Do not force theme, overview, keyboard layout, and nullable submap into a window
row abstraction. Query-only layers/color/tearing are finite acquisitions, not
new subscribed families.

`Replacement::upsert` is last-write-wins. Umbriel's adapter should explicitly
decide duplicate-ID behavior before publication; silent overwrite can hide
invalid inventory. Reject duplicates for the keyed inventory path, or expose an
explicit non-keyed representation for exceptional empty/nonunique IDs. This
decision belongs to the state adapter, not to JSON parsing or raw projections.

`replace_changes` retains every union key, including equal before/after values.
It does not automatically remove duplicate snapshots or semantic no-op rows.
Use `replace_affected` when only current reads matter; request rich evidence
only for actual consumers. If semantic dedup is wanted, compare the selected
fields and retain the observation metadata even when no semantic change emits.
Rich evidence retains old `Arc<V>` values until dropped, so bounded message
counts alone do not establish a byte budget.

Tests read in `view/memory/tests/replacement.rs` explicitly compare pure alignment
with actual replacement and show that equal values still produce evidence and
new handles. `observation/view/tests/lifecycle.rs` proves borrowed driver drop
leaves the owner/candidate alive, while owner drop closes the view.
`observation/reflector/tests/installed.rs` keeps installed rows, staged rows,
commit cursor, progress, and pending publications separate. These are existing
implementation tests, not tests newly executed by this assessment.

## Wins and risks across both projects

| Proposed win | Risk if copied too broadly | Recommended boundary |
|---|---|---|
| One operation vocabulary for direct and persistent clients | Treating every socket as persistent or pipelineable | Protocol query/action descriptors shared; client selects one-shot or accepted-subscriber driver |
| Single owner of wire progress and request completion | Dropped future frees the slot before its reply drains | Sans-I/O pending-request state outlives the waiter; explicit close or drain |
| Bounded independent subscriptions | Shared blocking publication stalls command replies and all observers | Per-observer loss/retirement; latest-state is a separate contract |
| Atomic replacement and coherent keyed reads | Row append creates history; family replacement wipes other families | Family-scoped `Replacement` adapter over Kubeless view |
| Seed-plus-commit bootstrap | Local generation mislabeled server revision; seed metadata captured separately races | One serialized owner captures family data and metadata; local generation named explicitly |
| Preserved owner across disposable drivers | `View::is_ready()` mistaken for healthy after disconnect | Source health and epoch stored separately from mechanical writer lifetime |
| Rich before/after evidence | Every equal snapshot becomes expensive churn | Cheap path by default; selected semantic comparison and bounded evidence lifetime |
| Reusable control attempts/work | Native `ok` becomes convergence, or per-window concurrency breaks wire correlation | Optional application control composition over native outcomes; no auto-retry/prune |
| Columnar output | Lazy query assumed to incrementally maintain live state | Owned finite snapshot batches; explicit replacement envelope and explicit execution |

## Umbriel subscription and state contract

### Wire ownership

1. Endpoint configuration discovers a socket but opens it on demand. One-shot
   command calls connect/send/read one reply/close. No idle one-shot pool.
2. A subscribed connection has one receive owner and at most one ordinary request
   outstanding. It continues to route event frames while waiting for `ok`/`err`.
3. Initial subscription uses its own state: a matching initial event on a fresh
   connection supplies acceptance evidence; the known keyboard-only blank LF is
   empty-response evidence. Neither an event nor LF completes an ordinary query.
   Family readiness is independent. No wait-for-all universal barrier exists.
4. Initially prefer a declared fixed family set per wire connection. Multiple
   local observers select from it without repeated wire subscribe. Local drop
   removes a consumer only: the server has no unsubscribe. Adding families needs
   explicit cumulative-subscribe scheduling, not reuse of the ordinary reply
   abstraction. Repeated already-selected family snapshots cannot, by themselves,
   identify which broadcast/refresh caused them or delimit a resubscribe response.
5. Bound encoded requests to 65536 bytes **including LF**, before writing. Bound
   receive frames and total queued delivery separately. Oversize-server rejection
   retires that connection because its request buffer may remain poisoned.
6. Requests have local admission and write-progress facts. After any possible
   transmission, a lost result may be unknown. A query decode failure after an
   `ok` is distinct from remote rejection. `msg` acknowledgment does not certify
   later geometry convergence.

### Observation identity and replacement

Use an owned metadata envelope (proposed names):

```text
ObservedSnapshot<T> {
  connection_epoch, family, receive_position, received_at,
  value: T
}
FamilyStatus = Unseen | Observed(metadata) | Gap(reason, last_metadata)
```

`received_at` is injected monotonic observation time for local reducers; export
wall-clock time only with its separate meaning. `receive_position` is checked
connection-local order, not a server revision or durable resume cursor. Unknown,
unselected, and reply frames may consume positions, so gaps in a filtered stream
do not prove delivery loss. Explicit loss evidence does.

- Fully decode/validate before installing. A malformed final row must not leave
  half an updated family. An empty windows/workspaces array is a successful
  empty replacement; submap `null` is a known value, not missing state.
- Missing keyboard initial data remains unseen/availability-unknown; a wait
  timeout does not turn it into an empty layout. No family should silently
  inherit a previous connection's readiness.
- On reconnect, increment epoch and reacquire family snapshots. No replay or
  continuity across the gap is promised. Retain old rows only as explicitly
  stale historical observation until fresh replacement, or start new epoch views.
- An observer joining an existing local materializer can get a coherent local
  seed with per-family stamps, then later commits. It cannot recover earlier
  transitions. Equal snapshots are legal, and server coalescing already hides
  some physical intermediate states.
- A windows/workspaces join uses the latest observed value of each family. Carry
  **both** family stamps in a derived result. A local owner can provide a coherent
  capture of that pair, but no API should call it an atomic compositor snapshot.
  A temporarily unmatched workspace ID is explainable skew, not proof of a
  malformed window or permission to delete it.

The minimal state owner can directly call Kubeless `Writer::replace_*` after a
validated frame. If adopting `Reflection`, synthesize begin/items/complete only
for **that complete family frame**, and use metadata as local commit evidence.
Do not feed local receive positions into `observation-session` as resumable
source cursors. The direct view route avoids needing that fiction at all.

### Delivery and recovery

Keep two explicitly different APIs:

- **Every delivered observation:** independent bounded queues; a lagging observer
  gets a retained `Gap/End` and must deliberately start another interval. Already
  queued items may drain before the error. Losing one observer does not block
  unrelated command replies.
- **Latest family state:** overwrite superseded snapshots by family, with stamps
  and skipped/coalesced information. This is suitable for dashboards and many
  columnar queries, not a promise of transition completeness or dwell continuity.

Kubeless's seeded publication can serve an explicitly backpressured downstream
pipeline if it is decoupled from socket draining with a bounded, stated policy.
Adding a queue merely postpones pressure; it must choose loss, coalescing, or
retirement at capacity. Unbounded buffering is not that policy. Source errors
must reach state health independently of successful commit publication.

## Proposed crate graph and integration seams

```text
umbriel-protocol  [Sonic framing, envelopes, all models, requests/actions]
       ↑
umbriel-session   [sans-I/O role, request slot, write progress, frame routing,
       ↑           connection/observation metadata, terminal effects]
umbriel-client    [Unix discovery/I/O, selected runtime, independent delivery]

umbriel-state ───→ protocol + session metadata + kubeless-view-memory
                  (or kubeless-view for explicit writer lifecycle)

umbriel-arrow ───→ protocol + polars-arrow     [optional owned batch emitter]
umbriel-polars ──→ umbriel-arrow + polars      [optional finite lazy query]

application adapters connect client → state / arrow / user projections
application control may use kubeless-control-pass / work leaves later
```

Arrows mean depends on. The main difference from architecture1 is a separate
`umbriel-session`: application execution state, cancellation, and write offsets
change for different reasons than wire models. It can be tested without an
executor and reused by custom event loops. Create it when the first complete
session slice lands, not as an empty crate now. Keeping this as a protocol module
initially remains workable; do not split merely to meet a crate count.

The second clarification is that **columnar emission must not require state**.
Functions taking borrowed validated family slices or caller projections can emit
columns directly. State integration can pass captured rows into the same builders;
avoid having `umbriel-arrow` pull in locks, publication, or a controller.

Third, put Polars lazy execution in its own optional package, rather than making
the wire or record-batch library build the query engine. Direct Apache Arrow
interop is an explicit later adapter/feature, as described below. No tmux crate
dependency is justified by this pass; a shared bounded-mailbox extraction would
need its own cross-project usefulness proof and ownership discussion.

## Arrow and Polars: feasible, with a precise Rust boundary

### What the inspected Rust APIs actually say

The requested “Polars 2.0” wording does not identify the inspected Rust package.
Archived Polars is **Rust `0.55.1`**; its Python-oriented skill is not authority
for Rust version numbers or execution types. This assessment does not claim a
released Rust 2.0 API or verify future compatibility.

| Source API | Checked fact |
|---|---|
| `polars-arrow/src/record_batch.rs` | `RecordBatch = RecordBatchT<ArrayRef>` owns a height, `ArrowSchemaRef`, and arrays; `try_new(height, schema, arrays)` checks counts and equal lengths. This is Polars's Arrow implementation. |
| `polars-core/src/frame/chunks.rs:8–34` | `impl From<polars_arrow::record_batch::RecordBatch> for DataFrame` exists. Its conversion assumes schema/array dtype agreement; a builder must establish that, not rely solely on the batch length checks. |
| `polars-lazy/src/frame/mod.rs:48–61` | `IntoLazy::lazy(self)` converts an owned DataFrame into a LazyFrame over its existing data. |
| Same file `:747–795` | `collect()` returns `PolarsResult<DataFrame>`. `collect_batches(engine, maintain_order, chunk_size: Option<NonZeroUsize>, lazy)` is under feature `async`, uses a one-slot synchronous channel, and starts blocking execution through Polars's runtime. |
| Same file `:2442–2463` | `CollectBatches` implements **`Iterator<Item = PolarsResult<DataFrame>>`**, whose `next()` blocks on `recv()`. It is not a `futures::Stream<RecordBatch>`. |
| `polars-core/src/frame/mod.rs:2109–2152` | `DataFrame::iter_chunks(CompatLevel, parallel)` emits Polars-native record batches and expects aligned/rechunked columns; compatibility conversion may allocate strings/binaries. |
| `arrow-array/src/record_batch.rs:224–265` | Apache `RecordBatch` stores `Vec<Arc<dyn arrow_array::Array>>`; `try_new(schema, columns)` is a different API/type ecosystem, inspected at `59.2.0`. |

The Polars facade has `lazy`, `async`, and `streaming` features at this revision
(`crates/polars/Cargo.toml`). No Python-style `collect(engine="streaming")` belongs
in a Rust implementation sketch. Start with finite `.lazy() … .collect()`; if
chunked execution is needed, use the checked Rust batch API on a worker that
cannot block the receive driver. `async` in a feature name does not make that
iterator nonblocking.

### Recommended first representation

For the user's Polars-first goal, have the optional emitter construct
**Polars-native Arrow batches**, name/re-export the type explicitly as such,
and pin the matching `polars-arrow`/Polars release. This avoids immediately
maintaining two string/buffer ecosystems or requiring the full query engine just
to emit columns. The name `umbriel-arrow` must document which representation it
returns; “Arrow compatible” is not Rust type identity.

If a consumer specifically requires Apache `arrow_array::RecordBatch`, provide
a separately selected adapter. Both inspected libraries expose Arrow C Data
Interface import/export (`polars-arrow/src/ffi/mod.rs`,
`arrow-array/src/ffi.rs`), but these use distinct Rust wrapper types and unsafe
ownership contracts. This assessment verifies the entry points, **not** an
end-to-end zero-copy bridge. An IPC interchange route is a simpler portable
fallback with serialization cost. Resolve offsets, string-view compatibility,
nulls, nested fields, release ownership, and metadata in an actual interoperability
probe before promising zero-copy conversion.

### Two useful input paths

1. **Direct:** validated full family frame → selected typed fields/column builders
   → owned batch. Avoids a row inventory and its per-row `Arc` allocations. JSON
   strings still need owned column storage; Sonic borrowing does not make that
   storage zero-copy. Validate the complete frame before publishing the batch.
2. **Materialized:** captured family rows → same builders → batch. Useful for
   late subscribers, joins, and re-querying unchanged data. Costs the row map plus
   columns and possible rechunking; do not force it on direct consumers.

Initial schemas should preserve strings for opaque IDs, signed geometry/PID,
booleans for active/focused, and explicit nullability. Windows/workspaces are
natural relation batches. Singleton families can be one-row batches; keyboard
names can be a list column. Query-only color/tearing may use separate output
tables when their full schemas are mapped. Version schemas explicitly instead
of appending incompatible rows into one historical table.

Use an external batch envelope such as:

```text
SnapshotBatch { family, epoch, receive_position, schema_version,
                replacement: Complete, batch }
```

An empty replacement must still emit that envelope and an empty schema-bearing
batch. Per-row epoch columns alone cannot describe a zero-row replacement. If a
large snapshot is chunked, add snapshot identity and explicit final-chunk
completion; consumers stage chunks before replacing old membership. Failure in
the middle must not publish a complete relation.

Lazy plans optimize computation on the data they are given. Building
`DataFrame::from(batch).lazy()` cannot retroactively avoid JSON decoding or column
construction. Projection selection must be provided to the emitter separately
if upstream construction savings matter. Each new snapshot creates another finite
input/plan execution; a LazyFrame is not a continuously maintained table or an
incremental join engine. Historical append is a distinct opt-in stream, with
snapshot identity and its own retained-volume policy.

Batch output remains bounded by rows/bytes and explicit replacement units. Run
CPU-heavy queries off the socket owner. Latest-state consumers may deliberately
replace queued batches by family; historical consumers get explicit loss/end
instead. The 750 ms focus example selects identity continuity, not the columnar
batch interval.

## Next coherent slices

1. **Complete the whole typed wire surface** after the existing codec/envelope
   foundation: all queries, six families, full result schemas, and source-backed
   action argument categories. Query/action descriptors work independently of
   both state and runtime.
2. **Specify and implement one sans-I/O session slot** with one-shot and fresh
   subscribe roles. Test partial writes, blank initial LF, interleaved events,
   pending-command cancellation/drain, unknown outcome, EOF, duplicates, and the
   request-size boundary. Defer dynamic resubscribe completion semantics until
   their weaker evidence is explicitly represented.
3. **One Unix/runtime adapter and public ownership contract:** direct query,
   subscribed sequential query, independent bounded observers, explicit local
   retirement and connection shutdown. Use a scripted socket peer; no compositor
   mutation is needed to establish the scheduler laws.
4. **A focused Kubeless view adapter proof:** windows and workspaces replacement,
   empty inventory, duplicate-ID handling, stale epoch status, and a late-reader
   local seed. Compare direct `Replacement` with staged `Reflection`; choose the
   smaller one unless staged input earns its cost. Prove slow consumers cannot
   hold the receive owner before selecting publication reuse.
5. **Optional columnar proof:** one windows schema, empty replacement envelope,
   Polars-native batch → DataFrame → lazy projection → collected batches. Pin
   versions/features, compile the actual Rust path, and check string/null/negative
   values and output ownership after input-buffer reuse. Add Apache interop only
   for a concrete consumer and verify that boundary separately.
6. **Consumer examples and later control:** typed command shell, latest inventory,
   mixed-family local join, and stable-active-identity dwell. An explicitly
   commissioned geometry controller could reuse Kubeless native pass/work
   mechanisms, retaining acknowledgment and observed result as separate evidence.

## Connections to the existing design

- [Architecture1](../architecture/architecture1.gpt6a.md#proposed-crate-boundaries)
  supplies the accepted direction. This assessment refines its session boundary
  and identifies actual view-package reuse rather than requiring a second store.
- [Protocol review](../protocol/review0.gpt6a.md) governs this document's blank LF,
  duplicate snapshot, request limit, and sequential reuse claims. Its corrections
  are essential to any adapter pretending to recognize initialization or loss.
- [Assessment brief](brief0.gpt6a.md#expected-return) remains the commission; the
  findings above distinguish implementation evidence from integration proposals.
- Tmux's [ordering contract](file:///home/rektide/src/tmux-interface-control/doc/library/ordering.md)
  is a cancellation/ownership precedent, while Kubeless's
  [view replacement contract](file:///home/rektide/src/kubeless/view/memory/README.md)
  is the concrete reusable materializer. Their different boundaries are exactly
  why the first should shape the session and the second should sit above it.

Parent integration should add navigation from the maintained README/architecture
when these findings are reviewed; this independent assessment owns its argument
without revising those concurrently maintained documents.
