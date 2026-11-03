// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual failed creation boundaries and partial native D3D owner records.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};
const CODES: [u64; 4] = [0x8000_4005, 0x8007_000e, 0x8000_4005, 0x8000_4002];
const CALLS: &str = "cycle\tmode\tcontrol\tsuccess\torder\tdevice_calls\tnative_devices\tfactory_calls\tnative_factories\tinjected_code\tsurface_code\tbefore_close_slots\tafter_close_slots\tconversions\tsamples\tcaller\tbegin\treturned\tobserved\tdevice_result\tfactory_result\tviolation";
const LIFE: &str = "cycle\tmode\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr";
fn table(path: &Path, header: &str, width: usize) -> Result<Vec<Vec<u64>>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("initialization table header differs".into());
    }
    lines
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != width {
                return Err("initialization table width differs".into());
            }
            fields
                .into_iter()
                .map(|f| {
                    let value = f.parse::<u64>()?;
                    if value.to_string() != f {
                        return Err("noncanonical initialization integer".into());
                    }
                    Ok(value)
                })
                .collect()
        })
        .collect()
}
fn side(root: &Path, rust: bool) -> Result<usize> {
    let mut expected = "2\t8\t8\t8\t4\t4\t4".to_owned();
    for code in CODES {
        expected.push_str(&format!("\t{code}"));
    }
    expected.push('\n');
    if fs::read(root.join("abi.tsv"))? != expected.as_bytes() {
        return Err("initialization native ABI/error codes differ".into());
    }
    let calls = table(&root.join("calls.tsv"), CALLS, 22)?;
    let life = table(&root.join("lifetimes.tsv"), LIFE, 16)?;
    if calls.len() != 100 || life.len() != 240 {
        return Err("incomplete initialization creation/lifetime matrix".into());
    }
    let mut rows = BTreeMap::new();
    for row in life {
        if rows
            .insert((row[0], row[1], row[2], row[3]), row[4..].to_vec())
            .is_some()
        {
            return Err("duplicate initialization lifetime row".into());
        }
    }
    let mut caller = 0;
    let mut last_end = 0;
    let mut returned_notifications = 0;
    let (groups, remainder) = calls.as_chunks::<5>();
    if !remainder.is_empty() {
        return Err("incomplete initialization cycle".into());
    }
    for (cycle, group) in groups.iter().enumerate() {
        for (mode, row) in group.iter().enumerate() {
            let code = if mode < 4 { CODES[mode] } else { 0 };
            let control = u64::from(mode == 4);
            let expected = [
                u64::try_from(cycle)?,
                u64::try_from(mode)?,
                control,
                control,
                if mode < 2 { 1 } else { 12 },
                1,
                u64::from(mode >= 2),
                u64::from(mode >= 2),
                control,
                code,
                if rust { code } else { 0 },
                if mode == 4 { 7 } else { 0 },
                0,
                0,
                0,
            ];
            if row[..15] != expected
                || row[15] == 0
                || row[16] == 0
                || row[16] > row[17]
                || row[17] > row[18]
                || row[16] < last_end
                || row[19] != if mode < 2 { code } else { 0 }
                || row[20] != if (2..4).contains(&mode) { code } else { 0 }
                || row[21] != 0
            {
                return Err("initialization call/result/native boundary differs".into());
            }
            if caller == 0 {
                caller = row[15];
            }
            u32::try_from(row[15])?;
            if row[15] != caller {
                return Err("initialization caller changed".into());
            }
            last_end = row[18];
            if (2..4).contains(&mode) {
                for kind in 0..2 {
                    let mut phases = Vec::new();
                    for phase in 0..3 {
                        phases.push(
                            rows.remove(&(
                                u64::try_from(cycle)?,
                                u64::try_from(mode)?,
                                phase,
                                kind,
                            ))
                            .ok_or("missing partial initialization lifetime stage")?,
                        );
                    }
                    let live = &phases[0];
                    let returned = &phases[1];
                    let final_row = &phases[2];
                    for observed in [live, returned, final_row] {
                        // Native DWORD thread identities and UINT callback IDs
                        // remain 32-bit even in the observer's u64 transport.
                        for field in [5, 6, 9] {
                            u32::try_from(observed[field])?;
                        }
                    }
                    if live[..5] != [1, 1, kind, 1, 0]
                        || live[5] != caller
                        || live[6] != 0
                        || live[7] < row[16]
                        || live[7] > row[17]
                        || live[8] != 0
                        || live[10] == 0
                        || live[11] != 0
                    {
                        return Err("partial initialization live native owner differs".into());
                    }
                    for observed in [returned, final_row] {
                        for field in [0, 1, 2, 3, 5, 7, 9, 10, 11] {
                            if observed[field] != live[field] {
                                return Err(
                                    "partial initialization native registration identity changed"
                                        .into(),
                                );
                            }
                        }
                    }
                    if returned[4] > 1
                        || final_row[4] != 1
                        || final_row[6] == 0
                        || final_row[8] < live[7]
                        || final_row[8] > row[18]
                    {
                        return Err(
                            "partial initialization native destruction notification missing".into(),
                        );
                    }
                    if returned[4] == 1
                        && (returned[6] != final_row[6] || returned[8] != final_row[8])
                    {
                        return Err("partial initialization completed notification changed".into());
                    }
                    returned_notifications += usize::try_from(returned[4])?;
                }
            }
        }
    }
    if !rows.is_empty() {
        return Err("additional partial initialization lifetimes".into());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !["abi.tsv", "calls.tsv", "lifetimes.tsv"]
                .iter()
                .any(|name| entry.file_name() == *name)
        {
            return Err("unregistered initialization side file".into());
        }
    }
    Ok(returned_notifications)
}
fn validate(root: &Path) -> Result<serde_json::Value> {
    let source = side(&root.join("source"), false)?;
    let rust = side(&root.join("rust"), true)?;
    let mut inputs = Vec::new();
    for name in ["source", "rust"] {
        for file in ["abi.tsv", "calls.tsv", "lifetimes.tsv"] {
            inputs.push(records::artifact(root, &root.join(name).join(file))?);
        }
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let kind = entry.file_type()?;
        if !((name == "source" || name == "rust") && kind.is_dir()
            || name == "initialization-validation.json" && kind.is_file())
        {
            return Err("unregistered initialization root file/directory".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(
        serde_json::json!({"layout_version":2,"sides":2,"cycles_per_side":20,"public_rejections":160,"direct_init_controls":40,"native_devices":120,"native_factories":40,"d3d_notifications":160,"notifications_in_post_return_snapshot":{"source":source,"rust":rust},"captures":0,"inputs":inputs,"scope":"Whole unchanged F DesktopDup class and same Rust backend with two explicit creation boundaries returning controlled E_FAIL/E_OUTOFMEMORY/E_NOINTERFACE. Factory failures follow real hardware device/context creation and actual D3D destruction registrations. Public failure slots clear, zero acquisition/conversion, direct no-fault forwarding initialization controls and bounded final notifications. Source returns optional/bool only; exact HRESULT surface is Rust-only. No actual OOM/device failure, factory destruction notification, atomic snapshot, native capture or full P1 acceptance.","full_P1_accepted":false}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace desktop-initialization <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let report = validate(&root)?;
    let output = root.join("initialization-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved initialization validation differs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup initialization public_rejections=160 direct_init_controls=40 native_devices=120 native_factories=40 d3d_notifications=160 captures=0 files=6 full_P1_acceptance=0 passed"
    );
    Ok(())
}
