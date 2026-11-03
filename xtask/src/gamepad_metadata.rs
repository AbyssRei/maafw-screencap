// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Complete original-class query records, with no capture or input capability.
use crate::{Result, records};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Info {
    gamepad_type: i64,
    hwnd: u64,
    screencap_method: i64,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Row {
    hwnd: u64,
    gamepad_type: u64,
    method: u64,
    phase: u8,
    uuid: String,
    features: u64,
    info: Info,
}
fn read_rows(path: &Path) -> Result<Vec<Row>> {
    let rows = fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str::<Row>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if rows.len() != 288 {
        return Err("Gamepad metadata requires all 144 cases and both query phases".into());
    }
    let mut index = 0;
    for hwnd in [0, 1, 0x1234_5678_9abc_def0, u64::MAX] {
        for kind in [0, 1, 2, (1 << 63) - 1, 1 << 63, u64::MAX] {
            for method in [0, 1, 63, 64, 1 << 63, u64::MAX] {
                for phase in 0..2 {
                    let row = &rows[index];
                    let type_name = if kind == 0 { "xbox360" } else { "ds4" };
                    if row.hwnd != hwnd
                        || row.gamepad_type != kind
                        || row.method != method
                        || row.phase != phase
                        || row.uuid != format!("gamepad_{type_name}_hwnd_{hwnd}")
                        || row.features != 7
                        || row.info
                            != (Info {
                                hwnd,
                                gamepad_type: i64::from_ne_bytes(kind.to_ne_bytes()),
                                screencap_method: i64::from_ne_bytes(method.to_ne_bytes()),
                                kind: "gamepad".into(),
                            })
                    {
                        return Err(format!("Gamepad metadata query case {index} differs").into());
                    }
                    index += 1;
                }
            }
        }
    }
    Ok(rows)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected upstream-delegate gamepad-metadata <existing external records> [--check-only]".into()),
    };
    if !Path::new(path).is_dir() {
        return Err("Gamepad metadata records must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    let expected: BTreeSet<_> = ["source.jsonl", "rust.jsonl", "source.stderr"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("Unexpected Gamepad metadata directory/link".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 metadata filename")?;
        if name != "gamepad-metadata-validation.json" {
            actual.insert(name);
        }
    }
    if actual != expected {
        return Err("Gamepad metadata record inventory differs".into());
    }
    let source = read_rows(&root.join("source.jsonl"))?;
    let rust = read_rows(&root.join("rust.jsonl"))?;
    if source != rust {
        return Err("Original/Rust Gamepad metadata observations differ".into());
    }
    if fs::read_to_string(root.join("source.stderr"))?.trim()
        != "original_cases=144 phases=288 queries=864 input_owners=0 capture_units=0 native_capture=0"
    {
        return Err("Original Gamepad query owner/count summary differs".into());
    }
    let inputs = expected
        .iter()
        .map(|name| records::artifact(&root, &root.join(name)))
        .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"cases":144,"phases":288,"queries_per_side":864,"registered_files":3,"input_owners":0,"capture_units":0,"native_captures":0,"full_P1_accepted":false,"inputs":inputs,
        "scope":"Complete unchanged F Gamepad class/header/API query consumption versus the same private Rust GamepadCapture. Both query phases precede connection; scalars include null/high/max HWND, Xbox360/DS4/nonstandard/high type values and high method masks. UUID replacement/retained values, source feature metadata and exact signed metadata casts are checked independently. Source feature bits are historical metadata, not implemented input capability. No connect, ViGEm/input, foreground, capture resource/frame, public SDK, arbitrary allocation failure or full P1 acceptance."});
    let output = root.join("gamepad-metadata-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Gamepad metadata report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Gamepad metadata records cases=144 phases=288 queries_per_side=864 files=3 input_owners=0 capture_units=0 native_capture=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
