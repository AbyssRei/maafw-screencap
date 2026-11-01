// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Original interface/query records, independent of native execution.
use crate::manager_metadata::case_matrix;
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

type Event = [u64; 6];
fn event(events: &mut Vec<Event>, kind: u64, thread: u64, a: u64, b: u64) {
    events.push([kind, thread, a, b, 0, 0]);
}
struct Expected<'a> {
    side: &'a str,
    kind: &'a str,
    case: usize,
    phase: &'a str,
    hwnd: u64,
    method: u64,
    valid: i32,
    connected: bool,
}
fn query(line: &str, expected: &Expected<'_>) -> Result<()> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != 12
        || fields[..4]
            != [
                expected.side,
                expected.kind,
                &expected.case.to_string(),
                expected.phase,
            ]
        || fields[4].parse::<u64>()? != expected.hwnd
        || fields[5].parse::<u64>()? != expected.method
        || fields[6].parse::<i32>()? != expected.valid
        || fields[7].parse::<u8>()? != u8::from(expected.connected)
        || fields[8] != "1"
        || fields[9] != "0"
        || fields[10] != format!("{:016X}", expected.hwnd)
    {
        return Err("Original virtual query scalar/UUID/window evidence differs".into());
    }
    let method = i64::from_ne_bytes(expected.method.to_ne_bytes());
    let info = serde_json::json!({"hwnd":expected.hwnd,"keyboard_method":0,"mouse_method":0,"screencap_method":method,"type":"win32"});
    if fields[11] != serde_json::to_string(&info)? {
        return Err("Original virtual query metadata bytes differ".into());
    }
    Ok(())
}
fn validate(
    summary: &serde_json::Value,
    queries: &str,
    held: &str,
    lifetimes: &str,
    native: &str,
) -> Result<()> {
    if summary.as_object().map(|object| object.len()) != Some(16) {
        return Err("Virtual query summary shape differs".into());
    }
    for (key, value) in [
        ("layout_version", 1),
        ("units", 256),
        ("raw_units", 176),
        ("window_units", 80),
        ("snapshots", 500),
        ("virtual_queries", 2000),
        ("events", 2836),
        ("host_created", 128),
        ("host_destroyed", 128),
        ("host_captures", 0),
        ("host_conversions", 0),
        ("host_inactive", 41),
        ("retained_snapshots", 500),
        ("actual_captures", 0),
    ] {
        if summary[key].as_u64() != Some(value) {
            return Err(format!("Virtual query {key} count differs").into());
        }
    }
    let thread = summary["thread"]
        .as_u64()
        .filter(|&n| n > 0 && n <= u64::from(u32::MAX))
        .ok_or("Missing native query thread")?;
    if summary["host_queries"] != serde_json::json!([250, 250, 250, 250]) || held != queries {
        return Err("Virtual query host counts or retained CPU records differ".into());
    }
    let mut lines = queries.lines();
    let mut expected_events = Vec::new();
    let mut expected_lifetimes = String::new();
    let mut count = 0usize;
    for side in ["source", "rust"] {
        for (case, (hwnd, method)) in case_matrix("win32").unwrap().into_iter().enumerate() {
            event(&mut expected_events, 1, thread, hwnd, method);
            let phases: &[&str] = if case == 0 {
                &["cold", "connected", "inactive"]
            } else {
                &["cold"]
            };
            for &phase in phases {
                if phase == "connected" {
                    event(&mut expected_events, 2, thread, 0, 0);
                }
                if phase == "inactive" {
                    event(&mut expected_events, 4, thread, 0, 0);
                }
                let expected = Expected {
                    side,
                    kind: "raw",
                    case,
                    phase,
                    hwnd,
                    method,
                    valid: -1,
                    connected: phase != "cold",
                };
                query(lines.next().ok_or("Missing raw virtual query")?, &expected)?;
                for kind in 8..=11 {
                    event(&mut expected_events, kind, thread, 0, 0);
                }
                count += 1;
            }
            event(&mut expected_events, 5, thread, 0, 0);
            use std::fmt::Write;
            writeln!(expected_lifetimes, "{side}\traw\t{case}\t1")?;
        }
        for case in 0..40 {
            let method = if case % 2 == 0 { 0 } else { 1 << 63 };
            let first = lines.next().ok_or("Missing message-window query")?;
            let hwnd = first
                .split('\t')
                .nth(4)
                .ok_or("Missing query HWND")?
                .parse::<u64>()?;
            if hwnd == 0 {
                return Err("Message-window query requires an actual nonnull HWND".into());
            }
            event(&mut expected_events, 12, thread, hwnd, 0);
            event(&mut expected_events, 1, thread, hwnd, method);
            for phase in ["cold", "connected", "destroyed", "inactive"] {
                match phase {
                    "connected" => event(&mut expected_events, 2, thread, 0, 0),
                    "destroyed" => event(&mut expected_events, 13, thread, hwnd, 0),
                    "inactive" => event(&mut expected_events, 4, thread, 0, 0),
                    _ => {}
                }
                let expected = Expected {
                    side,
                    kind: "window",
                    case,
                    phase,
                    hwnd,
                    method,
                    valid: i32::from(matches!(phase, "cold" | "connected")),
                    connected: phase == "connected" && method == 0,
                };
                query(
                    if phase == "cold" {
                        first
                    } else {
                        lines.next().ok_or("Missing message-window phase")?
                    },
                    &expected,
                )?;
                for kind in 8..=11 {
                    event(&mut expected_events, kind, thread, 0, 0);
                }
                count += 1;
            }
            event(&mut expected_events, 5, thread, 0, 0);
            use std::fmt::Write;
            writeln!(expected_lifetimes, "{side}\twindow\t{case}\t1")?;
        }
    }
    if count != 500 || lines.next().is_some() || lifetimes != expected_lifetimes {
        return Err("Virtual query matrix or module unload lifetime records differ".into());
    }
    let actual = native
        .lines()
        .map(|line| {
            let fields = line
                .split('\t')
                .map(str::parse)
                .collect::<std::result::Result<Vec<u64>, _>>()?;
            let event: Event = fields
                .try_into()
                .map_err(|_| "Invalid virtual query event width")?;
            Ok(event)
        })
        .collect::<Result<Vec<_>>>()?;
    if actual != expected_events || actual.len() != 2836 {
        return Err(
            "Virtual query actual factory/window/virtual-call event sequence differs".into(),
        );
    }
    Ok(())
}
fn files(base: &Path, directory: &Path, result: &mut BTreeSet<String>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("Unexpected virtual query artifact link".into());
        }
        if kind.is_dir() {
            files(base, &entry.path(), result)?;
        } else if kind.is_file() {
            result.insert(
                entry
                    .path()
                    .strip_prefix(base)?
                    .to_str()
                    .ok_or("Invalid query artifact path")?
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-delegate win32-queries <external directory> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let summary: serde_json::Value = records::read(&root.join("summary.json"))?;
    validate(
        &summary,
        &fs::read_to_string(root.join("queries.tsv"))?,
        &fs::read_to_string(root.join("held.tsv"))?,
        &fs::read_to_string(root.join("lifetimes.tsv"))?,
        &fs::read_to_string(root.join("events.tsv"))?,
    )?;
    let names = [
        "summary.json",
        "queries.tsv",
        "held.tsv",
        "lifetimes.tsv",
        "events.tsv",
        "source/MaaWin32ControlUnit.dll",
        "rust/MaaWin32ControlUnit.dll",
    ];
    let mut actual = BTreeSet::new();
    files(&root, &root, &mut actual)?;
    actual.remove("query-validation.json");
    if actual != names.iter().map(|n| n.to_string()).collect() {
        return Err("Virtual query artifact set differs".into());
    }
    for name in &names[5..] {
        if !fs::read(root.join(name))?.starts_with(b"MZ") {
            return Err("Missing native query provider image".into());
        }
    }
    let artifacts = names
        .into_iter()
        .map(|name| records::artifact(&root, &root.join(name)))
        .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"snapshots":500,"virtual_queries":2000,"units":256,"artifacts":artifacts,"scope":"Real original interface factory/query consumption through explicit capture-only original-body/Rust facades; Input=None, no screenshot or general failure/SDK acceptance. Native source/build identities are audited separately."});
    let report_path = root.join("query-validation.json");
    if check_only && report_path.exists() {
        if records::read::<serde_json::Value>(&report_path)? != report {
            return Err("Saved virtual query validation differs".into());
        }
    } else if !check_only {
        records::create_json(&report_path, &report)?;
    }
    println!(
        "Windows real virtual queries snapshots=500 calls=2000 units=256 window_lifetimes=80 retained_after_unload=500 captures=0 verified=1 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_query_spelling_signed_metadata_and_destroyed_window_state() {
        let expected = Expected {
            side: "source",
            kind: "window",
            case: 0,
            phase: "destroyed",
            hwnd: u64::MAX,
            method: 1 << 63,
            valid: 0,
            connected: false,
        };
        let line = "source\twindow\t0\tdestroyed\t18446744073709551615\t9223372036854775808\t0\t0\t1\t0\tFFFFFFFFFFFFFFFF\t{\"hwnd\":18446744073709551615,\"keyboard_method\":0,\"mouse_method\":0,\"screencap_method\":-9223372036854775808,\"type\":\"win32\"}";
        assert!(query(line, &expected).is_ok());
        for broken in [
            line.to_lowercase(),
            line.replace("-9223372036854775808", "9223372036854775808"),
            line.replace("\t0\t0\t1\t0\t", "\t1\t1\t1\t0\t"),
            line.replace(
                "\"mouse_method\":0",
                "\"mouse_method\":0,\"mouse_method\":0",
            ),
        ] {
            assert!(query(&broken, &expected).is_err());
        }
    }
}
