use umbriel_protocol::{
    REQUEST_LIMIT,
    action::{Action, ValidationError, catalog},
    codec::{self, EncodeError},
};

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

#[test]
fn spawn_preserves_all_text_after_the_first_colon() {
    for command in [
        "foot -e htop",
        "a:b:c",
        " leading and trailing ",
        "雪 \"q\" \\p\n\t\0",
        "window-focus:abc123",
        "submap:reset",
        "reset",
        "spawn:spawn",
        "bash -c 'echo hi; exit'",
    ] {
        let action = Action::spawn(command).unwrap();
        assert_eq!(action.as_str(), format!("spawn:{command}"));
        let mut bytes = Vec::new();
        action.request().encode(&mut bytes).unwrap();
        let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(decoded.cmd, "msg");
        // After JSON decoding, everything past the first colon is the command
        // verbatim: no shell reinterpretation, no family cross-matching.
        assert_eq!(decoded.arg.strip_prefix("spawn:"), Some(command));
        assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
    }
    assert_eq!(
        Action::spawn("").unwrap_err(),
        ValidationError::Empty {
            argument: "spawn command"
        }
    );
}

#[test]
fn spawn_text_is_escaped_only_by_the_encoder() {
    let action = Action::spawn("echo \"雪\"\n\\path").unwrap();
    let mut bytes = Vec::new();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        "{\"cmd\":\"msg\",\"arg\":\"spawn:echo \\\"雪\\\"\\n\\\\path\"}\n".as_bytes()
    );
}

#[test]
fn submap_rejects_only_disable_and_closing_bracket() {
    for name in [
        "resize",
        "reset",
        "name with spaces",
        "雪",
        "disabled",
        "disable ",
        "[open",
        "re:set",
    ] {
        assert!(name != "disable" && !name.contains(']'));
        let action = Action::submap(name).unwrap();
        assert_eq!(action.as_str(), format!("submap:{name}"));
    }
    assert_eq!(
        Action::submap("").unwrap_err(),
        ValidationError::Empty {
            argument: "submap name"
        }
    );
    for name in ["disable", "]", "invalid]name", "re]set"] {
        assert_eq!(
            Action::submap(name).unwrap_err(),
            ValidationError::InvalidSubmapName,
            "accepted {name:?}"
        );
    }
    // `reset` is ordinary accepted text that pops the current layer upstream.
    assert_eq!(Action::submap("reset").unwrap().as_str(), "submap:reset");
}

#[test]
fn command_specs_carry_wire_names_and_categories() {
    for (spec, name) in [(catalog::SPAWN, "spawn"), (catalog::SUBMAP, "submap")] {
        assert_eq!(spec.name, name);
        assert_eq!(spec.category, catalog::Category::Command);
    }
}

#[test]
fn spawn_commands_reach_but_do_not_exceed_the_request_limit() {
    let mut bytes = Vec::new();
    Action::spawn("x")
        .unwrap()
        .request()
        .encode(&mut bytes)
        .unwrap();
    // Fixed JSON syntax and LF, excluding the one measured command byte.
    let overhead = bytes.len() - 1;

    let exact = "x".repeat(REQUEST_LIMIT - overhead);
    let action = Action::spawn(&exact).unwrap();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(bytes.len(), REQUEST_LIMIT);
    let capacity = bytes.capacity();
    assert!(matches!(
        Action::spawn(format!("{}x", exact).as_str())
            .unwrap()
            .request()
            .encode(&mut bytes),
        Err(EncodeError::TooLong)
    ));
    assert!(bytes.is_empty());
    assert_eq!(bytes.capacity(), capacity);

    // Each double quote becomes an escaped pair, so growth is two bytes per
    // character; one step from the boundary stays within, the next exceeds.
    let quotes = "\"".repeat((REQUEST_LIMIT - overhead) / 2);
    Action::spawn(&quotes)
        .unwrap()
        .request()
        .encode(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= REQUEST_LIMIT);
    assert!(matches!(
        Action::spawn(format!("{quotes}\"").as_str())
            .unwrap()
            .request()
            .encode(&mut bytes),
        Err(EncodeError::TooLong)
    ));
    assert!(bytes.is_empty());

    // Recovery: the same storage still accepts an ordinary request.
    Action::submap("reset")
        .unwrap()
        .request()
        .encode(&mut bytes)
        .unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn command_actions_reuse_prepared_text_and_storage() {
    let action = Action::spawn("foot -e htop").unwrap();
    let text_pointer = action.as_str().as_ptr();
    let mut bytes = Vec::with_capacity(128);
    let bytes_pointer = bytes.as_ptr();
    for _ in 0..10 {
        action.request().encode(&mut bytes).unwrap();
        assert_eq!(action.as_str().as_ptr(), text_pointer);
        assert_eq!(bytes.as_ptr(), bytes_pointer);
    }
    let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
    assert_eq!(decoded.arg, "spawn:foot -e htop");
}
