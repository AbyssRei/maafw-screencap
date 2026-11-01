// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real Desktop Duplication Map/Unmap/ReleaseFrame records and retained pixels.
use crate::{
    Result,
    framepool_queries::{known, numbers},
    records,
};
use std::{collections::BTreeSet, fs, path::Path};
const HEADER: &str = "cycle\tfault\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\tmapped_result\treleased\ttimeout\tsubresource\tpitch\tthread\tcontext\tresource\tduplication\tmarker";
fn calls(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("Desktop Map header differs".into());
    }
    let mut result = Vec::new();
    let mut caller = None;
    for cycle in 0..20_i64 {
        let mut identities = None;
        for phase in ["probe", "recovery"] {
            let row = numbers(
                lines.next().ok_or("Missing desktop Map call")?,
                Some((2, phase)),
            )?;
            let fault = if phase == "probe" { cycle % 3 } else { 0 };
            let successful = fault == 0;
            if row.len() != 22
                || row[..2] != [cycle, fault]
                || row[3]
                    != if successful {
                        1
                    } else if fault == 1 {
                        0
                    } else {
                        2
                    }
                || row[4] != if fault == 1 { 1235 } else { 123456 }
                || row[5..11] != [1, 1, 1, i64::from(fault != 1), 1, i64::from(fault != 1)]
                || row[11..14]
                    .iter()
                    .any(|v| !(i32::MIN as i64..=i32::MAX as i64).contains(v))
                || row[11] < 0
                || row[13] < 0
                || (row[12] < 0) != (fault == 1)
                || row[14] != 2000
                || row[15] != if fault == 1 { u32::MAX as i64 } else { 0 }
                || (fault == 1 && row[16] != 0)
                || (fault != 1 && !(384..=u32::MAX as i64).contains(&row[16]))
                || !(1..=u32::MAX as i64).contains(&row[17])
                || row[18..21].iter().any(|v| *v <= 0)
                || *caller.get_or_insert(row[17]) != row[17]
                || *identities.get_or_insert(row[18..21].to_vec()) != row[18..21]
                || (successful && ![0, 1].contains(&row[21]))
                || (!successful && row[21] != -1)
            {
                return Err("Desktop Map native order/result/identity differs".into());
            }
            result.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra desktop Map calls".into());
    }
    Ok(result)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace desktop-map <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut names = BTreeSet::new();
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        names.insert(name.to_string());
        inputs.push(records::artifact(&root, &root.join(name))?);
        Ok(fs::read(root.join(name))?)
    };
    let source = calls(std::str::from_utf8(&read("source/map.tsv")?)?)?;
    let rust = calls(std::str::from_utf8(&read("rust/map.tsv")?)?)?;
    for (original, migrated) in source.iter().zip(&rust) {
        if original[..17] != migrated[..17] {
            return Err("Original/Rust desktop native calls differ".into());
        }
    }
    let state = String::from_utf8(read("rust/cpu-copy.tsv")?)?;
    let mut lines = state.lines();
    if lines.next() != Some("cycle\tphase\tsamples\tcopied\tfull_width\tfull_height") {
        return Err("Desktop CPU-copy header differs".into());
    }
    let mut widths = Vec::new();
    for cycle in 0..20_i64 {
        let mut size = None;
        for phase in ["probe", "recovery"] {
            let row = numbers(
                lines.next().ok_or("Missing desktop CPU-copy state")?,
                Some((1, phase)),
            )?;
            if row.len() != 6
                || row[0] != cycle
                || row[2] != 1
                || row[4] < 96
                || row[5] < 64
                || row[4] > i32::MAX as i64
                || row[5] > i32::MAX as i64
                || *size.get_or_insert((row[4], row[5])) != (row[4], row[5])
            {
                return Err("Desktop CPU-copy dimensions/samples differ".into());
            }
            let expected = if phase == "probe" && cycle % 3 == 1 {
                0
            } else {
                row[4]
                    .checked_mul(row[5])
                    .and_then(|n| n.checked_mul(4))
                    .ok_or("Desktop copy span overflow")?
            };
            if row[3] != expected {
                return Err("Desktop safety CPU copy differs".into());
            }
        }
        widths.push(size.unwrap().0);
    }
    if lines.next().is_some() {
        return Err("Extra desktop CPU-copy states".into());
    }
    for (side, rows) in [("source", &source), ("rust", &rust)] {
        for cycle in 0..20_usize {
            for phase in ["warm", "probe", "recovery"] {
                if phase == "probe" && cycle % 3 != 0 {
                    continue;
                }
                let expected_marker = if phase == "warm" {
                    None
                } else {
                    Some(rows[cycle * 2 + usize::from(phase == "recovery")][21])
                };
                let mut first = None;
                for suffix in ["", "-held"] {
                    let name = format!("{side}/{cycle}-{phase}{suffix}");
                    let layout = String::from_utf8(read(&format!("{name}.txt"))?)?;
                    let fields = numbers(layout.trim_end_matches('\n'), None)?;
                    if fields
                        != [
                            96,
                            64,
                            widths[cycle]
                                .checked_mul(3)
                                .ok_or("Desktop BGR stride overflow")?,
                            18432,
                        ]
                    {
                        return Err("Desktop full-output BGR ROI layout differs".into());
                    }
                    let bytes = read(&format!("{name}.bgr"))?;
                    let marker = match bytes.get(..3) {
                        Some([78, 95, 96]) => 0,
                        Some([26, 43, 60]) => 1,
                        _ => return Err("Desktop known marker differs".into()),
                    };
                    if expected_marker.is_some_and(|m| m != marker) {
                        return Err("Desktop record marker differs".into());
                    }
                    known(&bytes, marker)?;
                    if first.as_ref().is_some_and(|prior| *prior != bytes) {
                        return Err("Desktop retained pixels changed".into());
                    }
                    first = Some(bytes);
                }
            }
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected desktop Map directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 desktop Map file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "desktop-map-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected desktop Map root entry".into());
        }
    }
    if names != actual || inputs.len() != 379 {
        return Err("Desktop Map file inventory differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"captures_and_rejections":120,"observed_calls":80,"map_failures":14,"conversion_failures":12,"recovery_captures":40,"retained_images":94,"actual_pixel_files":188,"registered_files":379,"inputs":inputs,"scope":"Real original/Rust acquire/copy/map/unmap/release/conversion boundaries on actual interfaces. A native UINT_MAX-subresource Map rejection and a controlled conversion exception/error preserve cleanup and allow one later recovery. Original expired mapping and Rust full-output safety copy retain distinct ownership semantics. Exported/held target bytes and true full-output BGR ROI strides checked. Not spontaneous device/access loss, complete native resource accounting, warmup behavior, arbitrary formats/failures or full P1 acceptance."});
    let output = root.join("desktop-map-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved desktop Map report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup map_boundary calls=120 observed=80 map_failures=14 conversion_failures=12 recovery=40 retained=94 actual_pixels=188 actual_files=379 full_P1_acceptance=0 passed"
    );
    Ok(())
}
