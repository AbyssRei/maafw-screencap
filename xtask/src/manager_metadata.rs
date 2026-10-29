// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual v1 original/Rust get_info case-table and row format, without capture.
use crate::{Result, records};
use std::{fs, path::Path};
#[path = "../../tests/upstream-parity/manager-metadata/cases.rs"]
mod cases;

pub fn run(args: &[String]) -> Result<()> {
    let (kind, path, check_only) = match args {
        [kind, path] => (kind, path, false),
        [kind, path, flag] if flag == "--check-only" => (kind, path, true),
        _ => {
            return Err(
                "expected upstream-metadata <win32|macos> <external directory> [--check-only]"
                    .into(),
            );
        }
    };
    let matrix = cases::cases(kind).ok_or("unsupported metadata target kind")?;
    if !Path::new(path).is_dir() {
        return Err("metadata artifacts are missing".into());
    }
    let directory = records::external_root(Path::new(path))?;
    let input = fs::read_to_string(directory.join("cases.tsv"))?;
    let source = fs::read_to_string(directory.join("source-stdout.txt"))?;
    let rust = fs::read_to_string(directory.join("rust-stdout.txt"))?;
    validate(kind, &matrix, &input, &source, &rust)?;
    let artifacts = [
        "cases.tsv",
        "source-stdout.txt",
        "source-stderr.txt",
        "rust-stdout.txt",
    ]
    .iter()
    .map(|name| records::artifact(&directory, &directory.join(name)))
    .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"kind":kind,"cases":matrix.len(),
        "exact_original_json":true,"input_none":true,"artifacts":artifacts,
        "scope":"Exact fixed-source field/body query model only; excludes native manager constructor, virtual ABI, connection, input actions and capture."});
    let report_path = directory.join("metadata-validation.json");
    if check_only && report_path.exists() {
        let previous: serde_json::Value = records::read(&report_path)?;
        if previous != report {
            return Err("saved metadata verification differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&report_path, &report)?;
    }
    println!(
        "Manager metadata kind={kind} cases={} original_json_verified=1 passed",
        matrix.len()
    );
    Ok(())
}
fn validate(
    kind: &str,
    matrix: &[(u64, u64)],
    input: &str,
    source: &str,
    rust: &str,
) -> Result<()> {
    let inputs: Vec<_> = input.lines().collect();
    let sources: Vec<_> = source.lines().collect();
    let outputs: Vec<_> = rust.lines().collect();
    if inputs.len() != matrix.len()
        || sources.len() != matrix.len()
        || outputs.len() != matrix.len()
        || matrix.is_empty()
    {
        return Err("metadata case/source/Rust row counts differ".into());
    }
    for (index, &(id, method)) in matrix.iter().enumerate() {
        if inputs[index] != format!("{kind}\t{id}\t{method}") {
            return Err("metadata case matrix differs from its declared inputs".into());
        }
        // Exact compact JSON also rejects duplicate keys, extra fields or
        // changed numeric widths/lexemes that a Value-only comparison can hide.
        let signed = i64::from_ne_bytes(method.to_ne_bytes());
        let expected = match kind {
            "win32" => {
                serde_json::json!({"hwnd":id,"keyboard_method":0,"mouse_method":0,"screencap_method":signed,"type":"win32"})
            }
            "macos" => {
                serde_json::json!({"input_method":0,"screencap_method":signed,"type":"macos","window_id":u32::try_from(id)?})
            }
            _ => return Err("unsupported metadata row kind".into()),
        };
        let row = format!("{index}\t{}", serde_json::to_string(&expected)?);
        if sources[index] != row || outputs[index] != sources[index] {
            return Err(format!("metadata JSON mismatch at {kind} row {index}").into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_changed_widths_duplicate_keys_and_wrong_row_inputs() {
        let matrix = [(u64::MAX, u64::MAX)];
        let input = format!("win32\t{}\t{}\n", u64::MAX, u64::MAX);
        let source = "0\t{\"hwnd\":18446744073709551615,\"keyboard_method\":0,\"mouse_method\":0,\"screencap_method\":-1,\"type\":\"win32\"}\r\n";
        assert!(validate("win32", &matrix, &input, source, source).is_ok());
        assert!(
            validate(
                "win32",
                &matrix,
                &input,
                &source.replace("-1", "18446744073709551615"),
                source
            )
            .is_err()
        );
        assert!(
            validate(
                "win32",
                &matrix,
                &input,
                &source.replace(
                    "\"mouse_method\":0",
                    "\"mouse_method\":0,\"mouse_method\":0"
                ),
                source
            )
            .is_err()
        );
        assert!(
            validate(
                "win32",
                &matrix,
                &input.replace("win32", "macos"),
                source,
                source
            )
            .is_err()
        );
        assert!(validate("win32", &matrix, &input, source, "").is_err());
    }
}
