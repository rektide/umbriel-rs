---
type: Research
title: Umbriel IPC wire protocol — commands, subscriptions, and the action grammar
description: Source-grounded catalog of Umbriel's IPC request/reply envelope, all query and action commands, the six event families, connection lifecycle and reuse rules, and the full msg action grammar with parse-level constraints, pinned at git revision 7666af0.
resource: design/protocol/commands0.glm53m.md
status: draft
tags: [umbriel, ipc, protocol, wire-format, actions, research]
generated:
  at: 2026-10-13
  by: model:zai-coding-plan/glm-5.3#max
sources:
  - resource: opencode://ses_ecbf7de5cffe4m4a3AoV3OiXNb
    title: Full protocol source read (server, config, CLI, tests), live read-only socket probes, and wire catalog
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Parent added navigation to the source-review corrections and current architecture; original investigation preserved
  - resource: https://github.com/noctalia-dev/umbriel/tree/7666af0a43560658e7b966befcbe47c101968e1c
    title: Umbriel compositor checkout (VERSION 0.1.0, commit 2026-10-09) at /home/rektide/a/noctalia-dev/umbriel
  - resource: file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md
    title: Prior Umbriel IPC/Longcat investigation whose uncertainties this wave resolves
  - resource: ../architecture/architecture0.gpt6a.md
    title: Parent sans-I/O architecture proposal this catalog feeds
  - resource: ../performance/json0.gpt6a.md
    title: Parent JSON backend investigation (owns codec choice; out of scope here)
---

# Umbriel IPC wire protocol: commands, subscriptions, and the action grammar

**Read with the [parent source review](review0.gpt6a.md)**, which corrects
request-size accounting, blank subscription responses, subscriber-wide dedup
claims, and a few schema/decoder recommendations. The current library direction
is [Sonic-first and multi-crate](../architecture/architecture1.gpt6a.md).

## Scope and grounding

Everything below is read from the Umbriel compositor source at git revision
`7666af0a43560658e7b966befcbe47c101968e1c` (`VERSION` = `0.1.0`, commit dated
2026-10-09, local checkout `/home/rektide/a/noctalia-dev/umbriel`, remote
`https://github.com/noctalia-dev/umbriel`). File references like `ipc.cpp:281`
are line ranges in that revision; canonical URLs are
`https://github.com/noctalia-dev/umbriel/blob/7666af0a43560658e7b966befcbe47c101968e1c/src/server/ipc.cpp#L281`.

This document owns the **wire contract**: envelope shapes, the complete request
catalog, connection lifecycle and reuse rules, all six event families with
payload schemas, and the full `msg` action grammar with its parse-level
constraints. It deliberately does not choose a JSON backend
([json0](../performance/json0.gpt6a.md) owns that) or an architecture
([architecture0](../architecture/architecture0.gpt6a.md) owns that). Three
read-only probes against the live compositor (`/run/user/1000/umbriel-wayland-0.sock`)
verify the lifecycle claims; nothing was mutated.

There is **no protocol version negotiation anywhere** — no version request,
field, or handshake exists in `src/server/ipc*`. Everything below is the
0.1.x observed contract; a client must tolerate added keys and unknown
families across releases (see [Forward compatibility](#forward-compatibility)).

## Wire basics

Transport is one UNIX stream socket, path
`$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock` (`ipc.cpp:112`), unlinked on
compositor shutdown (`ipc.cpp:174-176`). `UMBRIEL_SOCKET` overrides the CLI's
path resolution only (`src/cli/ipc_client.cpp:20-30`); the server ignores it.

Framing is newline-delimited JSON (NDJSON): one JSON object per line each way.
`nlohmann::json::dump()` serializes object keys in **lexicographic order**
(std::map backing), and escapes control characters, so a line never contains a
raw `\n` — LF scanning is an exact frame boundary. Verified live.

### The three envelope shapes

Every line the server ever sends is exactly one of (`ipc.cpp:275-279`,
`ipc.cpp:340-416`, `ipc.cpp:418-419`):

| First key (wire order) | Shape | Meaning |
|---|---|---|
| `"ok"` | `{"ok":<value>}` | request succeeded; `<value>` depends on the command |
| `"err"` | `{"err":"<string>"}` | request failed; human-readable reason, no code |
| `"data"` | `{"data":<payload>,"event":"<family>"}` | event line; note `"data"` sorts before `"event"`, so the family name is the **last** key on the line |

A fast discriminator needs only the first 5–6 bytes after `{`: `"data` → event,
`"err"` → error, `"ok"` → reply. There is **no request ID, no echo of the
command, and no sequencing information** in any reply — correlation is possible
only by connection role and line order (see [Correlation and the one-outstanding
rule](#correlation-and-the-one-outstanding-rule)).

### Requests

A request is one JSON object: `{"cmd":"<name>"}` plus, for commands with
`takesArg`, `"arg":"<string>"`. Extra members are silently ignored
(`ipc.cpp:340-415` reads only `cmd` and `arg`). Malformed JSON, non-object,
missing/non-string `cmd`, missing/non-string `arg` when required, or an empty
`events` array all yield `{"err":"malformed request"}`.

## Connection lifecycle

This section resolves the prior investigation's uncertainty #4 ("subscribe+request
mixing … treat as unspecified") and the architecture doc's "subscribed-command
reuse is an implementation-dependent possibility". Both are now settled from
source and live probes.

### Roles and reuse

The entire lifecycle falls out of `writeResponse` (`ipc.cpp:281-306`): after a
reply is fully flushed, a connection with `subscribedEvents == 0` returns
`false` → the connection is **closed**; a subscribed connection instead clears
its output, stops "responding", and returns to read-readiness.

1. **Non-subscribed connection: strictly one-shot.** Exactly one request is
   answered, then the server closes the fd. `docs/user/ipc.md:27-28` ("A
   connection with no subscription closes once its replies are written")
   states it, `ipc.cpp:299-305` implements it, and the integration test
   (`tests/harness/checks/010_ipc.sh:84-86`) plus live probe P1 confirm it.
2. **Subscribed connection: reusable for sequential requests.** After
   `subscribe` is accepted, the connection stays open forever (no timeout) and
   each further request line gets its `{"ok"|"err"}` reply inline, interleaved
   with `{"data":…,"event":…}` lines in true send order. Implemented by
   `ipc.cpp:299-304` + `readRequest` continuing; verified live (probe P3 sent
   `submap` on a `windows`-subscribed connection and got `{"ok":null}` back on
   the same stream). This is *documented by implication* (the docs sentence
   above) and implemented, but **no test exercises it** — the harness always
   issues actions from separate connections. Rely on it, but keep it isolated
   as the architecture doc proposes.
3. **No pipelining, ever.** `readRequest` (`ipc.cpp:240-273`) handles only the
   first line and leaves the remainder in the connection's input buffer.
   - On a one-shot connection, pipelined extra requests are **silently
     dropped** when the server closes (verified live, probe P2: two requests
     sent, one reply, EOF).
   - On a subscribed connection, a pipelined second request sits in the
     server-side buffer until the *next* readable event — i.e. until the client
     sends at least one more byte or half-closes. It is not lost, but it is
     arbitrarily delayed. Never have more than one outstanding request.

### Timeouts and limits

| Limit | Value | Source |
|---|---|---|
| Connection deadline | 1000 ms from accept, removed only when `subscribe` is accepted | `ipc.cpp:30,208-211,369-372` |
| Max request line | input buffer > 65536 bytes before a newline → `{"err":"request too long"}` | `ipc.cpp:27,246-249`; test uses 65537 bytes |
| Subscriber backlog | undrained output > 256 KiB → that subscriber is disconnected (EOF) | `ipc.cpp:29,433-439` |
| Listen backlog | 4 (kernel pending-connection hint for the accept loop) | `ipc.cpp:139` |

Consequences worth designing around:

- **Idle one-shot pools are impossible.** A pre-opened non-subscribed
  connection dies at the 1 s deadline, and after its first reply it is closed
  anyway. Per-request cost is one connect + accept + close round through the
  compositor's single-threaded loop. A controller doing bursts should sequence
  over the subscribed command channel instead.
- **An oversized request poisons a subscriber.** The too-long branch never
  clears `connection.input`, so a subscribed connection that once exceeded
  64 KiB replies `{"err":"request too long"}` to every subsequent readable
  event forever. After that error on a subscriber, reconnect and resubscribe.
- **Slow readers are evicted, not stalled.** Backlog eviction produces EOF;
  the only recovery is reconnect + resubscribe (fresh initial state, no
  replay). Live snapshot math: 29 windows ≈ 8.4 KB per `windows` line, so
  256 KiB ≈ ~30 full snapshots of headroom; during animated geometry churn
  the compositor can emit one line per event-loop iteration (frame-rate
  cadence), so sustained non-draining will be evicted in seconds.
- Reconnect after EOF or socket disappearance is cheap by design: the
  subscribe reply *is* the fresh full state (see below).

## Request catalog

Command dispatch is a linear scan of `kIpcCommands`
(`src/server/ipc_commands.cpp:616-640`); `subscribe` is special-cased before it
(`ipc.cpp:346-403`). The `arg` column reflects `takesArg`.

| `cmd` | `arg` | `ok` payload | Notes |
|---|---|---|---|
| `windows` | — | array of [window entries](#windows) | unmapped windows excluded |
| `workspaces` | — | array of [workspace entries](#workspaces) | every output's group, in output then index order |
| `submap` | — | string or `null` | active keybind submap; `null` = default context |
| `layers` | — | array of [layer entries](#layers) | layer-shell surfaces |
| `msg` | required: action string | `null` | runs the [action grammar](#the-action-grammar-msg); errors name the failure |
| `color` | — | object | color-management state: `{color_manager, renderer:{…}, outputs:[…], surfaces:[…]}` (`ipc_commands.cpp:440-513`) |
| `tearing` | — | object | tearing-control state: `{protocol, outputs:[…], surfaces:[…]}` (`ipc_commands.cpp:515-576`) |
| `keyboard-layouts` | — | `{names:[string], current_index:int}` | `{"err":"no keyboard"}` when no physical keyboard exists |
| `output-create` | required: name | string (assigned output name) | headless sessions only |
| `output-destroy` | required: name | `null` | headless sessions only |
| `subscribe` | `events` array (not `arg`) | initial event lines (no `ok`) | see [Subscriptions](#subscriptions) |

### Error catalog

All observed error strings (`ipc.cpp`, `ipc_commands.cpp`, `src/server/actions.cpp`):

- Protocol: `"malformed request"`, `"request too long"`,
  `"unknown command: <cmd>"`, `"unknown subscription event: <name>"`.
- Action parse failure: `"unknown action: <arg>"` (`ipc_commands.cpp:589`).
- Action execution failures (dynamic, from handlers): `"unknown window: <id>"`
  (both id-taking focus/close), `"unknown workspace: <name>"`,
  `"unknown workspace position: <n>"`, `"unknown workspace on output <out>: <ref>"`,
  `"ambiguous workspace: <name> (qualify it as <token>/<output>)"`,
  `"unknown output: <name>"`, `"no other output"`, `"no output to the left"`
  (and friends for each direction), `"unknown scratchpad: <name>"`,
  `"scratchpad name required"`, `"no keyboard"`, `"output is disabled: <name>"`,
  `"no enabled outputs"`, plus layout-requirement messages for actions with no
  meaning in the active layout (e.g. `column-center` in dwindle/master).
- `msg`'s `{"ok":null}` means *parsed and executed*, not that geometry settled;
  the resulting `windows` snapshot arrives later on subscribers (coalesced per
  event-loop iteration).

### CLI notes (reference client)

`umbriel <cmd> [--json]` opens a fresh connection per invocation with a 2 s
receive timeout, reads to EOF, and prints `ok`'s value as JSON or the human
rendering (`ipc_client.cpp:77-151`). `umbriel subscribe a,b` (comma- or
space-separated) relays lines unparsed with no timeout; it checks only the
first line for an `err` rejection (`ipc_client.cpp:153-207`). `umbriel msg`
joins argv with spaces, except `msg spawn <cmd…>` which rewrites to
`spawn:<cmd…>` (`src/main.cpp:253-292`) — so other parameterized actions need
the explicit colon form: `umbriel msg 'window-focus:<id>'`.

## Subscriptions

Request: `{"cmd":"subscribe","events":["<family>",…]}` — non-empty string
array; duplicates fold (bitmask OR); a non-string element is malformed.
Validation of *all* names happens before any state changes
(`ipc.cpp:350-367`).

**Accept** (`ipc.cpp:368-402`): sets the subscription bits, removes the 1 s
deadline permanently, and replies with **the current full state of each
requested family, one line per family, in fixed family-bit order**
`theme, overview, keyboard_layout, windows, workspaces, submap` — *not* the
order in the request array. There is no `{"ok"}` ack; the initial lines are
the response. `keyboard_layout` is **skipped** when no physical keyboard
exists (the stream simply starts later), so count lines by their `event` field,
never by position.

**Reject**: `{"err":"unknown subscription event: <name>"}`, and on a fresh
connection the server then closes. On an *already-subscribed* connection the
early return leaves the existing bits intact: the stream survives with an err
line — a typo'd incremental subscribe is not fatal.

Further subscribe calls on a live subscriber are **cumulative**
(`subscribedEvents |= requested`); there is **no unsubscribe**. Each call
re-sends the initial state of the families named *in that call* (the response
is built from `requested`, not the cumulative mask), which doubles as a
cheap refresh.

### Event families (all six at this revision)

| Family | Payload | Fires on (`docs/user/ipc.md:72-79`) |
|---|---|---|
| `theme` | full theme object (below) | color/corner-radius change via config reload |
| `overview` | `{"open":bool}` | overview open/close |
| `keyboard_layout` | `{"names":[…],"current_index":int}` | layout switch |
| `windows` | array of window entries — byte-identical to the `windows` query (`ipc.cpp:89-93` reuses the handler) | open, close, focus, title, app id, geometry, workspace, scratchpad, floating |
| `workspaces` | array of workspace entries — identical to the `workspaces` query (`ipc.cpp:95-99`) | layout mode, activation, occupancy, names, named, indices, membership, cursor crossing outputs |
| `submap` | string or `null` | submap push/pop |

### Snapshot semantics

Every family payload is a **full snapshot**, never a delta: replace, don't
merge. Two server-side mechanisms bound the rate (`ipc.cpp:418-452`,
`src/server/server_events.cpp:1467-1501`):

- **Coalescing**: `windows`/`workspaces` notifications are scheduled as
  event-loop idle sources — many mutations within one loop iteration produce
  one line.
- **Byte dedup**: each family's last broadcast string is cached; a recomputed
  payload identical to the last broadcast is dropped entirely. A fresh
  subscriber still always gets its own initial line from the subscribe
  handler, so skipping a no-op broadcast strands no one (`ipc.h:83-86`).

Client-side consequence: every delivered line for a family represents a real,
distinct state. Timestamping lines on receipt is meaningful; there are no
duplicate or empty wakeups.

## Event payload schemas

Field lists are fixed and always complete at this revision (handlers assign
every key). Wire key order is alphabetical (see [Wire basics](#wire-basics)).

### Windows

Entry keys (alphabetical): `active, app_id, content_type, floating, focused,
h, id, pid, scratchpad, title, urgent, w, workspace, x, xdg_tag, xwayland, y`
(`ipc_commands.cpp:341-384`).

| Field | Type | Semantics |
|---|---|---|
| `id` | string | ext-foreign-toplevel identifier, reused verbatim as IPC identity (`src/view/view.h:87-91`); stable for the window's map lifetime; `""` only in the no-handle corner |
| `workspace` | string | workspace id `"<output>:<serial>"`; **empty while stored in a scratchpad** |
| `scratchpad` | string | configured scratchpad name, `"default"` for the implicit pad, `""` for regular windows |
| `active` | bool | **seat-global activation**; at most one window session-wide (zero when a layer surface or nothing holds keyboard focus) |
| `focused` | bool | **per-workspace remembered focus** (`workspace->focusedView() == view`); exactly one per occupied workspace family-wide; scratchpad windows never have it |
| `app_id`, `title` | string | as committed; `""` when unset |
| `xdg_tag` | string | xdg-toplevel tag, `""` when absent |
| `content_type` | string | `"none" \| "photo" \| "video" \| "game"` (`src/config/config.h:277-289`) |
| `floating` | bool | floating vs tiled |
| `urgent` | bool | urgency hint |
| `xwayland` | bool | XWayland (xwayland-satellite) provenance |
| `pid` | int | pid owning the Wayland connection; `-1` when unknown — **every XWayland window** shares the satellite's connection, so all report the same pid or -1 |
| `x`,`y` | int | **layout target slot** for tiled windows (post-animation destination, computed even for hidden workspaces; `layout().targetBox()`), own position for floats; can be negative for off-viewport columns |
| `w`,`h` | int | `committedContentBox()` — the size the client *has pixels for*, widened toward a configured-but-committed resize; during animated resizes a snapshot can mix target position with committed size |

The `active` vs `focused` split is the load-bearing distinction for
seat-tracking consumers: `active` answers "what does the user's keyboard talk
to" (0-or-1), `focused` answers "what would regain focus on entering this
workspace" (0-or-1 per workspace). Live session: exactly 1 `active:true`, 6
`focused:true` across 29 windows — the two sets are genuinely different
census rules. A window can be `active:true, focused:false` (a shown scratchpad
window holds the seat but belongs to no workspace) and `focused:true,
active:false` (every non-active workspace's remembered window).

### Workspaces

Entry keys: `active, focused, id, index, layout, name, named, occupied, output`
(`ipc_commands.cpp:386-413`).

| Field | Type | Semantics |
|---|---|---|
| `id` | string | `"<output>:<serial>"`, stable for the workspace's lifetime — the join key for windows |
| `name` | string | mutable; an anonymous workspace can be renumbered on moves/prunes |
| `named` | bool | membership in a static/persistent inventory; **use this, not name-shape** — a workspace literally named `"2"` is valid |
| `index` | int | 1-based display position |
| `output` | string | connector name |
| `active` | bool | the group's active workspace; exactly one per output group |
| `focused` | bool | `active && output == preferred(cursor) output` — the cursor crossing outputs moves `focused` without activating anything; exactly one session-wide (integration test asserts both singletons and `focused ⇒ active`) |
| `occupied` | bool | holds ≥ 1 window including invisible ones; scratchpad-stored windows don't count |
| `layout` | string | `"scrolling" \| "dwindle" \| "master"` — effective (runtime-overridden) mode; events are the only way to follow it |

### Smaller payloads

<a id="layers"></a>

- **Layers** (`ipc_commands.cpp:423-438`): entries `{layer, mapped, namespace,
  output}`; `layer` ∈ `background|bottom|top|overlay`, `mapped` bool.
- **Submap**: bare string or `null`; submaps **nest** (live-verified ordering:
  push `outer`, push `inner`, reset reveals `outer`, reset → `null`).
- **Overview**: `{"open":bool}`.
- **Keyboard layout**: `{"names":[string],"current_index":int}`.
- **Theme** (`ipc.cpp:32-66`): 11 flat `#RRGGBBAA` color strings
  (`background, text_primary, text_muted, accent_primary, accent_secondary,
  warning, error, insert_hint, backdrop, shadow`), nested `border` object
  (`focused, unfocused, scratchpad_focused, scratchpad_unfocused, outer`),
  nested `overview` object (`background_tint, workspace_background, badge`),
  and integer `corner_radius` (logical pixels).

## The action grammar (`msg`)

`msg` feeds its string through the *same* parser as keybind actions
(`IpcCommands::msg` → `parseAction` → `executeKeybindAction`,
`ipc_commands.cpp:586-597`; dispatch in `src/server/server_keybinds.cpp:104-146`).
The catalog is the single `kActionSpecs` table
(`src/config/keybind_parse.cpp:173-372`, ~110 entries, compile- and
test-enforced to be sorted, unique, and fully handled). IPC-issued actions
carry default `cooldownMs = 0`, an empty submap, and no `submapAfter`
(`Keybind{}` defaults, `keybind_parse.h:214-233`) — so they are never
cooldown-blocked and never rerouted by keybind state. The overview-open
interception hook (`server_keybinds.cpp:130-131`) only clears overview-internal
pending-focus/shortcut state and returns `false` (`src/overview/overview.cpp:2767-2791`)
— actions execute normally while the overview is open.

### Grammar

One action per request. Shape: `name` or `name:arg`, where the split is on the
**first** colon only, and `arg` must be non-empty when the kind takes one
(`takeActionArg`, `keybind_parse.cpp:123-129`). There is **no escaping and no
quoting at this level** — an argument simply is everything after the first
colon, and may itself contain colons (e.g. `workspace-switch:name:2` parses as
name `"name:2"`; a `spawn:` command line passes through verbatim to
`/bin/sh -c`, `src/server/server.cpp:997`). Matching is exact and case-sensitive
per name; argument-free actions reject any argument.

### Argument kinds and parse-level constraints

| Kind | Used by | Constraint (parse time) |
|---|---|---|
| none | e.g. `workspace-next`, `overview-toggle`, … | bare name only; any argument rejected |
| `<cmd>` (Command) | `spawn`, `submap` | non-empty rest; submap names additionally: ≠ `"disable"`, no `]` (`validSubmapName`) |
| Fraction | `window-set-primary-extent`, `window-set-secondary-extent` | finite double, `0.1 ≤ f ≤ 1.0`; `std::from_chars` grammar (no leading `+`, scientific notation accepted), full-string match; `nan`/trailing garbage rejected |
| FractionDelta | `window-modify-{primary,secondary}-extent`, `window-modify-width-{left,right}`, `window-modify-height-{up,down}` | finite, ≠ 0, `|d| ≤ 0.9`; one optional leading `+` |
| Workspace | `workspace-switch`, `window-move-to-workspace`, `column-move-to-workspace` | selector grammar below |
| `[<output>]` (OptionalOutput) | `dpms-on`, `dpms-off` | bare or non-empty name |
| `[<scratchpad>]` (OptionalScratchpad) | `scratchpad-toggle`, `scratchpad-focus-next`, `window-{move-to,restore-from,toggle-}scratchpad` | bare (implicit `default`) or non-empty name; bare-with-named-pads errors at execution: `"scratchpad name required"` |
| `<window-id>` (WindowId) | `window-focus`, `window-focus-warp` | any non-empty string; validated at execution |
| `[<window-id>]` (OptionalWindowId) | `window-close`, `window-toggle-floating` | bare = focused window, or non-empty id |
| `[skip-confirmation]` | `session-quit` | bare (confirmation dialog) or exactly `skip-confirmation` |
| LayoutMode | `workspace-set-layout` | exactly `scrolling`, `dwindle`, `master`, or `toggle` (lowercase) |

Workspace selector grammar (`parseWorkspaceArg`, `keybind_parse.cpp:131-171`):
`<selector>[/<output>]` where `/` splits at the **first** slash; the output
part must be non-empty and slash-free. The selector is then either

- a quoted name `"…"` (both quotes required, no embedded `"`; this is the only
  way to force a numeric *name* like `"2"`), or
- all digits → 1-based workspace index, `1 ≤ n ≤ kMaxWorkspaces = 64`
  (`keybind_parse.h:19`), or
- any other non-empty text → a name (may contain `:` but a `/` inside a name
  is inexpressible — even quoted, because the slash split happens first).

Index selectors resolve on the cursor-preferred output (clamped to the last
workspace); names resolve globally with preferred-output disambiguation
(`resolveWorkspaceSelector`, `src/server/actions.cpp:90-167`).

### Targeting semantics that matter for senders

- **Focused-window targeting is seat- and pointer-derived**: layout-relative
  actions use the active workspace of the *cursor-preferred* output, unless a
  scratchpad window holds the seat, in which case extent/resize actions
  retarget the floating scratchpad window (`actions.cpp:169-206`). `active`/`focused`
  in the last `windows` snapshot is a good but not perfect predictor during
  transitions.
- **Extent actions act only on the focused window** — `set/modifyFocused{Width,Height}`
  inside the workspace; there is no `[<window-id>]` form for them. Only
  `window-close` and `window-toggle-floating` take an optional id, plus the
  two id-required focus actions. Restoring a non-focused window's extent
  needs the documented focus round-trip (`window-focus:<id>` → set →
  `window-focus-last`).
- **Window-id resolution** is a linear scan over mapped views by ext-foreign
  identifier (`viewByForeignIdentifier`, `actions.cpp:543-554`); unknown id →
  `{"err":"unknown window: <id>"}`. `window-focus:<id>` summons a hidden
  scratchpad window to the pointer's output first, focuses with
  ForeignActivation semantics, and warps only per `input.cursor.follows_focus`;
  `window-focus-warp:<id>` always warps (`actions.cpp:1064-1096`).
- **`window-focus-last`** picks the first *other* mapped view in the view
  registry, which is ordered most-recently-focused-first
  (`src/view/registry.h:11-23`, `actions.cpp:1098-1110`) — i.e. the true MRU
  previous window, not a workspace-weighted choice.
- **`spawn:<cmd>`** runs `/bin/sh -c <cmd>` with `XDG_ACTIVATION_TOKEN`/
  `DESKTOP_STARTUP_ID` exported (one-shot token), under a systemd scope in
  managed sessions (`server.cpp:924-1012`). Failure to exec dies in the child;
  the action itself always returns `ok`.
- **No-op tolerance**: many actions (focus at layout edges, layout-scroll in
  non-scrolling layouts, master-count outside master) return `{"ok":null}`
  having done nothing. `ok` ≠ observable change; watch snapshots.

## Live verification (read-only probes, 2026-10-13)

Against `/run/user/1000/umbriel-wayland-0.sock` (29 windows, multiple outputs):

- **P1 — one-shot close**: sent `submap`, received `{"ok":null}`, then EOF
  (`recv` → `b''`). Confirms non-subscribed close-after-reply.
- **P2 — pipelining dropped**: sent `submap⏎layers⏎` on one fresh connection;
  got one reply (`{"ok":null}`), then EOF. Second request never answered.
- **P3 — subscriber reuse**: subscribed `windows`; received the initial
  `{"data":…,"event":"windows"}` line; then sent `submap` on the same
  connection and received `{"ok":null}` inline; stream stayed open and idle.
- **Envelope order**: raw bytes confirmed `{"ok":[…`, `{"err":"…`,
  `{"data":[…,"event":"windows"}⏎` — alphabetical key order, `data` first.
- **Sizes/counts**: initial `windows` line 8424 B for 29 entries (~290 B/entry);
  `workspaces` 1110 B; `windows` query reply 8405 B; exactly 1 `active:true`
  and 6 `focused:true` — the active/focused census difference is real.

## Discrepancies and confirmations vs prior material

1. **zk investigation uncertainty #4 → resolved.** Subscribed connections *do*
   answer further commands inline; it is implemented (`ipc.cpp:299-304`),
   documented by implication (`docs/user/ipc.md:27-28`), and live-verified.
   Caveat retained: untested by the upstream harness; a conservative library
   can still default to separate one-shot connections for commands.
2. **"Reusable connections" (shared ChatGPT answer) → needs precision.**
   Blanket reuse is wrong: *only subscribed* connections are reusable, and
   only sequentially. Non-subscribed connections are one request → reply →
   server-initiated close, and the 1 s pre-subscribe deadline forbids idle
   pooling of either kind (subscribers keep their deadline only until
   acceptance).
3. **"No request IDs" → confirmed** on the wire and in source; see
   [Correlation](#correlation-and-the-one-outstanding-rule).
4. **Screencast event family → absent at this revision.** The chat answer
   mentions one; `ipc.h:22-36` at `7666af0` enumerates exactly six families
   (theme, overview, keyboard_layout, windows, workspaces, submap), and
   subscribing to any other name errs and closes. With no version
   negotiation, the client contract should be "the six families at 0.1.x,
   discoverable by probe, tolerant of more later" — never a hard-coded
   seventh.
5. **Minor precision on "64 KiB cap"**: the check is `input.size() > 65536`
   *before* any newline is seen, so a maximal legal line plus newline is
   65537 bytes total; and oversize **poisons subscribers** (no buffer clear)
   — new hazard not in the prior investigation.
6. **New facts the prior investigation lacked**: fixed family-bit ordering of
   initial state (request order ignored); keyboard_layout initial-line skip;
   incremental/cumulative subscribe with re-sent initial state and surviving
   streams on a bad name; no unsubscribe; listen backlog 4; workspace
   `index` is 1-based; workspaces `focused` = active-on-cursor-output;
   content_type vocabulary; the alphabetical wire key order.

## Implications for the Rust protocol library

These are wire-contract facts a typed library can be built directly on; codec
choice and crate layout stay with the parent documents.

### Envelope module

The three-shape envelope with alphabetical key order admits an exact, cheap
first-key discriminator (`"data`/`"err`/`"ok"`), and frame splitting is exact
LF scanning with no escape ambiguity. A typed envelope layer can therefore:

- model server lines as an enum: `Ok(Value) | Err(String) | Event { family,
  data }`, with the family name read from the line's *tail* if a zero-copy
  fast path wants it before payload decode;
- keep request encoding trivial: `{"arg":"…","cmd":"…"}` (sorted keys match
  the reference client's bytes, though the server is order-insensitive); only
  `arg` values ever need JSON string escaping, and only `spawn`/workspace-name
  arguments carry arbitrary text.

### Typed query markers and models

Each command has a fixed ok-shape, which supports a typed reply enum, but the
*shape alone does not identify the command*: `null` is ambiguous between
`msg` and `output-destroy`; a bare string between `submap` (or `output-create`);
arrays between `windows`/`workspaces`/`layers` only by element schema; the
object payloads differ structurally (`color`, `tearing`, `keyboard-layouts`).
The library's session layer must therefore pair each request with its
expected reply type at the call site — the wire offers no marker. Sensible
Rust models fall out of the schemas above: a `WindowEntry` with the 17
alphabetical fields (string ids; `pid`/geometry as integers; `content_type`
as a four-variant enum with unknown-string tolerance), a `WorkspaceEntry`
with `WorkspaceId(Output, Serial)` parsed from `"<output>:<serial>"`,
`LayoutMode` and `Layer` enums, `Theme` with owned color strings or parsed
`RRGGBBAA` records, and a `Submap(Option<String>)`.

### Correlation and the one-outstanding rule

With no request IDs and one-shot/non-subscribed connections closing after a
single reply, the natural session state machines are:

- **Command role**: connect → send one request → read one reply (`ok`/`err`)
  → expect EOF. A per-call timeout (the CLI uses 2 s) and a fresh connection
  per call mirror the reference client; the 1 s server deadline means a
  request must be written promptly after connect.
- **Subscription role**: connect → subscribe within 1 s → read initial lines
  (count by `event` field, not position; `keyboard_layout` may be absent) →
  then an interleaved stream of event lines and (if used) sequential
  command replies. One outstanding request at a time is a protocol
  requirement, not a style choice; a lost connection after sending an action
  leaves that action's outcome genuinely unknown (reconnect state is fresh
  snapshots, not an answer).

### Backpressure, reconnect, and epochs

Design for: EOF at any time (compositor exit, backlog eviction, poisoned
oversize); reconnect + resubscribe as the only recovery, yielding a full
fresh state (an "epoch" boundary for derived state machines); per-family
snapshot replacement with byte-dedup making every line meaningful; and a
drain budget sized to real numbers (~290 B/window, 256 KiB eviction
threshold, frame-rate line cadence during animations). Subscription read
paths that cannot keep up should degrade by reconnecting, not by queueing
unboundedly.

### Forward compatibility

No version negotiation exists, so: tolerate unknown object keys everywhere
(the field sets are fixed *today*); treat unknown event families as a
subscribe-time failure with an exact error message to surface; treat unknown
`msg` actions as the server's `"unknown action: …"` rather than client-side
gating (the action table is upstream's to grow); and pin behavior to the
observed 0.1.x contract with the six-family, one-outstanding, one-shot rules
as the tested core. The upstream test suite (`tests/unit/keybind_parse.cpp`,
`tests/harness/checks/010_ipc.sh`) is the closest thing to a conformance
suite and is worth mirroring in miniature.

### The 750 ms stable-active-window example, in protocol terms

Subscribe `windows`; the initial line seeds the census; select the 0-or-1
`active:true` entry (never `focused` — that is per-workspace memory and
diverges from the seat, as the live 1-active/6-focused split shows); a
dwell timer keys on the active window's `id` across successive snapshots,
where every delivered line is a distinct real state (coalescing + dedup are
server-side guarantees). Valid no-active observations exist (layer surfaces,
empty focus) and are distinct from transport gaps by epoch. No geometry or
completion semantics are needed for this reducer — `active` alone suffices.

## Evidence index

All paths relative to the umbriel checkout at revision
`7666af0a43560658e7b966befcbe47c101968e1c`:

- **IPC server**: `src/server/ipc.h` (families `:22-36`, last-broadcast cache
  `:83-86`); `src/server/ipc.cpp` (limits `:27-30`, path `:112`, backlog `:139`,
  read `:240-273`, write/close-vs-keep `:281-306`, deadline `:308-312`,
  request/subscribe handling `:340-416`, broadcast dedup/evict `:418-452`).
- **Commands**: `src/server/ipc_commands.cpp` (windows `:341-384`, workspaces
  `:386-413`, submap `:415-421`, layers `:423-438`, color `:440-513`, tearing
  `:515-576`, keyboard-layouts `:578-584`, msg `:586-597`, outputs `:599-614`,
  table `:616-640`).
- **Actions**: `src/config/keybind_parse.cpp` (specs `:173-372`, arg split
  `:123-129`, workspace arg `:131-171`, parse `:463-622`);
  `src/config/keybind_parse.h` (`kMaxWorkspaces :19`, submap validity `:167-169`,
  payload types `:160-234`); `src/server/actions.cpp` (scratchpad errors
  `:47-71`, workspace resolution `:90-167`, spawn `:458-462`, id lookup
  `:543-554`, close `:584-600`, focus-by-id `:1064-1096`, focus-last
  `:1098-1110`, extents `:853-915`, handler table `:1708-1849`);
  `src/server/server_keybinds.cpp` (dispatch `:104-146`); `src/view/registry.h`
  `:11-23`; `src/server/server.cpp` (spawn exec `:924-1012`).
- **Events**: `src/server/server_events.cpp` (idle coalescing `:1467-1501`).
- **CLI**: `src/cli/ipc_client.cpp` (path `:20-30`, 2 s timeout `:84-86`,
  subscribe relay `:153-207`); `src/main.cpp` (arg joining `:253-292`).
- **Docs**: `docs/user/ipc.md` (one-shot sentence `:27-28`, event table
  `:62-95`); `docs/user/actions.md` (argument forms `:10-27`, layout
  differences `:202-223`, notes `:225-313`); `SCOPE.md` (IPC-in-scope,
  no-plugins posture).
- **Tests**: `tests/harness/checks/010_ipc.sh` (deadline `:51-77`, oversize
  `:81-82`, pipelined drop `:84-86`, initial ordering `:556-574`, unknown
  family `:320-325`, submap nesting `:294-318`); `tests/unit/keybind_parse.cpp`
  (full grammar matrix); `tests/unit/ipc_commands.cpp`.
- **Prior material**: zk investigation
  (`/home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md`),
  parent architecture (`../architecture/architecture0.gpt6a.md`), JSON
  investigation (`../performance/json0.gpt6a.md`), shared ChatGPT answer
  (screencast-family discrepancy, noted above).
- **Live probes**: P1/P2/P3 and measurements against
  `/run/user/1000/umbriel-wayland-0.sock`, 2026-10-13, read-only.
