// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recompute actual host-init/capture/close/held-pixel records. No native capture
//! or host authorization is performed by this independent record checker.
use crate::{
    Result,
    pipewire_trace::{pixels, task_identities},
    records,
};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace pipewire-host <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut payloads = 0;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let cases_path = directory.join(format!("{side}-cases.tsv"));
        let case_text = fs::read_to_string(&cases_path)?;
        let task_count = cases(&case_text)?;
        artifacts.push(records::artifact(&root, &cases_path)?);
        let baseline_path = directory.join("baseline-task-ids.txt");
        let baseline = task_identities(&fs::read_to_string(&baseline_path)?)?;
        if u64::try_from(baseline.len())? != task_count {
            return Err("host baseline task set/count differs".into());
        }
        artifacts.push(records::artifact(&root, &baseline_path)?);
        for cycle in 0..20 {
            let closed_path = directory.join(format!("closed-{cycle}-task-ids.txt"));
            if task_identities(&fs::read_to_string(&closed_path)?)? != baseline {
                return Err("host close substituted or retained task identities".into());
            }
            artifacts.push(records::artifact(&root, &closed_path)?);
            for call in 0..3 {
                let path = directory.join(format!("host-{cycle}-call-{call}.bgr"));
                let bgr = fs::read(&path)?;
                let serial = marker(&bgr, true)?;
                pixels(&bgr, serial, true)?;
                artifacts.push(records::artifact(&root, &path)?);
                payloads += 1;
                let held = directory.join(format!("held-{}.{side}.bgr", cycle * 3 + call));
                if fs::read(&held)? != bgr {
                    return Err("host BGR owner changed after native/host destruction".into());
                }
                artifacts.push(records::artifact(&root, &held)?);
                payloads += 1;
                if side == "rust" {
                    let path = directory.join(format!("host-{cycle}-call-{call}.rgb"));
                    let rgb = fs::read(&path)?;
                    pixels(&rgb, serial, false)?;
                    let (rgb_rows, rgb_tail) = rgb.as_chunks::<3>();
                    let (bgr_rows, bgr_tail) = bgr.as_chunks::<3>();
                    if !rgb_tail.is_empty()
                        || !bgr_tail.is_empty()
                        || rgb_rows.len() != bgr_rows.len()
                    {
                        return Err("host conversion channel span differs".into());
                    }
                    for (rgb, bgr) in rgb_rows.iter().zip(bgr_rows) {
                        if *bgr != [rgb[2], rgb[1], rgb[0]] {
                            return Err("host conversion bytes differ".into());
                        }
                    }
                    artifacts.push(records::artifact(&root, &path)?);
                    payloads += 1;
                    let held = directory.join(format!("held-{}.rust.rgb", cycle * 3 + call));
                    if fs::read(&held)? != rgb {
                        return Err("host raw CPU owner changed after close".into());
                    }
                    artifacts.push(records::artifact(&root, &held)?);
                    payloads += 1;
                }
            }
        }
        let image_count = fs::read_dir(&directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext == "bgr" || ext == "rgb")
            })
            .count();
        if image_count != if side == "rust" { 240 } else { 120 } {
            return Err("host matrix contains missing or extra pixel files".into());
        }
    }
    if payloads != 360 {
        return Err("host payload coverage differs from the complete matrix".into());
    }
    let report = serde_json::json!({"layout_version":1,"source_objects":20,"rust_objects":20,"init_true_per_side":7,
        "init_false_per_side":7,"init_error_per_side":6,"captures_per_side":60,"payloads":payloads,"artifacts":artifacts,
        "scope":"Actual owned-daemon/supplied-socket source/Rust PipeWire manager captures with explicit host init true/false/error scripts and real owned host socketpairs. Non-None config and original init order/features/inactive/capture-after-host-failure are retained over the same native backend. Held source/Rust BGR and Rust raw CPU bytes are verified after native/host destruction, with exact task sets, scalar host primary/cleanup errors and native-before-host close evidence. Controlled helper end precedes native connect; Rust uses the migrated legacy KWin constructor. No input actions, real DBus/desktop Portal authorization, full original compat C ABI, general failure ownership, GPU, scheduling or full P1 acceptance."});
    let destination = root.join("host-validation.json");
    if check_only && destination.exists() {
        let saved: serde_json::Value = records::read(&destination)?;
        if saved != report {
            return Err("host validation report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&destination, &report)?;
    }
    println!(
        "PipeWire host manager sides=2 objects=40 captures=120 payloads=360 true_false_error=14,14,12 exact_pixels_tasks_and_close_order=1 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn marker(bytes: &[u8], bgr: bool) -> Result<u64> {
    if bytes.len() != 2016 {
        return Err("incomplete host pixel target".into());
    }
    let value = if bgr {
        u64::from(bytes[2]) << 16 | u64::from(bytes[1]) << 8 | u64::from(bytes[0])
    } else {
        u64::from(bytes[0]) << 16 | u64::from(bytes[1]) << 8 | u64::from(bytes[2])
    };
    if value == 0 {
        return Err("missing producer serial in host capture".into());
    }
    Ok(value)
}
fn cases(text: &str) -> Result<u64> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 20 {
        return Err("incomplete host case matrix".into());
    }
    let (mut baseline, mut tasks) = (None, None);
    for (cycle, line) in rows.iter().enumerate() {
        let v = line
            .split('\t')
            .map(|value| {
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("non-decimal host record".into());
                }
                Ok(value.parse::<u64>()?)
            })
            .collect::<Result<Vec<_>>>()?;
        let cycle = u64::try_from(cycle)?;
        let outcome = cycle % 3;
        let expected = [
            cycle,
            outcome,
            if outcome == 0 {
                1
            } else if outcome == 1 {
                0
            } else {
                2
            },
            if outcome == 2 { 97 } else { 0 },
            1,
            1,
            1,
            1,
            1,
            2,
            0,
            1,
        ];
        if v.len() != 22
            || v[..12] != expected
            || v[12] == 0
            || v[12] != v[13]
            || v[14] == 0
            || v[15..] != [1, 0, 0, 0, 3, 1, 1]
            || baseline.is_some_and(|fd| fd != v[12])
            || tasks.is_some_and(|count| count != v[14])
        {
            return Err(
                "host result/primary-cleanup error/close order/resource evidence differs".into(),
            );
        }
        baseline = Some(v[12]);
        tasks = Some(v[14]);
    }
    tasks.ok_or_else(|| "missing host task baseline".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_lost_host_primary_errors_reordered_cleanup_and_replaced_resources() {
        let valid = (0..20)
            .map(|cycle| {
                let o = cycle % 3;
                format!(
                    "{cycle}\t{o}\t{}\t{}\t1\t1\t1\t1\t1\t2\t0\t1\t9\t9\t2\t1\t0\t0\t0\t3\t1\t1\n",
                    if o == 0 {
                        1
                    } else if o == 1 {
                        0
                    } else {
                        2
                    },
                    if o == 2 { 97 } else { 0 }
                )
            })
            .collect::<String>();
        assert_eq!(cases(&valid).unwrap(), 2);
        for bad in [
            valid.replacen("2\t2\t2\t97", "2\t2\t0\t0", 1),
            valid.replacen("\t1\t2\t0\t1\t9", "\t0\t2\t0\t1\t9", 1),
            valid.replacen("\t9\t9\t2", "\t9\t10\t2", 1),
            valid.replacen("\t0\t0\t0\t3\t1\t1", "\t0\t5\t0\t3\t1\t1", 1),
            valid.lines().take(19).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(cases(&bad).is_err());
        }
    }
    #[test]
    fn rejects_missing_or_zero_marker_before_decoding_target_pixels() {
        assert!(marker(&[], true).is_err());
        assert!(marker(&[0; 2016], true).is_err());
        let mut bytes = [0; 2016];
        bytes[..3].copy_from_slice(&[3, 2, 1]);
        assert_eq!(marker(&bytes, true).unwrap(), 0x010203);
    }
}
