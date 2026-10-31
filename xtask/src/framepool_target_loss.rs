// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independently validate actual repeated destroyed-target records and pixels.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const HEADER: &str = "cycle\tinvalid_window\trejected_calls\twidth\theight\tfixture_closed\tsource_destroyed\tcaller_thread\tfixture_thread\telapsed_ms";
const STATE: &str = "cycle\tinvalid_calls\tpolls\tcopied\treplayed\tcache_same\tnative_slots\tconversions\tclones\towners_after_capture\towners_after_drop\tcompletion_live\tclose_errors";
fn rows(text: &str, state: bool) -> Result<()> {
    let mut lines = text.lines();
    if lines.next() != Some(if state { STATE } else { HEADER }) {
        return Err("FramePool target-loss header differs".into());
    }
    let mut caller = None;
    for cycle in 0..20_u64 {
        let line = lines
            .next()
            .ok_or("Incomplete FramePool target-loss matrix")?;
        let raw: Vec<_> = line.split('\t').collect();
        let values = raw
            .iter()
            .map(|v| v.parse::<u64>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if values.len() != if state { 13 } else { 10 }
            || values[0] != cycle
            || raw.iter().zip(&values).any(|(s, n)| *s != n.to_string())
        {
            return Err("FramePool target-loss case order/width/scalars differ".into());
        }
        if state {
            if values[1..] != [3, 0, 0, 0, 1, 1, 1, 1, cycle + 2, cycle + 1, 0, 0] {
                return Err("FramePool invalid-target cache/native/CPU owners differ".into());
            }
        } else if values[1..7] != [1, 3, 96, 64, 1, 1]
            || values[7] == 0
            || values[8] == 0
            || values[7] == values[8]
            || u32::try_from(values[7]).is_err()
            || u32::try_from(values[8]).is_err()
            || *caller.get_or_insert(values[7]) != values[7]
        {
            return Err("FramePool destroyed target or native owner identity differs".into());
        }
    }
    if lines.next().is_some() {
        return Err("Extra FramePool target-loss records".into());
    }
    Ok(())
}
fn known(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 96 * 64 * 3 {
        return Err("FramePool target-loss pixel span differs".into());
    }
    for y in 8..56 {
        for x in 8..88 {
            let expected = if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = (y * 96 + x) * 3;
            if bytes[offset..offset + 3] != expected {
                return Err("FramePool target-loss known interior differs".into());
            }
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace framepool-target-loss <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut names = BTreeSet::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        names.insert(name.to_string());
        inputs.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    for side in ["source", "rust"] {
        rows(
            std::str::from_utf8(&read(&format!("{side}/target-loss.tsv"))?)?,
            false,
        )?;
    }
    rows(std::str::from_utf8(&read("rust/rust-state.tsv")?)?, true)?;
    for cycle in 0..20 {
        let mut original = None;
        for side in ["source", "rust"] {
            for suffix in ["", "-held"] {
                let name = format!("{side}/{cycle}{suffix}");
                if read(&format!("{name}.txt"))? != b"96\t64\t288\t18432\n" {
                    return Err("FramePool target-loss layout differs".into());
                }
                let pixels = read(&format!("{name}.bgr"))?;
                known(&pixels)?;
                if original.as_ref().is_some_and(|bytes| *bytes != pixels) {
                    return Err("FramePool original/Rust/retained full pixels differ".into());
                }
                if original.is_none() {
                    original = Some(pixels);
                }
            }
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected FramePool evidence directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 target-loss file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "target-loss-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected FramePool target-loss matrix entry".into());
        }
    }
    if inputs.len() != 163 || actual != names {
        return Err("FramePool target-loss files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"cases":40,"captures":40,
        "rejected_calls":120,"retained_images":40,"actual_pixel_files":80,"registered_files":163,"inputs":inputs,
        "scope":"Actual original/Rust calls after owned target destruction, completed window joins, known interior and full image/retained equality. Rust additionally verifies unchanged private native/cache slots, zero polls/conversions on rejection and final CPU/completion owners. Does not prove original private state introspection, complete GPU resource accounting, arbitrary foreign-window identity reuse, general failures or full P1."});
    let output = root.join("target-loss-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved FramePool target-loss report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool target_loss cases=40 captures=40 rejected_calls=120 retained=40 actual_pixels=80 actual_files=163 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_calls_changed_owner_and_blank_pixels() {
        let matrix = format!("{HEADER}\n")
            + &(0..20)
                .map(|i| format!("{i}\t1\t3\t96\t64\t1\t1\t17\t23\t0\n"))
                .collect::<String>();
        rows(&matrix, false).unwrap();
        assert!(rows(HEADER, false).is_err());
        assert!(rows(&matrix.replacen("\t1\t3\t", "\t1\t2\t", 1), false).is_err());
        assert!(
            rows(
                &matrix.replacen(
                    "19\t1\t3\t96\t64\t1\t1\t17",
                    "19\t1\t3\t96\t64\t1\t1\t29",
                    1
                ),
                false
            )
            .is_err()
        );
        assert!(known(&[]).is_err());
        assert!(known(&vec![0; 96 * 64 * 3]).is_err());
    }
}
