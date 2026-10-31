// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual close-prefix observations; the fixture explicitly delays disconnect.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const HEADER: &str = "cycle\tmode\tcalls\tdeferred\tmanager_refs\tshm_refs\toutput_refs\tglobals\tconnected\tfd_live\tclosed_capture_preserved\trebound\tsnapshots_expired\tforwarded\tfd_closed\tfds_before\tfds_after\ttasks_equal\theld";
const MODES: [&str; 4] = ["control", "cold", "buffered", "reconnect"];

fn values(mode: usize, fds: u64) -> [u64; 17] {
    let active = u64::from(mode != 0);
    [
        1,
        active,
        active,
        active,
        active,
        0,
        0,
        1,
        u64::from(mode == 1 || mode == 2),
        u64::from(mode == 3),
        1,
        1,
        1,
        fds,
        fds,
        1,
        [0, 0, 1, 2][mode],
    ]
}

fn observations(text: &str) -> Result<u64> {
    let mut rows = text.lines();
    if rows.next() != Some(HEADER) {
        return Err("Wrong early-close observation schema".into());
    }
    let mut baseline = None;
    for cycle in 0..20 {
        for (mode, name) in MODES.iter().enumerate() {
            let row: Vec<_> = rows
                .next()
                .ok_or("Incomplete early-close matrix")?
                .split('\t')
                .collect();
            if row.len() != 19 || row[0] != cycle.to_string() || row[1] != *name {
                return Err("Wrong early-close case order or fields".into());
            }
            let actual = row[2..]
                .iter()
                .map(|v| v.parse::<u64>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let fds = *baseline.get_or_insert(actual[13]);
            if fds < 4 || actual != values(mode, fds) {
                return Err(
                    "Native close prefix, deferred owner or resource evidence differs".into(),
                );
            }
        }
    }
    if rows.next().is_some() {
        return Err("Extra early-close observations".into());
    }
    baseline.ok_or_else(|| "No native early-close resource baseline".into())
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace wlr-early-close-prefix <external directory> [--check-only]"
                .into(),
        ),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut expected_names = BTreeSet::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        expected_names.insert(name.to_string());
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let rows = read("early-close.tsv")?;
    let fds = observations(std::str::from_utf8(&rows)?)?;
    let tasks = read("early-close-tasks.txt")?;
    let names: Vec<_> = std::str::from_utf8(&tasks)?.lines().collect();
    if names.is_empty()
        || names.windows(2).any(|pair| pair[0] >= pair[1])
        || names.iter().any(|name| {
            name.parse::<u32>()
                .map_or(true, |id| id == 0 || id.to_string() != *name)
        })
    {
        return Err("Missing, duplicate or invalid native task identities".into());
    }
    let pixels: Vec<u8> = (0..48)
        .flat_map(|y| {
            (0..64).flat_map(move |x| {
                if y >= 24 {
                    [255, 0, 0]
                } else if x < 32 {
                    [0, 0, 255]
                } else {
                    [0, 255, 0]
                }
            })
        })
        .collect();
    for cycle in 0..20 {
        for (mode, name) in MODES.iter().enumerate() {
            if read(&format!("{cycle}-{name}.tasks"))? != tasks {
                return Err("Early-close final native task identities changed".into());
            }
            for phase in match mode {
                2 => &["before"][..],
                3 => &["before", "fresh"][..],
                _ => &[][..],
            } {
                for suffix in ["bgr", "held"] {
                    if read(&format!("{cycle}-{name}-{phase}.{suffix}"))? != pixels {
                        return Err("Early-close native/retained pixels differ".into());
                    }
                }
            }
        }
    }
    let actual_names = fs::read_dir(&root)?
        .map(|entry| -> Result<_> {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err("Unexpected early-close directory or link".into());
            }
            Ok(entry
                .file_name()
                .into_string()
                .map_err(|_| "Non-UTF8 early-close filename")?)
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let actual_names = actual_names
        .into_iter()
        .filter(|name| name != "early-close-validation.json")
        .collect::<BTreeSet<_>>();
    if actual_names != expected_names || artifacts.len() != 202 {
        return Err("Early-close native evidence has missing or extra files".into());
    }
    let report = serde_json::json!({"layout_version":1,"cycles":20,"cases":80,
        "disconnect_prefixes":80,"fixture_deferred_disconnects":60,"closed_capture_preserved":40,
        "reconnects":20,"captures":60,"held_images":60,"baseline_fds":fds,"artifacts":artifacts,
        "scope":"Unchanged original close/reconnect prefix through its actual disconnect request. The fixture defers sixty native disconnects until old capture snapshots end; forty closed-client captures preserve output and twenty fresh post-reconnect sources capture known pixels. Exact proxy reference counts, cleared client globals, native descriptor closure, task identities and retained CPU pixels are checked. This controlled intervention does not validate disconnect with live proxies, capture through stale proxies after reconnect, a Rust early-close migration, or full P1 acceptance."});
    let report_path = root.join("early-close-validation.json");
    if check_only {
        let old: serde_json::Value = serde_json::from_slice(&fs::read(report_path)?)?;
        if old != report {
            return Err("Early-close records differ from saved validation".into());
        }
    } else {
        records::create_json(&report_path, &report)?;
    }
    println!(
        "Validated 80 original close prefixes, 60 fixture-delayed disconnects and 120 native/held pixel files; no Rust early-close or full P1 acceptance"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn matrix() -> String {
        let mut text = format!("{HEADER}\n");
        for cycle in 0..20 {
            for (mode, name) in MODES.iter().enumerate() {
                text.push_str(&format!("{cycle}\t{name}"));
                for value in values(mode, 7) {
                    text.push_str(&format!("\t{value}"));
                }
                text.push('\n');
            }
        }
        text
    }
    #[test]
    fn rejects_missing_prefixes_owner_mismatches_and_unbalanced_resources() {
        let matrix = matrix();
        assert_eq!(observations(&matrix).unwrap(), 7);
        for invalid in [
            String::new(),
            format!("{HEADER}\n"),
            matrix.lines().take(80).collect::<Vec<_>>().join("\n"),
            matrix.replacen("0\tcold\t1\t1\t1", "0\tcold\t1\t1\t0", 1),
            matrix.replacen("\t7\t7\t1\t0", "\t7\t8\t1\t0", 1),
            matrix.replacen("0\treconnect\t1\t1", "0\treconnect\t1\t0", 1),
            format!("{matrix}0\tcontrol\n"),
        ] {
            assert!(observations(&invalid).is_err());
        }
    }
}
