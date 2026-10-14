use umbriel_protocol::{
    action::{Action, no_argument::NoArgument},
    request::Request,
};

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
