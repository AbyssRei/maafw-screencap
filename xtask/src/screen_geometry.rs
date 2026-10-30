// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recorded native geometry and independent known pixels, not desktop execution.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

const STAGES: [&str; 5] = ["setup", "corrected", "presented", "capture-0", "capture-1"];
type Snapshot = [i64; 31];

#[derive(Serialize)]
struct Report {
    layout_version: u32,
    cases: usize,
    native_sides: usize,
    capture_files: usize,
    snapshots: usize,
    resource_rows: usize,
    inputs: Vec<records::Artifact>,
    scope: &'static str,
}
pub fn run(args: &[String]) -> Result<()> {
    run_backend(args, false)
}
pub fn run_desktop(args: &[String]) -> Result<()> {
    run_backend(args, true)
}
fn run_backend(args: &[String], desktop: bool) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace screen-geometry <external directory> [--check-only]"
                    .into(),
            );
        }
    };
    let directory = records::external_root(Path::new(path))?;
    let report = validate(&directory, desktop)?;
    let path = directory.join("geometry-validation.json");
    if check_only && path.exists() {
        if records::read::<serde_json::Value>(&path)? != serde_json::to_value(&report)? {
            return Err("saved ScreenDC geometry validation differs from actual inputs".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    if desktop {
        println!(
            "DesktopDupWindow native geometry cases=38 sides=76 captures=152 snapshots=380 known_pixels=1 complete_positions=1 retained_files=152 native_roi_stride=1 image_owners_released=1 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "ScreenDC native geometry cases=38 sides=76 captures=152 snapshots=380 known_pixels=1 complete_positions=1 passed"
        );
    }
    Ok(())
}
fn number(value: &str) -> Result<i64> {
    let result: i64 = value.parse()?;
    if result.to_string() != value {
        return Err("noncanonical geometry integer".into());
    }
    Ok(result)
}
fn table(text: &str, count: usize) -> Result<Vec<Vec<&str>>> {
    text.lines()
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != count {
                return Err("unexpected geometry record width".into());
            }
            Ok(fields)
        })
        .collect()
}
fn screen(row: &Snapshot) -> Result<[i64; 4]> {
    Ok([
        row[16]
            .checked_add(row[12])
            .ok_or("client coordinate overflow")?,
        row[17]
            .checked_add(row[13])
            .ok_or("client coordinate overflow")?,
        row[16]
            .checked_add(row[14])
            .ok_or("client coordinate overflow")?,
        row[17]
            .checked_add(row[15])
            .ok_or("client coordinate overflow")?,
    ])
}
fn target(row: &Snapshot) -> Result<[i64; 4]> {
    let client = screen(row)?;
    if row[18] & 0x8000_0000 != 0 && client == row[0..4] {
        return Ok(client);
    }
    let width = (client[2] - client[0]).min(row[6] - row[4]);
    let height = (client[3] - client[1]).min(row[7] - row[5]);
    if width <= 0 || height <= 0 {
        return Err("empty geometry rectangle".into());
    }
    let left = client[0].clamp(row[4], row[6] - width);
    let top = client[1].clamp(row[5], row[7] - height);
    Ok([left, top, left + width, top + height])
}
fn compare(before: &Snapshot, after: &Snapshot) -> Result<[i64; 4]> {
    let desired = target(before)?;
    let client = screen(before)?;
    let changed = i64::from(desired != client);
    let outer = [
        desired[0] - (client[0] - before[8]),
        desired[1] - (client[1] - before[9]),
        desired[2] + (before[10] - client[2]),
        desired[3] + (before[11] - client[3]),
    ];
    if after[0..8] != before[0..8]
        || after[12..14] != before[12..14]
        || after[18..20] != before[18..20]
        || after[26..31] != before[26..31]
        || screen(after)? != desired
        || after[8..12] != outer
        || after[20]
            != before[20]
                .checked_add(changed)
                .ok_or("geometry message count overflow")?
    {
        return Err("native correction differs from complete expected client/outer rectangle or message count".into());
    }
    if changed != 0
        && (after[21..25] != [outer[0], outer[1], outer[2] - outer[0], outer[3] - outer[1]]
            || after[25] & 0x14 != 0x14
            || after[25] & 3 != 0)
    {
        return Err("native WINDOWPOS request differs from complete expected tuple/flags".into());
    }
    Ok(desired)
}
fn extended_style_matches(before: i64, after: i64, stage: &str) -> bool {
    // Pinned Base/ForegroundUtils.h may end its foreground recovery with
    // HWND_NOTOPMOST. That original side effect is retained by Rust. Requiring
    // host-style restoration here would incorrectly impose the later SDK rule.
    after == before || (stage.starts_with("capture-") && after == before & !8)
}
fn expected_cases(monitor: [i64; 4], work: [i64; 4]) -> Vec<(String, i64, [i64; 4])> {
    let popup = 0x9000_0000;
    let ordinary = 0x10cf_0000;
    let mut cases = vec![("exact_popup".into(), popup, monitor)];
    for edge in 0..4 {
        for delta in [-9, -8, -7, -1, 1, 7, 8, 9] {
            let mut client = monitor;
            client[edge] += delta;
            cases.push((format!("edge_{edge}_{delta}"), popup, client));
        }
    }
    cases.extend([
        ("exact_bordered_popup".into(), popup | 0x40000, monitor),
        ("ordinary_monitor".into(), ordinary, monitor),
        ("ordinary_work".into(), ordinary, work),
        (
            "review_negative_one_translation".into(),
            popup,
            [monitor[0] - 1, monitor[1], monitor[2] - 1, monitor[3]],
        ),
        (
            "oversized_popup".into(),
            popup,
            [
                monitor[0] - 9,
                monitor[1] - 9,
                monitor[2] + 9,
                monitor[3] + 9,
            ],
        ),
    ]);
    cases
}
fn validate(root: &Path, desktop: bool) -> Result<Report> {
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<String> {
        inputs.push(records::artifact(root, &root.join(name))?);
        let text = fs::read_to_string(root.join(name))?;
        if !text.ends_with('\n') {
            return Err("incomplete geometry record".into());
        }
        Ok(text)
    };
    let (case_text, snapshot_text, resource_text) = if desktop {
        let cases = read("source-cases.tsv")?;
        if read("rust-cases.tsv")? != cases {
            return Err("source/Rust case definitions differ".into());
        }
        let snapshots = read("source-snapshots.tsv")? + &read("rust-snapshots.tsv")?;
        let source = read("source-resources.tsv")?;
        let rust = read("rust-resources.tsv")?;
        if source.lines().count() != 38 || rust.lines().count() != 38 {
            return Err("incomplete separate resource records".into());
        }
        let resources = source
            .lines()
            .zip(rust.lines())
            .map(|(s, r)| format!("{s}\n{r}\n"))
            .collect();
        (cases, snapshots, resources)
    } else {
        (
            read("cases.tsv")?,
            read("snapshots.tsv")?,
            read("resources.tsv")?,
        )
    };
    let cases = table(&case_text, 7)?;
    let rows = table(&snapshot_text, 34)?;
    let resources = table(&resource_text, 4)?;
    if cases.len() != 38 || rows.len() != 380 || resources.len() != 76 {
        return Err("native ScreenDC geometry matrix is incomplete".into());
    }
    let mut snapshots = BTreeMap::new();
    for row in &rows {
        let index = usize::try_from(number(row[0])?)?;
        if index >= 38 || !["source", "rust"].contains(&row[1]) || !STAGES.contains(&row[2]) {
            return Err("unknown geometry snapshot identity".into());
        }
        let mut values = [0; 31];
        for (index, value) in row[3..].iter().enumerate() {
            values[index] = number(value)?;
            if index < 18 {
                i32::try_from(values[index])?;
            }
        }
        for index in [18, 19, 25, 26] {
            u32::try_from(values[index])?;
        }
        if values[20] < 0
            || values[27..29] != [1, 0]
            || values[29] <= 0
            || values[30] <= 0
            || values[26] == 0
            || snapshots.insert((index, row[1], row[2]), values).is_some()
        {
            return Err("invalid or duplicate native geometry snapshot".into());
        }
    }
    let first = snapshots
        .get(&(0, "source", "setup"))
        .ok_or("initial geometry snapshot missing")?;
    let definitions = expected_cases(first[0..4].try_into()?, first[4..8].try_into()?);
    if desktop {
        desktop_records(root, &snapshots, &mut inputs)?;
    }
    for (index, definition) in cases.iter().enumerate() {
        let (name, style, rectangle) = &definitions[index];
        if usize::try_from(number(definition[0])?)? != index
            || definition[1] != name
            || number(definition[2])? != *style
            || definition[3..]
                .iter()
                .map(|v| number(v))
                .collect::<Result<Vec<_>>>()?
                != rectangle.as_slice()
        {
            return Err("geometry case differs from the complete declared matrix".into());
        }
        for (side_index, side) in ["source", "rust"].into_iter().enumerate() {
            let before = snapshots
                .get(&(index, side, "setup"))
                .ok_or("setup snapshot missing")?;
            let corrected = snapshots
                .get(&(index, side, "corrected"))
                .ok_or("corrected snapshot missing")?;
            // The OS adds clipping style bits to owned top-level windows.
            // Keep the requested popup/frame/visibility bits exact; retain the
            // complete observed DWORD in the raw record.
            if screen(before)? != *rectangle
                || before[18] & !0x0600_0000 != *style
                || before[0..8] != first[0..8]
                || before[29] != before[10] - before[8]
                || before[30] != before[11] - before[9]
            {
                return Err(
                    "native fixture geometry/style/monitor differs from declared premises".into(),
                );
            }
            let target = compare(before, corrected)?;
            for stage in &STAGES[2..] {
                let row = snapshots
                    .get(&(index, side, *stage))
                    .ok_or("capture geometry snapshot missing")?;
                if row[0..19] != corrected[0..19]
                    || !extended_style_matches(corrected[19], row[19], stage)
                    || row[20] != corrected[20]
                    || row[21..26] != corrected[21..26]
                    || row[26..31] != corrected[26..31]
                {
                    return Err(
                        "native geometry changed during presentation/repeated capture".into(),
                    );
                }
            }
            let resource = &resources[index
                .checked_mul(2)
                .and_then(|n| n.checked_add(side_index))
                .ok_or("resource index overflow")?];
            if usize::try_from(number(resource[0])?)? != index
                || resource[1] != side
                || number(resource[2])? < 0
                || number(resource[3])? < 0
                || number(resource[3])? > number(resource[2])?
            {
                return Err("native fixture resource identity or release counts differ".into());
            }
            for cycle in 0..2 {
                let name = format!("{index}-{side}-{cycle}");
                let metadata = fs::read_to_string(root.join(format!("{name}.txt")))?;
                let values = table(&metadata, 4)?;
                if values.len() != 1 {
                    return Err("extra image metadata rows".into());
                }
                let data = values[0]
                    .iter()
                    .map(|v| number(v))
                    .collect::<Result<Vec<_>>>()?;
                let width = usize::try_from(target[2] - target[0])?;
                let height = usize::try_from(target[3] - target[1])?;
                let stride = width.checked_mul(3).ok_or("image stride overflow")?;
                let length = stride.checked_mul(height).ok_or("image length overflow")?;
                if data
                    != [
                        i64::try_from(width)?,
                        i64::try_from(height)?,
                        i64::try_from(stride)?,
                        i64::try_from(length)?,
                    ]
                {
                    return Err("actual BGR metadata differs from native target".into());
                }
                let bytes = fs::read(root.join(format!("{name}.bgr")))?;
                if bytes.len() != length {
                    return Err("actual BGR readable length differs".into());
                }
                let marker = if desktop {
                    let marker = bytes.get(..3).ok_or("missing in-target marker")?;
                    if marker != [0x1a, 0x2b, 0x3c] && marker != [0x4e, 0x5f, 0x60] {
                        return Err("unknown native marker palette".into());
                    }
                    Some(marker)
                } else {
                    None
                };
                for (y, row) in bytes.chunks_exact(stride).enumerate() {
                    for (x, pixel) in row.as_chunks::<3>().0.iter().enumerate() {
                        let expected: &[u8] = if let Some(marker) = marker
                            && x < 16
                            && y < 16
                        {
                            marker
                        } else if x < width / 3 {
                            &[0x1f, 0x53, 0xa7]
                        } else if y < height / 2 {
                            &[0xb9, 0x4d, 0x27]
                        } else {
                            &[0x3b, 0xe2, 0x81]
                        };
                        if pixel != expected {
                            return Err(
                                format!("native known pixels differ: {name} ({x},{y})").into()
                            );
                        }
                    }
                }
                for extension in ["bgr", "txt"] {
                    inputs.push(records::artifact(
                        root,
                        &root.join(format!("{name}.{extension}")),
                    )?);
                }
                if desktop {
                    let held = root.join(format!("{name}.held.bgr"));
                    if fs::read(&held)? != bytes {
                        return Err("held target differs after native/window close".into());
                    }
                    inputs.push(records::artifact(root, &held)?);
                }
            }
        }
    }
    let mut expected_files = inputs
        .iter()
        .map(|input| input.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    expected_files.insert("geometry-validation.json");
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 geometry artifact name")?;
        if !expected_files.contains(name.as_str()) || !entry.file_type()?.is_file() {
            return Err("unexpected native geometry record or nested directory".into());
        }
    }
    Ok(Report {
        layout_version: 1,
        cases: 38,
        native_sides: 76,
        capture_files: 152,
        snapshots: 380,
        resource_rows: 76,
        inputs,
        scope: if desktop {
            "Native DesktopDupWindow post-correction 38-case geometry, original source twice and Rust twice per case in separate executions, complete target/marker pixels, actual full-output source ROI stride versus packed exported/Rust rows, held bytes after native/window close and Rust image owner counts. Original default warmup/wait/full conversion/crop remain. Does not accept first-capture asynchronous correction races, all DPI/taskbar/output/resource faults, original mapped-view safety or full P1."
        } else {
            "Native fixture geometry and independently checked recorded BGR pixels; test execution separately asserts CPU owners after close. Does not validate DesktopDupWindow, other taskbar/DPI/monitor environments, full call chains or P1 acceptance."
        },
    })
}

fn desktop_records(
    root: &Path,
    snapshots: &BTreeMap<(usize, &str, &str), Snapshot>,
    inputs: &mut Vec<records::Artifact>,
) -> Result<()> {
    let mut read = |name: &str| -> Result<String> {
        inputs.push(records::artifact(root, &root.join(name))?);
        let text = fs::read_to_string(root.join(name))?;
        if !text.ends_with('\n') {
            return Err("incomplete desktop native record".into());
        }
        Ok(text)
    };
    for side in ["source", "rust"] {
        let text = read(&format!("{side}-markers.tsv"))?;
        let rows = table(&text, 5)?;
        if rows.len() != 38 {
            return Err("incomplete desktop marker records".into());
        }
        for (index, row) in rows.iter().enumerate() {
            if usize::try_from(number(row[0])?)? != index
                || row[1] != side
                || row[2].parse::<u64>()? == 0
                || row[3].parse::<u64>()? == 0
                || number(row[4])? != 0
            {
                return Err("native marker identity/presentation/error differs".into());
            }
        }
    }
    let text = read("source-native-layouts.tsv")?;
    let rows = table(&text, 7)?;
    if rows.len() != 76 {
        return Err("incomplete actual source ROI layouts".into());
    }
    for (ordinal, row) in rows.iter().enumerate() {
        let index = ordinal / 2;
        let values = row.iter().map(|v| number(v)).collect::<Result<Vec<_>>>()?;
        let snapshot = snapshots
            .get(&(index, "source", "corrected"))
            .ok_or("source corrected snapshot missing")?;
        let target = screen(snapshot)?;
        let width = target[2] - target[0];
        let height = target[3] - target[1];
        let native_step = (snapshot[2] - snapshot[0])
            .checked_mul(3)
            .ok_or("native row overflow")?;
        let packed = width.checked_mul(3).ok_or("packed row overflow")?;
        let span = packed.checked_mul(height).ok_or("packed span overflow")?;
        if values
            != [
                i64::try_from(index)?,
                i64::try_from(ordinal % 2)?,
                width,
                height,
                native_step,
                packed,
                span,
            ]
        {
            return Err("actual original ROI step or packed file layout differs".into());
        }
    }
    let text = read("rust-owners.tsv")?;
    let rows = table(&text, 4)?;
    if rows.len() != 38 {
        return Err("incomplete target owner records".into());
    }
    for (index, row) in rows.iter().enumerate() {
        let values = row.iter().map(|v| number(v)).collect::<Result<Vec<_>>>()?;
        if values[0] != i64::try_from(index)?
            || values[1] < 0
            || values[2] != values[1].checked_add(2).ok_or("owner overflow")?
            || values[3] != values[1]
        {
            return Err("independent target owner retention/release differs".into());
        }
    }
    if read("rust-result.txt")?
        != "cases=38\ncaptures=76\nretained_after_close=76\nknown_pixels=1\nrepeated_geometry=0\nimage_owners_released=1\n"
    {
        return Err("desktop geometry completion missing".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_a_contained_but_incorrectly_shrunken_native_client() {
        let mut before = [0; 31];
        before[0..8].copy_from_slice(&[0, 0, 1920, 1080, 0, 0, 1920, 1032]);
        before[8..16].copy_from_slice(&[-1, 0, 1919, 1080, 0, 0, 1920, 1080]);
        before[16] = -1;
        before[18] = 0x9000_0000;
        let mut after = before;
        after[8..16].copy_from_slice(&[0, 0, 1920, 1032, 0, 0, 1920, 1032]);
        after[16] = 0;
        after[20] = 1;
        after[21..26].copy_from_slice(&[0, 0, 1920, 1032, 0x4014]);
        compare(&before, &after).unwrap();
        after[10] = 1;
        after[11] = 1;
        after[14] = 1;
        after[15] = 1;
        assert!(compare(&before, &after).is_err());
    }
    #[test]
    fn rejects_noncanonical_integers_and_extra_columns() {
        for value in ["+1", "01", "-0", " 1", "9223372036854775808"] {
            assert!(number(value).is_err());
        }
        assert!(table("0\tsource\textra\t0\t0\n", 4).is_err());
    }
    #[test]
    fn preserves_source_topmost_release_without_accepting_other_style_changes() {
        assert!(extended_style_matches(264, 256, "capture-0"));
        assert!(extended_style_matches(264, 264, "capture-1"));
        assert!(!extended_style_matches(264, 256, "presented"));
        assert!(!extended_style_matches(256, 264, "capture-0"));
        assert!(!extended_style_matches(264, 0, "capture-0"));
    }
}
