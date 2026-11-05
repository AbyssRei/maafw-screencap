// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Full target GDI observations around unchanged original capture calls.
use crate::{Result, framepool_border, records};
use std::{collections::BTreeMap, fs, path::Path};

fn scalar(s: &str) -> Result<i64> {
    let n = s.parse::<i64>()?;
    if n.to_string() != s {
        return Err("Noncanonical screen witness number".into());
    }
    Ok(n)
}

fn table(text: &str, rows: usize, columns: usize) -> Result<Vec<Vec<i64>>> {
    let values = text
        .lines()
        .map(|line| line.split('\t').map(scalar).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    if values.len() != rows || values.iter().any(|r| r.len() != columns) {
        return Err("Screen witness row/column inventory differs".into());
    }
    Ok(values)
}

fn contract(text: &str, expected: &[(&str, i64)]) -> Result<()> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("Screen witness field lacks value")?;
        if fields.insert(key, scalar(value)?).is_some() {
            return Err("Duplicate screen witness field".into());
        }
    }
    if fields != expected.iter().copied().collect::<BTreeMap<_, _>>() {
        return Err("Screen witness completion contract differs".into());
    }
    Ok(())
}

fn geometry(values: &[i64]) -> Result<()> {
    if values.len() != 12 || values[5..9] != [0, 0, 96, 64] {
        return Err("Screen witness full target geometry differs".into());
    }
    u32::try_from(values[0])?;
    u32::try_from(values[11])?;
    for &v in &values[1..11] {
        i32::try_from(v)?;
    }
    i32::try_from(
        values[9]
            .checked_add(95)
            .ok_or("Screen witness x overflow")?,
    )?;
    i32::try_from(
        values[10]
            .checked_add(63)
            .ok_or("Screen witness y overflow")?,
    )?;
    Ok(())
}

fn expected_pixels() -> Vec<u8> {
    (0..64)
        .flat_map(|y| {
            (0..96).flat_map(move |x| {
                if x < 32 {
                    [31, 83, 167]
                } else if y < 32 {
                    [185, 77, 39]
                } else {
                    [59, 226, 129]
                }
            })
        })
        .collect()
}

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected upstream-trace framepool-screen-witness <external observations> [--check-only]".into()),
    };
    if !Path::new(path).is_dir() {
        return Err("Screen witness records must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "capture" || name == "screen" {
            if !entry.file_type()?.is_dir() {
                return Err("Screen witness requires ordinary record directories".into());
            }
        } else if name != "screen-witness-validation.json" || !entry.file_type()?.is_file() {
            return Err("Unexpected screen witness root entry/link".into());
        }
    }
    framepool_border::run(&[
        root.join("capture").to_string_lossy().into(),
        "--check-only".into(),
    ])?;
    let mut inputs = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let p = root.join(name);
        if !p.symlink_metadata()?.file_type().is_file() {
            return Err("Screen witness input must be an ordinary file".into());
        }
        inputs.push(records::artifact(&root, &p)?);
        Ok(fs::read(p)?)
    };
    contract(
        std::str::from_utf8(&read("screen/summary.txt")?)?,
        &[
            ("schema", 2),
            ("reads", 54),
            ("width", 96),
            ("height", 64),
            ("row_bytes", 288),
            ("pixels_per_read", 6144),
            ("bitblt_calls", 54),
            ("getdibits_calls", 54),
            ("getdibits_rows", 3456),
            ("bitmap_restored", 54),
            ("bitmap_deleted", 54),
            ("memory_dc_deleted", 54),
            ("dc_acquired", 54),
            ("dc_released", 54),
            ("source_calls", 27),
            ("source_bodies_unchanged", 1),
            ("added_false_setters", 0),
            ("target_inset", 0),
            ("atomic_screen_snapshot", 0),
            ("producer_proved", 0),
            ("original_failure_reclassified", 0),
            ("full_P1_acceptance", 0),
        ],
    )?;
    contract(
        std::str::from_utf8(&read("screen/control-summary.txt")?)?,
        &[
            ("schema", 2),
            ("width", 3),
            ("height", 2),
            ("getpixel_calls", 7),
            ("valid_pixels", 6),
            ("clipped_rejected", 1),
            ("bitblt_calls", 1),
            ("getdibits_rows", 2),
            ("bitmap_restored", 2),
            ("bitmap_deleted", 2),
            ("memory_dc_deleted", 2),
            ("screen_dc_released", 1),
        ],
    )?;
    let control = table(
        std::str::from_utf8(&read("screen/pixel-control.tsv")?)?,
        6,
        8,
    )?;
    for (i, (row, color)) in control
        .iter()
        .zip([
            [31, 83, 167],
            [185, 77, 39],
            [59, 226, 129],
            [0, 0, 0],
            [255, 255, 255],
            [227, 66, 25],
        ])
        .enumerate()
    {
        let rgb = color[2] | (color[1] << 8) | (color[0] << 16);
        if row
            != &[
                i64::try_from(i)?,
                color[0],
                color[1],
                color[2],
                rgb,
                color[0],
                color[1],
                color[2],
            ]
        {
            return Err("Actual GetPixel/BitBlt/GetDIBits channel/control differs".into());
        }
    }
    let rows = table(
        std::str::from_utf8(&read("screen/observations.tsv")?)?,
        54,
        38,
    )?;
    let boundaries = table(
        std::str::from_utf8(&read("capture/fixture-boundaries.tsv")?)?,
        27,
        25,
    )?;
    let windows = table(std::str::from_utf8(&read("capture/windows.tsv")?)?, 4, 7)?;
    let shots = String::from_utf8(read("capture/shots.tsv")?)?;
    let shots: Vec<Vec<_>> = shots
        .lines()
        .skip(1)
        .map(|r| r.split('\t').collect())
        .collect();
    if shots.len() != 27 || shots.iter().any(|r| r.len() != 13) {
        return Err("Screen witness capture identity differs".into());
    }
    let expected = expected_pixels();
    let mut observations = Vec::new();
    let mut last = 0;
    let mut positive_reads = 0;
    let mut known_reads = 0;
    for slot in 0..27 {
        let arm = usize::try_from(scalar(shots[slot][1])?)?;
        let caller = windows
            .get(arm)
            .ok_or("Screen witness arm exceeds source inventory")?[6];
        let call_begin = scalar(shots[slot][6])?;
        let call_end = scalar(shots[slot][7])?;
        let pixels = read(&format!("capture/shot-{slot}/image.bgr"))?;
        let mut screens = Vec::new();
        let control = shots[slot][2].starts_with("dd-before-") || arm == 3;
        for side in 0..2 {
            let row = &rows[slot * 2 + side];
            let before = &row[14..26];
            let after = &row[26..38];
            geometry(before)?;
            geometry(after)?;
            if row[..4] != [i64::try_from(slot)?, i64::try_from(side)?, caller, caller]
                || !(1..=i64::from(u32::MAX)).contains(&caller)
                || row[4] <= 0
                || row[4] < last
                || row[5] < row[4]
                || row[6..14] != [1, 1, 1, 64, 1, 1, 1, 1]
                || before[..11] != after[..11]
                || (side == 0 && (row[5] > call_begin || after != &boundaries[slot][1..13]))
                || (side == 1 && (row[4] < call_end || before != &boundaries[slot][13..25]))
            {
                return Err("Screen witness source clock/owner/DC/target boundary differs".into());
            }
            last = row[5];
            let screen = read(&format!("screen/screen-{slot}-{side}.bgr"))?;
            if screen.len() != 18432 || (control && screen != expected) {
                return Err("Screen witness complete span/known positive control differs".into());
            }
            known_reads += usize::from(screen == expected);
            positive_reads += usize::from(control);
            screens.push(screen);
        }
        observations.push(
            serde_json::json!({"slot":slot,"phase":shots[slot][2],"arm":arm,
            "source_image":pixels.len()==18432,"source_known":pixels==expected,
            "before_known":screens[0]==expected,"after_known":screens[1]==expected,
            "screen_reads_equal":screens[0]==screens[1],"capture_equals_before":pixels==screens[0],
            "capture_equals_after":pixels==screens[1],"positive_screen_control":control,
            "before_begin_qpc":rows[slot*2][4],"before_end_qpc":rows[slot*2][5],
            "source_begin_qpc":call_begin,"source_end_qpc":call_end,
            "after_begin_qpc":rows[slot*2+1][4],"after_end_qpc":rows[slot*2+1][5]}),
        );
    }
    if positive_reads != 22 || inputs.len() != 88 {
        return Err("Screen witness checked input/control inventory differs".into());
    }
    let mut actual = 0;
    for entry in fs::read_dir(root.join("screen"))? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !inputs.iter().any(|i| root.join(&i.path) == entry.path())
        {
            return Err("Unexpected screen witness payload/link".into());
        }
        actual += 1;
    }
    if actual != 58 {
        return Err("Screen witness file inventory differs".into());
    }
    // The existing border reader validates all capture records. Register its
    // remaining raw files as well so this combined report closes its inputs.
    for entry in fs::read_dir(root.join("capture"))? {
        let entry = entry?;
        let paths = if entry.file_type()?.is_dir() {
            fs::read_dir(entry.path())?
                .map(|e| e.map(|e| e.path()))
                .collect::<std::io::Result<Vec<_>>>()?
        } else {
            vec![entry.path()]
        };
        for path in paths {
            if !inputs.iter().any(|i| root.join(&i.path) == path) {
                inputs.push(records::artifact(&root, &path)?);
            }
        }
    }
    if inputs.len() != 121 + usize::from(root.join("capture/border-validation.json").exists()) {
        return Err("Combined screen/capture input inventory differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":2,"source_calls":27,"screen_reads":54,
        "bitblt_calls":54,"getdibits_calls":54,"getdibits_rows":3456,"memory_getpixel_calls":7,"released_screen_dc":54,
        "positive_screen_reads":22,"known_screen_reads":known_reads,"observations":observations,"inputs":inputs,
        "atomic_screen_snapshot":false,"producer_proved":false,"original_failure_reclassified":false,"full_P1_accepted":false,
        "scope":"Additional screen-DC BitBlt(SRCCOPY|CAPTUREBLT)/GdiFlush/GetDIBits observations before/after each original call, with independent native GetPixel memory-DC channel/clipping/row control and same-thread successful bitmap restoration/deletion/DC release. All6144target pixels read into owned top-down32bit storage and scalar BGR, no target inset or shared OpenCV conversion. Native source/old border assertions remain unchanged; extra observation time may affect composition/capture timing. Equal bracketing screen reads do not prove an atomic screen state, physical scanout, capture-frame production time, pixel producer/cause, callback drain, Rust/manager parity or fullP1 acceptance."});
    let output = root.join("screen-witness-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved screen witness report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "source screen witness records calls=27 reads=54 bitblt_calls=54 getdibits_calls=54 released_dc=54 positive_reads=22 target_inset=0 atomic_snapshot=0 producer_proved=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
