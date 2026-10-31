// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent raw-pattern and BGR arithmetic for real seven-format streams.
use crate::{Result, records};
use std::{fs, path::Path};

const FORMATS: [(&str, usize); 7] = [
    ("RGB", 3),
    ("RGBA", 4),
    ("RGBx", 4),
    ("BGR", 3),
    ("BGRA", 4),
    ("BGRx", 4),
    ("YUY2", 2),
];

fn expected(name: &str, channels: usize, phase: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let stride = 28 * channels + 12;
    let mut padded = vec![0xa5; stride * 24];
    let mut raw = Vec::new();
    let mut bgr = Vec::new();
    for y in 0..24 {
        for x in 0..28 {
            for c in 0..channels {
                let value = ((y * 29 + x * 17 + c * 53 + phase * 71) & 255) as u8;
                padded[y * stride + x * channels + c] = value;
                raw.push(value);
            }
        }
    }
    for y in 0..24 {
        for x in 0..28 {
            let p = y * stride + x * channels;
            if name == "YUY2" {
                // OpenCV's BT.601 limited-range YUY2 integer convention:
                // Y0 U Y1 V, clamp Y-16, round at 20 fractional bits, saturate.
                // Independent arithmetic; no native conversion is called here.
                let pair = y * stride + (x / 2) * 4;
                let yy = (i32::from(padded[p]) - 16).max(0) * 1_220_542;
                let u = i32::from(padded[pair + 1]) - 128;
                let v = i32::from(padded[pair + 3]) - 128;
                for delta in [2_116_026 * u, -409_993 * u - 852_492 * v, 1_673_527 * v] {
                    bgr.push(((yy + delta + (1 << 19)) >> 20).clamp(0, 255) as u8);
                }
            } else if name.starts_with("RGB") {
                bgr.extend_from_slice(&[padded[p + 2], padded[p + 1], padded[p]]);
            } else {
                bgr.extend_from_slice(&padded[p..p + 3]);
            }
        }
    }
    (padded, raw, bgr)
}

fn phases(text: &str, both: bool) -> Result<Vec<usize>> {
    let mut lines = text.lines();
    if lines.next() != Some("cycle\tcall\tphase") {
        return Err("Format stream header differs".into());
    }
    let mut phases = Vec::new();
    for index in 0..60 {
        let fields = lines
            .next()
            .ok_or("Missing format stream row")?
            .split('\t')
            .collect::<Vec<_>>();
        if fields.len() != 3
            || fields[0] != (index / 3).to_string()
            || fields[1] != (index % 3).to_string()
            || !["0", "1"].contains(&fields[2])
        {
            return Err("Format stream row order/phase differs".into());
        }
        phases.push(usize::from(fields[2] == "1"));
    }
    if lines.next().is_some() || (both && (!phases.contains(&0) || !phases.contains(&1))) {
        return Err("Format stream count or two-phase evidence incomplete".into());
    }
    Ok(phases)
}

fn validate_held(values: &[usize], held: &[usize], shared: bool) -> Result<()> {
    if values.len() != 60 || held.len() != 60 {
        return Err("Incomplete held phase table".into());
    }
    for (index, phase) in held.iter().copied().enumerate() {
        if phase != values[if shared { index / 3 * 3 + 2 } else { index }] {
            return Err("Original held cache/conversion phases differ".into());
        }
    }
    if shared && values == held {
        return Err("Original BGR cache overwrite witness absent".into());
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let [root, rest @ ..] = args else {
        return Err("usage: upstream-trace pipewire-format-stream ROOT [--check-only]".into());
    };
    if !rest.is_empty() && rest != ["--check-only"] {
        return Err("Invalid format stream checker arguments".into());
    }
    let root = Path::new(root);
    let mut inputs = Vec::new();
    for (name, channels) in FORMATS {
        let directory = root.join(name);
        let oracle = [expected(name, channels, 0), expected(name, channels, 1)];
        let mut check = |path: &Path, bytes: &[u8]| -> Result<()> {
            inputs.push(records::artifact(root, path)?);
            if fs::read(path)? != bytes {
                return Err(format!("Format stream bytes differ: {}", path.display()).into());
            }
            Ok(())
        };
        for (phase, (padded, _, bgr)) in oracle.iter().enumerate() {
            check(
                &directory.join(format!("source/oracle/phase-{phase}.raw")),
                padded,
            )?;
            check(
                &directory.join(format!("source/oracle/phase-{phase}.bgr")),
                bgr,
            )?;
        }
        // End the closure borrow before adding the independently parsed tables.
        for side in ["source", "rust"] {
            let table = directory.join(side).join("calls.tsv");
            let values = phases(&fs::read_to_string(&table)?, true)?;
            inputs.push(records::artifact(root, &table)?);
            let held_values = if side == "source" {
                let table = directory.join(side).join("held.tsv");
                inputs.push(records::artifact(root, &table)?);
                let held = phases(&fs::read_to_string(table)?, false)?;
                validate_held(&values, &held, name == "BGR")?;
                held
            } else {
                values.clone()
            };
            for (index, phase) in values.iter().copied().enumerate() {
                for held in ["", "-held"] {
                    let phase = if held.is_empty() {
                        phase
                    } else {
                        held_values[index]
                    };
                    for extension in if side == "rust" {
                        &["raw", "bgr"][..]
                    } else {
                        &["bgr"][..]
                    } {
                        let path = directory.join(side).join(format!(
                            "cycle-{}-call-{}{held}.{extension}",
                            index / 3,
                            index % 3
                        ));
                        inputs.push(records::artifact(root, &path)?);
                        let expected = if *extension == "raw" {
                            &oracle[phase].1
                        } else {
                            &oracle[phase].2
                        };
                        if fs::read(&path)? != *expected {
                            return Err(format!(
                                "Captured/held {side} {name} {index} {extension} bytes differ"
                            )
                            .into());
                        }
                    }
                }
            }
            if side == "source" {
                for (file, index) in [("first", 0), ("last", 59)] {
                    let path = directory
                        .join(side)
                        .join(format!("{file}-after-shutdown.bgr"));
                    inputs.push(records::artifact(root, &path)?);
                    if fs::read(path)? != oracle[held_values[index]].2 {
                        return Err("Final source CPU owner differs".into());
                    }
                }
            }
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"formats":7,"phases_per_format":2,"cycles_per_side":140,"captures_per_side":420,
        "source_bgr_pixel_files":854,"rust_raw_bgr_pixel_files":1680,"oracle_files":28,"phase_tables":21,"source_bgr_shared_cache":true,"inputs":inputs,
        "scope":"Real owned producer, native negotiation, original captures and Rust raw rows/conversion/retained CPU samples. Raw fixture arithmetic and BT.601 YUY2 BGR are independently recomputed here; source expected images use its actual format table/OpenCV, with conversion oracle scope distinct from live capture. Original BGR heads alias the same cache and change to each lifecycle's final phase; Rust retained raw/BGR owners stay unchanged through its existing copy-on-write safety adaptation. Native commands separately require pointer aliases, lifecycle, queue, resource, conversion/reuse and shutdown checks. No Portal/GPU, reconfiguration, full failure matrix or complete P1 acceptance."});
    let output = root.join("format-stream-validation.json");
    if rest.is_empty() {
        records::create_json(&output, &report)?;
    } else if output.exists() && records::read::<serde_json::Value>(&output)? != report {
        return Err("Saved format stream validation differs".into());
    }
    println!(
        "PipeWire real format stream records formats=7 cycles=280 captures=840 capture_held_files=2534 oracle_files=28 phase_tables=21 source_BGR_cache_alias=1 verified"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_duplicate_out_of_order_and_single_phase_captures() {
        let good = format!(
            "cycle\tcall\tphase\n{}",
            (0..60)
                .map(|i| format!("{}\t{}\t{}\n", i / 3, i % 3, i % 2))
                .collect::<String>()
        );
        assert_eq!(phases(&good, true).unwrap().len(), 60);
        for bad in [
            good.replace("0\t1\t1", "0\t0\t1"),
            good.replace("0\t1\t1", "0\t1\t2"),
            good.replace("0\t0\t0\n", ""),
            good.replace("0\t0\t0", "+0\t0\t0"),
            format!("{good}0\t0\t0\n"),
            good.lines().take(60).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(phases(&bad, true).is_err());
        }
        let one_phase = format!(
            "cycle\tcall\tphase\n{}",
            (0..60)
                .map(|i| format!("{}\t{}\t0\n", i / 3, i % 3))
                .collect::<String>()
        );
        assert!(phases(&one_phase, true).is_err());
        assert!(phases(&one_phase, false).is_ok());
    }
    #[test]
    fn distinguishes_original_bgr_cache_overwrite_from_owning_conversion() {
        let calls = (0..60).map(|i| i % 2).collect::<Vec<_>>();
        let shared = (0..60).map(|i| calls[i / 3 * 3 + 2]).collect::<Vec<_>>();
        assert!(validate_held(&calls, &shared, true).is_ok());
        assert!(validate_held(&calls, &calls, false).is_ok());
        assert!(validate_held(&calls, &calls, true).is_err());
        assert!(validate_held(&calls, &shared, false).is_err());
        assert!(validate_held(&calls[..59], &shared, true).is_err());
    }
}
