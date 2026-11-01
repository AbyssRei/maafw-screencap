// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual caller-held native acquisition, rejected capture and one recovery.
use crate::{
    Result,
    framepool_queries::{known, numbers},
    records,
};
use std::{collections::BTreeSet, fs, path::Path};
const INVALID_CALL: i64 = -2_005_270_527; // SDK DXGI_ERROR_INVALID_CALL (0x887a0001).
const CALLS: &str = "cycle\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\treleased\ttimeout\tpreserved\tmarker\tpitch\tthread\tcontext\tresource\tduplication";
const HELD: &str =
    "cycle\tprepared\trejected\tmanual_release\tafter_release\tthread\tresource\tduplication";
const STATE: &str = "cycle\twidth\theight\tstaging\tduplication\tthread\tcopied_recovery";
fn table(text: &str, header: &str, count: usize) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("Desktop acquisition table header differs".into());
    }
    let rows = lines
        .map(|line| numbers(line, None))
        .collect::<Result<Vec<_>>>()?;
    if rows.len() != count {
        return Err("Incomplete desktop acquisition table".into());
    }
    Ok(rows)
}
fn calls(text: &str) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(CALLS) {
        return Err("Desktop acquisition call header differs".into());
    }
    let mut rows = Vec::new();
    for cycle in 0..20_i64 {
        for phase in ["rejected", "recovery"] {
            let row = numbers(
                lines.next().ok_or("Missing desktop acquisition call")?,
                Some((1, phase)),
            )?;
            let recovery = phase == "recovery";
            if row.len() != 20
                || row[0] != cycle
                || row[2] != i64::from(recovery)
                || row[3] != if recovery { 123456 } else { 1 }
                || row[4..10]
                    != [
                        1,
                        i64::from(recovery),
                        i64::from(recovery),
                        i64::from(recovery),
                        i64::from(recovery),
                        i64::from(recovery),
                    ]
                || row[10] != if recovery { 0 } else { INVALID_CALL }
                || row[11..14] != [0, 2000, 1]
                || (recovery && ![0, 1].contains(&row[14]))
                || (!recovery && row[14] != -1)
                || (recovery && !(384..=u32::MAX as i64).contains(&row[15]))
                || (!recovery && row[15] != 0)
                || !(1..=u32::MAX as i64).contains(&row[16])
                || row[19] <= 0
                || (recovery && (row[17] <= 0 || row[18] <= 0))
                || (!recovery && row[17..19] != [0, 0])
            {
                return Err("Desktop rejected-acquisition native calls/state differ".into());
            }
            rows.push(row);
        }
    }
    if lines.next().is_some() {
        return Err("Extra desktop acquisition calls".into());
    }
    Ok(rows)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace desktop-acquire <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut names = BTreeSet::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        names.insert(name.to_string());
        inputs.push(records::artifact(&root, &root.join(name))?);
        Ok(fs::read(root.join(name))?)
    };
    let mut dimensions = None;
    let mut source_calls: Option<Vec<Vec<i64>>> = None;
    for side in ["source", "rust"] {
        let rows = calls(std::str::from_utf8(&read(&format!("{side}/acquire.tsv"))?)?)?;
        let held = table(
            std::str::from_utf8(&read(&format!("{side}/held.tsv"))?)?,
            HELD,
            20,
        )?;
        let state = table(
            std::str::from_utf8(&read(&format!("{side}/state.tsv"))?)?,
            STATE,
            20,
        )?;
        let mut caller = None;
        let mut sizes = Vec::new();
        for cycle in 0..20_usize {
            let (rejected, recovery, held, state) = (
                &rows[cycle * 2],
                &rows[cycle * 2 + 1],
                &held[cycle],
                &state[cycle],
            );
            if held.len() != 8
                || state.len() != 7
                || held[0] != cycle as i64
                || state[0] != cycle as i64
                || held[1..5] != [0, INVALID_CALL, 0, INVALID_CALL]
                || held[5] != rejected[16]
                || held[5] != recovery[16]
                || held[5] != state[5]
                || *caller.get_or_insert(held[5]) != held[5]
                || held[6] <= 0
                || held[6] == state[3]
                || held[7] != rejected[19]
                || held[7] != recovery[19]
                || held[7] != state[4]
                || state[3] != recovery[18]
                || state[3] <= 0
                || !(96..=i32::MAX as i64).contains(&state[1])
                || !(64..=i32::MAX as i64).contains(&state[2])
                || recovery[15]
                    < state[1]
                        .checked_mul(4)
                        .ok_or("Desktop native pitch overflow")?
            {
                return Err(
                    "Desktop caller-held acquisition/release/native identity differs".into(),
                );
            }
            let copied = state[1]
                .checked_mul(state[2])
                .and_then(|n| n.checked_mul(4))
                .ok_or("Desktop CPU copy overflow")?;
            if state[6] != if side == "source" { -1 } else { copied } {
                return Err("Desktop recovery CPU-copy evidence differs".into());
            }
            sizes.push((state[1], state[2]));
            for phase in ["warm", "recovery"] {
                let mut first = None;
                for suffix in ["", "-held"] {
                    let name = format!("{side}/{cycle}-{phase}{suffix}");
                    let layout = String::from_utf8(read(&format!("{name}.txt"))?)?;
                    if numbers(layout.trim_end_matches('\n'), None)?
                        != [
                            96,
                            64,
                            state[1]
                                .checked_mul(3)
                                .ok_or("Desktop BGR stride overflow")?,
                            18432,
                        ]
                    {
                        return Err("Desktop held-acquisition ROI layout differs".into());
                    }
                    let bytes = read(&format!("{name}.bgr"))?;
                    let marker = match bytes.get(..3) {
                        Some([78, 95, 96]) => 0,
                        Some([26, 43, 60]) => 1,
                        _ => return Err("Desktop held-acquisition marker differs".into()),
                    };
                    if phase == "recovery" && marker != recovery[14] {
                        return Err("Desktop recovery marker record differs".into());
                    }
                    known(&bytes, marker)?;
                    if first.as_ref().is_some_and(|prior| *prior != bytes) {
                        return Err("Desktop held-acquisition retained pixels changed".into());
                    }
                    first = Some(bytes);
                }
            }
        }
        if let Some(expected) = &dimensions {
            if *expected != sizes {
                return Err("Original/Rust held-acquisition output sizes differ".into());
            }
        } else {
            dimensions = Some(sizes);
        }
        if let Some(expected) = &source_calls {
            for (source, rust) in expected.iter().zip(&rows) {
                if source[..14] != rust[..14] || source[15] != rust[15] {
                    return Err("Original/Rust held-acquisition native behavior differs".into());
                }
            }
        } else {
            source_calls = Some(rows);
        }
    }
    let mut actual = BTreeSet::new();
    for side in ["source", "rust"] {
        for entry in fs::read_dir(root.join(side))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected held-acquisition directory/link".into());
            }
            actual.insert(format!(
                "{side}/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-UTF8 acquisition file")?
            ));
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if (name == "source" || name == "rust") && entry.file_type()?.is_dir() {
            continue;
        }
        if name != "desktop-acquire-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected acquisition root entry".into());
        }
    }
    if names != actual || inputs.len() != 326 {
        return Err("Desktop held-acquisition files differ".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"public_calls":120,"observed_calls":80,"actual_acquire_rejections":40,"caller_precondition_acquires":40,"caller_manual_releases":40,"extra_release_rejections":40,"recovery_captures":40,"retained_images":80,"actual_pixel_files":160,"registered_files":326,"inputs":inputs,"scope":"A caller deliberately holds a real native acquired frame while the unchanged original/Rust capture's real AcquireNextFrame rejects INVALID_CALL. Capture leaves its staging/output state and caller's frame untouched; caller ReleaseFrame succeeds, extra release rejects, and one later capture recovers. Rust retains its existing full-output safety copy; original mapped lifetime remains distinct. Actual known target/held pixels and full-output ROI stride verified. Not access/device loss, timeout, general external ownership support, full GPU accounting or full P1 acceptance."});
    let output = root.join("desktop-acquire-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved desktop acquisition report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup held_acquisition calls=120 observed=80 rejected=40 caller_acquires=40 caller_releases=40 recovery=40 retained=80 actual_pixels=160 actual_files=326 full_P1_acceptance=0 passed"
    );
    Ok(())
}
