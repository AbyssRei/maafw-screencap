// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Controlled failed HRESULTs, actual old-object lifetimes and next-call state.
use crate::{Result, records};
use std::{fs, path::Path};
const CALLS: &str = "cycle\tmode\tcode\tempty\torder\tacquires\tnative_acquires\tinjected\tcopies\tmaps\tunmaps\treleases\tconversions\ttimeout\tviolation\tcaller\tacquire_duplication\tbegin_fault\tend_fault\tend_observation\tend_recovery\tend_recovery_observation\tend_close\tend_close_observation";
const STATES: &str = "cycle\tphase\tslots\tmonitor\tjust_initialized\twidth\theight\tdevice\tcontext\tfactory\tadapter\toutput\tduplication\tstaging";
const LIFE: &str = "cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr";
const CODES: [u64; 3] = [0x887a0026, 0x887a0027, 0x887a0005];
fn table(text: &str, header: &str, n: usize) -> Result<Vec<Vec<u64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("Acquire errors table header differs".into());
    }
    lines
        .map(|line| {
            let row = line
                .split('\t')
                .map(|v| -> Result<u64> {
                    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                        return Err("invalid Acquire errors scalar".into());
                    }
                    Ok(v.parse()?)
                })
                .collect::<Result<Vec<_>>>()?;
            if row.len() != n {
                return Err("Acquire errors column count differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn states(
    before: &[u64],
    failed: &[u64],
    recovered: &[u64],
    closed: &[u64],
    cycle: usize,
) -> Result<()> {
    let cycle64 = u64::try_from(cycle)?;
    for (phase, row) in [before, failed, recovered, closed].into_iter().enumerate() {
        if row[..2] != [cycle64, u64::try_from(phase)?] || row[4] != 0 {
            return Err("Acquire state cycle/phase/warm flag differs".into());
        }
        let mask = row[7..14]
            .iter()
            .enumerate()
            .fold(0_u64, |bits, (i, p)| bits | (u64::from(*p != 0) << i));
        if row[2] != mask {
            return Err("Acquire state mask differs from actual interface slots".into());
        }
    }
    if before[2] != 127
        || before[3] == 0
        || before[5] == 0
        || before[6] == 0
        || recovered[2] != 127
        || recovered[3..7] != before[3..7]
        || closed[2] != 0
        || closed[3] != recovered[3]
        || closed[5..7] != [0, 0]
    {
        return Err("Acquire warm/recovery/closed state differs".into());
    }
    if cycle.is_multiple_of(3) {
        if failed[2] != 31
            || failed[3] != 0
            || failed[5..12] != before[5..12]
            || failed[12..14] != [0, 0]
            || recovered[7..10] != before[7..10]
        {
            return Err("ACCESS_LOST partial release/retained descriptor differs".into());
        }
    } else if failed[2..] != before[2..] || recovered[2..] != before[2..] {
        return Err("other failed HRESULT changed live native state".into());
    }
    Ok(())
}
fn lifetime(
    row: &[u64],
    first: &[u64],
    call: &[u64],
    before: &[u64],
    cycle: usize,
    phase: usize,
    kind: usize,
) -> Result<()> {
    // This table has three prefix columns before the u64 observer transport;
    // DWORD threads and UINT IDs remain 32-bit, unlike pointers and QPC values.
    for field in [8, 9, 12] {
        u32::try_from(row[field])?;
    }
    let expected_object = before[[7, 8, 12, 13][kind]];
    if row[..3]
        != [
            u64::try_from(cycle)?,
            u64::try_from(phase)?,
            u64::try_from(kind)?,
        ]
        || row[3..7] != [1, if kind == 2 { 2 } else { 1 }, u64::try_from(kind)?, 1]
        || row[8] != call[15]
        || row[10] == 0
        || row[10] > call[17]
        || row[13] != expected_object
        || row[14] != 0
        || [3, 4, 5, 6, 8, 10, 12, 13, 14]
            .into_iter()
            .any(|i| row[i] != first[i])
    {
        return Err("Acquire old-object registration/identity differs".into());
    }
    let lost = cycle.is_multiple_of(3);
    let done = phase == 3 || lost && phase >= 1 && kind >= 2;
    if !done {
        if [row[7], row[9], row[11]] != [0; 3] {
            return Err("Acquire old native owner notified before its release phase".into());
        }
    } else {
        let earliest = if lost && kind >= 2 {
            call[17]
        } else {
            call[21]
        };
        let latest = call[match phase {
            1 => 19,
            2 => 21,
            _ => 23,
        }];
        if row[7] != 1 || row[9] == 0 || row[11] < earliest || row[11] > latest {
            return Err("Acquire old native release notification count/time differs".into());
        }
    }
    Ok(())
}
fn pixels(bytes: &[u8]) -> Result<u8> {
    if bytes.len() != 18432 {
        return Err("Acquire errors target span differs".into());
    }
    let (marker, phase) = match &bytes[..3] {
        [78, 95, 96] => ([78, 95, 96], 0),
        [26, 43, 60] => ([26, 43, 60], 1),
        _ => return Err("Acquire errors marker differs".into()),
    };
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 16 && y < 16 {
                marker
            } else if x < 32 {
                [31, 83, 167]
            } else if y < 32 {
                [185, 77, 39]
            } else {
                [59, 226, 129]
            };
            let at = (y * 96 + x) * 3;
            if bytes[at..at + 3] != expected {
                return Err("Acquire errors target pixels differ".into());
            }
        }
    }
    Ok(phase)
}
fn side(root: &Path) -> Result<serde_json::Value> {
    let abi = fs::read_to_string(root.join("abi.tsv"))?;
    let expected = format!(
        "1\t8\t8\t8\t4\t4\t4\t{}\t{}\t{}\n",
        CODES[0], CODES[1], CODES[2]
    );
    if abi != expected {
        return Err("Acquire errors native ABI/HRESULT constants differ".into());
    }
    let calls = table(&fs::read_to_string(root.join("calls.tsv"))?, CALLS, 24)?;
    let state = table(&fs::read_to_string(root.join("states.tsv"))?, STATES, 14)?;
    let life = table(&fs::read_to_string(root.join("lifetimes.tsv"))?, LIFE, 15)?;
    if calls.len() != 20 || state.len() != 80 || life.len() != 320 {
        return Err("incomplete Acquire errors matrix".into());
    }
    let caller = calls[0][15];
    u32::try_from(caller)?;
    if caller == 0 {
        return Err("Acquire errors caller absent".into());
    }
    let mut objects = Vec::new();
    let mut markers = Vec::new();
    for (cycle, call) in calls.iter().enumerate() {
        if call[..15]
            != [
                u64::try_from(cycle)?,
                u64::try_from(cycle % 3)?,
                CODES[cycle % 3],
                1,
                1,
                1,
                0,
                1,
                0,
                0,
                0,
                0,
                0,
                2000,
                0,
            ]
            || call[15] != caller
            || call[16] != state[cycle * 4][12]
            || call[17] == 0
            || call[17..24].windows(2).any(|p| p[0] > p[1])
        {
            return Err("controlled Acquire call/result/order/time evidence differs".into());
        }
        let before = &state[cycle * 4];
        states(
            before,
            &state[cycle * 4 + 1],
            &state[cycle * 4 + 2],
            &state[cycle * 4 + 3],
            cycle,
        )?;
        let addresses = [before[7], before[8], before[12], before[13]]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        if addresses.len() != 4 {
            return Err("Acquire old native identities duplicated".into());
        }
        for kind in 0..4 {
            let first = &life[cycle * 16 + kind];
            let final_row = &life[cycle * 16 + 12 + kind];
            for phase in 0..4 {
                let observed = &life[cycle * 16 + phase * 4 + kind];
                lifetime(observed, first, call, before, cycle, phase, kind)?;
                if observed[7] != 0 && [7, 9, 11].into_iter().any(|i| observed[i] != final_row[i]) {
                    return Err("Acquire release notification changed after observation".into());
                }
            }
            objects.push(serde_json::json!({"cycle":cycle,"kind":kind,"method":final_row[4],"interface_identity":final_row[13],"callback_thread":final_row[9],"callback_qpc":final_row[11]}));
        }
        for label in ["warm", "recovery"] {
            let name = format!("{cycle}-{label}");
            let current = fs::read(root.join(format!("{name}.bgr")))?;
            markers.push(pixels(&current)?);
            if fs::read(root.join(format!("{name}-held.bgr")))? != current {
                return Err("Acquire errors held CPU pixels changed".into());
            }
            for suffix in ["", "-held"] {
                let layout = fs::read_to_string(root.join(format!("{name}{suffix}.txt")))?;
                let values = layout
                    .trim_end_matches('\n')
                    .split('\t')
                    .map(str::parse::<u64>)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                if values.len() != 4
                    || values[0..2] != [96, 64]
                    || values[2] < 288
                    || values[2] % 3 != 0
                    || values[3] != 18432
                {
                    return Err("Acquire errors target layout differs".into());
                }
            }
        }
    }
    Ok(
        serde_json::json!({"caller":caller,"objects":objects,"markers":markers,"public_calls":60,"controlled_failures":20,"access_lost":7,"retained_other_errors":13,"recoveries":20,"observed_old_objects":80,"retained_cpu":40,"actual_driver_calls_at_controlled_fault":0}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) =
        match args {
            [p] => (p, false),
            [p, f] if f == "--check-only" => (p, true),
            _ => return Err(
                "expected upstream-trace desktop-acquire-errors <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let source = side(&root.join("source"))?;
    let rust = side(&root.join("rust"))?;
    let mut artifacts = Vec::new();
    for name in ["source", "rust"] {
        for entry in fs::read_dir(root.join(name))? {
            let path = entry?.path();
            if path.is_file() {
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
    }
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"source":source,"rust":rust,"artifacts":artifacts,"physical_device_faults":false,"new_recovery_objects_not_observed":true,"global_gpu_allocation_proof":false,"full_P1_accepted":false,"scope":"Controlled AcquireNextFrame failed HRESULTs on live original/Rust objects, nominal unchanged2000ms argument and zero driver calls only at the explicit injected boundary. ACCESS_LOST releases old duplication/staging and clears monitor while retaining device/context/factory/adapter/output and descriptor; next operation reselects adapter/output and recreates duplication/staging while retaining the original device/context/factory. Other errors preserve live objects. Actual old D3D destruction and weaker DXGI private-owner callbacks, full known/held target pixels. No physical ACCESS_LOST/timeout/device removal, actual two-second failed driver wait, multi-monitor/resolution event, full recovery-generation object observation, global GPU proof or full P1 acceptance."});
    let output = root.join("acquire-errors-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Acquire errors report differs from actual inputs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup acquire_errors public_calls=120 controlled_failures=40 access_lost=14 retained_other_errors=26 recoveries=40 observed_old_objects=160 retained_cpu=80 physical_device_faults=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(phase: u64) -> Vec<u64> {
        vec![
            0, phase, 127, 99, 0, 1920, 1080, 100, 101, 102, 103, 104, 105, 106,
        ]
    }
    #[test]
    fn accepts_partial_access_lost_and_rejects_full_reset_or_other_error_teardown() {
        let before = row(0);
        let mut failed = row(1);
        failed[2] = 31;
        failed[3] = 0;
        failed[12] = 0;
        failed[13] = 0;
        let recovered = row(2);
        let mut closed = row(3);
        closed[2] = 0;
        closed[5..].fill(0);
        states(&before, &failed, &recovered, &closed, 0).unwrap();
        let mut replaced = recovered.clone();
        replaced[7] += 1;
        assert!(states(&before, &failed, &replaced, &closed, 0).is_err());
        let mut wrong = failed.clone();
        wrong[7] = 0;
        wrong[2] = 30;
        assert!(states(&before, &wrong, &recovered, &closed, 0).is_err());
        let mut b = before.clone();
        b[0] = 1;
        let mut f = row(1);
        f[0] = 1;
        let mut r = recovered;
        r[0] = 1;
        let mut c = closed;
        c[0] = 1;
        states(&b, &f, &r, &c, 1).unwrap();
        f[12] = 0;
        f[2] = 95;
        assert!(states(&b, &f, &r, &c, 1).is_err());
        assert!(table("wrong\n", CALLS, 24).is_err());
        assert!(pixels(&[0; 18432]).is_err());
    }
}
