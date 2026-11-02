// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Verify observations without relabeling failed screenshot calls as captures.
use crate::macos_display_geometry as geometry;
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn fields(text: &str) -> Result<BTreeMap<String, String>> {
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
pub(super) fn number(map: &BTreeMap<String, String>, key: &str) -> Result<u64> {
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
pub(super) fn rows(text: &str, columns: usize) -> Result<Vec<Vec<u64>>> {
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
fn original_call(
    arm: &Path,
    call: usize,
    caller_result: u64,
    g: &geometry::Geometry,
    consumer: &BTreeMap<String, String>,
    finish: &BTreeMap<String, String>,
) -> Result<serde_json::Value> {
    let row = rows(
        &fs::read_to_string(arm.join(format!("original-call-{call}.tsv")))?,
        5,
    )?;
    if row.len() != 1
        || row[0][0] != u64::try_from(call)?
        || !matches!(row[0][1], 0 | 1 | 3)
        || row[0][1] != caller_result
        || row[0][2] == 0
        || row[0][3] == 0
        || row[0][3] != number(finish, "caller_thread")?
        || row[0][4] != number(consumer, "pid")?
    {
        return Err("same-process original/Rust return or native caller identity differs".into());
    }
    let usable = row[0][1] == 1;
    let mut known = false;
    let names = [
        format!("original-call-{call}.bgr"),
        format!("original-call-{call}-layout.tsv"),
        format!("original-held-{call}.bgr"),
    ];
    if usable {
        let bgr = fs::read(arm.join(&names[0]))?;
        let layout = rows(&fs::read_to_string(arm.join(&names[1]))?, 5)?;
        if layout.len() != 1
            || layout[0][0] != u64::try_from(g.width)?
            || layout[0][1] != u64::try_from(g.height)?
            || layout[0][2]
                < u64::try_from(g.width.checked_mul(3).ok_or("original stride overflow")?)?
            || layout[0][3] == 0
            || layout[0][3] != layout[0][4]
            || bgr.len()
                != g.width
                    .checked_mul(g.height)
                    .and_then(|n| n.checked_mul(3))
                    .ok_or("original span overflow")?
            || bgr != fs::read(arm.join(&names[2]))?
        {
            return Err("same-process original CPU layout/span/held bytes differ".into());
        }
        known = g.pixels(&bgr, 3).is_ok();
    } else if names.iter().any(|name| arm.join(name).exists()) {
        return Err("original None/empty result has a substituted CPU image".into());
    }
    Ok(
        serde_json::json!({"call":call,"caller_result":row[0][1],"elapsed_ns":row[0][2],
        "native_caller_thread":row[0][3],"pid":row[0][4],"usable_image":usable,"known_patch":known,
        "caller_result_matches_rust":true,"full_display_pixels_paired":false}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    run_kind(args, false, false, false)
}
pub fn run_full(args: &[String]) -> Result<()> {
    run_kind(args, true, false, false)
}
pub fn run_full_witness(args: &[String]) -> Result<()> {
    run_kind(args, true, true, false)
}
pub fn run_maximum_level(args: &[String]) -> Result<()> {
    run_kind(args, true, true, true)
}
fn run_kind(args: &[String], full: bool, witness: bool, maximum: bool) -> Result<()> {
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
    let mut common_schema = None;
    let mut originals = Vec::new();
    let mut order_records = Vec::new();
    let mut witness_records = Vec::new();
    let mut level_records = Vec::new();
    for mode in 0..=1 {
        let arm = root.join(format!("mode-{mode}"));
        if !maximum
            && (arm.join("maximum-level-enabled.txt").exists()
                || arm.join("scene/maximum-level-control.txt").exists())
        {
            return Err("maximum-level fixture requires its separate diagnostic validator".into());
        }
        if full && !witness && arm.join("witness-enabled.txt").exists() {
            return Err("interleaved witnesses require their separate diagnostic validator".into());
        }
        if full && arm.join("missing-environment.txt").exists() {
            return Err("missing native environment cannot accept full-display captures".into());
        }
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
        let g = if full {
            geometry::parse_full(&expected)
        } else {
            geometry::parse(&expected)
        }?;
        if g.count != 1 || u64::from(g.pid) != number(&producer, "pid")? {
            return Err("same-binary native window owner/single display differs".into());
        }
        if mode == 1 || full {
            let cg = fields(&text("scene/cg-scene-state.txt")?)?;
            if cg.len() != if full { 12 } else { 8 }
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
            if full {
                for (key, value) in [
                    ("full_display_rgb", 1),
                    ("pixel_inset", 0),
                    ("backing_scale", 1),
                ] {
                    expect(&cg, key, value)?;
                }
                if number(&cg, "window_level")? == 0
                    || number(&cg, "window_level")? > i32::MAX as u64
                {
                    return Err("full-display fixture level differs".into());
                }
                if maximum {
                    level_records.push(serde_json::json!({"mode":mode,"control":
                        crate::macos_maximum_level::validate(&arm, &g, number(&cg, "window_level")?)?}));
                }
                if !text("scene-stdout.txt")?.contains("owned macOS full_display_rgb=1 inset=0 single_display_1to1=1 window_closed=1 passed") {
                    return Err("full-display producer did not finish its declared scene".into());
                }
                let ready: serde_json::Value = serde_json::from_slice(&read("scene/ready.json")?)?;
                if ready.as_object().is_none_or(|v| v.len() != 5)
                    || ready["window_id"] != g.window
                    || ready["owner_pid"] != g.pid
                    || ready["width"] != g.width
                    || ready["height"] != g.height
                    || ready["full_display_rgb"] != 1
                {
                    return Err("full-display ready identity differs".into());
                }
            }
        }
        let summary = fields(&text("summary.txt")?)?;
        let schema = number(&summary, "schema")?;
        if (if full {
            schema != 3
        } else {
            !matches!(schema, 1 | 2)
        }) || summary.len()
            != if schema == 1 {
                12
            } else if full {
                19
            } else {
                18
            }
            || common_schema.is_some_and(|s| s != schema)
        {
            return Err("same-binary summary fields differ".into());
        }
        common_schema = Some(schema);
        for (key, value) in [
            ("schema", schema),
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
        let original_finish = if schema >= 2 {
            for (key, value) in [
                ("original_invocations", 3),
                (
                    "original_first",
                    if full { u64::from(mode == 0) } else { 1 },
                ),
                ("original_owner_closed", 1),
                ("paired_caller_results", 1),
            ] {
                expect(&summary, key, value)?;
            }
            let module = fields(&text("original-module.txt")?)?;
            if module.len() != 3 || module.get("module_path") != producer.get("module_path") {
                return Err("same-process original loaded module identity differs".into());
            }
            expect(&module, "schema", 1)?;
            expect(&module, "module_retained_until_process_exit", 1)?;
            if read("original-owner-closed.txt")?
                != b"context_owners=0\nsource_callbacks_drained=0\n"
            {
                return Err("same-process original context closure evidence differs".into());
            }
            let finish = fields(&text("original-finish.txt")?)?;
            if finish.len() != 8 || number(&finish, "caller_thread")? == 0 {
                return Err("same-process original finish fields differ".into());
            }
            for (key, value) in [
                ("schema", 1),
                ("calls", 3),
                ("pid", number(&consumer, "pid")?),
                ("source_object_destroyed", 1),
                ("callback_drain_proved", 0),
                ("module_retained_until_process_exit", 1),
            ] {
                expect(&finish, key, value)?;
            }
            Some(finish)
        } else {
            if fs::read_dir(&arm)?
                .any(|e| e.is_ok_and(|e| e.file_name().to_string_lossy().starts_with("original-")))
            {
                return Err("legacy same-binary schema has new original caller evidence".into());
            }
            None
        };
        let sequence = if full {
            expect(&summary, "full_display", 1)?;
            let data = text("sequence.tsv")?;
            let mut lines = data.lines();
            if lines.next() != Some("rank\timplementation\tcall\tbegin_ns\tend_ns") {
                return Err("full-display sequence header differs".into());
            }
            let values = rows(&lines.collect::<Vec<_>>().join("\n"), 5)?;
            if values.len() != 6 {
                return Err("full-display sequence requires six actual calls".into());
            }
            for (rank, row) in values.iter().enumerate() {
                let side = if rank < 3 { mode } else { 1 - mode };
                if row[0] != rank as u64
                    || row[1] != side
                    || row[2] != (rank % 3) as u64
                    || row[3] == 0
                    || row[4] <= row[3]
                    || rank > 0 && row[3] < values[rank - 1][4]
                {
                    return Err(
                        "full-display source/Rust order or synchronous call interval differs"
                            .into(),
                    );
                }
            }
            order_records.push(serde_json::json!({"mode":mode,"calls":values}));
            if witness {
                let cg = fields(&text("scene/cg-scene-state.txt")?)?;
                witness_records.push(crate::macos_full_display_witness::validate(
                    &arm,
                    &g,
                    mode,
                    &values,
                    number(&cg, "window_level")?,
                )?);
            }
            Some(values)
        } else {
            None
        };
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
        let (mut original_usable, mut original_known) = (0_u64, 0_u64);
        for (call, row) in table.iter().enumerate() {
            if row[0] != u64::try_from(call)? || row[1] > 3 || row[2] > 1 || row[3] == 0 {
                return Err("same-binary call index/result/elapsed differs".into());
            }
            let state = fields(&text(&format!("call-{call}-status.txt"))?)?;
            if state.len() != 11 {
                return Err("same-binary return-state fields differ".into());
            }
            for (key, value) in [("schema", 1), ("result", row[1]), ("fault", 0)] {
                expect(&state, key, value)?;
            }
            if full {
                expect(&state, "source_failure", 0)?;
                if state["native_error"] != "0"
                    || read(&format!("call-{call}-error.txt"))? != b"None\n"
                {
                    return Err("full-display capture retains a native/conversion error".into());
                }
            }
            // This flag marks Inputs::release_initial inside the sample
            // callback, not every way its retained fields can be destroyed.
            // A timeout without that callback may validly record zero; it is
            // an observation, never proof that native owners have drained.
            for key in [
                "timed_out",
                "content_calls",
                "sample_calls",
                "completed",
                "initial_inputs_released",
            ] {
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
                    ("initial_inputs_released", 1),
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
                if raw
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(bgr.as_chunks::<3>().0)
                    .any(|(raw, bgr)| raw[..3] != *bgr)
                {
                    return Err("same-binary full BGRA/BGR channel conversion differs".into());
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
            if let Some(finish) = &original_finish {
                if row[1] == 1 && !correct && !witness {
                    return Err(
                        "same-process Rust successful result has unknown target pixels".into(),
                    );
                }
                let mut original = original_call(&arm, call, row[1], &g, &consumer, finish)?;
                if !witness && original["usable_image"] == true && original["known_patch"] != true {
                    return Err(
                        "same-process original successful result has unknown target pixels".into(),
                    );
                }
                if let Some(sequence) = &sequence {
                    let original_rank = if mode == 0 { call } else { call + 3 };
                    let rust_rank = if mode == 0 { call + 3 } else { call };
                    let native = rows(&text(&format!("original-call-{call}.tsv"))?, 5)?;
                    let paired = read(&format!("original-call-{call}.bgr"))?
                        == read(&format!("call-{call}.bgr"))?;
                    if native[0][2] > sequence[original_rank][4] - sequence[original_rank][3]
                        || row[3] > sequence[rust_rank][4] - sequence[rust_rank][3]
                        || !witness && !paired
                    {
                        return Err(
                            "full-display paired bytes or enclosing call intervals differ".into(),
                        );
                    }
                    original["full_display_pixels_paired"] = paired.into();
                    if witness {
                        original["full_pixel_differences"] =
                            crate::macos_full_display_witness::differences(
                                &g,
                                &read(&format!("original-call-{call}.bgr"))?,
                                3,
                            )?;
                        original["rust_full_pixel_differences"] =
                            crate::macos_full_display_witness::differences(
                                &g,
                                &read(&format!("call-{call}.raw"))?,
                                4,
                            )?;
                    }
                }
                original_usable += u64::from(original["usable_image"] == true);
                original_known += u64::from(original["known_patch"] == true);
                original["mode"] = serde_json::json!(mode);
                originals.push(original);
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
        if let Some(finish) = &original_finish {
            expect(&summary, "original_usable_images", original_usable)?;
            expect(&summary, "original_known_patch_images", original_known)?;
            expect(finish, "held_images", original_usable)?;
            for entry in fs::read_dir(&arm)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("original-")
                    && (!entry.file_type()?.is_file()
                        || ![
                            "original-module.txt",
                            "original-finish.txt",
                            "original-owner-closed.txt",
                        ]
                        .contains(&name.as_ref())
                            && !(0..3).any(|call| {
                                [
                                    format!("original-call-{call}.tsv"),
                                    format!("original-call-{call}.bgr"),
                                    format!("original-call-{call}-layout.tsv"),
                                    format!("original-held-{call}.bgr"),
                                ]
                                .contains(&name.to_string())
                            }))
                {
                    return Err("unexpected original call/context record".into());
                }
            }
        }
        if witness && (usable != 3 || original_usable != 3) {
            return Err("view/census diagnostic requires all six actual capture images".into());
        }
        if !witness && (mode == 1 || full) && (usable, known) != (3, 3) {
            return Err("same-binary CG metadata positive control failed".into());
        }
        arms.push(serde_json::json!({"mode":mode,"consumer":consumer,"producer":producer,"usable_images":usable,"known_patch_images":known,"scene_closed":true,"scene_module_explicitly_unloaded":false}));
    }
    fn inventory(root: &Path, folder: &Path, out: &mut Vec<records::Artifact>) -> Result<()> {
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if entry.path() == root.join("same-binary-validation.json")
                || entry.path() == root.join("full-display-validation.json")
                || entry.path() == root.join("full-display-witness-validation.json")
                || entry.path() == root.join("maximum-level-validation.json")
            {
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
    let mut report = serde_json::json!({"layout_version":1,"arms":arms,"call_observations":calls,"actual_invocations":6,"inputs":artifacts,
        "scope":"Fresh Rust consumer/scene pairs with the same actual executable path, archived arm64 executable bytes and observed child/window identity. CG metadata is a strict three-image positive control; SC metadata results, including None/error/empty, are observations. Original five-second waits unchanged; no capture retry, source oracle substitution, late-frame recovery, callback-quiescence claim, border/broker cause or full P1 acceptance. The child retains its separate scene module until process exit."});
    if matches!(common_schema, Some(2 | 3)) {
        report["layout_version"] = serde_json::json!(2);
        report["actual_invocations"] = serde_json::json!(12);
        report["actual_original_invocations"] = serde_json::json!(6);
        report["actual_rust_invocations"] = serde_json::json!(6);
        report["original_call_observations"] = serde_json::json!(originals);
        report["original_first"] = serde_json::json!(true);
        report["scope"] = serde_json::json!(
            "Same-executable scene and same-process original/Rust display callers, with three unchanged original calls before three Rust calls on the same worker. Original/Rust None/empty/image results must match; Some images require exact known target pixels and unchanged held CPU bytes. CG is a strict three-image control per implementation. SC failures remain failed capture observations. Full display pixels, opposite invocation order, atomic timing, native callback drainage, prior failed oracle reclassification, root cause and complete P1 acceptance are not proved. Both explicit native module references remain loaded until process exit."
        );
    }
    if full {
        report["layout_version"] = 3.into();
        report["orders"] = serde_json::json!(order_records);
        report["original_first"] = serde_json::json!([true, false]);
        report["full_target_rgb_images"] = 12.into();
        report["scope"]="Same-executable full-display RGB fixture on a single 1:1 display; both implementations run on one caller worker in original-first and Rust-first arms. All twelve returned images, paired full bytes, actual native display selection and CPU owners after scene/source closure are strict. Sequence intervals enclose synchronous fixture/API calls, not hardware frame timestamps or callback quiescence. Static content, not changing-display freshness or general scaling/occlusion support. Old source oracles and failures remain unchanged; no full backend/P1 acceptance.".into();
    }
    if witness {
        report["layout_version"] = 4.into();
        report
            .as_object_mut()
            .unwrap()
            .remove("full_target_rgb_images");
        report["captured_full_images"] = 12.into();
        report["witnesses"] = serde_json::json!(witness_records);
        report["capture_parity_accepted"] = false.into();
        report["diagnostic_only"] = true.into();
        report["scope"] = "Separate interleaved CPU-view/window-census diagnostic. All twelve display images, held bytes and every pixel difference are recorded; seven native AppKit view redraws and CG window lists per arm are required. This added work can affect scheduling and is not a simultaneous display witness or proof of a dot owner. Original strict native RGB assertions and historical failures remain failed when pixels differ. No masked pixels, display parity, source-oracle replacement, callback-drain or full P1 acceptance.".into();
    }
    if maximum {
        report["layout_version"] = 5.into();
        report["owned_scene_level_control"] = serde_json::json!(level_records);
        report["maximum_window_level_fixture"] = true.into();
        report["scope"] = "Independent owned NSWindow maximum-level fixture with the original strict native full-RGB oracle and both implementation orders. Public CGWindowLevelForKey queries, applied NSWindow level, seven actual window censuses/view redraws per arm and all twelve returned/held images are required. Only owned fixture window state is changed. Diagnostic records do not waive native pixel failures, attribute historical pixels, prove callback drainage or accept full P1.".into();
    }
    let output = root.join(if maximum {
        "maximum-level-validation.json"
    } else if witness {
        "full-display-witness-validation.json"
    } else if full {
        "full-display-validation.json"
    } else {
        "same-binary-validation.json"
    });
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved same-binary observation report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    if maximum {
        println!(
            "macOS maximum_level_fixture arms=2 invocations=12 view_images=14 window_censuses=14 held_images=12 owned_scene_only=1 strict_native_rgb_assertions=1 diagnostic_only=1 full_P1_acceptance=0 observations_verified"
        );
    } else if witness {
        println!(
            "macOS full_display_witness arms=2 invocations=12 view_images=14 window_censuses=14 both_orders=1 held_images=12 diagnostic_only=1 full_display_acceptance=0 full_P1_acceptance=0 observations_verified"
        );
    } else if full {
        println!(
            "macOS full_display arms=2 invocations=12 original_calls=6 rust_calls=6 known_full_images=12 both_orders=1 full_bytes_paired=1 retained_after_close=12 source_oracle_replaced=0 full_P1_acceptance=0 passed"
        );
    } else if common_schema == Some(2) {
        println!(
            "macOS same_binary arms=2 invocations=12 original_calls=6 rust_calls=6 CG_positive_images=6 original_first=1 caller_results_equal=1 source_oracle_replaced=0 late_used_as_current=0 diagnostic_only=1 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "macOS same_binary arms=2 invocations=6 CG_positive_images=3 source_oracle_replaced=0 late_used_as_current=0 diagnostic_only=1 full_P1_acceptance=0 passed"
        );
    }
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
