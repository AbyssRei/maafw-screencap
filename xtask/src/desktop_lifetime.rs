// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Finite real-object notifications, distinct from global GPU allocation counts.
use crate::{Result, records};
use std::{fs, path::Path};
const CALLS: &str = "cycle\theld\tbefore_slots\tafter_slots\tcaller\tbegin_close\tend_close\trelease_qpc\tfinal_qpc";
const LIFETIMES: &str = "cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr";
fn table(text: &str, header: &str, columns: usize) -> Result<Vec<Vec<u64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("desktop lifetime header differs".into());
    }
    lines
        .map(|line| {
            let values = line
                .split('\t')
                .map(|v| -> Result<u64> {
                    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                        return Err("invalid desktop lifetime scalar".into());
                    }
                    Ok(v.parse()?)
                })
                .collect::<Result<Vec<_>>>()?;
            if values.len() != columns {
                return Err("desktop lifetime column count differs".into());
            }
            Ok(values)
        })
        .collect()
}
fn phase(
    r: &[u64],
    first: &[u64],
    call: &[u64],
    cycle: usize,
    stage: usize,
    kind: usize,
) -> Result<()> {
    // The observer widens DWORD thread identities and UINT callback IDs to
    // u64. Object addresses (column 13) and QPC values retain their full width.
    for field in [8, 9, 12] {
        u32::try_from(r[field])?;
    }
    if r[..3]
        != [
            u64::try_from(cycle)?,
            u64::try_from(stage)?,
            u64::try_from(kind)?,
        ]
        || r[3] != 1
        || r[4] != (if kind == 2 { 2 } else { 1 })
        || r[5] != u64::try_from(kind)?
        || r[6] != 1
        || r[8] != call[4]
        || r[10] == 0
        || r[10] > call[5]
        || r[13] == 0
        || r[14] != 0
        || [3, 4, 5, 6, 8, 10, 12, 13, 14]
            .into_iter()
            .any(|i| r[i] != first[i])
    {
        return Err("native lifetime registration/identity/caller fields differ".into());
    }
    if stage == 0 {
        if r[7] != 0 || r[9] != 0 || r[11] != 0 {
            return Err("live object destruction was reported before close".into());
        }
    } else if stage == 1 {
        if r[7] > 1 || r[7] == 1 && (r[9] == 0 || r[11] < call[5] || r[11] > call[7]) {
            return Err("intermediate native callback count/time differs".into());
        }
        if call[1] == 1 && (kind == 0 || kind == 3) && [r[7], r[9], r[11]] != [0; 3] {
            return Err("explicit staging texture/device references notified early".into());
        }
    } else if r[7] != 1
        || r[9] == 0
        || r[11] < call[5]
        || r[11] > call[8]
        || call[1] == 1 && (kind == 0 || kind == 3) && r[11] < call[7]
    {
        return Err(
            "final native destruction/association notification missing or premature".into(),
        );
    }
    Ok(())
}
fn pixels(bytes: &[u8]) -> Result<u32> {
    if bytes.len() != 96 * 64 * 3 {
        return Err("desktop lifetime target pixel span differs".into());
    }
    let phase = match &bytes[..3] {
        [78, 95, 96] => 0,
        [26, 43, 60] => 1,
        _ => return Err("desktop lifetime target marker differs".into()),
    };
    for y in 0..64 {
        for x in 0..96 {
            let color = if x < 16 && y < 16 {
                if phase == 0 {
                    [78, 95, 96]
                } else {
                    [26, 43, 60]
                }
            } else if x < 32 {
                [31, 83, 167]
            } else if y < 32 {
                [185, 77, 39]
            } else {
                [59, 226, 129]
            };
            let at = (y * 96 + x) * 3;
            if bytes[at..at + 3] != color {
                return Err("desktop lifetime target pixels differ".into());
            }
        }
    }
    Ok(phase)
}
fn side(root: &Path) -> Result<serde_json::Value> {
    if fs::read(root.join("abi.tsv"))? != b"1\t8\t8\t8\t4\t4\t4\n" {
        return Err("native lifetime scalar ABI differs".into());
    }
    let calls = table(&fs::read_to_string(root.join("calls.tsv"))?, CALLS, 9)?;
    let lifetimes = table(
        &fs::read_to_string(root.join("lifetimes.tsv"))?,
        LIFETIMES,
        15,
    )?;
    if calls.len() != 20 || lifetimes.len() != 240 {
        return Err("incomplete native object lifetime matrix".into());
    }
    let caller = calls[0][4];
    u32::try_from(caller)?;
    if caller == 0 {
        return Err("native lifetime caller absent".into());
    }
    let mut objects = Vec::new();
    let mut markers = Vec::new();
    for (cycle, call) in calls.iter().enumerate() {
        if call[..4] != [u64::try_from(cycle)?, u64::try_from(cycle % 2)?, 127, 0]
            || call[4] != caller
            || call[5] == 0
            || call[5] > call[6]
            || call[6] > call[7]
            || call[7] > call[8]
        {
            return Err("native close/held-reference state or clock order differs".into());
        }
        let addresses = std::array::from_fn::<_, 4, _>(|kind| lifetimes[cycle * 12 + kind][13]);
        if addresses
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 4
        {
            return Err("native object identities were duplicated across kinds".into());
        }
        for kind in 0..4 {
            let first = &lifetimes[cycle * 12 + kind];
            for stage in 0..3 {
                phase(
                    &lifetimes[cycle * 12 + stage * 4 + kind],
                    first,
                    call,
                    cycle,
                    stage,
                    kind,
                )?;
            }
            let final_row = &lifetimes[cycle * 12 + 8 + kind];
            objects.push(serde_json::json!({"cycle":cycle,"kind":kind,"method":final_row[4],"interface_identity":final_row[13],"callback_thread":final_row[9],"callback_qpc":final_row[11]}));
        }
        let current = fs::read(root.join(format!("{cycle}-current.bgr")))?;
        markers.push(pixels(&current)?);
        if fs::read(root.join(format!("{cycle}-held.bgr")))? != current {
            return Err("CPU pixels changed after native/window shutdown".into());
        }
    }
    Ok(
        serde_json::json!({"caller":caller,"objects":objects,"markers":markers,"public_capture_calls":20,"d3d_destruction_notifications":60,"dxgi_private_owner_releases":20,"held_texture_controls":10,"held_device_controls":10,"post_close_cpu_images":20}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace desktop-lifetime <actual external matrix> [--check-only]"
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
    let report = serde_json::json!({"layout_version":1,"source":source,"rust":rust,"artifacts":artifacts,"extra_flush":false,"global_gpu_allocation_proof":false,"full_P1_accepted":false,
        "scope":"Finite original/Rust DesktopDup object lifetime matrix. Real device/context/staging texture use ID3DDestructionNotifier; real duplication uses a unique private-data IUnknown owner and its weaker association-release meaning is retained. Twenty actual public captures per side, eighty observed objects per side, ten explicit staging/device reference-pair controls and full known CPU pixels after native/window shutdown. Registration borrows add no retained native reference. No implicit parent-retention assumption, Flush/ClearState, GPU reset, screenshot retry, arbitrary device-loss, global GPU memory closure or whole P1 acceptance."});
    let output = root.join("desktop-lifetime-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("desktop lifetime report differs from actual inputs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "DesktopDup lifetime sides=2 public_calls=40 native_objects=160 d3d_destruction_notifications=120 dxgi_private_owner_releases=40 held_texture_controls=20 held_device_controls=20 retained_cpu=40 extra_flush=0 global_gpu_allocation_proof=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn call(held: u64) -> Vec<u64> {
        vec![0, held, 127, 0, 7, 100, 101, 102, 104]
    }
    fn record(stage: usize, kind: usize, held: bool) -> Vec<u64> {
        let done = stage == 2 || stage == 1 && !(held && (kind == 0 || kind == 3));
        vec![
            0,
            stage as u64,
            kind as u64,
            1,
            if kind == 2 { 2 } else { 1 },
            kind as u64,
            1,
            u64::from(done),
            7,
            if done { 11 } else { 0 },
            90,
            if done {
                if held && (kind == 0 || kind == 3) {
                    103
                } else {
                    101
                }
            } else {
                0
            },
            0,
            1000 + kind as u64,
            0,
        ]
    }
    #[test]
    fn accepts_distinct_notifier_private_owner_and_explicit_reference_observations() {
        for held in [false, true] {
            for kind in 0..4 {
                let first = record(0, kind, held);
                for stage in 0..3 {
                    phase(
                        &record(stage, kind, held),
                        &first,
                        &call(u64::from(held)),
                        0,
                        stage,
                        kind,
                    )
                    .unwrap();
                }
            }
        }
        assert!(table("wrong\n", CALLS, 9).is_err());
        assert!(table(&(CALLS.to_owned() + "\n0\t1\n"), CALLS, 9).is_err());
    }
    #[test]
    fn rejects_wrong_registration_thread_identity_early_release_and_missing_notification() {
        let first = record(0, 3, true);
        let c = call(1);
        for (stage, index, bad) in [
            (0, 7, 1),
            (0, 9, 11),
            (0, 11, 99),
            (1, 7, 1),
            (1, 9, 11),
            (1, 11, 101),
            (2, 7, 0),
            (2, 7, 2),
            (2, 9, 0),
            (2, 11, 101),
            (2, 11, 105),
            (2, 3, 0),
            (2, 4, 2),
            (2, 5, 2),
            (2, 6, 0),
            (2, 8, 8),
            (2, 10, 99),
            (2, 12, 1),
            (2, 13, 1004),
            (2, 14, 2147500037),
        ] {
            let mut r = record(stage, 3, true);
            r[index] = bad;
            assert!(
                phase(&r, &first, &c, 0, stage, 3).is_err(),
                "stage={stage} field={index}"
            );
        }
    }
    #[test]
    fn refuses_changed_full_target_pixels_and_incomplete_spans() {
        let mut image = vec![0; 96 * 64 * 3];
        for y in 0..64 {
            for x in 0..96 {
                let color = if x < 16 && y < 16 {
                    [78, 95, 96]
                } else if x < 32 {
                    [31, 83, 167]
                } else if y < 32 {
                    [185, 77, 39]
                } else {
                    [59, 226, 129]
                };
                image[(y * 96 + x) * 3..(y * 96 + x) * 3 + 3].copy_from_slice(&color);
            }
        }
        assert_eq!(pixels(&image).unwrap(), 0);
        image[9000] ^= 1;
        assert!(pixels(&image).is_err());
        assert!(pixels(&image[..9000]).is_err());
    }
}
