//! Reusable contiguous framing storage and checked Sonic serialization.
use std::io::{self, Write};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};
use thiserror::Error;

use crate::REQUEST_LIMIT;

/// Incoming client policy. Frame bytes exclude LF; buffered bytes include LF.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub frame: usize,
    pub buffered: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("frame exceeds {limit} bytes")]
    FrameTooLong { limit: usize },
    #[error("buffered input exceeds {limit} bytes; drain frames before feeding more")]
    BufferFull { limit: usize },
    #[error("EOF with {bytes} unterminated bytes")]
    Truncated { bytes: usize },
}

/// LF-only framer. Feed chunks, then drain `next_frame` until it returns `None`.
/// Multiple frames in one chunk are retained, including empty frames. Returned
/// slices exclude LF (but not CR) and borrow storage until the next mutation.
/// `finish` rejects an incomplete EOF tail; it never promotes that tail to a frame.
/// Capacity is retained after draining; consumed prefixes are compacted only
/// when large or needed for appending. Limits apply to unconsumed bytes, not capacity.
#[derive(Debug)]
pub struct Framer {
    bytes: Vec<u8>,
    head: usize,
    scan: usize,
    tail: usize,
    limits: Limits,
}

impl Framer {
    pub fn new(limits: Limits) -> Self {
        Self {
            bytes: Vec::new(),
            head: 0,
            scan: 0,
            tail: 0,
            limits,
        }
    }

    pub fn buffered_len(&self) -> usize {
        self.bytes.len() - self.head
    }
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Atomic on limit errors: rejected input does not change stored bytes.
    pub fn push(&mut self, input: &[u8]) -> Result<(), FrameError> {
        if input.len() > self.limits.buffered.saturating_sub(self.buffered_len()) {
            return Err(FrameError::BufferFull {
                limit: self.limits.buffered,
            });
        }
        let mut tail = self.tail;
        let mut start = 0;
        for end in memchr::memchr_iter(b'\n', input) {
            if end - start > self.limits.frame.saturating_sub(tail) {
                return Err(FrameError::FrameTooLong {
                    limit: self.limits.frame,
                });
            }
            tail = 0;
            start = end + 1;
        }
        if input.len() - start > self.limits.frame.saturating_sub(tail) {
            return Err(FrameError::FrameTooLong {
                limit: self.limits.frame,
            });
        }
        tail += input.len() - start;
        if self.head > 0
            && (self.head >= self.bytes.len() / 2
                || input.len() > self.bytes.capacity() - self.bytes.len())
        {
            self.bytes.copy_within(self.head.., 0);
            self.bytes.truncate(self.bytes.len() - self.head);
            self.scan -= self.head;
            self.head = 0;
        }
        self.bytes.extend_from_slice(input);
        self.tail = tail;
        Ok(())
    }

    pub fn next_frame(&mut self) -> Option<&[u8]> {
        match memchr::memchr(b'\n', &self.bytes[self.scan..]) {
            Some(offset) => {
                let end = self.scan + offset;
                let start = self.head;
                self.head = end + 1;
                self.scan = self.head;
                Some(&self.bytes[start..end])
            }
            None => {
                self.scan = self.bytes.len();
                None
            }
        }
    }

    /// May be called before draining already complete frames.
    pub fn finish(&self) -> Result<(), FrameError> {
        if self.tail == 0 {
            Ok(())
        } else {
            Err(FrameError::Truncated { bytes: self.tail })
        }
    }
}

/// Checked decoding, including UTF-8 and all trailing input. The selected type
/// can borrow from the frame; callers need not build a generic JSON tree.
/// Caller `Deserialize` code runs before supplementary Unicode-escape checks:
/// stage application state until this function returns success.
pub fn decode<'a, T: Deserialize<'a>>(frame: &'a [u8]) -> sonic_rs::Result<T> {
    // Validate first: Sonic's lazy-value path can inspect strings before its
    // final UTF-8 check (and panic in debug builds on invalid string bytes).
    let text = std::str::from_utf8(frame).map_err(serde::de::Error::custom)?;
    // Typed parsing validates structure, ordinary escapes and trailing input,
    // including LazyValue and IgnoredAny paths (both use skip_one(true)). Their
    // remaining gap is Unicode escape digits and surrogate pairing. Check only those
    // string tokens after parsing; do not run a second full syntax deserializer.
    let value = sonic_rs::from_str(text)?;
    for_unicode_strings(text, |token| {
        sonic_rs::from_str::<CheckedString>(token).map(|_| ())
    })?;
    Ok(value)
}

// Sonic checks structure and ordinary escapes, but skipped Unicode digits can
// hide delimiters. Validate those digits before locating the closing quote.
// Only actual Unicode escapes need supplementary string decoding.
// This is token selection, not a second JSON grammar parser, and never converts
// lazy numbers into f64/u64/i64 merely for validation.
fn for_unicode_strings(
    text: &str,
    mut check: impl FnMut(&str) -> sonic_rs::Result<()>,
) -> sonic_rs::Result<()> {
    let bytes = text.as_bytes();
    // Ordinary frames (even those with only \n/\"/\\ escapes) stop here.
    // A literal \\u can produce a false positive, filtered by the token scan.
    if memchr::memmem::find(bytes, b"\\u").is_none() {
        return Ok(());
    }
    let mut cursor = 0;
    while let Some(offset) = memchr::memchr(b'"', &bytes[cursor..]) {
        let start = cursor + offset;
        cursor = start + 1;
        let mut unicode = false;
        loop {
            let Some(offset) = memchr::memchr2(b'"', b'\\', &bytes[cursor..]) else {
                return Err(de::Error::custom("unterminated supplemental string token"));
            };
            cursor += offset;
            if bytes[cursor] == b'"' {
                cursor += 1;
                if unicode {
                    check(&text[start..cursor])?;
                }
                break;
            }
            let escaped = bytes.get(cursor + 1).ok_or_else(|| {
                <sonic_rs::Error as de::Error>::custom("incomplete supplemental string escape")
            })?;
            if *escaped == b'u' {
                let digits = bytes.get(cursor + 2..cursor + 6).ok_or_else(|| {
                    <sonic_rs::Error as de::Error>::custom("incomplete Unicode escape")
                })?;
                if !digits.iter().all(u8::is_ascii_hexdigit) {
                    return Err(de::Error::custom("invalid Unicode escape digits"));
                }
                unicode = true;
                cursor += 6;
            } else {
                cursor += 2;
            }
        }
    }
    Ok(())
}

struct CheckedString;
impl<'de> Deserialize<'de> for CheckedString {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct CheckString;
        impl Visitor<'_> for CheckString {
            type Value = CheckedString;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON string")
            }
            fn visit_str<E: de::Error>(self, _: &str) -> Result<CheckedString, E> {
                Ok(CheckedString)
            }
        }
        d.deserialize_string(CheckString)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_selector_rejects_incomplete_tokens_without_indexing_past_end() {
        for text in [
            r#""\u0041"#,
            r#""\u0041\"#,
            r#""\u0041\""#,
            r#""\u000\""#,
            r#""\u"#,
            r#""\u00"#,
        ] {
            assert!(
                for_unicode_strings(text, |_| Ok(())).is_err(),
                "accepted {text}"
            );
        }
    }

    // Reproducible work-count check, not a timing or allocation benchmark:
    // count supplementary string deserializations on window-like inventories.
    #[test]
    fn supplementary_decode_only_visits_actual_unicode_escape_tokens() {
        let row = r#"{"id":"opaque","title":"雪🦀","app_id":"terminal","workspace":"out:1"}"#;
        let rows = [row; 32].join(",");
        let plain = format!("{{\"data\":[{rows}],\"event\":\"windows\"}}");
        let ordinary_escapes = plain.replace("雪🦀", r#"line\n\"quoted\"\\path"#);
        let literal_unicode = plain.replace("雪🦀", r#"literal \\uD800"#);
        let one_unicode = ordinary_escapes.replacen("terminal", r"term\u0069nal", 1);
        let all_unicode = plain.replace("雪🦀", r"\uD83E\uDD80");

        for (text, expected) in [
            (plain, 0),
            (ordinary_escapes, 0),
            (literal_unicode, 0),
            (one_unicode, 1),
            (all_unicode, 32),
        ] {
            // Establish the token selector's precondition as in decode().
            sonic_rs::from_str::<de::IgnoredAny>(&text).unwrap();
            let mut calls = 0;
            for_unicode_strings(&text, |token| {
                calls += 1;
                sonic_rs::from_str::<CheckedString>(token).map(|_| ())
            })
            .unwrap();
            assert_eq!(calls, expected);
            decode::<sonic_rs::LazyValue<'_>>(text.as_bytes()).unwrap();
        }
    }
}

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("request exceeds {REQUEST_LIMIT} bytes including LF")]
    TooLong,
    #[error("request serialization failed: {0}")]
    Json(#[from] sonic_rs::Error),
}

struct BoundedWriter<'a> {
    bytes: &'a mut Vec<u8>,
    exceeded: bool,
}
impl Write for BoundedWriter<'_> {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.len() > (REQUEST_LIMIT - 1).saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("request limit exceeded"));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Replace caller-owned storage with serialized JSON plus LF. On failure the
/// buffer is empty (capacity retained); no partial request can be sent by mistake.
pub fn encode<T: Serialize + ?Sized>(request: &T, bytes: &mut Vec<u8>) -> Result<(), EncodeError> {
    bytes.clear();
    let mut writer = BoundedWriter {
        bytes,
        exceeded: false,
    };
    let result = sonic_rs::to_writer(sonic_rs::writer::BufferedWriter::new(&mut writer), request);
    let exceeded = writer.exceeded;
    if let Err(error) = result {
        bytes.clear();
        return Err(if exceeded {
            EncodeError::TooLong
        } else {
            EncodeError::Json(error)
        });
    }
    bytes.push(b'\n');
    Ok(())
}
