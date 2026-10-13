---
type: Review
title: Protocol baseline corrections and architectural consequences
description: Parent source review correcting request limits, empty subscription responses, dedup scope, and portable envelope handling before comparative assessment.
resource: design/protocol/review0.gpt6a.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Review returned protocol research against critical source branches before architectural assessment
  - resource: commands0.glm53m.md
    title: Independent GLM protocol catalog and reported read-only live probes
  - resource: https://github.com/noctalia-dev/umbriel/blob/7666af0a43560658e7b966befcbe47c101968e1c/src/server/ipc.cpp
    title: Read request handling, response preparation, subscribe construction, theme payload, and broadcast dedup/backlog branches
---

# Protocol baseline review

Read alongside the [command investigation](commands0.glm53m.md). It provides
a sufficient baseline for the requested tmux/kubeless assessment. Its connection-
role and sequential subscriber-reuse findings are useful; the following source-
reviewed qualifications govern implementation.

## Request-size accounting

`readRequest` appends the entire `recv` chunk, checks `input.size() > 65536`,
and **then** searches for LF (`ipc.cpp:240–255`). The limit includes any received
newline and any buffered following bytes. It is not a check of only the
pre-newline payload.

For one outstanding LF-terminated request with no buffered extra input, enforce
**at most 65536 bytes including LF** (65535 JSON bytes). The catalog's claim of
a legal 65537-byte newline-terminated request is incorrect. The server also
accepts an unterminated request on peer EOF (`:259–266`), but the normal client
should send LF. Oversized input is not cleared in the length-error branch;
discard a subscriber connection after this error instead of trying to reuse it.

## An empty subscription response exists

When subscribing only to `keyboard_layout` without a keyboard, `handleRequest`
builds an empty response string (`:373–402`). `prepareResponse` unconditionally
appends LF (`:275–278`). The server therefore sends **a bare newline** in this
case, while keeping the accepted subscription alive.

This is an exception to “every server line is one JSON envelope.” The framing
layer should expose the empty frame; the protocol session/driver can consume it
as the known empty initial response rather than treating it as malformed JSON.
It is not a successful command reply or a keyboard-state payload. A blank-frame
policy must not make a pending command complete.

There is no separate subscribe ack, unsubscribe, or universal end-of-initial-
state marker. A requested keyboard family may remain unseen indefinitely. Expose
per-family readiness/availability rather than promising that every requested
family must produce a first snapshot before a usable subscription exists.

## Broadcast dedup is not subscriber-wide uniqueness

The cached payload belongs to **broadcasting** (`:418–427`); initial and repeated
subscribe responses bypass that cache. A repeated subscription deliberately
re-sends current state. A fresh initial snapshot may also equal a later first
broadcast after the cache catches up.

Thus the catalog's “every delivered line is a distinct real state” overstates
the guarantee. Consumers must tolerate duplicate snapshots. Changes within one
compositor iteration can be coalesced away, so a stream is not a complete physical
transition history either. A continuity reducer compares identity; an inventory
materializer replaces family state; neither should require raw-byte uniqueness.

## Connection reuse is real, with constrained scheduling

The source and the agent's live P3 support ordinary sequential requests on an
accepted subscription. `writeResponse` restores reading rather than closing it
(`:299–304`). Keep one command outstanding, distinguish events from its reply,
and continue draining events while awaiting the reply.

Do not pipeline: buffered following lines are processed only after another
`recv` path makes progress, rather than being drained immediately after a reply.
The single-owner session driver is consequently a promising reusable mechanism
to seek in tmux-interface-control and kubeless. A persistent subscription is not
the same as an arbitrary multiplexed RPC channel.

## Envelope classification and schema precision

Alphabetical key order is verified for the current serializer, but it is not
a JSON-level guarantee or a negotiated Umbriel capability. Use an order-
independent checked envelope decoder. A first-key hint could optimize a pinned
wire fast path only with validation and fallback; it must not be the sole
discriminator. Likewise do not extract a family by blindly reading a suffix.

Dynamic `arg` values of many kinds can contain arbitrary text (including IDs,
output names and scratchpad names), and extensible `cmd`/`events` values also
require JSON escaping. Use the serializer for every dynamic JSON string, not
only spawn/workspace arguments.

The theme source has **10**, not 11, flat color-string fields (`:39–48`), plus
nested border/overview colors and corner radius. IDs should remain opaque at
the public boundary; the observed workspace `output:serial` spelling can have
an optional parser without becoming a universal type restriction.

The 256 KiB backlog eviction branch executes while a connection is already
responding (`:433–444`). It is not a universal maximum event-frame size; the
initial non-responding assignment and subscribe-response construction do not
apply that same check. Client inbound frame bounds are their own policy.

## Review standing

This review checked the cited source branches; it did not rerun the agent's
reported live probes. The investigation's color/tearing schemas and action-name
enumeration are not exhaustive enough to claim a completed typed API mapping,
but the top-level commands, families, argument categories, and lifecycle are
sufficient for the current architectural assessment.

The [current architecture](../architecture/architecture1.gpt6a.md) and
[assessment brief](../assessment/brief0.gpt6a.md) incorporate these constraints.
