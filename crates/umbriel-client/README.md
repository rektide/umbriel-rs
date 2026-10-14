---
type: Reference
title: Umbriel Tokio Unix client
description: On-demand one-shot sockets and separately driven bounded subscriptions.
resource: crates/umbriel-client/README.md
status: draft
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec773a6beffeBs3acUzwNtvSoZ
    title: Implement Unix discovery, one-shot execution, and independently bounded subscription ownership
  - resource: ../umbriel-session/README.md
    title: Single wire slot and transmission uncertainty
  - resource: ../../design/architecture/architecture1.gpt6a.md
    title: Runtime and protocol boundary
  - resource: ../../design/protocol/review0.gpt6a.md
    title: Initial keyboard blank, sequential commands, and framing boundaries
  - resource: ../../design/assessment/reuse0.gpt6a.md
    title: Independent bounded observers and explicit reconnect gaps
  - resource: file:///home/rektide/archive/noctalia-dev/umbriel/src/cli/ipc_client.cpp
    title: CLI environment precedence and byte-preserving socket name construction
---

# Umbriel client

Tokio Unix sockets, with no server launch, idle pool, state store, or Polars
dependency. `Discovery` accepts injected OS strings; `from_env` is the optional
process-environment boundary. Explicit path wins, followed by nonempty
`UMBRIEL_SOCKET`, then literal `XDG_RUNTIME_DIR + "/umbriel-" + WAYLAND_DISPLAY +
".sock"`, matching the CLI rather than assuming display names are basenames.

One-shot calls validate before connecting, account partial writes through the
session core, read the native reply, and wait for clean EOF within a deadline.
A truncated EOF tail remains a protocol error even after a native reply.
`RawReply::decode` selects a borrowed projection or owned result; remote rejection
and local decode failure are distinct. Connection failures are provably unsent;
terminal/deadline errors expose possible transmission and native-reply evidence.
