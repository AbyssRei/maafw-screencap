// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Finite successful-capture GPU ownership, preserving source uninit retention.
use crate::{Result, records};
use std::{fs, path::Path};
const CALLS: &str = "cycle\theld\tbefore_slots\tstopped_slots\tcaller\tbegin_stop\tend_stop\tbegin_drop\tend_drop\trelease_qpc\tfinal_qpc\timage\tcache\tstopped_cache";
const LIFETIMES: &str = "cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr";
const KINDS: [u64; 3] = [0, 1, 3];
fn table(text: &str, header: &str, count: usize) -> Result<Vec<Vec<u64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("FramePool lifetime table header differs".into());
    }
    lines
        .map(|line| {
            let row = line
                .split('\t')
                .map(|v| -> Result<u64> {
                    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                        return Err("invalid FramePool lifetime scalar".into());
                    }
                    Ok(v.parse()?)
                })
                .collect::<Result<Vec<_>>>()?;
            if row.len() != count {
                return Err("FramePool lifetime column count differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn phase(
    row: &[u64],
    first: &[u64],
    call: &[u64],
    cycle: usize,
    stage: usize,
    kind: u64,
) -> Result<()> {
    // Native DWORD threads and the opaque UINT callback ID are transported
    // as u64; the interface address and QPC fields are separate 64-bit values.
    for field in [8, 9, 12] {
        u32::try_from(row[field])?;
    }
    if row[..3] != [u64::try_from(cycle)?, u64::try_from(stage)?, kind]
        || row[3..7] != [1, 1, kind, 1]
        || row[8] != call[4]
        || row[10] == 0
        || row[10] > call[5]
        || row[13] == 0
        || row[14] != 0
        || [3, 4, 5, 6, 8, 10, 12, 13, 14]
            .into_iter()
            .any(|i| row[i] != first[i])
    {
        return Err("FramePool native registration/interface/caller identity differs".into());
    }
    let held = call[1] == 1 && (kind == 0 || kind == 3);
    let live = stage == 0 || stage == 1 && kind != 3 || stage < 3 && held;
    if live {
        if [row[7], row[9], row[11]] != [0; 3] {
            return Err("FramePool retained native object notified before release".into());
        }
    } else {
        let required = stage == 3 || stage == 2 && call[1] == 0;
        let earliest = if stage == 3 && held {
            call[9]
        } else if kind == 3 {
            call[5]
        } else {
            call[7]
        };
        let latest = match stage {
            1 => call[7],
            2 => call[9],
            _ => call[10],
        };
        if row[7] > 1
            || required && row[7] != 1
            || row[7] == 1 && (row[9] == 0 || row[11] < earliest || row[11] > latest)
        {
            return Err("FramePool native destruction callback count/time differs".into());
        }
    }
    Ok(())
}
fn pixels(bytes: &[u8]) -> Result<u8> {
    if bytes.len() != 18432 {
        return Err("FramePool lifetime CPU span differs".into());
    }
    let (marker, phase) = match &bytes[..3] {
        [78, 95, 96] => ([78, 95, 96], 0),
        [26, 43, 60] => ([26, 43, 60], 1),
        _ => return Err("FramePool lifetime marker differs".into()),
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
            let offset = (y * 96 + x) * 3;
            if bytes[offset..offset + 3] != expected {
                return Err("FramePool lifetime target pixels differ".into());
            }
        }
    }
    Ok(phase)
}
fn side(root: &Path, rust: bool) -> Result<serde_json::Value> {
    if fs::read(root.join("abi.tsv"))? != b"1\t8\t8\t8\t4\t4\t4\n" {
        return Err("FramePool lifetime native scalar ABI differs".into());
    }
    let calls = table(&fs::read_to_string(root.join("calls.tsv"))?, CALLS, 14)?;
    let rows = table(
        &fs::read_to_string(root.join("lifetimes.tsv"))?,
        LIFETIMES,
        15,
    )?;
    if calls.len() != 20 || rows.len() != 240 {
        return Err("incomplete FramePool lifetime matrix".into());
    }
    let caller = calls[0][4];
    u32::try_from(caller)?;
    if caller == 0 {
        return Err("FramePool native lifetime caller absent".into());
    }
    let mut objects = Vec::new();
    let mut markers = Vec::new();
    for (cycle, call) in calls.iter().enumerate() {
        if call[..4] != [u64::try_from(cycle)?, u64::try_from(cycle % 2)?, 127, 7]
            || call[4] != caller
            || call[5] == 0
            || call[5..11].windows(2).any(|p| p[0] > p[1])
            || call[11] == 0
            || call[12] == 0
            || call[11] == call[12]
            || call[12] != call[13]
        {
            return Err(
                "FramePool uninit/owner-drop/held/cache state or clock order differs".into(),
            );
        }
        let addresses = KINDS
            .iter()
            .enumerate()
            .map(|(i, _)| rows[cycle * 12 + i][13])
            .collect::<std::collections::BTreeSet<_>>();
        if addresses.len() != 3 {
            return Err("FramePool native identities duplicated across kinds".into());
        }
        for (index, kind) in KINDS.into_iter().enumerate() {
            let first = &rows[cycle * 12 + index];
            for stage in 0..4 {
                phase(
                    &rows[cycle * 12 + stage * 3 + index],
                    first,
                    call,
                    cycle,
                    stage,
                    kind,
                )?;
            }
            let final_row = &rows[cycle * 12 + 9 + index];
            objects.push(serde_json::json!({"cycle":cycle,"kind":kind,"interface_identity":final_row[13],"callback_thread":final_row[9],"callback_qpc":final_row[11]}));
        }
        let current = fs::read(root.join(format!("{cycle}-current.bgr")))?;
        markers.push(pixels(&current)?);
        if fs::read(root.join(format!("{cycle}-held.bgr")))? != current {
            return Err("FramePool CPU pixels changed after native/window shutdown".into());
        }
        if rust {
            for name in ["current", "held"] {
                if fs::read(root.join(format!("{cycle}-{name}.txt")))? != b"96\t64\t288\t18432\n" {
                    return Err("FramePool lifetime Rust CPU layout differs".into());
                }
            }
        }
    }
    Ok(
        serde_json::json!({"caller":caller,"objects":objects,"markers":markers,"public_capture_calls":20,"d3d_destruction_notifications":60,"explicit_reference_pairs":10,"retained_cpu":20,"uninit_gpu_slots":7}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) = match args {
        [p] => (p, false),
        [p, f] if f == "--check-only" => (p, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-lifetime <external matrix> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let source = side(&root.join("source"), false)?;
    let rust = side(&root.join("rust"), true)?;
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
    let report = serde_json::json!({"layout_version":1,"source":source,"rust":rust,"artifacts":artifacts,"extra_flush":false,"global_gpu_allocation_proof":false,"full_P1_accepted":false,"scope":"Finite native successful-capture FramePool owner lifetime, twenty calls per side. Explicit uninit retains device/context/swapchain and independent CPU cache per source; source owner destruction and extra texture/device pair release are distinct phases. Actual device/context/staging D3D destruction notifications, full known target/held CPU bytes and finite two-second notification observations. No swapchain destruction observation, global GPU allocation closure, general framework callback drainage, arbitrary failure, synchronous native destruction guarantee or full P1 acceptance."});
    let output = root.join("framepool-lifetime-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("FramePool lifetime report differs from actual inputs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool lifetime sides=2 public_calls=40 d3d_destruction_notifications=120 explicit_reference_pairs=20 uninit_gpu_slots=7 retained_cpu=40 extra_flush=0 global_gpu_allocation_proof=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn call(held: bool) -> [u64; 14] {
        [
            0,
            u64::from(held),
            127,
            7,
            7,
            100,
            101,
            102,
            103,
            104,
            105,
            500,
            600,
            600,
        ]
    }
    fn record(stage: usize, kind: u64, held: bool) -> [u64; 15] {
        let live = stage == 0 || stage == 1 && kind != 3 || stage < 3 && held && kind != 1;
        [
            0,
            stage as u64,
            kind,
            1,
            1,
            kind,
            1,
            u64::from(!live),
            7,
            if live { 0 } else { 8 },
            90,
            if live {
                0
            } else if held && kind != 1 {
                104
            } else if kind == 3 {
                101
            } else {
                103
            },
            1,
            1000 + kind,
            0,
        ]
    }
    #[test]
    fn distinguishes_uninit_owned_and_explicit_reference_lifetimes() {
        for held in [false, true] {
            for kind in KINDS {
                let first = record(0, kind, held);
                for stage in 0..4 {
                    phase(
                        &record(stage, kind, held),
                        &first,
                        &call(held),
                        0,
                        stage,
                        kind,
                    )
                    .unwrap();
                }
            }
        }
    }
    #[test]
    fn refuses_early_release_missing_final_notification_and_changed_native_identity() {
        for (stage, kind, index, value) in [
            (1, 0, 7, 1),
            (1, 1, 7, 1),
            (2, 3, 7, 1),
            (3, 0, 7, 0),
            (3, 1, 7, 2),
            (3, 3, 11, 103),
            (3, 0, 13, 99),
            (3, 1, 4, 2),
            (3, 3, 14, 1),
        ] {
            let first = record(0, kind, true);
            let mut row = record(stage, kind, true);
            row[index] = value;
            assert!(phase(&row, &first, &call(true), 0, stage, kind).is_err());
        }
        assert!(table("wrong\n", CALLS, 14).is_err());
        assert!(pixels(&[0; 18432]).is_err());
    }
}
