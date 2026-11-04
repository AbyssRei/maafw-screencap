// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent original-only property/device controls, never a parity waiver.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

const HEADER: &str = "slot\tarm\tphase\tkind\twgc_alive\tdevice_alive\tbegin_qpc\tend_qpc\tengaged\twidth\theight\tknown_scene\tfnv64";
const FILES: usize = 61;

fn scalar(value: &str) -> Result<u64> {
    let result = value.parse::<u64>()?;
    if result.to_string() != value {
        return Err("Noncanonical border observation scalar".into());
    }
    Ok(result)
}

fn numbers(text: &str, rows: usize, columns: usize) -> Result<Vec<Vec<u64>>> {
    let result = text
        .lines()
        .map(|line| line.split('\t').map(scalar).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    if result.len() != rows || result.iter().any(|r| r.len() != columns) {
        return Err("Incomplete or extra border observation rows/columns".into());
    }
    Ok(result)
}

struct Phase {
    arm: u64,
    name: String,
    kind: u64,
    wgc: bool,
    device: bool,
    control: bool,
}

fn phases() -> Vec<Phase> {
    let mut result = Vec::new();
    for arm in 0..4 {
        let mut add = |name: String, kind, wgc, device, control| {
            result.push(Phase {
                arm,
                name,
                kind,
                wgc,
                device,
                control,
            });
        };
        for name in ["dd-before-warm", "dd-before-test"] {
            add(name.into(), 0, false, false, true);
        }
        if arm < 2 {
            add("wgc-default".into(), 1, true, false, true);
            add("dd-with-default".into(), 0, true, false, false);
            if arm == 1 {
                add("dd-with-required".into(), 0, true, false, false);
            }
        } else if arm == 2 {
            for name in ["dd-with-device", "dd-with-device-test"] {
                add(name.into(), 0, false, true, false);
            }
        }
        for call in 0..3 {
            add(format!("dd-post-{call}"), 0, false, false, arm == 3);
        }
    }
    result
}

fn summary(text: &str) -> Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("Invalid border summary")?;
        if result.insert(key.into(), scalar(value)?).is_some() {
            return Err("Duplicate border summary field".into());
        }
    }
    let schema = *result.get("schema").ok_or("Missing border schema")?;
    if !matches!(schema, 1 | 2) {
        return Err("Unknown border observation schema".into());
    }
    let mut expected = BTreeMap::from([
        ("schema".into(), schema),
        ("shots".into(), 27),
        ("closed_windows".into(), 4),
        ("dispatcher_completed".into(), 1),
        ("dispatcher_within_budget".into(), 1),
        ("caller_dpi_restored".into(), 1),
        ("positive_controls".into(), 1),
        ("source_bodies_unchanged".into(), 1),
        ("extra_cpu_snapshots".into(), 1),
        ("required_true_setters".into(), 1),
        ("added_false_setters".into(), 0),
        ("original_failure_reclassified".into(), 0),
        ("full_P1_acceptance".into(), 0),
        ("cv_8u".into(), 0),
    ]);
    if schema == 2 {
        let case = *result
            .get("fixture_case")
            .ok_or("Missing matched border fixture case")?;
        if case > 1 {
            return Err("Unknown matched border fixture case".into());
        }
        expected.insert("fixture_case".into(), case);
        expected.insert("fixture_geometry_calls".into(), 12);
        expected.insert("fixture_boundary_observations".into(), 54);
    }
    for field in ["qpc_frequency", "cv_cn_shift", "cv_8uc3"] {
        expected.insert(
            field.into(),
            *result.get(field).ok_or("Missing border native field")?,
        );
    }
    let shift = result["cv_cn_shift"];
    if result != expected
        || result["qpc_frequency"] == 0
        || !(1..=16).contains(&shift)
        || result["cv_8uc3"]
            != 2_u64
                .checked_shl(u32::try_from(shift)?)
                .ok_or("Border OpenCV type overflow")?
    {
        return Err("Border completion/native OpenCV contract differs".into());
    }
    Ok(result)
}

fn properties(text: &str) -> Result<Vec<serde_json::Value>> {
    let rows: Vec<_> = text
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect();
    if rows.len() != 4 || rows.iter().any(|r| r.len() != 9) {
        return Err("Incomplete or extra native border property rows/columns".into());
    }
    let mut result = Vec::new();
    for (arm, row) in rows.iter().enumerate() {
        let session = scalar(row[1])?;
        if scalar(row[0])? != u64::try_from(arm)? {
            return Err("Border property arm differs".into());
        }
        let valid = match arm {
            0 => {
                session != 0
                    && row[2] == "1"
                    && row[3] == "0"
                    && matches!(row[4], "0" | "1")
                    && row[5..] == ["0", "0", "0", "-1"]
            }
            1 => {
                session != 0
                    && row[2] == "2"
                    && row[3] == "0"
                    && matches!(row[4], "0" | "1")
                    && row[5..] == ["1", "0", "0", "1"]
            }
            _ => session == 0 && row[2..] == ["0", "0", "-1", "0", "0", "0", "-1"],
        };
        if !valid {
            return Err(
                "Native border property/session/call-count positive control differs".into(),
            );
        }
        result.push(serde_json::json!({"arm":arm,"session_pointer":session,"getter_calls":scalar(row[2])?,
            "before_value":row[4].parse::<i32>()?,"setter_calls":scalar(row[5])?,"after_value":row[8].parse::<i32>()?,
            "before_hresult":0,"setter_hresult":0,"after_hresult":0}));
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

fn owners_and_drivers(
    windows: &[Vec<u64>],
    owners: &[Vec<u64>],
    drivers: &[Vec<u64>],
) -> Result<()> {
    for arm in 0..4 {
        let window = &windows[arm];
        let owner = &owners[arm];
        let driver = &drivers[arm];
        if window[0] != u64::try_from(arm)?
            || window[1..].contains(&0)
            || window[2] != window[4]
            || window[2] == window[3]
            || window[3] != window[6]
            || window[3] != windows[0][3]
            || window[5] != windows[0][5]
            || window[2..].iter().any(|id| u32::try_from(*id).is_err())
            || owner[0] != window[0]
            || owner[1] == 0
            || owner[1] > owner[2]
            || driver[0] != window[0]
            || driver[1] == 0
            || driver[1] == window[1]
            || driver[2] != window[2]
            || driver[3] != window[5]
            || driver[4] == 0
            || driver[5] <= driver[4]
            || driver[6..] != [0, 1, 1, 1]
        {
            return Err("Border target/thread/owner/frame-driver premise differs".into());
        }
    }
    Ok(())
}

fn signed_numbers(text: &str) -> Result<Vec<Vec<i64>>> {
    text.lines()
        .map(|line| {
            line.split('\t')
                .map(|field| {
                    let value = field.parse::<i64>()?;
                    if value.to_string() != field {
                        return Err("Noncanonical matched fixture geometry".into());
                    }
                    Ok(value)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()
}

fn fixture_geometries(text: &str, case: u64) -> Result<serde_json::Value> {
    let rows = signed_numbers(text)?;
    if rows.len() != 4 || rows.iter().any(|row| row.len() != 63) || case > 1 {
        return Err("Incomplete matched fixture geometry".into());
    }
    let mut result = Vec::new();
    for (arm, row) in rows.iter().enumerate() {
        let before = &row[1..32];
        let after = &row[32..];
        let style = 0x9400_0000_i64 | if case == 1 { 0x00c0_0000 } else { 0 };
        let coordinates_valid = before[..18]
            .iter()
            .chain(&after[..18])
            .all(|value| i32::try_from(*value).is_ok());
        let native_fields_valid = [before, after].into_iter().all(|geometry| {
            [18, 19, 25, 26]
                .into_iter()
                .all(|index| u32::try_from(geometry[index]).is_ok())
                && [21, 22, 23, 24, 27, 28, 29, 30]
                    .into_iter()
                    .all(|index| i32::try_from(geometry[index]).is_ok())
                && geometry[26] > 0
                && geometry[20] >= 0
                && geometry[0] <= geometry[4]
                && geometry[1] <= geometry[5]
                && geometry[2] >= geometry[6]
                && geometry[3] >= geometry[7]
                && geometry[2] > geometry[0]
                && geometry[3] > geometry[1]
        });
        if row[0] != i64::try_from(arm)?
            || !coordinates_valid
            || !native_fields_valid
            || before[..8] != after[..8]
            || after[..8] != rows[0][32..40]
            || before[12..16] != [0, 0, 96, 64]
            || after[12..16] != [0, 0, 96, 64]
            || after[6] - after[4] < 224
            || after[7] - after[5] < 192
            || after[16] != after[4] + 64
            || after[17] != after[5] + 64
            || after[18] != style
            || after[19] & 8 != 8
            || after[20] < before[20]
            || after[26] != before[26]
            || after[27..29] != [1, 0]
            || after[29] != after[10] - after[8]
            || after[30] != after[11] - after[9]
        {
            return Err(
                "Matched fixture style/full-client/work-area/native geometry differs".into(),
            );
        }
        let client = [after[16], after[17], after[16] + 96, after[17] + 64];
        let outer = &after[8..12];
        let insets = [
            client[0] - outer[0],
            client[1] - outer[1],
            outer[2] - client[2],
            outer[3] - client[3],
        ];
        if outer[0] < after[4]
            || outer[1] < after[5]
            || outer[2] > after[6]
            || outer[3] > after[7]
            || (case == 0 && insets != [0; 4])
            || (case == 1 && insets.iter().any(|value| *value <= 0))
        {
            return Err("Matched fixture outer/client separation differs".into());
        }
        result.push(serde_json::json!({"arm":arm,"before":before,"after":after,
            "client_screen":client,"outer_client_insets":insets,"target_inset":0}));
    }
    Ok(
        serde_json::json!({"case":case,"geometry_calls":12,"full_client_width":96,
        "full_client_height":64,"target_inset":0,"observations":result}),
    )
}

fn fixture_boundaries(
    text: &str,
    geometries: &str,
    states: &[Vec<u64>],
) -> Result<serde_json::Value> {
    let rows = signed_numbers(text)?;
    let geometry = signed_numbers(geometries)?;
    if rows.len() != 27
        || rows.iter().any(|row| row.len() != 25)
        || geometry.len() != 4
        || geometry.iter().any(|row| row.len() != 63)
        || states.len() != 27
        || states.iter().any(|row| row.len() != 13)
    {
        return Err("Incomplete matched fixture call-boundary geometry".into());
    }
    let mut result = Vec::new();
    for (index, phase) in phases().iter().enumerate() {
        let row = &rows[index];
        let prepared = &geometry[usize::try_from(phase.arm)?][32..];
        let expected = [
            prepared[18],
            prepared[8],
            prepared[9],
            prepared[10],
            prepared[11],
            prepared[12],
            prepared[13],
            prepared[14],
            prepared[15],
            prepared[16],
            prepared[17],
            prepared[19],
        ];
        let ex_style = u64::try_from(expected[11])?;
        let state = &states[index];
        if row[0] != i64::try_from(index)?
            || row[1..13] != expected
            || row[13..] != expected
            || state[0] != u64::try_from(index)?
            || [1, 7]
                .into_iter()
                .any(|start| state[start] != ex_style || state[start + 1..start + 3] != [255, 0])
        {
            return Err(
                "Actual capture boundary changed matched style/outer/full-client geometry".into(),
            );
        }
        result.push(serde_json::json!({"slot":index,"arm":phase.arm,"before":&row[1..13],"after":&row[13..]}));
    }
    Ok(serde_json::json!({"observations":54,"boundaries":result,"during_call_unobserved":true}))
}

fn inventory(root: &Path, matched: bool) -> Result<()> {
    let mut actual = 0;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 border entry")?;
        let kind = entry.file_type()?;
        if kind.is_file() && name == "border-validation.json" {
            continue;
        }
        if kind.is_file()
            && ([
                "summary.txt",
                "shots.tsv",
                "states.tsv",
                "properties.tsv",
                "owners.tsv",
                "windows.tsv",
                "frame-drivers.tsv",
            ]
            .contains(&name.as_str())
                || matched
                    && ["fixture-geometries.tsv", "fixture-boundaries.tsv"]
                        .contains(&name.as_str()))
        {
            actual += 1;
        } else if kind.is_dir() && (0..27).any(|i| name == format!("shot-{i}")) {
            for item in fs::read_dir(entry.path())? {
                let item = item?;
                if !item.file_type()?.is_file()
                    || !["image.bgr", "layout.tsv"]
                        .iter()
                        .any(|n| item.file_name() == *n)
                {
                    return Err("Unexpected border payload entry/link".into());
                }
                actual += 1;
            }
        } else {
            return Err("Unexpected border root entry/link".into());
        }
    }
    if actual != FILES + 2 * usize::from(matched) {
        return Err("Border observation file inventory differs".into());
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace framepool-border <external observations> [--check-only]"
                    .into(),
            ),
        };
    if !Path::new(path).is_dir() {
        return Err("Border observations must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        inputs.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let contract = summary(std::str::from_utf8(&read("summary.txt")?)?)?;
    let matched = contract["schema"] == 2;
    let files = FILES + 2 * usize::from(matched);
    inventory(&root, matched)?;
    let windows = numbers(std::str::from_utf8(&read("windows.tsv")?)?, 4, 7)?;
    let owners = numbers(std::str::from_utf8(&read("owners.tsv")?)?, 4, 3)?;
    let drivers = numbers(std::str::from_utf8(&read("frame-drivers.tsv")?)?, 4, 10)?;
    owners_and_drivers(&windows, &owners, &drivers)?;
    let properties = properties(std::str::from_utf8(&read("properties.tsv")?)?)?;
    let states = numbers(std::str::from_utf8(&read("states.tsv")?)?, 27, 13)?;
    let fixture = if matched {
        let geometries = String::from_utf8(read("fixture-geometries.tsv")?)?;
        let mut fixture = fixture_geometries(&geometries, contract["fixture_case"])?;
        fixture["call_boundaries"] = fixture_boundaries(
            std::str::from_utf8(&read("fixture-boundaries.tsv")?)?,
            &geometries,
            &states,
        )?;
        Some(fixture)
    } else {
        None
    };
    let shots = String::from_utf8(read("shots.tsv")?)?;
    let mut lines = shots.lines();
    if lines.next() != Some(HEADER) {
        return Err("Border shot header differs".into());
    }
    let expected = expected_pixels();
    let mut last = 0;
    let mut observations = Vec::new();
    for (index, phase) in phases().iter().enumerate() {
        let fields: Vec<_> = lines
            .next()
            .ok_or("Missing border source call")?
            .split('\t')
            .collect();
        if fields.len() != 13 || fields[2] != phase.name {
            return Err("Border call phase/columns differ".into());
        }
        let row = fields
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, v)| scalar(v))
            .collect::<Result<Vec<_>>>()?;
        if row[0] != u64::try_from(index)?
            || row[1] != phase.arm
            || row[2] != phase.kind
            || row[3] != u64::from(phase.wgc)
            || row[4] != u64::from(phase.device)
            || row[5] == 0
            || row[5] < last
            || row[6] < row[5]
            || row[7] > 1
            || row[10] > 1
        {
            return Err("Border call identity/lifetime/clock/scalars differ".into());
        }
        last = row[6];
        let owner = &owners[usize::try_from(phase.arm)?];
        if (phase.name.starts_with("dd-post-") && row[5] < owner[2])
            || (!phase.name.starts_with("dd-post-") && row[6] > owner[1])
        {
            return Err("Border object destruction contradicts call clock".into());
        }
        let state = &states[index];
        if state[0] != row[0]
            || [1, 7].into_iter().any(|start| {
                let s = &state[start..start + 6];
                s[1] > 255 || s[2] > 1 || s[3] > 1 || s[4..] != [96, 64]
            })
        {
            return Err("Border native target geometry/state differs".into());
        }
        let layout = numbers(
            std::str::from_utf8(&read(&format!("shot-{index}/layout.tsv"))?)?,
            1,
            5,
        )?;
        let l = &layout[0];
        let pixels = read(&format!("shot-{index}/image.bgr"))?;
        let empty = l == &[row[7], 0, 0, 0, 0] && pixels.is_empty();
        let image = l == &[row[7], 96, 64, contract["cv_8uc3"], 288] && pixels.len() == 18432;
        if l[0] != row[7] || l[1..3] != row[8..10] || (!empty && !image) || (image && row[7] != 1) {
            return Err("Border native payload span/layout differs".into());
        }
        let fnv = pixels.iter().fold(0xcbf29ce484222325_u64, |v, b| {
            (v ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        let known = pixels == expected;
        if fnv != row[11] || known != (row[10] == 1) || (phase.control && (row[7] != 1 || !known)) {
            return Err("Border pixel digest/known-scene/positive control differs".into());
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
        observations.push(serde_json::json!({"slot":index,"arm":phase.arm,"phase":phase.name,"kind":phase.kind,
            "wgc_object_alive":phase.wgc,"device_only_alive":phase.device,"positive_control":phase.control,
            "engaged":row[7]==1,"known_scene":known,"different_pixels":differences.len(),
            "exact_outer_three_pixel_difference":image && differences==perimeter,"fnv64":fnv,"begin_qpc":row[5],"end_qpc":row[6]}));
    }
    if lines.next().is_some() || inputs.len() != files {
        return Err("Extra border call or incomplete inputs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let mut report = serde_json::json!({"layout_version":1,"actual_source_calls":27,"owned_windows_closed":4,
        "positive_control_calls":13,"registered_files":files,"native_cv_contract":contract,"windows":windows,
        "object_boundaries":owners,"window_states":states,"frame_drivers":drivers,"properties":properties,
        "observations":observations,"inputs":inputs,"producer_proved":false,"capture_acceptance":false,"full_P1_accepted":false,
        "scope":"Original-only four-arm property/device diagnostic with extra CPU snapshots and a same-output, nonoverlapping owned marker. Only the added true setter is a treatment; original false request is unchanged. A false getter does not prove an invisible system indicator. Fixed post-destruction calls are observations, not retries or native session/callback quiescence. No Rust/manager parity, producer proof, historical failure reclassification or P1 acceptance."});
    if let Some(fixture) = fixture {
        report["layout_version"] = 2.into();
        report["matched_fixture"] = fixture;
    }
    let output = root.join("border-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved border observations differ".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "source border records calls=27 windows_closed=4 positive_controls=13 files={files} producer_proved=0 capture_acceptance=0 full_P1_acceptance=0 passed"
    );
    if matched {
        println!(
            "matched border records case={} geometry_calls=12 boundary_observations=54 full_client=96x64 target_inset=0 producer_proved=0 passed",
            contract["fixture_case"]
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched_rows(case: u64) -> Vec<Vec<i64>> {
        let before = vec![
            0,
            0,
            1920,
            1080,
            0,
            0,
            1920,
            1040,
            128,
            128,
            224,
            192,
            0,
            0,
            96,
            64,
            128,
            128,
            0x9400_0000,
            8,
            0,
            0,
            0,
            0,
            0,
            0,
            96,
            0,
            0,
            0,
            0,
        ];
        let mut after = before.clone();
        after[16..18].copy_from_slice(&[64, 64]);
        after[20] = 1;
        after[25] = 0x34;
        after[27] = 1;
        let outer = if case == 0 {
            [64, 64, 160, 128]
        } else {
            after[18] |= 0x00c0_0000;
            [61, 38, 163, 131]
        };
        after[8..12].copy_from_slice(&outer);
        after[21..25].copy_from_slice(&[
            outer[0],
            outer[1],
            outer[2] - outer[0],
            outer[3] - outer[1],
        ]);
        after[29] = outer[2] - outer[0];
        after[30] = outer[3] - outer[1];
        (0..4)
            .map(|arm| [vec![arm], before.clone(), after.clone()].concat())
            .collect()
    }

    fn geometry_text(rows: &[Vec<i64>]) -> String {
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\t")
                    + "\n"
            })
            .collect()
    }

    #[test]
    fn rejects_changed_or_missing_full_client_call_boundaries() {
        let geometry = matched_rows(1);
        let rows = phases()
            .iter()
            .enumerate()
            .map(|(slot, phase)| {
                let g = &geometry[usize::try_from(phase.arm).unwrap()][32..];
                let boundary = vec![
                    g[18], g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15], g[16], g[17],
                    g[19],
                ];
                [
                    vec![i64::try_from(slot).unwrap()],
                    boundary.clone(),
                    boundary,
                ]
                .concat()
            })
            .collect::<Vec<_>>();
        let states = (0..27)
            .map(|slot| vec![slot, 8, 255, 0, 1, 96, 64, 8, 255, 0, 1, 96, 64])
            .collect::<Vec<_>>();
        fixture_boundaries(&geometry_text(&rows), &geometry_text(&geometry), &states).unwrap();
        for column in [0, 1, 5, 8, 10, 12, 13, 17, 20, 22, 24] {
            let mut invalid = rows.clone();
            invalid[10][column] += 1;
            assert!(
                fixture_boundaries(&geometry_text(&invalid), &geometry_text(&geometry), &states)
                    .is_err()
            );
        }
        assert!(
            fixture_boundaries(
                &geometry_text(&rows[..26]),
                &geometry_text(&geometry),
                &states
            )
            .is_err()
        );
        let mut invalid = states.clone();
        invalid[10][1] = 0;
        assert!(
            fixture_boundaries(&geometry_text(&rows), &geometry_text(&geometry), &invalid).is_err()
        );
    }

    #[test]
    fn rejects_changed_client_style_native_width_and_invented_nonclient_geometry() {
        for case in 0..=1 {
            fixture_geometries(&geometry_text(&matched_rows(case)), case).unwrap();
        }
        for (field, value) in [
            (0, 99),
            (32 + 14, 97),
            (32 + 16, 65),
            (32 + 18, 0x9400_0000),
            (32 + 19, 0),
            (32 + 26, 1_i64 << 32),
            (32 + 21, 1_i64 << 32),
            (1 + 27, 1_i64 << 32),
            (1 + 30, 1_i64 << 32),
            (32 + 27, 0),
            (32 + 28, -1),
            (32 + 29, 96),
            (32 + 8, 64),
            (32, i64::MIN),
        ] {
            let mut invalid = matched_rows(1);
            invalid[0][field] = value;
            assert!(fixture_geometries(&geometry_text(&invalid), 1).is_err());
        }
        let valid = geometry_text(&matched_rows(0));
        assert!(fixture_geometries(&valid, 1).is_err());
        assert!(fixture_geometries(&valid, 2).is_err());
        assert!(fixture_geometries(&valid.replacen("0\t", "-0\t", 1), 0).is_err());
        assert!(fixture_geometries(&geometry_text(&matched_rows(0)[..3]), 0).is_err());
    }

    #[test]
    fn retains_negative_desktop_coordinates_without_changing_full_client_scope() {
        let mut rows = matched_rows(1);
        for row in &mut rows {
            for start in [1, 32] {
                for index in [0, 2, 4, 6, 8, 10, 16] {
                    row[start + index] -= 2560;
                }
                for index in [1, 3, 5, 7, 9, 11, 17] {
                    row[start + index] -= 1080;
                }
            }
        }
        fixture_geometries(&geometry_text(&rows), 1).unwrap();
    }

    #[test]
    fn rejects_missing_session_getter_setter_and_invented_inactive_calls() {
        let valid = "0\t100\t1\t0\t0\t0\t0\t0\t-1\n1\t200\t2\t0\t0\t1\t0\t0\t1\n2\t0\t0\t0\t-1\t0\t0\t0\t-1\n3\t0\t0\t0\t-1\t0\t0\t0\t-1\n";
        properties(valid).unwrap();
        for (old, new) in [
            ("100", "0"),
            ("200", "0"),
            ("200\t2", "200\t1"),
            ("\t1\t0\t0\t1\n", "\t0\t0\t0\t1\n"),
            ("\t1\t0\t0\t1\n", "\t1\t-2147024891\t0\t1\n"),
            ("\t0\t0\t1\n", "\t0\t0\t0\n"),
            ("2\t0\t0", "2\t1\t0"),
            ("100", "0100"),
        ] {
            assert!(properties(&valid.replace(old, new)).is_err(), "{new}");
        }
        assert!(properties(&format!("{valid}{valid}")).is_err());
        assert_eq!(phases().len(), 27);
        assert_eq!(phases().iter().filter(|p| p.control).count(), 13);
    }

    #[test]
    fn rejects_foreign_threads_and_stalled_or_unclosed_driver_owners() {
        let windows: Vec<_> = (0..4).map(|a| vec![a, 100 + a, 7, 9, 7, 12, 9]).collect();
        let owners: Vec<_> = (0..4).map(|a| vec![a, 1000 + a, 1001 + a]).collect();
        let drivers: Vec<_> = (0..4)
            .map(|a| vec![a, 200 + a, 7, 12, 1, 2, 0, 1, 1, 1])
            .collect();
        owners_and_drivers(&windows, &owners, &drivers).unwrap();
        for (field, value) in [
            (0, 1),
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
            let mut invalid = drivers.clone();
            invalid[0][field] = value;
            assert!(owners_and_drivers(&windows, &owners, &invalid).is_err());
        }
        for (field, value) in [(1, 0), (2, 8), (3, 7), (5, u64::MAX), (6, 10)] {
            let mut invalid = windows.clone();
            invalid[0][field] = value;
            assert!(owners_and_drivers(&invalid, &owners, &drivers).is_err());
        }
    }
}
