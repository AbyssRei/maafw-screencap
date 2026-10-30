// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Additional actual original KWin C entry/virtual-handle/config witnesses.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};
pub fn run(args: &[String]) -> Result<()> {
    // Same native host pixel/lifetime validator; no duplicate capture semantics.
    crate::pipewire_host::run(args)?;
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace pipewire-host-api <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = vec![records::artifact(
        &root,
        &root.join("host-validation.json"),
    )?];
    let mut node = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let path = directory.join(format!("{side}-api.tsv"));
        let cases = rows(&fs::read_to_string(&path)?, side == "source")?;
        artifacts.push(records::artifact(&root, &path)?);
        for (cycle, (fd, selected_node)) in cases.into_iter().enumerate() {
            if node.is_some_and(|value| value != selected_node) {
                return Err("KWin/Rust fixture node changed".into());
            }
            node = Some(selected_node);
            for (extension, expected) in [
                ("events", b"create,open,fd,node,destroy".to_vec()),
                (
                    "uuid",
                    format!("{fd}_{selected_node}_/capture-only-host").into_bytes(),
                ),
                ("info", metadata(fd, selected_node).into_bytes()),
            ] {
                let path = directory.join(format!("api-{cycle}.{extension}"));
                if fs::read(&path)? != expected {
                    return Err(
                        "actual KWin/Rust host events, UUID or complete metadata differ".into(),
                    );
                }
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
        let count = fs::read_dir(&directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
            .iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|e| e == "events" || e == "uuid" || e == "info")
            })
            .count();
        if count != 60 {
            return Err("missing/extra KWin metadata witnesses".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"source_original_compat_entries":20,"source_actual_virtual_handles":20,
        "rust_migrated_compat_constructors":20,"metadata_uuid_event_sets":40,"native_captures":120,"payloads":360,"artifacts":artifacts,
        "scope":"Whole fixed KWin compat cpp creates/destroys the actual original public LinuxControlUnitAPI handle with complete unchanged original manager/config/base declarations. Original virtual connect/features/UUID/get_info/inactive/capture and destructor forward to original PW capture, paired with the existing migrated Rust compat constructor/typed host/native backend. Controlled Portal C calls own real private sockets and prove duplicate identity across original helper close; host init true/false/error stays scripted. Existing host validator verifies all 360 actual pixels/held owners/native-before-host closure and exact task sets. No device input, actual DBus/desktop authorization, generic host failures, complete framework/ABI or P1 acceptance."});
    let path = root.join("host-api-validation.json");
    if check_only && path.exists() {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("KWin API report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "KWin native API source_handles=20 Rust_constructors=20 metadata_uuid_events=40 captures=120 payloads=360 actual_records_verified=1 input_actions=0 authorization=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn metadata(fd: i32, node: u32) -> String {
    // Independent complete byte-sorted original get_info contract, not the
    // migrated serializer or a parser that silently accepts duplicate keys.
    format!(
        r#"{{"eis_socket_path":"","input_method":2,"pw_node_id":{node},"pw_socket_fd":{fd},"screencap_method":4,"type":"linux","uinput_path":"/capture-only-host","uinput_screen_height":19,"uinput_screen_width":17,"use_win32_vk_code":true,"wlr_socket_path":""}}"#
    )
}
fn rows(text: &str, source: bool) -> Result<Vec<(i32, u32)>> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 20 {
        return Err("incomplete KWin API cases".into());
    }
    let mut result = Vec::new();
    let mut cookies = BTreeSet::new();
    for (cycle, line) in lines.iter().enumerate() {
        let v = line
            .split('\t')
            .map(|text| {
                if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid decimal KWin API witness".into());
                }
                Ok(text.parse::<u64>()?)
            })
            .collect::<Result<Vec<_>>>()?;
        if v.len() != 9
            || v[0] != u64::try_from(cycle)?
            || v[2] == 0
            || v[3..6] != [u64::from(source), u64::from(source), 1]
            || v[6] == 0
            || v[6] != v[7]
            || !cookies.insert(v[6])
            || v[8] != 5
        {
            return Err("KWin handle/host-call/duplicate-socket witness differs".into());
        }
        result.push((i32::try_from(v[1])?, u32::try_from(v[2])?));
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_virtual_handles_replaced_sockets_and_narrowing() {
        let valid = (0..20)
            .map(|cycle| {
                format!(
                    "{cycle}\t7\t41\t1\t1\t1\t{}\t{}\t5\n",
                    500 + cycle,
                    500 + cycle
                )
            })
            .collect::<String>();
        assert_eq!(rows(&valid, true).unwrap().len(), 20);
        assert!(rows(&valid, false).is_err());
        for bad in [
            valid.replacen("\t41\t1\t1\t1", "\t41\t0\t1\t1", 1),
            valid.replacen("\t500\t500", "\t500\t501", 1),
            valid.replacen("\t7\t41", "\t2147483648\t41", 1),
            valid.replacen("\t7\t41", "\t7\t4294967296", 1),
            valid.lines().take(19).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(rows(&bad, true).is_err());
        }
    }
}
