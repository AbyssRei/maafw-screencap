// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Original-interface macOS manager query records; never capture proof.
use crate::{Result, manager_metadata::case_matrix, records};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-macos-queries <external directory> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let summary = fs::read_to_string(root.join("summary.txt"))?;
    let environment = summary
        .lines()
        .find_map(|line| line.strip_prefix("environment="))
        .ok_or("missing query environment")?
        .parse::<u32>()?;
    let source = fs::read_to_string(root.join("queries.tsv"))?;
    let held = fs::read_to_string(root.join("held.tsv"))?;
    let controls = fs::read_to_string(root.join("controls.tsv"))?;
    validate(environment, &summary, &source, &held, &controls)?;
    let libraries: Vec<_> = [
        "mfs_macos_manager_queries.dll",
        "libmfs_macos_manager_queries.so",
        "libmfs_macos_manager_queries.dylib",
    ]
    .into_iter()
    .filter(|name| root.join(name).is_file())
    .collect();
    if libraries.len() != 1
        || (environment != 0 && libraries[0] != "libmfs_macos_manager_queries.dylib")
        || (environment == 0 && libraries[0].ends_with(".dylib"))
    {
        return Err("query environment and single actual native library disagree".into());
    }
    let artifacts = [
        "summary.txt",
        "queries.tsv",
        "held.tsv",
        "controls.tsv",
        libraries[0],
    ]
    .into_iter()
    .map(|name| records::artifact(&root, &root.join(name)))
    .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({
        "layout_version":1,"environment":environment,"cases":66,"factory_units":133,
        "matrix_units":132,"snapshots":528,"virtual_queries":2113,"retained":528,"host_callbacks":1255,
        "rejection_controls":6,"captures":0,"input_none":true,"artifacts":artifacts,
        "scope":if environment == 0 { "Portable controlled-unavailable source/Rust query facade through real original headers/factories/virtual interface. Not native macOS acceptance." } else { "Native macOS source/Rust query facade through real original headers/factories/virtual interface and actual runtime availability; no screenshot, TCC, full manager/input/framework or P1 acceptance." },
        "adaptation":"Same private Rust manager through synchronous callbacks, fixed CPU copies, and explicit native JSON parse/representation; query-only source facade is not the entire original manager."
    });
    let path = root.join("query-validation.json");
    if check_only {
        if path.exists() && records::read::<serde_json::Value>(&path)? != report {
            return Err("saved query assessment differs from actual records".into());
        }
    } else {
        records::create_json(&path, &report)?;
    }
    println!(
        "macOS manager queries environment={environment} snapshots=528 virtual_queries=2113 actual_records_verified=1 captures=0 passed"
    );
    Ok(())
}
fn validate(
    environment: u32,
    summary: &str,
    source: &str,
    held: &str,
    controls: &str,
) -> Result<()> {
    if environment > 2 {
        return Err("unknown query environment".into());
    }
    let expected = format!(
        "version=1\nenvironment={environment}\ncases=66\nfactory_units=133\nmatrix_units=132\nsnapshots=528\nvirtual_queries=2113\nmatrix_queries=2112\nretained=528\ncaptures=0\ninput_none=1\njoined_cpu_reader=1\ncallback_counts=132,264,264,264,264,66\nfailed_callback_count=1\nrejection_controls=6\n"
    );
    if summary != expected {
        return Err("query summary differs from complete execution".into());
    }
    let mut expected = String::new();
    for side in ["source", "rust"] {
        for (case, (id, method)) in case_matrix("macos").unwrap().into_iter().enumerate() {
            for phase in ["cold", "connected", "reconnected", "inactive"] {
                let connected = u8::from(phase != "cold" && (method != 1 || environment == 1));
                let signed = i64::from_ne_bytes(method.to_ne_bytes());
                expected.push_str(&format!("{side}\t{case}\t{phase}\t{id}\t{method}\t{connected}\t1\t0\t{id}\t{{\"input_method\":0,\"screencap_method\":{signed},\"type\":\"macos\",\"window_id\":{id}}}\n"));
            }
        }
    }
    if source != expected || held != expected {
        return Err("source/Rust/retained query values, counts or sequence differ".into());
    }
    if controls
        != "null_create_output\t1\nnull_host_context\t1\t1\nnull_connect_owner\t1\t0\nnull_query_owner\t1\t1\nnull_original_destroy\t0\ncallback_failure\t2\t1\n"
    {
        return Err("actual native rejection controls differ".into());
    }
    Ok(())
}
