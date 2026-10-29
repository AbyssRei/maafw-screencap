// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recompute actual original/Rust native factory images and lifecycle evidence.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};
fn fields(path: &Path) -> Result<BTreeMap<String, u64>> {
    let mut output = BTreeMap::new();
    for line in fs::read_to_string(path)?.lines() {
        let (key, value) = line.split_once('=').ok_or("Malformed runtime summary")?;
        if output.insert(key.to_owned(), value.parse()?).is_some() {
            return Err("Duplicate runtime summary field".into());
        }
    }
    Ok(output)
}
fn expect(fields: &BTreeMap<String, u64>, key: &str, value: u64) -> Result<()> {
    if fields.get(key) != Some(&value) {
        return Err(format!("Runtime {key} differs").into());
    }
    Ok(())
}
struct Cycle {
    selected: u64,
    scope: [usize; 4],
}
fn selections(path: &Path, mask: u64) -> Result<Vec<Cycle>> {
    let text = fs::read_to_string(path)?;
    let mut selected = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let values: Vec<u64> = line
            .split('\t')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        if ![3, 7].contains(&values.len())
            || values[0] != u64::try_from(index)?
            || !values[1].is_power_of_two()
            || values[1] & mask != values[1]
            || values[2] != 1
        {
            return Err("Runtime winner/module-unload row differs".into());
        }
        let scope = if values.len() == 7 {
            [
                usize::try_from(values[3])?,
                usize::try_from(values[4])?,
                usize::try_from(values[5])?,
                usize::try_from(values[6])?,
            ]
        } else {
            // Initial successful window-only records predate explicit output
            // geometry. Their executable checked exactly96x64. Never infer a
            // monitor/combined-mask layout or reinterpret either failed run.
            if ![1, 2, 16, 32].contains(&mask) {
                return Err("Explicit output/ROI geometry required for this runtime mask".into());
            }
            [96, 64, 0, 0]
        };
        if scope[0] == 0
            || scope[1] == 0
            || scope.iter().any(|&value| value > i32::MAX as usize)
            || scope[2]
                .checked_add(96)
                .is_none_or(|right| right > scope[0])
            || scope[3]
                .checked_add(64)
                .is_none_or(|bottom| bottom > scope[1])
        {
            return Err("Runtime known-window ROI not inside output".into());
        }
        if values[1] != 4 && scope != [96, 64, 0, 0] {
            return Err("Window runtime method changed output scope".into());
        }
        selected.push(Cycle {
            selected: values[1],
            scope,
        });
    }
    if selected.len() != 20 {
        return Err("Runtime requires all twenty complete lifecycles".into());
    }
    Ok(selected)
}
fn pixels(bytes: &[u8], scope: [usize; 4]) -> Result<()> {
    if Some(bytes.len())
        != scope[0]
            .checked_mul(scope[1])
            .and_then(|value| value.checked_mul(3))
    {
        return Err("Runtime target pixel span differs".into());
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
            let offset = ((y + scope[3]) * scope[0] + x + scope[2]) * 3;
            if bytes[offset..offset + 3] != expected {
                return Err("Runtime known target pixels differ".into());
            }
        }
    }
    Ok(())
}
fn events(
    path: &Path,
    mask: u64,
    source: bool,
    selected: &[Cycle],
    caller: Option<u64>,
) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let values: Vec<Vec<u64>> = text
        .lines()
        .map(|line| line.split('\t').map(str::parse).collect())
        .collect::<std::result::Result<_, _>>()?;
    let mut rows = values.iter();
    let mut thread = caller;
    let mut hwnd = None;
    let mut next = |kind: u64, arguments: Option<[u64; 4]>| -> Result<()> {
        let row = rows.next().ok_or("Missing native runtime event")?;
        if row.len() != 6 || row[0] != kind || row[1] == 0 {
            return Err("Native runtime event order/type differs".into());
        }
        if let Some(previous) = thread.replace(row[1])
            && previous != row[1]
        {
            return Err("Native runtime event caller changed".into());
        }
        if let Some(arguments) = arguments {
            if row[2..] != arguments {
                return Err("Native runtime event payload differs".into());
            }
        } else {
            if row[2] == 0 || row[3..] != [mask, 0, 0] {
                return Err("Native runtime factory selectors/input None differ".into());
            }
            if let Some(previous) = hwnd.replace(row[2])
                && previous != row[2]
            {
                return Err("Native runtime target changed".into());
            }
        }
        Ok(())
    };
    for cycle in selected {
        let winner = cycle.selected;
        next(1, None)?;
        next(2, Some([0; 4]))?;
        if source && mask & 1 != 0 {
            next(6, Some([0; 4]))?;
            next(6, Some([0; 4]))?;
        }
        if source {
            next(7, Some([winner, 0, 0, 0]))?;
        }
        for _ in 0..3 {
            next(3, Some([0; 4]))?;
            if source && winner == 1 {
                next(6, Some([0; 4]))?;
            }
        }
        next(4, Some([0; 4]))?;
        next(5, Some([0; 4]))?;
    }
    if rows.next().is_some() {
        return Err("Unexpected native runtime event or unclosed owner".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (mask, path, check_only) =
        match args {
            [mask, path] => (mask, path, false),
            [mask, path, flag] if flag == "--check-only" => (mask, path, true),
            _ => return Err(
                "expected upstream-delegate win32-runtime <mask> <external case> [--check-only]"
                    .into(),
            ),
        };
    let mask: u64 = mask.parse()?;
    if ![1, 2, 4, 8, 16, 32, 63].contains(&mask) {
        return Err("Unknown native runtime mask".into());
    }
    let root = records::external_root(Path::new(path))?;
    let source = fields(&root.join("source/source-summary.txt"))?;
    let rust = fields(&root.join("rust/rust-summary.txt"))?;
    for report in [&source, &rust] {
        for (key, value) in [
            ("method", mask),
            ("cycles", 20),
            ("held", 60),
            ("window_closed", 1),
            ("dispatcher_closed", 1),
            ("known_pixels", 1),
        ] {
            expect(report, key, value)?;
        }
    }
    if source.len() != 6 || rust.len() != 13 {
        return Err("Runtime summary shape differs".into());
    }
    expect(&rust, "context_created", 20)?;
    expect(&rust, "context_destroyed", 20)?;
    let caller = *rust.get("caller_thread").ok_or("Missing runtime caller")?;
    let ui = *rust.get("ui_thread").ok_or("Missing runtime UI owner")?;
    if caller == 0 || ui == 0 || caller == ui || rust.get("conversions").copied().unwrap_or(0) == 0
    {
        return Err("Runtime UI/conversion evidence absent".into());
    }
    let source_selected = selections(&root.join("source/source-cycles.tsv"), mask)?;
    let rust_selected = selections(&root.join("rust/rust-cycles.tsv"), mask)?;
    events(
        &root.join("source/source-events.tsv"),
        mask,
        true,
        &source_selected,
        None,
    )?;
    events(
        &root.join("rust/rust-events.tsv"),
        mask,
        false,
        &rust_selected,
        Some(caller),
    )?;
    if !fs::read(root.join("source.stderr"))?.is_empty() {
        return Err("Original runtime process reported errors".into());
    }
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let selected = if side == "source" {
            &source_selected
        } else {
            &rust_selected
        };
        for (cycle, observation) in selected.iter().enumerate() {
            for shot in 0..3 {
                let current = root.join(format!("{side}/{side}-{cycle}-{shot}.bgr"));
                let held = root.join(format!("{side}/{side}-held-{}.bgr", cycle * 3 + shot));
                let bytes = fs::read(&current)?;
                pixels(&bytes, observation.scope)?;
                if fs::read(&held)? != bytes {
                    return Err(
                        "Runtime retained frame changed after unit/module/window close".into(),
                    );
                }
                artifacts.push(records::artifact(&root, &current)?);
                artifacts.push(records::artifact(&root, &held)?);
            }
        }
        for suffix in ["summary.txt", "cycles.tsv", "events.tsv"] {
            artifacts.push(records::artifact(
                &root,
                &root.join(format!("{side}/{side}-{suffix}")),
            )?);
        }
    }
    for name in ["source.stdout", "source.stderr"] {
        artifacts.push(records::artifact(&root, &root.join(name))?);
    }
    let report = serde_json::json!({"layout_version":1,"method":mask,"lifecycles":40,"images":240,"artifacts":artifacts,
        "scope":"Actual fixed-source Boost/LibraryHolder/outer delegation, original Create/Destroy bodies and original manager/backend virtual calls versus the private Rust manager/factory through a real DLL/class facade. Each side owns an independent known-pattern 96x64 UI-thread window, MTA/current-thread dispatcher, 20 native unit/module lifecycles and 60 independent CPU frames retained across later captures and complete window/module closure. DesktopDup preserves the ENTIRE monitor output; its explicit native monitor/client geometry selects only the owned-window ROI for known-color validation. Other monitor pixels have full byte-span and retained-identity checks, not a deterministic ambient-desktop color oracle. Other methods preserve window-only output. Native measured winners may differ; both must be inside the requested mask. GDI counts/source type identity and all native unit/caller/selector events are recomputed. Original waiting/cache/selection implementation remains unchanged. This visible-target/unrotated-output slice does not prove all host/format/failure/pseudo-minimize/resource matrices or complete P1. Extra boundary safety copies remain explicit."});
    let output = root.join("delegate-runtime-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved native runtime verification differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Win32 actual factory runtime method={mask} source_lifecycles=20 Rust_lifecycles=20 images=240 native_events_and_owners_verified=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
