// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Verify actual Map/Unmap, conversion, cache and retained-pixel records.
use crate::{
    Result,
    framepool_queries::{known, numbers},
    records,
};
use std::{collections::BTreeSet, fs, path::Path};
const HEADER: &str = "cycle\tcase\tphase\toutcome\torder\tmaps\tunmaps\tconversions\tcache_clones\thresult\tsubresource\tpitch\tmapped\tviolation\tquery_order\tcache_preserved\tcache_independent\tmarker\tcaller_thread\tfixture_thread\tcontext\tresource";
const STATE: &str =
    "cycle\tphase\tcopied\tconversions\tclones\towners\tcompletion_live\tclose_errors";
const FINAL: &str = "cycle\theld\towners\tcompletion_live\tclose_errors";
const PHASES: [&str; 3] = ["warm", "probe", "recovery"];
fn calls(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("Map-boundary header differs".into());
    }
    let mut rows = Vec::new();
    let mut caller = None;
    for cycle in 0..20 {
        let mut context = None;
        let mut resource = None;
        for phase in PHASES {
            let row = numbers(
                lines.next().ok_or("Missing Map-boundary call")?,
                Some((2, phase)),
            )?;
            let fault = if phase == "probe" { cycle % 3 } else { 0 };
            let outcome = match fault {
                1 => 0,
                2 => 2,
                _ => 1,
            };
            let order = match fault {
                1 => 12,
                2 => 1235,
                _ => 12345,
            };
            if row.len() != 22
                || row[..2] != [cycle as i64, (cycle % 3) as i64]
                || row[3] != outcome
                || row[4] != order
                || row[5..9]
                    != [
                        1,
                        i64::from(fault != 1),
                        i64::from(fault != 1),
                        i64::from(fault == 0),
                    ]
                || !(i32::MIN as i64..=i32::MAX as i64).contains(&row[9])
                || (fault == 1 && row[9] >= 0)
                || (fault != 1 && row[9] < 0)
                || row[10] != if fault == 1 { u32::MAX as i64 } else { 0 }
                || (fault == 1 && row[11] != 0)
                || (fault != 1 && (row[11] < 384 || row[11] > u32::MAX as i64 || row[11] % 4 != 0))
                || row[12..17]
                    != [
                        0,
                        0,
                        if fault == 1 { 0 } else { 123 },
                        i64::from(fault != 0),
                        1,
                    ]
                || (fault != 0 && row[17] != -1)
                || (fault == 0 && ![0, 1].contains(&row[17]))
                || !(1..=u32::MAX as i64).contains(&row[18])
                || !(1..=u32::MAX as i64).contains(&row[19])
                || row[18] == row[19]
                || *caller.get_or_insert(row[18]) != row[18]
                || row[20] <= 0
                || row[21] <= 0
                || *context.get_or_insert(row[20]) != row[20]
                || *resource.get_or_insert(row[21]) != row[21]
            {
                return Err("Map/unmap/conversion/cache/thread/native identity differs".into());
            }
            rows.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra Map-boundary calls".into());
    }
    Ok(rows)
}
fn states(text: &str, final_text: &str) -> Result<()> {
    let mut lines = text.lines();
    let mut finals = final_text.lines();
    if lines.next() != Some(STATE) || finals.next() != Some(FINAL) {
        return Err("Map-boundary Rust state header differs".into());
    }
    let mut total = 0;
    for cycle in 0..20 {
        let mut successes = 0;
        for phase in PHASES {
            let fault = if phase == "probe" { cycle % 3 } else { 0 };
            if fault == 0 {
                successes += 1;
                total += 1;
            }
            let row = numbers(
                lines.next().ok_or("Missing Map-boundary Rust state")?,
                Some((1, phase)),
            )?;
            if row.len() != 8
                || row[0] != cycle
                || row[2..]
                    != [
                        if fault == 1 { 0 } else { 24576 },
                        successes,
                        successes,
                        total + 1,
                        0,
                        0,
                    ]
            {
                return Err("Map-boundary copy/conversion/CPU/completion state differs".into());
            }
        }
        if numbers(
            finals.next().ok_or("Missing Map-boundary final owner")?,
            None,
        )? != [cycle, total, total, 0, 0]
        {
            return Err("Map-boundary final owners differ".into());
        }
    }
    if lines.next().is_some() || finals.next().is_some() {
        return Err("Extra Map-boundary states".into());
    }
    Ok(())
}
fn marker(bytes: &[u8]) -> Result<i64> {
    match bytes.get(..3) {
        Some([0x4e, 0x5f, 0x60]) => Ok(0),
        Some([0x1a, 0x2b, 0x3c]) => Ok(1),
        _ => Err("Unknown map-cache marker".into()),
    }
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-map <external matrix> [--check-only]".into(),
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
    let source = calls(std::str::from_utf8(&read("source/map.tsv")?)?)?;
    let rust = calls(std::str::from_utf8(&read("rust/map.tsv")?)?)?;
    for (a, b) in source.iter().zip(&rust) {
        if a[..11] != b[..11] || a[12..17] != b[12..17] {
            return Err("Original/Rust Map-boundary outcomes/calls/native HRESULT differ".into());
        }
    }
    states(
        std::str::from_utf8(&read("rust/rust-state.tsv")?)?,
        std::str::from_utf8(&read("rust/final.tsv")?)?,
    )?;
    for (side, rows) in [("source", source), ("rust", rust)] {
        let mut prior_cache = None;
        for (index, row) in rows.iter().enumerate() {
            if index % 3 == 0 {
                prior_cache = None;
            }
            let name = format!("{side}/{}-{}", index / 3, PHASES[index % 3]);
            let mut first = None;
            for suffix in ["", "-held"] {
                let expected: &[u8] = if row[3] == 1 {
                    b"96\t64\t288\t18432\n"
                } else {
                    b"0\t0\t0\t0\n"
                };
                if read(&format!("{name}{suffix}.txt"))? != expected {
                    return Err("Map-boundary output layout differs".into());
                }
                let pixels = read(&format!("{name}{suffix}.bgr"))?;
                known(&pixels, row[17])?;
                if first.as_ref().is_some_and(|bytes| *bytes != pixels) {
                    return Err("Map-boundary retained pixels differ".into());
                }
                first = Some(pixels);
            }
            if read(&format!("{name}-cache.txt"))? != b"96\t64\t288\t18432\n" {
                return Err("Map-boundary cache layout differs".into());
            }
            let cache = read(&format!("{name}-cache.bgr"))?;
            known(&cache, marker(&cache)?)?;
            if row[3] == 1 {
                if first.as_ref() != Some(&cache) {
                    return Err("Returned image/cache full bytes differ".into());
                }
            } else if prior_cache.as_ref() != Some(&cache) {
                return Err("Native error replaced warm cache bytes".into());
            }
            prior_cache = Some(cache);
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected Map-boundary directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 map file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "map-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected Map-boundary root entry".into());
        }
    }
    if names != actual || inputs.len() != 724 {
        return Err("Map-boundary files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"calls":120,"map_failures":14,"conversion_failures":12,"recovery_captures":40,"retained_images":94,"actual_pixel_files":360,"nonempty_pixel_files":308,"registered_files":724,"inputs":inputs,
        "scope":"Real D3D context/resource expression views forward original calls without a COM substitute. UINT_MAX subresource rejection uses the actual native HRESULT and never consumes failed mapping output. A controlled C++ conversion exception/Rust converter error both exercise the real successful mapping cleanup. Exact copy/map/convert/cache-clone/unmap order, caller channel, native identity, unchanged warm cache on failure, recovery and held CPU bytes verified. Original Mat cache assignment and Rust clone completion are separate implementation points before Unmap; Rust additionally reports its safety copy and final CPU/completion owners. Not a device-loss, native allocator failure, Rust panic, complete GPU accounting, arbitrary failures or full P1 acceptance."});
    let output = root.join("map-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Map-boundary report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool map_boundary calls=120 map_failures=14 conversion_failures=12 recovery=40 retained=94 actual_pixels=360 actual_files=724 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_map_calls_and_native_cleanup_records() {
        assert!(calls(HEADER).is_err());
        assert!(states(STATE, FINAL).is_err());
        assert!(marker(&[]).is_err());
    }
}
