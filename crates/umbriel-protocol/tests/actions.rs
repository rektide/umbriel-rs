use umbriel_protocol::{
    action::{Action, no_argument::NoArgument},
    request::Request,
};

use umbriel_protocol::{
    REQUEST_LIMIT,
    action::{WorkspaceSelector, WorkspaceTarget, catalog},
    codec::{self, EncodeError},
};

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

// Independent miniature of the upstream workspace branch, checking semantic
// interpretation after JSON decoding, not just the builder's own Display.
fn workspace_meaning(text: &str) -> (Option<usize>, String, Option<&str>) {
    let arg = text.strip_prefix("workspace-switch:").unwrap();
    let (selector, output) = match arg.split_once('/') {
        Some((selector, output)) => {
            assert!(!output.is_empty() && !output.contains('/'));
            (selector, Some(output))
        }
        None => (arg, None),
    };
    if selector.starts_with('"') || selector.ends_with('"') {
        assert!(selector.starts_with('"') && selector.ends_with('"'));
        let name = &selector[1..selector.len() - 1];
        assert!(!name.is_empty() && !name.contains('"'));
        (None, name.to_owned(), output)
    } else if selector.bytes().all(|byte| byte.is_ascii_digit()) {
        let index = selector.parse().unwrap();
        assert!((1..=64).contains(&index));
        (Some(index), String::new(), output)
    } else {
        (None, selector.to_owned(), output)
    }
}

#[test]
fn workspace_names_preserve_literal_meaning_through_json() {
    let mut bytes = Vec::with_capacity(1024);
    let pointer = bytes.as_ptr();
    for name in [
        "2",
        "0",
        "0002",
        "65",
        "999999999999999999999999",
        "name:2",
        "雪🦀",
        "two words",
        " ",
        "a\"b",
        "back\\slash",
        "line\n\t\r\0",
        "+2",
        "２",
    ] {
        let target = WorkspaceTarget::new(WorkspaceSelector::name(name).unwrap())
            .on_output("out:雪\"\n\\")
            .unwrap();
        let action = Action::workspace_switch(target);
        action.request().encode(&mut bytes).unwrap();
        let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
        assert_eq!(decoded.cmd, "msg");
        assert_eq!(decoded.arg, action.as_str());
        let (index, decoded_name, output) = workspace_meaning(&decoded.arg);
        assert_eq!(index, None);
        assert_eq!(decoded_name, name);
        assert_eq!(output, target.output());
        assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
        assert_eq!(bytes.as_ptr(), pointer);
    }
    let action = Action::workspace_switch(WorkspaceSelector::name("2").unwrap());
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        b"{\"cmd\":\"msg\",\"arg\":\"workspace-switch:\\\"2\\\"\"}\n"
    );
}

#[test]
fn workspace_indexes_and_unqualified_targets_are_distinct() {
    for index in 1..=64 {
        let selector = WorkspaceSelector::index(index).unwrap();
        let action = Action::workspace_switch(selector);
        assert_eq!(
            workspace_meaning(action.as_str()),
            (Some(index), String::new(), None)
        );
        let target = WorkspaceTarget::new(selector).on_output("DP-1").unwrap();
        assert_eq!(target.selector(), selector);
        assert_eq!(
            workspace_meaning(Action::workspace_switch(target).as_str()),
            (Some(index), String::new(), Some("DP-1"))
        );
    }
}

#[test]
fn workspace_validation_matches_first_slash_and_quote_rules() {
    for index in [0, 65, usize::MAX] {
        assert!(WorkspaceSelector::index(index).is_err());
    }
    for name in ["", "/", "a/b", "\"", "\"2\"", "\"a", "a\""] {
        assert!(WorkspaceSelector::name(name).is_err(), "accepted {name:?}");
    }
    let target = WorkspaceTarget::new(WorkspaceSelector::index(2).unwrap());
    for output in ["", "/", "a/b", "a/b/c"] {
        assert!(target.on_output(output).is_err(), "accepted {output:?}");
    }
    assert_eq!(
        Action::workspace_switch(target.on_output("\"2\"").unwrap()).as_str(),
        "workspace-switch:2/\"2\""
    );
}

#[test]
fn prepared_action_outlives_inputs_and_reuses_text_and_output_storage() {
    let action = {
        let name = String::from("owned:雪");
        let output = String::from("DP-1");
        Action::workspace_switch(
            WorkspaceTarget::new(WorkspaceSelector::name(&name).unwrap())
                .on_output(&output)
                .unwrap(),
        )
    };
    let text_pointer = action.as_str().as_ptr();
    let mut bytes = Vec::with_capacity(256);
    let bytes_pointer = bytes.as_ptr();
    for _ in 0..10 {
        action.request().encode(&mut bytes).unwrap();
        assert_eq!(action.as_str().as_ptr(), text_pointer);
        assert_eq!(bytes.as_ptr(), bytes_pointer);
    }
}

#[test]
fn workspace_limits_apply_to_final_escaped_json_including_lf() {
    let mut bytes = Vec::new();
    Action::workspace_switch(WorkspaceSelector::name("x").unwrap())
        .request()
        .encode(&mut bytes)
        .unwrap();
    let overhead = bytes.len() - 1;
    let name = "x".repeat(REQUEST_LIMIT - overhead);
    let action = Action::workspace_switch(WorkspaceSelector::name(&name).unwrap());
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(bytes.len(), REQUEST_LIMIT);
    let capacity = bytes.capacity();
    for name in [
        format!("{name}x"),
        "\n".repeat((REQUEST_LIMIT - overhead) / 2 + 1),
    ] {
        let action = Action::workspace_switch(WorkspaceSelector::name(&name).unwrap());
        assert!(matches!(
            action.request().encode(&mut bytes),
            Err(EncodeError::TooLong)
        ));
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), capacity);
    }
    Action::overview_toggle()
        .request()
        .encode(&mut bytes)
        .unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn implemented_catalog_is_sorted_unique_and_categorized() {
    assert!(
        catalog::IMPLEMENTED
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name)
    );
    assert_eq!(catalog::OVERVIEW_TOGGLE.category, catalog::Category::None);
    assert_eq!(
        catalog::WORKSPACE_SWITCH.category,
        catalog::Category::Workspace
    );
}

#[test]
fn overview_toggle_uses_the_real_msg_encoder() {
    let action = Action::overview_toggle();
    assert_eq!(action.as_str(), "overview-toggle");
    assert_eq!(Action::from(NoArgument::OverviewToggle), action);
    let mut bytes = Vec::with_capacity(128);
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(bytes, b"{\"cmd\":\"msg\",\"arg\":\"overview-toggle\"}\n");
    let capacity = bytes.capacity();
    let pointer = bytes.as_ptr();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(bytes.capacity(), capacity);
    assert_eq!(bytes.as_ptr(), pointer);
    let mut raw = Vec::new();
    Request::Msg(action.as_str()).encode(&mut raw).unwrap();
    assert_eq!(bytes, raw);
}

#[test]
fn raw_actions_remain_an_explicit_unvalidated_escape_hatch() {
    let action = Action::raw("future-action:\"雪\"\n\\path");
    let mut bytes = Vec::new();
    action.request().encode(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        "{\"cmd\":\"msg\",\"arg\":\"future-action:\\\"雪\\\"\\n\\\\path\"}\n".as_bytes()
    );
}
