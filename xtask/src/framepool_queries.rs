// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Verify complete native query records and every immediate/retained pixel.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const HEADER: &str =
    "cycle\tcase\tphase\tstatus\torder\twin32\tdwm\tmarker\tcaller_thread\tfixture_thread\tticks";
const STATE: &str = "cycle\tphase\tpitch\tcopied\tconversions\tclones\tcache_same\tnative_slots\towners\tcompletion_live\tclose_errors";
const FINAL: &str = "cycle\theld\towners\tcompletion_live\tclose_errors";
const PHASES: [&str; 3] = ["warm", "probe", "recovery"];
fn failure(case: usize) -> bool {
    matches!(case, 1 | 2 | 4)
}
fn numbers(line: &str, text_column: Option<(usize, &str)>) -> Result<Vec<i64>> {
    line.split('\t')
        .enumerate()
        .map(|(i, value)| {
            if let Some((column, expected)) = text_column
                && column == i
            {
                return if value == expected {
                    Ok(0)
                } else {
                    Err("FramePool query phase differs".into())
                };
            }
            let parsed = value.parse::<i64>()?;
            if value != parsed.to_string() {
                return Err("Noncanonical FramePool query scalar".into());
            }
            Ok(parsed)
        })
        .collect()
}
fn rows(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("FramePool query header differs".into());
    }
    let mut caller = None;
    let mut all = Vec::new();
    for cycle in 0..20 {
        for phase in PHASES {
            let row = numbers(
                lines.next().ok_or("Incomplete FramePool query calls")?,
                Some((2, phase)),
            )?;
            let case = if phase == "probe" { cycle % 10 } else { 0 };
            let order = match case {
                1 => 1,
                2 => 12,
                3 | 4 => 1234,
                _ => 123,
            };
            if row.len() != 11
                || row[0] != cycle as i64
                || row[1] != (cycle % 10) as i64
                || row[3] != i64::from(!failure(case))
                || row[4] != order
                || (failure(case) && !(1..=u32::MAX as i64).contains(&row[5]))
                || (!failure(case) && row[5] != 0)
                || (matches!(case, 3 | 4) && !(i32::MIN as i64..0).contains(&row[6]))
                || (!matches!(case, 3 | 4) && row[6] != 0)
                || (failure(case) && row[7] != -1)
                || (!failure(case) && ![0, 1].contains(&row[7]))
                || !(1..=u32::MAX as i64).contains(&row[8])
                || !(1..=u32::MAX as i64).contains(&row[9])
                || row[8] == row[9]
                || row[10] <= 0
                || *caller.get_or_insert(row[8]) != row[8]
            {
                return Err("FramePool query branch/error/owner/presentation differs".into());
            }
            all.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra FramePool query calls".into());
    }
    Ok(all)
}
fn states(text: &str, final_text: &str) -> Result<()> {
    let mut lines = text.lines();
    let mut finals = final_text.lines();
    if lines.next() != Some(STATE) || finals.next() != Some(FINAL) {
        return Err("FramePool query state header differs".into());
    }
    let mut total = 0;
    for cycle in 0..20 {
        let mut successes = 0;
        for phase in PHASES {
            let failed = phase == "probe" && failure(cycle % 10);
            if !failed {
                successes += 1;
                total += 1;
            }
            let row = numbers(
                lines.next().ok_or("Missing FramePool query state")?,
                Some((1, phase)),
            )?;
            if row.len() != 11
                || row[0] != cycle as i64
                || row[2] < 384
                || row[2] > u32::MAX as i64
                || row[2] % 4 != 0
                || row[3..]
                    != [
                        if failed { 0 } else { 24576 },
                        successes,
                        successes,
                        i64::from(failed),
                        1,
                        total + 1,
                        0,
                        0,
                    ]
            {
                return Err("FramePool query map/copy/cache/conversion/owner state differs".into());
            }
        }
        let row = numbers(finals.next().ok_or("Missing FramePool final state")?, None)?;
        if row != [cycle as i64, total, total, 0, 0] {
            return Err("FramePool query final owners differ".into());
        }
    }
    if lines.next().is_some() || finals.next().is_some() {
        return Err("Extra FramePool query states".into());
    }
    Ok(())
}
fn known(bytes: &[u8], marker: i64) -> Result<()> {
    if marker == -1 {
        return if bytes.is_empty() {
            Ok(())
        } else {
            Err("Rejected query leaked pixels".into())
        };
    }
    if bytes.len() != 18432 {
        return Err("FramePool query pixel span differs".into());
    }
    let marker = match marker {
        0 => [0x4e, 0x5f, 0x60],
        1 => [0x1a, 0x2b, 0x3c],
        _ => return Err("Unknown query marker".into()),
    };
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 16 && y < 16 {
                marker
            } else if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = (y * 96 + x) * 3;
            if bytes[offset..offset + 3] != expected {
                return Err("FramePool query known full pixels differ".into());
            }
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-queries <external matrix> [--check-only]".into(),
            );
        }
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
    let original = rows(std::str::from_utf8(&read("source/queries.tsv")?)?)?;
    let rust = rows(std::str::from_utf8(&read("rust/queries.tsv")?)?)?;
    for (a, b) in original.iter().zip(&rust) {
        if a[..7] != b[..7] {
            return Err("Original/Rust query outcomes/native errors differ".into());
        }
    }
    states(
        std::str::from_utf8(&read("rust/rust-state.tsv")?)?,
        std::str::from_utf8(&read("rust/final.tsv")?)?,
    )?;
    for (side, rows) in [("source", original), ("rust", rust)] {
        for (index, row) in rows.iter().enumerate() {
            let name = format!("{side}/{}-{}", index / 3, PHASES[index % 3]);
            let mut first = None;
            for suffix in ["", "-held"] {
                let layout = read(&format!("{name}{suffix}.txt"))?;
                let expected: &[u8] = if row[3] == 0 {
                    b"0\t0\t0\t0\n"
                } else {
                    b"96\t64\t288\t18432\n"
                };
                if layout != expected {
                    return Err("FramePool query layout differs".into());
                }
                let pixels = read(&format!("{name}{suffix}.bgr"))?;
                known(&pixels, row[7])?;
                if first.as_ref().is_some_and(|bytes| *bytes != pixels) {
                    return Err("FramePool query retained pixels changed".into());
                }
                first = Some(pixels);
            }
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected query evidence directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 query file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "query-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected FramePool query entry".into());
        }
    }
    if actual != names || inputs.len() != 484 {
        return Err("FramePool query files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"query_modes":10,"calls":120,"rejected_calls":12,
        "recovery_captures":40,"retained_images":108,"actual_pixel_files":240,"nonempty_pixel_files":216,"registered_files":484,"inputs":inputs,
        "scope":"Actual original/Rust WGC mapping with four test-only Win32 query interpositions: native invalid-handle failures, DWM fallback and five controlled rectangle clamps. Every successful image has full known pixels; failed calls expose no pixels; immediate/held bytes match. Rust also verifies warm cache retention on failure, real recovery mapping, conversion counts, native slots and CPU/completion cleanup. Query faults/rectangles are fixture interventions, not actual foreign-window races, complete GPU accounting or full P1."});
    let output = root.join("query-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved FramePool query report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool queries calls=120 rejected=12 recovery=40 retained=108 actual_pixels=240 actual_files=484 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_calls_and_undefined_failure_pixels() {
        assert!(rows(HEADER).is_err());
        assert!(numbers("01\tprobe", Some((1, "probe"))).is_err());
        assert!(numbers("1\twarm", Some((1, "probe"))).is_err());
        assert!(known(&[], -1).is_ok());
        assert!(known(&[0], -1).is_err());
        assert!(known(&vec![0; 18432], 0).is_err());
    }
}
