// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[path = "../../../tests/upstream-parity/linux-manager/config-cases.rs"]
mod corpus;

#[test]
fn rejects_typed_overflow_and_nested_duplicates_but_preserves_unknown_lexemes() {
    for case in corpus::cases() {
        let result = Config::parse_c_string(case.input.as_deref());
        assert_eq!(result.is_ok(), case.accepted, "{}: {result:?}", case.name);
        if let Ok(config) = result {
            let info = config.info().unwrap();
            let again = Config::parse_c_string(Some(&info)).unwrap();
            assert_eq!(info, again.info().unwrap(), "{}", case.name);
            assert_eq!(
                config.uuid().unwrap(),
                again.uuid().unwrap(),
                "{}",
                case.name
            );
        }
    }
}

#[test]
fn retains_binary_strings_and_input_metadata_without_performing_input_actions() {
    let config=Config::parse_c_string(Some(br#"{"screencap_method":1,"input_method":2,"wlr_socket_path":"a\u0000b","uinput_path":"p\nq","uinput_screen_width":-2147483648,"pw_node_id":4294967295}"#)).unwrap();
    assert_eq!(config.uuid().unwrap(), b"a\0b_p\nq");
    let info = config.info().unwrap();
    assert!(info.windows(12).any(|b| b == b"-2147483648,"));
    assert_eq!(
        Config::parse_c_string(Some(&info)).unwrap().uuid().unwrap(),
        b"a\0b_p\nq"
    );
}

#[test]
#[ignore = "requires explicitly archived original configuration/API observations"]
fn matches_original_configuration_creation_uuid_and_metadata_bytes() {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_P1_LINUX_CONFIG_OBSERVATIONS").expect("original observations"),
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("MFS_P1_LINUX_CONFIG_RUST_ARTIFACTS").expect("new Rust config artifacts"),
    );
    std::fs::create_dir(&output).unwrap();
    let mut accepted = 0;
    let mut rejected = 0;
    for case in corpus::cases() {
        let source = root.join(&case.name);
        if let Some(input) = &case.input {
            assert_eq!(std::fs::read(source.join("input.bin")).unwrap(), *input);
        }
        let text = std::fs::read_to_string(source.join("stdout.txt")).unwrap();
        let fields: std::collections::BTreeMap<_, _> = text
            .lines()
            .map(|line| line.split_once('=').unwrap())
            .collect();
        let result = Config::parse_c_string(case.input.as_deref());
        assert_eq!(result.is_ok(), fields["created"] == "1", "{}", case.name);
        if let Ok(config) = result {
            accepted += 1;
            let uuid = config.uuid().unwrap();
            let info = config.info().unwrap();
            assert_eq!(uuid, unhex(fields["uuid_hex"]), "{} UUID", case.name);
            assert_eq!(info, unhex(fields["info_hex"]), "{} metadata", case.name);
            std::fs::write(output.join(format!("{}.uuid", case.name)), uuid).unwrap();
            std::fs::write(output.join(format!("{}.info", case.name)), info).unwrap();
        } else {
            rejected += 1;
        }
    }
    assert_eq!(accepted + rejected, corpus::cases().len());
    println!(
        "Rust Linux config cases={} accepted={accepted} rejected={rejected} exact_uuid_metadata=1 input_actions=0 passed",
        accepted + rejected
    );
}
fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2));
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
