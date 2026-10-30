// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Self-owned byte and syntax fixtures, shared by native execution and its
//! independent record verifier. Expectations describe the source parser rules.
pub const DEFAULT: &[u8] = br#"{"type":"custom"}"#;
pub struct Case {
    pub name: &'static str,
    pub input: &'static [u8],
    pub expected: &'static [u8],
    pub object: bool,
}
pub fn cases() -> Vec<Case> {
    let rejected = [
        ("empty", b"".as_slice()),
        ("whitespace", b" \r\n\t"),
        ("null", b"null"),
        ("boolean", b"true"),
        ("number", b"17"),
        ("array", b"[1,2]"),
        ("string", br#""text""#),
        ("raw_nul", b"{\"value\":\"a\0b\"}"),
        ("trailing_bytes", br#"{"ok":1}garbage"#),
        ("duplicate_field", br#"{"duplicate":1,"duplicate":2}"#),
        ("surrogate_high", br#"{"v":"\ud800"}"#),
        ("surrogate_low", br#"{"v":"\udc00"}"#),
        ("raw_control", b"{\"v\":\"a\x01b\"}"),
        ("unknown_escape", br#"{"v":"\x41"}"#),
        ("leading_zero", br#"{"v":01}"#),
        ("trailing_comma", br#"{"v":1,}"#),
        ("utf8_bom", b"\xef\xbb\xbf{\"v\":1}"),
        ("raw_newline", b"{\"v\":\"a\nb\"}"),
        ("duplicate_type", br#"{"type":"a","type":"b","extra":2}"#),
    ];
    let mut result = rejected
        .into_iter()
        .map(|(name, input)| Case {
            name,
            input,
            expected: DEFAULT,
            object: false,
        })
        .collect::<Vec<_>>();
    for (name, input, expected) in [
        ("object", b"{}".as_slice(), DEFAULT),
        ("field", br#"{"tag":7}"#, br#"{"tag":7,"type":"custom"}"#),
        (
            "overwrite_type",
            br#"{"type":"foreign","nested":{"a":[1,true,null]}}"#,
            br#"{"nested":{"a":[1,true,null]},"type":"custom"}"#,
        ),
        (
            "escaped_nul",
            br#"{"value":"a\u0000b"}"#,
            br#"{"type":"custom","value":"a\u0000b"}"#,
        ),
        (
            "invalid_utf8_value",
            b"{\"value\":\"\xff\"}",
            b"{\"type\":\"custom\",\"value\":\"\xff\"}",
        ),
        (
            "invalid_utf8_key",
            b"{\"\xff\":1}",
            b"{\"type\":\"custom\",\"\xff\":1}",
        ),
        (
            "nul_after_object",
            b"{\"ok\":1}\0garbage",
            br#"{"ok":1,"type":"custom"}"#,
        ),
        (
            "surrogate_pair",
            br#"{"v":"\ud83d\ude00"}"#,
            b"{\"type\":\"custom\",\"v\":\"\xf0\x9f\x98\x80\"}",
        ),
        (
            "u64_max",
            br#"{"v":18446744073709551615}"#,
            br#"{"type":"custom","v":18446744073709551615}"#,
        ),
        (
            "i64_min",
            br#"{"v":-9223372036854775808}"#,
            br#"{"type":"custom","v":-9223372036854775808}"#,
        ),
        (
            "float_overflow",
            br#"{"v":1e1000}"#,
            br#"{"type":"custom","v":1e1000}"#,
        ),
        (
            "integer_precision",
            br#"{"v":9007199254740993}"#,
            br#"{"type":"custom","v":9007199254740993}"#,
        ),
        (
            "valid_utf8",
            "{\"value\":\"中文\"}".as_bytes(),
            "{\"type\":\"custom\",\"value\":\"中文\"}".as_bytes(),
        ),
        (
            "negative_zero",
            br#"{"v":-0.0}"#,
            br#"{"type":"custom","v":-0.0}"#,
        ),
        (
            "invalid_utf8_sequence",
            b"{\"v\":\"\xc3(\"}",
            b"{\"type\":\"custom\",\"v\":\"\xc3(\"}",
        ),
        (
            "trailing_comment",
            br#"{"v":1}//comment"#,
            br#"{"type":"custom","v":1}"#,
        ),
        (
            "unicode_noncharacter",
            br#"{"v":"\uffff"}"#,
            b"{\"type\":\"custom\",\"v\":\"\xef\xbf\xbf\"}",
        ),
        (
            "lone_continuation",
            b"{\"v\":\"\x80\"}",
            b"{\"type\":\"custom\",\"v\":\"\x80\"}",
        ),
    ] {
        result.push(Case {
            name,
            input,
            expected,
            object: true,
        });
    }
    result
}
