// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independently recompute actual GDI-only delegate DLL records; no captures.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-delegate win32-native <external records> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let summary: serde_json::Value = records::read(&root.join("summary.json"))?;
    let get = |key: &str| -> Result<u64> {
        summary[key]
            .as_u64()
            .ok_or_else(|| format!("Missing native delegate count {key}").into())
    };
    for (key, expected) in [
        ("layout_version", 1),
        ("owners_before_release", 160),
        ("owners_after_release", 0),
        ("created", 163),
        ("destroyed", 162),
        ("host_created", 40),
        ("host_destroyed", 40),
        ("host_conversions", 120),
        ("host_captures", 40),
        ("host_inactive", 20),
    ] {
        if get(key)? != expected {
            return Err(format!("Native delegate {key} differs").into());
        }
    }
    if summary.as_object().map(|map| map.len()) != Some(12)
        || get("thread")? == 0
        || get("hwnd")? == 0
    {
        return Err("Invalid native delegate summary identity/shape".into());
    }
    let mut artifacts = vec![records::artifact(&root, &root.join("summary.json"))?];
    let cycle_text = fs::read_to_string(root.join("cycles.tsv"))?;
    let mut rows = cycle_text.lines();
    let mut resources = None;
    for style in ["normal", "prefixed", "bare", "rust"] {
        for cycle in 0..20 {
            let row = rows.next().ok_or("Missing native DLL lifecycle")?;
            let fields: Vec<_> = row.split('\t').collect();
            if fields.len() != 6
                || fields[0] != style
                || fields[1] != cycle.to_string()
                || fields[2] != "1"
                || fields[3] != "1"
                || fields[4] != fields[5]
            {
                return Err("Native DLL sharing/unload/resource evidence differs".into());
            }
            let objects: u32 = fields[4].parse()?;
            if let Some(previous) = resources.replace(objects)
                && previous != objects
            {
                return Err("Native GDI resource baseline changed".into());
            }
            for slot in ["first", "second"] {
                let path = root.join(format!("{style}-{cycle}-{slot}.bgr"));
                pixels(&fs::read(&path)?)?;
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
        if style != "rust" {
            let image = root.join(format!("{style}-source.bgr"));
            pixels(&fs::read(&image)?)?;
            artifacts.push(records::artifact(&root, &image)?);
            let stdout = root.join(format!("{style}-source.stdout"));
            if fs::read_to_string(&stdout)?.replace("\r\n", "\n")
                != "source_native_delegate=1\nwidth=37\nheight=23\nmodule_unloaded=1\n"
            {
                return Err("Original real Boost DLL observation differs".into());
            }
            let stderr = root.join(format!("{style}-source.stderr"));
            if !fs::read(&stderr)?.is_empty() {
                return Err("Original native delegate wrote errors".into());
            }
            artifacts.push(records::artifact(&root, &stdout)?);
            artifacts.push(records::artifact(&root, &stderr)?);
        }
    }
    if rows.next().is_some() {
        return Err("Unexpected native DLL lifecycle".into());
    }
    artifacts.push(records::artifact(&root, &root.join("cycles.tsv"))?);
    let mut counts = BTreeMap::<u32, u32>::new();
    let event_text = fs::read_to_string(root.join("events.tsv"))?;
    for row in event_text.lines() {
        let values: Vec<u64> = row
            .split('\t')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        if values.len() != 6 || values[1] != get("thread")? {
            return Err("Native callback thread/layout differs".into());
        }
        let kind = u32::try_from(values[0])?;
        if kind == 1 {
            if values[2..] != [get("hwnd")?, 1, 0, 0] {
                return Err("Native factory selectors/input None differ".into());
            }
        } else if values[2..] != [0; 4] {
            return Err("Unexpected native event fields".into());
        }
        *counts.entry(kind).or_default() += 1;
    }
    if counts != BTreeMap::from([(1, 163), (2, 162), (3, 163), (4, 80), (5, 162), (6, 363)]) {
        return Err("Native factory/capture/destruction event counts differ".into());
    }
    artifacts.push(records::artifact(&root, &root.join("events.tsv"))?);
    for name in ["post-false.bgr", "post-exception.bgr"] {
        let path = root.join(name);
        if fs::read(&path)? != [7, 9, 11].repeat(6) {
            return Err("Native post-write false/exception pixels differ".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"lifecycles":80,"images":165,"events":1093,"artifacts":artifacts,
        "scope":"Actual fixed-name Windows DLL loads with flags 0, real Boost/LibraryHolder original capture bodies, actual original class ABI, GDI-only original provider and 40 actual Rust manager/GDI contexts through an acceptance-only C++ facade. Shared module references, missing DLL/exports, contained create/connect and post-write capture exceptions, native thread/resource balance, 160 images after provider/window shutdown. Source bridge adds one CPU copy; Rust facade adds two. Generic filesystem aliases, full factory/framework integration, native destroy failure and complete P1 remain unvalidated."});
    let output = root.join("delegate-native-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved native delegate validation differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Win32 real delegate lifecycles=80 images=165 Rust_native_units=40 actual_records_recomputed=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn pixels(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 37 * 23 * 3 {
        return Err("Native delegate image size differs".into());
    }
    for y in 0..23 {
        for x in 0..37 {
            let expected = if x < 37 / 3 {
                [0x1f, 0x53, 0xa7]
            } else if y < 23 / 2 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            if bytes[(y * 37 + x) * 3..(y * 37 + x + 1) * 3] != expected {
                return Err("Native delegate known pixel differs".into());
            }
        }
    }
    Ok(())
}

pub fn run_factory(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-delegate win32-factory <external records> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let cases = fs::read_to_string(root.join("cases.tsv"))?;
    let events = fs::read_to_string(root.join("events.tsv"))?;
    let mut rows = cases.lines();
    let mut event_rows = events.lines();
    let mut thread = None;
    for provider in ["source", "rust"] {
        for hwnd in [0, 0xfedc_ba98_7654_3210u64] {
            for method in [0, 1, 2, 4, 8, 16, 32, 63, u64::MAX] {
                if rows.next() != Some(format!("{provider}\t{hwnd}\t{method}\t1\t1").as_str()) {
                    return Err("Factory selector/owner matrix differs".into());
                }
                for (kind, expected) in [(1, [hwnd, method, 0, 0]), (5, [0; 4])] {
                    let values: Vec<u64> = event_rows
                        .next()
                        .ok_or("Missing actual factory event")?
                        .split('\t')
                        .map(str::parse)
                        .collect::<std::result::Result<_, _>>()?;
                    if values.len() != 6
                        || values[0] != kind
                        || values[1] == 0
                        || values[2..] != expected
                    {
                        return Err("Factory create/destroy sequence or selectors differ".into());
                    }
                    if let Some(previous) = thread.replace(values[1])
                        && previous != values[1]
                    {
                        return Err("Factory thread identity changed".into());
                    }
                }
            }
        }
    }
    if rows.next().is_some() || event_rows.next().is_some() {
        return Err("Unexpected factory capture/connect/event".into());
    }
    if fs::read_to_string(root.join("host-counts.tsv"))?.replace("\r\n", "\n")
        != "18\t18\t0\t0\t0\n"
    {
        return Err("Factory host ownership/no-capture counts differ".into());
    }
    let artifacts = ["cases.tsv", "events.tsv", "host-counts.tsv"]
        .into_iter()
        .map(|name| records::artifact(&root, &root.join(name)))
        .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"cases":36,"native_events":72,"captures":0,"artifacts":artifacts,
        "scope":"Actual original class ABI/source and Rust manager facade create/destroy, nine selector masks with null and unvalidated opaque HWND, exact input None, no connected/capture gate during construction, module final unload. Original backend objects are constructed later on connect; this test does not connect, capture, exercise native six-backend factories or establish full P1 acceptance."});
    let output = root.join("delegate-factory-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved factory validation differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Win32 native factory construction cases=36 events=72 actual_captures=0 independent_records_verified=1 passed"
    );
    Ok(())
}
