use umbriel_protocol::{
    REQUEST_LIMIT,
    codec::{EncodeError, Framer, Limits},
    envelope::{EventName, Family, Message},
    request::{Query, Request},
};
use umbriel_session::{
    Effect, EndReason, Error, FamilyStatus, Phase, ProtocolFailure, Role, Session, StopDisposition,
    StopReason, Unexpected, Waiter,
};

const LIMIT: usize = 1024 * 1024;
const WINDOWS: &[u8] = br#"{"event":"windows","data":[]}"#;
const SUBMAP: &[u8] = br#"{"data":null,"event":"submap"}"#;
const OK: &[u8] = br#"{"ok":null}"#;
const ERR: &[u8] = br#"{"err":"remote reason"}"#;

fn flush(session: &mut Session) {
    let count = session.begin_write().unwrap().len();
    session.write_progress(count).unwrap();
    assert!(session.outbound().is_empty());
}

fn streaming() -> Session {
    let mut session = Session::subscription(&[Family::Windows, Family::Submap], LIMIT).unwrap();
    flush(&mut session);
    assert!(matches!(
        session.receive(WINDOWS).unwrap(),
        Effect::Event {
            accepted_subscription: Some(_),
            ..
        }
    ));
    session
}

fn expect_protocol(session: &mut Session, frame: &[u8], expected: Unexpected) {
    assert!(matches!(
        session.receive(frame),
        Err(Error::Protocol(ProtocolFailure::Unexpected(actual))) if actual == expected
    ));
    assert_eq!(session.phase(), Phase::Terminal);
    assert_eq!(
        session.terminal().unwrap().reason,
        EndReason::Protocol(ProtocolFailure::Unexpected(expected))
    );
}

#[test]
fn one_shot_success_acknowledges_then_expects_eof() {
    let mut session = Session::one_shot(LIMIT);
    assert_eq!(session.role(), Role::OneShot);
    let id = session.admit(Request::Query(Query::Submap)).unwrap();
    assert_eq!(session.outbound(), b"{\"cmd\":\"submap\"}\n");
    flush(&mut session);
    assert!(matches!(
        session.receive(OK).unwrap(),
        Effect::Reply { id: actual, waiter: Waiter::Waiting, message: Message::Ok(_), local_receive_position: 1 } if actual == id
    ));
    assert_eq!(session.phase(), Phase::AwaitingEof);
    assert!(matches!(
        session.admit(Request::Msg("noop")),
        Err(Error::NotReady)
    ));
    assert_eq!(session.eof().reason, EndReason::Eof);
    assert!(session.terminal().unwrap().unresolved.is_none());
}

#[test]
fn remote_error_is_not_local_failure_or_unknown_outcome() {
    let mut session = Session::one_shot(LIMIT);
    let id = session.admit(Request::Msg("unknown-action")).unwrap();
    flush(&mut session);
    match session.receive(ERR).unwrap() {
        Effect::Reply {
            id: actual,
            message: Message::Err(text),
            ..
        } => {
            assert_eq!(actual, id);
            assert_eq!(text, "remote reason");
        }
        other => panic!("{other:?}"),
    }
    assert!(session.eof().unresolved.is_none());
}

#[test]
fn eof_distinguishes_unsent_partial_full_and_unacknowledged_write() {
    for progress in [None, Some(0), Some(1), Some(usize::MAX)] {
        let mut session = Session::one_shot(LIMIT);
        let id = session.admit(Request::Msg("window-close")).unwrap();
        if let Some(progress) = progress {
            let offered = session.begin_write().unwrap().len();
            session.write_progress(progress.min(offered)).unwrap();
        }
        let unresolved = session.eof().unresolved.unwrap();
        assert_eq!(unresolved.id, id);
        assert_eq!(
            unresolved.possibly_sent,
            matches!(progress, Some(n) if n > 0)
        );
    }
    let mut session = Session::one_shot(LIMIT);
    session.admit(Request::Msg("window-close")).unwrap();
    session.begin_write().unwrap();
    assert!(
        session
            .transport_lost("unresolved write")
            .unresolved
            .unwrap()
            .possibly_sent
    );
}

#[test]
fn fresh_subscription_acceptance_has_independent_family_readiness() {
    let mut session = Session::subscription(
        &[Family::KeyboardLayout, Family::Windows, Family::Submap],
        LIMIT,
    )
    .unwrap();
    let id = session.pending_request().unwrap();
    assert_eq!(session.family_status(Family::Windows), FamilyStatus::Unseen);
    flush(&mut session);
    assert!(matches!(session.receive(WINDOWS).unwrap(), Effect::Event {
        accepted_subscription: Some((actual, Waiter::Waiting)), local_receive_position: 1, ..
    } if actual == id));
    assert_eq!(session.phase(), Phase::Streaming);
    assert_eq!(
        session.family_status(Family::Windows),
        FamilyStatus::Observed
    );
    assert_eq!(
        session.family_status(Family::KeyboardLayout),
        FamilyStatus::Unseen
    );
    assert_eq!(session.family_status(Family::Submap), FamilyStatus::Unseen);
    assert_eq!(
        session.family_status(Family::Theme),
        FamilyStatus::NotDeclared
    );
    // No wait-for-all barrier: commands can enter with keyboard still unseen.
    let command = session.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut session);
    assert!(matches!(
        session.receive(SUBMAP).unwrap(),
        Effect::Event {
            accepted_subscription: None,
            local_receive_position: 2,
            ..
        }
    ));
    assert_eq!(session.pending_request(), Some(command));
    assert!(matches!(session.receive(OK).unwrap(), Effect::Reply { id, .. } if id == command));
    assert_eq!(
        session.family_status(Family::Submap),
        FamilyStatus::Observed
    );
    session.eof();
    assert_eq!(
        session.family_status(Family::Windows),
        FamilyStatus::Observed
    );
}

#[test]
fn keyboard_only_blank_acceptance_is_not_keyboard_data() {
    // Duplicate declarations are still keyboard-only.
    let mut session = Session::subscription(&[Family::KeyboardLayout; 2], LIMIT).unwrap();
    flush(&mut session);
    assert!(matches!(
        session.receive(b"").unwrap(),
        Effect::SubscriptionAcceptedEmpty { .. }
    ));
    assert_eq!(session.phase(), Phase::Streaming);
    assert_eq!(
        session.family_status(Family::KeyboardLayout),
        FamilyStatus::Unseen
    );
    let id = session
        .admit(Request::Query(Query::KeyboardLayouts))
        .unwrap();
    flush(&mut session);
    assert!(
        matches!(session.receive(br#"{"err":"no keyboard"}"#).unwrap(), Effect::Reply { id: actual, .. } if actual == id)
    );
}

#[test]
fn blank_is_rejected_in_all_other_contexts() {
    let mut shot = Session::one_shot(LIMIT);
    shot.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut shot);
    expect_protocol(&mut shot, b"", Unexpected::Blank);
    for families in [
        vec![Family::Windows],
        vec![Family::Windows, Family::KeyboardLayout],
    ] {
        let mut session = Session::subscription(&families, LIMIT).unwrap();
        flush(&mut session);
        expect_protocol(&mut session, b"", Unexpected::Blank);
    }
    let mut session = Session::subscription(&[Family::KeyboardLayout], LIMIT).unwrap();
    flush(&mut session);
    session.receive(b"").unwrap();
    expect_protocol(&mut session, b"", Unexpected::Blank);
    let mut session = streaming();
    let id = session.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut session);
    expect_protocol(&mut session, b"", Unexpected::Blank);
    assert_eq!(session.terminal().unwrap().unresolved.unwrap().id, id);
}

#[test]
fn whitespace_is_decode_failure_not_blank_acceptance() {
    for frame in [b" ".as_slice(), b"\r", b"\t"] {
        let mut session = Session::subscription(&[Family::KeyboardLayout], LIMIT).unwrap();
        flush(&mut session);
        assert!(matches!(
            session.receive(frame),
            Err(Error::Protocol(ProtocolFailure::Decode(_)))
        ));
    }
}

#[test]
fn initial_rejection_is_terminal_but_preserves_remote_reason() {
    let mut session = Session::subscription(&[Family::Windows], LIMIT).unwrap();
    let id = session.pending_request().unwrap();
    flush(&mut session);
    assert!(
        matches!(session.receive(ERR).unwrap(), Effect::SubscriptionRejected {
        id: actual, message: Message::Err(_), ..
    } if actual == id)
    );
    assert_eq!(
        session.terminal().unwrap().reason,
        EndReason::SubscriptionRejected("remote reason".into())
    );
    assert!(session.eof().unresolved.is_none());
}

#[test]
fn subscription_requires_matching_event_and_never_ok() {
    for frame in [OK, SUBMAP, br#"{"event":"future_family","data":{}}"#] {
        let mut session = Session::subscription(&[Family::Windows], LIMIT).unwrap();
        flush(&mut session);
        assert!(matches!(session.receive(frame), Err(Error::Protocol(_))));
    }
}

#[test]
fn duplicates_and_unknown_events_are_routed_without_releasing_command() {
    let mut session = streaming();
    let id = session.admit(Request::Query(Query::Submap)).unwrap();
    // Events also route during partial write progress.
    session.begin_write().unwrap();
    session.write_progress(1).unwrap();
    for frame in [WINDOWS, WINDOWS, br#"{"event":"future_family","data":{}}"#] {
        assert!(matches!(
            session.receive(frame).unwrap(),
            Effect::Event {
                accepted_subscription: None,
                ..
            }
        ));
        assert_eq!(session.pending_request(), Some(id));
    }
    match session.receive(br#"{"event":"theme","data":{}}"#).unwrap() {
        Effect::Event {
            message:
                Message::Event {
                    name: EventName::Known(Family::Theme),
                    ..
                },
            ..
        } => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(
        session.family_status(Family::Theme),
        FamilyStatus::NotDeclared
    );
    flush(&mut session);
    assert!(
        matches!(session.receive(ERR).unwrap(), Effect::Reply { id: actual, .. } if actual == id)
    );
}

#[test]
fn partial_write_offsets_and_zero_progress_preserve_exact_suffix() {
    let mut session = Session::one_shot(LIMIT);
    session
        .admit(Request::Msg("spawn:printf 'a\\b\n雪'"))
        .unwrap();
    let encoded = session.outbound().to_vec();
    let mut written = Vec::new();
    assert!(matches!(
        session.write_progress(0),
        Err(Error::NoWriteInFlight)
    ));
    session.begin_write().unwrap();
    session.write_progress(0).unwrap();
    assert_eq!(session.outbound(), encoded);
    while !session.outbound().is_empty() {
        let offered = session.begin_write().unwrap();
        let count = offered.len().min(3);
        written.extend_from_slice(&offered[..count]);
        session.write_progress(count).unwrap();
        assert_eq!(session.outbound(), &encoded[written.len()..]);
    }
    assert_eq!(written, encoded);
    assert!(matches!(session.begin_write(), Err(Error::NoWrite)));
    session.receive(OK).unwrap();
}

#[test]
fn invalid_write_progress_is_atomic_and_attempts_do_not_overlap() {
    let mut session = Session::one_shot(LIMIT);
    session.admit(Request::Query(Query::Windows)).unwrap();
    let bytes = session.begin_write().unwrap().to_vec();
    assert!(matches!(session.begin_write(), Err(Error::WriteInFlight)));
    assert!(matches!(
        session.write_progress(bytes.len() + 1),
        Err(Error::InvalidProgress { .. })
    ));
    assert_eq!(session.outbound(), bytes);
    assert!(matches!(session.begin_write(), Err(Error::WriteInFlight)));
    session.write_progress(bytes.len()).unwrap();
    session.receive(OK).unwrap();
}

#[test]
fn unsent_cancellation_withdraws_without_consuming_connection() {
    let mut session = Session::one_shot(LIMIT);
    let old = session.admit(Request::Msg("window-close")).unwrap();
    assert_eq!(
        session.stop_waiting(old, StopReason::Cancelled).unwrap(),
        StopDisposition::Withdrawn
    );
    assert!(session.outbound().is_empty());
    assert!(session.pending_request().is_none());
    let new = session.admit(Request::Query(Query::Windows)).unwrap();
    assert!(new.get() > old.get());
    assert!(matches!(
        session.stop_waiting(old, StopReason::TimedOut),
        Err(Error::WrongRequest)
    ));
    flush(&mut session);
    assert!(
        matches!(session.receive(OK).unwrap(), Effect::Reply { id, waiter: Waiter::Waiting, .. } if id == new)
    );
}

#[test]
fn cancellation_and_timeout_after_progress_keep_ownership_until_late_reply() {
    for reason in [StopReason::Cancelled, StopReason::TimedOut] {
        for remote_reply in [OK, ERR] {
            let mut session = streaming();
            let old = session.admit(Request::Msg("window-close")).unwrap();
            session.begin_write().unwrap();
            session.write_progress(1).unwrap();
            assert_eq!(
                session.stop_waiting(old, reason).unwrap(),
                StopDisposition::DrainRequired
            );
            assert!(matches!(
                session.stop_waiting(old, reason),
                Err(Error::AlreadyStopped)
            ));
            assert!(matches!(
                session.admit(Request::Query(Query::Windows)),
                Err(Error::Busy)
            ));
            session.receive(WINDOWS).unwrap();
            flush(&mut session); // cancellation must not discard the partial suffix
            assert!(matches!(
                session.admit(Request::Query(Query::Windows)),
                Err(Error::Busy)
            ));
            assert!(
                matches!(session.receive(remote_reply).unwrap(), Effect::Reply {
                id, waiter: Waiter::Stopped(actual), ..
            } if id == old && actual == reason)
            );
            let next = session.admit(Request::Query(Query::Windows)).unwrap();
            assert_ne!(next, old);
            flush(&mut session);
            assert!(
                matches!(session.receive(OK).unwrap(), Effect::Reply { id, waiter: Waiter::Waiting, .. } if id == next)
            );
        }
    }
}

#[test]
fn cancellation_during_write_requires_resolution_even_before_acknowledged_progress() {
    let mut session = streaming();
    let id = session.admit(Request::Msg("window-close")).unwrap();
    session.begin_write().unwrap();
    assert_eq!(
        session.stop_waiting(id, StopReason::Cancelled).unwrap(),
        StopDisposition::DrainRequired
    );
    assert!(matches!(
        session.admit(Request::Query(Query::Windows)),
        Err(Error::Busy)
    ));
    session.write_progress(0).unwrap();
    assert!(session.pending_request().is_none());
    session.admit(Request::Query(Query::Windows)).unwrap();

    let mut session = streaming();
    let id = session.admit(Request::Msg("window-close")).unwrap();
    session.begin_write().unwrap();
    session.stop_waiting(id, StopReason::TimedOut).unwrap();
    let unknown = session
        .transport_lost("write result lost")
        .unresolved
        .unwrap();
    assert!(unknown.possibly_sent);
    assert_eq!(unknown.waiter, Waiter::Stopped(StopReason::TimedOut));
}

#[test]
fn detached_subscription_still_accepts_or_rejects_on_its_own_boundary() {
    let mut session = Session::subscription(&[Family::Windows], LIMIT).unwrap();
    let id = session.pending_request().unwrap();
    flush(&mut session);
    session.stop_waiting(id, StopReason::TimedOut).unwrap();
    assert!(matches!(session.receive(WINDOWS).unwrap(), Effect::Event {
        accepted_subscription: Some((actual, Waiter::Stopped(StopReason::TimedOut))), ..
    } if actual == id));
    assert_eq!(session.phase(), Phase::Streaming);
    let mut unsent = Session::subscription(&[Family::Windows], LIMIT).unwrap();
    let id = unsent.pending_request().unwrap();
    assert_eq!(
        unsent.stop_waiting(id, StopReason::Cancelled).unwrap(),
        StopDisposition::Withdrawn
    );
    assert_eq!(unsent.phase(), Phase::Terminal);
    assert!(!unsent.terminal().unwrap().unresolved.unwrap().possibly_sent);
}

#[test]
fn replies_before_complete_write_and_double_replies_are_terminal() {
    let mut unsent = Session::one_shot(LIMIT);
    unsent.admit(Request::Query(Query::Windows)).unwrap();
    expect_protocol(&mut unsent, OK, Unexpected::ReplyBeforeWriteComplete);
    let mut partial = Session::one_shot(LIMIT);
    partial.admit(Request::Query(Query::Windows)).unwrap();
    partial.begin_write().unwrap();
    partial.write_progress(1).unwrap();
    expect_protocol(&mut partial, ERR, Unexpected::ReplyBeforeWriteComplete);
    for mut session in [Session::one_shot(LIMIT), streaming()] {
        session.admit(Request::Query(Query::Windows)).unwrap();
        flush(&mut session);
        session.receive(OK).unwrap();
        expect_protocol(&mut session, ERR, Unexpected::Reply);
    }
    let mut idle = streaming();
    expect_protocol(&mut idle, OK, Unexpected::Reply);
}

#[test]
fn one_shot_events_never_acknowledge_a_command() {
    let mut session = Session::one_shot(LIMIT);
    session.admit(Request::Query(Query::Windows)).unwrap();
    flush(&mut session);
    expect_protocol(&mut session, WINDOWS, Unexpected::Event);
    assert!(
        session
            .terminal()
            .unwrap()
            .unresolved
            .unwrap()
            .possibly_sent
    );
}

#[test]
fn poisoned_request_rejection_drains_reply_and_retires_connection() {
    let mut session = streaming();
    let id = session.admit(Request::Msg("noop")).unwrap();
    flush(&mut session);
    assert!(
        matches!(session.receive(br#"{"err":"request too long"}"#).unwrap(), Effect::Reply { id: actual, message: Message::Err(_), .. } if actual == id)
    );
    assert_eq!(
        session.terminal().unwrap().reason,
        EndReason::RequestBufferPoisoned
    );
    assert!(session.terminal().unwrap().unresolved.is_none());
    assert!(matches!(
        session.admit(Request::Query(Query::Windows)),
        Err(Error::Terminal)
    ));
}

#[test]
fn terminal_is_first_wins_and_no_further_transitions_occur() {
    let mut session = streaming();
    let id = session.admit(Request::Msg("window-close")).unwrap();
    flush(&mut session);
    session.stop_waiting(id, StopReason::TimedOut).unwrap();
    let first = session.transport_lost("peer disappeared").clone();
    assert_eq!(session.eof(), &first);
    assert_eq!(session.close(), &first);
    assert_eq!(
        session.protocol_failed(ProtocolFailure::Decode("later".into())),
        &first
    );
    assert_eq!(session.transport_lost("later"), &first);
    assert!(session.outbound().is_empty());
    assert!(session.pending_request().is_none());
    assert!(matches!(session.receive(OK), Err(Error::Terminal)));
    assert!(matches!(session.write_progress(0), Err(Error::Terminal)));
    assert!(matches!(session.begin_write(), Err(Error::Terminal)));
    assert!(matches!(
        session.stop_waiting(id, StopReason::Cancelled),
        Err(Error::Terminal)
    ));
    assert!(matches!(
        session.admit(Request::Query(Query::Windows)),
        Err(Error::Terminal)
    ));
}

#[test]
fn encoded_limit_is_inclusive_and_admission_failure_is_atomic() {
    let mut session = Session::one_shot(LIMIT);
    let empty_len = {
        let id = session.admit(Request::Msg("")).unwrap();
        let count = session.outbound().len();
        session.stop_waiting(id, StopReason::Cancelled).unwrap();
        count
    };
    let largest = "x".repeat(REQUEST_LIMIT - empty_len);
    let id = session.admit(Request::Msg(&largest)).unwrap();
    assert_eq!(session.outbound().len(), REQUEST_LIMIT);
    session.stop_waiting(id, StopReason::Cancelled).unwrap();
    assert!(matches!(
        session.admit(Request::Msg(&(largest + "x"))),
        Err(Error::Encode(EncodeError::TooLong))
    ));
    assert!(session.outbound().is_empty());
    assert!(session.pending_request().is_none());
    assert_eq!(session.phase(), Phase::Fresh);
    assert_eq!(
        session.admit(Request::Msg("small")).unwrap().get(),
        id.get() + 1
    );
}

#[test]
fn subscription_and_raw_subscribe_cannot_enter_ordinary_slot() {
    assert!(matches!(
        Session::subscription(&[], LIMIT),
        Err(Error::Encode(_))
    ));
    for mut session in [Session::one_shot(LIMIT), streaming()] {
        for request in [
            Request::Subscribe(&[Family::Windows]),
            Request::Raw {
                cmd: "subscribe",
                arg: None,
            },
        ] {
            assert!(matches!(
                session.admit(request),
                Err(Error::UnsupportedSubscribe)
            ));
            assert!(session.pending_request().is_none());
            assert!(session.outbound().is_empty());
        }
    }
}

#[test]
fn decode_failure_and_inbound_policy_leave_unknown_native_outcome() {
    for frame in [br#"{"ok":null,"err":"conflict"}"#.as_slice(), b"{", b"\xff"] {
        let mut session = Session::one_shot(LIMIT);
        let id = session.admit(Request::Msg("window-close")).unwrap();
        flush(&mut session);
        assert!(matches!(
            session.receive(frame),
            Err(Error::Protocol(ProtocolFailure::Decode(_)))
        ));
        assert!(matches!(
            session.terminal().unwrap().reason,
            EndReason::Protocol(ProtocolFailure::Decode(_))
        ));
        let unresolved = session.terminal().unwrap().unresolved.unwrap();
        assert_eq!(unresolved.id, id);
        assert!(unresolved.possibly_sent);
    }
    let mut session = Session::one_shot(OK.len() - 1);
    session.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut session);
    assert!(matches!(
        session.receive(OK),
        Err(Error::Protocol(ProtocolFailure::FrameTooLong { .. }))
    ));
    let mut session = Session::one_shot(OK.len());
    session.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut session);
    session.receive(OK).unwrap();
}

#[test]
fn framer_transcript_drains_interleaved_frames_before_eof_and_detects_tail() {
    let mut session = streaming();
    let id = session.admit(Request::Query(Query::Submap)).unwrap();
    flush(&mut session);
    let mut framer = Framer::new(Limits {
        frame: LIMIT,
        buffered: LIMIT,
    });
    let transcript = b"{\"event\":\"windows\",\"data\":[]}\n{\"ok\":null}\n{\"data\":null,\"event\":\"submap\"}\n";
    let mut effects = 0;
    for chunk in transcript.chunks(5) {
        framer.push(chunk).unwrap();
        while let Some(frame) = framer.next_frame() {
            match session.receive(frame).unwrap() {
                Effect::Reply { id: actual, .. } => assert_eq!(actual, id),
                Effect::Event { .. } => {}
                other => panic!("{other:?}"),
            }
            effects += 1;
        }
    }
    assert_eq!(effects, 3);
    framer.finish().unwrap();
    assert!(session.eof().unresolved.is_none());

    let mut session = Session::one_shot(LIMIT);
    session.admit(Request::Msg("window-close")).unwrap();
    flush(&mut session);
    framer.push(b"{\"ok\":nu").unwrap();
    let error = framer.finish().unwrap_err();
    session.protocol_failed(ProtocolFailure::Decode(error.to_string()));
    assert!(session.eof().unresolved.unwrap().possibly_sent);
    assert!(matches!(
        session.terminal().unwrap().reason,
        EndReason::Protocol(_)
    ));
}
