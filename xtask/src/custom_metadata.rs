// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independently inspect actual byte outputs; never decode them through a
//! replacement UTF-8/number parser that would erase source-specific behavior.
use crate::{Result, records};
use std::{fs, path::Path};
#[path = "../../tests/upstream-parity/custom-capture/metadata_cases.rs"]
mod corpus;

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace custom-metadata <external metadata directory> [--check-only]"
                .into(),
        ),
    };
    let directory = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = directory.join(name);
        artifacts.push(records::artifact(&directory, &path)?);
        Ok(fs::read(path)?)
    };
    let cases = corpus::cases();
    if cases.len() != 37 {
        return Err("Custom metadata corpus incomplete".into());
    }
    for case in &cases {
        if read(&format!("{}.input", case.name))? != case.input {
            return Err(format!("Custom metadata input changed: {}", case.name).into());
        }
    }
    let rows = String::from_utf8(read("cases.tsv")?)?;
    let mut expected_rows = String::new();
    let mut index = 0;
    for table in [false, true] {
        for present in [false, true] {
            for code in [0, 1, 2, 255] {
                for case in &cases {
                    let active = table && present;
                    let expected = if active && code != 0 {
                        case.expected
                    } else {
                        corpus::DEFAULT
                    };
                    for side in ["source", "rust"] {
                        if read(&format!("{index}-{side}.json.bin"))? != expected {
                            return Err(format!(
                                "Custom metadata {side} raw bytes differ at {index} ({})",
                                case.name
                            )
                            .into());
                        }
                    }
                    expected_rows.push_str(&format!(
                        "{index}\t{}\t{}\t{code}\t{}\t{}\t{}\t{}\n",
                        u8::from(table),
                        u8::from(present),
                        case.name,
                        expected.len(),
                        if active { 2 } else { 0 },
                        u8::from(active && code != 0 && case.object)
                    ));
                    index += 1;
                }
            }
        }
    }
    if rows != expected_rows || index != 592 {
        return Err("Custom metadata callbacks/control rows differ".into());
    }
    let parsed = cases.iter().filter(|c| c.object).count() * 3 + 2;
    let expected_stats = [0, 599, 113, parsed, 597, 152, 152, 0, 0]
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{i}\t{v}\n"))
        .collect::<String>();
    if read("stats.tsv")? != expected_stats.as_bytes() {
        return Err("Custom metadata parser/scope/object owners differ".into());
    }
    if read("abi.txt")? != b"8\n160\n" {
        return Err("Custom get_info actual native callback ABI differs".into());
    }
    if read("controls.tsv")? != b"absent\t0\npresent\t2\nremoved\t2\nexception\t3\t3\nrecovered\t4\nnull\t17\ntype-inside-scope\t1\n" { return Err("Custom metadata mutation/exception/lifetime controls differ".into()); }
    for label in ["absent", "present", "removed", "recovered"] {
        for side in ["source", "rust"] {
            let expected: &[u8] = if matches!(label, "present" | "recovered") {
                b"{\"type\":\"custom\",\"v\":\"\xff\"}"
            } else {
                corpus::DEFAULT
            };
            if read(&format!("{label}-{side}.json.bin"))? != expected {
                return Err("Custom metadata control bytes differ".into());
            }
        }
    }
    if read("null.json.bin")? != corpus::DEFAULT {
        return Err("Missing Custom manager did not produce the source default object".into());
    }
    let report = serde_json::json!({"layout_version":1,"cases":592,"payloads":37,"actual_json_payloads":1193,"retained_pairs":592,
        "native_string_scopes":152,"parse_calls":113,"independent_parsed_objects":parsed,"type_replacements":597,"artifacts":artifacts,
        "scope":"Actual original/Rust Custom callback/default/parse/merge with real StringBuffer/meojson object owners. Byte/syntax/numeric semantics and lifetime order retained in explicit acceptance adapter; opaque heads add bookkeeping allocations. No core JSON dependency, full manager virtual ABI, SDK provider or P1 acceptance."});
    let output = directory.join("custom-metadata-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Custom metadata verification differs from actual data".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom metadata cases=592 actual_json_payloads=1193 native_string_scopes=152 source_parser_and_owners=1 passed"
    );
    Ok(())
}
