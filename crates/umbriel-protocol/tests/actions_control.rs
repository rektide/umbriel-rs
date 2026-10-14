use umbriel_protocol::{
    action::{
        Action, LayoutSelection, QuitConfirmation, WorkspaceSelector, WorkspaceTarget, catalog,
    },
    codec,
};

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

#[test]
fn layout_selection_uses_exactly_the_four_lowercase_words() {
    let words = [
        (LayoutSelection::Scrolling, "scrolling"),
        (LayoutSelection::Dwindle, "dwindle"),
        (LayoutSelection::Master, "master"),
        (LayoutSelection::Toggle, "toggle"),
    ];
    for (selection, word) in words {
        assert_eq!(selection.as_str(), word);
        assert_eq!(selection.to_string(), word);
        assert_eq!(
            Action::workspace_set_layout(selection).as_str(),
            format!("workspace-set-layout:{word}")
        );
    }
    // The four words are distinct; misspellings like "spiral" or "Scrolling"
    // are unrepresentable because there is no text constructor.
    let distinct: std::collections::HashSet<_> = words.iter().map(|(_, word)| *word).collect();
    assert_eq!(distinct.len(), 4);
    assert_eq!(catalog::WORKSPACE_SET_LAYOUT.name, "workspace-set-layout");
    assert_eq!(
        catalog::WORKSPACE_SET_LAYOUT.category,
        catalog::Category::LayoutMode
    );
}

#[test]
fn quit_confirmation_is_an_explicit_binary_choice() {
    assert_eq!(
        Action::session_quit(QuitConfirmation::ConfirmFirst).as_str(),
        "session-quit"
    );
    assert_eq!(
        Action::session_quit(QuitConfirmation::Skip).as_str(),
        "session-quit:skip-confirmation"
    );
    // Arbitrary arguments such as "bogus" or "" are unrepresentable; the
    // choice is the enum, and the bare form has no colon suffix.
    assert_eq!(catalog::SESSION_QUIT.name, "session-quit");
    assert_eq!(
        catalog::SESSION_QUIT.category,
        catalog::Category::SkipConfirmation
    );
    let mut bytes = Vec::new();
    Action::session_quit(QuitConfirmation::Skip)
        .request()
        .encode(&mut bytes)
        .unwrap();
    assert_eq!(
        bytes,
        b"{\"cmd\":\"msg\",\"arg\":\"session-quit:skip-confirmation\"}\n"
    );
}

#[test]
fn workspace_movements_reuse_the_switch_target_grammar() {
    // Upstream conformance spellings, including the quoted numeric name.
    let by_index = WorkspaceTarget::new(WorkspaceSelector::index(2).unwrap());
    assert_eq!(
        Action::window_move_to_workspace(by_index).as_str(),
        "window-move-to-workspace:2"
    );
    let on_output = by_index.on_output("HDMI-A-1").unwrap();
    assert_eq!(
        Action::window_move_to_workspace(on_output).as_str(),
        "window-move-to-workspace:2/HDMI-A-1"
    );
    let quoted = WorkspaceTarget::new(WorkspaceSelector::name("2").unwrap())
        .on_output("DP-1")
        .unwrap();
    assert_eq!(
        Action::column_move_to_workspace(quoted).as_str(),
        "column-move-to-workspace:\"2\"/DP-1"
    );
    // A bare selector is accepted through the same Into as switch.
    assert_eq!(
        Action::column_move_to_workspace(WorkspaceSelector::index(3).unwrap()).as_str(),
        "column-move-to-workspace:3"
    );
    assert_eq!(
        Action::window_move_to_workspace(WorkspaceSelector::name("web").unwrap()).as_str(),
        "window-move-to-workspace:web"
    );
    // Movement targets face the same validation as switch targets.
    assert!(WorkspaceSelector::name("a/b").is_err());
    for (spec, name) in [
        (
            catalog::WINDOW_MOVE_TO_WORKSPACE,
            "window-move-to-workspace",
        ),
        (
            catalog::COLUMN_MOVE_TO_WORKSPACE,
            "column-move-to-workspace",
        ),
        (catalog::WORKSPACE_SWITCH, "workspace-switch"),
    ] {
        assert_eq!(spec.name, name);
        assert_eq!(spec.category, catalog::Category::Workspace);
    }
}

#[test]
fn control_actions_roundtrip_through_the_real_encoder() {
    let action = Action::window_move_to_workspace(
        WorkspaceTarget::new(WorkspaceSelector::name("a\"b:雪").unwrap())
            .on_output("DP-1")
            .unwrap(),
    );
    let mut bytes = Vec::new();
    action.request().encode(&mut bytes).unwrap();
    let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
    assert_eq!(decoded.cmd, "msg");
    assert_eq!(decoded.arg, "window-move-to-workspace:a\"b:雪/DP-1");
    assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);

    // Prepared text and storage are reused across sends.
    let layout = Action::workspace_set_layout(LayoutSelection::Toggle);
    let text_pointer = layout.as_str().as_ptr();
    let bytes_pointer = bytes.as_ptr();
    for _ in 0..10 {
        layout.request().encode(&mut bytes).unwrap();
        assert_eq!(layout.as_str().as_ptr(), text_pointer);
        assert_eq!(bytes.as_ptr(), bytes_pointer);
    }
    let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
    assert_eq!(decoded.arg, "workspace-set-layout:toggle");
}
