// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Verify observations without relabeling failed screenshot calls as captures.
use crate::macos_display_geometry as geometry;
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

fn fields(text: &str) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("malformed same-binary field")?;
        if key.is_empty()
            || value.is_empty()
            || value.contains(['\r', '\n', '\t'])
            || map.insert(key.into(), value.into()).is_some()
        {
            return Err("duplicate/empty same-binary field".into());
        }
    }
    Ok(map)
}
fn number(map: &BTreeMap<String, String>, key: &str) -> Result<u64> {
    let text = map.get(key).ok_or("missing same-binary scalar")?;
    let value = text.parse::<u64>()?;
    if value.to_string() != *text {
        return Err("noncanonical same-binary scalar".into());
    }
    Ok(value)
}
fn expect(map: &BTreeMap<String, String>, key: &str, value: u64) -> Result<()> {
    if number(map, key)? != value {
        return Err(format!("same-binary {key} differs").into());
    }
    Ok(())
}
fn rows(text: &str, columns: usize) -> Result<Vec<Vec<u64>>> {
    text.lines()
        .map(|line| {
            let row = line
                .split('\t')
                .map(|v| {
                    let n = v.parse::<u64>()?;
                    if n.to_string() != v {
                        return Err("noncanonical same-binary row".into());
                    }
                    Ok(n)
                })
                .collect::<Result<Vec<_>>>()?;
            if row.len() != columns {
                return Err("same-binary column count differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |h, v| {
        (h ^ u64::from(*v)).wrapping_mul(0x100000001b3)
    })
}
fn executable(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 32
        || u32::from_le_bytes(bytes[0..4].try_into()?) != 0xfeedfacf
        || u32::from_le_bytes(bytes[4..8].try_into()?) != 0x0100000c
        || u32::from_le_bytes(bytes[12..16].try_into()?) != 2
    {
        return Err("same-binary fixture is not an arm64 Mach-O executable".into());
    }
    Ok(())
}
fn identity(
    map: &BTreeMap<String, String>,
    role: &str,
    mode: u64,
    length: u64,
    hash: u64,
) -> Result<()> {
    if map.len() != if role == "consumer" { 7 } else { 10 }
        || map.get("role").map(String::as_str) != Some(role)
    {
        return Err("same-binary identity role/fields differ".into());
    }
    for (key, value) in [
        ("schema", 1),
        ("mode", mode),
        ("program_length", length),
        ("program_fnv64", hash),
    ] {
        expect(map, key, value)?;
    }
    let pid = number(map, "pid")?;
    if pid == 0 || u32::try_from(pid).is_err() {
        return Err("invalid same-binary PID".into());
    }
    let path = map
        .get("program_path")
        .ok_or("missing same-binary executable path")?;
    if !path.starts_with('/') || path.split('/').any(|p| p == "..") {
        return Err("invalid recorded native executable path".into());
    }
    if role == "producer" {
        expect(map, "main_thread", 1)?;
        expect(map, "module_retained_until_process_exit", 1)?;
        let module = map
            .get("module_path")
            .ok_or("missing prepared scene image path")?;
        if !module.starts_with('/') || !module.ends_with(".dylib") {
            return Err("invalid native scene image path".into());
        }
    }
    Ok(())
}
fn selection(text: &str, g: &geometry::Geometry, required: bool) -> Result<()> {
    let row = rows(text, 5)?;
    if row.len() != 1
        || (row[0] != [0, 0, 0, 0, 0]
            && row[0]
                != [
                    1,
                    u64::from(g.count),
                    u64::from(g.display),
                    u64::try_from(g.width)?,
                    u64::try_from(g.height)?,
                ])
        || (required && row[0][0] != 1)
    {
        return Err("actual same-binary display selection differs".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace macos-same-binary <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let binary = fs::read(root.join("same-binary-test-executable"))?;
    executable(&binary)?;
    let length = u64::try_from(binary.len())?;
    let hash = fnv(&binary);
    let mut calls = Vec::new();
    let mut arms = Vec::new();
    let mut common_path = None;
    for mode in 0..=1 {
        let arm = root.join(format!("mode-{mode}"));
        let read = |name: &str| -> Result<Vec<u8>> { Ok(fs::read(arm.join(name))?) };
        let text = |name: &str| -> Result<String> { Ok(String::from_utf8(read(name)?)?) };
        let consumer = fields(&text("consumer-host.txt")?)?;
        let producer = fields(&text("scene/same-binary-host.txt")?)?;
        identity(&consumer, "consumer", mode, length, hash)?;
        identity(&producer, "producer", mode, length, hash)?;
        if consumer["program_path"] != producer["program_path"]
            || consumer["pid"] == producer["pid"]
            || common_path
                .as_ref()
                .is_some_and(|p| *p != consumer["program_path"])
        {
            return Err("same-binary producer/consumer executable relation differs".into());
        }
        common_path = Some(consumer["program_path"].clone());
        let child = fields(&text("scene-child.txt")?)?;
        if child.len() != 3 {
            return Err("same-binary child record fields differ".into());
        }
        expect(&child, "pid", number(&producer, "pid")?)?;
        expect(&child, "mode", mode)?;
        expect(&child, "same_executable_requested", 1)?;
        let done = fields(&text("scene/same-binary-result.txt")?)?;
        if done.len() != 6 {
            return Err("same-binary native completion fields differ".into());
        }
        for (key, value) in [
            ("schema", 1),
            ("mode", mode),
            ("pid", number(&producer, "pid")?),
            ("exit", 0),
            ("main_thread", 1),
            ("module_retained_until_process_exit", 1),
        ] {
            expect(&done, key, value)?;
        }
        if read("scene/stopped.txt")? != b"window_closed=1\n"
            || arm.join("scene/forced-stop.txt").exists()
            || !text("scene-stdout.txt")?.contains("owned macOS scene stop=1 main_thread=1 passed")
        {
            return Err("same-binary owned scene close/exit evidence differs".into());
        }
        let expected = text("consumer-expected-geometry.tsv")?;
        if expected != text("scene/display-geometry.tsv")? {
            return Err("same-binary expected geometry was changed".into());
        }
        let g = geometry::parse(&expected)?;
        if g.count != 1 || u64::from(g.pid) != number(&producer, "pid")? {
            return Err("same-binary native window owner/single display differs".into());
        }
        if mode == 1 {
            let cg = fields(&text("scene/cg-scene-state.txt")?)?;
            if cg.len() != 8
                || cg.get("geometry_origin").map(String::as_str) != Some("core_graphics")
            {
                return Err("same-binary CG producer metadata origin differs".into());
            }
            for (key, value) in [
                ("geometry_queries", 2),
                ("display_count", 1),
                ("display_id", u64::from(g.display)),
                ("scene_pid", u64::from(g.pid)),
                ("window_id", u64::from(g.window)),
                ("main_thread", 1),
                ("window_closed", 1),
            ] {
                expect(&cg, key, value)?;
            }
        }
        let summary = fields(&text("summary.txt")?)?;
        if summary.len() != 12 {
            return Err("same-binary summary fields differ".into());
        }
        for (key, value) in [
            ("schema", 1),
            ("mode", mode),
            ("invocations", 3),
            ("scene_closed", 1),
            ("scene_child_exit", 0),
            ("same_binary", 1),
            ("faults", 0),
            ("no_capture_retry", 1),
            ("original_oracle_replaced", 0),
            ("full_P1_acceptance", 0),
        ] {
            expect(&summary, key, value)?;
        }
        let table = text("calls.tsv")?;
        let mut lines = table.lines();
        if lines.next() != Some("call\tresult\tknown_patch\telapsed_ns") {
            return Err("same-binary calls header differs".into());
        }
        let table = rows(&lines.collect::<Vec<_>>().join("\n"), 4)?;
        if table.len() != 3 {
            return Err("same-binary requires three actual call observations".into());
        }
        let (mut usable, mut known) = (0, 0);
        for (call, row) in table.iter().enumerate() {
            if row[0] != u64::try_from(call)? || row[1] > 3 || row[2] > 1 || row[3] == 0 {
                return Err("same-binary call index/result/elapsed differs".into());
            }
            let state = fields(&text(&format!("call-{call}-status.txt"))?)?;
            if state.len() != 11 {
                return Err("same-binary return-state fields differ".into());
            }
            for (key, value) in [
                ("schema", 1),
                ("result", row[1]),
                ("fault", 0),
                ("initial_inputs_released", 1),
            ] {
                expect(&state, key, value)?;
            }
            for key in ["timed_out", "content_calls", "sample_calls", "completed"] {
                if number(&state, key)? > 1 {
                    return Err("duplicate same-binary callback/invalid flag".into());
                }
            }
            if number(&state, "source_failure")? > 4 {
                return Err("unknown source failure observation".into());
            }
            for key in ["native_error", "wait_result"] {
                let _ = state
                    .get(key)
                    .ok_or("missing signed native scalar")?
                    .parse::<i64>()?;
            }
            selection(
                &text(&format!("call-{call}-selection.tsv"))?,
                &g,
                matches!(row[1], 1 | 3),
            )?;
            if matches!(row[1], 1 | 3) {
                for (key, value) in [
                    ("timed_out", 0),
                    ("content_calls", 1),
                    ("sample_calls", 1),
                    ("completed", 1),
                ] {
                    expect(&state, key, value)?;
                }
                if state["wait_result"] != "0" {
                    return Err("Some result contradicts actual source wait".into());
                }
            }
            let mut correct = false;
            if row[1] == 1 {
                let layout = rows(&text(&format!("call-{call}-layout.tsv"))?, 3)?;
                let raw = read(&format!("call-{call}.raw"))?;
                let bgr = read(&format!("call-{call}.bgr"))?;
                if layout.len() != 1
                    || layout[0][0] != u64::try_from(g.width)?
                    || layout[0][1] != u64::try_from(g.height)?
                    || layout[0][2]
                        < u64::try_from(g.width.checked_mul(4).ok_or("raw stride overflow")?)?
                    || raw.len()
                        != g.width
                            .checked_mul(g.height)
                            .and_then(|n| n.checked_mul(4))
                            .ok_or("raw span overflow")?
                    || bgr.len()
                        != g.width
                            .checked_mul(g.height)
                            .and_then(|n| n.checked_mul(3))
                            .ok_or("BGR span overflow")?
                    || raw != read(&format!("held-{call}.raw"))?
                    || bgr != read(&format!("held-{call}.bgr"))?
                {
                    return Err("same-binary current/held CPU span or identity differs".into());
                }
                correct = g.pixels(&raw, 4).is_ok() && g.pixels(&bgr, 3).is_ok();
                usable += 1;
                known += u64::from(correct);
            } else {
                for suffix in [
                    format!("call-{call}.raw"),
                    format!("call-{call}.bgr"),
                    format!("call-{call}-layout.tsv"),
                    format!("held-{call}.raw"),
                    format!("held-{call}.bgr"),
                ] {
                    if arm.join(suffix).exists() {
                        return Err("failed/empty call has a substituted CPU frame".into());
                    }
                }
            }
            if row[2] != u64::from(correct) {
                return Err("same-binary known-patch flag differs from actual pixels".into());
            }
            let late = fields(&text(&format!("call-{call}-late.txt"))?)?;
            if late.len() != 7 {
                return Err("same-binary late-state fields differ".into());
            }
            expect(&late, "fault", 0)?;
            for key in [
                "request_released",
                "semaphore_block_owner_released",
                "fixture_reclaimed",
                "completed",
                "late_completed",
                "raw_samples",
            ] {
                if number(&late, key)? > 1 {
                    return Err("invalid late same-binary owner/sample observation".into());
                }
            }
            if number(&late, "fixture_reclaimed")? == 1 {
                expect(&late, "request_released", 1)?;
                expect(&late, "semaphore_block_owner_released", 1)?;
            }
            selection(
                &text(&format!("call-{call}-late-selection.tsv"))?,
                &g,
                false,
            )?;
            calls.push(serde_json::json!({"mode":mode,"call":call,"caller_result":row[1],"known_patch":correct,"elapsed_ns":row[3],"return_state":state,"late_state":late,"late_used_as_caller_result":false}));
        }
        expect(&summary, "usable_images", usable)?;
        expect(&summary, "known_patch_images", known)?;
        if mode == 1 && (usable, known) != (3, 3) {
            return Err("same-binary CG metadata positive control failed".into());
        }
        arms.push(serde_json::json!({"mode":mode,"consumer":consumer,"producer":producer,"usable_images":usable,"known_patch_images":known,"scene_closed":true,"scene_module_explicitly_unloaded":false}));
    }
    fn inventory(root: &Path, folder: &Path, out: &mut Vec<records::Artifact>) -> Result<()> {
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if entry.path() == root.join("same-binary-validation.json") {
                continue;
            }
            if kind.is_file() {
                out.push(records::artifact(root, &entry.path())?);
            } else if kind.is_dir() {
                inventory(root, &entry.path(), out)?;
            } else {
                return Err("unexpected same-binary link/native evidence entry".into());
            }
        }
        Ok(())
    }
    inventory(&root, &root, &mut artifacts)?;
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"arms":arms,"call_observations":calls,"actual_invocations":6,"inputs":artifacts,
        "scope":"Fresh Rust consumer/scene pairs with the same actual executable path, archived arm64 executable bytes and observed child/window identity. CG metadata is a strict three-image positive control; SC metadata results, including None/error/empty, are observations. Original five-second waits unchanged; no capture retry, source oracle substitution, late-frame recovery, callback-quiescence claim, border/broker cause or full P1 acceptance. The child retains its separate scene module until process exit."});
    let output = root.join("same-binary-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved same-binary observation report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS same_binary arms=2 invocations=6 CG_positive_images=3 source_oracle_replaced=0 late_used_as_current=0 diagnostic_only=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_noncanonical_and_foreign_executable_records() {
        assert!(fields("pid=1\npid=2\n").is_err());
        assert!(fields("pid=\n").is_err());
        assert!(rows("00\t1\n", 2).is_err());
        assert!(rows("0\t1\t2\n", 2).is_err());
        assert!(executable(&[0; 32]).is_err());
        let mut header = [0_u8; 32];
        header[0..4].copy_from_slice(&0xfeedfacf_u32.to_le_bytes());
        header[4..8].copy_from_slice(&0x0100000c_u32.to_le_bytes());
        header[12..16].copy_from_slice(&2_u32.to_le_bytes());
        executable(&header).unwrap();
        header[4..8].copy_from_slice(&0x01000007_u32.to_le_bytes());
        assert!(executable(&header).is_err());
    }
}
