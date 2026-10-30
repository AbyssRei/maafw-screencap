// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Inspect actual original/Rust public-vtable query outputs and native owners.
use crate::{Result, records};
use std::{fs, path::Path};
const UUID: &[u8] = b"custom-native\0\xff\x80uuid";
const DEFAULT: &[u8] = br#"{"type":"custom"}"#;
const INFO:&[u8]=b"{\"large\":1e1000,\"nested\":[true,null],\"nul\":\"a\\u0000b\",\"type\":\"custom\",\"v\":\"\xff\"}";

pub fn run(args: &[String]) -> Result<()> {
    let (path,check_only)=match args {
        [path]=>(path,false),[path,flag] if flag=="--check-only"=>(path,true),
        _=>return Err("expected upstream-trace custom-native-api <external native-api directory> [--check-only]".into()),
    };
    let directory = records::external_root(Path::new(path))?;
    let parent = directory
        .parent()
        .ok_or("Custom native API parent missing")?;
    let parent_abi = parent.join("abi.txt");
    let parent_artifact = records::artifact(parent, &parent_abi)?;
    // Recheck the actual parent pixels/ABI too; CV type packing is not frozen
    // to an old OpenCV major or inferred from a row in the new consumer alone.
    crate::custom_capture::run(&[parent.to_string_lossy().into_owned(), "--check-only".into()])?;
    let parent_abi = fs::read_to_string(parent_abi)?;
    let field = parent_abi
        .lines()
        .find_map(|line| line.strip_prefix("CV_8UC3="))
        .ok_or("Actual parent CV_8UC3 is missing")?;
    let kind = field.parse::<i32>()?;
    if kind < 0 || kind.to_string() != field {
        return Err("Noncanonical actual Custom image type".into());
    }
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = directory.join(name);
        artifacts.push(records::artifact(&directory, &path)?);
        Ok(fs::read(path)?)
    };
    if read("version.txt")? != b"mfs-p1-custom-api"
        || read("abi.txt")? != b"8\n8\n8\n8\n8\n8\n"
        || read("pixel-kind.txt")? != format!("{kind}\n").as_bytes()
    {
        return Err("Custom actual native version/ABI/header type differs".into());
    }
    if read("creation-controls.tsv")? != b"null-table\t0\t0\nnull-destroy\t2\n" {
        return Err("Custom original null-create/destroy controls missing".into());
    }
    if read("publication-controls.tsv")? != b"outside-scopes\t3\n" {
        return Err("Custom actual caller assignment was not refused outside buffer scopes".into());
    }
    let rows = String::from_utf8(read("cases.tsv")?)?;
    let mut expected_rows = String::new();
    let mut paired_queries = 0;
    let mut lifecycle_rows = String::new();
    for cycle in 0..20 {
        let code = [0, 1, 2, 255][cycle % 4];
        let features = [1u64 << 63, u64::MAX, 0, (1u64 << 32) | 7][cycle % 4];
        let mut uuid = b"original\0\xffstate".to_vec();
        let mut image = [9u8, 17, 23].repeat(35);
        let mut info = b"{}".to_vec();
        let mut phases = Vec::new();
        for phase in ["absent", "present", "empty", "removed"] {
            for op in 0..7 {
                phases.push((phase.to_string(), op));
            }
        }
        if cycle == 0 {
            for op in 0..7 {
                phases.push((format!("fault-{op}"), op));
                phases.push((format!("recovered-{op}"), op));
            }
        }
        for op in [1, 4, 5, 6] {
            phases.push(("hold".into(), op));
        }
        for (phase, op) in phases {
            let fault = phase.starts_with("fault-");
            let recovery = phase.starts_with("recovered-");
            let present = !matches!(phase.as_str(), "absent" | "removed");
            let empty = phase == "empty";
            let active_code = if empty || recovery || phase == "hold" {
                255
            } else {
                code
            };
            let boolean_code = if recovery || phase == "hold" {
                255
            } else {
                code
            };
            let status = if fault { 3 } else { 0 };
            let value = if fault {
                0
            } else {
                match op {
                    0 => u64::from(present && boolean_code != 0),
                    1 => u64::from(!present || boolean_code == 0),
                    2 => u64::from(!present || boolean_code != 1),
                    3 => {
                        if present {
                            features
                        } else {
                            0
                        }
                    }
                    4 | 5 => u64::from(present && active_code != 0),
                    6 => 1,
                    _ => unreachable!(),
                }
            };
            if !fault {
                match op {
                    4 if value != 0 => uuid = if empty { Vec::new() } else { UUID.to_vec() },
                    5 if value != 0 => image = if empty { Vec::new() } else { pixels(cycle) },
                    6 => {
                        info = if present && active_code != 0 && !empty {
                            INFO.to_vec()
                        } else {
                            DEFAULT.to_vec()
                        }
                    }
                    _ => {}
                }
            }
            expected_rows.push_str(&format!(
                "{cycle}\t{phase}\t{op}\t{status}\t{status}\t{value}\t{value}\n"
            ));
            if op >= 4 {
                let expected = match op {
                    4 => &uuid,
                    5 => &image,
                    6 => &info,
                    _ => unreachable!(),
                };
                for side in ["source", "rust"] {
                    if read(&format!("{cycle}-{phase}-{side}-{op}.bin"))? != *expected {
                        return Err(format!(
                            "Custom actual {side} vtable bytes differ at {cycle}/{phase}/{op}"
                        )
                        .into());
                    }
                    if op == 5 {
                        let layout = if image.is_empty() {
                            "0\t0\t1\t0\t0\t0\t0\n".into()
                        } else {
                            format!("7\t5\t3\t21\t{kind}\t105\t1\n")
                        };
                        if read(&format!("{cycle}-{phase}-{side}.layout"))? != layout.as_bytes() {
                            return Err("Custom actual Mat layout/owner count differs".into());
                        }
                    }
                }
            }
            paired_queries += 1;
        }
        for side in ["source", "rust"] {
            if read(&format!("{cycle}-held-{side}.bgr"))? != pixels(cycle) {
                return Err(
                    "Custom pixels did not retain their real native owner after API destruction"
                        .into(),
                );
            }
        }
        let (operations, calls) = if cycle == 0 {
            (46, [6, 8, 6, 6, 8, 8, 8])
        } else {
            (32, [4, 6, 4, 4, 6, 6, 6])
        };
        lifecycle_rows.push_str(&format!("{cycle}\t15\t15\t1\t{operations}\t{calls:?}\t0\n"));
    }
    if rows != expected_rows || paired_queries != 654 {
        return Err("Custom actual query order/status/value matrix differs".into());
    }
    if read("lifecycles.tsv")? != lifecycle_rows.as_bytes() {
        return Err(
            "Custom public bases/drop callback/native argument lifetime evidence differs".into(),
        );
    }
    let counters = [
        20, 20, 20, 20, 20, 56, 56, 0, 0, 0, 0, 62, 62, 124, 124, 142, 56, 36, 101,
    ];
    let stats = counters
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{i}\t{v}\n"))
        .collect::<String>();
    if read("stats.tsv")? != stats.as_bytes() {
        return Err("Custom actual creation/destruction/value/scope ledgers differ".into());
    }
    let report = serde_json::json!({"layout_version":1,"lifecycles":20,"source_rust_virtual_queries":1308,"paired_queries":654,
        "native_pixel_pairs_retained_after_close":20,"rust_destructor_callbacks":20,"actual_public_bases_mask":15,"scoped_actual_caller_publications":112,
        "parent_image_abi":parent_artifact,"artifacts":artifacts,
        "scope":"Complete original Custom API/manager compiled unchanged with logging overlay; actual creation/destruction, capture-only public virtual consumers and Rust manager dispatch. Actual Mat/string/JSON values and synchronous owners; Rust unit has uninvoked false input/app/shell slots solely to instantiate the original interface. UUID acceptance adds a byte copy and opaque native heads add bookkeeping allocations. Not general heap-leak, input, framework-loader, SDK or full P1 acceptance."});
    let output = directory.join("custom-native-api-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Custom native API assessment differs from actual files".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom native API lifecycles=20 actual_virtual_queries=1308 retained_pixel_pairs=20 native_owners_closed=1 passed"
    );
    Ok(())
}
fn pixels(cycle: usize) -> Vec<u8> {
    (0..105)
        .map(|i| u8::try_from((i * 17 + cycle * 23 + 7) % 256).unwrap())
        .collect()
}
