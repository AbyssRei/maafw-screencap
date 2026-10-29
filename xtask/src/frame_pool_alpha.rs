// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual alpha/content diagnostic records; never a capture acceptance gate.
use crate::{Result, records};
use std::{fs, path::Path};

const HEADER: &str = "arm\tphase\talpha\tframe_image\tframe_known\tframe_zero\tprint_image\tprint_known\tpaint_before\tpaint_after\tframe_prior\telapsed_us";

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [kind, path] if kind == "frame-pool" => (path, false),
        [kind, path, flag] if kind == "frame-pool" && flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-alpha frame-pool <external directory> [--check-only]".into(),
            );
        }
    };
    let directory = records::external_root(Path::new(path))?;
    let text = fs::read_to_string(directory.join("observations.tsv"))?;
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("Unsupported alpha diagnostic header".into());
    }
    let lines: Vec<_> = lines.collect();
    if lines.len() != 10 {
        return Err("Alpha diagnostic requires both complete five-phase sequences".into());
    }
    let mut artifacts = vec![records::artifact(
        &directory,
        &directory.join("observations.tsv"),
    )?];
    let mut rows = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let row = row(index, line)?;
        let alternative = row[2] == 0;
        let name = format!(
            "{}-{}",
            if row[0] == 0 { "fresh" } else { "persistent" },
            row[1]
        );
        for (file, palette) in [
            (
                "source.bgr",
                if row[4] == 1 {
                    Some(alternative)
                } else if row[10] == 1 {
                    Some(!alternative)
                } else {
                    None
                },
            ),
            ("print/source.bgr", Some(alternative)),
        ] {
            let path = directory.join(&name).join(file);
            if fs::read(&path)? != pattern(palette) {
                return Err(format!("Alpha diagnostic bytes differ: {name}/{file}").into());
            }
            artifacts.push(records::artifact(&directory, &path)?);
        }
        let path = directory.join(&name).join("states.tsv");
        states(&fs::read_to_string(&path)?, row[2])?;
        artifacts.push(records::artifact(&directory, &path)?);
        rows.push(row);
    }
    let summary = directory.join("summary.tsv");
    if fs::read_to_string(&summary)?.lines().ne([
        "held_frames\t10",
        "closed_windows\t2",
        "dispatcher_completed\t1",
        "positive_controls\t1",
    ]) {
        return Err("Alpha diagnostic owner/shutdown/positive-control summary differs".into());
    }
    artifacts.push(records::artifact(&directory, &summary)?);
    let report = serde_json::json!({"layout_version":1,"rows":rows,"artifacts":artifacts,
        "scope":"Source-only alpha/content observation: ten FramePool results and ten independent PrintWindow content witnesses, two unchanged native palettes, fresh/persistent sessions, actual pre/post capture window state and shutdown/held-owner controls. Alpha255/current PrintWindow pixels are strict; alpha0 full-black or prior-scene outputs are diagnostic classifications only. Native cache branch not intercepted. Does not accept original failed pseudo colors, Rust parity, full transparency/format/cleanup, backend or P1."});
    let output = directory.join("alpha-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved alpha diagnostic report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool alpha diagnostic phases=10 payloads=20 bytes_verified=1 positive_controls=1 capture_acceptance=0 passed"
    );
    Ok(())
}

fn numbers(line: &str) -> Result<Vec<u64>> {
    line.split('\t')
        .map(|value| {
            let number: u64 = value.parse()?;
            if number.to_string() != value {
                return Err("Noncanonical alpha numeric field".into());
            }
            Ok(number)
        })
        .collect()
}

fn row(index: usize, line: &str) -> Result<[u64; 12]> {
    let values: [u64; 12] = numbers(line)?
        .try_into()
        .map_err(|_| "Wrong alpha row width")?;
    let phase = index % 5;
    if index >= 10
        || values[0] != (index / 5) as u64
        || values[1] != phase as u64
        || values[2] != if phase % 2 == 0 { 255 } else { 0 }
        || [3, 4, 5, 6, 7, 10].iter().any(|i| values[*i] > 1)
        || values[3] != 1
        || values[6] != 1
        || values[7] != 1
        || values[4] + values[5] + values[10] != 1
        || (values[2] == 255 && values[4] != 1)
        || values[8].checked_add(1) != Some(values[9])
    {
        return Err("Alpha row/positive control/classification differs".into());
    }
    Ok(values)
}

fn states(text: &str, alpha: u64) -> Result<()> {
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 2 {
        return Err("Missing actual pre/post capture alpha states".into());
    }
    let mut values = Vec::new();
    for (line, label) in lines.iter().zip(["before_capture", "after_capture"]) {
        let (actual, fields) = line.split_once('\t').ok_or("Missing alpha state label")?;
        let fields = numbers(fields)?;
        if actual != label
            || fields.len() != 6
            || fields[0] & 0x80020 != 0x80020
            || fields[1] != alpha
            || fields[2] != 0
            || fields[3] > 1
            || fields[4..] != [96, 64]
        {
            return Err("Alpha state/geometry differs".into());
        }
        values.push(fields);
    }
    if values[0][0] != values[1][0] {
        return Err("Source changed alpha fixture style".into());
    }
    Ok(())
}

fn pattern(palette: Option<bool>) -> Vec<u8> {
    let mut bytes = vec![0; 96 * 64 * 3];
    let Some(alternative) = palette else {
        return bytes;
    };
    let colors = if alternative {
        [[0xe3, 0x42, 0x19], [0x57, 0x8b, 0xc1], [0x9d, 0x23, 0xf5]]
    } else {
        [[0x1f, 0x53, 0xa7], [0xb9, 0x4d, 0x27], [0x3b, 0xe2, 0x81]]
    };
    for y in 0..64 {
        for x in 0..96 {
            let region = if x < 32 {
                0
            } else if y < 32 {
                1
            } else {
                2
            };
            bytes[(y * 96 + x) * 3..(y * 96 + x + 1) * 3].copy_from_slice(&colors[region]);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_scene_claims_missing_redraw_and_fabricated_state_labels() {
        assert!(row(0, "0\t0\t255\t1\t0\t1\t1\t1\t1\t2\t0\t10").is_err());
        assert!(row(1, "0\t1\t0\t1\t0\t0\t1\t1\t3\t4\t1\t2000").is_ok());
        assert!(row(1, "0\t1\t0\t1\t0\t0\t1\t1\t3\t3\t1\t2000").is_err());
        assert!(row(1, "0\t1\t0\t1\t1\t0\t1\t1\t3\t4\t1\t2000").is_err());
        assert!(
            states(
                "before_capture\t524328\t0\t0\t0\t96\t64\nafter_capture\t524328\t0\t0\t0\t96\t64\n",
                0
            )
            .is_ok()
        );
        assert!(
            states(
                "closed\t524328\t0\t0\t0\t96\t64\nafter_capture\t524328\t0\t0\t0\t96\t64\n",
                0
            )
            .is_err()
        );
        assert!(numbers("01\t0").is_err());
        assert_ne!(pattern(Some(true)), pattern(Some(false)));
        assert!(pattern(None).iter().all(|byte| *byte == 0));
    }
}
