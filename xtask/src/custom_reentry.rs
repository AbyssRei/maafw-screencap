// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independently inspect every nested virtual result, callback and native owner.
use crate::{Result, records};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace custom-reentry <external reentry directory> [--check-only]"
                .into(),
        ),
    };
    let directory = records::external_root(Path::new(path))?;
    let parent = directory.parent().ok_or("Custom reentry parent missing")?;
    crate::custom_capture::run(&[parent.to_string_lossy().into_owned(), "--check-only".into()])?;
    let parent_abi_path = parent.join("abi.txt");
    let parent_abi = fs::read_to_string(&parent_abi_path)?;
    let kind = parent_abi
        .lines()
        .find_map(|line| line.strip_prefix("CV_8UC3="))
        .ok_or("Missing actual BGR kind")?
        .parse::<i32>()?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = directory.join(name);
        artifacts.push(records::artifact(&directory, &path)?);
        Ok(fs::read(path)?)
    };
    if read("pixel-kind.txt")? != format!("{kind}\n").as_bytes() {
        return Err("Custom nested actual image ABI differs".into());
    }
    let rows = read("queries.tsv")?;
    let callbacks = read("callbacks.tsv")?;
    let mut expected_rows = String::new();
    let mut expected_callbacks = String::new();
    let mut expected_lifecycles = String::new();
    let mut queries = 0;
    let mut actual_values = 0;
    for cycle in 0..20 {
        for outer in 0..7 {
            for inner in 0..7 {
                for fault in 0..2 {
                    for side in ["source", "rust"] {
                        expected_callbacks.push_str(&format!(
                            "{cycle}\t{outer}\t{inner}\t{fault}\t{side}\t0\t{outer}\n"
                        ));
                        if fault == 0 {
                            expected_callbacks.push_str(&format!(
                                "{cycle}\t{outer}\t{inner}\t{fault}\t{side}\t1\t{inner}\n"
                            ));
                        }
                        for (depth, operation) in [(1, inner), (0, outer)] {
                            let failed = depth == 1 && fault == 1;
                            let status = if failed { 3 } else { 0 };
                            let value = if failed {
                                0
                            } else {
                                scalar(cycle, depth, operation)
                            };
                            expected_rows.push_str(&format!("{cycle}\t{outer}\t{inner}\t{fault}\t{side}\t{depth}\t{operation}\t{status}\t{value}\n"));
                            let name = format!("{cycle}-{outer}-{inner}-{fault}-{side}-{depth}");
                            if operation >= 4 {
                                let expected = bytes(cycle, depth, operation, failed);
                                if read(&format!("{name}.bin"))? != expected {
                                    return Err(format!(
                                        "Custom nested caller bytes differ: {name}/{operation}"
                                    )
                                    .into());
                                }
                                actual_values += 1;
                            }
                            if operation == 5
                                && read(&format!("{name}.layout"))?
                                    != format!("7\t5\t3\t21\t{kind}\t105\t1\n").as_bytes()
                            {
                                return Err(format!(
                                    "Custom nested Mat layout/owner differs: {name}"
                                )
                                .into());
                            }
                            if fault == 0
                                && outer == inner
                                && outer >= 4
                                && read(&format!("{name}.held"))?
                                    != bytes(cycle, depth, operation, false)
                            {
                                return Err("Custom nested native values did not survive unit/context destruction".into());
                            }
                            queries += 1;
                        }
                    }
                }
            }
        }
        expected_lifecycles.push_str(&format!("{cycle}\t196\t[42, 42, 42, 42, 42, 42, 42]\t1\n"));
    }
    if rows != expected_rows.as_bytes() || queries != 7840 || actual_values != 3360 {
        return Err("Custom nested query order/status/full-width result matrix differs".into());
    }
    if callbacks != expected_callbacks.as_bytes()
        || read("lifecycles.tsv")? != expected_lifecycles.as_bytes()
    {
        return Err(
            "Custom nested callback depth/order or actual destructor evidence differs".into(),
        );
    }
    if read("scope-controls.tsv")? != b"outside-publication\t3\nouter-type-before-release\t1\nforeign-buffer-parse\t1\nouter-type-during-inner\t1\ninner-type-before-release\t1\ninner-type-after-release\t0\nouter-type-after-release\t0\n"
        || read("control-inner.json.bin")? != br#"{"scope":"inner","type":"custom"}"#
        || read("control-outer.json.bin")? != br#"{"scope":"outer","type":"custom"}"# {
        return Err("Custom nested scope guards lost early/foreign/outside rejection or valid enclosing-scope publication".into());
    }
    let counts = [
        20, 20, 20, 20, 20, 420, 420, 0, 0, 0, 0, 560, 560, 1122, 1122, 1681, 422, 422, 422,
    ];
    let expected_stats = counts
        .iter()
        .enumerate()
        .map(|(i, value)| format!("{i}\t{value}\n"))
        .collect::<String>();
    if read("stats.tsv")? != expected_stats.as_bytes() {
        return Err("Custom nested real value/scope/publication ledgers differ".into());
    }
    if artifacts.len() != 4728 {
        return Err("Custom nested complete payload/owner inventory differs".into());
    }
    let report = serde_json::json!({"layout_version":1,"lifecycles":20,"operation_pairs":49,"fault_variants":2,
        "actual_virtual_queries":queries,"actual_output_payloads":actual_values,"nested_cpp_exceptions":1960,
        "native_values_retained_after_close":240,"actual_scoped_publications":840,"scope_controls":7,
        "parent_image_abi":records::artifact(parent,&parent_abi_path)?,"artifacts":artifacts,
        "scope":"Same live original public C++ virtual unit and same Rust manager, one synchronous nested call from each callback with a distinct native caller. Actual buffer/Mat/string/JSON scope, real C++ throws, original borrowed table restoration and value owners. No input actions, concurrent use/destruction, SDK reentry policy, general heap-failure or full P1 acceptance."});
    let output = directory.join("custom-reentry-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Custom nested assessment differs from actual files".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom reentry lifecycles=20 actual_queries=7840 actual_payloads=3360 retained_values=240 scope_controls=7 passed"
    );
    Ok(())
}
fn scalar(cycle: usize, depth: u32, operation: u32) -> u64 {
    let code = [0, 1, 2, 255][(cycle + usize::try_from(depth).unwrap()) % 4];
    match operation {
        0 => u64::from(code != 0),
        1 => u64::from(code == 0),
        2 => u64::from(code != 1),
        3 => u64::MAX - u64::try_from(cycle).unwrap() * 17 - u64::from(depth),
        4..=6 => 1,
        _ => unreachable!(),
    }
}
fn bytes(cycle: usize, depth: u32, operation: u32, failed: bool) -> Vec<u8> {
    match (operation, failed) {
        (4, false) => {
            let mut value = format!("uuid-{cycle}-{depth}\0").into_bytes();
            value.push(255);
            value
        }
        (5, false) => (0..105)
            .map(|i| {
                u8::try_from((i * 17 + cycle * 23 + usize::try_from(depth).unwrap() * 41 + 7) % 256)
                    .unwrap()
            })
            .collect(),
        (6, false) => {
            format!("{{\"cycle\":{cycle},\"depth\":{depth},\"type\":\"custom\"}}").into_bytes()
        }
        (4, true) => b"original\0\xffstate".to_vec(),
        (5, true) => [9, 17, 23].repeat(35),
        (6, true) => b"{}".to_vec(),
        _ => Vec::new(),
    }
}
