// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Readonly complete-manager/NoneInput Wlr capture records, not native execution.
use crate::{
    Result,
    pipewire_trace::task_identities,
    records,
    wlr_host_api::{layout, pixels},
};
use std::{fs, path::Path};
const FAILED: &[u8]=b"first_connected=0 first_features=0 retained_connected=1 retained_features=3 same_client=1 failed_output_preserved=1 resources_balanced=1\n";
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace wlr-whole-manager <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut socket = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let mut read = |name: &str| -> Result<Vec<u8>> {
            let path = directory.join(name);
            let bytes = fs::read(&path)?;
            artifacts.push(records::artifact(&root, &path)?);
            Ok(bytes)
        };
        let actual = String::from_utf8(read("socket-path.txt")?)?;
        if !actual.starts_with('/')
            || actual.len() >= 108
            || actual.contains(['\0', '\r', '\n'])
            || socket
                .replace(actual.clone())
                .is_some_and(|old| old != actual)
        {
            return Err("whole Wlr consumer socket identity differs".into());
        }
        let count = cases(std::str::from_utf8(&read(&format!("{side}-cases.tsv"))?)?)?;
        let baseline = task_identities(std::str::from_utf8(&read("baseline-task-ids.txt")?)?)?;
        if u64::try_from(baseline.len())? != count {
            return Err("whole Wlr baseline count differs".into());
        }
        if read("failed-client-control.txt")? != FAILED {
            return Err("whole Wlr retained failed-client behavior differs".into());
        }
        if side == "source" {
            if read("whole-origin.txt")?!=b"whole_original_manager=1 full_original_api=1 real_Wlr=1 real_NoneInput=1 input_actions=0\n" {
                return Err("whole Wlr original class/API scope differs".into());
            }
            owners(std::str::from_utf8(&read("source-owners.tsv")?)?)?;
        }
        let quoted = serde_json::to_string(&actual)?;
        let metadata = format!(
            r#"{{"eis_socket_path":"","input_method":0,"pw_node_id":0,"pw_socket_fd":-1,"screencap_method":1,"type":"linux","uinput_path":"/dev/uinput","uinput_screen_height":0,"uinput_screen_width":0,"use_win32_vk_code":false,"wlr_socket_path":{quoted}}}"#
        );
        for cycle in 0..20 {
            if task_identities(std::str::from_utf8(&read(&format!(
                "closed-{cycle}-task-ids.txt"
            ))?)?)?
                != baseline
            {
                return Err("whole Wlr task identities changed after native destruction".into());
            }
            if read(&format!("api-{cycle}.uuid"))? != format!("{actual}_-1").as_bytes()
                || read(&format!("api-{cycle}.info"))? != metadata.as_bytes()
            {
                return Err("whole Wlr UUID/full metadata bytes differ".into());
            }
            for call in 0..3 {
                let stem = format!("manager-{cycle}-{call}");
                let current = read(&format!("{stem}.bgr"))?;
                pixels(&current, 3, [2, 1, 0])?;
                if read(&format!("held-{}.bgr", cycle * 3 + call))? != current {
                    return Err("whole Wlr held BGR changed after destruction".into());
                }
                if side == "rust" {
                    if current != fs::read(root.join("source").join(format!("{stem}.bgr")))? {
                        return Err("whole Wlr actual original/Rust BGR bytes differ".into());
                    }
                    let raw = read(&format!("{stem}.raw"))?;
                    let (channels, order) =
                        layout(std::str::from_utf8(&read(&format!("{stem}.meta.txt"))?)?)?;
                    pixels(&raw, channels, order)?;
                    if read(&format!("held-{}.raw", cycle * 3 + call))? != raw {
                        return Err("whole Wlr held raw CPU owner changed".into());
                    }
                }
            }
        }
        let count = fs::read_dir(&directory)?
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|e| e == "bgr" || e == "raw")
            })
            .count();
        if count != if side == "source" { 120 } else { 240 } {
            return Err("whole Wlr missing or extra payloads".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"source_objects":21,"rust_objects":21,"capture_objects":40,"failed_client_objects":2,"connect_calls":84,"captures":120,
        "payloads":360,"source_owner_observations":140,"active_features":3,"artifacts":artifacts,
        "scope":"Complete unchanged original Linux manager/C API/NoneInput and Wlr/client/buffer/protocol sources, same Rust NativeManager capture implementation. Owned compositor, full target and held CPU images, exact metadata, source typed owners and forwarded same-client roundtrip observations, failed-client control, fd/task closure. External UUID provider retained. No input actions, arbitrary desktop/failure/UUID implementation or full P1 acceptance."});
    let path = root.join("whole-manager-validation.json");
    if check_only && path.exists() {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("whole Wlr report differs from saved records".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "Wlr whole manager objects=42 captures=120 payloads=360 source_owner_observations=140 active_features=3 exact_pixels_tasks_metadata_and_reconstruction=1 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn numbers(line: &str) -> Result<Vec<u64>> {
    line.split('\t')
        .map(|s| {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return Err("invalid whole Wlr decimal record".into());
            }
            Ok(s.parse()?)
        })
        .collect()
}
fn cases(text: &str) -> Result<u64> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 20 {
        return Err("whole Wlr cases incomplete".into());
    }
    let mut baseline = None;
    for (cycle, line) in lines.iter().enumerate() {
        let v = numbers(line)?;
        if v.len() != 12
            || v[0] != u64::try_from(cycle)?
            || v[1..7] != [0, 3, 3, 3, 2, 3]
            || v[7] < 3
            || v[7] != v[8]
            || v[9] == 0
            || v[10..] != [1, 1]
            || baseline
                .replace((v[7], v[9]))
                .is_some_and(|old| old != (v[7], v[9]))
        {
            return Err("whole Wlr lifecycle/features/resources differ".into());
        }
    }
    Ok(baseline.ok_or("missing whole Wlr cases")?.1)
}
fn owners(text: &str) -> Result<()> {
    let lines = text.lines().map(numbers).collect::<Result<Vec<_>>>()?;
    if lines.len() != 140 || lines.iter().any(|v| v.len() != 12) {
        return Err("whole Wlr owner events incomplete".into());
    }
    for (cycle, rows) in lines.as_chunks::<7>().0.iter().enumerate() {
        let cold = &rows[0];
        if cold[0] != u64::try_from(cycle)?
            || cold[1] != 0
            || cold[2] == 0
            || cold[3..10] != [0; 7]
            || cold[11] != 0
        {
            return Err("whole Wlr cold native owner state differs".into());
        }
        for stage in 1..7 {
            let v = &rows[stage];
            let previous = &rows[stage - 1];
            if v[0] != cold[0]
                || v[1] != u64::try_from(stage)?
                || v[2] != cold[2]
                || v[3..6].contains(&0)
                || v[6..10] != [1, 1, 3, 1]
                || v[11] != 1
                || (stage > 1 && v[5] != rows[1][5])
                || (matches!(stage, 2 | 3 | 4 | 6) && v[3..6] != previous[3..6])
                || (matches!(stage, 3 | 5) && v[10] != previous[10])
                || (matches!(stage, 1 | 2 | 4 | 6) && v[10] <= previous[10])
            {
                return Err(
                    "whole Wlr actual types/client/owners/roundtrip behavior differs".into(),
                );
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_owner_substitution_extra_roundtrip_and_wrong_features() {
        let good = (0..20)
            .map(|cycle| {
                let mut rows = format!("{cycle}\t0\t100\t0\t0\t0\t0\t0\t0\t0\t0\t0\n");
                for (stage, rt) in [(1, 3), (2, 5), (3, 5), (4, 7), (5, 7), (6, 9)] {
                    rows.push_str(&format!(
                        "{cycle}\t{stage}\t100\t200\t300\t400\t1\t1\t3\t1\t{rt}\t1\n"
                    ));
                }
                rows
            })
            .collect::<String>();
        owners(&good).unwrap();
        for (from, to) in [
            ("\t3\t100\t200", "\t3\t100\t201"),
            (
                "\t5\t100\t200\t300\t400\t1\t1\t3\t1\t7",
                "\t5\t100\t200\t300\t400\t1\t1\t3\t1\t8",
            ),
            ("\t1\t1\t3\t1", "\t1\t1\t0\t1"),
            ("\t300\t400\t", "\t300\t401\t"),
        ] {
            assert!(owners(&good.replacen(from, to, 1)).is_err());
        }
        assert!(owners(&good.lines().skip(1).collect::<Vec<_>>().join("\n")).is_err());
    }
    #[test]
    fn rejects_missing_capture_and_resource_failure() {
        let good = (0..20)
            .map(|i| format!("{i}\t0\t3\t3\t3\t2\t3\t7\t7\t1\t1\t1\n"))
            .collect::<String>();
        assert_eq!(cases(&good).unwrap(), 1);
        for (from, to) in [
            ("\t2\t3\t7", "\t2\t2\t7"),
            ("\t7\t7\t", "\t7\t8\t"),
            ("\t1\t1\t1", "\t1\t1\t0"),
        ] {
            assert!(cases(&good.replacen(from, to, 1)).is_err());
        }
    }
}
