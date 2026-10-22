use serde::{
    Deserialize, Serialize,
    de::{
        DeserializeOwned,
        value::{BorrowedStrDeserializer, Error, StrDeserializer, StringDeserializer},
    },
};
use umbriel_protocol::{
    codec,
    envelope::{Family, Message},
    model::*,
};

const WINDOWS: &str = include_str!("fixtures/models/windows.json");
const NO_ACTIVE: &str = include_str!("fixtures/models/no-active-windows.json");
const WORKSPACES: &str = include_str!("fixtures/models/workspaces.json");
const LAYERS: &str = include_str!("fixtures/models/layers.json");
const COLOR: &str = include_str!("fixtures/models/color.json");
const TEARING: &str = include_str!("fixtures/models/tearing.json");
const THEME: &str = include_str!("fixtures/models/theme.json");
const OVERVIEW: &str = include_str!("fixtures/models/overview.json");
const KEYBOARD: &str = include_str!("fixtures/models/keyboard-layouts.json");
const SUBMAP: &str = include_str!("fixtures/models/submap.json");
const DEFAULT_SUBMAP: &str = include_str!("fixtures/models/submap-default.json");

fn query<T: DeserializeOwned>(payload: &str) -> T {
    let frame = format!("{{\"ok\":{payload}}}");
    let Message::Ok(data) = Message::decode(frame.as_bytes()).unwrap() else {
        panic!("expected successful query reply");
    };
    codec::decode(data.as_raw_str().as_bytes()).unwrap()
}

fn roundtrip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = sonic_rs::to_vec(value).unwrap();
    assert_eq!(&codec::decode::<T>(&encoded).unwrap(), value);
}

#[test]
fn windows_preserve_activation_focus_identity_and_signed_geometry() {
    let windows: Windows = query(WINDOWS);
    assert!(!windows[0].active && windows[0].focused);
    assert!(windows[1].active && !windows[1].focused);
    assert_eq!(windows[0].id.as_str(), "synthetic-window-α");
    assert_eq!(windows[0].workspace.as_str(), "SYNTH-1:42");
    assert_eq!(windows[0].title, "Notes 雪\n\"quoted\" \\ path");
    assert_eq!(windows[0].xdg_tag, "tag\tα");
    assert_eq!((windows[0].x, windows[0].y), (-900, -12));
    assert_eq!((windows[0].w, windows[0].h), (1000, 800));
    assert_eq!(windows[1].pid, -1);
    assert_eq!(windows[1].workspace.as_str(), "");
    assert_eq!(windows[1].id.as_str(), "");
    assert_eq!(windows[1].content_type.as_str(), "future-content");
    assert!(matches!(windows[1].content_type, ContentType::Unknown(_)));
    roundtrip(&windows);

    let no_active: Windows = query(NO_ACTIVE);
    assert!(!no_active.is_empty());
    assert!(!no_active.iter().any(|window| window.active));
    assert!(no_active[0].focused);
    assert!(query::<Windows>("[]").is_empty());
    assert_eq!(WindowId::new(""), WindowId::from(String::new()));
    assert_eq!(
        WorkspaceId::from("opaque/not:parsed").into_string(),
        "opaque/not:parsed"
    );
}

#[test]
fn workspaces_do_not_infer_named_or_focus_from_other_fields() {
    let workspaces: Workspaces = query(WORKSPACES);
    assert_eq!(workspaces[0].name, "2");
    assert!(workspaces[0].named);
    assert_eq!(workspaces[0].index, 1);
    assert!(workspaces[0].active && workspaces[0].focused);
    assert!(workspaces[1].active && !workspaces[1].focused);
    assert!(!workspaces[1].named && !workspaces[1].occupied);
    assert_eq!(workspaces[1].id.as_str(), "opaque/雪:identity");
    assert_eq!(workspaces[1].layout.as_str(), "future-layout");
    roundtrip(&workspaces);
    assert!(query::<Workspaces>("[]").is_empty());
}

#[test]
fn query_only_results_have_complete_nested_schemas() {
    let layers: Layers = query(LAYERS);
    assert_eq!(layers[0].layer, Layer::Overlay);
    assert_eq!(layers[1].layer.as_str(), "future-layer");
    assert!(!layers[1].mapped);
    assert_eq!(layers[1].output, "");
    roundtrip(&layers);

    let color: Color = query(COLOR);
    assert!(color.color_manager && color.renderer.input_color_transform);
    assert!(!color.renderer.output_color_transform);
    assert!(color.renderer.timeline);
    assert_eq!(color.outputs[0].hdr_mode, HdrMode::Auto);
    assert_eq!(color.outputs[0].sdr_white, 203.5);
    assert_eq!(color.outputs[0].render_format, "XR30");
    assert_eq!(color.outputs[0].supported_transfer_functions.len(), 5);
    assert_eq!(color.outputs[1].hdr_mode.as_str(), "future-hdr");
    let hdr = &color.surfaces[0];
    assert_eq!(
        hdr.mastering_display_primaries.as_ref().unwrap().white.x,
        0.3127
    );
    assert_eq!(hdr.mastering_luminance.as_ref().unwrap().min, 0.005);
    assert_eq!((hdr.max_cll, hdr.max_fall), (1000, 400));
    assert_eq!(hdr.transfer_function, TransferFunction::Hlg);
    assert!(color.surfaces[1].mastering_luminance.is_none());
    assert!(color.surfaces[1].mastering_display_primaries.is_none());
    assert_eq!(color.surfaces[1].primaries.as_str(), "future-primaries");
    assert_eq!(
        color.surfaces[1].transfer_function.as_str(),
        "future-transfer"
    );
    roundtrip(&color);

    let tearing: Tearing = query(TEARING);
    assert!(tearing.protocol);
    assert_eq!(tearing.outputs[0].last_presentation_flags, Some(2147483649));
    assert_eq!(tearing.outputs[0].last_presentation_vsync, Some(false));
    assert_eq!(tearing.outputs[1].last_presentation_presented, None);
    assert_eq!(tearing.outputs[2].last_presentation_presented, Some(false));
    assert_eq!(tearing.outputs[2].last_presentation_flags, Some(0));
    assert_eq!(tearing.surfaces[0].rule_override, Some(true));
    assert_eq!(tearing.surfaces[1].rule_override, Some(false));
    assert_eq!(tearing.surfaces[2].rule_override, None);
    assert_eq!(tearing.surfaces[2].hint.as_str(), "future-hint");
    roundtrip(&tearing);

    assert!(query::<Layers>("[]").is_empty());
    let empty_color: Color = query(
        r#"{"color_manager":false,"renderer":{"input_color_transform":false,"output_color_transform":false,"timeline":false},"outputs":[],"surfaces":[]}"#,
    );
    assert!(empty_color.outputs.is_empty() && empty_color.surfaces.is_empty());
    let empty_tearing: Tearing = query(r#"{"protocol":false,"outputs":[],"surfaces":[]}"#);
    assert!(empty_tearing.outputs.is_empty() && empty_tearing.surfaces.is_empty());
}

#[test]
fn all_events_route_to_the_correct_owned_payload() {
    for (family, payload) in [
        (Family::Theme, THEME),
        (Family::Overview, OVERVIEW),
        (Family::KeyboardLayout, KEYBOARD),
        (Family::Windows, WINDOWS),
        (Family::Workspaces, WORKSPACES),
        (Family::Submap, SUBMAP),
        (
            Family::LayoutState,
            r#"{"version":1,"instance":"native","active":null,"columns":[],"unsupported_workspaces":[]}"#,
        ),
    ] {
        // Deliberately data-first, just like the pinned compositor.
        let frame = format!(r#"{{"data":{payload},"event":"{}"}}"#, family.as_str());
        let message = Message::decode(frame.as_bytes()).unwrap();
        let EventDecode::Known(event) = decode_event(&message).unwrap() else {
            panic!("known event must be typed");
        };
        assert_eq!(event.family(), family);
        match event {
            Event::LayoutState(layout) => {
                assert_eq!(layout.instance, "native");
                assert!(layout.active.is_none());
                roundtrip(&layout);
            }
            Event::Theme(theme) => {
                assert_eq!(theme.background, "#010203ff");
                assert_eq!(theme.border.scratchpad_focused, "#445566ff");
                assert_eq!(theme.overview.badge, "#8899aaff");
                assert_eq!(theme.corner_radius, 12);
                roundtrip(&theme);
            }
            Event::Overview(overview) => assert!(overview.open),
            Event::KeyboardLayout(layouts) => {
                assert_eq!(layouts.current_index, 1);
                assert_eq!(layouts.names[1], "Synthetic 日本語");
                assert_eq!(layouts, query::<KeyboardLayouts>(KEYBOARD));
            }
            Event::Windows(windows) => assert_eq!(windows, query::<Windows>(WINDOWS)),
            Event::Workspaces(workspaces) => {
                assert_eq!(workspaces, query::<Workspaces>(WORKSPACES))
            }
            Event::Submap(submap) => {
                assert_eq!(submap.as_deref(), Some("synthetic-outer 雪\n\"binding\""));
                assert_eq!(submap, query::<Submap>(SUBMAP));
            }
        }
    }
    let message = Message::decode(br#"{"event":"submap","data":null}"#).unwrap();
    assert!(matches!(
        decode_event(&message).unwrap(),
        EventDecode::Known(Event::Submap(None))
    ));
    assert_eq!(query::<Submap>(DEFAULT_SUBMAP), None);
    assert_eq!(query::<Submap>(r#""""#), Some(String::new()));
    assert!(
        query::<KeyboardLayouts>(r#"{"names":[],"current_index":0}"#)
            .names
            .is_empty()
    );
    assert_eq!(query::<OutputCreate>(r#""SYNTH-NEW""#), "SYNTH-NEW");
    query::<OutputDestroy>("null");
    query::<ActionResult>("null");
}

#[test]
fn typed_routing_preserves_unknown_families_and_does_not_consume_replies() {
    let message =
        Message::decode(br#"{"event":"future_family","data":{"new":[null,42]}}"#).unwrap();
    let EventDecode::Unsupported { name, data } = decode_event(&message).unwrap() else {
        panic!("expected unsupported event with raw payload");
    };
    assert_eq!(name, "future_family");
    assert_eq!(data.as_raw_str(), r#"{"new":[null,42]}"#);
    assert!(matches!(message, Message::Event { .. }));
    for frame in [
        b"".as_slice(),
        br#"{"ok":null}"#,
        br#"{"err":"synthetic error"}"#,
    ] {
        assert!(matches!(
            decode_event(&Message::decode(frame).unwrap()).unwrap(),
            EventDecode::NotEvent
        ));
    }
    let invalid = Message::decode(br#"{"event":"overview","data":{"open":null}}"#).unwrap();
    assert!(decode_event(&invalid).is_err());
}

#[test]
fn required_fields_and_nullable_keys_are_not_silently_defaulted() {
    for invalid in [
        WINDOWS.replacen("\"active\": false, ", "", 1),
        WINDOWS.replacen("\"pid\": 42001", "\"pid\": null", 1),
        WINDOWS.replacen("\"x\": -900", "\"x\": 1.5", 1),
        WINDOWS.replacen("\"focused\": true", "\"focused\": 1", 1),
        WINDOWS.replacen("\"workspace\": \"\"", "\"workspace\": null", 1),
        WINDOWS.replacen("\"content_type\": \"none\"", "\"content_type\": 0", 1),
    ] {
        assert!(codec::decode::<Windows>(invalid.as_bytes()).is_err());
    }
    for invalid in [
        COLOR.replacen("\"mastering_luminance\": null, ", "", 1),
        COLOR.replacen("\"mastering_display_primaries\": null, ", "", 1),
        COLOR.replacen(
            "\"mastering_luminance\": null",
            "\"mastering_luminance\": false",
            1,
        ),
        COLOR.replacen(
            "\"white\": {\"x\": 0.3127, \"y\": 0.329}",
            "\"white\": {\"x\": 0.3127}",
            1,
        ),
    ] {
        assert!(codec::decode::<Color>(invalid.as_bytes()).is_err());
    }
    for key in [
        "last_presentation_flags",
        "last_presentation_presented",
        "last_presentation_vsync",
        "rule_override",
    ] {
        let invalid = TEARING.replacen(&format!("\"{key}\": null, "), "", 1);
        assert_ne!(invalid, TEARING, "fixture must contain nullable key {key}");
        assert!(
            codec::decode::<Tearing>(invalid.as_bytes()).is_err(),
            "missing {key}"
        );
    }
    assert!(
        codec::decode::<Theme>(
            THEME
                .replacen("\"shadow\": \"#00000040\", ", "", 1)
                .as_bytes()
        )
        .is_err()
    );
    assert!(codec::decode::<Workspace>(br#"{}"#).is_err());
    assert!(codec::decode::<LayerSurface>(br#"{}"#).is_err());
    assert!(codec::decode::<KeyboardLayouts>(br#"{"names":[]}"#).is_err());
    assert!(codec::decode::<KeyboardLayouts>(br#"{"names":[],"current_index":-1}"#).is_err());
    assert!(codec::decode::<Overview>(br#"{}"#).is_err());
}

#[test]
fn extensible_vocabularies_roundtrip_known_and_unknown_wire_spellings() {
    macro_rules! check {
        ($ty:ty, [$($spelling:literal),+]) => {
            for spelling in [$($spelling,)+ "future 雪"] {
                let value = <$ty>::from(spelling);
                assert_eq!(value.as_str(), spelling);
                assert_eq!(
                    <$ty>::deserialize(BorrowedStrDeserializer::<Error>::new(spelling)).unwrap(),
                    value,
                );
                assert_eq!(
                    <$ty>::deserialize(StrDeserializer::<Error>::new(spelling)).unwrap(),
                    value,
                );
                assert_eq!(
                    <$ty>::deserialize(StringDeserializer::<Error>::new(spelling.to_owned())).unwrap(),
                    value,
                );
                roundtrip(&value);
            }
        };
    }
    check!(ContentType, ["none", "photo", "video", "game"]);
    check!(LayoutMode, ["scrolling", "dwindle", "master", "unknown"]);
    check!(Layer, ["background", "bottom", "top", "overlay", "unknown"]);
    check!(HdrMode, ["off", "on", "auto", "fullscreen"]);
    check!(PresentationHint, ["async", "vsync"]);
    check!(
        TransferFunction,
        [
            "none",
            "sRGB",
            "PQ",
            "extended linear",
            "gamma 2.2",
            "BT.1886",
            "gamma 2.8",
            "ST 240",
            "log 100",
            "log 316",
            "xvYCC",
            "extended sRGB",
            "ST 428",
            "HLG",
            "compound power 2.4",
            "unknown"
        ]
    );
    check!(
        Primaries,
        [
            "none",
            "sRGB",
            "BT.2020",
            "PAL-M",
            "PAL",
            "NTSC",
            "generic film",
            "CIE 1931 XYZ",
            "DCI-P3",
            "Display P3",
            "Adobe RGB",
            "unknown"
        ]
    );
}

#[test]
fn enum_visitors_classify_escaped_known_spellings_and_preserve_unknown_text() {
    assert_eq!(
        codec::decode::<ContentType>(br#""n\u006fne""#).unwrap(),
        ContentType::None
    );
    assert_eq!(
        codec::decode::<LayoutMode>(br#""scr\u006flling""#).unwrap(),
        LayoutMode::Scrolling
    );
    assert_eq!(
        codec::decode::<TransferFunction>(br#""extended\u0020linear""#).unwrap(),
        TransferFunction::ExtendedLinear
    );

    let unknown =
        codec::decode::<ContentType>(br#""future\u0020\u96ea\ud83e\udda6\n\"\\""#).unwrap();
    assert_eq!(unknown.as_str(), "future 雪🦦\n\"\\");
    assert!(matches!(unknown, ContentType::Unknown(_)));
    roundtrip(&unknown);
    assert_eq!(ContentType::from(""), ContentType::Unknown(String::new()));

    // Both owned entry paths transfer the original String buffer into Unknown.
    let incoming = String::from("future owned 雪");
    let pointer = incoming.as_ptr();
    let owned = ContentType::deserialize(StringDeserializer::<Error>::new(incoming)).unwrap();
    assert_eq!(owned.as_str().as_ptr(), pointer);
    assert_eq!(owned.as_str(), "future owned 雪");

    let incoming = String::from("future constructed 🦦");
    let pointer = incoming.as_ptr();
    let owned = ContentType::from(incoming);
    assert_eq!(owned.as_str().as_ptr(), pointer);
    roundtrip(&owned);

    for invalid in ["null", "true", "42", "1.5", "[]", "{}"] {
        assert!(codec::decode::<ContentType>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<LayoutMode>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<Layer>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<HdrMode>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<PresentationHint>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<TransferFunction>(invalid.as_bytes()).is_err());
        assert!(codec::decode::<Primaries>(invalid.as_bytes()).is_err());
    }
}
