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
    let report = validate(&directory)?;
    let output = directory.join("pending-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != serde_json::to_value(&report)? {
            return Err("saved pending validation differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "ScreenDC queued adjustment cases=40 captures=80 retained_images=80 queue_timeouts=0 actual_files=321 full_P1_acceptance=0 passed"
    );
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
fn validate(root: &Path) -> Result<Report> {
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
    if rows.len() != 404 {
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
                || objects[0] <= 0
                || objects[0] != objects[1]
                || closed != [1]
            {
                return Err(
                    format!("pending native premise/result differs at {cycle} {side}").into(),
                );
            }
            let delta = [after[0] - before[0], after[1] - before[1]];
            let outer = [
                setup[8] + delta[0],
                setup[9] + delta[1],
                setup[10] + delta[0],
                setup[11] + delta[1],
            ];
            if completed[8..12] != outer
                || completed[21..25]
                    != [outer[0], outer[1], outer[2] - outer[0], outer[3] - outer[1]]
                || completed[..8] != setup[..8]
                || completed[12..20] != setup[12..20]
                || completed[26..31] != setup[26..31]
                || final_geometry != completed
            {
                return Err("pending completed geometry/message differs".into());
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
    if !rows.is_empty() || inputs.len() != 321 {
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
        layout_version: 1,
        cases: 40,
        captures: 80,
        retained_images: 80,
        queue_timeouts: 0,
        inputs,
        scope: "Original/Rust capture with a fixture-held UI queue and real asynchronous geometry request, known pixels, one native correction after direct release and post-close owners. Does not prove arbitrary foreign-window races, DesktopDupWindow, input-free execution or full P1 acceptance.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
