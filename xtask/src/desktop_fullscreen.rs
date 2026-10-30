// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recheck the actual F fullscreen DesktopDupWindow source/Rust records.
use crate::{Result, records};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    run_case(args, false)
}
pub fn run_workarea(args: &[String]) -> Result<()> {
    run_case(args, true)
}
fn run_case(args: &[String], workarea: bool) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected upstream-trace desktop-fullscreen <external window-fullscreen directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let source = rows(&read(if workarea {
        "workarea.tsv"
    } else {
        "fullscreen.tsv"
    })?)?;
    let rust = rows(&read("rust/snapshots.tsv")?)?;
    let count = if workarea { 4 } else { 3 };
    if source.len() != count || rust.len() != count || source[count - 1].len() != 3 {
        return Err("incomplete fullscreen native records".into());
    }
    if workarea {
        correction(&source[0], &source[1])?;
        correction(&rust[0], &rust[1])?;
    }
    let start = usize::from(workarea);
    let source_shape = geometry_case(&source[start..count - 1], workarea)?;
    let rust_shape = geometry_case(&rust[start..], workarea)?;
    if source_shape != rust_shape || source[0][..8] != rust[0][..8] {
        return Err("fullscreen sides used different native monitor/work geometry".into());
    }
    let [width, height] = source_shape;
    let pixels = read("target.bgr")?;
    known(&pixels, width, height)?;
    if read("image.bgr")? != pixels || read("held-after-close.bgr")? != pixels {
        return Err("original fullscreen owner changed after close".into());
    }
    let marker = rows(&read("rust/marker.tsv")?)?;
    if marker.len() != 1 || marker[0].len() != 3 {
        return Err("incomplete Rust in-target update record".into());
    }
    for row in [&source[count - 1], &marker[0]] {
        if row[0] == 0 || row[1] < 1 || row[2] != 0 {
            return Err("fullscreen marker did not present or reported errors".into());
        }
    }
    for call in 0..2 {
        let pixels = read(&format!("rust/capture-{call}.bgr"))?;
        known(&pixels, width, height)?;
        let layout = rows(&read(&format!("rust/capture-{call}.txt"))?)?;
        let row = width.checked_mul(3).ok_or("row overflow")?;
        let span = row.checked_mul(height).ok_or("span overflow")?;
        if layout
            != vec![vec![
                i64::try_from(width)?,
                i64::try_from(height)?,
                i64::try_from(row)?,
                i64::try_from(span)?,
            ]]
            || read(&format!("rust/held-{call}.bgr"))? != pixels
        {
            return Err("Rust fullscreen layout or owner changed after native close".into());
        }
    }
    let result = if workarea {
        b"cases=1\ncaptures=2\nretained_after_close=2\nknown_pixels=1\nhelper_adjustments=1\ncapture_adjustments=0\nimage_owners_released=1\n".as_slice()
    } else {
        b"cases=1\ncaptures=2\nretained_after_close=2\nknown_pixels=1\ngeometry_adjustments=0\nimage_owners_released=1\n".as_slice()
    };
    if read("rust/result.txt")? != result {
        return Err("Rust fullscreen completion/owner release missing".into());
    }
    let mut report = serde_json::json!({"layout_version":1,"cases":1,"source_captures":1,"rust_captures":2,
        "width":width,"height":height,"retained_after_close":3,"source_marker_presentations":source[count-1][1],
        "rust_marker_presentations":marker[0][1],"artifacts":artifacts,
        "scope":"Exact WS_POPUP client equals the native monitor while rcWork differs. Unchanged original DesktopDupWindow in a separate process captures once, Rust twice; original warmup/wait/conversion/crop remain. A UI-owner timer redraws a known 16x16 in-target marker; every target pixel, consistent marker palette, unchanged client geometry/message count, native presentation errors and CPU bytes after capture/window close are checked. This one fullscreen case is not the complete edge/DPI/output/manager matrix, first-capture correction race, source resource-leak proof or full P1 acceptance."});
    if workarea {
        report["helper_adjustments_per_side"] = 1.into();
        report["capture_adjustments_per_side"] = 0.into();
        report["scope"] = "Ordinary WS_OVERLAPPEDWINDOW client initially equals rcMonitor while rcWork differs. The exact original helper and migrated Rust helper each correct once to rcWork, and their asynchronous native geometry completion is observed before any capture. Both sides repaint at the corrected size and present a known in-target 16x16 marker. Separate-process unchanged original DesktopDupWindow captures once and Rust twice, retaining default warmup/wait/full-output conversion and target crop. Complete target pixels, exact before/after client/outer geometry and WINDOWPOS, no repeated correction during capture, marker consistency and CPU bytes after native/window close are checked. This is one post-correction ordinary-window case, not the first-capture adjustment race, all edges/DPI/outputs/manager/resource faults or full P1 acceptance.".into();
    }
    let output = root.join(if workarea {
        "workarea-validation.json"
    } else {
        "fullscreen-validation.json"
    });
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved fullscreen report differs from native records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    if workarea {
        println!(
            "DesktopDupWindow workarea cases=1 source_captures=1 rust_captures=2 known_target_pixels=1 marker_consistent=1 retained_after_close=3 helper_adjustments=1 capture_adjustments=0 work_area_distinct=1 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "DesktopDupWindow fullscreen cases=1 source_captures=1 rust_captures=2 known_target_pixels=1 marker_consistent=1 retained_after_close=3 geometry_adjustments=0 work_area_distinct=1 full_P1_acceptance=0 passed"
        );
    }
    Ok(())
}
fn rows(bytes: &[u8]) -> Result<Vec<Vec<i64>>> {
    let text = std::str::from_utf8(bytes)?;
    if !text.ends_with('\n') {
        return Err("incomplete fullscreen record".into());
    }
    text.lines()
        .map(|line| line.split('\t').map(|v| Ok(v.parse::<i64>()?)).collect())
        .collect()
}
#[cfg(test)]
fn geometry(rows: &[Vec<i64>]) -> Result<[usize; 2]> {
    geometry_case(rows, false)
}
fn geometry_case(rows: &[Vec<i64>], workarea: bool) -> Result<[usize; 2]> {
    let first = rows.first().ok_or("missing fullscreen setup")?;
    if first.len() != 31 || first[..4] == first[4..8] {
        return Err("fullscreen requires real distinct monitor/work rectangles".into());
    }
    let offset = if workarea { 4 } else { 0 };
    let width = first[offset + 2]
        .checked_sub(first[offset])
        .ok_or("width overflow")?;
    let height = first[offset + 3]
        .checked_sub(first[offset + 1])
        .ok_or("height overflow")?;
    if width < 16 || height < 16 {
        return Err("invalid fullscreen dimensions".into());
    }
    for row in rows {
        if row.len() != 31
            || row[..8] != first[..8]
            || row[12..16] != [0, 0, width, height]
            || row[16..18] != first[offset..offset + 2]
            || (row[18] & 0x8000_0000 == 0) != workarea
            || row[20] != first[20]
            || row[26] <= 0
            || row[27] != 1
            || row[28] != 0
        {
            return Err(
                "native fullscreen geometry/correction or square-corner premise differs".into(),
            );
        }
    }
    Ok([usize::try_from(width)?, usize::try_from(height)?])
}
fn correction(before: &[i64], after: &[i64]) -> Result<()> {
    if before.len() != 31 || after.len() != 31 {
        return Err("incomplete work-area correction snapshots".into());
    }
    for value in &before[..18] {
        i32::try_from(*value)?;
    }
    for value in &after[..18] {
        i32::try_from(*value)?;
    }
    for row in [before, after] {
        for slot in [18, 19, 25, 26] {
            u32::try_from(row[slot])?;
        }
    }
    let mw = before[2] - before[0];
    let mh = before[3] - before[1];
    let outer = [
        before[4] - (before[16] - before[8]),
        before[5] - (before[17] - before[9]),
        before[6] + (before[10] - before[16] - before[14]),
        before[7] + (before[11] - before[17] - before[15]),
    ];
    if mw < 16
        || mh < 16
        || before[0..8] != after[0..8]
        || before[0..4] == before[4..8]
        || before[4] < before[0]
        || before[5] < before[1]
        || before[6] > before[2]
        || before[7] > before[3]
        || before[12..16] != [0, 0, mw, mh]
        || before[16..18] != before[..2]
        || before[18] & !0x0600_0000 != 0x10cf_0000
        || before[18..20] != after[18..20]
        || before[20] < 0
        || after[20]
            != before[20]
                .checked_add(1)
                .ok_or("correction count overflow")?
        || after[8..12] != outer
        || after[21..25] != [outer[0], outer[1], outer[2] - outer[0], outer[3] - outer[1]]
        || after[25] & 0x14 != 0x14
        || after[25] & 3 != 0
        || before[26] != after[26]
        || before[26] <= 0
        || before[27..29] != [1, 0]
        || after[27..29] != [1, 0]
    {
        return Err(
            "ordinary native correction differs from complete work-area geometry/flags/count"
                .into(),
        );
    }
    geometry_case(&[after.to_vec()], true)?;
    Ok(())
}
fn known(bytes: &[u8], width: usize, height: usize) -> Result<()> {
    let stride = width.checked_mul(3).ok_or("fullscreen row overflow")?;
    let span = stride
        .checked_mul(height)
        .ok_or("fullscreen span overflow")?;
    if width < 16 || height < 16 || bytes.len() != span {
        return Err("fullscreen pixel span differs".into());
    }
    let marker = &bytes[..3];
    if marker != [0x1a, 0x2b, 0x3c] && marker != [0x4e, 0x5f, 0x60] {
        return Err("fullscreen marker has unknown palette".into());
    }
    for y in 0..height {
        for x in 0..width {
            let expected: &[u8] = if x < 16 && y < 16 {
                marker
            } else if x < width / 3 {
                &[0x1f, 0x53, 0xa7]
            } else if y < height / 2 {
                &[0xb9, 0x4d, 0x27]
            } else {
                &[0x3b, 0xe2, 0x81]
            };
            if &bytes[y * stride + x * 3..y * stride + x * 3 + 3] != expected {
                return Err("known fullscreen target pixel differs".into());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ordinary_monitor_clients_incomplete_correction_and_wrong_windowpos() {
        let mut before = vec![0; 31];
        before[..8].copy_from_slice(&[0, 0, 24, 24, 0, 0, 24, 20]);
        before[8..18].copy_from_slice(&[-1, -2, 25, 25, 0, 0, 24, 24, 0, 0]);
        before[18] = 0x10cf_0000;
        before[26] = 96;
        before[27] = 1;
        let mut after = before.clone();
        after[11] = 21;
        after[15] = 20;
        after[20] = 1;
        after[21..26].copy_from_slice(&[-1, -2, 26, 23, 0x4014]);
        correction(&before, &after).unwrap();
        assert_eq!(geometry_case(&[after.clone()], true).unwrap(), [24, 20]);
        assert!(geometry_case(&[before.clone()], true).is_err());
        assert!(geometry_case(&[after.clone()], false).is_err());
        for slot in [8, 11, 14, 15, 16, 18, 20, 21, 22, 23, 24, 25, 26, 27, 28] {
            let mut bad = after.clone();
            bad[slot] += 1;
            assert!(correction(&before, &bad).is_err(), "accepted slot {slot}");
        }
        let mut popup = before.clone();
        popup[18] = 0x9000_0000;
        assert!(correction(&popup, &after).is_err());
        let mut invalid_flags = after.clone();
        invalid_flags[25] = -12;
        assert!(correction(&before, &invalid_flags).is_err());
        assert!(correction(&before, &before).is_err());
        assert!(correction(&before[..30], &after).is_err());
    }
    #[test]
    fn rejects_work_area_clients_correction_repetition_and_unknown_marker_pixels() {
        let mut row = vec![0; 31];
        row[..8].copy_from_slice(&[0, 0, 24, 24, 0, 0, 24, 20]);
        row[14] = 24;
        row[15] = 24;
        row[18] = 0x9000_0000;
        row[20] = 1;
        row[26] = 96;
        row[27] = 1;
        geometry(&[row.clone(), row.clone()]).unwrap();
        for slot in [14, 15, 16, 17, 20, 27, 28] {
            let mut bad = row.clone();
            bad[slot] += 1;
            assert!(geometry(&[row.clone(), bad]).is_err());
        }
        let mut work = row.clone();
        work[15] = 20;
        assert!(geometry(&[work]).is_err());
        let mut no_taskbar = row.clone();
        no_taskbar[7] = 24;
        assert!(geometry(&[no_taskbar]).is_err());
        assert!(known(&vec![0; 24 * 24 * 3], 24, 24).is_err());
        assert!(known(&[], usize::MAX, 24).is_err());
    }
}
