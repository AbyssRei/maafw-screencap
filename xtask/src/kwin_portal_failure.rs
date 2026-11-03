// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Pre-manager controlled Portal failures and real unconsumed socket identities.
use crate::{Result, pipewire_trace::task_identities, records};
use std::{collections::BTreeSet, fs, path::Path};

const HEADER: &str = "index\tstage\terror\thelper_live\toriginal_closed\thelper_ended\tfd\tcookie_before\tcookie_after\tfd_calls\tnode_calls\tdestroy_calls\tcreated\tfds_before\tfds_residual\tfds_final\ttasks\tcaller\tfixture_closed\tfd_closed\thelper_final\tnative_constructed\tevents";

fn cases(text: &str) -> Result<(u64, u64)> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("Portal failure header differs".into());
    }
    let rows = lines.collect::<Vec<_>>();
    if rows.len() != 100 {
        return Err("Portal failure matrix is partial or has extra rows".into());
    }
    let mut cookies = BTreeSet::new();
    let mut baseline = None;
    for (index, row) in rows.iter().enumerate() {
        let fields = row.split('\t').collect::<Vec<_>>();
        if fields.len() != 23 {
            return Err("Portal failure column count differs".into());
        }
        let v = fields[..22]
            .iter()
            .map(|field| {
                let value = field.parse::<u64>()?;
                if value.to_string() != *field {
                    return Err("Noncanonical Portal failure scalar".into());
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>>>()?;
        let stage = (index % 5) as u64;
        let events = match stage {
            0 | 1 => "create,open",
            2 => "create,open,fd",
            3 => "create,open,fd,node",
            _ => "create,open,fd,node,destroy",
        };
        let identity = (v[13], v[16], v[17]);
        if v[..6]
            != [
                index as u64,
                stage,
                if stage == 0 { 0 } else { 90 + stage },
                1,
                1,
                0,
            ]
            || i32::try_from(v[6]).is_err()
            || v[7] == 0
            || v[7] != v[8]
            || !cookies.insert(v[7])
            || v[9..13]
                != [
                    u64::from(stage >= 2),
                    u64::from(stage >= 3),
                    u64::from(stage == 4),
                    0,
                ]
            || v[13] < 3
            || v[13].checked_add(1) != Some(v[14])
            || v[13] != v[15]
            || v[16] == 0
            || v[17] == 0
            || u32::try_from(v[17]).is_err()
            || v[18..22] != [1, 1, 0, 0]
            || fields[22] != events
            || *baseline.get_or_insert(identity) != identity
        {
            return Err(
                "Portal outcome, host call order, residual identity or fixture cleanup differs"
                    .into(),
            );
        }
    }
    let (_, tasks, caller) = baseline.ok_or("Portal failure baseline missing")?;
    Ok((tasks, caller))
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace kwin-portal-failure <existing external matrix> [--check-only]"
                .into(),
        ),
    };
    if !Path::new(path).is_dir() {
        return Err("Portal failure observations must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut expected = BTreeSet::new();
    for side in ["source", "rust"] {
        let mut read = |name: &str| -> Result<String> {
            let name = format!("{side}/{name}");
            let path = root.join(&name);
            expected.insert(name);
            inputs.push(records::artifact(&root, &path)?);
            Ok(fs::read_to_string(path)?)
        };
        let (count, caller) = cases(&read("cases.tsv")?)?;
        let baseline = task_identities(&read("baseline-task-ids.txt")?)?;
        if u64::try_from(baseline.len())? != count || !baseline.contains(&caller) {
            return Err("Portal caller/baseline task identities differ".into());
        }
        for index in 0..100 {
            if task_identities(&read(&format!("closed-{index}-task-ids.txt"))?)? != baseline {
                return Err("Portal failure fixture changed actual task identities".into());
            }
        }
    }
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 Portal evidence entry")?;
        if ["source", "rust"].contains(&name.as_str()) && entry.file_type()?.is_dir() {
            for child in fs::read_dir(entry.path())? {
                let child = child?;
                if !child.file_type()?.is_file() {
                    return Err("Unexpected Portal evidence directory/link".into());
                }
                actual.insert(format!(
                    "{name}/{}",
                    child
                        .file_name()
                        .into_string()
                        .map_err(|_| "Non-UTF8 Portal evidence file")?
                ));
            }
        } else if name != "portal-failure-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected Portal failure evidence entry".into());
        }
    }
    if actual != expected || inputs.len() != 204 {
        return Err("Portal failure file inventory differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cases":200,"stages":5,"open_false":40,"controlled_host_errors":160,
        "residual_sockets":200,"fixture_reclaims":200,"manager_created":0,"native_captures":0,"full_P1_accepted":false,"inputs":inputs,
        "scope":"Whole unchanged F KWin Create and migrated private Config constructor. Controlled host Open false or errors 91/92/93/94 at Open/FD/node/destroy retain the actual private-daemon socket and helper before manager construction; same SO_COOKIE and one residual fd are observed before explicit fixture-only reclaim. No actual DBus/Portal authorization, input action, PipeWire stream/init, exception safety across a C/Rust ABI, arbitrary failures or complete P1 acceptance. Source residuals are not product cleanup success."});
    let destination = root.join("portal-failure-validation.json");
    if check_only && destination.exists() {
        if records::read::<serde_json::Value>(&destination)? != report {
            return Err("Saved Portal failure report differs".into());
        }
    } else if !check_only {
        records::create_json(&destination, &report)?;
    }
    println!(
        "KWin Portal failure records cases=200 stages=5 residual_sockets=200 fixture_reclaims=200 files=204 native_capture=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> String {
        let mut text = format!("{HEADER}\n");
        for index in 0..100 {
            let stage = index % 5;
            let events = match stage {
                0 | 1 => "create,open",
                2 => "create,open,fd",
                3 => "create,open,fd,node",
                _ => "create,open,fd,node,destroy",
            };
            let cookie = 1000 + index;
            text.push_str(&format!("{index}\t{stage}\t{}\t1\t1\t0\t7\t{cookie}\t{cookie}\t{}\t{}\t{}\t0\t8\t9\t8\t1\t17\t1\t1\t0\t0\t{events}\n",if stage==0 {0}else{90+stage},u8::from(stage>=2),u8::from(stage>=3),u8::from(stage==4)));
        }
        text
    }
    #[test]
    fn rejects_wrong_error_order_socket_residual_reclaim_and_owner_state() {
        let text = fixture();
        assert_eq!(cases(&text).unwrap(), (1, 17));
        for altered in [
            text.replacen("1\t1\t91", "1\t1\t0", 1),
            text.replacen("1000\t1000", "1000\t1001", 1),
            text.replacen("1001\t1001", "1000\t1000", 1),
            text.replacen("\t8\t9\t8", "\t8\t8\t8", 1),
            text.replacen("\t1\t17\t1\t1\t0\t0", "\t1\t17\t0\t1\t0\t0", 1),
            text.replacen("create,open,fd,node", "create,open,node,fd", 1),
            text.replacen("0\t0\t0\t1\t1\t0", "0\t0\t0\t1\t1\t1", 1),
            text.replacen("0\t0\t0\t1", "00\t0\t0\t1", 1),
            text.lines().take(100).collect::<Vec<_>>().join("\n"),
            format!("{text}{}\n", text.lines().last().unwrap()),
        ] {
            assert_ne!(altered, text);
            assert!(cases(&altered).is_err());
        }
    }
}
