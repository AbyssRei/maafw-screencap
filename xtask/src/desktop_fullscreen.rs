// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recheck the actual F fullscreen DesktopDupWindow source/Rust records.
use crate::{Result, records};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
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
    let source = rows(&read("fullscreen.tsv")?)?;
    let rust = rows(&read("rust/snapshots.tsv")?)?;
    if source.len() != 3 || rust.len() != 3 || source[2].len() != 3 {
        return Err("incomplete fullscreen native records".into());
    }
    let source_shape = geometry(&source[..2])?;
    let rust_shape = geometry(&rust)?;
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
    for row in [&source[2], &marker[0]] {
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
    if read("rust/result.txt")?!=b"cases=1\ncaptures=2\nretained_after_close=2\nknown_pixels=1\ngeometry_adjustments=0\nimage_owners_released=1\n" {
        return Err("Rust fullscreen completion/owner release missing".into());
    }
    let report = serde_json::json!({"layout_version":1,"cases":1,"source_captures":1,"rust_captures":2,
        "width":width,"height":height,"retained_after_close":3,"source_marker_presentations":source[2][1],
        "rust_marker_presentations":marker[0][1],"artifacts":artifacts,
        "scope":"Exact WS_POPUP client equals the native monitor while rcWork differs. Unchanged original DesktopDupWindow in a separate process captures once, Rust twice; original warmup/wait/conversion/crop remain. A UI-owner timer redraws a known 16x16 in-target marker; every target pixel, consistent marker palette, unchanged client geometry/message count, native presentation errors and CPU bytes after capture/window close are checked. This one fullscreen case is not the complete edge/DPI/output/manager matrix, first-capture correction race, source resource-leak proof or full P1 acceptance."});
    let output = root.join("fullscreen-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved fullscreen report differs from native records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDupWindow fullscreen cases=1 source_captures=1 rust_captures=2 known_target_pixels=1 marker_consistent=1 retained_after_close=3 geometry_adjustments=0 work_area_distinct=1 full_P1_acceptance=0 passed"
    );
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
fn geometry(rows: &[Vec<i64>]) -> Result<[usize; 2]> {
    let first = rows.first().ok_or("missing fullscreen setup")?;
    if first.len() != 31 || first[..4] == first[4..8] {
        return Err("fullscreen requires real distinct monitor/work rectangles".into());
    }
    let width = first[2].checked_sub(first[0]).ok_or("width overflow")?;
    let height = first[3].checked_sub(first[1]).ok_or("height overflow")?;
    if width < 16 || height < 16 {
        return Err("invalid fullscreen dimensions".into());
    }
    for row in rows {
        if row.len() != 31
            || row[..8] != first[..8]
            || row[12..16] != [0, 0, width, height]
            || row[16..18] != first[..2]
            || row[18] & 0x8000_0000 == 0
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
