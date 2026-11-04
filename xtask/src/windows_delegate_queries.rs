// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Original interface/query records, independent of native execution.
use crate::manager_metadata::case_matrix;
use crate::{Result, records};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

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
fn returned_calls(
    events: &mut Vec<Event>,
    actual: &[Event],
    thread: u64,
    pointer: &mut Option<u64>,
    expected: &Expected<'_>,
    complete: bool,
) -> Result<()> {
    for kind in 8..=11 {
        if !complete || expected.side == "rust" {
            event(events, kind, thread, 0, 0);
        }
        if complete {
            let unit = match pointer {
                Some(value) => *value,
                None => {
                    let value = actual
                        .get(events.len())
                        .ok_or("Missing observed query return")?[2];
                    if value == 0 || value % 8 != 0 {
                        return Err("Invalid observed query unit pointer".into());
                    }
                    *pointer = Some(value);
                    value
                }
            };
            let method = i64::from_ne_bytes(expected.method.to_ne_bytes());
            let length=serde_json::to_string(&serde_json::json!({"hwnd":expected.hwnd,"keyboard_method":0,"mouse_method":0,"screencap_method":method,"type":"win32"}))?.len();
            let (b, c) = match kind {
                8 => (u64::from(expected.connected), 0),
                9 => (1, 16),
                10 => (0, 0),
                11 => (u64::try_from(length)?, 0),
                _ => unreachable!(),
            };
            events.push([kind + 12, thread, unit, b, c, 0]);
        }
    }
    Ok(())
}

fn original_types(text: &str, thread: u64) -> Result<BTreeMap<(String, usize), u64>> {
    let mut rows = text.lines();
    let mut units = BTreeMap::new();
    let mut size = None;
    for (kind, count) in [("raw", case_matrix("win32").unwrap().len()), ("window", 40)] {
        for case in 0..count {
            let mut pointer = None;
            for stage in 0..2 {
                let fields = rows
                    .next()
                    .ok_or("Missing original query RTTI row")?
                    .split('\t')
                    .collect::<Vec<_>>();
                if fields.len() != 12
                    || fields[..3] != [kind, &case.to_string(), &stage.to_string()]
                {
                    return Err("Original query RTTI order/shape differs".into());
                }
                let numbers = fields[3..]
                    .iter()
                    .map(|text| {
                        let value = text.parse::<u64>()?;
                        if value.to_string() != *text {
                            return Err("Noncanonical original type scalar".into());
                        }
                        Ok(value)
                    })
                    .collect::<Result<Vec<_>>>()?;
                u32::try_from(numbers[8])?;
                if numbers[0] == 0
                    || numbers[0] % 8 != 0
                    || numbers[1..6] != [0x4d465357, 1, 0, 0, 1]
                    || numbers[6] < 8
                    || numbers[7] != 8
                    || numbers[8] != thread
                {
                    return Err("Complete original query class/counters/thread differs".into());
                }
                if let Some(old) = pointer.replace(numbers[0])
                    && old != numbers[0]
                {
                    return Err("Original query unit pointer changed".into());
                }
                if let Some(old) = size.replace(numbers[6])
                    && old != numbers[6]
                {
                    return Err("Original query class size changed".into());
                }
            }
            units.insert((kind.to_owned(), case), pointer.unwrap());
        }
    }
    if rows.next().is_some() {
        return Err("Extra original query RTTI rows".into());
    }
    Ok(units)
}
fn validate(
    summary: &serde_json::Value,
    queries: &str,
    held: &str,
    lifetimes: &str,
    native: &str,
    complete: bool,
    types: Option<&str>,
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
        ("events", if complete { 3498 } else { 2836 }),
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
    let types = if complete {
        original_types(
            types.ok_or("Missing original query class observations")?,
            thread,
        )?
    } else {
        BTreeMap::new()
    };
    let mut lines = queries.lines();
    let mut expected_events = Vec::new();
    let mut expected_lifetimes = String::new();
    let mut count = 0usize;
    for side in ["source", "rust"] {
        for (case, (hwnd, method)) in case_matrix("win32").unwrap().into_iter().enumerate() {
            let mut pointer = if complete && side == "source" {
                Some(types[&("raw".into(), case)])
            } else {
                None
            };
            if !complete || side == "rust" {
                event(&mut expected_events, 1, thread, hwnd, method);
            }
            let phases: &[&str] = if case == 0 {
                &["cold", "connected", "inactive"]
            } else {
                &["cold"]
            };
            for &phase in phases {
                if phase == "connected" && (!complete || side == "rust") {
                    event(&mut expected_events, 2, thread, 0, 0);
                }
                if phase == "inactive" && (!complete || side == "rust") {
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
                returned_calls(
                    &mut expected_events,
                    &actual,
                    thread,
                    &mut pointer,
                    &expected,
                    complete,
                )?;
                count += 1;
            }
            if !complete || side == "rust" {
                event(&mut expected_events, 5, thread, 0, 0);
            }
            use std::fmt::Write;
            writeln!(expected_lifetimes, "{side}\traw\t{case}\t1")?;
        }
        for case in 0..40 {
            let mut pointer = if complete && side == "source" {
                Some(types[&("window".into(), case)])
            } else {
                None
            };
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
            if !complete || side == "rust" {
                event(&mut expected_events, 1, thread, hwnd, method);
            }
            for phase in ["cold", "connected", "destroyed", "inactive"] {
                match phase {
                    "connected" if !complete || side == "rust" => {
                        event(&mut expected_events, 2, thread, 0, 0)
                    }
                    "destroyed" => event(&mut expected_events, 13, thread, hwnd, 0),
                    "inactive" if !complete || side == "rust" => {
                        event(&mut expected_events, 4, thread, 0, 0)
                    }
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
                returned_calls(
                    &mut expected_events,
                    &actual,
                    thread,
                    &mut pointer,
                    &expected,
                    complete,
                )?;
                count += 1;
            }
            if !complete || side == "rust" {
                event(&mut expected_events, 5, thread, 0, 0);
            }
            use std::fmt::Write;
            writeln!(expected_lifetimes, "{side}\twindow\t{case}\t1")?;
        }
    }
    if count != 500 || lines.next().is_some() || lifetimes != expected_lifetimes {
        return Err("Virtual query matrix or module unload lifetime records differ".into());
    }
    if actual != expected_events || actual.len() != if complete { 3498 } else { 2836 } {
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
    run_variant(args, false)
}
pub fn run_whole(args: &[String]) -> Result<()> {
    run_variant(args, true)
}
fn run_variant(args: &[String], complete: bool) -> Result<()> {
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
    let types = if complete {
        Some(fs::read_to_string(root.join("original-types.tsv"))?)
    } else {
        None
    };
    validate(
        &summary,
        &fs::read_to_string(root.join("queries.tsv"))?,
        &fs::read_to_string(root.join("held.tsv"))?,
        &fs::read_to_string(root.join("lifetimes.tsv"))?,
        &fs::read_to_string(root.join("events.tsv"))?,
        complete,
        types.as_deref(),
    )?;
    let mut names = vec![
        "summary.json",
        "queries.tsv",
        "held.tsv",
        "lifetimes.tsv",
        "events.tsv",
        "source/MaaWin32ControlUnit.dll",
        "rust/MaaWin32ControlUnit.dll",
    ];
    if complete {
        names.push("original-types.tsv");
    }
    let mut actual = BTreeSet::new();
    files(&root, &root, &mut actual)?;
    let report_name = if complete {
        "whole-query-validation.json"
    } else {
        "query-validation.json"
    };
    actual.remove(report_name);
    if actual != names.iter().map(|n| n.to_string()).collect() {
        return Err("Virtual query artifact set differs".into());
    }
    for name in [
        "source/MaaWin32ControlUnit.dll",
        "rust/MaaWin32ControlUnit.dll",
    ] {
        if !fs::read(root.join(name))?.starts_with(b"MZ") {
            return Err("Missing native query provider image".into());
        }
    }
    let artifacts = names
        .into_iter()
        .map(|name| records::artifact(&root, &root.join(name)))
        .collect::<Result<Vec<_>>>()?;
    let mut report = serde_json::json!({"layout_version":1,"snapshots":500,"virtual_queries":2000,"units":256,"artifacts":artifacts,"scope":"Real original interface factory/query consumption through explicit capture-only original-body/Rust facades; Input=None, no screenshot or general failure/SDK acceptance. Native source/build identities are audited separately."});
    if complete {
        report["original_class_api"] = true.into();
        report["original_class_rtti_observations"] = 256.into();
        report["observed_query_returns"] = 2000.into();
        report["scope"]="Actual complete unchanged original F Win32 class/header/API and empty BackgroundManagedKeyInput versus same Rust manager/factory, Input=None. Four actual virtual query returns per500CPU snapshots have independent bridge events, with original RTTI/pointer binding and zero excluded-input/application counters. All units/modules/message-HWNDs end before held CPU snapshots move to a joined thread. No screenshot, native input/application action, compatible macro, arbitrary failures or full P1 acceptance.".into();
    }
    let report_path = root.join(report_name);
    if check_only && report_path.exists() {
        if records::read::<serde_json::Value>(&report_path)? != report {
            return Err("Saved virtual query validation differs".into());
        }
    } else if !check_only {
        records::create_json(&report_path, &report)?;
    }
    if complete {
        println!(
            "Windows whole original queries snapshots=500 calls=2000 units=256 original_rtti=256 observed_returns=2000 window_lifetimes=80 retained_after_unload=500 captures=0 verified=1 passed"
        );
    } else {
        println!(
            "Windows real virtual queries snapshots=500 calls=2000 units=256 window_lifetimes=80 retained_after_unload=500 captures=0 verified=1 passed"
        );
    }
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
