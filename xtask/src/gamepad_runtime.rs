// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual whole-Gamepad host control composed with existing native pixel checks.
use crate::{Result, records, windows_delegate};
use std::{fs, path::Path};

fn expected() -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let mut next = 0;
    for cycle in 0..20 {
        let mut event = |name: String, mode, created, destroyed, captures, loaded| {
            writeln!(
                out,
                "{name}\t{mode}\t{created}\t{destroyed}\t{captures}\t{loaded}"
            )
            .unwrap();
        };
        event(
            format!("input.create:{}", cycle % 2),
            0,
            cycle,
            cycle,
            cycle * 3,
            0,
        );
        next += 1;
        let first = next;
        event(format!("input.new:{first}"), 0, cycle, cycle, cycle * 3, 0);
        event(
            format!("input.connect:{first}"),
            0,
            cycle,
            cycle,
            cycle * 3,
            0,
        );
        event(
            format!("input.connected:{first}"),
            0,
            cycle + 1,
            cycle,
            cycle * 3,
            1,
        );
        let mode = cycle % 3 + 1;
        event(
            format!("input.create:{}", cycle % 2),
            mode,
            cycle + 1,
            cycle,
            cycle * 3 + 1,
            1,
        );
        let final_input = if mode == 3 {
            first
        } else {
            next += 1;
            event(
                format!("input.new:{next}"),
                mode,
                cycle + 1,
                cycle,
                cycle * 3 + 1,
                1,
            );
            event(
                format!("input.drop:{first}"),
                mode,
                cycle + 1,
                cycle,
                cycle * 3 + 1,
                1,
            );
            event(
                format!("input.connect:{next}"),
                mode,
                cycle + 1,
                cycle,
                cycle * 3 + 1,
                1,
            );
            next
        };
        event(
            format!("input.reset:{final_input}"),
            mode,
            cycle + 1,
            cycle,
            cycle * 3 + 3,
            1,
        );
        event(
            format!("input.drop:{final_input}"),
            mode,
            cycle + 1,
            cycle + 1,
            cycle * 3 + 3,
            0,
        );
    }
    out
}

pub fn run(args: &[String]) -> Result<()> {
    let (mask, path, check) = match args {
        [mask, path] => (mask, path, false),
        [mask, path, flag] if flag == "--check-only" => (mask, path, true),
        _ => return Err(
            "expected upstream-delegate gamepad-runtime <mask> <external records> [--check-only]"
                .into(),
        ),
    };
    let method: u64 = mask.parse()?;
    if !(1..=63).contains(&method) {
        return Err("Gamepad native runtime requires one nonempty original mask".into());
    }
    // Preserve every original pixel/held/native-event/owner assertion. A
    // host-control success must never substitute for those actual checks.
    let root = records::external_root(Path::new(path))?;
    let mut base_args = vec![mask.clone(), path.clone()];
    if check || root.join("delegate-runtime-validation.json").exists() {
        base_args.push("--check-only".into());
    }
    windows_delegate::verify_runtime(&base_args)?;
    let wanted = expected();
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let file = root.join(format!("{side}/{side}-gamepad-host.tsv"));
        if fs::read_to_string(&file)?.replace("\r\n", "\n") != wanted {
            return Err(format!(
                "Gamepad {side} host/native counters, retention or teardown differ"
            )
            .into());
        }
        artifacts.push(records::artifact(&root, &file)?);
    }
    artifacts.push(records::artifact(
        &root,
        &root.join("delegate-runtime-validation.json"),
    )?);
    let report = serde_json::json!({"layout_version":1,"method":method,"cycles_per_side":20,"failed_reconnects_per_side":20,"host_rows_per_side":182,"input_actions":0,"full_P1_accepted":false,"artifacts":artifacts,
        "scope":"Complete unchanged F Gamepad class/header/C factories with an explicit opaque input-owner and identified directory boundary, versus the same Rust GamepadCapture/Native delegate. Actual provider DLL/factory/virtual capture and original 240-image, selected-scope, held-output and native-owner assertions are recomputed first without changes. Each cycle captures before and twice after one host false/connect-error/create-error; native counter snapshots require no delegate replacement, reset while its module/unit are live, then input release after unit destruction/module unload. ViGEm/input actions, arbitrary reset/destructor failures, full callbacks/host matrices and complete P1 remain unaccepted."});
    let output = root.join("gamepad-runtime-validation.json");
    if check && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Gamepad native runtime report differs".into());
        }
    } else if !check {
        records::create_json(&output, &report)?;
    }
    println!(
        "Gamepad native outer records method={method} cycles_per_side=20 failed_reconnects_per_side=20 host_rows_per_side=182 native_counter_snapshots=1 files=3 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
