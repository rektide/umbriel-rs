use umbriel_protocol::{
    REQUEST_LIMIT,
    action::{Action, ValidationError, WorkspaceSelector, WorkspaceTarget, catalog},
    codec::{self, EncodeError},
};

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

type Optional = fn(Option<&str>) -> Result<Action<'static>, ValidationError>;

#[test]
fn absent_outputs_are_bare_and_named_outputs_take_any_nonempty_text() {
    for family in [Action::dpms_on, Action::dpms_off] {
        let action = family(None).unwrap();
        let bare = action.as_str();
        // The bare spelling carries no colon: absence is not an empty suffix.
        assert!(!bare.contains(':'), "{bare}");
        let mut bytes = Vec::new();
        family(None).unwrap().request().encode(&mut bytes).unwrap();
        let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(decoded.cmd, "msg");
        assert!(!decoded.arg.ends_with(':'), "{}", decoded.arg);
    }
    assert_eq!(Action::dpms_on(None).unwrap().as_str(), "dpms-on");
    assert_eq!(Action::dpms_off(None).unwrap().as_str(), "dpms-off");

    // Standalone outputs are opaque: slashes, colons, and quotes are ordinary
    // characters, unlike workspace output qualifiers which reject slashes.
    for output in ["DP-1", "eDP-1", "a/b", "out:1", "\"q\"", "雪", " "] {
        assert_eq!(
            Action::dpms_on(Some(output)).unwrap().as_str(),
            format!("dpms-on:{output}")
        );
        assert_eq!(
            Action::dpms_off(Some(output)).unwrap().as_str(),
            format!("dpms-off:{output}")
        );
    }
    let target = WorkspaceTarget::new(WorkspaceSelector::index(1).unwrap());
    assert!(target.on_output("a/b").is_err());
    for family in [Action::dpms_on as Optional, Action::dpms_off] {
        assert_eq!(
            family(Some("")).unwrap_err(),
            ValidationError::Empty { argument: "output" }
        );
    }
}

#[test]
fn scratchpads_default_when_absent_and_take_opaque_names() {
    let families: [(Optional, &str); 5] = [
        (
            Action::scratchpad_focus_next as Optional,
            "scratchpad-focus-next",
        ),
        (Action::scratchpad_toggle, "scratchpad-toggle"),
        (
            Action::window_move_to_scratchpad,
            "window-move-to-scratchpad",
        ),
        (
            Action::window_restore_from_scratchpad,
            "window-restore-from-scratchpad",
        ),
        (Action::window_toggle_scratchpad, "window-toggle-scratchpad"),
    ];
    // Absence means the implicit default scratchpad, not a named empty one.
    for (family, name) in families {
        assert_eq!(family(None).unwrap().as_str(), name);
        assert_eq!(
            family(Some("")).unwrap_err(),
            ValidationError::Empty {
                argument: "scratchpad name"
            }
        );
    }
    // The upstream conformance spellings, plus arbitrary opaque text.
    for (family, name, scratchpad) in [
        (
            Action::scratchpad_focus_next as Optional,
            "scratchpad-focus-next",
            "terminal",
        ),
        (Action::scratchpad_toggle, "scratchpad-toggle", "terminal"),
        (
            Action::window_move_to_scratchpad,
            "window-move-to-scratchpad",
            "notes",
        ),
        (
            Action::window_restore_from_scratchpad,
            "window-restore-from-scratchpad",
            "music",
        ),
        (
            Action::window_toggle_scratchpad,
            "window-toggle-scratchpad",
            "chat",
        ),
    ] {
        assert_eq!(
            family(Some(scratchpad)).unwrap().as_str(),
            format!("{name}:{scratchpad}")
        );
    }
    for opaque in ["a/b:c\"d\"", "雪 ", "music:2"] {
        assert_eq!(
            Action::scratchpad_toggle(Some(opaque)).unwrap().as_str(),
            format!("scratchpad-toggle:{opaque}")
        );
    }
}

#[test]
fn window_ids_are_opaque_text_required_or_optional_per_family() {
    // Required ids.
    assert_eq!(
        Action::window_focus("abc123").unwrap().as_str(),
        "window-focus:abc123"
    );
    assert_eq!(
        Action::window_focus_warp("abc123").unwrap().as_str(),
        "window-focus-warp:abc123"
    );
    for opaque in ["0x1a2b 雪/:\"", "1"] {
        assert_eq!(
            Action::window_focus(opaque).unwrap().as_str(),
            format!("window-focus:{opaque}")
        );
    }
    // Optional ids.
    assert_eq!(Action::window_close(None).unwrap().as_str(), "window-close");
    assert_eq!(
        Action::window_close(Some("abc123")).unwrap().as_str(),
        "window-close:abc123"
    );
    assert_eq!(
        Action::window_toggle_floating(None).unwrap().as_str(),
        "window-toggle-floating"
    );
    assert_eq!(
        Action::window_toggle_floating(Some("abc123"))
            .unwrap()
            .as_str(),
        "window-toggle-floating:abc123"
    );
    for family in [
        Action::window_focus as fn(&str) -> Result<Action<'static>, ValidationError>,
        Action::window_focus_warp,
    ] {
        assert_eq!(
            family("").unwrap_err(),
            ValidationError::Empty {
                argument: "window id"
            }
        );
    }
    for family in [
        Action::window_close as Optional,
        Action::window_toggle_floating,
    ] {
        assert_eq!(
            family(Some("")).unwrap_err(),
            ValidationError::Empty {
                argument: "window id"
            }
        );
    }
}

#[test]
fn opaque_target_text_survives_json_roundtrip_verbatim() {
    let output = "out\"雪\"\n\\:\t/";
    let action = Action::dpms_on(Some(output)).unwrap();
    let mut bytes = Vec::new();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        "{\"cmd\":\"msg\",\"arg\":\"dpms-on:out\\\"雪\\\"\\n\\\\:\\t/\"}\n".as_bytes()
    );
    let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
    assert_eq!(decoded.cmd, "msg");
    assert_eq!(decoded.arg, action.as_str());
    assert_eq!(decoded.arg.strip_prefix("dpms-on:"), Some(output));
    assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
}

#[test]
fn target_specs_carry_wire_names_and_categories() {
    let specs: [(catalog::Spec, &str, catalog::Category); 11] = [
        (
            catalog::DPMS_OFF,
            "dpms-off",
            catalog::Category::OptionalOutput,
        ),
        (
            catalog::DPMS_ON,
            "dpms-on",
            catalog::Category::OptionalOutput,
        ),
        (
            catalog::SCRATCHPAD_FOCUS_NEXT,
            "scratchpad-focus-next",
            catalog::Category::OptionalScratchpad,
        ),
        (
            catalog::SCRATCHPAD_TOGGLE,
            "scratchpad-toggle",
            catalog::Category::OptionalScratchpad,
        ),
        (
            catalog::WINDOW_MOVE_TO_SCRATCHPAD,
            "window-move-to-scratchpad",
            catalog::Category::OptionalScratchpad,
        ),
        (
            catalog::WINDOW_RESTORE_FROM_SCRATCHPAD,
            "window-restore-from-scratchpad",
            catalog::Category::OptionalScratchpad,
        ),
        (
            catalog::WINDOW_TOGGLE_SCRATCHPAD,
            "window-toggle-scratchpad",
            catalog::Category::OptionalScratchpad,
        ),
        (
            catalog::WINDOW_FOCUS,
            "window-focus",
            catalog::Category::WindowId,
        ),
        (
            catalog::WINDOW_FOCUS_WARP,
            "window-focus-warp",
            catalog::Category::WindowId,
        ),
        (
            catalog::WINDOW_CLOSE,
            "window-close",
            catalog::Category::OptionalWindowId,
        ),
        (
            catalog::WINDOW_TOGGLE_FLOATING,
            "window-toggle-floating",
            catalog::Category::OptionalWindowId,
        ),
    ];
    for (spec, name, category) in specs {
        assert_eq!(spec.name, name);
        assert_eq!(spec.category, category);
    }
}

#[test]
fn long_output_names_hit_the_request_limit() {
    let mut bytes = Vec::new();
    Action::dpms_on(Some("x"))
        .unwrap()
        .request()
        .encode(&mut bytes)
        .unwrap();
    let overhead = bytes.len() - 1;
    let name = "x".repeat(REQUEST_LIMIT - overhead);
    let action = Action::dpms_on(Some(&name)).unwrap();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(bytes.len(), REQUEST_LIMIT);
    assert!(matches!(
        Action::dpms_on(Some(format!("{name}x").as_str()))
            .unwrap()
            .request()
            .encode(&mut bytes),
        Err(EncodeError::TooLong)
    ));
    assert!(bytes.is_empty());
    Action::window_close(None)
        .unwrap()
        .request()
        .encode(&mut bytes)
        .unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn target_actions_reuse_prepared_text_and_storage() {
    let action = Action::window_toggle_scratchpad(Some("terminal")).unwrap();
    let text_pointer = action.as_str().as_ptr();
    let mut bytes = Vec::with_capacity(128);
    let bytes_pointer = bytes.as_ptr();
    for _ in 0..10 {
        action.request().encode(&mut bytes).unwrap();
        assert_eq!(action.as_str().as_ptr(), text_pointer);
        assert_eq!(bytes.as_ptr(), bytes_pointer);
    }
    // The action outlives the borrowed scratchpad name it was built from.
    let owned = {
        let name = String::from("owned:雪");
        Action::window_move_to_scratchpad(Some(&name)).unwrap()
    };
    assert_eq!(owned.as_str(), "window-move-to-scratchpad:owned:雪");
}
