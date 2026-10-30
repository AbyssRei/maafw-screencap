// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent verifier of actual Wlr C-entry/typed-host/shared-client records.
use crate::{Result, pipewire_trace::task_identities, records};
use std::{collections::BTreeMap, fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace wlr-host-api <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut selected = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let start = artifacts.len();
        let mut read = |name: &str| -> Result<Vec<u8>> {
            let path = directory.join(name);
            artifacts.push(records::artifact(&root, &path)?);
            Ok(fs::read(path)?)
        };
        let socket = String::from_utf8(read("socket-path.txt")?)?;
        if !socket.starts_with('/')
            || socket.contains(['\0', '\n', '\r'])
            || socket.len() >= 108
            || selected.as_ref().is_some_and(|value| value != &socket)
        {
            return Err("Wlr source/Rust private socket identity differs".into());
        }
        selected = Some(socket.clone());
        let task_count = cases(
            std::str::from_utf8(&read(&format!("{side}-cases.tsv"))?)?,
            side == "source",
        )?;
        let baseline = task_identities(std::str::from_utf8(&read("baseline-task-ids.txt")?)?)?;
        if u64::try_from(baseline.len())? != task_count {
            return Err("Wlr host baseline tasks differ".into());
        }
        let quoted = serde_json::to_string(&socket)?;
        let metadata = format!(
            r#"{{"eis_socket_path":"","input_method":1,"pw_node_id":0,"pw_socket_fd":-1,"screencap_method":1,"type":"linux","uinput_path":"/dev/uinput","uinput_screen_height":0,"uinput_screen_width":0,"use_win32_vk_code":true,"wlr_socket_path":{quoted}}}"#
        );
        for cycle in 0..20 {
            if task_identities(std::str::from_utf8(&read(&format!(
                "closed-{cycle}-task-ids.txt"
            ))?)?)?
                != baseline
            {
                return Err("Wlr host closed task identities differ".into());
            }
            if read(&format!("api-{cycle}.uuid"))? != format!("{socket}_{socket}").as_bytes()
                || read(&format!("api-{cycle}.info"))? != metadata.as_bytes()
            {
                return Err("Wlr original/Rust UUID or full eleven-field metadata differs".into());
            }
            if side == "source" && read(&format!("snapshots-{cycle}.txt"))? != b"29\t1\n" {
                return Err("Wlr original input proxy snapshots survived final destruction".into());
            }
            for call in 0..3 {
                let stem = format!("host-{cycle}-call-{call}");
                let bgr = read(&format!("{stem}.bgr"))?;
                pixels(&bgr, 3, [2, 1, 0])?;
                if read(&format!("held-{}.bgr", cycle * 3 + call))? != bgr {
                    return Err("Wlr host BGR owner changed after all native owners ended".into());
                }
                if side == "rust" {
                    let raw = read(&format!("{stem}.raw"))?;
                    let meta = read(&format!("{stem}.meta.txt"))?;
                    let (channels, order) = layout(std::str::from_utf8(&meta)?)?;
                    pixels(&raw, channels, order)?;
                    if read(&format!("held-{}.raw", cycle * 3 + call))? != raw {
                        return Err(
                            "Wlr raw CPU owner changed after all native owners ended".into()
                        );
                    }
                }
            }
        }
        if side == "rust" {
            ledgers(std::str::from_utf8(&read("rust-ledgers.tsv")?)?)?;
        }
        let count = fs::read_dir(&directory)?
            .collect::<std::io::Result<Vec<_>>>()?
            .len();
        if count != artifacts.len() - start {
            return Err("Wlr host matrix has missing/extra files".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"source_actual_virtual_handles":20,"rust_legacy_constructors":20,
        "init_true_per_side":7,"init_false_per_side":7,"init_error_per_side":6,"native_captures":120,"payloads":360,
        "metadata_uuid_sets":40,"shared_last_client_witnesses":40,"source_proxy_expiries":20,"rust_final_callback_ledgers":20,"artifacts":artifacts,
        "scope":"Whole fixed original Wlr compat cpp and unchanged public manager/API/base/config declarations forward actual virtual capture to original Wlr/Wayland native sources. Rust uses the migrated compat constructor and same NativeFactory/Backend with typed host ownership. Controlled input init true/false/error retains Input=Wlr and the same client through input destruction; no input actions/devices are implemented. Exact task sets, fd counts, last connected client reference, final source weak proxy/client expiry, distinct Rust early/final callback counts, original metadata, independently known 64x48 pixels and post-native CPU retention are verified. Not original input initialization, live-client explicit close/reconnect, general host/fault/resource coverage or full P1 acceptance."});
    let path = root.join("wlr-host-api-validation.json");
    if check_only && path.exists() {
        if records::read::<serde_json::Value>(&path)? != report {
            return Err("Wlr host API report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "Wlr native API source_handles=20 Rust_constructors=20 metadata_uuid_sets=40 captures=120 payloads=360 shared_last_client=40 final_native_owners_released=40 actual_records_verified=1 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
pub fn run_construction(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace wlr-host-construction <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut selected = None;
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let start = artifacts.len();
        let mut read = |name: &str| -> Result<Vec<u8>> {
            let path = directory.join(name);
            artifacts.push(records::artifact(&root, &path)?);
            Ok(fs::read(path)?)
        };
        let socket = String::from_utf8(read("socket-path.txt")?)?;
        if !socket.starts_with('/')
            || socket.contains(['\0', '\n', '\r'])
            || socket.len() >= 108
            || selected.as_ref().is_some_and(|value| value != &socket)
        {
            return Err("Wlr constructor-error private socket differs".into());
        }
        selected = Some(socket.clone());
        let task_count = construction_cases(
            std::str::from_utf8(&read(&format!("{side}-cases.tsv"))?)?,
            side == "source",
        )?;
        let baseline = task_identities(std::str::from_utf8(&read("baseline-task-ids.txt")?)?)?;
        if u64::try_from(baseline.len())? != task_count {
            return Err("Wlr constructor-error baseline task identities differ".into());
        }
        let quoted = serde_json::to_string(&socket)?;
        let metadata = format!(
            r#"{{"eis_socket_path":"","input_method":1,"pw_node_id":0,"pw_socket_fd":-1,"screencap_method":1,"type":"linux","uinput_path":"/dev/uinput","uinput_screen_height":0,"uinput_screen_width":0,"use_win32_vk_code":true,"wlr_socket_path":{quoted}}}"#
        );
        for cycle in 0..20 {
            if task_identities(std::str::from_utf8(&read(&format!(
                "closed-{cycle}-task-ids.txt"
            ))?)?)?
                != baseline
            {
                return Err("Wlr constructor-error final task identities differ".into());
            }
            if read(&format!("api-{cycle}.uuid"))? != format!("{socket}_{socket}").as_bytes()
                || read(&format!("api-{cycle}.info"))? != metadata.as_bytes()
            {
                return Err("Wlr constructor-error UUID/full metadata differs".into());
            }
            for call in 0..3 {
                let stem = format!("host-{cycle}-call-{call}");
                let bgr = read(&format!("{stem}.bgr"))?;
                pixels(&bgr, 3, [2, 1, 0])?;
                if read(&format!("held-{}.bgr", cycle * 3 + call))? != bgr {
                    return Err("Wlr retained capture BGR changed after native teardown".into());
                }
                if side == "rust" {
                    let raw = read(&format!("{stem}.raw"))?;
                    let meta = read(&format!("{stem}.meta.txt"))?;
                    let (channels, order) = layout(std::str::from_utf8(&meta)?)?;
                    pixels(&raw, channels, order)?;
                    if read(&format!("held-{}.raw", cycle * 3 + call))? != raw {
                        return Err("Wlr retained raw owner changed after native teardown".into());
                    }
                }
            }
        }
        if side == "rust" {
            construction_ledgers(std::str::from_utf8(&read("rust-ledgers.tsv")?)?)?;
        }
        if fs::read_dir(&directory)?
            .collect::<std::io::Result<Vec<_>>>()?
            .len()
            != artifacts.len() - start
        {
            return Err("Wlr constructor-error matrix has missing/extra files".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"constructor_errors_per_side":20,"controlled_error_code":98,
        "native_captures":120,"payloads":360,"metadata_uuid_sets":40,"source_partial_snapshot_expiries":20,"rust_final_callback_ledgers":20,"artifacts":artifacts,
        "scope":"Original Wlr compatibility/manager virtual capture and the same migrated Rust manager/native factory remain callable after a controlled host input constructor failure. No input owner/init/action/device is created. Exact tasks, fd counts, manager/capture client ownership, source partial proxy expiry, Rust native callback teardown, full metadata, known64x48 pixels and CPU owners after shutdown are checked. This is not native input initialization, allocation-failure recovery, shared-client explicit close/reconnect, a complete native source/binary audit or full P1 acceptance."});
    let path = root.join("wlr-host-construction-validation.json");
    if check_only && path.exists() {
        if records::read::<serde_json::Value>(&path)? != report {
            return Err("Wlr constructor-error report differs from raw records".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "Wlr host_create_failure source_objects=20 Rust_objects=20 constructor_errors=40 captures=120 payloads=360 final_native_owners_released=40 actual_records_verified=1 input_actions=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn construction_cases(text: &str, source: bool) -> Result<u64> {
    let rows = numbers(text)?;
    if rows.len() != 20 {
        return Err("incomplete Wlr constructor-error cases".into());
    }
    let mut baseline = None;
    for (cycle, v) in rows.iter().enumerate() {
        if v.len() != 20
            || v[0] != u64::try_from(cycle)?
            || v[1..14] != [98, 1, 0, 0, 0, 0, 3, 2, 1, 3, 0, 0, 1]
            || v[14] < 3
            || v[14] != v[15]
            || v[16] == 0
            || v[17] != 1
            || v[18] != if source { 29 } else { 0 }
            || v[19] != u64::from(source)
            || baseline.is_some_and(|value| value != (v[14], v[16]))
        {
            return Err(
                "Wlr constructor error lost capture/client state or native resources".into(),
            );
        }
        baseline = Some((v[14], v[16]));
    }
    Ok(rows[0][16])
}
fn construction_ledgers(text: &str) -> Result<()> {
    let rows = numbers(text)?;
    if rows.len() != 20 {
        return Err("incomplete Wlr constructor-error callback ledgers".into());
    }
    for (cycle, v) in rows.iter().enumerate() {
        if v.len() != 10
            || v[0] != u64::try_from(cycle)?
            || v[1] == 0
            || v[1] != v[2]
            || v[1] != v[3]
            || v[4..] != [3, 3, 0, 0, 0, 0]
        {
            return Err("Wlr constructor-error callback owners or cleanup differ".into());
        }
    }
    Ok(())
}
fn numbers(text: &str) -> Result<Vec<Vec<u64>>> {
    text.lines()
        .map(|line| {
            line.split('\t')
                .map(|value| {
                    if value.is_empty()
                        || !value.bytes().all(|b| b.is_ascii_digit())
                        || value.len() > 1 && value.starts_with('0')
                    {
                        return Err("invalid unsigned decimal Wlr host witness".into());
                    }
                    Ok(value.parse()?)
                })
                .collect()
        })
        .collect()
}
fn cases(text: &str, source: bool) -> Result<u64> {
    let rows = numbers(text)?;
    if rows.len() != 20 {
        return Err("incomplete Wlr host cases".into());
    }
    let mut baseline = None;
    for (cycle, v) in rows.iter().enumerate() {
        let outcome = u64::try_from(cycle % 3)?;
        if v.len() != 24
            || v[0] != u64::try_from(cycle)?
            || v[1] != outcome
            || v[2] != [1, 0, 2][cycle % 3]
            || v[3] != if outcome == 2 { 97 } else { 0 }
            || v[4..14] != [1, 1, 1, 1, 3, 1, 1, 2, 0, 1]
            || v[14] < 3
            || v[14] != v[15]
            || v[16] == 0
            || v[17..] != [1, 3, u64::from(source), u64::from(source), 3, 1, 1]
            || baseline.is_some_and(|value| value != (v[14], v[16]))
        {
            return Err("Wlr shared input/client init/destruction/resources differ".into());
        }
        baseline = Some((v[14], v[16]));
    }
    Ok(rows[0][16])
}
fn ledgers(text: &str) -> Result<()> {
    let rows = numbers(text)?;
    if rows.len() != 20 {
        return Err("incomplete Wlr callback ledgers".into());
    }
    for (cycle, v) in rows.iter().enumerate() {
        if v.len() != 11
            || v[0] != u64::try_from(cycle)?
            || v[1] <= v[2]
            || v[1] != v[3]
            || v[3] != v[4]
            || v[3] != v[5]
            || v[6..10] != [3, 3, 0, 0]
            || v[1] <= v[10]
            || v[2] > v[10]
        {
            return Err("Wlr early/final callback ownership or cleanup differs".into());
        }
    }
    Ok(())
}
fn layout(text: &str) -> Result<(usize, [usize; 3])> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("invalid Wlr raw metadata")?;
        if fields.insert(key, value).is_some() {
            return Err("duplicate Wlr raw metadata".into());
        }
    }
    if fields.keys().copied().collect::<Vec<_>>()
        != [
            "conversion",
            "flags",
            "height",
            "native_stride",
            "ready_time",
            "width",
        ]
        || fields["width"] != "64"
        || fields["height"] != "48"
    {
        return Err("Wlr raw metadata fields/geometry differ".into());
    }
    let (channels, order) = match fields["conversion"] {
        "Bgra" => (4, [2, 1, 0]),
        "Rgba" => (4, [0, 1, 2]),
        "Rgb" => (3, [0, 1, 2]),
        "BgrCopy" => (3, [2, 1, 0]),
        _ => return Err("unrecognized native Wlr conversion".into()),
    };
    if fields["native_stride"] != (64 * channels).to_string() {
        return Err("Wlr native packed stride differs".into());
    }
    let time = fields["ready_time"]
        .strip_prefix("Some((")
        .and_then(|v| v.strip_suffix("))"))
        .ok_or("missing native Ready time")?;
    let time = time
        .split(", ")
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if time.len() != 3 || time[2] >= 1_000_000_000 {
        return Err("invalid native Ready time".into());
    }
    fields["flags"]
        .strip_prefix("Some(")
        .and_then(|v| v.strip_suffix(')'))
        .ok_or("missing native flags")?
        .parse::<u32>()?;
    Ok((channels, order))
}
fn pixels(bytes: &[u8], channels: usize, order: [usize; 3]) -> Result<()> {
    if bytes.len() != 64 * 48 * channels {
        return Err("Wlr host native pixel span differs".into());
    }
    for y in 0..48 {
        for x in 0..64 {
            let expected = if y >= 24 {
                [0, 0, 255]
            } else if x < 32 {
                [255, 0, 0]
            } else {
                [0, 255, 0]
            };
            for c in 0..3 {
                if bytes[(y * 64 + x) * channels + order[c]] != expected[c] {
                    return Err("Wlr host known pattern differs".into());
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_constructor_success_lost_capture_live_client_and_incomplete_cleanup() {
        let source = (0..20)
            .map(|c| format!("{c}\t98\t1\t0\t0\t0\t0\t3\t2\t1\t3\t0\t0\t1\t7\t7\t2\t1\t29\t1\n"))
            .collect::<String>();
        assert_eq!(construction_cases(&source, true).unwrap(), 2);
        assert!(construction_cases(&source, false).is_err());
        for (old, new) in [
            ("\t98\t1\t0", "\t98\t1\t1"),
            ("\t3\t2\t1", "\t3\t1\t1"),
            ("\t1\t3\t0\t0\t1", "\t1\t0\t0\t0\t1"),
            ("\t29\t1", "\t29\t0"),
            ("\t7\t7", "\t7\t8"),
        ] {
            assert!(construction_cases(&source.replacen(old, new, 1), true).is_err());
        }
        assert!(
            construction_cases(
                &source.lines().take(19).collect::<Vec<_>>().join("\n"),
                true
            )
            .is_err()
        );
        let ledgers = (0..20)
            .map(|c| format!("{c}\t11\t11\t11\t3\t3\t0\t0\t0\t0\n"))
            .collect::<String>();
        construction_ledgers(&ledgers).unwrap();
        assert!(construction_ledgers(&ledgers.replacen("\t11\t11\t11", "\t11\t11\t5", 1)).is_err());
        assert!(construction_ledgers(&ledgers.replacen("\t3\t3\t0", "\t2\t2\t0", 1)).is_err());
        assert!(
            construction_ledgers(&ledgers.replacen("\t0\t0\t0\t0", "\t1\t0\t0\t0", 1)).is_err()
        );
    }
    #[test]
    fn rejects_wrong_destruction_order_early_ledgers_and_pseudo_success() {
        let valid = (0..20)
            .map(|c| {
                format!(
                    "{c}\t{}\t{}\t{}\t1\t1\t1\t1\t3\t1\t1\t2\t0\t1\t7\t7\t2\t1\t3\t1\t1\t3\t1\t1\n",
                    c % 3,
                    [1, 0, 2][c % 3],
                    if c % 3 == 2 { 97 } else { 0 }
                )
            })
            .collect::<String>();
        assert_eq!(cases(&valid, true).unwrap(), 2);
        assert!(cases(&valid, false).is_err());
        assert!(cases(&valid.replacen("\t3\t1\t1\t2", "\t3\t2\t0\t2", 1), true).is_err());
        assert!(cases(&valid.replacen("\t7\t7", "\t7\t8", 1), true).is_err());
        assert!(cases(&valid.lines().take(19).collect::<Vec<_>>().join("\n"), true).is_err());
        let valid = (0..20)
            .map(|c| format!("{c}\t15\t8\t15\t15\t15\t3\t3\t0\t0\t8\n"))
            .collect::<String>();
        ledgers(&valid).unwrap();
        assert!(ledgers(&valid.replacen("\t15\t15\t15\t3", "\t15\t15\t8\t3", 1)).is_err());
        assert!(ledgers(&valid.replacen("\t15\t8\t15", "\t15\t15\t15", 1)).is_err());
        let meta = "conversion=Rgba\nwidth=64\nheight=48\nnative_stride=256\nready_time=Some((0, 1, 2))\nflags=Some(0)\n";
        assert_eq!(layout(meta).unwrap(), (4, [0, 1, 2]));
        assert!(layout(&meta.replace("256", "192")).is_err());
        assert!(layout(&(meta.to_owned() + "width=64\n")).is_err());
        assert!(pixels(&vec![0; 64 * 48 * 3], 3, [2, 1, 0]).is_err());
    }
}
