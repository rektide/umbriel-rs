---
type: Project
title: umbriel-rs
description: A Sonic-first Rust Umbriel protocol library with a sans-I/O core and separate client, state, and columnar concerns.
resource: README.md
status: draft
generated:
  at: 2026-10-13
  by: model:openai/gpt-6-astra
sources:
  - resource: opencode://ses_ecbf8c82affe2AMb3CQpUO3eoZ
    title: Establish the Rust project direction, protocol research, Sonic-first multi-crate design, and comparative assessment brief
  - resource: design/architecture/architecture1.gpt6a.md
    title: Current library-wide architecture and human-selected direction
  - resource: design/assessment/brief0.gpt6a.md
    title: Commission for learning from tmux-interface-control followed by kubeless
---

# umbriel-rs

A high-performance Rust implementation of the **Umbriel compositor's client
protocol**, using **sonic-rs** and a **sans-I/O core**.

The direction is a multi-crate workspace: wire protocol and framing, Unix client
drivers, optional materialized state, and optional Arrow/Polars output. Typed
queries, actions, and subscriptions are the main surface. A stable-active-window
stream is one example consumer.

Early development: the architecture is being grounded in Umbriel's source and
existing Rust protocol/control libraries. Detailed designs remain proposals;
performance has not yet been benchmarked.

## Read first

- [Protocol commands and connection semantics](design/protocol/commands0.glm53m.md)
  — source-grounded command, event, and action research. Read with the
  [source-review corrections](design/protocol/review0.gpt6a.md), especially
  request limits, empty subscription responses, and dedup scope.
- [Current architecture](design/architecture/architecture1.gpt6a.md)
  — Sonic-first whole-protocol design, crate boundaries, subscriptions,
  materialized state, and the Arrow/Polars direction.
- [Assessment brief](design/assessment/brief0.gpt6a.md)
  — read tmux-interface-control first, then kubeless, to identify reusable
  session, subscription, observation, and control mechanisms.

## Supporting research

- [JSON and buffer findings](design/performance/json0.gpt6a.md) — source-checked
  allocation/lifetime tradeoffs. Sonic has since been selected; comparative
  parser benchmarking is a future tool, not a development prerequisite.
- [Initial architecture](design/architecture/architecture0.gpt6a.md) — preserved
  predecessor, including detailed 750 ms stable-active-window semantics.
- [Earlier zooming-kittens Umbriel investigation](file:///home/rektide/src/zooming-kittens/design/umbriel/investigation0.glm53max.md)
  — protocol and Longcat research. zooming-kittens' existing compositor
  implementation is niri; this document is not an existing Umbriel client.
- [Shared ChatGPT discussion](https://chatgpt.com/s/t_6acdd415b14481918cce70cf0cec5e09)
  — read in full; motivates selective decoding and reusable buffers.
