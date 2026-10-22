use umbriel_protocol::{
    codec,
    envelope::{Family, Message},
    model::*,
    request::Request,
};

fn basis() -> LayoutBasis {
    codec::decode(br#"{
      "instance":"native-run","workspace":"HEADLESS-1:1","column":0,"axis":"vertical",
      "members":[{"id":"window","title_bar_height":0,"constraints":{"min_width":1,"min_height":1,"max_width":0,"max_height":0},"fullscreen":false,"maximized":false,"maximized_to_edges":false,"floating":false}],
      "weights":{"rows":[1.0],"top":0.0,"bottom":0.0},
      "area":{"x":0,"y":0,"width":1280,"height":720},"edge_pad":10,"gap":12,
      "width_fraction":0.5,"primary_extent":624,"tabbed":false,"supported":true
    }"#).unwrap()
}

#[test]
fn allocation_encodes_complete_basis_and_native_restore_weights() {
    let expected = basis();
    let allocation = LayoutAllocation {
        target: "window".into(),
        expected: expected.clone(),
        weights: LayoutWeights {
            rows: vec![420.0],
            top: 0.0,
            bottom: 280.0,
        },
        expected_active: Some("window".into()),
        target_content_extents: Some(vec![420]),
    };
    let mut bytes = Vec::new();
    allocation.request().encode(&mut bytes).unwrap();
    let wire: sonic_rs::Value = sonic_rs::from_slice(&bytes).unwrap();
    assert_eq!(wire["cmd"], "layout-allocate");
    assert_eq!(wire["expected"]["weights"]["rows"][0], 1.0);
    assert_eq!(wire["weights"]["rows"][0], 420.0);
    assert_eq!(wire["target_content_extents"][0], 420);
    let mut restore = allocation;
    restore.expected.weights = restore.weights.clone();
    restore.weights = expected.weights;
    restore.expected_active = None;
    restore.target_content_extents = None;
    bytes.clear();
    restore.request().encode(&mut bytes).unwrap();
    let wire: sonic_rs::Value = sonic_rs::from_slice(&bytes).unwrap();
    assert_eq!(wire["expected"]["weights"]["rows"][0], 420.0);
    assert_eq!(wire["weights"]["rows"][0], 1.0);
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains("expected_active")
    );
}

#[test]
fn invalid_allocation_never_encodes_as_null_or_partial_valid_request() {
    for row in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.049] {
        let allocation = LayoutAllocation {
            target: "window".into(),
            expected: basis(),
            weights: LayoutWeights {
                rows: vec![row],
                top: 0.0,
                bottom: 0.0,
            },
            expected_active: None,
            target_content_extents: None,
        };
        assert!(allocation.request().encode(&mut Vec::new()).is_err());
    }
}

#[test]
fn layout_event_is_typed_and_query_is_opt_in() {
    let message =
        Message::decode(br#"{"event":"layout-state","data":{"version":1,"instance":"run","active":null,"columns":[],"unsupported_workspaces":["ws"]}}"#)
            .unwrap();
    let EventDecode::Known(Event::LayoutState(state)) = decode_event(&message).unwrap() else {
        panic!("wrong event")
    };
    assert_eq!(state.unsupported_workspaces, ["ws"]);
    let mut bytes = Vec::new();
    layout_state_request().encode(&mut bytes).unwrap();
    assert_eq!(bytes, b"{\"cmd\":\"layout-state\"}\n");
    bytes.clear();
    Request::Subscribe(&[Family::LayoutState])
        .encode(&mut bytes)
        .unwrap();
    assert!(String::from_utf8(bytes).unwrap().contains("layout-state"));
}
