// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Finite real-monitor lookup/fallback records, without creating any native owner.
use crate::{Result, records};
use std::{fs, path::Path};
const FIELDS: usize = 38;
const FAIL: u64 = 0x8000_4005;
const NO_INTERFACE: u64 = 0x8000_4002;
const ABORT: u64 = 0x8000_4004;
const NOT_FOUND: u64 = 0x887a_0002;
const CALLS: &str = "cycle\tcontrol\tsuccess\tbefore_slots\tafter_slots\tbefore_monitor\tafter_monitor\tsamples\tconversions\tsurface_code\tsurface_error\treturned\tclosed\tobserved";
fn number(text: &str) -> Result<u64> {
    let value = text.parse::<u64>()?;
    if value.to_string() != text {
        return Err("noncanonical lookup scalar".into());
    }
    Ok(value)
}
fn table(path: &Path, header: &str, width: usize) -> Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("lookup header differs".into());
    }
    lines
        .map(|line| {
            let row = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
            if row.len() != width {
                return Err("lookup row width differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn numeric(path: &Path, header: &str, width: usize) -> Result<Vec<Vec<u64>>> {
    table(path, header, width)?
        .iter()
        .map(|r| r.iter().map(|v| number(v)).collect())
        .collect()
}
fn side(root: &Path, rust: bool) -> Result<u32> {
    let abi = fs::read_to_string(root.join("abi.tsv"))?;
    let fields = abi
        .trim_end_matches('\n')
        .split('\t')
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    if fields.len() != 13
        || fields[0] != 1
        || fields[1] > 4
        || fields[2..9] != [38, 8, 8, 8, 4, 4, 4]
        || fields[9..] != [FAIL, NO_INTERFACE, ABORT, NOT_FOUND]
    {
        return Err("lookup native ABI differs".into());
    }
    let mode = u32::try_from(fields[1])?;
    if abi
        != format!(
            "1\t{mode}\t38\t8\t8\t8\t4\t4\t4\t{FAIL}\t{NO_INTERFACE}\t{ABORT}\t{NOT_FOUND}\n"
        )
    {
        return Err("noncanonical lookup ABI".into());
    }
    let target = numeric(
        &root.join("target.tsv"),
        "phase\thwnd\tmonitor\tcaller\towner\tvisible\tvalid\tresolved",
        8,
    )?;
    if target.len() != 2 {
        return Err("lookup target observations incomplete".into());
    }
    let t = &target[0];
    u32::try_from(t[3])?;
    if t[0] != 0
        || t[1] == 0
        || t[2] == 0
        || t[3] == 0
        || t[4] != t[3]
        || t[5..7] != [0, 1]
        || t[7] != t[2]
        || target[1] != [1, t[1], t[2], t[3], 0, 0, 0, 0]
    {
        return Err("lookup hidden target/real monitor/caller/checked close differs".into());
    }
    let calls = table(&root.join("calls.tsv"), CALLS, 14)?;
    let traces = numeric(
        &root.join("trace.tsv"),
        "cycle\tcontrol\tphase\tfields",
        3 + FIELDS,
    )?;
    let life = numeric(
        &root.join("lifetimes.tsv"),
        "cycle\tcontrol\tphase\tkind\tfields",
        16,
    )?;
    if calls.len() != 40 || traces.len() != 80 || life.len() != 160 {
        return Err("incomplete lookup matrix".into());
    }
    let evidence = if rust {
        Some(numeric(
            &root.join("evidence.tsv"),
            "cycle\tcontrol\tadapters\toutputs\tfallback\tlookup_error_present\tlookup_error\tmonitor_failed",
            8,
        )?)
    } else {
        None
    };
    if evidence.as_ref().is_some_and(|r| r.len() != 40) {
        return Err("lookup Rust evidence incomplete".into());
    }
    let m = usize::try_from(mode)?;
    let mut previous = 0;
    for (index, call) in calls.iter().enumerate() {
        let cycle = u64::try_from(index / 2)?;
        let control = u64::try_from(index % 2)?;
        let fallback = u64::from(control == 0);
        let monitor = if control == 1 { t[2] } else { 0 };
        let values = call
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 10)
            .map(|(_, v)| number(v))
            .collect::<Result<Vec<_>>>()?;
        if values[..10]
            != [
                cycle,
                control,
                0,
                31,
                0,
                monitor,
                monitor,
                0,
                0,
                if rust { ABORT } else { 0 },
            ]
            || call[10] != (if rust { "DuplicateOutput" } else { "-" })
            || values[10] < previous
            || values[10] == 0
            || values[10] > values[11]
            || values[11] > values[12]
        {
            return Err("lookup public rejection/partial owners/monitor/clocks differ".into());
        }
        let ac = if control == 1 { 1 } else { [1, 2, 2, 2, 2][m] };
        let oc = if control == 1 { 1 } else { [1, 1, 2, 2, 2][m] };
        for phase in 0..2_usize {
            let r = &traces[index * 2 + phase];
            if r[..3] != [cycle, control, u64::try_from(phase)?] {
                return Err("lookup trace order differs".into());
            }
            let a = &r[3..];
            let expected = [
                u64::from(mode),
                control,
                t[3],
                0,
                1,
                1,
                if control == 1 { 1 } else { [2, 3, 3, 2, 3][m] },
                if control == 1 { 1 } else { [1, 2, 2, 2, 2][m] },
                if control == 1 { 1 } else { [1, 2, 3, 2, 3][m] },
                if control == 1 { 1 } else { [1, 1, 2, 2, 2][m] },
                u64::from(control == 1 || mode >= 2),
                u64::from(control == 1 || mode >= 3),
                if control == 1 || mode != 3 { 1 } else { 2 },
                1,
                1,
                0,
                1,
                u64::try_from(phase)?,
                ac,
                ac - u64::from(phase == 0),
                oc,
                oc - u64::from(phase == 0),
                fallback,
                if control == 1 || mode == 4 {
                    0
                } else if mode == 3 {
                    NO_INTERFACE
                } else {
                    FAIL
                },
                ABORT,
                fallback,
                u64::from(control == 0 && mode >= 1),
                u64::from(control == 0 && mode >= 1),
                u64::from(control == 0 && mode >= 2),
                u64::from(control == 0 && mode >= 2),
                u64::from(control == 0 && matches!(mode, 1 | 2 | 4)),
                u64::from(control == 0 && matches!(mode, 2 | 4)),
            ];
            if a[..32] != expected
                || a[34] != t[2]
                || [32, 33, 35, 36, 37].into_iter().any(|i| a[i] == 0)
            {
                return Err(
                    "lookup native calls/fault/sentinel/temporary release before fallback differs"
                        .into(),
                );
            }
            u32::try_from(a[2])?;
            if phase == 1 {
                let before = &traces[index * 2][3..];
                if (0..FIELDS)
                    .filter(|i| !matches!(i, 17 | 19 | 21))
                    .any(|i| a[i] != before[i])
                {
                    return Err("lookup non-release evidence changed during close".into());
                }
            }
            for kind in 0..2_usize {
                let r = &life[index * 4 + phase * 2 + kind];
                let first = &life[index * 4 + kind];
                for i in [9, 10, 13] {
                    u32::try_from(r[i])?;
                }
                if r[..10]
                    != [
                        cycle,
                        control,
                        u64::try_from(phase)?,
                        u64::try_from(kind)?,
                        1,
                        1,
                        u64::try_from(kind)?,
                        1,
                        u64::try_from(phase)?,
                        t[3],
                    ]
                    || r[11] < previous
                    || r[11] > values[10]
                    || r[11] == 0
                    || r[14] != a[32 + kind]
                    || r[15] != 0
                    || [4, 5, 6, 7, 9, 11, 13, 14, 15]
                        .into_iter()
                        .any(|i| r[i] != first[i])
                    || phase == 0 && [r[10], r[12]] != [0, 0]
                    || phase == 1 && (r[10] == 0 || r[12] < values[10] || r[12] > values[12])
                {
                    return Err(
                        "lookup actual D3D registration/notification/width/clock differs".into(),
                    );
                }
            }
        }
        if let Some(rows) = &evidence {
            let error = if control == 1 {
                0
            } else if mode == 0 {
                FAIL
            } else if mode == 3 {
                NO_INTERFACE
            } else {
                NOT_FOUND
            };
            if rows[index]
                != [
                    cycle,
                    control,
                    u64::from(control == 1 || mode != 0),
                    u64::from(control == 1 || mode >= 2),
                    fallback,
                    fallback,
                    error,
                    0,
                ]
            {
                return Err("lookup Rust branch/last error evidence differs".into());
            }
        }
        previous = values[12];
    }
    Ok(mode)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) =
        match args {
            [p] => (p, false),
            [p, f] if f == "--check-only" => (p, true),
            _ => return Err(
                "expected upstream-trace desktop-lookup <external paired directory> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mode = side(&root.join("source"), false)?;
    if side(&root.join("rust"), true)? != mode {
        return Err("paired lookup modes differ".into());
    }
    let mut inputs = Vec::new();
    for name in ["source", "rust"] {
        let files = if name == "source" {
            &[
                "abi.tsv",
                "calls.tsv",
                "trace.tsv",
                "lifetimes.tsv",
                "target.tsv",
            ][..]
        } else {
            &[
                "abi.tsv",
                "calls.tsv",
                "trace.tsv",
                "lifetimes.tsv",
                "target.tsv",
                "evidence.tsv",
            ][..]
        };
        for file in files {
            inputs.push(records::artifact(&root, &root.join(name).join(file))?);
        }
        if fs::read_dir(root.join(name))?.count() != files.len() {
            return Err("additional lookup side entry".into());
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if !matches!(
            name.to_str(),
            Some("source" | "rust" | "lookup-validation.json")
        ) {
            return Err("additional lookup root entry".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"mode":mode,"public_rejections":80,"primary_fallbacks":40,"matching_controls":40,"d3d_notifications":160,"hidden_targets":2,"captures":0,"inputs":inputs,"scope":"Whole unchanged F public screencap and same Rust backend on actual hidden HWND/first-output monitor. Controlled lookup enumeration/GetDesc/QI failures or mismatched descriptor, explicit NOT_FOUND tails bound the physical scan. Temporary facade references end before primary fallback; selected resources/monitor persist until explicit close, actual device/context destruction notifications complete. Every duplication stops with E_ABORT. No physical multi-adapter search, driver duplication/frame/foreground/window adjustment, underlying DXGI/global GPU destruction, repeated-call recovery or full P1 acceptance."});
    let output = root.join("lookup-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved lookup report differs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup lookup mode={mode} public_rejections=80 fallbacks=40 matching_controls=40 d3d_notifications=160 captures=0 actual_files=11 full_P1_acceptance=0 passed"
    );
    Ok(())
}
