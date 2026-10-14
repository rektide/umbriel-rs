//! Conformance for the shared action catalog and the bare (no-argument)
//! slice, against a fixture copied from the compositor source.
//!
//! Provenance: /home/rektide/a/noctalia-dev/umbriel at git commit
//! 7666af0a43560658e7b966befcbe47c101968e1c (2026-10-09). Rows transcribed
//! from the `kActionSpecs` table in `src/config/keybind_parse.cpp` (lines
//! 173–372); category spellings follow `ActionArgKind` in
//! `src/config/keybind_parse.h` (lines 259–271), where the default `None`
//! kind is omitted from table rows. Upstream's own
//! `payloadAlternativeMatchesTheDeclaredArgKind` unit test pins each row's
//! kind to the payload its parser produces. The fixture is a frozen copy:
//! nothing here reads that checkout at runtime.
use umbriel_protocol::action::catalog::{self, Category};
use umbriel_protocol::action::{Action, no_argument::NoArgument};
use umbriel_protocol::{codec, request::Request};

/// (wire name, argument category) for every upstream action, sorted by name.
const UPSTREAM: &[(&str, Category)] = &[
    ("cheatsheet-close", Category::None),
    ("cheatsheet-open", Category::None),
    ("cheatsheet-toggle", Category::None),
    ("column-center", Category::None),
    ("column-focus-first", Category::None),
    ("column-focus-last", Category::None),
    ("column-move-left", Category::None),
    ("column-move-right", Category::None),
    ("column-move-to-first", Category::None),
    ("column-move-to-last", Category::None),
    ("column-move-to-output-down", Category::None),
    ("column-move-to-output-left", Category::None),
    ("column-move-to-output-right", Category::None),
    ("column-move-to-output-up", Category::None),
    ("column-move-to-workspace", Category::Workspace),
    ("column-move-to-workspace-next", Category::None),
    ("column-move-to-workspace-previous", Category::None),
    ("config-reload", Category::None),
    ("dpms-off", Category::OptionalOutput),
    ("dpms-on", Category::OptionalOutput),
    ("keyboard-layout-next", Category::None),
    ("layout-master-count-decrease", Category::None),
    ("layout-master-count-increase", Category::None),
    ("layout-scroll-down", Category::None),
    ("layout-scroll-drag", Category::None),
    ("layout-scroll-left", Category::None),
    ("layout-scroll-right", Category::None),
    ("layout-scroll-up", Category::None),
    ("output-focus-down", Category::None),
    ("output-focus-left", Category::None),
    ("output-focus-next", Category::None),
    ("output-focus-previous", Category::None),
    ("output-focus-right", Category::None),
    ("output-focus-up", Category::None),
    ("overview-close", Category::None),
    ("overview-open", Category::None),
    ("overview-toggle", Category::None),
    ("scratchpad-focus-next", Category::OptionalScratchpad),
    ("scratchpad-toggle", Category::OptionalScratchpad),
    ("session-quit", Category::SkipConfirmation),
    ("shortcuts-inhibit-toggle", Category::None),
    ("spawn", Category::Command),
    ("submap", Category::Command),
    ("window-center", Category::None),
    ("window-close", Category::OptionalWindowId),
    ("window-consume-left", Category::None),
    ("window-consume-or-expel-left", Category::None),
    ("window-consume-or-expel-right", Category::None),
    ("window-consume-right", Category::None),
    ("window-cycle-primary-extent", Category::None),
    ("window-cycle-primary-extent-back", Category::None),
    ("window-cycle-secondary-extent", Category::None),
    ("window-cycle-secondary-extent-back", Category::None),
    ("window-focus", Category::WindowId),
    ("window-focus-down", Category::None),
    ("window-focus-last", Category::None),
    ("window-focus-left", Category::None),
    ("window-focus-next", Category::None),
    ("window-focus-or-output-down", Category::None),
    ("window-focus-or-output-left", Category::None),
    ("window-focus-or-output-right", Category::None),
    ("window-focus-or-output-up", Category::None),
    ("window-focus-or-workspace-down", Category::None),
    ("window-focus-or-workspace-up", Category::None),
    ("window-focus-previous", Category::None),
    ("window-focus-right", Category::None),
    ("window-focus-switch-floating", Category::None),
    ("window-focus-up", Category::None),
    ("window-focus-warp", Category::WindowId),
    ("window-modify-height-down", Category::FractionDelta),
    ("window-modify-height-up", Category::FractionDelta),
    ("window-modify-primary-extent", Category::FractionDelta),
    ("window-modify-secondary-extent", Category::FractionDelta),
    ("window-modify-width-left", Category::FractionDelta),
    ("window-modify-width-right", Category::FractionDelta),
    ("window-move-down", Category::None),
    ("window-move-or-output-down", Category::None),
    ("window-move-or-output-left", Category::None),
    ("window-move-or-output-right", Category::None),
    ("window-move-or-output-up", Category::None),
    ("window-move-or-workspace-down", Category::None),
    ("window-move-or-workspace-up", Category::None),
    ("window-move-to-output-down", Category::None),
    ("window-move-to-output-left", Category::None),
    ("window-move-to-output-next", Category::None),
    ("window-move-to-output-previous", Category::None),
    ("window-move-to-output-right", Category::None),
    ("window-move-to-output-up", Category::None),
    ("window-move-to-scratchpad", Category::OptionalScratchpad),
    ("window-move-to-workspace", Category::Workspace),
    ("window-move-to-workspace-next", Category::None),
    ("window-move-to-workspace-previous", Category::None),
    ("window-move-up", Category::None),
    (
        "window-restore-from-scratchpad",
        Category::OptionalScratchpad,
    ),
    ("window-set-primary-extent", Category::Fraction),
    ("window-set-secondary-extent", Category::Fraction),
    ("window-swap-next", Category::None),
    ("window-swap-previous", Category::None),
    ("window-toggle-floating", Category::OptionalWindowId),
    ("window-toggle-fullscreen", Category::None),
    ("window-toggle-maximize", Category::None),
    ("window-toggle-maximize-to-edges", Category::None),
    ("window-toggle-pinned", Category::None),
    ("window-toggle-scratchpad", Category::OptionalScratchpad),
    ("workspace-focus-last", Category::None),
    ("workspace-move-down", Category::None),
    ("workspace-move-to-output-down", Category::None),
    ("workspace-move-to-output-left", Category::None),
    ("workspace-move-to-output-right", Category::None),
    ("workspace-move-to-output-up", Category::None),
    ("workspace-move-up", Category::None),
    ("workspace-next", Category::None),
    ("workspace-previous", Category::None),
    ("workspace-set-layout", Category::LayoutMode),
    ("workspace-swap-active-output-down", Category::None),
    ("workspace-swap-active-output-left", Category::None),
    ("workspace-swap-active-output-next", Category::None),
    ("workspace-swap-active-output-previous", Category::None),
    ("workspace-swap-active-output-right", Category::None),
    ("workspace-swap-active-output-up", Category::None),
    ("workspace-switch", Category::Workspace),
];

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

#[test]
fn catalog_matches_the_pinned_upstream_table() {
    // Anchors against accidental truncation of the frozen fixture.
    assert_eq!(UPSTREAM.len(), 121);
    assert_eq!(
        UPSTREAM
            .iter()
            .filter(|(_, category)| *category == Category::None)
            .count(),
        95
    );
    for pair in UPSTREAM.windows(2) {
        assert!(pair[0].0 < pair[1].0, "fixture out of order at {pair:?}");
    }

    assert_eq!(catalog::IMPLEMENTED.len(), UPSTREAM.len());
    for (spec, (name, category)) in catalog::IMPLEMENTED.iter().zip(UPSTREAM) {
        assert_eq!(spec.name, *name, "spelling drift at {name}");
        assert_eq!(spec.category, *category, "category drift at {name}");
    }
}

#[test]
fn no_argument_enumerates_exactly_the_bare_actions() {
    let bare: Vec<&str> = catalog::IMPLEMENTED
        .iter()
        .filter(|spec| spec.category == Category::None)
        .map(|spec| spec.name)
        .collect();
    assert_eq!(NoArgument::ALL.len(), bare.len());
    for (variant, name) in NoArgument::ALL.iter().zip(&bare) {
        assert_eq!(variant.as_str(), *name);
    }

    // Actions that merely permit a bare spelling alongside an argument — the
    // optional-output, optional-scratchpad, optional-window-id and
    // skip-confirmation families — are parameterized families, not bare ones.
    for spec in catalog::IMPLEMENTED {
        if spec.category != Category::None {
            assert!(
                NoArgument::ALL
                    .iter()
                    .all(|variant| variant.as_str() != spec.name),
                "{} must not be a NoArgument variant",
                spec.name
            );
        }
    }
}

#[test]
fn bare_actions_encode_without_arguments_through_the_shared_encoder() {
    let mut bytes = Vec::with_capacity(128);
    let pointer = bytes.as_ptr();
    let mut decoded_one = false;
    for variant in NoArgument::ALL {
        let action = Action::from(*variant);
        let text = variant.as_str();
        assert_eq!(action.as_str(), text);
        assert!(!text.contains(':'), "{text} grew an argument");
        action.request().encode(&mut bytes).unwrap();
        let expected = format!("{{\"cmd\":\"msg\",\"arg\":\"{text}\"}}\n");
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), expected);
        if !decoded_one {
            let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
            assert_eq!((decoded.cmd.as_str(), decoded.arg.as_str()), ("msg", text));
            decoded_one = true;
        }
        bytes.clear();
    }
    assert_eq!(bytes.as_ptr(), pointer);

    // The From conversion and the named constructor agree on the starter.
    assert_eq!(
        Action::from(NoArgument::OverviewToggle),
        Action::overview_toggle()
    );
    let mut raw = Vec::new();
    Request::Msg(Action::from(NoArgument::OverviewToggle).as_str())
        .encode(&mut raw)
        .unwrap();
    let mut typed = Vec::new();
    Action::from(NoArgument::OverviewToggle)
        .request()
        .encode(&mut typed)
        .unwrap();
    assert_eq!(typed, raw);
}
