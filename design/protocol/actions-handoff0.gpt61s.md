---
type: ImplementationHandoff
title: Typed action starter and family handoff
description: Established action encoding and workspace invariants, with independent family ownership for the remaining catalog.
resource: design/protocol/actions-handoff0.gpt61s.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent integrated the g expansions, corrected numeric conformance evidence, and added completion navigation
  - resource: opencode://ses_ec7757ed9ffejpJVbqe777EnUJ
    title: Implemented and verified overview-toggle then typed workspace-switch; prepared family delegation seams
  - resource: commands0.glm53m.md
    title: Protocol and argument-category baseline
  - resource: review0.gpt6a.md
    title: Corrected LF-inclusive limits and universal JSON string escaping
  - resource: file:///home/rektide/a/noctalia-dev/umbriel/src/config/keybind_parse.cpp
    title: Read action table, workspace grammar, and every argument dispatch branch
  - resource: file:///home/rektide/a/noctalia-dev/umbriel/src/config/keybind_parse.h
    title: Read workspace limit and payload/submap semantics
  - resource: file:///home/rektide/a/noctalia-dev/umbriel/tests/unit/keybind_parse.cpp
    title: Read argument-family and workspace conformance examples
extensions:
  ticket: umbrs-protocol-actions
---

# Typed action starter

**Implemented continuation:** all 121 catalog paths are now covered by typed
builders and the exact-once integration test. The
[current architecture](../architecture/architecture2.gpt6a.md) describes the
completed protocol layer. The account below preserves the two-action starter
and the handoff used for the remaining families.

Only `overview-toggle` and `workspace-switch` are implemented. This is a
reviewable starter, not catalog completion. The [catalog research](commands0.glm53m.md)
supplies the inventory; the [source review](review0.gpt6a.md) governs final size
accounting and escaping. No live compositor commands were issued.

## Public interface and established invariants

```rust
use umbriel_protocol::action::{Action, WorkspaceSelector, WorkspaceTarget};

let selector = WorkspaceSelector::name("2")?;
let target = WorkspaceTarget::new(selector).on_output("DP-1")?;
let action = Action::workspace_switch(target);
action.request().encode(&mut bytes)?;
```

`Action<'a>` holds private `Cow<'a, str>` text. No-argument actions borrow static
text. Parameterized builders prepare one owned string, and the resulting
`Action<'static>` outlives borrowed constructor inputs. `Action::request()`
borrows that text into existing `Request::Msg`; neither Request nor codec was
changed. Repeated sends do not rebuild action text or an intermediate JSON tree.
The existing encoder reuses caller-owned `Vec<u8>` storage and performs JSON
escaping and the 65,536-byte LF-inclusive limit check. Its Sonic serialization
internals were not allocation-benchmarked; tests establish storage reuse, not
a global zero-allocation claim.

`Action::raw(text)` deliberately bypasses action validation, including unknown
future names. Existing direct `Request::Msg` is unchanged. Neither raw path
bypasses JSON escaping or request limits. Action does not implement Serialize:
use `request()` so the wire value is a msg request, not an ambiguous string.

`WorkspaceSelector` and `WorkspaceTarget` have private fields and cheap Copy
semantics. Indexes are 1..=64. Names consisting of ASCII digits are emitted
quoted, including `0`, `65`, leading zeros, and values too large for an index.
Other names are emitted unquoted, preserving colons, whitespace, controls,
backslashes, non-ASCII digits and **interior quotes**. Empty names, slash-bearing
names and names beginning or ending in a quote are unrepresentable and rejected.
This follows first-slash splitting before quote interpretation, not shell
escaping. Output qualifiers reject only empty text and slashes; quotes and
colons remain legal. These restrictions apply to workspace qualification,
**not** to standalone OptionalOutput arguments, which permit any nonempty text.

All checks are local syntax checks. Existence, ambiguity, current focus,
headless-only applicability, and execution success belong to the compositor.

## Adding actions

For a bare action, add a `Category::None` Spec constant in
`crates/umbriel-protocol/src/action/catalog.rs`, include it in the sorted unique
`IMPLEMENTED` slice, and extend `no_argument::NoArgument` and its `as_str` match.
`Action::from(NoArgument::...)` already supplies borrowed static text. Optional
named convenience constructors can follow `Action::overview_toggle()`; a
constructor for every bare action is not necessary.

For parameterized families, use ordinary sibling modules. A family module can
add `impl Action<'static>` methods and access private `Action::parameterized`
to format validated arguments once. Keep validation before that helper; do not
expose it as a public generic builder. A private-field argument type with
validated constructors and Display is appropriate for reusable fractions or
selectors. Do not build a generic argument registry, generator, or another
crate. Optional families can return a bare borrowed Action for None and reject
Some(""); no colon with an empty suffix is valid.

Workspace movement builders should reuse `WorkspaceTarget` directly, with the
same `impl Into<WorkspaceTarget<'a>>` input as switch. No new selector grammar is
needed for `window-move-to-workspace` or `column-move-to-workspace`.

## Independent g ownership

The parent should reserve `action/mod.rs`, `action/catalog.rs`, and
`tests/actions.rs` for integration. Agents return their needed Spec constants,
module declarations and ValidationError additions instead of concurrently
editing these files. New sibling modules can contain their own Action impls.

| Slice | Independent owned files | Required upstream regions |
| --- | --- | --- |
| Bare catalog | `action/no_argument.rs`, `tests/actions_no_argument.rs` | cpp table 173–372; parse 467–473; unit tests 375–453 |
| Fractions and deltas | `action/extent.rs`, `tests/actions_extent.rs` | cpp 490–527; unit tests 257–341 |
| Commands and submaps | `action/command.rs`, `tests/actions_command.rs` | cpp 123–129, 474–489; h 167–169, 242–244; unit tests 216–255 |
| Output, scratchpad, window targets | `action/target.rs`, `tests/actions_target.rs` | cpp 540–582; unit tests 515–580 |
| Workspace movements, layout and quit | `action/workspace.rs`, new `action/control.rs`, `tests/actions_control.rs` | cpp 528–539, 583–618; unit tests 199–214, 344–370, 455–513 |

Paths in the last column are relative to
`/home/rektide/a/noctalia-dev/umbriel`: `src/config/keybind_parse.cpp`, `.h`, and
`tests/unit/keybind_parse.cpp`. The full table remains the source of exact
names/categories. Consult `payloadAlternativeMatchesTheDeclaredArgKind`
(unit tests 582 onward) when checking catalog coverage.

Command text must remain verbatim after the first colon; submap additionally
rejects `disable` and `]`, while `reset` is valid. Fractions require finite
0.1..=1.0; deltas require finite nonzero magnitude <=0.9. Typed f64 builders
should reject NaN/infinities before formatting and check emitted text against
the C++ grammar. Layout uses scrolling/dwindle/master/toggle, where toggle is
an operation, not an observed layout mode. Quit uses an explicit confirmation
choice, not arbitrary text. Window IDs and scratchpad/output arguments are
opaque nonempty text, with absence represented separately from empty text.

The shared catalog does create a real integration point. It is not sensible to
let every agent edit it simultaneously. Family-local work and tests are
independent; parent integration should add returned constants and sorted
inventory entries, then check every upstream spec has one typed path.

## Verification and remaining scope

The starter's eight tests cover action meaning with a small independent
workspace-branch model, literal numeric-name disambiguation, every legal index,
invalid index/name/output forms, colon/interior-quote/non-ASCII/control text,
final JSON roundtrip and exact escaping, input lifetimes, reusable storage,
and exact/over-limit escaped requests with recovery after failure. The small
model is not an executed C++ parser; source review supplies that correspondence.

Passed `cargo nextest run -p umbriel-protocol` (32 tests),
`cargo clippy -p umbriel-protocol --all-targets -- -D warnings`, and formatting.
Stage one committed the bare action separately before workspace implementation.
Future agents should run their family tests and these same protocol checks;
no live mutation is needed. Parent owns root documentation, tickets and lockfile
updates and decides how to expose this handoff in broader indexes.
