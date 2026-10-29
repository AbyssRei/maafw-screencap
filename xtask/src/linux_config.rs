// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit original API/configuration characterization, without capture calls.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path, process::Command};
#[path = "../../tests/upstream-parity/linux-manager/config-cases.rs"]
mod corpus;

#[derive(Serialize)]
struct Observation {
    case: String,
    expected_created: bool,
    input_sha256: Option<String>,
    stdout_sha256: String,
    created: bool,
    normalized_roundtrip: bool,
}

pub fn run(args: &[String]) -> Result<()> {
    let [kind, executable, path] = args else {
        return Err(
            "expected upstream-config linux <original executable> <new external directory>".into(),
        );
    };
    if kind != "linux" {
        return Err("only the Linux original configuration oracle is implemented".into());
    }
    let executable = fs::canonicalize(executable)?;
    let executable_sha256 = records::hash(&executable)?;
    if Path::new(path).exists() {
        return Err("configuration observation directory must be new".into());
    }
    let directory = records::external_root(Path::new(path))?;
    let mut observations = Vec::new();
    for case in corpus::cases() {
        let case_dir = directory.join(&case.name);
        fs::create_dir(&case_dir)?;
        let mut command = Command::new(&executable);
        if let Some(input) = &case.input {
            let path = case_dir.join("input.bin");
            fs::write(&path, input)?;
            command.arg("--file").arg(path);
        } else {
            command.arg("--null");
        }
        let output = command.output()?;
        fs::write(case_dir.join("stdout.txt"), &output.stdout)?;
        fs::write(case_dir.join("stderr.txt"), &output.stderr)?;
        if !output.status.success() {
            return Err(format!("{}: source process failed ({})", case.name, output.status).into());
        }
        let fields = fields(&output.stdout)?;
        let created = match fields.get("created") {
            Some(&"1") if fields.len() == 8 => true,
            Some(&"0") if fields.len() == 1 => false,
            _ => return Err("invalid original API creation result/field set".into()),
        };
        if created != case.accepted {
            return Err(format!(
                "{}: actual source creation differs from corpus expectation",
                case.name
            )
            .into());
        }
        let mut normalized_roundtrip = false;
        if created {
            for (key, value) in [
                ("connected", "0"),
                ("features", "0"),
                ("no_native_connection", "1"),
                ("failed_output_preserved", "1"),
                ("destroyed", "1"),
            ] {
                if fields.get(key) != Some(&value) {
                    return Err(
                        format!("{}: original API invariant {key} failed", case.name).into(),
                    );
                }
            }
            let info = unhex(fields.get("info_hex").ok_or("missing original metadata")?)?;
            unhex(fields.get("uuid_hex").ok_or("missing original UUID")?)?;
            let normalized = case_dir.join("source-info.bin");
            fs::write(&normalized, &info)?;
            let again = Command::new(&executable)
                .arg("--file")
                .arg(&normalized)
                .output()?;
            fs::write(case_dir.join("roundtrip-stdout.txt"), &again.stdout)?;
            fs::write(case_dir.join("roundtrip-stderr.txt"), &again.stderr)?;
            if !again.status.success() || again.stdout != output.stdout {
                return Err(
                    format!("{}: original info/config roundtrip differed", case.name).into(),
                );
            }
            normalized_roundtrip = true;
        }
        observations.push(Observation {
            case: case.name,
            expected_created: case.accepted,
            created,
            normalized_roundtrip,
            input_sha256: case
                .input
                .as_ref()
                .map(|_| records::hash(&case_dir.join("input.bin")))
                .transpose()?,
            stdout_sha256: records::hash(&case_dir.join("stdout.txt"))?,
        });
    }
    let accepted = observations.iter().filter(|case| case.created).count();
    if records::hash(&executable)? != executable_sha256 {
        return Err("original configuration executable changed during observation".into());
    }
    records::create_json(
        &directory.join("observations.json"),
        &serde_json::json!({
            "layout_version":1, "phase":"P1", "phase_status":"incomplete", "cases":observations,
        "executable_sha256":executable_sha256,
            "scope":"Original Config.h/meojson/API and queried manager bodies with real C++ handles/vtables/Mat; no native/input connection. Byte metadata/UUID and normalization roundtrip observations, not Rust config or backend acceptance."
        }),
    )?;
    println!(
        "Original Linux configuration cases={} accepted={accepted} rejected={} normalized_roundtrips={accepted} no_native_connection=1 passed",
        observations.len(),
        observations.len() - accepted
    );
    Ok(())
}

fn fields(bytes: &[u8]) -> Result<BTreeMap<&str, &str>> {
    let mut fields = BTreeMap::new();
    for line in std::str::from_utf8(bytes)?.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("invalid original API output line")?;
        if fields.insert(key, value).is_some() {
            return Err("duplicate original API observation".into());
        }
    }
    if fields.contains_key("caller_caught_exception_hex") {
        return Err("original API threw into its C++ caller; inspect retained output".into());
    }
    Ok(fields)
}
fn unhex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid source hex bytes".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}
