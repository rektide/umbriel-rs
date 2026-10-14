//! Shared fixture loading for protocol benches. Loading happens at startup,
//! never inside measured iterations.

use std::path::PathBuf;

pub fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches/fixtures")
        .join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("missing bench fixture {}: {error}", path.display()))
}

/// Wire lines carry a trailing LF; `Framer::next_frame` excludes it, so decode
/// benches receive the same LF-stripped slice the receive loop would hand on.
/// (The framing target drives LF itself and does not call this.)
#[allow(dead_code)]
pub fn frame_without_lf(line_with_lf: &[u8]) -> &[u8] {
    line_with_lf
        .strip_suffix(b"\n")
        .unwrap_or_else(|| panic!("fixture line missing LF"))
}

/// Concatenate `copies` of a wire line into one multi-frame stream.
/// (Targets use different subsets of this module's helpers.)
#[allow(dead_code)]
pub fn repeat_frames(line_with_lf: &[u8], copies: usize) -> Vec<u8> {
    let mut stream = Vec::with_capacity(line_with_lf.len() * copies);
    for _ in 0..copies {
        stream.extend_from_slice(line_with_lf);
    }
    stream
}

/// Split a stream into fixed-size socket-read-sized chunks (remainder last).
#[allow(dead_code)]
pub fn chunks(stream: &[u8], size: usize) -> Vec<&[u8]> {
    stream.chunks(size).collect()
}

/// Generous limits: framing policy is not under measurement here, only the
/// push/scan/drain/compact machinery. 1 MiB frames, 8 MiB buffered headroom.
#[allow(dead_code)]
pub fn bench_limits() -> umbriel_protocol::codec::Limits {
    umbriel_protocol::codec::Limits {
        frame: 1 << 20,
        buffered: 8 << 20,
    }
}
