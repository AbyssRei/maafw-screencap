// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read actual queued-adjustment records and pixels; never execute a desktop.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Serialize)]
struct Report {
    layout_version: u32,
    cases: usize,
    captures: usize,
    retained_images: usize,
    queue_timeouts: usize,
    inputs: Vec<records::Artifact>,
    scope: &'static str,
}

pub fn run(args: &[String]) -> Result<()> {
    run_kind(args, false)
}
pub(crate) fn run_desktop(args: &[String]) -> Result<()> {
    run_kind(args, true)
}
fn run_kind(args: &[String], desktop: bool) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace screen-pending <external directory> [--check-only]".into(),
            );
        }
    };
    let directory = records::external_root(Path::new(path))?;
    let report = validate(&directory, desktop)?;
    let output = directory.join("pending-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != serde_json::to_value(&report)? {
            return Err("saved pending validation differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    if desktop && report.layout_version == 2 {
        println!(
            "DesktopDupWindow queued adjustment cases=40 captures=80 retained_images=80 queue_timeouts=0 independent_update_owners=40 actual_files=323 quiescent_counts=40 full_P1_acceptance=0 passed"
        );
    } else if desktop {
        println!(
            "DesktopDupWindow queued adjustment cases=40 captures=80 retained_images=80 queue_timeouts=0 independent_update_owners=40 actual_files=322 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "ScreenDC queued adjustment cases=40 captures=80 retained_images=80 queue_timeouts=0 actual_files=321 full_P1_acceptance=0 passed"
        );
    }
    Ok(())
}
fn array(text: &str, width: usize) -> Result<Vec<i64>> {
    let values: Vec<i64> = serde_json::from_str(text)?;
    if values.len() != width {
        return Err("pending record width differs".into());
    }
    Ok(values)
}
fn native_coordinates(values: &[i64]) -> Result<()> {
    if values.iter().any(|n| i32::try_from(*n).is_err()) {
        return Err("pending native coordinate range differs".into());
    }
    Ok(())
}
fn visibility(text: &str) -> Result<Vec<i64>> {
    let raw: [u64; 16] = serde_json::from_str(text)?;
    raw.into_iter()
        .enumerate()
        .map(|(index, value)| {
            if index == 10 || index == 11 {
                // screen-parity.h stores signed POINT values in u64 slots. Preserve
                // their declared two's-complement representation on negative outputs.
                let signed = i64::from_ne_bytes(value.to_ne_bytes());
                native_coordinates(&[signed])?;
                Ok(signed)
            } else {
                Ok(i64::try_from(value)?)
            }
        })
        .collect()
}
fn screen(row: &[i64]) -> [i64; 4] {
    // Every field was narrowed to i32; widened sums cannot overflow i64.
    [
        row[16] + row[12],
        row[17] + row[13],
        row[16] + row[14],
        row[17] + row[15],
    ]
}
fn completed_geometry_matches(
    setup: &[i64],
    completed: &[i64],
    final_geometry: &[i64],
    delta: [i64; 2],
) -> bool {
    if setup.len() != 31 || completed.len() != 31 || final_geometry.len() != 31 {
        return false;
    }
    // The caller has validated native i32 coordinates. Widened translations
    // fit i64. Both the outer bounds and ClientToScreen origin move; the local
    // GetClientRect, styles, monitor and fixture configuration stay unchanged.
    let outer = [
        setup[8] + delta[0],
        setup[9] + delta[1],
        setup[10] + delta[0],
        setup[11] + delta[1],
    ];
    completed[8..12] == outer
        && completed[21..25] == [outer[0], outer[1], outer[2] - outer[0], outer[3] - outer[1]]
        && completed[..8] == setup[..8]
        && completed[12..16] == setup[12..16]
        && completed[16..18] == [setup[16] + delta[0], setup[17] + delta[1]]
        && completed[18..20] == setup[18..20]
        && completed[26..31] == setup[26..31]
        && final_geometry == completed
}
fn pixels(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 192 * 128 * 3 {
        return Err("pending image size differs".into());
    }
    for y in 0..128 {
        for x in 0..192 {
            let expected = if x < 64 {
                [0x1f, 0x53, 0xa7]
            } else if y < 64 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = (y * 192 + x) * 3;
            if bytes[offset..offset + 3] != expected {
                return Err("pending known pixel differs".into());
            }
        }
    }
    Ok(())
}
fn validate(root: &Path, desktop: bool) -> Result<Report> {
    let text = fs::read_to_string(root.join("pending.tsv"))?;
    let mut lines = text.lines();
    if lines.next() != Some("cycle\tside\tstage\tvalues") {
        return Err("pending header differs".into());
    }
    let mut rows = BTreeMap::new();
    for line in lines {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err("pending row width differs".into());
        }
        let cycle: i32 = fields[0].parse()?;
        if cycle.to_string() != fields[0]
            || rows
                .insert((cycle, fields[1], fields[2]), fields[3])
                .is_some()
        {
            return Err("noncanonical or duplicate pending row".into());
        }
    }
    let resource_layout = rows.contains_key(&(-1, "fixture", "resources"));
    if rows.len() != 404 + usize::from(resource_layout) || (resource_layout && !desktop) {
        return Err("incomplete or additional pending matrix".into());
    }
    let mut take = |cycle, side, stage, width| -> Result<Vec<i64>> {
        let text = rows
            .remove(&(cycle, side, stage))
            .ok_or("missing pending stage")?;
        if stage == "visible" {
            visibility(text)
        } else {
            array(text, width)
        }
    };
    let monitor = take(-1, "fixture", "monitor", 4)?;
    let work = take(-1, "fixture", "work", 4)?;
    let before = take(-1, "fixture", "before", 4)?;
    let after = take(-1, "fixture", "after", 4)?;
    if resource_layout && take(-1, "fixture", "resources", 1)? != [2] {
        return Err("unknown desktop resource boundary layout".into());
    }
    for rectangle in [&monitor, &work, &before, &after] {
        native_coordinates(rectangle)?;
    }
    if monitor[0] > work[0]
        || monitor[1] > work[1]
        || work[2] > monitor[2]
        || work[3] > monitor[3]
        || monitor[0] > before[0]
        || monitor[1] > before[1]
        || before[2] > monitor[2]
        || before[3] > monitor[3]
        || before[2] - before[0] != 192
        || before[3] - before[1] != 128
        || work[2] - work[0] < 192
        || work[3] - work[1] < 128
        || before == after
    {
        return Err("pending native geometry premise differs".into());
    }
    let left = before[0].clamp(work[0], work[2] - 192);
    let top = before[1].clamp(work[1], work[3] - 128);
    if after != [left, top, left + 192, top + 128] {
        return Err("pending work-area correction differs".into());
    }
    let mut inputs = vec![records::artifact(root, &root.join("pending.tsv"))?];
    let update_threads = if desktop {
        let threads = validate_updates(root, &monitor, &before, &after)?;
        inputs.push(records::artifact(root, &root.join("updates.tsv"))?);
        Some(threads)
    } else {
        None
    };
    // Historical layout-one records keep their original in-flight count scope.
    // New desktop records add independently checked before-create/after-join
    // observations, including real window validity and owner/caller identities.
    let resource_boundaries = if resource_layout {
        inputs.push(records::artifact(root, &root.join("resources.tsv"))?);
        Some(validate_resource_text(&fs::read_to_string(
            root.join("resources.tsv"),
        )?)?)
    } else {
        if root.join("resources.tsv").exists() {
            return Err("resource observations require the explicit layout marker".into());
        }
        None
    };
    for cycle in 0..20 {
        for side in ["source", "rust"] {
            let setup = take(cycle, side, "setup", 31)?;
            let visible = take(cycle, side, "visible", 16)?;
            let entered = take(cycle, side, "entered", 7)?;
            let pending = take(cycle, side, "pending", 7)?;
            let initial_client = take(cycle, side, "initial-client", 4)?;
            let released = take(cycle, side, "released", 7)?;
            let completed = take(cycle, side, "completed", 31)?;
            let final_geometry = take(cycle, side, "final", 31)?;
            let objects = take(cycle, side, "gdi", 2)?;
            let closed = take(cycle, side, "closed", 1)?;
            if let Some(threads) = &update_threads {
                let update = threads[usize::try_from(cycle)? * 2 + usize::from(side == "rust")];
                if update[0] == u64::try_from(entered[5])?
                    || update[1] != u64::try_from(entered[6])?
                {
                    return Err("update owner must be independent of held target queue on the same capture caller".into());
                }
            }
            if let Some(resources) = &resource_boundaries {
                let index = usize::try_from(cycle)? * 2 + usize::from(side == "rust");
                let resource = &resources[index];
                let update = update_threads.as_ref().ok_or("missing update owners")?[index];
                if resource.caller != u32::try_from(entered[6])?
                    || resource.target_owner != u32::try_from(entered[5])?
                    || resource.update_owner != u32::try_from(update[0])?
                    || resource.target != u64::try_from(visible[4])?
                    || resource.update != update[2]
                    || objects != [i64::from(resource.before), i64::from(resource.after)]
                {
                    return Err(
                        "resource boundary differs from actual capture/update owners".into(),
                    );
                }
            }
            for geometry in [&setup, &completed, &final_geometry] {
                native_coordinates(&geometry[..18])?;
                native_coordinates(&geometry[21..25])?;
            }
            if setup[..4] != monitor
                || setup[4..8] != work
                || screen(&setup) != before.as_slice()
                || setup[18] & 0x8000_0000 != 0
                || setup[19] & 8 == 0
                || visible[..3] != [1, 1, 0]
                || visible[3] != visible[4]
                || visible[4] == 0
                || visible[6] != visible[7]
                || visible[6] == 0
                || visible[8..10] != [0, 0]
                || visible[10..14] != [before[0], before[1], 192, 128]
                || entered[..5] != [1, 1, 0, 0, 0]
                || entered[5] == 0
                || entered[6] == 0
                || entered[5] == entered[6]
                || pending != entered
                || initial_client != before
                || released[..5] != [1, 1, 1, 1, 0]
                || released[5..] != entered[5..]
                || screen(&completed) != after.as_slice()
                || completed[20] != setup[20].checked_add(1).ok_or("message count overflow")?
                || completed[25] & 0x14 != 0x14
                || objects[0] < i64::from(resource_boundaries.is_none())
                || objects[0] != objects[1]
                || closed != [1]
            {
                return Err(
                    format!("pending native premise/result differs at {cycle} {side}").into(),
                );
            }
            let delta = [after[0] - before[0], after[1] - before[1]];
            if !completed_geometry_matches(&setup, &completed, &final_geometry, delta) {
                return Err(format!(
                    "pending completed geometry/message differs at {cycle} {side}"
                )
                .into());
            }
            for stage in ["pending", "completed"] {
                let mut original = Vec::new();
                for held in [false, true] {
                    let name = format!("{cycle}-{side}-{stage}{}", if held { "-held" } else { "" });
                    let layout = root.join(format!("{name}.txt"));
                    let image = root.join(format!("{name}.bgr"));
                    if fs::read_to_string(&layout)? != "192\t128\t576\t73728\n" {
                        return Err("pending image layout differs".into());
                    }
                    let body = fs::read(&image)?;
                    pixels(&body)?;
                    if held && body != original {
                        return Err("pending retained owner bytes differ".into());
                    }
                    if !held {
                        original = body;
                    }
                    inputs.push(records::artifact(root, &layout)?);
                    inputs.push(records::artifact(root, &image)?);
                }
            }
        }
    }
    if !rows.is_empty()
        || inputs.len() != 321 + usize::from(desktop) + usize::from(resource_boundaries.is_some())
    {
        return Err("additional pending records".into());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|n| n == "pending-validation.json")
        {
            continue;
        }
        if !path.is_file() || !inputs.iter().any(|input| root.join(&input.path) == path) {
            return Err("unregistered pending evidence file".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Report {
        layout_version: if resource_boundaries.is_some() { 2 } else { 1 },
        cases: 40,
        captures: 80,
        retained_images: 80,
        queue_timeouts: 0,
        inputs,
        scope: if resource_boundaries.is_some() {
            "Original whole F DesktopDupWindow/Rust first capture with a held target UI queue, unchanged native capture and independent nonoverlapping update owner. Strict GDI counts surround both fixture lifetimes: before create and after checked close/worker join, with actual invalid HWND and cross-bound caller/owner observations. Complete target pixels, one queued correction and post-close CPU owners. No historical failure reclassification, arbitrary races, global GPU, input-free execution or full P1 acceptance follows."
        } else if desktop {
            "Original whole F DesktopDupWindow/Rust first capture with a held target UI queue, real asynchronous correction and a distinct nonoverlapping same-output update owner. Native warmups/waits/full-output conversion/crop remain unchanged; source target clone occurs only at acceptance publication. Known complete target pixels, one movement after direct release, retained CPU owners and explicit update-owner close. No arbitrary foreign-window races, in-flight driver fault, global GPU, input-free execution or full P1 acceptance follows."
        } else {
            "Original/Rust capture with a fixture-held UI queue and real asynchronous geometry request, known pixels, one native correction after direct release and post-close owners. Does not prove arbitrary foreign-window races, DesktopDupWindow, input-free execution or full P1 acceptance."
        },
    })
}

fn validate_updates(
    root: &Path,
    monitor: &[i64],
    before: &[i64],
    after: &[i64],
) -> Result<Vec<[u64; 3]>> {
    let text = fs::read_to_string(root.join("updates.tsv"))?;
    validate_update_text(&text, monitor, before, after)
}
fn validate_update_text(
    text: &str,
    monitor: &[i64],
    before: &[i64],
    after: &[i64],
) -> Result<Vec<[u64; 3]>> {
    let mut lines = text.lines();
    if lines.next() != Some("cycle\tside\tstage\tvalues") {
        return Err("update owner header differs".into());
    }
    let mut rows = BTreeMap::new();
    for line in lines {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err("update owner row width differs".into());
        }
        let cycle: u32 = fields[0].parse()?;
        if cycle.to_string() != fields[0]
            || rows
                .insert((cycle, fields[1], fields[2]), fields[3])
                .is_some()
        {
            return Err("noncanonical/duplicate update owner row".into());
        }
    }
    if rows.len() != 200 {
        return Err("incomplete update owner matrix".into());
    }
    let mut owners = Vec::new();
    for cycle in 0..20 {
        for side in ["source", "rust"] {
            // Geometry is a signed record; other arrays use exact native u64.
            let geometry_text = rows
                .get(&(cycle, side, "geometry"))
                .ok_or("missing update geometry")?;
            let geometry: Vec<i64> = serde_json::from_str(geometry_text)?;
            if geometry.len() != 31 {
                return Err("update geometry width differs".into());
            }
            native_coordinates(&geometry[..18])?;
            let client = screen(&geometry);
            let overlaps = |target: &[i64]| {
                client[0] < target[2]
                    && target[0] < client[2]
                    && client[1] < target[3]
                    && target[1] < client[3]
            };
            if geometry[..4] != *monitor
                || client[2] - client[0] != 32
                || client[3] - client[1] != 32
                || client[0] < monitor[0]
                || client[1] < monitor[1]
                || client[2] > monitor[2]
                || client[3] > monitor[3]
                || overlaps(before)
                || overlaps(after)
            {
                return Err("update owner overlaps target or selects a different output".into());
            }
            let mut take = |stage| -> Result<Vec<u64>> {
                Ok(serde_json::from_str(
                    rows.remove(&(cycle, side, stage))
                        .ok_or("missing update owner stage")?,
                )?)
            };
            let threads = take("threads")?;
            let first = take("before")?;
            let last = take("after")?;
            let closed = take("closed")?;
            if threads.len() != 4
                || threads[0] == 0
                || threads[0] == threads[1]
                || threads[0] != threads[2]
                || first.len() != 3
                || last.len() != 3
                || first[0] == 0
                || last[0] != first[0]
                || last[1] <= first[1]
                || first[2] != 0
                || last[2] != 0
                || closed != [1]
            {
                return Err("update owner threads/progress/error/close differs".into());
            }
            rows.remove(&(cycle, side, "geometry"));
            owners.push([threads[0], threads[1], first[0]]);
        }
    }
    if !rows.is_empty() {
        return Err("additional update owner records".into());
    }
    Ok(owners)
}

struct ResourceBoundary {
    before: u32,
    after: u32,
    caller: u32,
    target: u64,
    update: u64,
    target_owner: u32,
    update_owner: u32,
}
fn validate_resource_text(text: &str) -> Result<Vec<ResourceBoundary>> {
    let mut lines = text.lines();
    if lines.next() != Some("cycle\tside\tstage\tvalues") {
        return Err("resource boundary header differs".into());
    }
    let mut boundaries = Vec::new();
    for cycle in 0..20 {
        for side in ["source", "rust"] {
            let mut take = |stage: &str, width| -> Result<Vec<u64>> {
                let line = lines.next().ok_or("missing resource boundary")?;
                let fields: Vec<_> = line.split('\t').collect();
                if fields.len() != 4
                    || fields[0] != cycle.to_string()
                    || fields[1] != side
                    || fields[2] != stage
                {
                    return Err("noncanonical resource boundary order/identity".into());
                }
                let values: Vec<u64> = serde_json::from_str(fields[3])?;
                if values.len() != width {
                    return Err("resource boundary width differs".into());
                }
                Ok(values)
            };
            let baseline = take("baseline", 2)?;
            let owners = take("owners", 5)?;
            let closed = take("closed", 4)?;
            let boundary = ResourceBoundary {
                before: u32::try_from(baseline[0])?,
                after: u32::try_from(closed[0])?,
                caller: u32::try_from(baseline[1])?,
                target: owners[0],
                update: owners[1],
                target_owner: u32::try_from(owners[2])?,
                update_owner: u32::try_from(owners[3])?,
            };
            if boundary.before != boundary.after
                || boundary.caller == 0
                || owners[4] != u64::from(boundary.caller)
                || closed[1] != u64::from(boundary.caller)
                || closed[2..] != [0, 0]
                || boundary.target == 0
                || boundary.update == 0
                || boundary.target == boundary.update
                || boundary.target_owner == 0
                || boundary.update_owner == 0
                || boundary.target_owner == boundary.update_owner
                || boundary.target_owner == boundary.caller
                || boundary.update_owner == boundary.caller
            {
                return Err("unbalanced resources, live window or invalid resource owner".into());
            }
            boundaries.push(boundary);
        }
    }
    if lines.next().is_some() {
        return Err("additional resource boundaries".into());
    }
    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_resource_leaks_live_windows_invalid_native_ids_and_incomplete_boundaries() {
        let mut valid = String::from("cycle\tside\tstage\tvalues\n");
        for cycle in 0..20 {
            for side in ["source", "rust"] {
                for (stage, values) in [
                    ("baseline", "[0,20]"),
                    ("owners", "[101,102,10,11,20]"),
                    ("closed", "[0,20,0,0]"),
                ] {
                    valid.push_str(&format!("{cycle}\t{side}\t{stage}\t{values}\n"));
                }
            }
        }
        assert_eq!(validate_resource_text(&valid).unwrap().len(), 40);
        for invalid in [
            valid.replace("[0,20,0,0]", "[1,20,0,0]"),
            valid.replace("[0,20,0,0]", "[0,20,1,0]"),
            valid.replace("[0,20,0,0]", "[0,20,0,1]"),
            valid.replace("[0,20,0,0]", "[0,21,0,0]"),
            valid.replace("[0,20]", "[4294967296,20]"),
            valid.replace("[0,20]", "[0,4294967296]"),
            valid.replace("101,102,10,11,20", "101,102,4294967296,11,20"),
            valid.replace("101,102,10,11,20", "101,102,10,4294967296,20"),
            valid.replace("101,102,10,11,20", "101,101,10,11,20"),
            valid.replace("101,102,10,11,20", "101,102,10,10,20"),
            valid.replace("101,102,10,11,20", "101,102,20,11,20"),
            valid.replace("101,102,10,11,20", "0,102,10,11,20"),
            valid.lines().take(120).collect::<Vec<_>>().join("\n"),
            valid.clone() + "0\tsource\tclosed\t[0,20,0,0]\n",
        ] {
            assert!(validate_resource_text(&invalid).is_err());
        }
    }
    #[test]
    fn rejects_disjoint_update_owner_overlap_missing_progress_foreign_threads_and_incomplete_rows()
    {
        let monitor = [0, 0, 1920, 1080];
        let before = [0, 952, 192, 1080];
        let after = [0, 904, 192, 1032];
        let mut geometry = [0; 31];
        geometry[..4].copy_from_slice(&monitor);
        geometry[14] = 32;
        geometry[15] = 32;
        let make = |geometry: &[i64], threads: &str, last: &str, closed: &str| {
            let mut text = String::from("cycle\tside\tstage\tvalues\n");
            for cycle in 0..20 {
                for side in ["source", "rust"] {
                    for (stage, value) in [
                        ("geometry", serde_json::to_string(geometry).unwrap()),
                        ("threads", threads.into()),
                        ("before", "[1,2,0]".into()),
                        ("after", last.into()),
                        ("closed", closed.into()),
                    ] {
                        text.push_str(&format!("{cycle}\t{side}\t{stage}\t{value}\n"));
                    }
                }
            }
            text
        };
        let valid = make(&geometry, "[10,20,10,30]", "[1,3,0]", "[1]");
        validate_update_text(&valid, &monitor, &before, &after).unwrap();
        for invalid in [
            make(&geometry, "[20,20,20,30]", "[1,3,0]", "[1]"),
            make(&geometry, "[10,20,11,30]", "[1,3,0]", "[1]"),
            make(&geometry, "[10,20,10,30]", "[1,2,0]", "[1]"),
            make(&geometry, "[10,20,10,30]", "[2,3,0]", "[1]"),
            make(&geometry, "[10,20,10,30]", "[1,3,1]", "[1]"),
            make(&geometry, "[10,20,10,30]", "[1,3,0]", "[0]"),
            valid.lines().take(200).collect::<Vec<_>>().join("\n"),
            valid.clone() + "0\tsource\tclosed\t[1]\n",
        ] {
            assert!(validate_update_text(&invalid, &monitor, &before, &after).is_err());
        }
        geometry[17] = 950;
        assert!(
            validate_update_text(
                &make(&geometry, "[10,20,10,30]", "[1,3,0]", "[1]"),
                &monitor,
                &before,
                &after
            )
            .is_err()
        );
    }
    #[test]
    fn accepts_translated_screen_origin_and_rejects_other_geometry_changes() {
        // A framed 192x128 client moves upward by a 48px work-area correction.
        // The screen origin follows the move; the local client rect does not.
        let setup = [
            0,
            0,
            1920,
            1080,
            0,
            0,
            1920,
            1032,
            -8,
            921,
            200,
            1088,
            0,
            0,
            192,
            128,
            0,
            952,
            0x14cf_0000,
            0x108,
            1,
            -8,
            921,
            208,
            167,
            0x34,
            96,
            1,
            0,
            208,
            167,
        ];
        let mut moved = setup;
        moved[9] = 873;
        moved[11] = 1040;
        moved[17] = 904;
        moved[20] = 2;
        moved[22] = 873;
        assert!(completed_geometry_matches(&setup, &moved, &moved, [0, -48]));
        for index in [0, 4, 8, 9, 12, 14, 16, 17, 18, 19, 21, 22, 23, 24, 26, 30] {
            let mut wrong = moved;
            wrong[index] += 1;
            assert!(!completed_geometry_matches(
                &setup,
                &wrong,
                &wrong,
                [0, -48]
            ));
        }
        let mut later = moved;
        later[20] += 1;
        assert!(!completed_geometry_matches(
            &setup,
            &moved,
            &later,
            [0, -48]
        ));
        assert!(!completed_geometry_matches(
            &setup[..30],
            &moved,
            &moved,
            [0, -48]
        ));
    }
    #[test]
    fn rejects_wrong_layout_out_of_range_coordinates_and_blank_capture() {
        assert!(array("[1,2]", 4).is_err());
        assert!(native_coordinates(&[2147483648]).is_err());
        let mut raw = [0_u64; 16];
        raw[10] = u64::MAX;
        assert_eq!(
            visibility(&serde_json::to_string(&raw).unwrap()).unwrap()[10],
            -1
        );
        assert!(pixels(&vec![0; 192 * 128 * 3]).is_err());
        assert!(pixels(&[]).is_err());
    }
}
