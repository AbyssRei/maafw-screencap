// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Check actual CPU record controls without treating corrupt images as captures.
use crate::{Result, records};
use std::{fs, path::Path};

const ROWS: [&str; 8] = [
    "0\t0\t1\t1",
    "1\t0\t1\t1",
    "2\t1\t1\t1",
    "3\t1\t1\t1",
    "4\t1\t1\t1",
    "5\t1\t1\t1",
    "6\t0\t0\t1",
    "7\t0\t0\t0",
];
fn expected(case: usize, cache: bool) -> Option<Vec<u8>> {
    if case == 7 || (case == 6 && !cache) {
        return None;
    }
    let bad_marker = (case == 2 && !cache) || (case == 3 && cache);
    let bad_body = (case == 4 && !cache) || (case == 5 && cache);
    let mut bytes = Vec::with_capacity(18432);
    for y in 0..64 {
        for x in 0..96 {
            bytes.extend_from_slice(&if x < 16 && y < 16 {
                if bad_marker {
                    [0x1f, 0x53, 0xa7]
                } else if case == 1 {
                    [0x1a, 0x2b, 0x3c]
                } else {
                    [0x4e, 0x5f, 0x60]
                }
            } else if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            });
        }
    }
    if bad_body {
        bytes[18431] ^= 1;
    }
    Some(bytes)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace framepool-record-control <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let table = directory.join("record-control.tsv");
        let text = fs::read_to_string(&table)?;
        let mut lines = text.lines();
        if lines.next() != Some("case\trejected\timage\tcache") || lines.collect::<Vec<_>>() != ROWS
        {
            return Err("CPU record control matrix/rejections differ".into());
        }
        artifacts.push(records::artifact(&root, &table)?);
        for case in 0..8 {
            for cache in [false, true] {
                let name = format!("control-{case}{}", if cache { "-cache" } else { "" });
                let layout = directory.join(format!("{name}.txt"));
                let pixels = directory.join(format!("{name}.bgr"));
                let wanted = expected(case, cache);
                if fs::read(&pixels)? != wanted.as_deref().unwrap_or_default()
                    || fs::read_to_string(&layout)?.trim_end()
                        != if wanted.is_some() {
                            "96\t64\t288\t18432"
                        } else {
                            "0\t0\t0\t0"
                        }
                {
                    return Err(
                        "CPU record control lost or changed actual image/cache bytes".into(),
                    );
                }
                artifacts.push(records::artifact(&root, &layout)?);
                artifacts.push(records::artifact(&root, &pixels)?);
            }
        }
    }
    let report = serde_json::json!({"layout_version":1,"cases_per_side":8,"expected_rejections_per_side":4,"pixel_files":32,"captures":0,"artifacts":artifacts,
        "scope":"Actual original OpenCV Mat and Rust/native CPU image recording helpers, with both valid marker phases, independently corrupted result/cache marker and final pixel, and explicit missing values. Both output and cache bytes must be preserved before either strict assertion. These are record-control inputs, not screen captures, passed corrupt images, historical root-cause proof or full P1 acceptance."});
    let output = root.join("record-control-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved CPU record control verification differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool CPU record controls source_cases=8 Rust_cases=8 rejected_per_side=4 pixel_files=32 captures=0 passed"
    );
    Ok(())
}
