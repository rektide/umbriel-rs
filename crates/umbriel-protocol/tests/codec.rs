use umbriel_protocol::codec::{self, FrameError, Framer, Limits};

#[test]
fn every_split_and_multiple_frames() {
    let input = "{\"ok\":\"雪🦀\\n\\\"\"}\n\n{\"ok\":null}\n".as_bytes();
    let expected: Vec<_> = input.split(|b| *b == b'\n').take(3).collect();
    for split in 0..=input.len() {
        let mut framer = Framer::new(Limits {
            frame: 128,
            buffered: 256,
        });
        let mut got = Vec::new();
        for chunk in [&input[..split], &input[split..]] {
            framer.push(chunk).unwrap();
            while let Some(frame) = framer.next_frame() {
                got.push(frame.to_vec());
            }
        }
        assert_eq!(got, expected);
        framer.finish().unwrap();
    }
}

#[test]
fn exact_limits_atomic_rejection_eof_and_reuse() {
    let mut f = Framer::new(Limits {
        frame: 3,
        buffered: 5,
    });
    f.push(b"abc").unwrap();
    assert_eq!(f.finish(), Err(FrameError::Truncated { bytes: 3 }));
    assert!(matches!(f.push(b"d"), Err(FrameError::FrameTooLong { .. })));
    f.push(b"\n\n").unwrap();
    assert!(matches!(f.push(b"x"), Err(FrameError::BufferFull { .. })));
    f.finish().unwrap();
    assert_eq!(f.next_frame(), Some(&b"abc"[..]));
    let capacity = f.capacity();
    f.push(b"ab\n").unwrap(); // compaction preserves queued blank
    assert_eq!(f.next_frame(), Some(&b""[..]));
    assert_eq!(f.next_frame(), Some(&b"ab"[..]));
    assert!(f.capacity() >= capacity);
    assert_eq!(f.buffered_len(), 0);
}

#[test]
fn fragmented_scanning_and_zero_frame_bound() {
    let mut f = Framer::new(Limits {
        frame: 4096,
        buffered: 4097,
    });
    for _ in 0..4096 {
        f.push(b"x").unwrap();
        assert!(f.next_frame().is_none());
    }
    f.push(b"\n").unwrap();
    assert_eq!(f.next_frame().unwrap().len(), 4096);
    let mut empty = Framer::new(Limits {
        frame: 0,
        buffered: 1,
    });
    empty.push(b"\n").unwrap();
    assert_eq!(empty.next_frame(), Some(&b""[..]));
    assert!(empty.push(b"x").is_err());
}

#[test]
fn checked_decode_and_lf_inclusive_encoding() {
    assert_eq!(codec::decode::<u32>(b"12 ").unwrap(), 12);
    for bad in [&b"12 false"[..], &b"\"\xff\""[..], &b"[1,"[..]] {
        assert!(codec::decode::<sonic_rs::LazyValue<'_>>(bad).is_err());
    }
    let mut bytes = Vec::new();
    codec::encode(&"x".repeat(65533), &mut bytes).unwrap();
    assert_eq!(bytes.len(), 65536);
    let capacity = bytes.capacity();
    assert!(matches!(
        codec::encode(&"x".repeat(65534), &mut bytes),
        Err(codec::EncodeError::TooLong)
    ));
    assert!(bytes.is_empty());
    codec::encode(&"雪\n\"\\", &mut bytes).unwrap();
    assert_eq!(codec::decode::<String>(&bytes).unwrap(), "雪\n\"\\");
    assert_eq!(bytes.capacity(), capacity);
}

#[test]
fn typed_decode_checks_unicode_in_ignored_fields_before_returning_success() {
    #[derive(serde::Deserialize)]
    struct Projection {
        selected: u32,
    }
    for bad in [
        r#"{"selected":7,"ignored":"\uD800"}"#,
        r#"{"selected":7,"ignored":{"\uDC00":1}}"#,
        r#"{"selected":7,"ignored":["\uD800\u0041"]}"#,
    ] {
        assert!(codec::decode::<Projection>(bad.as_bytes()).is_err());
    }
    let frame = br#"{"selected":7,"ignored":["\uD83E\uDD80",1e400]}"#;
    assert_eq!(codec::decode::<Projection>(frame).unwrap().selected, 7);
}
