// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual native sample records, not a desktop-capture or whole-P1 gate.
use crate::{Result, macos_sample_cases as cases, records};
use std::{fs, path::Path};
fn numbers(text: &str, columns: usize) -> Result<Vec<Vec<u64>>> {
    text.lines()
        .map(|line| {
            let values = line
                .split('\t')
                .map(|v| -> Result<u64> {
                    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                        return Err("invalid native sample scalar".into());
                    }
                    Ok(v.parse()?)
                })
                .collect::<Result<Vec<_>>>()?;
            if values.len() != columns {
                return Err("native sample column count differs".into());
            }
            Ok(values)
        })
        .collect()
}
fn row(
    value: &[u64],
    cycle: usize,
    case: usize,
    caller: u64,
) -> Result<(usize, usize, usize, usize)> {
    let (width, height, alignment) = cases::CASES[case];
    let row = width.checked_mul(4).ok_or("native sample row overflow")?;
    let packed = row
        .checked_mul(height)
        .ok_or("native sample CPU span overflow")?;
    let stride = usize::try_from(value[4])?;
    let span = (height - 1)
        .checked_mul(stride)
        .and_then(|last| last.checked_add(row))
        .ok_or("native sample readable span overflow")?;
    let size = usize::try_from(value[6])?;
    if value[0] != u64::try_from(cycle)?
        || value[1] != u64::try_from(case)?
        || value[2] != u64::try_from(width)?
        || value[3] != u64::try_from(height)?
        || stride <= row
        || stride % alignment != 0
        || value[5] != u64::try_from(span)?
        || span > size
        || size > isize::MAX as usize
        || value[7] != u64::from(u32::from_be_bytes(*b"BGRA"))
        || caller == 0
        || value[8..11] != [caller; 3]
        || value[11..14] != [1; 3]
        || value[14] != u64::try_from(packed)?
        || value[15..18] != [1, 0, 1]
    {
        return Err("native sample layout, channel or lock/conversion evidence differs".into());
    }
    Ok((width, height, stride, size))
}
fn native_bytes(case: usize, cycle: usize, stride: usize, size: usize, input: &[u8]) -> Result<()> {
    let (width, height, _) = cases::CASES[case];
    let row = width.checked_mul(4).ok_or("native row overflow")?;
    if input.len() != size {
        return Err("native sample saved span differs".into());
    }
    let expected = cases::raw(case, cycle);
    for y in 0..height {
        let start = y.checked_mul(stride).ok_or("native row offset overflow")?;
        let end = start.checked_add(row).ok_or("native row end overflow")?;
        let next = if y + 1 == height {
            size
        } else {
            (y + 1)
                .checked_mul(stride)
                .ok_or("native padding offset overflow")?
        };
        if end > next
            || next > size
            || input[start..end] != expected[y * row..(y + 1) * row]
            || input[end..next].iter().any(|b| *b != 0xa5)
        {
            return Err("native sample target pattern or initialized padding differs".into());
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path,check_only)=match args {
        [path]=>(path,false),[path,flag] if flag=="--check-only"=>(path,true),
        _=>return Err("expected upstream-trace macos-sample-matrix <actual external directory> [--check-only]".into())
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let counters = numbers(std::str::from_utf8(&read("counters.tsv")?)?, 13)?;
    if counters.len() != 1
        || counters[0][..10] != [80, 80, 80, 80, 80, 80, 160, 160, 0, 0]
        || counters[0][10] == 0
        || counters[0][11] == 0
        || counters[0][10] == counters[0][11]
        || counters[0][12] != 0
    {
        return Err("native sample owners/mutations/cleanup or CPU reader evidence differs".into());
    }
    let caller = counters[0][10];
    let reader = counters[0][11];
    let rows = numbers(std::str::from_utf8(&read("samples.tsv")?)?, 18)?;
    let releases = numbers(std::str::from_utf8(&read("releases.tsv")?)?, 3)?;
    if rows.len() != 80 || releases.len() != 80 || read("module.tsv")? != b"1\t0\n" {
        return Err("incomplete native sample/release/module records".into());
    }
    for cycle in 0..cases::CYCLES {
        for case in 0..cases::CASES.len() {
            let index = cycle * cases::CASES.len() + case;
            let (_, _, stride, size) = row(&rows[index], cycle, case, caller)?;
            if releases[index] != [u64::try_from(cycle)?, u64::try_from(case)?, reader] {
                return Err("source CPU image did not release on joined reader".into());
            }
            let prefix = format!("case-{case}-{cycle}");
            native_bytes(
                case,
                cycle,
                stride,
                size,
                &read(&format!("{prefix}.native.raw"))?,
            )?;
            let mutated = read(&format!("{prefix}.mutated.raw"))?;
            if mutated.len() != size || mutated.iter().any(|b| *b != 0) {
                return Err("native sample was not actually overwritten".into());
            }
            let raw = cases::raw(case, cycle);
            let bgr = cases::bgr(&raw);
            for (name, expected) in [
                ("rust.raw", &raw),
                ("source.bgr", &bgr),
                ("rust.bgr", &bgr),
                ("held.raw", &raw),
                ("held.source.bgr", &bgr),
                ("held.rust.bgr", &bgr),
            ] {
                if read(&format!("{prefix}.{name}"))? != *expected {
                    return Err("native source/Rust conversion or held CPU bytes differ".into());
                }
            }
        }
    }
    let report = serde_json::json!({"layout_version":1,"samples":80,"cases":4,"cycles":20,"payloads":640,
        "desktop_capture":false,"full_p1_acceptance":false,
        "native_caller_thread":caller,"cpu_reader_thread":reader,"artifacts":artifacts,
        "scope":"Native-owned CVPixelBuffer and ready CMSampleBuffer fixture, four odd-width/aligned row layouts and actual returned stride/span, eighty unmodified original image_from_sample_buffer conversions and eighty private Rust with_sample conversions. Initialized native bytes/padding, complete packed BGRA/BGR, real native overwrite, fixture CF-reference closure, CPU owners retained after native inputs and released on a joined thread, Rust lock/copy/conversion and native counters are checked. Native sample-conversion evidence only; not desktop capture, source native lock-failure/empty-image/error paths, all format/color evidence, full backend or P1 acceptance."});
    let output = root.join("sample-matrix-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("native sample report differs from actual inputs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS sample_matrix samples=80 native_inputs=80 payloads=640 padded_rows=80 source_conversions=80 rust_conversions=80 native_overwrite=80 retained_cpu=240 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_short_stride_wrong_owner_and_unlock_relabeling() {
        let good = vec![
            0,
            0,
            7,
            5,
            64,
            284,
            320,
            u64::from(u32::from_be_bytes(*b"BGRA")),
            7,
            7,
            7,
            1,
            1,
            1,
            140,
            1,
            0,
            1,
        ];
        row(&good, 0, 0, 7).unwrap();
        for (index, bad) in [
            (4, 28),
            (5, 283),
            (6, 283),
            (9, 8),
            (12, 0),
            (14, 139),
            (16, 1),
            (17, 0),
        ] {
            let mut values = good.clone();
            values[index] = bad;
            assert!(row(&values, 0, 0, 7).is_err());
        }
        assert!(numbers("0\t1\n", 18).is_err());
        assert!(numbers("-1\n", 1).is_err());
    }
    #[test]
    fn rejects_changed_native_padding_and_target_bytes() {
        let packed = cases::raw(0, 0);
        let mut input = vec![0xa5; 320];
        for y in 0..5 {
            input[y * 64..y * 64 + 28].copy_from_slice(&packed[y * 28..(y + 1) * 28]);
        }
        native_bytes(0, 0, 64, 320, &input).unwrap();
        input[28] = 0;
        assert!(native_bytes(0, 0, 64, 320, &input).is_err());
        input[28] = 0xa5;
        input[0] ^= 1;
        assert!(native_bytes(0, 0, 64, 320, &input).is_err());
    }
}
