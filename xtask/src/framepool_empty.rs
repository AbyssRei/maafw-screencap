// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual UI-owner zero-client geometry, native empty output and recovery.
use crate::{
    Result,
    framepool_queries::{known, numbers},
    records,
};
use std::{collections::BTreeSet, fs, path::Path};
const HEADER: &str = "cycle\tcase\tphase\toutcome\torder\tclient_width\tclient_height\tmarker\tcaller_thread\tfixture_thread";
const GEOMETRY: &str = "cycle\tphase\tclient_width\tclient_height\touter_width\touter_height\tstyle\towner_timer_errors";
const STATE: &str = "cycle\tphase\tpitch\tcopied\tconversions\tclones\tcache_present\tnative_slots\towners\tcompletion_live\tclose_errors";
const FINAL: &str = "cycle\theld\towners\tcompletion_live\tclose_errors";
const PRESENTATIONS: &str = "cycle\tbefore\tafter\towner_timer_errors";
const PHASES: [&str; 3] = ["warm", "zero", "recovery"];
fn dimensions(case: usize) -> [i64; 2] {
    match case {
        0 => [0, 64],
        1 => [96, 0],
        _ => [0, 0],
    }
}
fn calls(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("FramePool empty-client header differs".into());
    }
    let mut rows = Vec::new();
    let mut caller = None;
    for cycle in 0..20 {
        for phase in PHASES {
            let row = numbers(
                lines.next().ok_or("Missing empty-client call")?,
                Some((2, phase)),
            )?;
            let zero = phase == "zero";
            let size = if zero {
                dimensions(cycle % 3)
            } else {
                [96, 64]
            };
            if row.len() != 10
                || row[..2] != [cycle as i64, (cycle % 3) as i64]
                || row[3] != if zero { 2 } else { 1 }
                || row[4] != 123
                || row[5..7] != size
                || (zero && row[7] != -1)
                || (!zero && ![0, 1].contains(&row[7]))
                || !(1..=u32::MAX as i64).contains(&row[8])
                || !(1..=u32::MAX as i64).contains(&row[9])
                || row[8] == row[9]
                || *caller.get_or_insert(row[8]) != row[8]
            {
                return Err("Empty-client native outcome/query/owner differs".into());
            }
            rows.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra empty-client calls".into());
    }
    Ok(rows)
}
fn geometry(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(GEOMETRY) {
        return Err("Empty-client geometry header differs".into());
    }
    let mut rows = Vec::new();
    for cycle in 0..20 {
        for phase in PHASES {
            let row = numbers(
                lines.next().ok_or("Missing zero-client geometry")?,
                Some((1, phase)),
            )?;
            let zero = phase == "zero";
            let size = if zero {
                dimensions(cycle % 3)
            } else {
                [96, 64]
            };
            if row.len() != 8
                || row[0] != cycle as i64
                || row[2..4] != size
                || row[7] != 0
                || (row[6] & !0x0400_0000) != if zero { 0x9080_0000 } else { 0x9000_0000 }
                || (zero
                    && (row[4] <= size[0]
                        || row[5] <= size[1]
                        || row[4] > i32::MAX as i64
                        || row[5] > i32::MAX as i64))
                || (!zero && row[4..6] != [96, 64])
            {
                return Err("Actual zero/restored client or nonclient rectangle differs".into());
            }
            rows.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra zero-client geometry".into());
    }
    Ok(rows)
}
fn states(text: &str, final_text: &str) -> Result<()> {
    let mut lines = text.lines();
    let mut finals = final_text.lines();
    if lines.next() != Some(STATE) || finals.next() != Some(FINAL) {
        return Err("Empty-client state header differs".into());
    }
    let mut total = 0;
    for cycle in 0..20 {
        for phase in PHASES {
            let zero = phase == "zero";
            if !zero {
                total += 1;
            }
            let n = if phase == "recovery" { 2 } else { 1 };
            let row = numbers(
                lines.next().ok_or("Missing empty-client Rust state")?,
                Some((1, phase)),
            )?;
            if row.len() != 11
                || row[0] != cycle
                || row[2] < 384
                || row[2] > u32::MAX as i64
                || row[2] % 4 != 0
                || row[3..]
                    != [
                        if zero { 0 } else { 24576 },
                        n,
                        n,
                        i64::from(!zero),
                        1,
                        total + i64::from(!zero),
                        0,
                        0,
                    ]
            {
                return Err("Empty-client Map/cache/conversion/native/owner state differs".into());
            }
        }
        if numbers(
            finals.next().ok_or("Missing empty-client final owners")?,
            None,
        )? != [cycle, total, total, 0, 0]
        {
            return Err("Empty-client final owner counts differ".into());
        }
    }
    if lines.next().is_some() || finals.next().is_some() {
        return Err("Extra empty-client states".into());
    }
    Ok(())
}
fn presentations(text: &str) -> Result<()> {
    let mut lines = text.lines();
    if lines.next() != Some(PRESENTATIONS) {
        return Err("Missing prepared restore presentation header".into());
    }
    for cycle in 0..20 {
        let row = numbers(lines.next().ok_or("Missing restore presentation")?, None)?;
        if row.len() != 4
            || row[0] != cycle
            || row[1] <= 0
            || row[2].checked_sub(row[1]).is_none_or(|n| n < 2)
            || row[3] != 0
        {
            return Err("Restored scene did not complete two owner presentations".into());
        }
    }
    if lines.next().is_some() {
        return Err("Extra restore presentation records".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-empty <external matrix> [--check-only]".into(),
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
    let source = calls(std::str::from_utf8(&read("source/empty.tsv")?)?)?;
    let rust = calls(std::str::from_utf8(&read("rust/empty.tsv")?)?)?;
    for (a, b) in source.iter().zip(&rust) {
        if a[..7] != b[..7] {
            return Err("Source/Rust zero-client semantic outcomes differ".into());
        }
    }
    if geometry(std::str::from_utf8(&read("source/geometry.tsv")?)?)?
        != geometry(std::str::from_utf8(&read("rust/geometry.tsv")?)?)?
    {
        return Err("Source/Rust actual zero-client geometry differs".into());
    }
    states(
        std::str::from_utf8(&read("rust/rust-state.tsv")?)?,
        std::str::from_utf8(&read("rust/final.tsv")?)?,
    )?;
    for side in ["source", "rust"] {
        presentations(std::str::from_utf8(&read(&format!(
            "{side}/restore-presentations.tsv"
        ))?)?)?;
    }
    for (side, rows) in [("source", source), ("rust", rust)] {
        for (index, row) in rows.iter().enumerate() {
            let name = format!("{side}/{}-{}", index / 3, PHASES[index % 3]);
            let mut first = None;
            for suffix in ["", "-held"] {
                let layout = read(&format!("{name}{suffix}.txt"))?;
                let expected: &[u8] = if row[3] == 2 {
                    b"0\t0\t0\t0\n"
                } else {
                    b"96\t64\t288\t18432\n"
                };
                if layout != expected {
                    return Err("Empty-client result layout differs".into());
                }
                let pixels = read(&format!("{name}{suffix}.bgr"))?;
                known(&pixels, row[7])?;
                if first.as_ref().is_some_and(|bytes| *bytes != pixels) {
                    return Err("Empty-client held bytes differ".into());
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
                return Err("Unexpected empty-client directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 empty-client path")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "empty-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected empty-client root entry".into());
        }
    }
    if names != actual || inputs.len() != 488 {
        return Err("Empty-client evidence files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":2,"cycles_per_side":20,"calls":120,"actual_zero_calls":40,"recovery_captures":40,"restore_barriers":40,"retained_images":80,"actual_pixel_files":240,"nonempty_pixel_files":160,"registered_files":488,"inputs":inputs,
        "scope":"UI-owner changes real client width/height/both to zero after an actual WGC Map; nonclient rectangle remains nonempty. Original engaged empty Mat and Rust private EmptySample are explicitly mapped to empty outcome. Before recovery the fixture redraws the complete scene and awaits two actual owner presentations; one original capture is then made without retry. Real OS geometry, known/held pixels and Rust conversion/cache/native/CPU/completion counts checked. Immediate-restore failures remain separate and unaccepted; no naturally occurring race, zero-size GraphicsCaptureItem, complete GPU accounting, public SDK or full P1 acceptance."});
    let output = root.join("empty-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved empty-client report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool empty_client calls=120 real_zero=40 recovery=40 restore_barriers=40 retained=80 actual_pixels=240 actual_files=488 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_real_geometry_and_empty_client_calls() {
        assert!(calls(HEADER).is_err());
        assert!(geometry(GEOMETRY).is_err());
        assert!(states(STATE, FINAL).is_err());
    }
}
