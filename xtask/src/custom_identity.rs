// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual original/Rust Custom UUID bytes and native StringBuffer records.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace custom-identity <external identity directory> [--check-only]"
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
    let rows = String::from_utf8(read("cases.tsv")?)?;
    if rows.lines().count() != 64 {
        return Err("Custom identity requires all 64 original/Rust cases".into());
    }
    for (index, line) in rows.lines().enumerate() {
        let expected = case(index, line)?;
        for side in ["source", "rust"] {
            if read(&format!("{index}-{side}.uuid"))? != expected {
                return Err(format!("Custom {side} complete UUID bytes differ at {index}").into());
            }
        }
    }
    if String::from_utf8(read("abi.txt")?)? != "16\n8\n0\n8\n8\n8\n8\n8\n16\n24\n" {
        return Err("Custom native string view/callback/feature layout differs".into());
    }
    let stats_text = String::from_utf8(read("stats.tsv")?)?;
    let mut stats = BTreeMap::new();
    for line in stats_text.lines() {
        let (key, value) = line
            .split_once('\t')
            .ok_or("Malformed Custom identity ledger")?;
        let number: u64 = value.parse()?;
        if number.to_string() != value || stats.insert(key, number).is_some() {
            return Err("Noncanonical/duplicate Custom identity ledger".into());
        }
    }
    if stats
        != BTreeMap::from([
            ("uuid_scopes_entered", 20),
            ("uuid_scopes_exited", 20),
            ("native_uuid_outputs", 0),
            ("native_contexts", 0),
            ("retained_uuid_pairs", 64),
        ])
    {
        return Err("Custom identity scoped owners differ".into());
    }
    let wide = u64::MAX;
    let controls = format!(
        "table-absent\t0\t0\t0\t0\ntable-present\t1\t1\t{wide}\t{wide}\ntable-removed\t0\t0\t0\t0\nnull-manager\t0\t0\nuuid-exception\t3\t3\nrecovery-0\t1\t1\t{wide}\t{wide}\nfeatures-exception\t3\t3\nrecovery-1\t1\t1\t{wide}\t{wide}\n"
    );
    if String::from_utf8(read("controls.tsv")?)? != controls {
        return Err("Custom identity mutation/exception controls missing".into());
    }
    for name in [
        "exception-source.uuid",
        "exception-rust.uuid",
        "recovered-source.uuid",
        "recovered-rust.uuid",
    ] {
        if read(name)? != [0, 0xff, b'a', 0, 0x80, b'z'] {
            return Err("Custom exception/recovery complete UUID bytes differ".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"cases":64,"matrix_uuid_queries":128,"matrix_feature_queries":128,
        "control_uuid_queries":13,"control_feature_queries":13,"uuid_payloads":132,"native_stack_string_scopes":20,"retained_uuid_pairs":64,"artifacts":artifacts,
        "scope":"Actual selected-body original/Rust UUID/features, real StringBuffer and owned std::string/CPU byte outputs; no input actions, full manager virtual ABI, metadata, SDK provider or P1 acceptance."});
    let output = directory.join("custom-identity-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Custom identity saved verification differs from actual data".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom identity cases=64 actual_uuid_payloads=132 native_layout_and_scoped_owners=1 passed"
    );
    Ok(())
}

fn case(index: usize, line: &str) -> Result<Vec<u8>> {
    if index >= 64 {
        return Err("Custom identity case outside matrix".into());
    }
    let kind = index % 4;
    let code = [0, 1, 2, 255][index / 4 % 4];
    let present = index / 16 % 2;
    let table = index / 32;
    let features = [0, 1u64 << 63, u64::MAX, (1u64 << 32) | 7][kind];
    let success = usize::from(table == 1 && present == 1 && code != 0);
    let expected = if success == 0 {
        (0..257)
            .map(|n| u8::try_from((n * 11 + 3) % 256).unwrap())
            .collect::<Vec<_>>()
    } else {
        match kind {
            0 => Vec::new(),
            1 => b"custom-capture-id".to_vec(),
            2 => vec![0, 0xff, b'a', 0, 0x80, b'z'],
            3 => (0..1025)
                .map(|n| u8::try_from((n * 31 + 7) % 256).unwrap())
                .collect(),
            _ => unreachable!(),
        }
    };
    let active = table == 1 && present == 1;
    let actual_features = if active { features } else { 0 };
    let calls = if active { 2 } else { 0 };
    let row = format!(
        "{index}\t{table}\t{present}\t{code}\t{kind}\t{features}\t{success}\t{}\t{actual_features}\t{actual_features}\t{calls}\t{calls}",
        expected.len()
    );
    if line != row {
        return Err(format!("Custom identity row/width/call count differs at {index}").into());
    }
    Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_truncated_features_missing_calls_and_changed_failure_outputs() {
        let wide = "63\t1\t1\t255\t3\t4294967303\t1\t1025\t4294967303\t4294967303\t2\t2";
        assert_eq!(case(63, wide).unwrap().len(), 1025);
        assert!(case(63, &wide.replace("4294967303", "7")).is_err());
        assert!(case(63, wide.strip_suffix("\t2").unwrap()).is_err());
        let failure = "0\t0\t0\t0\t0\t0\t0\t257\t0\t0\t0\t0";
        assert_eq!(case(0, failure).unwrap().len(), 257);
        assert!(case(0, &failure.replace("\t257\t", "\t0\t")).is_err());
        assert!(case(64, wide).is_err());
    }
}
