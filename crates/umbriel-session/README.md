---
type: Reference
title: Umbriel session core
description: Bounded sans-I/O request ownership, write accounting, and connection roles.
resource: crates/umbriel-session/README.md
status: implemented
generated:
  at: 2026-10-14
  by: model:openai/gpt-6.1-sol#medium
sources:
  - resource: opencode://ses_ec773a6beffeBs3acUzwNtvSoZ
    title: Add validated prepared admission for bounded runtime ingress without another reply ledger
  - resource: opencode://ses_ec7a6b0aeffeO1NsW5IPjs6snU
    title: Implemented the bounded session ledger and deterministic ownership transcripts
  - resource: ../../design/protocol/review0.gpt6a.md
    title: Governing blank-frame, request-bound, and sequential-reuse corrections
  - resource: ../../design/protocol/commands0.glm53m.md
    title: Wire roles, subscription families, and reply semantics
  - resource: ../../design/assessment/reuse0.gpt6a.md#wire-ownership
    title: Sent requests retain reply ownership after waiter cancellation
  - resource: ../../design/architecture/architecture1.gpt6a.md#session-roles-and-command-outcomes
    title: Sans-I/O layer and explicit command uncertainty
  - resource: ../umbriel-protocol/src/codec.rs
    title: Existing checked Sonic encoder, decoder, and bounded framing APIs
  - resource: ../umbriel-protocol/src/envelope.rs
    title: Existing borrowed Message and event-family API
  - resource: ../umbriel-protocol/src/request.rs
    title: Existing typed Request encoder and raw-command escape hatch
verification:
  method: 30 deterministic nextest cases, one README doctest, crate fmt check, and all-targets Clippy with warnings denied.
  limits: No sockets, live compositor probes, timing benchmarks, or runtime adapter were used.
---

# Umbriel session core

`umbriel-session` owns one connection's protocol ledger, not its socket, clock,
futures, state inventory, or observer queues. It depends only on the protocol
crate and error derivation. One bounded encoded request is retained; receive
effects borrow the supplied frame and are returned immediately, without a queue
or a generic JSON tree. The protocol encoder enforces 65536 bytes including LF.

## Driving it

```rust
use umbriel_protocol::{envelope::Message, request::{Query, Request}};
use umbriel_session::{Effect, Session, Waiter};

let mut core = Session::one_shot(1024 * 1024); // inbound frame policy, excluding LF
let request_id = core.admit(Request::Query(Query::Submap))?;
let offered = core.begin_write()?;
// Adapter writes offered bytes; this example acknowledges the whole slice.
let written = offered.len();
core.write_progress(written)?;

match core.receive(br#"{"ok":null}"#)? {
    Effect::Reply { id, waiter: Waiter::Waiting, message: Message::Ok(value), .. } => {
        assert_eq!(id, request_id);
        // Decode value into the request's expected result or a custom projection.
        // A caller decode failure is local, not remote rejection or rollback.
        let _ = value;
    }
    other => panic!("unexpected effect: {other:?}"),
}
assert!(core.eof().unresolved.is_none());
# Ok::<(), umbriel_session::Error>(())
```

Use the protocol `Framer` for chunks, drain its complete frames through
`receive`, and handle each effect before reusing framing storage. Call `finish`
at EOF; report a truncated tail through
`protocol_failed(ProtocolFailure::Framing(error.to_string()))`, then `eof`. Adapter
framing limits and delivery byte/item bounds remain separate policies.

Always call `begin_write` before attempting I/O, then acknowledge **only** actual
progress through `write_progress`. Zero is valid and retains the bytes. A failed
or abandoned in-flight write is `transport_lost` unless the adapter can prove
and report its completed progress. Invalid progress does not mutate the ledger.
Do not feed a reply before accounting for the write that caused it.

`stop_waiting(id, Cancelled | TimedOut)` distinguishes unsent withdrawal from
drain-required retirement. A partial request still needs its suffix written, or
the connection closed. After any acknowledged byte or unresolved write attempt,
the old slot remains occupied. Its eventual `Reply` carries the old local ID and
`Waiter::Stopped`, allowing the adapter to discard delivery but preserve native
outcome evidence. Only then may another request enter. If an in-flight attempt
later reports zero and no earlier bytes were written, the unsent request is
withdrawn. Cancelling an entirely unsent initial subscribe closes that core.

## Roles and outcomes

- A one-shot reply retires its slot and enters `AwaitingEof`. No second command
  is admitted, even before the server's EOF arrives.
- `Session::subscription(families, limit)` immediately owns the initial subscribe
  request, whose ID is available through `pending_request`. The first matching
  event proves acceptance and is also delivered as data. There is no `ok` ack.
- Only a keyboard-only initial subscription may consume an exactly blank frame
  as acceptance. Keyboard status remains `Unseen`; this is not an empty layout.
  Blank frames elsewhere are fatal, and never complete an ordinary command.
- `Streaming` permits sequential ordinary commands with interleaved events.
  Readiness is per declared family, based on a validated envelope, **not** typed
  payload validity or an atomic all-family snapshot. It remains historical after
  termination. Initial and broadcast snapshots may duplicate.
- Unknown/unselected events on an accepted stream are returned unchanged for
  forward inspection, but do not mark declared families observed. They cannot
  establish fresh subscription acceptance. Dynamic/repeated subscribe, including
  raw `cmd: "subscribe"`, is explicitly unsupported; no unsubscribe exists.
- `Ok` acknowledges a command, never geometry convergence. `Err` retains remote
  rejection text. `"request too long"` additionally retires the potentially
  poisoned connection. Envelope/decode/unexpected-input failures are local
  protocol failures. Typed payload decode occurs downstream after reply drainage.
- EOF/transport/local close retains the first `Terminal` and any unresolved
  request's transmission and waiter facts. `possibly_sent: true` means outcome
  unknown, not executed or rolled back. Terminal methods are idempotent. Local
  request IDs and receive positions increment checked, never wrap, and are not
  server IDs, cursors, or cross-connection identities.

The single-slot law prevents a cancelled request's late reply from reaching a
later caller. It cannot detect a server duplicating an already-drained reply
after the next request has been fully sent: without wire IDs, that envelope is
indistinguishable from the new request's reply. Extra replies while no slot is
occupied are detected and terminate the connection.

`PreparedCommand::new(Request)` checks and encodes an ordinary command once,
before a runtime adapter queues it. `Session::admit_prepared` transfers that
bounded storage into the existing slot. There is no raw-byte constructor and
subscribe remains forbidden. This small admission extension leaves the original
ledger, write accounting, and deterministic transcripts intact; the predecessor
was already checkpointed before this extension.

The adapter must retain this core independently of disposable waiting futures,
continuously drain subscribed input, and close its socket when terminal appears.
No automatic reconnect, retry, deadlines, or delivery backpressure policy is
implemented here. No downstream state/Polars dependency is needed merely to
share these small local facts.

## Verification

Run `cargo nextest run -p umbriel-session`, `cargo fmt -p umbriel-session --check`,
and `cargo clippy -p umbriel-session --all-targets -- -D warnings`.
The tests use deterministic frames and write acknowledgements, not clocks or
live compositor mutations.
