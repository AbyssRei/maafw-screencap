// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Separate complete-client scene; original runtime reports remain unchanged.
use crate::{Result, gamepad_runtime, records};
use std::{fs, path::Path};

fn numbers(path: &Path, rows: usize, columns: usize) -> Result<Vec<Vec<i64>>> {
    let values = fs::read_to_string(path)?
        .lines()
        .map(|line| {
            line.split('\t')
                .map(|text| {
                    let value = text.parse::<i64>()?;
                    if value.to_string() != text {
                        return Err("Noncanonical manager band scalar".into());
                    }
                    Ok(value)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    if values.len() != rows || values.iter().any(|r| r.len() != columns) {
        return Err("Incomplete manager band geometry matrix".into());
    }
    Ok(values)
}

pub fn run(args: &[String]) -> Result<()> {
    let (mask, path, check) = match args {
        [mask, path] => (mask, path, false),
        [mask, path, flag] if flag == "--check-only" => (mask, path, true),
        _ => return Err("expected upstream-delegate gamepad-client-band-runtime <mask> <external records> [--check-only]".into()),
    };
    let method: u64 = mask.parse()?;
    if !(1..=63).contains(&method) {
        return Err("Unknown manager client-band mask".into());
    }
    let root = records::external_root(Path::new(path))?;
    // Keep every original whole-Gamepad, actual DLL/factory, current/held pixel,
    // selected monitor ROI, native event and owner assertion before new data.
    gamepad_runtime::run(args)?;
    let mut artifacts = Vec::new();
    let mut observations = Vec::new();
    let mut shared_desktop = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let setup = numbers(&directory.join("client-band-setup.tsv"), 2, 31)?;
        let before = &setup[0];
        let after = &setup[1];
        for row in &setup {
            for &value in &row[..18] {
                i32::try_from(value)?;
            }
            for index in [18, 19, 25, 26] {
                u32::try_from(row[index])?;
            }
            for index in [21, 22, 23, 24, 27, 28, 29, 30] {
                i32::try_from(row[index])?;
            }
            if row[20] < 0
                || row[26] <= 0
                || row[0] > row[4]
                || row[1] > row[5]
                || row[2] < row[6]
                || row[3] < row[7]
                || row[2] <= row[0]
                || row[3] <= row[1]
            {
                return Err("Manager band native desktop/metadata differs".into());
            }
        }
        let x = after[4] + 64;
        let y = after[5] + 64;
        let expected = [x - 6, y - 6, x + 102, y + 70, 0, 0, 96, 64, x, y];
        if before[..8] != after[..8]
            || after[8..18] != expected
            || before[12..16] != [0, 0, 96, 64]
            || after[18] != 0x9400_0000
            || after[19] & 8 != 8
            || after[20] < before[20]
            || after[26] != before[26]
            || after[27..31] != [1, 0, 108, 76]
            || after[8] < after[4]
            || after[9] < after[5]
            || after[10] > after[6]
            || after[11] > after[7]
        {
            return Err("Manager band complete client/work-area/nonclient setup differs".into());
        }
        if let Some(previous) = shared_desktop.replace(after[..8].to_vec())
            && previous != after[..8]
        {
            return Err("Manager band source/Rust desktop geometry differs".into());
        }
        let rows = numbers(&directory.join("client-band-boundaries.tsv"), 120, 22)?;
        for (index, row) in rows.iter().enumerate() {
            if row[0] != i64::try_from(index / 6)?
                || row[1] != i64::try_from(index % 6)?
                || row[2..12] != expected
                || row[12..] != expected
            {
                return Err(
                    "Manager band public-operation boundary changed complete-client geometry"
                        .into(),
                );
            }
        }
        if fs::read_to_string(directory.join("client-band-summary.txt"))?
            .lines()
            .ne([
                "schema=1",
                "rows=120",
                "geometry_valid=1",
                "nonclient_band=6",
                "target_inset=0",
            ])
        {
            return Err("Manager band completion differs".into());
        }
        for name in [
            "client-band-setup.tsv",
            "client-band-boundaries.tsv",
            "client-band-summary.txt",
        ] {
            artifacts.push(records::artifact(&root, &directory.join(name))?);
        }
        observations.push(
            serde_json::json!({"side":side,"setup":setup,"boundaries":rows,
            "full_client":[96,64],"outer_client_insets":[6,6,6,6],"target_inset":0}),
        );
    }
    for name in [
        "delegate-runtime-validation.json",
        "gamepad-runtime-validation.json",
    ] {
        artifacts.push(records::artifact(&root, &root.join(name))?);
    }
    let report = serde_json::json!({"layout_version":1,"method":method,"artifacts":artifacts,
        "observations":observations,"boundary_rows_per_side":120,"full_P1_accepted":false,
        "scope":"Independent six-pixel native nonclient-band fixture through the same original/Rust whole-Gamepad/native capture facades. All original240current/heldimage,selectedscope,nativeunit/event/module/host assertions remain required. Every complete96x64client is known without target inset/mask/tolerance; each side has120public connect/capture/controlled-reconnect/inactive boundary pairs with exact outer/client/origin geometry. Caller-side geometry queries do not drain/pump the UI queue and are not atomic or observations during calls. No indicator setter or capture/default/wait/conversion change. Does not infer actual privacy getter, native pixel producer/drain, per-call style stability or full P1; old scenes/results remain separate."});
    let output = root.join("manager-client-band-validation.json");
    if check && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved manager band report differs".into());
        }
    } else if !check {
        records::create_json(&output, &report)?;
    }
    println!(
        "Manager client band records method={method} images=240 full_client=96x64 nonclient_band=6 boundary_rows_per_side=120 target_inset=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
