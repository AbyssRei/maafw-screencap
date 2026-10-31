// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Manager fallback after ordinary native FramePool item-size rejection.
use crate::{Result, framepool_queries::numbers, records};
use std::{collections::BTreeSet, fs, path::Path};
const HEADER: &str = "cycle\tsource_connected\tsource_selected\trust_connected\trust_selected\twarm_class\tmeasured_class\tqueries\tconversions\towners\tcaller_thread\tfixture_thread\tgdi_before\tgdi_after";
const FINAL: &str = "cycle\towners\tcompletion_live\tclose_errors\tconversions";
fn calls(text: &str, final_text: &str) -> Result<Vec<(i64, i64)>> {
    let mut rows = text.lines();
    let mut finals = final_text.lines();
    if rows.next() != Some(HEADER) || finals.next() != Some(FINAL) {
        return Err("Manager Size headers differ".into());
    }
    let mut caller = None;
    let mut gdi = Vec::new();
    for cycle in 0..20 {
        let row = numbers(rows.next().ok_or("Missing manager Size lifecycle")?, None)?;
        if row.len() != 14
            || row[..10] != [cycle, 1, 1, 1, 1, 0, 0, 2, 2, cycle]
            || !(1..=u32::MAX as i64).contains(&row[10])
            || !(1..=u32::MAX as i64).contains(&row[11])
            || row[10] == row[11]
            || *caller.get_or_insert(row[10]) != row[10]
            || !(1..=u32::MAX as i64).contains(&row[12])
            || !(1..=u32::MAX as i64).contains(&row[13])
        {
            return Err(
                "Manager source outcome/native rejection/fallback/CPU owner differs".into(),
            );
        }
        if numbers(
            finals.next().ok_or("Missing manager Size final owner")?,
            None,
        )? != [cycle, cycle + 1, 0, 0, 3]
        {
            return Err("Manager Size publication/conversion/close differs".into());
        }
        gdi.push((row[12], row[13]));
    }
    if rows.next().is_some() || finals.next().is_some() {
        return Err("Extra manager Size records".into());
    }
    Ok(gdi)
}
fn known(data: &[u8]) -> Result<()> {
    if data.len() != 18432 {
        return Err("Manager Size pixel span differs".into());
    }
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = (y * 96 + x) * 3;
            if data[offset..offset + 3] != expected {
                return Err("Manager Size known full pixels differ".into());
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
                "expected upstream-trace manager-size <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut names = BTreeSet::new();
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        names.insert(name.to_string());
        let p = root.join(name);
        inputs.push(records::artifact(&root, &p)?);
        Ok(fs::read(p)?)
    };
    let gdi = calls(
        std::str::from_utf8(&read("manager.tsv")?)?,
        std::str::from_utf8(&read("final.tsv")?)?,
    )?;
    for cycle in 0..20 {
        let mut first = None;
        for suffix in ["", "-held"] {
            let name = format!("{cycle}{suffix}");
            if read(&format!("{name}.txt"))? != b"96\t64\t288\t18432\n" {
                return Err("Manager Size layout differs".into());
            }
            let data = read(&format!("{name}.bgr"))?;
            known(&data)?;
            if first.as_ref().is_some_and(|bytes| *bytes != data) {
                return Err("Manager Size held pixels differ".into());
            }
            first = Some(data);
        }
    }
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("Unexpected manager Size directory/link".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 manager Size file")?;
        if name != "manager-validation.json" {
            actual.insert(name);
        }
    }
    if names != actual || inputs.len() != 82 {
        return Err("Manager Size file set differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles":20,"original_manager_model_cases":20,"real_framepool_rejections":40,"native_gdi_captures":60,"retained_images":20,"actual_pixel_files":40,"registered_files":82,"gdi_counts":gdi,"inputs":inputs,"scope":"Exact original manager function-body model with explicit backend/clock/input=None boundaries supplies the ordinary no-image fallback expectation. Real Rust native factories construct GDI and FramePool/pseudo candidates; each FramePool capture performs a real native Size query then explicitly returns zero width. Warmup and measured rejection both remain NoImage; the surviving GDI performs warmup/measurement/publication. Full known pixels and CPU owners survive manager/window/COM close. GDI counts are observations, not a full GPU/resource accounting claim. Separate original/Rust Size matrix supplies original native rejection evidence; this is compositional validation, not the complete original native manager or all-mask/framework/P1 acceptance."});
    let output = root.join("manager-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved manager Size report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Manager initial_size_rejection cycles=20 source_model_cases=20 real_framepool_rejections=40 gdi_captures=60 retained=20 pixels=40 actual_files=82 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_incomplete_manager_size_lifecycles() {
        assert!(calls(HEADER, FINAL).is_err());
        assert!(known(&[]).is_err());
    }
}
