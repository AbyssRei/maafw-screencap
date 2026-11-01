// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Consumed-object cleanup-error evidence; never an unknown-consumption proof.
use crate::{Result, records};
use std::{fs, path::Path};

const SCOPE: &str = "consumed_objects=3\nreported_cleanup_errors=2\nfirst_error=7101\nprovider_retained_after_bridge_owner=1\nactual_captures=0\nunknown_consumption_tested=0\n";

fn validate(cases: &str, events: &str, scope: &str) -> Result<()> {
    if cases != "0\t0\t1\t0\t0\n1\t7101\t1\t7101\t1\n2\t7102\t1\t7101\t1\n" || scope != SCOPE {
        return Err("Cleanup consumption/module/first-error observations differ".into());
    }
    let mut thread = None;
    let rows: Vec<_> = events.lines().collect();
    if rows.len() != 6 || !events.ends_with('\n') {
        return Err("Expected three real original create/destroy pairs".into());
    }
    for (index, row) in rows.iter().enumerate() {
        let fields = row
            .split('\t')
            .map(|field| -> Result<u64> {
                let value: u64 = field.parse()?;
                if value.to_string() != field {
                    return Err("Noncanonical cleanup event integer".into());
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>>>()?;
        if fields.len() != 6
            || fields[0] != if index % 2 == 0 { 1 } else { 5 }
            || fields[1] == 0
            || fields[1] > u64::from(u32::MAX)
            || fields[2..] != [0; 4]
        {
            return Err("Unexpected native cleanup event/selector/thread".into());
        }
        if thread
            .replace(fields[1])
            .is_some_and(|old| old != fields[1])
        {
            return Err("Cleanup create/destroy changed thread".into());
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected win32-destroy <external records> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    validate(
        &fs::read_to_string(root.join("cases.tsv"))?,
        &fs::read_to_string(root.join("events.tsv"))?,
        &fs::read_to_string(root.join("scope.txt"))?,
    )?;
    let mut artifacts = Vec::new();
    for name in [
        "cases.tsv",
        "events.tsv",
        "scope.txt",
        "MaaWin32ControlUnit.dll",
    ] {
        artifacts.push(records::artifact(&root, &root.join(name))?);
    }
    let report = serde_json::json!({"layout_version":1,"consumed_units":3,"reported_errors":2,"actual_captures":0,"artifacts":artifacts,
        "scope":"Three real original factory/virtual-destroy objects without connect/capture. One successful cleanup unloads; two post-consumption injected return errors retain module references, preserving the first error through borrowed bridge-owner shutdown. Original factory/destructor bodies are unchanged. No actual native destroy exception, unknown-consumption/quiescence, callback drain, recovery/reclamation of leaked modules or complete P1 acceptance is established."});
    let output = root.join("delegate-destroy-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Cleanup evidence differs from its saved validation".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Real delegate cleanup records passed: consumed_units=3 reported_errors=2 actual_captures=0"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unconsumed_objects_unloaded_modules_changed_errors_and_native_event_gaps() {
        let cases = "0\t0\t1\t0\t0\n1\t7101\t1\t7101\t1\n2\t7102\t1\t7101\t1\n";
        let events = "1\t7\t0\t0\t0\t0\n5\t7\t0\t0\t0\t0\n".repeat(3);
        validate(cases, &events, SCOPE).unwrap();
        for bad in [
            cases.replace("7101\t1\t7101", "7101\t0\t7101"),
            cases.replace("7101\t1\n", "7101\t0\n"),
            cases.replace("7102\t1\t7101", "7102\t1\t7102"),
            String::new(),
        ] {
            assert!(validate(&bad, &events, SCOPE).is_err());
        }
        for bad in [
            events.replacen("5\t7", "1\t7", 1),
            events.replacen("5\t7", "5\t8", 1),
            events.replace("\t7\t", "\t0\t"),
            events.replacen("\t0\t0\t0\t0", "\t1\t0\t0\t0", 1),
            events.replacen("\t7\t", "\t07\t", 1),
            events.lines().take(5).collect::<Vec<_>>().join("\n"),
            events.clone() + "5\t7\t0\t0\t0\t0\n",
        ] {
            assert!(validate(cases, &bad, SCOPE).is_err());
        }
        assert!(validate(cases, &events, &SCOPE.replace("tested=0", "tested=1")).is_err());
    }
}
