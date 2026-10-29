// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Shared *inputs*, never replacement expected serialization. The actual source
// oracle supplies UUID/info bytes. No backend/input connection is requested.
pub struct Case {
    pub name: String,
    pub input: Option<Vec<u8>>,
    pub accepted: bool,
}

pub fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut add = |name: String, input: Option<Vec<u8>>, accepted| {
        cases.push(Case {
            name,
            input,
            accepted,
        });
    };
    add("null-pointer".into(), None, false);
    for (name, bytes, accepted) in [
        ("empty", "", false),
        ("whitespace", " \t\r\n", false),
        ("empty-object", "{}", false),
        ("empty-array", "[]", false),
        ("scalar-null", "null", false),
        ("scalar-string", "\"hello\"", false),
        ("scalar-number", "0", false),
        ("missing-input", "{\"screencap_method\":0}", false),
        ("missing-capture", "{\"input_method\":0}", false),
        (
            "defaults",
            "{\"screencap_method\":0,\"input_method\":0}",
            true,
        ),
        (
            "reordered",
            " \t{\"input_method\":0,\"screencap_method\":4}\r\n",
            true,
        ),
        (
            "escaped-key",
            "{\"screencap_\\u006dethod\":1,\"input_method\":0}",
            true,
        ),
        (
            "type-shadow",
            "{\"screencap_method\":0,\"input_method\":0,\"type\":\"other\"}",
            true,
        ),
    ] {
        add(name.into(), Some(bytes.as_bytes().to_vec()), accepted);
    }
    let base = "\"screencap_method\":0,\"input_method\":0";
    for (field, kind) in [
        ("screencap_method", "u64"),
        ("input_method", "u64"),
        ("pw_node_id", "u32"),
        ("pw_socket_fd", "i32"),
        ("uinput_screen_width", "i32"),
        ("uinput_screen_height", "i32"),
    ] {
        for (index, number) in [
            "0",
            "1",
            "2",
            "3",
            "4",
            "7",
            "-0",
            "-1",
            "-2147483648",
            "2147483647",
            "2147483648",
            "-2147483649",
            "4294967295",
            "4294967296",
            "18446744073709551615",
            "18446744073709551616",
            "-18446744073709551616",
            "0.0",
            "1.0",
            "1e0",
            "1E+0",
            "1e9999",
            "01",
            "-01",
            "+1",
            ".1",
            "1.",
            "1e",
            "1e+",
            "true",
            "null",
            "\"1\"",
            "[]",
            "{}",
        ]
        .iter()
        .enumerate()
        {
            let prefix = match field {
                "screencap_method" => "\"input_method\":0".into(),
                "input_method" => "\"screencap_method\":0".into(),
                _ => base.to_string(),
            };
            let lexical_integer = *number == "0"
                || (!number.starts_with('0')
                    && !number.starts_with("-0")
                    && !number.starts_with('+')
                    && number
                        .bytes()
                        .enumerate()
                        .all(|(i, b)| b.is_ascii_digit() || (i == 0 && b == b'-')))
                || *number == "-0";
            let accepted = lexical_integer
                && match kind {
                    "u64" => !number.starts_with('-') && number.parse::<u64>().is_ok(),
                    "u32" => !number.starts_with('-') && number.parse::<u32>().is_ok(),
                    _ => number.parse::<i32>().is_ok(),
                };
            add(
                format!("{field}-number-{index:02}"),
                Some(format!("{{{prefix},\"{field}\":{number}}}").into_bytes()),
                accepted,
            );
        }
    }
    for field in ["wlr_socket_path", "uinput_path", "eis_socket_path"] {
        for (index, (value, accepted)) in [
            ("\"\"", true),
            ("\"hello/world\"", true),
            ("\"a b\\\"c\\\\d\"", true),
            ("\"\\b\\f\\n\\r\\t\\/\\u0000\\u001f\"", true),
            ("\"\\u007f\\u0080\\u07ff\\u0800\\uffff\"", true),
            ("\"\\ud800\\udc00\\udbff\\udfff\"", true),
            // Observed current meojson behavior; a BMP escape may occur between
            // a high/low surrogate. Keep this separate from standard pairs.
            ("\"\\ud800\\u0061\\udc00\"", true),
            ("\"\\ud800\"", false),
            ("\"\\udc00\"", false),
            ("\"\\ud800\\ud800\\udc00\"", false),
            ("\"\\ud800x\\udc00\"", false),
            ("\"\\ud800\\n\\udc00\"", false),
            ("\"\\uZZZZ\"", false),
            ("\"\\q\"", false),
            ("\"unfinished", false),
            ("null", false),
            ("false", false),
            ("0", false),
            ("[]", false),
            ("{}", false),
        ]
        .iter()
        .enumerate()
        {
            add(
                format!("{field}-string-{index:02}"),
                Some(format!("{{{base},\"{field}\":{value}}}").into_bytes()),
                *accepted,
            );
        }
        for byte in [
            0x01, 0x08, 0x09, 0x0a, 0x0d, 0x1f, 0x20, 0x7f, 0x80, 0xc0, 0xff,
        ] {
            let mut bytes = format!("{{{base},\"{field}\":\"a").into_bytes();
            bytes.push(byte);
            bytes.extend_from_slice(b"b\"}");
            add(format!("{field}-raw-{byte:02x}"), Some(bytes), byte >= 0x20);
        }
    }
    for (index, value) in ["true", "false", "0", "1", "null", "\"true\"", "[]", "{}"]
        .iter()
        .enumerate()
    {
        add(
            format!("bool-{index}"),
            Some(format!("{{{base},\"use_win32_vk_code\":{value}}}").into_bytes()),
            index < 2,
        );
    }
    for (name, suffix, accepted) in [
        ("duplicate-known", ",\"input_method\":0", false),
        ("duplicate-escaped", ",\"input_\\u006dethod\":0", false),
        ("duplicate-unknown", ",\"x\":1,\"x\":2", false),
        ("duplicate-nested", ",\"x\":{\"a\":0,\"a\":1}", false),
        ("duplicate-array", ",\"x\":[{\"a\":0,\"a\":1}]", false),
        ("distinct-scopes", ",\"x\":[{\"a\":0},{\"a\":1}]", true),
        (
            "unknown-overflow",
            ",\"x\":184467440737095516160000000000000000000",
            true,
        ),
        ("unknown-exponent", ",\"x\":1e9999999", true),
        ("unknown-negative", ",\"x\":-1e9999999", true),
        ("unknown-leading-zero", ",\"x\":01", false),
        ("unknown-fraction", ",\"x\":0.00000000000000001", true),
        ("unknown-array-trailing-comma", ",\"x\":[1,]", false),
        ("unknown-object-trailing-comma", ",\"x\":{\"a\":1,}", false),
        ("root-trailing-comma", ",", false),
        ("comment", ",/* hello */\"x\":1", false),
    ] {
        add(
            name.into(),
            Some(format!("{{{base}{suffix}}}").into_bytes()),
            accepted,
        );
    }
    for depth in [1, 2, 32, 127, 128, 129, 256] {
        add(
            format!("unknown-depth-{depth}"),
            Some(
                format!(
                    "{{{base},\"x\":{}null{}}}",
                    "[".repeat(depth),
                    "]".repeat(depth)
                )
                .into_bytes(),
            ),
            true,
        );
    }
    for (name, tail, accepted) in [
        ("tail-space", b" \t\n".as_slice(), true),
        ("tail-extra", b"{}", false),
        // parse() treats skip_whitespace()==false as end-of-input, including
        // the slash rejection in strict mode. This is observed source behavior.
        ("tail-comment", b"//comment", true),
        ("tail-slash-junk", b"/arbitrary non-JSON", true),
        ("tail-spaced-slash", b" \r\n/", true),
        ("nul-tail", b"\0this is outside the C string", true),
    ] {
        let mut bytes = format!("{{{base}}}").into_bytes();
        bytes.extend_from_slice(tail);
        add(name.into(), Some(bytes), accepted);
    }
    for byte in 1_u8..=255 {
        let mut bytes = format!("{{{base}}}").into_bytes();
        bytes.push(byte);
        add(
            format!("tail-byte-{byte:02x}"),
            Some(bytes),
            b" \t\n\r/".contains(&byte),
        );
    }
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(format!("{{{base}}}").as_bytes());
    add("bom-prefix".into(), Some(bom), false);
    add(
        "nul-inside".into(),
        Some(format!("{{{base},\"wlr_socket_path\":\"a\0b\"}}").into_bytes()),
        false,
    );
    // Characterize the source's packed-string boundary. Padding is outside the
    // root object yet still visible to the C-string parser's remaining length.
    // A raw literal can be skipped between surrogate escapes when a full
    // 16-byte packed read is available. These are legacy invalid-Unicode cases.
    for gap in [0, 1, 7, 8, 15, 16, 17, 31, 32, 33] {
        for padding in [0, 1, 8, 16, 32] {
            let bytes = format!(
                "{{{base},\"wlr_socket_path\":\"\\ud800{}\\udc00\"}}{}",
                "x".repeat(gap),
                " ".repeat(padding)
            );
            let accepted = gap % 16 == 0 || gap % 16 + 8 + padding >= 16;
            add(
                format!("surrogate-gap-{gap}-padding-{padding}"),
                Some(bytes.into_bytes()),
                accepted,
            );
        }
    }
    for byte in 0_u8..=255 {
        let mut bytes = format!("{{{base},\"wlr_socket_path\":\"").into_bytes();
        bytes.push(byte);
        bytes.extend_from_slice(b"\"}");
        add(
            format!("string-byte-{byte:02x}"),
            Some(bytes),
            byte >= 32 && byte != b'"' && byte != b'\\',
        );
    }
    for capture in [0_u64, 1, 2, 4, 7, u64::MAX] {
        for input in [0_u64, 1, 2, 4, 7, u64::MAX] {
            let bytes=format!(r#"{{"screencap_method":{capture},"input_method":{input},"wlr_socket_path":"w\u0000\n","pw_socket_fd":-2147483648,"pw_node_id":4294967295,"uinput_screen_width":2147483647,"uinput_screen_height":-2147483648,"uinput_path":"u\t\"\\","eis_socket_path":"e\u0001","use_win32_vk_code":true}}"#).into_bytes();
            add(
                format!("metadata-capture-{capture}-input-{input}"),
                Some(bytes),
                true,
            );
        }
    }
    cases
}
