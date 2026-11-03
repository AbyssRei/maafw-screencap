// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Primary-output failure records; validation never creates a device or capture.
use crate::{Result, records};
use serde::Serialize;
use std::{fs, path::Path};
const FIELDS: usize = 26;
const FAIL: u64 = 0x8000_4005;
const NO_INTERFACE: u64 = 0x8000_4002;
const ABORT: u64 = 0x8000_4004;
const CALLS: &str = "cycle\tcontrol\tsuccess\tbefore_close_slots\tafter_close_slots\tsamples\tconversions\tsurface_code\tsurface_error\treturned\tclosed\tobserved";

fn number(text: &str) -> Result<u64> {
    let value = text.parse::<u64>()?;
    if value.to_string() != text {
        return Err("noncanonical primary-output integer".into());
    }
    Ok(value)
}
fn table(path: &Path, header: &str, width: usize) -> Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("primary-output header differs".into());
    }
    lines
        .map(|line| {
            let row = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
            if row.len() != width {
                return Err("primary-output row width differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn numeric_table(path: &Path, header: &str, width: usize) -> Result<Vec<Vec<u64>>> {
    table(path, header, width)?
        .iter()
        .map(|r| r.iter().map(|s| number(s)).collect())
        .collect()
}
fn side(root: &Path, rust: bool) -> Result<u32> {
    let abi = fs::read_to_string(root.join("abi.tsv"))?;
    let fields = abi
        .trim_end_matches('\n')
        .split('\t')
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    if fields.len() != 12
        || fields[0] != 1
        || fields[1] > 3
        || fields[2..9] != [26, 8, 8, 8, 4, 4, 4]
        || fields[9..] != [FAIL, NO_INTERFACE, ABORT]
    {
        return Err("primary-output native ABI differs".into());
    }
    let mode = u32::try_from(fields[1])?;
    let expected_abi =
        format!("1\t{mode}\t26\t8\t8\t8\t4\t4\t4\t{FAIL}\t{NO_INTERFACE}\t{ABORT}\n");
    if abi != expected_abi {
        return Err("noncanonical primary-output ABI".into());
    }
    let calls = table(&root.join("calls.tsv"), CALLS, 12)?;
    let traces = numeric_table(
        &root.join("trace.tsv"),
        "cycle\tcontrol\tphase\tfields",
        3 + FIELDS,
    )?;
    let lives = numeric_table(
        &root.join("lifetimes.tsv"),
        "cycle\tcontrol\tphase\tkind\tfields",
        16,
    )?;
    if calls.len() != 40 || traces.len() != 80 || lives.len() != 160 {
        return Err("incomplete primary-output matrix".into());
    }
    let mut caller = 0;
    let mut last_observed = 0;
    for (index, call) in calls.iter().enumerate() {
        let cycle = u64::try_from(index / 2)?;
        let control = u64::try_from(index % 2)?;
        let values = call
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 8)
            .map(|(_, v)| number(v))
            .collect::<Result<Vec<_>>>()?;
        let code = if control == 1 {
            ABORT
        } else if mode == 2 {
            NO_INTERFACE
        } else {
            FAIL
        };
        let before = if control == 1 {
            31
        } else {
            [7, 15, 15, 31][usize::try_from(mode)?]
        };
        if values[..8]
            != [
                cycle,
                control,
                0,
                before,
                0,
                0,
                0,
                if rust { code } else { 0 },
            ]
        {
            return Err("primary-output call outcome/retained slots differ".into());
        }
        let expected_name = if !rust {
            "-"
        } else if control == 1 {
            "DuplicateOutput"
        } else {
            [
                "EnumAdapters primary",
                "EnumOutputs primary",
                "primary QueryInterface IDXGIOutput1",
                "DuplicateOutput",
            ][usize::try_from(mode)?]
        };
        if call[8] != expected_name {
            return Err("primary-output public error surface differs".into());
        }
        let returned = values[8];
        let closed = values[9];
        let observed = values[10];
        if returned <= last_observed || closed < returned || observed < closed {
            return Err("primary-output observation clocks differ".into());
        }
        last_observed = observed;
        let adapters = u64::from(control == 1 || mode > 0);
        let outputs = u64::from(control == 1 || mode > 1);
        let queries = u64::from(control == 1 || mode > 2);
        let mut actual = [[0_u64; FIELDS]; 2];
        for (phase, slot) in actual.iter_mut().enumerate() {
            let row = &traces[index * 2 + phase];
            let phase_u64 = u64::try_from(phase)?;
            if row[..3] != [cycle, control, phase_u64] {
                return Err("primary-output trace identity/order differs".into());
            }
            slot.copy_from_slice(&row[3..]);
            let r = *slot;
            let native_caller = u32::try_from(r[2])?;
            if native_caller == 0 {
                return Err("missing primary-output caller".into());
            }
            if caller == 0 {
                caller = native_caller;
            }
            if caller != native_caller {
                return Err("primary-output caller changed".into());
            }
            let expected = [
                u64::from(mode),
                control,
                u64::from(caller),
                0,
                1,
                1,
                1,
                adapters,
                adapters,
                outputs,
                outputs,
                queries,
                queries,
                0,
                1,
                phase_u64,
                adapters,
                phase_u64 * adapters,
                outputs,
                if phase == 1 {
                    outputs
                } else {
                    u64::from(control == 0 && mode == 2)
                },
                code,
            ];
            if r[..21] != expected {
                return Err("primary-output native calls/facade owners differ".into());
            }
            for (offset, present) in [true, true, true, adapters == 1, outputs == 1]
                .into_iter()
                .enumerate()
            {
                let pointer = r[21 + offset];
                usize::try_from(pointer)?;
                if (pointer != 0) != present {
                    return Err("primary-output native/facade pointer presence differs".into());
                }
            }
        }
        if actual[0][21..] != actual[1][21..] {
            return Err("primary-output observed identity changed after close".into());
        }
        for phase in 0..2 {
            for kind in 0..2 {
                let row = &lives[index * 4 + phase * 2 + kind];
                if row[..4] != [cycle, control, u64::try_from(phase)?, u64::try_from(kind)?] {
                    return Err("primary-output lifetime identity/order differs".into());
                }
                let r = &row[4..];
                let old = &lives[index * 4 + kind][4..];
                u32::try_from(r[5])?;
                u32::try_from(r[6])?;
                u32::try_from(r[9])?;
                usize::try_from(r[10])?;
                if r[..6]
                    != [
                        1,
                        1,
                        u64::try_from(kind)?,
                        1,
                        u64::try_from(phase)?,
                        u64::from(caller),
                    ]
                    || r[7] == 0
                    || r[7] > returned
                    || r[10] != actual[0][21 + kind]
                    || r[11] != 0
                    || r[0..4] != old[0..4]
                    || r[5] != old[5]
                    || r[7] != old[7]
                    || r[9..] != old[9..]
                    || (phase == 0 && (r[6] != 0 || r[8] != 0))
                    || (phase == 1 && (r[6] == 0 || r[8] < returned || r[8] > observed))
                {
                    return Err("primary-output D3D lifetime/clock/native width differs".into());
                }
            }
        }
    }
    Ok(mode)
}
#[derive(Serialize)]
struct Report {
    layout_version: u32,
    mode: u32,
    public_rejections: usize,
    primary_forwarding_controls: usize,
    d3d_notifications: usize,
    captures: usize,
    inputs: Vec<records::Artifact>,
    scope: &'static str,
}
pub fn run(args: &[String]) -> Result<()> {
    let (directory, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace desktop-primary-output <external pair> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(directory))?;
    let source_mode = side(&root.join("source"), false)?;
    if side(&root.join("rust"), true)? != source_mode {
        return Err("primary-output paired modes differ".into());
    }
    let mut inputs = Vec::new();
    for name in ["source", "rust"] {
        for file in ["abi.tsv", "calls.tsv", "trace.tsv", "lifetimes.tsv"] {
            inputs.push(records::artifact(&root, &root.join(name).join(file))?);
        }
        if fs::read_dir(root.join(name))?.count() != 4 {
            return Err("additional primary-output side file".into());
        }
    }
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        let text = name.to_str().ok_or("nontext output entry")?;
        if !matches!(text, "source" | "rust" | "primary-output-validation.json") {
            return Err("additional primary-output evidence entry".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = Report {
        layout_version: 1,
        mode: source_mode,
        public_rejections: 80,
        primary_forwarding_controls: 40,
        d3d_notifications: 160,
        captures: 0,
        inputs,
        scope: "Whole unchanged F class and same Rust backend, real default hardware device/factory/primary enumeration/query with caller-local COM facades. Four selected HRESULT failure boundaries; controls forward native primary lookup and stop duplication with E_ABORT. Public failures preserve partial slots until explicit cleanup; facade releases are weaker than global DXGI destruction, distinct from actual D3D notifications. No native DuplicateOutput success, frame acquisition, target lookup/fallback, physical device fault, arbitrary COM consumption or full P1 acceptance.",
    };
    let output = root.join("primary-output-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != serde_json::to_value(&report)? {
            return Err("saved primary-output validation differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup primary output mode={source_mode} public_rejections=80 primary_forwarding_controls=40 d3d_notifications=160 captures=0 actual_files=8 full_P1_acceptance=0 passed"
    );
    Ok(())
}
