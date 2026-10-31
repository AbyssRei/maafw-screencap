// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual native items with explicit controlled Size outcomes and partial state.
use crate::{
    Result,
    framepool_queries::{known, numbers},
    records,
};
use std::{collections::BTreeSet, fs, path::Path};
const HEADER: &str = "cycle\tcase\tphase\toutcome\terror\tsize_calls\tbefore_slots\tafter_slots\tlast_width\tlast_height\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread";
const QUERY: &str = "cycle\tphase\tordinal\tnative_hresult\tnative_width\tnative_height\treturned_hresult\treturned_width\treturned_height\tcaller_thread\titem";
const STATE: &str =
    "cycle\tphase\terror_code\tcopied\tconversions\tclones\towners\tcompletion_live\tclose_errors";
const FINAL: &str = "cycle\theld\towners\tcompletion_live\tclose_errors";
const PHASES: [&str; 3] = ["warm", "probe", "recovery"];
const AFTER: [i64; 10] = [127, 23, 7, 119, 7, 23, 119, 23, 7, 7];
const CALLS: [usize; 10] = [1, 1, 2, 3, 1, 3, 4, 2, 1, 2];
const INJECTED: i64 = 0x8000_4004_u32 as i32 as i64;
fn exceptional(mode: usize) -> bool {
    matches!(mode, 1 | 3 | 6 | 7)
}
fn calls(text: &str, query_text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    let mut queries = query_text.lines();
    if lines.next() != Some(HEADER) || queries.next() != Some(QUERY) {
        return Err("Size boundary headers differ".into());
    }
    let mut rows = Vec::new();
    let mut caller = None;
    for cycle in 0..20 {
        let mode = cycle % 10;
        for phase in PHASES {
            let failed = phase == "probe" && mode != 0;
            let count = if phase == "warm" {
                4
            } else if phase == "probe" {
                CALLS[mode]
            } else if mode == 0 {
                1
            } else {
                4
            };
            let row = numbers(
                lines.next().ok_or("Missing Size boundary call")?,
                Some((2, phase)),
            )?;
            let before = if phase == "warm" {
                0
            } else if phase == "probe" {
                if matches!(mode, 1..=3 | 8 | 9) {
                    7
                } else {
                    127
                }
            } else {
                AFTER[mode]
            };
            if row.len() != 15
                || row[..2] != [cycle as i64, mode as i64]
                || row[3]
                    != if failed {
                        if exceptional(mode) { 2 } else { 0 }
                    } else {
                        1
                    }
                || row[4]
                    != if failed && exceptional(mode) {
                        INJECTED
                    } else {
                        0
                    }
                || row[5] != count as i64
                || row[6] != before
                || row[7] != if failed { AFTER[mode] } else { 127 }
                || row[8..12]
                    != [
                        if failed { 0 } else { 96 },
                        if failed { 0 } else { 64 },
                        i64::from(failed),
                        1,
                    ]
                || !if failed {
                    row[12] == -1
                } else {
                    [0, 1].contains(&row[12])
                }
                || !(1..=u32::MAX as i64).contains(&row[13])
                || !(1..=u32::MAX as i64).contains(&row[14])
                || row[13] == row[14]
                || *caller.get_or_insert(row[13]) != row[13]
            {
                return Err("Size catch/partial native state/cache/thread differs".into());
            }
            let fault = if phase != "probe" {
                0
            } else {
                match mode {
                    1..=3 => mode,
                    4 => 1,
                    5 => 3,
                    6 => 4,
                    7 => 2,
                    _ => 0,
                }
            };
            for ordinal in 1..=count {
                let query = numbers(
                    queries.next().ok_or("Missing actual Size query")?,
                    Some((1, phase)),
                )?;
                let code = if ordinal == fault { INJECTED } else { 0 };
                let width = if code != 0
                    || phase == "probe"
                        && ((mode == 8 && ordinal == 1) || (mode == 9 && ordinal == 2))
                {
                    0
                } else if phase == "probe" && (5..=7).contains(&mode) && ordinal == 1 {
                    97
                } else {
                    96
                };
                if query.len() != 11
                    || query[..1] != [cycle as i64]
                    || query[2..9]
                        != [
                            ordinal as i64,
                            0,
                            96,
                            64,
                            code,
                            width,
                            if code != 0 { 0 } else { 64 },
                        ]
                    || query[9] != row[13]
                    || query[10] <= 0
                {
                    return Err("Size actual native result/intervention/channel differs".into());
                }
            }
            rows.push(row);
        }
    }
    if lines.next().is_some() || queries.next().is_some() {
        return Err("Extra Size calls/queries".into());
    }
    Ok(rows)
}
fn states(text: &str, final_text: &str) -> Result<()> {
    let mut lines = text.lines();
    let mut finals = final_text.lines();
    if lines.next() != Some(STATE) || finals.next() != Some(FINAL) {
        return Err("Size state headers differ".into());
    }
    let mut total = 0;
    for cycle in 0..20 {
        let mode = cycle % 10;
        let mut successes = 0;
        for phase in PHASES {
            let failed = phase == "probe" && mode != 0;
            if !failed {
                total += 1;
                successes += 1;
            }
            let row = numbers(
                lines.next().ok_or("Missing Size Rust state")?,
                Some((1, phase)),
            )?;
            let code = if !failed || mode == 8 {
                0
            } else if mode == 9 {
                0x8007_0057_u32 as i32 as i64
            } else {
                INJECTED
            };
            if row.len() != 9
                || row[0] != cycle as i64
                || row[2..]
                    != [
                        code,
                        if failed { 0 } else { 24576 },
                        successes,
                        successes,
                        total + 1,
                        0,
                        0,
                    ]
            {
                return Err("Size Rust error/copy/conversion/owner differs".into());
            }
        }
        if numbers(finals.next().ok_or("Missing Size final owners")?, None)?
            != [cycle as i64, total, total, 0, 0]
        {
            return Err("Size final owners differ".into());
        }
    }
    if lines.next().is_some() || finals.next().is_some() {
        return Err("Extra Size Rust state".into());
    }
    Ok(())
}
fn marker(bytes: &[u8]) -> Result<i64> {
    match bytes.get(..3) {
        Some([0x4e, 0x5f, 0x60]) => Ok(0),
        Some([0x1a, 0x2b, 0x3c]) => Ok(1),
        _ => Err("Size cache marker differs".into()),
    }
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-size <external matrix> [--check-only]".into(),
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
    let source = calls(
        std::str::from_utf8(&read("source/size.tsv")?)?,
        std::str::from_utf8(&read("source/queries.tsv")?)?,
    )?;
    let rust = calls(
        std::str::from_utf8(&read("rust/size.tsv")?)?,
        std::str::from_utf8(&read("rust/queries.tsv")?)?,
    )?;
    for (a, b) in source.iter().zip(&rust) {
        if a[..12] != b[..12] {
            return Err("Original/Rust Size catch/partial state differs".into());
        }
    }
    states(
        std::str::from_utf8(&read("rust/rust-state.tsv")?)?,
        std::str::from_utf8(&read("rust/final.tsv")?)?,
    )?;
    for (side, rows) in [("source", source), ("rust", rust)] {
        let mut prior = None;
        for (i, row) in rows.iter().enumerate() {
            if i % 3 == 0 {
                prior = None;
            }
            let name = format!("{side}/{}-{}", i / 3, PHASES[i % 3]);
            let mut first = None;
            for suffix in ["", "-held"] {
                let layout: &[u8] = if row[3] == 1 {
                    b"96\t64\t288\t18432\n"
                } else {
                    b"0\t0\t0\t0\n"
                };
                if read(&format!("{name}{suffix}.txt"))? != layout {
                    return Err("Size output layout differs".into());
                }
                let data = read(&format!("{name}{suffix}.bgr"))?;
                known(&data, row[12])?;
                if first.as_ref().is_some_and(|bytes| *bytes != data) {
                    return Err("Size retained pixels differ".into());
                }
                first = Some(data);
            }
            if read(&format!("{name}-cache.txt"))? != b"96\t64\t288\t18432\n" {
                return Err("Size cache layout differs".into());
            }
            let cache = read(&format!("{name}-cache.bgr"))?;
            known(&cache, marker(&cache)?)?;
            if row[3] == 1 {
                if first.as_ref() != Some(&cache) {
                    return Err("Size output/cache bytes differ".into());
                }
            } else if prior.as_ref() != Some(&cache) {
                return Err("Size failure replaced cache".into());
            }
            prior = Some(cache);
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected Size directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 Size file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "size-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected Size root entry".into());
        }
    }
    if actual != names || inputs.len() != 726 {
        return Err("Size matrix files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"calls":120,"rejected_calls":36,"recovery_captures":40,"retained_images":84,"actual_size_queries":388,"actual_pixel_files":360,"nonempty_pixel_files":288,"registered_files":726,"inputs":inputs,"scope":"Real GraphicsCaptureItems are queried successfully before explicit Size result/exception interventions. Four original Size call sites and original catch boundaries are preserved in a generated source copy. Only the separate test header makes class fields accessible for direct scalar state observations. Cold-init versus size-change-init failures preserve distinct native slots; zero pool-size arguments exercise actual native Create validation. Old cache remains independent and unchanged; recovery is one capture without retries; held CPU bytes survive native close. Rust additionally records typed errors, copy/conversion and final CPU/completion owners. Not an actual native Size failure, real window resize/race, full GPU accounting, allocator/device fault or full P1 acceptance."});
    let output = root.join("size-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Size report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool size_boundary calls=120 rejected=36 recovery=40 retained=84 actual_queries=388 actual_pixels=360 actual_files=726 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_size_calls_and_state() {
        assert!(calls(HEADER, QUERY).is_err());
        assert!(states(STATE, FINAL).is_err());
        assert!(marker(&[]).is_err());
    }
}
