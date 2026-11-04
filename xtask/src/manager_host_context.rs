// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Original/Rust factory observations. Preserve native apartment/DPI effects;
//! paired native rejection does not establish successful capture support.
use crate::{Result, records};
use std::{fs, path::Path};

fn table(path: &Path, rows: usize, columns: usize) -> Result<Vec<Vec<i64>>> {
    let text = fs::read_to_string(path)?;
    let values = text
        .lines()
        .map(|line| {
            line.split('\t')
                .map(|value| {
                    let parsed = value.parse::<i64>()?;
                    if parsed.to_string() != value {
                        return Err("Noncanonical host-context scalar".into());
                    }
                    Ok(parsed)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    if !text.ends_with('\n')
        || values.len() != rows
        || values.iter().any(|row| row.len() != columns)
    {
        return Err("Incomplete host-context matrix".into());
    }
    Ok(values)
}

pub fn run(args: &[String]) -> Result<()> {
    run_variant(args, false)
}

pub fn run_whole(args: &[String]) -> Result<()> {
    run_variant(args, true)
}

fn run_variant(args: &[String], complete: bool) -> Result<()> {
    let (mask, mode, dpi, path, check) = match args {
        [mask, mode, dpi, path] => (mask, mode, dpi, path, false),
        [mask, mode, dpi, path, flag] if flag == "--check-only" => (mask, mode, dpi, path, true),
        _ => {
            return Err(
                "expected win32-host-context <mask> <mode> <dpi> <external records> [--check-only]"
                    .into(),
            );
        }
    };
    let method: i64 = mask.parse()?;
    let dpi: i64 = dpi.parse()?;
    if ![0, 1, 2, 4, 8, 16, 32, 63, 64].contains(&method)
        || ![1, 2, 4].contains(&dpi)
        || !matches!(
            mode.as_str(),
            "uninitialized" | "mta" | "mta-dispatcher" | "sta"
        )
    {
        return Err("Unknown host-context case".into());
    }
    let root = records::external_root(Path::new(path))?;
    let observed_selection = complete && root.join("source/whole-source-selected.tsv").is_file();
    if complete && method == 63 && !observed_selection {
        return Err("Mixed complete-class method requires native winner observations".into());
    }
    let mut artifacts = Vec::new();
    let mut observations = Vec::new();
    let mut previous = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let states = table(&directory.join("states.tsv"), 14, 6)?;
        let caller = states[0][5];
        if caller == 0 {
            return Err("Missing host-context caller".into());
        }
        for (stage, row) in states.iter().enumerate() {
            i32::try_from(row[1])?;
            u32::try_from(row[5])?;
            if row[0] != i64::try_from(stage)?
                || row[5] != caller
                || !(1..=5).contains(&row[4])
                || !(row[1] == -2147221008 && row[2..4] == [-1, -1]
                    || row[1] == 0 && (0..=3).contains(&row[2]) && (0..=7).contains(&row[3]))
            {
                return Err("Invalid host-context observation/thread/width".into());
            }
        }
        if states[0][1..4] != [-2147221008, -1, -1]
            || states[1][4] != dpi
            || states[13][4] != states[0][4]
        {
            return Err("Fresh host or explicit DPI preparation/restoration differs".into());
        }
        let prepared = &states[1][1..4];
        let valid_host = match mode.as_str() {
            "uninitialized" => prepared == [-2147221008, -1, -1],
            "sta" => prepared[0] == 0 && [0, 3].contains(&prepared[1]) && prepared[2] == 0,
            _ => prepared == [0, 1, 0],
        };
        if !valid_host {
            return Err("Explicit host apartment was not established".into());
        }
        let calls = table(&directory.join("calls.tsv"), 3, 3)?;
        for (index, row) in calls.iter().enumerate() {
            if row[0] != i64::try_from(index)?
                || ![0, 2, 3].contains(&row[1])
                || ![0, 1].contains(&row[2])
                || row[1] != 0 && row[2] != 0
            {
                return Err("Invalid native host-context call result".into());
            }
        }
        let expected_calls = vec![
            vec![0, 0, i64::from(method != 64)],
            vec![1, 0, i64::from(![0, 64].contains(&method))],
            vec![2, 0, 1],
        ];
        if calls != expected_calls {
            return Err("Native host-context call/control outcomes differ".into());
        }
        let setup = table(&directory.join("setup.tsv"), 1, 31)?;
        for &value in &setup[0][..18] {
            i32::try_from(value)?;
        }
        let s = &setup[0];
        for index in [18, 19, 25, 26] {
            u32::try_from(s[index])?;
        }
        for index in [21, 22, 23, 24, 27, 28, 29, 30] {
            i32::try_from(s[index])?;
        }
        if s[20] < 0
            || s[26] == 0
            || s[0] > s[4]
            || s[1] > s[5]
            || s[2] < s[6]
            || s[3] < s[7]
            || s[2] <= s[0]
            || s[3] <= s[1]
        {
            return Err("Invalid native host-context desktop metadata".into());
        }
        let x = s[4].checked_add(64).ok_or("context x overflow")?;
        let y = s[5].checked_add(64).ok_or("context y overflow")?;
        if s[8..18] != [x - 6, y - 6, x + 102, y + 70, 0, 0, 96, 64, x, y]
            || s[18] != 0x9400_0000
            || s[19] & 8 != 8
            || s[27..31] != [1, 0, 108, 76]
        {
            return Err("Host-context full-client fixture differs".into());
        }
        let image = table(&directory.join("image.tsv"), 1, 10)?.remove(0);
        let selected = image[0];
        if ![0, 1, 2, 4, 8, 16, 32].contains(&selected) || selected & method != selected {
            return Err("Host-context selection outside original mask".into());
        }
        for &value in &image[1..3] {
            i32::try_from(value)?;
        }
        for &value in &image[3..9] {
            usize::try_from(value)?;
        }
        let bytes = fs::read(directory.join("current.bgr"))?;
        let held = fs::read(directory.join("held.bgr"))?;
        if bytes != held || u64::try_from(bytes.len())? != u64::try_from(image[4])? {
            return Err("Host-context held bytes/span changed".into());
        }
        if selected == 0 {
            if !bytes.is_empty()
                || image[1..5] != [0, 0, 0, 0]
                || image[9] != 0
                || calls[1][1..] != [0, 0]
            {
                return Err("Unselected host-context method published an image".into());
            }
        } else {
            let [
                width,
                height,
                stride,
                length,
                scope_width,
                scope_height,
                ox,
                oy,
                known,
            ]: [i64; 9] = image[1..].try_into()?;
            if width <= 0
                || height <= 0
                || stride != width.checked_mul(3).ok_or("context stride overflow")?
                || length
                    != stride
                        .checked_mul(height)
                        .ok_or("context length overflow")?
                || width != scope_width
                || height != scope_height
                || ox < 0
                || oy < 0
                || ox.checked_add(96).ok_or("context ROI overflow")? > width
                || oy.checked_add(64).ok_or("context ROI overflow")? > height
                || known != 1
                || selected != 4 && [width, height, ox, oy] != [96, 64, 0, 0]
                || calls[0][1..] != [0, 1]
                || calls[1][1..] != [0, 1]
            {
                return Err(
                    "Selected native method did not deliver a complete successful known image"
                        .into(),
                );
            }
            for y in 0..64_i64 {
                for x in 0..96_i64 {
                    let expected = if x < 32 {
                        [0x1f, 0x53, 0xa7]
                    } else if y < 32 {
                        [0xb9, 0x4d, 0x27]
                    } else {
                        [0x3b, 0xe2, 0x81]
                    };
                    let offset = usize::try_from(((oy + y) * width + ox + x) * 3)?;
                    if bytes.get(offset..offset + 3) != Some(expected.as_slice()) {
                        return Err("Host-context complete target pixels differ".into());
                    }
                }
            }
        }
        if [1, 2, 4, 8, 16, 32, 63].contains(&method) && selected == 0 {
            return Err("Positive native host-context method failed selection".into());
        }
        if fs::read_to_string(directory.join("completed.txt"))?
            != "factory_destroyed=1\nmodule_unloaded=1\nfixture_joined=1\nimages_released=1\nhost_unregistered=1\ndispatcher_closed=1\nexplicit_com_balanced=1\n"
            || fs::read_to_string(root.join(format!("{side}.exit")))? != "0\n"
        {
            return Err("Host-context child/shutdown incomplete".into());
        }
        let normalized: Vec<_> = states.iter().map(|row| row[..5].to_vec()).collect();
        let pairing = (normalized, calls.clone(), s[..8].to_vec());
        if let Some(previous) = previous.replace(pairing.clone())
            && previous != pairing
        {
            return Err("Original/Rust call results or COM/DPI boundary effects differ".into());
        }
        if complete && side == "source" {
            let types = table(&directory.join("whole-source-types.tsv"), 2, 9)?;
            for (stage, row) in types.iter().enumerate() {
                u32::try_from(row[8])?;
                if row[..6] != [i64::try_from(stage)?, 0x4d465357, 1, 0, 0, 1]
                    || row[6] < 8
                    || row[7] != 8
                    || row[8] != caller
                {
                    return Err("Complete original Win32 class or exclusion counters differ".into());
                }
            }
            let configuration = if observed_selection {
                let selection = table(&directory.join("whole-source-selected.tsv"), 4, 6)?;
                let pointer = selection[1][4];
                for (stage, row) in selection.iter().enumerate() {
                    u32::try_from(row[5])?;
                    usize::try_from(row[4])?;
                    if row[..3] != [i64::try_from(stage)?, 0x4d465353, 1]
                        || row[5] != caller
                        || row[3] != if stage == 0 { 0 } else { selected }
                        || row[4] != if stage == 0 { 0 } else { pointer }
                        || (row[3] == 0) != (row[4] == 0)
                        || row[4] % 8 != 0
                    {
                        return Err("Original selected backend/type/pointer/thread changed".into());
                    }
                }
                artifacts.push(records::artifact(
                    &root,
                    &directory.join("whole-source-selected.tsv"),
                )?);
                format!(
                    "configured_mask={method}\nselection_observer=1\noriginal_class_api=1\ninput_none=1\n"
                )
            } else {
                format!(
                    "single_method_mask={method}\nselection_observer=0\noriginal_class_api=1\ninput_none=1\n"
                )
            };
            if types[0][1..] != types[1][1..]
                || fs::read_to_string(directory.join("whole-source-selection.txt"))?
                    != configuration
            {
                return Err("Original whole-class configuration evidence differs".into());
            }
            for name in ["whole-source-types.tsv", "whole-source-selection.txt"] {
                artifacts.push(records::artifact(&root, &directory.join(name))?);
            }
        }
        for name in [
            "states.tsv",
            "calls.tsv",
            "setup.tsv",
            "image.tsv",
            "current.bgr",
            "held.bgr",
            "completed.txt",
        ] {
            artifacts.push(records::artifact(&root, &directory.join(name))?);
        }
        for suffix in ["stdout", "stderr", "exit"] {
            artifacts.push(records::artifact(
                &root,
                &root.join(format!("{side}.{suffix}")),
            )?);
        }
        observations.push(serde_json::json!({"side":side,"states":states,"calls":calls,"image":image,"capture_succeeded":selected!=0}));
    }
    let mut report = serde_json::json!({"layout_version":1,"method":method,"mode":mode,"dpi":dpi,"artifacts":artifacts,"observations":observations,"full_P1_accepted":false,
        "scope":"Fresh same-binary processes consume real original/Rust Win32 factories, capture-only manager facades and actual virtual calls through the same C++ ABI bridge/Rust loader. Fourteen native COM/DPI boundary observations, exact call outcomes and full target/held bytes; source side effects are preserved rather than repaired. Dispatcher pumping occurs only after capture owners end. Paired rejection is not successful capture. No complete original input manager, compatible-build branch, physical mixed-DPI environment, producer/callback drain or full P1 acceptance."});
    let output = if complete {
        report["original_class_api"] = true.into();
        report["source_selection_evidence"] =
            "single_method_configuration_not_winner_observation".into();
        report["scope"] = "Fresh same-binary source/Rust processes consume the complete unchanged original F Win32ControlUnitMgr and API translation units/headers with Input=None, six actual capture backends and complete actual BackgroundManagedKeyInput, versus the same private Rust manager/factory. Native RTTI matches the actual original class, excluded non-None input/application boundaries are never invoked, fourteen COM/DPI snapshots and complete known/held images match. Source single-bit selection is configuration evidence; no actual mixed-method winner observer is claimed. No original input/application action support, compatible macro, physical mixed DPI, producer/drain or full P1 acceptance.".into();
        if observed_selection {
            report["layout_version"] = 2.into();
            report["source_selection_evidence"] = "actual_owned_backend_RTTI_and_pointer".into();
            report["source_selection_observations"] = 4.into();
            report["scope"]="Complete unchanged original F Win32 class/header/API and actual six capture methods/empty BackgroundManagedKeyInput versus the same Rust manager/factory, Input=None. A native typed member pointer borrows the original owned backend at four serialized boundaries; exact dynamic type and identity are observed without copying its owner, changing original code or calling/pumping native capture. Cold owner is empty; connected/captured/inactive owner and selected type agree with saved full-target/held bytes. Mixed-mask winners may differ with actual measured timings in fresh processes; each winner must belong to its configured mask. Native RTTI/exclusion counters and fourteen COM/DPI snapshots remain required. No input/application actions, compatible macro, physical mixed DPI, producer/drain or full P1 acceptance.".into();
        }
        root.join("manager-whole-host-context-validation.json")
    } else {
        root.join("manager-host-context-validation.json")
    };
    if check {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved host-context report differs".into());
        }
    } else {
        records::create_json(&output, &report)?;
    }
    if complete {
        println!(
            "Win32 whole host context mode={mode} dpi={dpi} method={method} sides=2 snapshots_per_side=14 original_class_api=1 input_none=1 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "Win32 host context mode={mode} dpi={dpi} method={method} sides=2 snapshots_per_side=14 full_P1_acceptance=0 passed"
        );
    }
    Ok(())
}
