// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent complete-class native records; never performs capture itself.
use crate::{
    Result,
    pipewire_trace::{pixels, task_identities},
    records,
};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace pipewire-whole-manager <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut payloads = 0;
    let mut node = None;
    for (mode, sessions) in [("daemon", 2), ("fd", 1)] {
        for side in ["source", "rust"] {
            let directory = root.join(mode).join(side);
            let mut read = |name: &str| -> Result<Vec<u8>> {
                let path = directory.join(name);
                let bytes = fs::read(&path)?;
                artifacts.push(records::artifact(&root, &path)?);
                Ok(bytes)
            };
            let text = String::from_utf8(read(&format!("{side}-cases.tsv"))?)?;
            let rows = cases(&text, sessions)?;
            if node
                .replace(rows[0].node)
                .is_some_and(|old| old != rows[0].node)
            {
                return Err("whole manager consumers used different producer nodes".into());
            }
            let baseline = task_identities(&String::from_utf8(read("baseline-task-ids.txt")?)?)?;
            if u64::try_from(baseline.len())? != rows[0].tasks {
                return Err("whole manager task baseline count differs".into());
            }
            if side == "source" {
                if read("whole-origin.txt")? != b"whole_original_manager=1 full_original_api=1 real_pipewire=1 real_NoneInput=1 input_actions=0\n" {
                    return Err("whole manager origin/scope differs".into());
                }
                owners(&String::from_utf8(read("source-owners.tsv")?)?, sessions)?;
            }
            for (cycle, row) in rows.iter().enumerate() {
                if task_identities(&String::from_utf8(read(&format!(
                    "closed-{cycle}-task-ids.txt"
                ))?)?)?
                    != baseline
                {
                    return Err("whole manager changed or retained task identities".into());
                }
                if read(&format!("api-{cycle}.uuid"))?
                    != format!("{}_{}_-1", row.fd, row.node).as_bytes()
                    || read(&format!("api-{cycle}.info"))? != metadata(row.fd, row.node).as_bytes()
                {
                    return Err("whole manager actual UUID/info bytes differ".into());
                }
                for session in 0..sessions {
                    for call in 0..3 {
                        let index = cycle * sessions * 3 + session * 3 + call;
                        let stem = format!("manager-{cycle}-session-{session}-call-{call}");
                        let bgr = read(&format!("{stem}.bgr"))?;
                        let serial = marker(&bgr)?;
                        pixels(&bgr, serial, true)?;
                        if read(&format!("held-{index}.bgr"))? != bgr {
                            return Err(
                                "whole manager held BGR changed after native destruction".into()
                            );
                        }
                        payloads += 2;
                        if side == "rust" {
                            let rgb = read(&format!("{stem}.rgb"))?;
                            pixels(&rgb, serial, false)?;
                            if read(&format!("held-{index}.rgb"))? != rgb {
                                return Err("whole manager held raw storage changed after native destruction".into());
                            }
                            let (rgb_rows, rgb_tail) = rgb.as_chunks::<3>();
                            let (bgr_rows, bgr_tail) = bgr.as_chunks::<3>();
                            if !rgb_tail.is_empty() || !bgr_tail.is_empty() {
                                return Err("whole manager conversion has a partial pixel".into());
                            }
                            for (rgb, bgr) in rgb_rows.iter().zip(bgr_rows) {
                                if *bgr != [rgb[2], rgb[1], rgb[0]] {
                                    return Err(
                                        "whole manager native RGB/OpenCV BGR conversion differs"
                                            .into(),
                                    );
                                }
                            }
                            payloads += 2;
                        }
                    }
                }
            }
            let count = fs::read_dir(&directory)?
                .collect::<std::io::Result<Vec<_>>>()?
                .iter()
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .is_some_and(|e| e == "bgr" || e == "rgb")
                })
                .count();
            if count != 20 * sessions * 3 * if side == "source" { 2 } else { 4 } {
                return Err("whole manager has missing or extra pixel files".into());
            }
        }
    }
    if payloads != 1080 {
        return Err("whole manager payload coverage differs".into());
    }
    let report = serde_json::json!({"layout_version":1,"source_objects":40,"rust_objects":40,
        "captures":360,"payloads":payloads,"source_owner_observations":340,"active_features":3,
        "artifacts":artifacts,"scope":"Complete original Linux manager/API/PipeWire/NoneInput and same native Rust implementation, owned padded RGB producer, daemon reconstruction and single offered fd. Exact target bytes, held CPU images, metadata, typed original owner observations and native resource records. No input actions, Portal desktop authorization, arbitrary failures or full P1 acceptance."});
    let path = root.join("whole-manager-validation.json");
    if check_only && path.exists() {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("whole manager report differs from saved data".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "PipeWire whole manager objects=80 captures=360 payloads=1080 source_owner_observations=340 active_features=3 exact_pixels_tasks_and_metadata=1 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
struct Case {
    fd: i32,
    node: u32,
    tasks: u64,
}
fn numbers(line: &str) -> Result<Vec<i64>> {
    line.split('\t')
        .map(|text| {
            if text != "-1" && (text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit())) {
                return Err("invalid whole manager decimal record".into());
            }
            Ok(text.parse()?)
        })
        .collect()
}
fn cases(text: &str, sessions: usize) -> Result<Vec<Case>> {
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 20 {
        return Err("incomplete whole manager lifecycle rows".into());
    }
    let mut result = Vec::new();
    let mut baseline = None;
    for (cycle, line) in lines.iter().enumerate() {
        let v = numbers(line)?;
        if v.len() != 12
            || v[0] != i64::try_from(cycle)?
            || v[3] != i64::try_from(sessions)?
            || v[4..6] != [0, 3]
            || v[6] < 3
            || v[6] != v[7]
            || v[8] < 1
            || v[9] != 1
            || v[10] != i64::from(sessions == 1)
            || v[11] != i64::try_from(sessions * 3)?
            || (sessions == 1 && v[1] < 3)
            || (sessions == 2 && v[1] != -1)
            || v[2] <= 0
            || baseline
                .replace((v[2], v[6], v[8]))
                .is_some_and(|old| old != (v[2], v[6], v[8]))
        {
            return Err("whole manager lifecycle, features or native resources differ".into());
        }
        result.push(Case {
            fd: i32::try_from(v[1])?,
            node: u32::try_from(v[2])?,
            tasks: u64::try_from(v[8])?,
        });
    }
    Ok(result)
}
fn owners(text: &str, sessions: usize) -> Result<()> {
    let lines: Vec<_> = text.lines().collect();
    let per_cycle = 1 + sessions * 5;
    if lines.len() != 20 * per_cycle {
        return Err("incomplete whole manager owner observations".into());
    }
    for (cycle, rows) in lines.chunks_exact(per_cycle).enumerate() {
        let cold = numbers(rows[0])?;
        if cold.len() != 10
            || cold[0] != i64::try_from(cycle)?
            || cold[1..3] != [-1, 0]
            || cold[3] <= 0
            || cold[4..] != [0, 0, 0, 0, 0, 0]
        {
            return Err("whole manager cold state/actual identity differs".into());
        }
        for session in 0..sessions {
            let mut identity = None;
            for stage in 1..=5 {
                let v = numbers(rows[1 + session * 5 + stage - 1])?;
                if v.len() != 10
                    || v[0] != cold[0]
                    || v[1] != i64::try_from(session)?
                    || v[2] != i64::try_from(stage)?
                    || v[3] != cold[3]
                    || v[4] <= 0
                    || v[5] <= 0
                    || v[6..] != [4, 1, 3, 1]
                    || identity
                        .replace((v[4], v[5]))
                        .is_some_and(|old| old != (v[4], v[5]))
                {
                    return Err(
                        "whole manager changed actual owners, type, features or connected state"
                            .into(),
                    );
                }
            }
        }
    }
    Ok(())
}
fn marker(bytes: &[u8]) -> Result<u64> {
    if bytes.len() != 2016 {
        return Err("incomplete whole manager BGR target".into());
    }
    let value = u64::from(bytes[2]) << 16 | u64::from(bytes[1]) << 8 | u64::from(bytes[0]);
    if value == 0 {
        return Err("missing whole manager producer serial".into());
    }
    Ok(value)
}
fn metadata(fd: i32, node: u32) -> String {
    format!(
        r#"{{"eis_socket_path":"","input_method":0,"pw_node_id":{node},"pw_socket_fd":{fd},"screencap_method":4,"type":"linux","uinput_path":"/dev/uinput","uinput_screen_height":0,"uinput_screen_width":0,"use_win32_vk_code":false,"wlr_socket_path":""}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_lifecycle_feature_fd_and_resource_corruption() {
        let good = (0..20)
            .map(|i| format!("{i}\t-1\t42\t2\t0\t3\t7\t7\t1\t1\t0\t6\n"))
            .collect::<String>();
        assert_eq!(cases(&good, 2).unwrap().len(), 20);
        for (from, to) in [
            ("\t0\t3\t", "\t0\t0\t"),
            ("\t7\t7\t", "\t7\t8\t"),
            ("\t-1\t", "\t4\t"),
            ("\t1\t1\t0\t6", "\t1\t0\t0\t6"),
        ] {
            assert!(cases(&good.replacen(from, to, 1), 2).is_err());
        }
        assert!(cases(&good, 1).is_err());
        assert!(cases(&good.lines().skip(1).collect::<Vec<_>>().join("\n"), 2).is_err());
    }
    #[test]
    fn rejects_owner_substitution_type_features_and_missing_events() {
        let good = (0..20)
            .map(|i| {
                let mut rows = format!("{i}\t-1\t0\t100\t0\t0\t0\t0\t0\t0\n");
                for stage in 1..=5 {
                    rows.push_str(&format!("{i}\t0\t{stage}\t100\t200\t300\t4\t1\t3\t1\n"));
                }
                rows
            })
            .collect::<String>();
        owners(&good, 1).unwrap();
        for (from, to) in [
            ("\t200\t300\t", "\t201\t300\t"),
            ("\t4\t1\t3\t1", "\t1\t1\t3\t1"),
            ("\t4\t1\t3\t1", "\t4\t0\t3\t1"),
            ("\t4\t1\t3\t1", "\t4\t1\t0\t1"),
            ("\t4\t1\t3\t1", "\t4\t1\t3\t0"),
        ] {
            assert!(owners(&good.replacen(from, to, 1), 1).is_err());
        }
        assert!(owners(&(good.clone() + good.lines().next().unwrap()), 1).is_err());
    }
}
