use umbriel_protocol::{envelope::Family, request::Request};
use umbriel_session::{Error, PreparedCommand, Session};

#[test]
fn preparation_is_bounded_and_cannot_bypass_subscription_rules() {
    assert!(matches!(
        PreparedCommand::new(Request::Subscribe(&[Family::Windows])),
        Err(Error::UnsupportedSubscribe)
    ));
    assert!(matches!(
        PreparedCommand::new(Request::Raw {
            cmd: "subscribe",
            arg: None
        }),
        Err(Error::UnsupportedSubscribe)
    ));
    assert!(PreparedCommand::new(Request::Msg(&"x".repeat(65536))).is_err());
    let prepared = PreparedCommand::new(Request::Msg("noop")).unwrap();
    let len = prepared.encoded_len();
    let mut session = Session::one_shot(1024);
    session.admit_prepared(prepared).unwrap();
    assert_eq!(session.outbound().len(), len);
}
