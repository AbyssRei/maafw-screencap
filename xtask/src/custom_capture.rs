// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual v1 Custom case/ABI/scoped-owner records, separate from capture.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected upstream-custom <external directory> [--check-only]".into()),
    };
    let directory = records::external_root(Path::new(path))?;
    let cases = fs::read_to_string(directory.join("cases.tsv"))?;
    if cases.lines().count() != 96 {
        return Err("Custom matrix requires all 96 rows".into());
    }
    let mut artifacts = Vec::new();
    let mut nonempty = 0;
    for (index, line) in cases.lines().enumerate() {
        let expected = case(index, line)?;
        for side in ["source", "rust"] {
            let file = directory.join(format!("case-{index}.{side}.bin"));
            if fs::read(&file)? != expected {
                return Err(format!("Custom {side} pixels differ at case {index}").into());
            }
            artifacts.push(records::artifact(&directory, &file)?);
        }
        if !expected.is_empty() {
            nonempty += 2;
        }
    }
    let abi_text = fs::read_to_string(directory.join("abi.txt"))?;
    let abi = named(&abi_text, '=')?;
    let required = [
        "callback_table_size",
        "screencap_offset",
        "pointer_size",
        "CV_8UC1",
        "CV_8UC3",
        "CV_8UC4",
    ];
    if abi.len() != required.len()
        || required.iter().any(|name| !abi.contains_key(*name))
        || abi["pointer_size"] != 8
        || abi["callback_table_size"] != 21 * 8
        || abi["screencap_offset"] != 6 * 8
    {
        return Err("Custom fixed native table identity/width differs".into());
    }
    let kinds = [abi["CV_8UC1"], abi["CV_8UC3"], abi["CV_8UC4"]];
    if kinds.iter().any(|v| i32::try_from(*v).is_err())
        || kinds[0] == kinds[1]
        || kinds[0] == kinds[2]
        || kinds[1] == kinds[2]
    {
        return Err("Custom compiled CV type constants are invalid".into());
    }
    let stats_text = fs::read_to_string(directory.join("stats.tsv"))?;
    let stats = named(&stats_text, '\t')?;
    let expected_stats = BTreeMap::from([
        ("native_mat_heads", 0),
        ("native_contexts", 0),
        ("rust_adapter_scopes_entered", 26),
        ("rust_adapter_scopes_exited", 26),
        ("retained_mat_heads", 36),
    ]);
    if stats != expected_stats {
        return Err("Custom scoped native owner counts differ".into());
    }
    for name in ["cases.tsv", "abi.txt", "stats.tsv"] {
        artifacts.push(records::artifact(&directory, &directory.join(name))?);
    }
    let report = serde_json::json!({"layout_version":1,"cases":96,"payloads":192,"nonempty_payloads":nonempty,
        "compiled_cv_types":kinds,"fixed_callback_table_size":abi["callback_table_size"],"stats":stats,"artifacts":artifacts,
        "scope":"Exact selected-body original/Rust Custom matrix and real raw ImageBuffer ownership. Stats cover opaque native Mat heads, contexts and Rust adapter stack scopes; not all native pixel allocations, full manager virtual ABI, encoded/resize/all types, SDK provider or P1 acceptance."});
    let output = directory.join("custom-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Custom saved validation differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom cases=96 source_rust_payloads=192 actual_bytes_verified=1 scoped_owners_verified=1 passed"
    );
    Ok(())
}

fn case(index: usize, line: &str) -> Result<Vec<u8>> {
    if index >= 96 {
        return Err("Custom row outside fixed matrix".into());
    }
    let channels = [1, 3, 4][index % 3];
    let mut remainder = index / 3;
    let populate = remainder % 2;
    remainder /= 2;
    let code = [0, 1, 2, 255][remainder % 4];
    remainder /= 4;
    let present = remainder % 2;
    let table = remainder / 2;
    let success = usize::from(table == 1 && present == 1 && code != 0);
    let length = if success == 0 {
        105
    } else if populate == 0 {
        0
    } else {
        7 * 5 * channels
    };
    let references = usize::from(length != 0);
    let calls = if table == 1 && present == 1 { 2 } else { 0 };
    let expected = format!(
        "{index}\t{table}\t{present}\t{code}\t{populate}\t{channels}\t{success}\t{length}\t{references}\t{calls}"
    );
    if line != expected {
        return Err(format!("Custom row/matrix mismatch at {index}").into());
    }
    Ok(if success == 0 {
        (0..105).map(|i| [9, 17, 23][i % 3]).collect()
    } else {
        (0..length)
            .map(|i| u8::try_from((i * 29 + 11) % 251 + 1).unwrap())
            .collect()
    })
}
fn named(text: &str, separator: char) -> Result<BTreeMap<&str, u64>> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once(separator)
            .ok_or("Malformed Custom named record")?;
        let parsed: u64 = value.parse()?;
        if key.is_empty() || value != parsed.to_string() || values.insert(key, parsed).is_some() {
            return Err("Duplicate/noncanonical Custom named record".into());
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_changed_failure_outputs_nonzero_results_spans_and_omitted_calls() {
        let failure = "0\t0\t0\t0\t0\t1\t0\t105\t1\t0";
        assert_eq!(case(0, failure).unwrap().len(), 105);
        assert!(case(0, &failure.replace("\t105\t", "\t0\t")).is_err());
        let success = "95\t1\t1\t255\t1\t4\t1\t140\t1\t2";
        assert_eq!(case(95, success).unwrap().len(), 140);
        assert!(case(95, &success.replace("\t140\t", "\t141\t")).is_err());
        assert!(case(95, &success.replace("\t255\t", "\t1\t")).is_err());
        assert!(case(95, success.strip_suffix("\t2").unwrap()).is_err());
        assert!(case(94, success).is_err());
        assert!(named("pointer_size=08\n", '=').is_err());
        assert!(named("pointer_size=8\npointer_size=8\n", '=').is_err());
        assert!(named("pointer_size=-1\n", '=').is_err());
    }
}
