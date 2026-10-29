// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual fixed-source delegate model records, not native DLL acceptance.
use crate::{Result, records};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
#[path = "../../tests/upstream-parity/windows-delegate/cases.rs"]
mod cases;
#[path = "windows_delegate_native.rs"]
mod native;
#[path = "windows_delegate_runtime.rs"]
mod runtime;

pub const ORACLE_INPUTS: [&str; 22] = [
    "mfs_windows_delegate_model.exe",
    "windows-delegate-loader.inc",
    "windows-delegate-source.inc",
    "windows-delegate-condition.inc",
    "windows-delegate-LibraryHolder.h",
    "windows-delegate-scope.txt",
    "windows-delegate-CMakeCache.txt",
    "windows-delegate-toolchain-Release.txt",
    "mfs_delegate_bridge.dll",
    "mfs_delegate_source.dll",
    "mfs_delegate_missing_create.dll",
    "mfs_delegate_missing_destroy.dll",
    "mfs_delegate_rust.dll",
    "mfs_delegate_native_source.exe",
    "windows-delegate-native-scope.txt",
    "windows-delegate-native-manager-source.inc",
    "windows-delegate-native-manager-enum.inc",
    "windows-delegate-native-manager-time.inc",
    "windows-delegate-native-toolchain-Release.txt",
    "windows-delegate-factory-create.inc",
    "windows-delegate-factory-destroy.inc",
    "mfs_delegate_runtime_source.exe",
];
pub fn require_oracle(inputs: &[records::Artifact]) -> Result<()> {
    for name in ORACLE_INPUTS {
        if inputs
            .iter()
            .filter(|input| input.path.rsplit('/').next() == Some(name))
            .count()
            != 1
        {
            return Err(format!("Offline manager delegation requires one archived {name}; prepare a new online manager batch").into());
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    if args.first().is_some_and(|arg| arg == "win32-runtime") {
        return runtime::run(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "win32-factory") {
        return native::run_factory(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "win32-native") {
        return native::run(&args[1..]);
    }
    let (path, check_only) = match args {
        [kind, path] if kind == "win32" => (path, false),
        [kind, path, flag] if kind == "win32" && flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-delegate win32 <external model records> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let matrix = cases::matrix();
    let table = root.join("cases.tsv");
    let text = fs::read_to_string(&table)?;
    let rows: Vec<_> = text.lines().collect();
    if rows.len() != matrix.len() || rows.len() != 274 {
        return Err("Delegate model requires all 274 original/Rust records".into());
    }
    let mut artifacts = vec![records::artifact(&root, &table)?];
    for (index, case) in matrix.into_iter().enumerate() {
        if rows[index] != format!("{index}\t{}", case.arguments().join("\t")) {
            return Err("Delegate matrix selector/fault coverage differs".into());
        }
        let source = root.join(format!("case-{index}.source.txt"));
        let rust = root.join(format!("case-{index}.rust.txt"));
        let native = fs::read_to_string(&source)?.replace("\r\n", "\n");
        let migrated = fs::read_to_string(&rust)?.replace("\r\n", "\n");
        if native != migrated {
            return Err(format!("Delegate source/Rust record mismatch at {index}").into());
        }
        validate(case, &native)?;
        for path in [
            source,
            rust,
            root.join(format!("case-{index}.source.stderr")),
        ] {
            artifacts.push(records::artifact(&root, &path)?);
        }
    }
    let report = serde_json::json!({"layout_version":1,"cases":274,"phases":548,"artifacts":artifacts,
        "scope":"Exact selected capture delegate/loader and original MaaUtils LibraryHolder with real original ControlUnitAPI, boost::function, shared control blocks, mutex and OpenCV Mat owners. DLL/runtime/log/backend outcomes are scripted; native module loading, real Win32 capture, full shared-loader/control-block-allocation failures and complete delegate/P1 acceptance remain unvalidated. Dead source views are observed without virtual calls; Rust refusal is a safety adaptation, not proof of defined original calls. Input/gamepad behavior is excluded."});
    let output = root.join("delegate-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved delegate model validation differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Win32 delegate cases=274 phases=548 source_rust_records_verified=1 pixels_owners_events_verified=1 stale_calls_not_executed=1 native_dll_acceptance=0 passed"
    );
    Ok(())
}
fn fields(text: &str) -> Result<BTreeMap<&str, &str>> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (name, value) = line.split_once('=').ok_or("Malformed delegate field")?;
        if fields.insert(name, value).is_some() {
            return Err("Duplicate delegate field".into());
        }
    }
    if fields.len() != 22 {
        return Err("Incomplete delegate record".into());
    }
    Ok(fields)
}
fn get<'a>(fields: &BTreeMap<&str, &'a str>, name: &str) -> Result<&'a str> {
    fields
        .get(name)
        .copied()
        .ok_or_else(|| format!("Missing delegate field {name}").into())
}
fn expect(fields: &BTreeMap<&str, &str>, name: &str, expected: impl ToString) -> Result<()> {
    if get(fields, name)? != expected.to_string() {
        return Err(format!("Delegate field {name} differs").into());
    }
    Ok(())
}
fn unit_id(value: &str) -> Result<u8> {
    let id: u8 = value.parse()?;
    if id == 0 || id.to_string() != value {
        return Err("Invalid delegate unit identity".into());
    }
    Ok(id)
}
fn validate(case: cases::Case, text: &str) -> Result<()> {
    let fields = fields(text)?;
    expect(&fields, "initial_capture", 0)?;
    expect(&fields, "initial_inactive", 1)?;
    expect(&fields, "owners_closed", 1)?;
    let mut module = false;
    let mut live = BTreeSet::new();
    let mut next = 0u8;
    let mut captures = Vec::new();
    let mut inactive = Vec::new();
    let mut load_calls = 0u32;
    for event in get(&fields, "events")?
        .split(',')
        .filter(|event| !event.is_empty())
    {
        match event {
            "module.load" if !module && live.is_empty() => {
                load_calls = load_calls
                    .checked_add(1)
                    .ok_or("Delegate load count overflow")?;
            }
            "symbol.create" if !module => {
                module = true;
            }
            "symbol.destroy" if module => {}
            "module.unload" if module && live.is_empty() => {
                module = false;
            }
            _ if event.starts_with("unit.create:") && module => {
                if event != format!("unit.create:{}:{}:0:0", case.hwnd, case.method) {
                    return Err("Delegate did not forward exact selectors/input None".into());
                }
            }
            _ if event.starts_with("unit.connect:") && module => {
                let id = unit_id(&event[13..])?;
                next = next.checked_add(1).ok_or("Delegate identity overflow")?;
                if id != next || !live.insert(id) {
                    return Err("Delegate native owner identity repeated".into());
                }
            }
            _ if event.starts_with("unit.drop:") && module => {
                if !live.remove(&unit_id(&event[10..])?) {
                    return Err("Delegate owner destroyed twice".into());
                }
            }
            _ if event.starts_with("unit.capture:") && module => {
                let id = unit_id(&event[13..])?;
                if !live.contains(&id) {
                    return Err("Capture dispatched through a destroyed delegate".into());
                }
                captures.push(id);
            }
            _ if event.starts_with("unit.inactive:") && module => {
                let id = unit_id(&event[14..])?;
                if !live.contains(&id) {
                    return Err("Inactive dispatched through a destroyed delegate".into());
                }
                inactive.push(id);
            }
            _ => return Err(format!("Invalid delegate resource event {event}").into()),
        }
    }
    if module || !live.is_empty() || captures != inactive {
        return Err("Delegate resource/call ownership differs".into());
    }
    if load_calls
        != if case.hwnd != 0 && case.method != 0 {
            2
        } else {
            0
        }
    {
        return Err("Delegate optional selection/load call count differs".into());
    }
    let mut previous_view = false;
    let mut output = "63".repeat(18);
    let mut calls = 0;
    for phase in 0..2 {
        let flag = case.flags[phase];
        let selected = case.hwnd != 0 && case.method != 0;
        let initialized = if !selected {
            3
        } else if [32, 64, 128].contains(&flag) {
            2
        } else {
            u8::from(flag == 0)
        };
        let loader = selected && [0, 32, 64, 128].contains(&flag);
        let loaded = selected && [0, 64, 128].contains(&flag);
        let alive = selected && [0, 128].contains(&flag);
        let view = selected && (alive || (flag != 16 && previous_view));
        for (name, value) in [
            ("init", initialized),
            ("loader", u8::from(loader)),
            ("view", u8::from(view)),
            ("live", u8::from(alive)),
            ("module", u8::from(loaded)),
        ] {
            expect(&fields, &format!("{name}{phase}"), value)?;
        }
        let (captured, deactivated) = if !view {
            (0, 1)
        } else if !alive {
            (4, 4)
        } else {
            let id = *captures.get(calls).ok_or("Missing actual delegate call")?;
            calls += 1;
            if [1, 4, 5].contains(&case.outcome) {
                output = format!("07{id:02x}0b").repeat(6);
            }
            if case.outcome == 2 {
                output = "empty".into();
            }
            (
                match case.outcome {
                    0 | 4 => 0,
                    1 | 2 => 1,
                    3 | 5 => 2,
                    _ => unreachable!(),
                },
                case.inactive,
            )
        };
        expect(&fields, &format!("capture{phase}"), captured)?;
        expect(&fields, &format!("inactive{phase}"), deactivated)?;
        expect(&fields, &format!("output{phase}"), &output)?;
        expect(&fields, &format!("held{phase}"), &output)?;
        previous_view = view;
    }
    if calls != captures.len() {
        return Err("Extra source delegate calls".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_delegation_rejects_missing_or_ambiguous_source_oracles() {
        let inputs: Vec<_> = ORACLE_INPUTS
            .into_iter()
            .map(|name| records::Artifact {
                path: format!("runs/prepared/inputs/{name}"),
                sha256: "0".repeat(64),
            })
            .collect();
        assert!(require_oracle(&inputs).is_ok());
        assert!(require_oracle(&inputs[..6]).is_err());
        let mut duplicated = inputs.clone();
        duplicated.push(inputs[0].clone());
        assert!(require_oracle(&duplicated).is_err());
        assert!(fields("initial_capture=0\ninitial_capture=0\n").is_err());
        assert!(unit_id("01").is_err());
    }
}
