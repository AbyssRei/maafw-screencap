// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Source-only observations, not a successful manager/Rust parity oracle.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

const HEADER: &str = "slot\tarm\tphase\twgc_object_alive\tbegin_qpc\tend_qpc\tengaged\twidth\theight\tknown_scene\tfnv64";

fn scalar(value: &str) -> Result<u64> {
    let result = value.parse::<u64>()?;
    if result.to_string() != value {
        return Err("Noncanonical order observation scalar".into());
    }
    Ok(result)
}

fn numbers(text: &str, rows: usize, columns: usize) -> Result<Vec<Vec<u64>>> {
    let result = text
        .lines()
        .map(|line| line.split('\t').map(scalar).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    if result.len() != rows || result.iter().any(|r| r.len() != columns) {
        return Err("Incomplete or extra order observation rows/columns".into());
    }
    Ok(result)
}

fn phases() -> Vec<(u64, String, bool)> {
    let mut result = Vec::new();
    for arm in 0..4 {
        if arm == 2 {
            result.extend(["wgc-warm", "wgc-test"].map(|phase| (arm, phase.into(), true)));
        }
        result.extend(["dd-warm", "dd-test"].map(|phase| (arm, phase.into(), arm == 2)));
        if arm == 1 {
            result.extend(
                ["wgc-warm", "wgc-test", "dd-with-wgc"].map(|phase| (arm, phase.into(), true)),
            );
        }
        result.extend((0..3).map(|i| (arm, format!("dd-post-object-{i}"), false)));
    }
    result
}

fn summary(text: &str) -> Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("Invalid source order summary")?;
        if result.insert(key.into(), scalar(value)?).is_some() {
            return Err("Duplicate source order summary field".into());
        }
    }
    let schema = *result.get("schema").ok_or("Missing order schema")?;
    if !matches!(schema, 1 | 2) {
        return Err("Unsupported source order schema".into());
    }
    let mut expected = BTreeMap::from([
        ("schema".into(), schema),
        ("shots".into(), 25),
        ("closed_windows".into(), 4),
        ("dispatcher_completed".into(), 1),
        ("dispatcher_within_budget".into(), 1),
        ("caller_dpi_preserved".into(), 1),
        ("positive_controls".into(), 1),
        ("source_bodies_unchanged".into(), 1),
        ("extra_snapshot_copy".into(), 1),
        ("post_close_calls_are_retries".into(), 0),
        ("original_failure_reclassified".into(), 0),
        ("full_P1_acceptance".into(), 0),
        ("cv_8u".into(), 0),
    ]);
    if schema == 2 {
        expected.insert("frame_driver".into(), 1);
    }
    for field in ["qpc_frequency", "cv_cn_shift", "cv_8uc3"] {
        expected.insert(
            field.into(),
            *result
                .get(field)
                .ok_or("Missing native order summary field")?,
        );
    }
    let shift = result["cv_cn_shift"];
    if result != expected
        || result["qpc_frequency"] == 0
        || !(1..=16).contains(&shift)
        || result["cv_8uc3"]
            != 2_u64
                .checked_shl(u32::try_from(shift)?)
                .ok_or("OpenCV type shift overflow")?
    {
        return Err("Source order completion/native OpenCV contract differs".into());
    }
    Ok(result)
}

fn expected_pixels() -> Vec<u8> {
    (0..64)
        .flat_map(|y| {
            (0..96).flat_map(move |x| {
                if x < 32 {
                    [0x1f, 0x53, 0xa7]
                } else if y < 32 {
                    [0xb9, 0x4d, 0x27]
                } else {
                    [0x3b, 0xe2, 0x81]
                }
            })
        })
        .collect()
}

fn frame_drivers(rows: &[Vec<u64>], windows: &[Vec<u64>]) -> Result<()> {
    for (arm, row) in rows.iter().enumerate() {
        let window = &windows[arm];
        if row[0] != u64::try_from(arm)?
            || row[1] == 0
            || row[1] == window[1]
            || row[2] != window[2]
            || row[3] != window[5]
            || row[4] == 0
            || row[5] <= row[4]
            || row[6] != 0
            || row[7..] != [1, 1, 1]
        {
            return Err("Source order frame-driver owner/progress/isolation/close differs".into());
        }
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace framepool-order <external observations> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        inputs.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let contract = summary(std::str::from_utf8(&read("summary.txt")?)?)?;
    let windows = numbers(std::str::from_utf8(&read("windows.tsv")?)?, 4, 6)?;
    let owners = numbers(std::str::from_utf8(&read("owners.tsv")?)?, 4, 3)?;
    let states = numbers(std::str::from_utf8(&read("states.tsv")?)?, 25, 13)?;
    for arm in 0..4 {
        let r = &windows[arm];
        if r[0] != u64::try_from(arm)?
            || r[1..].contains(&0)
            || r[2] != r[4]
            || r[2] == r[3]
            || r[3] != windows[0][3]
            || r[5] != windows[0][5]
            || r[2..].iter().any(|id| u32::try_from(*id).is_err())
            || owners[arm][0] != r[0]
            || owners[arm][1] == 0
            || owners[arm][1] > owners[arm][2]
        {
            return Err("Source order target/thread/object-lifetime identity differs".into());
        }
    }
    let drivers = if contract["schema"] == 2 {
        let rows = numbers(std::str::from_utf8(&read("frame-drivers.tsv")?)?, 4, 10)?;
        frame_drivers(&rows, &windows)?;
        Some(rows)
    } else {
        None
    };
    let expected_files = 55 + usize::from(drivers.is_some());
    let shots = String::from_utf8(read("shots.tsv")?)?;
    let mut lines = shots.lines();
    if lines.next() != Some(HEADER) {
        return Err("Source order shot header differs".into());
    }
    let expected = expected_pixels();
    let mut last = 0;
    let mut observations = Vec::new();
    for (index, (arm, phase, alive)) in phases().iter().enumerate() {
        let fields: Vec<_> = lines
            .next()
            .ok_or("Missing source order call")?
            .split('\t')
            .collect();
        if fields.len() != 11 || fields[2] != phase {
            return Err("Source order call phase/field count differs".into());
        }
        let row = fields
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, v)| scalar(v))
            .collect::<Result<Vec<_>>>()?;
        if row[0] != u64::try_from(index)?
            || row[1] != *arm
            || row[2] != u64::from(*alive)
            || row[3] == 0
            || row[3] < last
            || row[4] < row[3]
            || row[5] > 1
            || row[8] > 1
        {
            return Err("Source order index/lifetime/clock/scalars differ".into());
        }
        last = row[4];
        let owner = &owners[usize::try_from(*arm)?];
        if (*alive && row[4] > owner[1])
            || (phase.starts_with("dd-post-object-") && row[3] < owner[2])
        {
            return Err("Source order object boundary contradicts call clock".into());
        }
        let state = &states[index];
        if state[0] != row[0]
            || [1, 7].into_iter().any(|start| {
                let s = &state[start..start + 6];
                s[1] > 255 || s[2] > 1 || s[3] > 1 || s[4..] != [96, 64]
            })
        {
            return Err("Source order native target geometry/state differs".into());
        }
        let layout = numbers(
            std::str::from_utf8(&read(&format!("shot-{index}/layout.tsv"))?)?,
            1,
            5,
        )?;
        let l = &layout[0];
        let pixels = read(&format!("shot-{index}/image.bgr"))?;
        let empty = l == &[row[5], 0, 0, 0, 0] && pixels.is_empty();
        let image = l == &[row[5], 96, 64, contract["cv_8uc3"], 288] && pixels.len() == 18432;
        if l[0] != row[5] || l[1..3] != row[6..8] || (!empty && !image) || (image && row[5] != 1) {
            return Err("Source order payload span/layout differs".into());
        }
        let fnv = pixels.iter().fold(0xcbf29ce484222325_u64, |v, b| {
            (v ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        let known = pixels == expected;
        if fnv != row[9]
            || known != (row[8] == 1)
            || ((*arm == 0 || *arm == 3 || phase.starts_with("wgc-")) && (row[5] != 1 || !known))
        {
            return Err("Source order pixel digest/known-scene/positive control differs".into());
        }
        let differences: Vec<_> = pixels
            .as_chunks::<3>()
            .0
            .iter()
            .zip(expected.as_chunks::<3>().0.iter())
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect();
        let perimeter: Vec<_> = (0..96 * 64)
            .filter(|i| {
                let (x, y) = (i % 96, i / 96);
                x < 3 || y < 3 || x >= 93 || y >= 61
            })
            .collect();
        observations.push(
            serde_json::json!({"slot":index,"arm":arm,"phase":phase,"wgc_object_alive":alive,
            "engaged":row[5]==1,"known_scene":known,"different_pixels":differences.len(),
            "exact_outer_three_pixel_difference":image && differences==perimeter,"fnv64":fnv,
            "begin_qpc":row[3],"end_qpc":row[4]}),
        );
    }
    if lines.next().is_some() {
        return Err("Extra source order call".into());
    }
    let mut actual = 0;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 order record")?;
        let kind = entry.file_type()?;
        if kind.is_file() && name == "order-validation.json" {
            continue;
        }
        if kind.is_file()
            && [
                "shots.tsv",
                "states.tsv",
                "owners.tsv",
                "windows.tsv",
                "summary.txt",
            ]
            .contains(&name.as_str())
            || kind.is_file() && drivers.is_some() && name == "frame-drivers.tsv"
        {
            actual += 1;
        } else if kind.is_dir() && (0..25).any(|i| name == format!("shot-{i}")) {
            for item in fs::read_dir(entry.path())? {
                let item = item?;
                if !item.file_type()?.is_file()
                    || !["image.bgr", "layout.tsv"]
                        .iter()
                        .any(|n| item.file_name() == *n)
                {
                    return Err("Unexpected source order payload entry/link".into());
                }
                actual += 1;
            }
        } else {
            return Err("Unexpected source order root entry/link".into());
        }
    }
    if actual != expected_files || inputs.len() != expected_files {
        return Err("Source order file inventory differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let mut report = serde_json::json!({"layout_version":1,"actual_source_calls":25,"owned_windows_closed":4,
        "positive_control_calls":14,"native_cv_contract":contract,"windows":windows,"object_boundaries":owners,
        "window_states":states,"observations":observations,"inputs":inputs,"registered_files":55,
        "scope":"Original-only owned-window order/lifetime diagnostic with immediate extra CPU clones. Foreground/topmost and treatment pixels are observations. C++ owner destruction is not native session/callback quiescence; no manager/Rust parity, capture freshness, border cause, historical failure reclassification or full P1 acceptance."});
    if let Some(drivers) = drivers {
        report["layout_version"] = serde_json::json!(2);
        report["registered_files"] = serde_json::json!(expected_files);
        report["frame_drivers"] = serde_json::json!(drivers);
        report["scope"] = serde_json::json!(
            "Original-only order/lifetime diagnostic with an owned, same-output, nonoverlapping desktop marker, observed paint progress and closure. Target pixels and source calls/waits/order remain unchanged. Marker presentation is fixture work, not screenshot retry or per-call capture-freshness proof. C++ owner destruction is not native session/callback quiescence; no manager/Rust parity, border cause, historical failure reclassification or full P1 acceptance."
        );
    }
    let output = root.join("order-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved source order observations differ".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "source order records calls=25 windows_closed=4 positive_controls=14 files={expected_files} diagnostic_only=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_truncated_extra_and_noncanonical_native_rows() {
        assert!(numbers("0\t1\n1\t2\n", 2, 2).is_ok());
        for text in [
            "0\t1\n",
            "0\t1\n1\t2\n2\t3\n",
            "00\t1\n1\t2\n",
            "0\t1\t2\n1\t2\n",
        ] {
            assert!(numbers(text, 2, 2).is_err());
        }
        assert_eq!(phases().len(), 25);
        assert_eq!(expected_pixels().len(), 18432);
    }

    #[test]
    fn rejects_stalled_foreign_overlapping_and_unclosed_frame_drivers() {
        let windows: Vec<Vec<u64>> = (0..4)
            .map(|arm| vec![arm, 100 + arm, 7, 9, 7, 12])
            .collect();
        let rows: Vec<Vec<u64>> = (0..4)
            .map(|arm| vec![arm, 200 + arm, 7, 12, 1, 2, 0, 1, 1, 1])
            .collect();
        frame_drivers(&rows, &windows).unwrap();
        for (field, value) in [
            (0, 1),
            (1, 0),
            (1, 100),
            (2, 8),
            (3, 13),
            (4, 0),
            (5, 1),
            (6, 1),
            (7, 0),
            (8, 0),
            (9, 0),
        ] {
            let mut invalid = rows.clone();
            invalid[0][field] = value;
            assert!(frame_drivers(&invalid, &windows).is_err());
        }
    }
}
