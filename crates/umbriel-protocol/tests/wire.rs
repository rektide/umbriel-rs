use std::borrow::Cow;

use serde::Deserialize;
use umbriel_protocol::{
    codec::{self, Framer, Limits},
    envelope::{EventName, Family, Message},
    request::{Query, Request},
};

#[test]
fn arbitrary_success_payloads_and_null_presence() {
    for raw in [
        "null",
        "true",
        "123",
        "-1.5e2",
        "1e400",
        "123456789012345678901234567890",
        "\"雪\\n\"",
        "[]",
        "{}",
        "[1, null, {\"x\":false}]",
    ] {
        let frame = format!("{{\"ok\":{raw}}}");
        let Message::Ok(value) = Message::decode(frame.as_bytes()).unwrap() else {
            panic!()
        };
        assert_eq!(value.as_raw_str(), raw);
        if !raw.contains('\\') {
            assert!(matches!(value.as_raw_cow(), Cow::Borrowed(_)));
        }
    }
    assert!(Message::decode(b"{}").is_err());
    assert!(matches!(Message::decode(b"").unwrap(), Message::Blank));
    assert!(Message::decode(b" ").is_err());
}

#[test]
fn family_names_key_order_and_additive_fields() {
    for family in Family::ALL {
        for frame in [
            format!("{{\"event\":\"{}\",\"data\":null}}", family.as_str()),
            format!(
                "{{\"extra\":{{\"nested\":[true,2]}},\"data\":null,\"event\":\"{}\"}}",
                family.as_str()
            ),
        ] {
            let Message::Event { name, data } = Message::decode(frame.as_bytes()).unwrap() else {
                panic!()
            };
            assert_eq!(name, EventName::Known(family));
            assert_eq!(data.as_raw_str(), "null");
        }
    }
    let Message::Event { name, .. } = Message::decode(br#"{"data":[],"event":"future"}"#).unwrap()
    else {
        panic!()
    };
    assert_eq!(name, EventName::Unknown(Cow::Borrowed("future")));
    let Message::Event { name, .. } =
        Message::decode(br#"{"data":[],"event":"fu\nture"}"#).unwrap()
    else {
        panic!()
    };
    assert_eq!(name, EventName::Unknown(Cow::Owned("fu\nture".into())));
    assert!(matches!(
        Message::decode(br#"{"d\u0061ta":null,"ev\u0065nt":"wind\u006fws"}"#).unwrap(),
        Message::Event {
            name: EventName::Known(Family::Windows),
            ..
        }
    ));
}

#[test]
fn strings_borrow_or_own_and_payload_projections_borrow() {
    let frame = br#"{"err":"plain"}"#;
    let Message::Err(Cow::Borrowed(text)) = Message::decode(frame).unwrap() else {
        panic!()
    };
    assert_eq!(text.as_ptr(), frame[8..].as_ptr());
    let Message::Err(Cow::Owned(text)) = Message::decode(br#"{"err":"line\n\"quoted\""}"#).unwrap()
    else {
        panic!()
    };
    assert_eq!(text, "line\n\"quoted\"");
    #[derive(Deserialize)]
    struct Projection<'a> {
        title: &'a str,
    }
    let Message::Ok(raw) = Message::decode(br#"{"ok":{"title":"borrowed","other":123}}"#).unwrap()
    else {
        panic!()
    };
    let p: Projection<'_> = codec::decode(raw.as_raw_str().as_bytes()).unwrap();
    assert_eq!(p.title, "borrowed");
}

#[test]
fn reject_duplicates_conflicts_missing_keys_and_bad_types() {
    for bad in [
        r#"{"ok":null,"ok":2}"#,
        r#"{"err":"a","err":"b"}"#,
        r#"{"event":"windows","event":"theme","data":[]}"#,
        r#"{"data":null,"data":[],"event":"windows"}"#,
        r#"{"ok":null,"\u006fk":true}"#,
        r#"{"ok":null,"err":"no"}"#,
        r#"{"err":"no","ok":null}"#,
        r#"{"ok":null,"data":[]}"#,
        r#"{"event":"windows","ok":1}"#,
        r#"{"err":"no","event":"windows","data":[]}"#,
        r#"{"event":"windows"}"#,
        r#"{"data":null}"#,
        r#"{"err":null}"#,
        r#"{"err":42}"#,
        r#"{"event":3,"data":[]}"#,
        "null",
        "[]",
        "true",
    ] {
        assert!(Message::decode(bad.as_bytes()).is_err(), "accepted {bad}");
    }
}

#[test]
fn full_validation_including_lazy_and_ignored_values() {
    for bad in [
        r#"{"ok":null} {}"#,
        r#"{"ok":null}garbage"#,
        r#"{"ok":[1,]}"#,
        r#"{"ok":{"x":true false}}"#,
        r#"{"ok":"\q"}"#,
        r#"{"ok":01}"#,
        r#"{"ok":1e}"#,
        r#"{"ok":null,"extra":[true,]}"#,
        r#"{"ok":null,"extra":"\q"}"#,
        r#"{"data":[1,],"event":"windows"}"#,
        r#"{"ok":"\uD800"}"#,
        r#"{"ok":null,"extra":"\uDC00"}"#,
        r#"{"ok":null,"extra":{"\uD800":0}}"#,
        r#"{"ok":null,"extra":"\uD800\u0041"}"#,
        r#"{"ok":null,"extra":"\uDD80\uD83E"}"#,
        r#"{"ok":"\uZZZZ"}"#,
        r#"{"ok":null,"extra":"\u12G4"}"#,
    ] {
        assert!(Message::decode(bad.as_bytes()).is_err(), "accepted {bad}");
    }
    for bad in [
        &b"{\"ok\":\"\xff\"}"[..],
        &b"{\"ok\":null,\"x\":\"\xff\"}"[..],
        &b"{\"ok\":null}\xff"[..],
    ] {
        assert!(Message::decode(bad).is_err());
    }
    assert!(Message::decode(b"{\"ok\":null}\t\r ").is_ok());
    assert!(
        Message::decode(br#"{"ok":null,"extra":{"emoji":"\uD83E\uDD80","number":1e400}}"#).is_ok()
    );
    assert!(Message::decode(br#"{"ok":"literal \\uD800 and \\\"quote\\\""}"#).is_ok());
}

#[test]
fn every_split_decodes_escaped_utf8_envelopes() {
    let input =
        "{\"err\":\"雪🦀\\n\\\"\\\\\"}\n{\"event\":\"未知\\n\",\"data\":{\"title\":\"雪\\t\"}}\n"
            .as_bytes();
    for split in 0..=input.len() {
        let mut framer = Framer::new(Limits {
            frame: 256,
            buffered: 512,
        });
        let mut count = 0;
        for chunk in [&input[..split], &input[split..]] {
            framer.push(chunk).unwrap();
            while let Some(frame) = framer.next_frame() {
                let decoded = Message::decode(frame).unwrap();
                assert!(matches!(decoded, Message::Err(_) | Message::Event { .. }));
                count += 1;
            }
        }
        framer.finish().unwrap();
        assert_eq!(count, 2);
    }
}

#[test]
fn typed_requests_escape_all_dynamic_strings() {
    #[derive(Deserialize)]
    struct Wire {
        cmd: String,
        arg: Option<String>,
        events: Option<Vec<String>>,
    }
    let mut bytes = Vec::new();
    for query in Query::ALL {
        Request::Query(query).encode(&mut bytes).unwrap();
        let wire: Wire = codec::decode(&bytes).unwrap();
        assert_eq!(wire.cmd, query.as_str());
        assert!(wire.arg.is_none());
    }
    let text = "雪\n\"\\";
    for (request, cmd) in [
        (Request::Msg(text), "msg"),
        (Request::OutputCreate(text), "output-create"),
        (Request::OutputDestroy(text), "output-destroy"),
        (
            Request::Raw {
                cmd: text,
                arg: Some(text),
            },
            text,
        ),
    ] {
        request.encode(&mut bytes).unwrap();
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 1);
        let wire: Wire = codec::decode(&bytes).unwrap();
        assert_eq!(wire.cmd, cmd);
        assert_eq!(wire.arg.as_deref(), Some(text));
    }
    Request::Subscribe(&Family::ALL).encode(&mut bytes).unwrap();
    let wire: Wire = codec::decode(&bytes).unwrap();
    assert_eq!(wire.cmd, "subscribe");
    assert_eq!(wire.events.unwrap(), Family::ALL.map(|f| f.as_str()));
    assert!(Request::Subscribe(&[]).encode(&mut bytes).is_err());
    assert!(bytes.is_empty());
}

#[test]
fn typed_request_limit_includes_lf_and_escaping_expansion() {
    let mut bytes = Vec::new();
    Request::Msg("").encode(&mut bytes).unwrap();
    let overhead = bytes.len();
    Request::Msg(&"x".repeat(65536 - overhead))
        .encode(&mut bytes)
        .unwrap();
    assert_eq!(bytes.len(), 65536);
    assert!(
        Request::Msg(&"x".repeat(65537 - overhead))
            .encode(&mut bytes)
            .is_err()
    );
    assert!(
        Request::Msg(&"\n".repeat(32768))
            .encode(&mut bytes)
            .is_err()
    );
    assert!(bytes.is_empty());
}
