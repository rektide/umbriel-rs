---
type: Research
title: Noctalia shell capabilities — layout control, Umbriel control path, and exported services
description: Source-grounded survey of the Noctalia v5 shell's compositor integration, its (deliberately absent) core window-layout control, the official umbriel-companion plugin that supplies layout control over Umbriel IPC, and the IPC/API/services Noctalia itself exports.
resource: design/layout-control/noctalia0.glm53max.md
status: draft
tags: [noctalia, umbriel, ipc, layout-control, research]
generated:
  at: 2026-10-22
  by: 'model:zai-coding-plan/glm-5.3#max'
sources:
  - resource: opencode://ses_e9b39b353ffe32xtHZa0jZA7F3
    title: Commissioned G research on Noctalia shell capabilities; read the noctalia-dev archives, fetched all three upstream repos to current main, cross-verified plugin actions against the compositor action registry, and wrote this document
  - resource: longcat-needs0.gpt6a.md
    title: The layout-control requirements this survey feeds; owns the Longcat/Umbriel/Kubeless framing
  - resource: file:///home/rektide/archive/noctalia-dev/noctalia/src/compositors/umbriel/umbriel_runtime.cpp
    title: Shell-side Umbriel IPC client — socket resolution, subscribe request, per-request connections (archive checkout e4eb0ff, 2026-09-14)
  - resource: file:///home/rektide/archive/noctalia-dev/noctalia/src/ipc/ipc_service.cpp
    title: The shell's own exported msg socket service and its text protocol
  - resource: file:///home/rektide/archive/noctalia-dev/noctalia/src/cli/schema_msg.h
    title: Complete exported command registry (110 commands at the archive checkout; 116 at fetched origin/main)
  - resource: file:///home/rektide/archive/noctalia-dev/official-plugins/umbriel-companion/README.md
    title: The official plugin that actually performs layout control over Umbriel IPC (v1.1.0 at archive f5d7f80; v1.1.3 at fetched origin/main)
  - resource: file:///home/rektide/archive/noctalia-dev/noctalia/docs/user/compositor-settings/umbriel.mdx
    title: Documented autostart, layer rules, and keybind integration between the two programs
verification:
  method: Read archive checkouts of noctalia-dev/noctalia (e4eb0ff), noctalia-shell (1396425), official-plugins (f5d7f80), and umbriel (7666af0); fetched origin/main of the first, third, and fourth (e59cc8320, 10dd0255, 5d657581e) and re-read every changed integration file via git show; verified each companion-plugin action string against the compositor keybind action registry at origin/main; confirmed release notes v5.2.0–v5.3.0 via the GitHub API.
  limits: Static source reading only; no live Noctalia or Umbriel session was run, so runtime behavior (socket framing in practice, timing, failure paths) is inferred from code, not observed. Archive working copies remain at their old checkouts; every origin/main claim is labeled as such and was read with git show, not from the working tree. Umbriel-side IPC semantics beyond what the shell and plugin consume belong to the parent's Umbriel research, not this document.
---

# Noctalia shell capabilities

## Why this document exists

The [layout-control needs](longcat-needs0.gpt6a.md) thread needs to know what the
Noctalia shell — the desktop environment Umbriel ships with — already does over
Umbriel's IPC: which observations it takes, which actions it sends, and whether
any shell-level surface already exercises layout control that a Rust client
should compose with rather than duplicate. This is the Noctalia-side answer.
It covers three questions: what the shell's own layout control is, how it
controls Umbriel, and what IPC/API/services it exports.

### Evidence coordinates

Two revisions matter for the shell because upstream moved five weeks past the
archive during this research:

| Repository | Archive checkout | Fetched `origin/main` (2026-10-22) |
|---|---|---|
| `noctalia-dev/noctalia` | `e4eb0ff9` (2026-09-14, pre-v5.2.0) | `e59cc8320` (2026-10-22) |
| `noctalia-dev/noctalia-shell` (v4) | `13964255` (2026-05-15, v4.7.7+7) | not fetched; upstream declares v4 effectively frozen |
| `noctalia-dev/official-plugins` | `f5d7f804` (2026-09-07) | `10dd02555` (2026-10-21) |
| `noctalia-dev/umbriel` | `7666af0a` (2026-10-09) | `5d657581e` (2026-10-22) |

Archive paths below (`file:///home/rektide/archive/noctalia-dev/...`) refer to
the checked-out snapshot; claims read via `git show origin/main:` are marked
**current main**. Releases in between: v5.2.0 (2026-09-27, added Umbriel
overview type-to-launch), v5.2.1 (2026-10-02, removed Triad compositor
support), v5.3.0 (2026-10-15, current release at research time).

## Two Noctalias

- **v4** (`noctalia-shell`, Quickshell/QML): maintenance mode; the upstream
  README banner says v4 issues will mostly not be fixed. Its
  [`Services/Compositor/`](file:///home/rektide/archive/noctalia-dev/noctalia-shell/Services/Compositor/CompositorService.qml)
  has Hyprland/Niri/Sway/Mango/Labwc/ExtWorkspace backends and **no Umbriel
  backend**; a source-wide search finds no `umbriel` references. Umbriel
  integration is v5-only.
- **v5** (`noctalia`): the native Wayland/GLES C++ rewrite — bars, dock,
  launcher, control center, notifications, wallpaper, lock screen, OSDs,
  clipboard, tray, desktop widgets — with a per-compositor backend layer under
  `src/compositors/`: dwl, ext-workspace, hyprland, kde, labwc, mango, niri,
  sway, **umbriel** (triad removed in v5.2.1). All findings below are v5.

## How Noctalia controls Umbriel

### Detection

`src/compositors/compositor_detect.cpp` checks `UMBRIEL_SOCKET` **first** —
before Labwc/Niri/Hyprland/Sway — then falls back to substring-matching
`XDG_CURRENT_DESKTOP`/`XDG_SESSION_DESKTOP`/`DESKTOP_SESSION` for `umbriel`.
Umbriel exports both signals itself: `src/server/server.cpp` sets
`UMBRIEL_SOCKET=$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock`,
`XDG_CURRENT_DESKTOP=umbriel`, and publishes the session environment to systemd
and D-Bus activation.

### Two channels: Wayland protocols plus Umbriel IPC

The backend (`src/compositors/umbriel/`) splits responsibilities:

**Wayland protocols** (generic layer, shared with other compositors):

- wlr-layer-shell for all shell surfaces (bars reserve exclusive zone — the
  shell's only "layout gap" on the desktop);
- ext-workspace-v1 for the authoritative workspace list and **workspace
  activation**: `CompositorPlatform::activateWorkspace` goes through
  `WaylandWorkspaces`, not Umbriel IPC;
- wlr-foreign-toplevel-management and ext-foreign-toplevel-list for window
  identity/activation, wlr-data-control (clipboard), wlr-gamma-control,
  wlr-screencopy, virtual-keyboard. Umbriel implements all of these
  (`src/server/server.cpp` at the archived compositor snapshot).

**Umbriel IPC** (JSON lines over `AF_UNIX`):

- `UmbrielRuntime::resolveSocketPath` honors `UMBRIEL_SOCKET`, else falls back
  to `$XDG_RUNTIME_DIR/umbriel-$WAYLAND_DISPLAY.sock` only if that path exists
  as a socket.
- One persistent event connection sends
  `{"cmd":"subscribe","events":["windows","overview","keyboard_layout","workspaces"]}`
  (**current main** adds `"outputs"`). Frames are newline-delimited
  `{"event":...,"data":...}` objects; the read buffer caps at 1 MiB (overflow
  forces reconnect); reconnect backoff doubles 2 s → 30 s; an `err` frame
  forces reconnect. The compositor sends initial state for each subscribed
  family in a fixed order at subscribe time.
- Requests (`{"cmd":...,"arg":...}` + `\n`, reply `{"ok":...}` or `{"err":...}`)
  open a **fresh connection per request**. **Current main** adds a 500 ms
  deadline over non-blocking connect/read/write; the archived version blocked
  without a timeout.
- The compositor coalesces broadcasts per family (identical payload dropped;
  slow subscribers disconnected), so events are snapshots, not deltas.

### What the core shell actually sends (verified inventory)

| Purpose | Mechanism | Exact strings |
|---|---|---|
| Taskbar click / window activate | `msg` action | `window-focus:<id>`, `window-focus-warp:<id>` |
| Taskbar close | `msg` action | `window-close:<id>` |
| Keyboard layout | `msg` action + command | `keyboard-layout-next`; `keyboard-layouts` (reply `ok.names[]`, `ok.current_index`) |
| Display power | `msg` action | `dpms-on`, `dpms-off` (**current main** also reads per-output `powered` via the `outputs` event and an `outputs` command) |
| Session logout | `msg` action | `session-quit:skip-confirmation` ("noctalia's session menu is its own confirmation, so bypass umbriel's") |
| Overview type-to-launch keys | `msg` action | `overview-toggle`, `window-focus-left`, `window-focus-right`, `window-focus-or-workspace-up`, `window-focus-or-workspace-down` |
| Audio-reactive effects | dedicated push | `{"cmd":"effect-audio","version":2,"bands":[64 × 0..1 floats]}` at ≥16.67 ms intervals, heartbeat 100 ms, expects `{"ok":true}` within 200 ms (**current main**, new `umbriel_audio_provider.cpp`) |

Events consumed: `windows` (fields `id`, `workspace`, `app_id`, `title`,
`active`, `x`, `y`; drives taskbar, occupancy, focused window; scratchpad
windows with empty workspace are deliberately hidden), `workspaces` (only the
`focused`+`output` fields, for focused-output tracking — the payload's
per-workspace `layout` mode is **not** consumed by the core shell), `overview`
(`open` bool), `keyboard_layout`.

Two structural facts follow. First, workspace **activation** never touches
Umbriel IPC: even `noctalia msg workspace-switch next|prev` computes the
neighbor shell-side from the ext-workspace list and activates it via the
protocol (stopping at the ends). Second, the shell's overview integration is
inverted: Umbriel renders its own overview; when open, Noctalia maps a 1×1
click-through overlay-layer surface with exclusive keyboard
(`noctalia-overview-launcher` namespace) to capture typing keys — opening its
own launcher — while forwarding navigation keys back as Umbriel actions.

### Session bootstrap

Documented in
[`docs/user/compositor-settings/umbriel.mdx`](file:///home/rektide/archive/noctalia-dev/noctalia/docs/user/compositor-settings/umbriel.mdx):
Umbriel's `[general] autostart = ["noctalia"]` spawns the shell once after
startup; keybinds integrate by *spawning the CLI*, e.g.
`"Mod+Space" = "spawn:noctalia msg panel-toggle launcher"`. The compositor also
gives Noctalia's layer surfaces keyboard focus on Escape
(`src/layer/layer_surface.cpp`), and the docs ship layer rules matching
namespace `^noctalia-(bar-…|notification|dock|panel|attached-panel|osd)$` for
blur. There is no deeper embedding: two processes, sockets, and Wayland.

## Layout control

**The core shell performs no window-layout control.** The 110-command msg
registry at the archive snapshot (116 at **current main**) was enumerated in
full: there is no window move, resize, column, workspace-layout, or
sizing command. The only layout-adjacent surface is the shell's **own**
chrome: `bar-hide` "release their layout gaps", `bar-reserve-toggle`,
`bar-layer-set top|overlay` — layer-shell exclusive-zone management for bars,
not tiling. The workspace verbs are `workspace-switch next|prev`,
`taskbar-cycle`, and the `workspace-alert-*` annotation set.

**Layout control lives in the official `umbriel-companion` plugin**
([`official-plugins/umbriel-companion`](file:///home/rektide/archive/noctalia-dev/official-plugins/umbriel-companion/README.md)),
v1.1.0 at the archive, v1.1.3 at **current main** of official-plugins. It is a
Luau plugin (manifest `noctalia/umbriel-companion`, `plugin_api = 28`), not
core code, and it shells out to the `umbriel` CLI rather than linking
anything:

- `service.luau` holds `umbriel subscribe workspaces,windows,submap` open via
  `noctalia.runStream`, republishes each line as a snapshot on the
  `umbriel.state` channel, and executes every UI request as
  `noctalia.runAsync({"umbriel","msg",action})` with a 2 s timeout — "nothing
  here polls". Liveness is re-checked with `noctalia.processMatches` on the
  next UI request, because a silent stream is indistinguishable from a dead
  one.
- `bar.luau` shows the focused workspace's layout (or the active keybind
  submap); a scroll flick cycles scrolling → dwindle → master.
- `panel.luau` provides: per-workspace layout switching; layout-scoped actions
  (master-count steppers for `master`, center-column for `scrolling`); focused
  window width presets ⅓/½/⅔/full, floating, pinned, fullscreen, maximize,
  center; and a workspace switcher where right-click moves the focused window
  **or its whole column** to that workspace.

Every action string it uses, verified against the compositor's keybind action
registry at Umbriel **current main** (`src/config/keybind_parse.cpp`):

| Plugin action | Notes |
|---|---|
| `workspace-set-layout:{scrolling\|dwindle\|master}` | Applies to the active workspace of the focused output; override lasts until config reload |
| `layout-master-count-increase` / `-decrease` | Renamed from `master-count-*` between plugin v1.1.0 and v1.1.3 |
| `column-center` | Scrolling-only; compositor rejects with "column-center requires the scrolling layout" |
| `window-set-primary-extent:{0.333\|0.5\|0.667\|1.0}` | Renamed from `window-set-width` (v1.1.0) — see the vocabulary note below |
| `window-toggle-floating` / `-pinned` / `-fullscreen` / `-maximize` | Panel reflects current state from the `windows` stream (v1.1.3 added focused-window identity: title, app id, w×h) |
| `window-center` | For floating windows |
| `workspace-switch:{selector}` | By workspace selector; complements the core shell's protocol-side next/prev |
| `window-move-to-workspace:{selector}` / `column-move-to-workspace:{selector}` | Right-click menu |

Availability gating: the widget hides itself unless `UMBRIEL_SOCKET` is set
**and** `umbriel` is on PATH; the panel is inert elsewhere. Every other
compositor simply sees nothing.

**Vocabulary note for this repository.** Umbriel commit `2f2e4e3a`
(2026-09-16, "adopt extent-based sizing vocabulary") renamed
`window-set-width`/`window-set-height` to
**`window-set-primary-extent`** / **`window-set-secondary-extent`**, both
`<fraction>`-argument actions on the *focused* area. The archived compositor
snapshot `7666af0a` postdates the rename. This belongs to the parent's Umbriel
research, but it directly affects the [longcat needs](longcat-needs0.gpt6a.md)
claim that the Rust builder mirrors the native focused-window action: there
are now *two* extent actions, and neither takes an explicit window target —
the companion plugin sends them blind and relies on focus, which is exactly
the focus-race gap that document analyzes.

## What Noctalia exports

### `noctalia msg` — the primary control surface

- **Socket**: `$XDG_RUNTIME_DIR/noctalia-$WAYLAND_DISPLAY.sock` (falls back to
  `/tmp` and `wayland-0`; no environment override variable — unlike Umbriel).
- **Protocol**: plain text, not JSON. The client sends a command line
  (optionally prefixed `cwd\x1e` so relative paths resolve against the
  caller); the service reads until EOF (legacy newline-terminated single
  commands still work), executes, and returns one `\n`-terminated string
  (human text, or JSON for `status`). Limits: 64 KiB command cap, 100 ms recv
  timeout, 2 s CLI-side socket timeouts.
- **Registry-driven**: handlers bind to commands declared in
  `src/cli/schema_msg.h`; `noctalia msg --help` prints the live set with
  signatures. Same registry serves socket IPC, in-process dispatch (config
  hooks, idle actions, bar gestures), and action-editor UI.
- **Invocation context**: socket commands have none; bar-widget gestures carry
  their output, so `workspace-switch` targets the gesturing monitor while a
  keybind targets the focused one. `cycles`-flagged commands (`workspace-switch`,
  `taskbar-cycle`, `power-cycle`) run once per scroll flick, not per notch.
- **Command families** (116 at current main): bar show/hide/layer/auto-hide/
  reserve; panel open/toggle/close with ids like `launcher`,
  `control-center`, plugin panels `author/plugin:panel`; dock; desktop and
  lockscreen widget editors; window-switcher (with `close|hide|hold` actions
  and `all|output|workspace|current` scopes at current main); workspace
  switch/alerts; notifications and DND; clipboard read/copy/clear; media;
  volume/mic/brightness/keyboard-backlight; night light; Wi-Fi/Bluetooth
  toggles; power profiles; DPMS; caffeine; screenshots (extensive per-call
  flags added at current main) and annotate; wallpaper set/next/random/get;
  color scheme and theme mode; templates; settings open/toggle (optionally at
  a plugin or widget); session actions `lock|suspend|lock-and-suspend|logout|
  reboot|reboot-uefi|shutdown`; config-reload; log level; `status` (small
  JSON: `barVisible`, `panelOpen`, `activePanelId`, `locked`); and plugin
  dispatch/management (`plugin author/plugin:entry target event payload`,
  `plugins list/enable/disable/update/source …`).
- **CLI verbs** beyond `msg`: `config`, `dmenu`, `plugins`, `theme`,
  `firefox-theme`, `completions`; `--daemon` backgrounds the shell.

### dmenu socket — a second service

`$XDG_RUNTIME_DIR/noctalia-dmenu-$WAYLAND_DISPLAY.sock` backs
`noctalia dmenu`: stdin newline-separated items presented in the launcher,
selection printed to stdout. A dedicated contract for script launchers.

### Plugin platform — the extension API

Luau plugins with a TOML manifest (`plugin_api` compatibility declaration,
level 32 in the archive snapshot's `docs/plugin-api.json`, level 39 at
**current main**), installed from git/path
sources with a plugin store, settings UI integration, and entries for bar
widgets, panels, services, desktop/lockscreen widgets. The host API relevant
here: `noctalia.runAsync` (argv or shell string, timeout, exit/stderr
callback), `noctalia.runStream` (long-lived child, per-line callback),
`noctalia.state.set/watch` (typed channels between entries),
`noctalia.processMatches` (process table matching), `noctalia.getenv`,
`noctalia.commandExists`, `noctalia.getSetting` (config by dotted path),
`noctalia.notifyError`, i18n, sandboxed HTTP, bounded file reads. `umbriel-companion`
is the canonical compositor-integration example; the plugin API is
self-contained and exports no compositor abstractions of its own.

### What Noctalia does *not* export

No D-Bus control interface (it is a D-Bus *consumer*: notifications, MPRIS,
tray, logind, UPower, NetworkManager, BlueZ, polkit agent); no JSON request
API; no event stream to third parties (the dmenu and msg sockets are
request/response); and — most relevant to the layout-control thread — **no
window-layout or compositor-action proxy**: a client wanting layout control
must talk to Umbriel itself, as the companion plugin does by shelling out.

## Implications for umbriel-rs and Longcat

1. **The shell is another IPC consumer, not a competitor.** Everything the
   core shell does over Umbriel IPC is within the protocol
   [catalog](../protocol/commands0.glm53m.md) this repository already models:
   subscribe families, `msg` action strings, `keyboard-layouts`. A Rust client
   can coexist; the compositor's per-family coalescing and per-request
   connections are shared conditions, not shell-specific behavior.
2. **Layout-control precedent is focus-based and plugin-level.** The only
   existing layout control over Umbriel sends `workspace-set-layout`,
   `window-set-primary-extent`, and move-to-workspace actions with no target
   IDs, from a poll-free event-driven Luau service. It demonstrates the
   observation half of the Longcat contract (full `workspaces`+`windows`
   streams suffice for UI) while living with the focus-race write gap — the
   same gap [longcat-needs](longcat-needs0.gpt6a.md) Thread 1B targets with
   explicit-target writes.
3. **The compositor's `workspaces` payload already names each workspace's
   layout mode**, and the plugin consumes it; the core shell does not. If
   Weave 1 (exact layout observation) lands, both consumers can adopt it
   without protocol competition.
4. **Sizing vocabulary moved**: `window-set-primary-extent` and
   `window-set-secondary-extent` are the current focused-area actions
   (renamed 2026-09-16). The parent's Umbriel pass should confirm the Rust
   action builders' names and semantics against this pair.
5. **Lockstep drift is real**: current shell main subscribes to `outputs` — an
   event family that does not exist in the archived compositor snapshot (six
   families there; nine at compositor current main). Any capability-discovery
   design (longcat-needs Thread 1D) must assume clients and compositor
   versions interleave; "unknown subscription event" is a real reply the
   companion plugin surfaces as an error string today.

## Connections

- [Layout-control needs](longcat-needs0.gpt6a.md): the requirements document
  this survey feeds; owns the Longcat/Umbriel/Kubeless decomposition and the
  proposed native protocol work.
- [Protocol catalog](../protocol/commands0.glm53m.md) and
  [source review](../protocol/review0.gpt6a.md): the wire facts this
  repository models; the shell's usage verified here is a consumer-side
  cross-check of that catalog.
- [zooming-kittens Umbriel investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md):
  the earlier consumer-side inquiry; its "no Rust client exists" claim is
  historical.
- Upstream: [noctalia](https://github.com/noctalia-dev/noctalia),
  [umbriel](https://github.com/noctalia-dev/umbriel),
  [official-plugins](https://github.com/noctalia-dev/official-plugins);
  docs at <https://docs.noctalia.dev/noctalia/ipc/> and
  `/noctalia/compositor-settings/umbriel/`.
