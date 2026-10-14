//! Coverage contract between the typed action paths and the shared catalog.
//!
//! The frozen fixture in `actions_no_argument.rs` proves the catalog's
//! spelling/category inventory against the compositor source. This test
//! closes the remaining gap: every declared spec has a usable typed path —
//! `NoArgument::ALL` for the bare slice, and one representative valid call
//! per parameterized builder. It is a completeness contract only; argument
//! validation and edge cases belong to the family test files.

use umbriel_protocol::action::catalog;
use umbriel_protocol::action::no_argument::NoArgument;
use umbriel_protocol::action::{
    Action, ExtentDelta, ExtentFraction, LayoutSelection, QuitConfirmation, WorkspaceSelector,
    WorkspaceTarget,
};

/// One valid action through every typed path: all bare variants plus each of
/// the 26 parameterized builders with representative arguments. Optional
/// families appear in both spellings across the set; each builder is invoked
/// exactly once.
fn representative_actions() -> Vec<Action<'static>> {
    let mut actions: Vec<Action<'static>> = NoArgument::ALL
        .iter()
        .map(|variant| Action::from(*variant))
        .collect();
    actions.extend([
        Action::spawn("foot").unwrap(),
        Action::submap("media").unwrap(),
        Action::workspace_set_layout(LayoutSelection::Dwindle),
        Action::session_quit(QuitConfirmation::Skip),
        Action::window_set_primary_extent(ExtentFraction::new(0.5).unwrap()),
        Action::window_set_secondary_extent(ExtentFraction::new(1.0).unwrap()),
        Action::window_modify_primary_extent(ExtentDelta::new(0.1).unwrap()),
        Action::window_modify_secondary_extent(ExtentDelta::new(-0.2).unwrap()),
        Action::window_modify_width_left(ExtentDelta::new(-0.15).unwrap()),
        Action::window_modify_width_right(ExtentDelta::new(0.25).unwrap()),
        Action::window_modify_height_up(ExtentDelta::new(0.3).unwrap()),
        Action::window_modify_height_down(ExtentDelta::new(-0.35).unwrap()),
        Action::dpms_on(Some("DP-1")).unwrap(),
        Action::dpms_off(None).unwrap(),
        Action::scratchpad_focus_next(Some("terms")).unwrap(),
        Action::scratchpad_toggle(None).unwrap(),
        Action::window_move_to_scratchpad(Some("terms")).unwrap(),
        Action::window_restore_from_scratchpad(None).unwrap(),
        Action::window_toggle_scratchpad(Some("music")).unwrap(),
        Action::window_focus("1").unwrap(),
        Action::window_focus_warp("1").unwrap(),
        Action::window_close(Some("1")).unwrap(),
        Action::window_toggle_floating(None).unwrap(),
        Action::workspace_switch(
            WorkspaceTarget::new(WorkspaceSelector::name("web").unwrap())
                .on_output("DP-1")
                .unwrap(),
        ),
        Action::window_move_to_workspace(WorkspaceSelector::index(2).unwrap()),
        Action::column_move_to_workspace(WorkspaceSelector::index(3).unwrap()),
    ]);
    actions
}

/// The action name is everything before the first colon; bare actions have no
/// colon, and no wire name contains one.
fn name_of<'a>(action: &'a Action<'_>) -> &'a str {
    action.as_str().split(':').next().unwrap()
}

#[test]
fn typed_paths_cover_the_whole_catalog_exactly_once() {
    let actions = representative_actions();
    assert_eq!(actions.len(), catalog::IMPLEMENTED.len());

    let declared: Vec<&str> = catalog::IMPLEMENTED.iter().map(|spec| spec.name).collect();
    let mut covered: Vec<&str> = actions.iter().map(name_of).collect();
    for name in &covered {
        assert!(
            declared.contains(name),
            "{name} is emitted by a typed path but has no catalog spec"
        );
    }

    // Sorted equality requires exactly one typed path per catalog entry:
    // omissions, duplicates, and invented names all fail here.
    covered.sort_unstable();
    let mut expected = declared;
    expected.sort_unstable();
    assert_eq!(
        covered, expected,
        "typed paths and catalog::IMPLEMENTED must cover the same action names"
    );
}
