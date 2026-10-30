// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual native pre-init host-error/socket-identity records, without capture.
use crate::{Result, pipewire_trace::task_identities, records};
use std::{collections::BTreeSet, fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace pipewire-host-offer <external matrix> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let path = directory.join(format!("{side}-cases.tsv"));
        let task_count = cases(&fs::read_to_string(&path)?)?;
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("baseline-task-ids.txt");
        let baseline = task_identities(&fs::read_to_string(&path)?)?;
        if u64::try_from(baseline.len())? != task_count {
            return Err("host offer baseline task identities/count differ".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
        for cycle in 0..20 {
            let path = directory.join(format!("closed-{cycle}-task-ids.txt"));
            if task_identities(&fs::read_to_string(&path)?)? != baseline {
                return Err("host offer teardown changed task identities".into());
            }
            artifacts.push(records::artifact(&root, &path)?);
        }
    }
    let report = serde_json::json!({"layout_version":1,"sides":2,"cycles_per_side":20,"host_create_errors":40,
        "unconsumed_after_native_destroy":40,"fixture_reclaims":40,"capture_count":0,"artifacts":artifacts,
        "scope":"Real source/Rust manager and PipeWire constructor/destructor with controlled host input create error 98 BEFORE native initialization. A real socketpair offer is duplicated before the helper descriptor closes, retains the same SO_COOKIE through the host error and complete manager/native destruction, then is explicitly reclaimed by the fixture. No native loop/core creation, successful capture, Portal/DBus authorization, general failed-connect consumption guarantee or complete P1 acceptance. Product retention of the unconsumed fd is a source residual, not cleanup success."});
    let destination = root.join("offer-validation.json");
    if check_only && destination.exists() {
        let saved: serde_json::Value = records::read(&destination)?;
        if saved != report {
            return Err("host offer report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&destination, &report)?;
    }
    println!(
        "PipeWire host offer sides=2 cycles=40 create_errors=40 exact_socket_and_task_identity=1 unconsumed_after_destroy=40 fixture_reclaims=40 capture=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn cases(text: &str) -> Result<u64> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 20 {
        return Err("incomplete native host offer matrix".into());
    }
    let mut cookies = BTreeSet::new();
    let (mut fds, mut tasks) = (None, None);
    for (cycle, line) in lines.iter().enumerate() {
        let values = line
            .split('\t')
            .map(|value| {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid decimal host offer record".into());
                }
                Ok(value.parse::<u64>()?)
            })
            .collect::<Result<Vec<_>>>()?;
        if values.len() != 19
            || values[..7] != [u64::try_from(cycle)?, 98, 1, 0, 0, 0, 1]
            || values[7] == 0
            || values[7] != values[8]
            || values[7] != values[9]
            || !cookies.insert(values[7])
            || values[10..14] != [1, 1, 1, 1]
            || values[14] == 0
            || values[14] != values[15]
            || values[16] == 0
            || values[16] != values[17]
            || values[18] != 1
            || fds.is_some_and(|fd| fd != values[14])
            || tasks.is_some_and(|count| count != values[16])
        {
            return Err("host offer primary error, socket identity, residual/reclaim or exact resources differ".into());
        }
        fds = Some(values[14]);
        tasks = Some(values[16]);
    }
    tasks.ok_or_else(|| "missing native host offer baseline".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_consumed_replaced_or_unreclaimed_host_offers_and_hidden_owners() {
        let valid=(0..20).map(|cycle|{let cookie=100+cycle;format!("{cycle}\t98\t1\t0\t0\t0\t1\t{cookie}\t{cookie}\t{cookie}\t1\t1\t1\t1\t9\t9\t2\t2\t1\n")}).collect::<String>();
        assert_eq!(cases(&valid).unwrap(), 2);
        for bad in [
            valid.replacen("0\t98", "0\t0", 1),
            valid.replacen("\t98\t1\t0\t0", "\t98\t1\t1\t0", 1),
            valid.replacen("\t100\t100\t100", "\t100\t101\t100", 1),
            valid.replacen("\t101\t101\t101", "\t100\t100\t100", 1),
            valid.replacen("\t1\t1\t1\t1\t9", "\t1\t1\t1\t0\t9", 1),
            valid.replacen("\t9\t9\t2\t2", "\t9\t10\t2\t2", 1),
            valid.replacen("\t9\t9\t2\t2", "\t9\t9\t2\t3", 1),
            valid.lines().take(19).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(cases(&bad).is_err());
        }
    }
}
