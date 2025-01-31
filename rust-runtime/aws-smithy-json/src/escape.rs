/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

use std::borrow::Cow;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    ExpectedSurrogatePair(String),
    InvalidEscapeCharacter(char),
    InvalidSurrogatePair(u16, u16),
    InvalidUnicodeEscape(String),
    InvalidUtf8,
    UnexpectedEndOfString,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            ExpectedSurrogatePair(low) => {
                write!(
                    f,
                    "expected a UTF-16 surrogate pair, but got {} as the low word",
                    low
                )
            }
            InvalidEscapeCharacter(chr) => write!(f, "invalid JSON escape: \\{}", chr),
            InvalidSurrogatePair(high, low) => {
                write!(f, "invalid surrogate pair: \\u{:04X}\\u{:04X}", high, low)
            }
            InvalidUnicodeEscape(escape) => write!(f, "invalid JSON Unicode escape: \\u{}", escape),
            InvalidUtf8 => write!(f, "invalid UTF-8 codepoint in JSON string"),
            UnexpectedEndOfString => write!(f, "unexpected end of string"),
        }
    }
}

/// Escapes a string for embedding in a JSON string value.
pub fn escape_string(value: &str) -> Cow<str> {
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            0..=0x1F | b'"' | b'\\' => {
                return Cow::Owned(escape_string_inner(&bytes[0..index], &bytes[index..]))
            }
            _ => {}
        }
    }
    Cow::Borrowed(value)
}

fn escape_string_inner(start: &[u8], rest: &[u8]) -> String {
    let mut escaped = Vec::with_capacity(start.len() + rest.len() + 1);
    escaped.extend(start);

    for byte in rest {
        match byte {
            b'"' => escaped.extend(b"\\\""),
            b'\\' => escaped.extend(b"\\\\"),
            0x08 => escaped.extend(b"\\b"),
            0x0C => escaped.extend(b"\\f"),
            b'\n' => escaped.extend(b"\\n"),
            b'\r' => escaped.extend(b"\\r"),
            b'\t' => escaped.extend(b"\\t"),
            0..=0x1F => escaped.extend(format!("\\u{:04x}", byte).bytes()),
            _ => escaped.push(*byte),
        }
    }

    // This is safe because:
    // - The original input was valid UTF-8 since it came in as a `&str`
    // - Only single-byte code points were escaped
    // - The escape sequences are valid UTF-8
    debug_assert!(std::str::from_utf8(&escaped).is_ok());
    unsafe { String::from_utf8_unchecked(escaped) }
}

/// Unescapes a JSON-escaped string.
/// If there are no escape sequences, it directly returns the reference.
pub fn unescape_string(value: &str) -> Result<Cow<str>, Error> {
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\\' {
            return unescape_string_inner(&bytes[0..index], &bytes[index..]).map(Cow::Owned);
        }
    }
    Ok(Cow::Borrowed(value))
}

fn unescape_string_inner(start: &[u8], rest: &[u8]) -> Result<String, Error> {
    let mut unescaped = Vec::with_capacity(start.len() + rest.len());
    unescaped.extend(start);

    let mut index = 0;
    while index < rest.len() {
        match rest[index] {
            b'\\' => {
                index += 1;
                if index == rest.len() {
                    return Err(Error::UnexpectedEndOfString);
                }
                match rest[index] {
                    b'u' => {
                        index -= 1;
                        index += read_unicode_escapes(&rest[index..], &mut unescaped)?;
                    }
                    byte => {
                        match byte {
                            b'\\' => unescaped.push(b'\\'),
                            b'/' => unescaped.push(b'/'),
                            b'"' => unescaped.push(b'"'),
                            b'b' => unescaped.push(0x08),
                            b'f' => unescaped.push(0x0C),
                            b'n' => unescaped.push(b'\n'),
                            b'r' => unescaped.push(b'\r'),
                            b't' => unescaped.push(b'\t'),
                            _ => return Err(Error::InvalidEscapeCharacter(byte.into())),
                        }
                        index += 1;
                    }
                }
            }
            byte => {
                unescaped.push(byte);
                index += 1
            }
        }
    }

    String::from_utf8(unescaped).map_err(|_| Error::InvalidUtf8)
}

fn is_utf16_low_surrogate(codepoint: u16) -> bool {
    codepoint & 0xFC00 == 0xDC00
}

fn is_utf16_high_surrogate(codepoint: u16) -> bool {
    codepoint & 0xFC00 == 0xD800
}

fn read_codepoint(rest: &[u8]) -> Result<u16, Error> {
    if rest.len() < 6 {
        return Err(Error::UnexpectedEndOfString);
    }
    if &rest[0..2] != b"\\u" {
        // The first codepoint is always prefixed with "\u" since unescape_string_inner does
        // that check, so this error will always be for the low word of a surrogate pair.
        return Err(Error::ExpectedSurrogatePair(
            String::from_utf8_lossy(&rest[0..6]).into(),
        ));
    }

    let codepoint_str = std::str::from_utf8(&rest[2..6]).map_err(|_| Error::InvalidUtf8)?;

    // Error on characters `u16::from_str_radix` would otherwise accept, such as `+`
    if codepoint_str
        .bytes()
        .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
    {
        return Err(Error::InvalidUnicodeEscape(codepoint_str.into()));
    }
    Ok(u16::from_str_radix(codepoint_str, 16).expect("hex string is valid 16-bit value"))
}

/// Reads JSON Unicode escape sequences (i.e., "\u1234"). Will also read
/// an additional codepoint if the first codepoint is the start of a surrogate pair.
fn read_unicode_escapes(bytes: &[u8], into: &mut Vec<u8>) -> Result<usize, Error> {
    let high = read_codepoint(bytes)?;
    let (bytes_read, chr) = if is_utf16_high_surrogate(high) {
        let low = read_codepoint(&bytes[6..])?;
        if !is_utf16_low_surrogate(low) {
            return Err(Error::InvalidSurrogatePair(high, low));
        }

        let codepoint =
            std::char::from_u32(0x10000 + (high - 0xD800) as u32 * 0x400 + (low - 0xDC00) as u32)
                .ok_or(Error::InvalidSurrogatePair(high, low))?;
        (12, codepoint)
    } else {
        let codepoint = std::char::from_u32(high as u32).ok_or_else(|| {
            Error::InvalidUnicodeEscape(String::from_utf8_lossy(&bytes[0..6]).into())
        })?;
        (6, codepoint)
    };

    match chr.len_utf8() {
        1 => into.push(chr as u8),
        _ => into.extend_from_slice(chr.encode_utf8(&mut [0; 4]).as_bytes()),
    }
    Ok(bytes_read)
}

#[cfg(test)]
mod test {
    use super::escape_string;
    use crate::escape::{unescape_string, Error};
    use std::borrow::Cow;

    #[test]
    fn escape() {
        assert_eq!("", escape_string("").as_ref());
        assert_eq!("foo", escape_string("foo").as_ref());
        assert_eq!("foo\\r\\n", escape_string("foo\r\n").as_ref());
        assert_eq!("foo\\r\\nbar", escape_string("foo\r\nbar").as_ref());
        assert_eq!(r#"foo\\bar"#, escape_string(r#"foo\bar"#).as_ref());
        assert_eq!(r#"\\foobar"#, escape_string(r#"\foobar"#).as_ref());
        assert_eq!(
            r#"\bf\fo\to\r\n"#,
            escape_string("\u{08}f\u{0C}o\to\r\n").as_ref()
        );
        assert_eq!("\\\"test\\\"", escape_string("\"test\"").as_ref());
        assert_eq!("\\u0000", escape_string("\u{0}").as_ref());
        assert_eq!("\\u001f", escape_string("\u{1f}").as_ref());
    }

    #[test]
    fn unescape_no_escapes() {
        let unescaped = unescape_string("test test").unwrap();
        assert_eq!("test test", unescaped);
        assert!(matches!(unescaped, Cow::Borrowed(_)));
    }

    #[test]
    fn unescape() {
        assert_eq!(
            "\x08f\x0Co\to\r\n",
            unescape_string(r#"\bf\fo\to\r\n"#).unwrap()
        );
        assert_eq!("\"test\"", unescape_string(r#"\"test\""#).unwrap());
        assert_eq!("\x00", unescape_string("\\u0000").unwrap());
        assert_eq!("\x1f", unescape_string("\\u001f").unwrap());
        assert_eq!("foo\r\nbar", unescape_string("foo\\r\\nbar").unwrap());
        assert_eq!("foo\r\n", unescape_string("foo\\r\\n").unwrap());
        assert_eq!("\r\nbar", unescape_string("\\r\\nbar").unwrap());
        assert_eq!("\u{10437}", unescape_string("\\uD801\\uDC37").unwrap());

        assert_eq!(Err(Error::UnexpectedEndOfString), unescape_string("\\"));
        assert_eq!(Err(Error::UnexpectedEndOfString), unescape_string("\\u"));
        assert_eq!(Err(Error::UnexpectedEndOfString), unescape_string("\\u00"));
        assert_eq!(
            Err(Error::InvalidEscapeCharacter('z')),
            unescape_string("\\z")
        );

        assert_eq!(
            Err(Error::ExpectedSurrogatePair("\\nasdf".into())),
            unescape_string("\\uD801\\nasdf")
        );
        assert_eq!(
            Err(Error::UnexpectedEndOfString),
            unescape_string("\\uD801\\u00")
        );
        assert_eq!(
            Err(Error::InvalidSurrogatePair(0xD801, 0xC501)),
            unescape_string("\\uD801\\uC501")
        );

        assert_eq!(
            Err(Error::InvalidUnicodeEscape("+04D".into())),
            unescape_string("\\u+04D")
        );
    }

    use proptest::proptest;
    proptest! {
        #[test]
        fn matches_serde_json(s in ".*") {
            let serde_escaped = serde_json::to_string(&s).unwrap();
            let serde_escaped = &serde_escaped[1..(serde_escaped.len() - 1)];
            assert_eq!(serde_escaped,escape_string(&s))
        }

        #[test]
        fn round_trip(chr in proptest::char::any()) {
            let mut original = String::new();
            original.push(chr);

            let escaped = escape_string(&original);
            let unescaped = unescape_string(&escaped).unwrap();
            assert_eq!(original, unescaped);
        }

        #[test]
        fn unicode_surrogates(chr in proptest::char::range(
            std::char::from_u32(0x10000).unwrap(),
            std::char::from_u32(0x10FFFF).unwrap(),
        )) {
            let mut codepoints = [0; 2];
            chr.encode_utf16(&mut codepoints);

            let escaped = format!("\\u{:04X}\\u{:04X}", codepoints[0], codepoints[1]);
            let unescaped = unescape_string(&escaped).unwrap();

            let expected = format!("{}", chr);
            assert_eq!(expected, unescaped);
        }
    }

    #[test]
    #[ignore] // This tests escaping of all codepoints, but can take a long time in debug builds
    fn all_codepoints() {
        for value in 0..u32::MAX {
            if let Some(chr) = char::from_u32(value) {
                let string = String::from(chr);
                let escaped = escape_string(&string);
                let serde_escaped = serde_json::to_string(&string).unwrap();
                let serde_escaped = &serde_escaped[1..(serde_escaped.len() - 1)];
                assert_eq!(&escaped, serde_escaped);
            }
        }
    }
}
