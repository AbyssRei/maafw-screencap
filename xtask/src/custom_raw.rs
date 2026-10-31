// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent actual raw representations and retained-native-owner records.
use crate::{Result, records};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace custom-raw <external raw directory> [--check-only]".into(),
            );
        }
    };
    let directory = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = directory.join(name);
        if fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
            return Err("Custom raw record exceeds the bounded matrix size".into());
        }
        artifacts.push(records::artifact(&directory, &path)?);
        Ok(fs::read(path)?)
    };
    if read("catalog.txt")? != b"depths=13\nchannels=128\nchannel_shift=5\ntype_mask=4095\n" {
        return Err(
            "Custom compiled raw catalog changed; review the complete new header catalog".into(),
        );
    }
    if read("rejections.tsv")? != b"short-span\t1\t0\t0\nnegative-type\t1\t0\t0\nreserved-depth\t1\t0\t0\nmax-reserved-depth\t1\t0\t0\ntype-bits-outside-mask\t1\t0\t0\nhuge-span\t1\t0\t0\n" {
        return Err("Custom raw rejected input changed output or reached the original setter".into());
    }
    let types = String::from_utf8(read("types.tsv")?)?;
    let cases = String::from_utf8(read("cases.tsv")?)?;
    let captured = read("captured.bin")?;
    let held = read("held.bin")?;
    if captured != held {
        return Err("Custom raw held payloads differ after input/context destruction".into());
    }
    let scalar_sizes = [1, 1, 2, 2, 4, 4, 8, 2, 2, 1, 8, 8, 4];
    let mut type_rows = types.lines();
    let mut case_rows = cases.lines();
    let mut index = 0;
    let mut offset = 0_usize;
    for (depth, scalar) in scalar_sizes.into_iter().enumerate() {
        for channels in 1..=128 {
            let kind = depth + ((channels - 1) << 5);
            let element = scalar * channels;
            let stride = element * 3;
            let span = stride * 2;
            let expected_type =
                format!("{depth}\t{channels}\t{kind}\t{scalar}\t{element}\t{stride}\t{span}");
            if type_rows.next() != Some(expected_type.as_str()) {
                return Err(format!(
                    "Custom actual compiled raw type/layout differs at {depth}/{channels}"
                )
                .into());
            }
            for code in [0, 1, 255] {
                let (width, height, output_kind, output_channels, row, length) = if code == 0 {
                    (7, 5, 64, 3, 21, 105)
                } else {
                    (3, 2, kind, channels, stride, span)
                };
                for side in ["source", "rust"] {
                    let expected_case = format!(
                        "{index}\t{side}\t{depth}\t{channels}\t{kind}\t{code}\t{}\t{width}\t{height}\t{output_kind}\t{output_channels}\t{row}\t{length}\t1",
                        usize::from(code != 0)
                    );
                    if case_rows.next() != Some(expected_case.as_str()) {
                        return Err(format!(
                            "Custom raw callback/order/owner differs at {index}/{side}"
                        )
                        .into());
                    }
                    let end = offset
                        .checked_add(length)
                        .ok_or("raw record offset overflow")?;
                    let bytes = captured
                        .get(offset..end)
                        .ok_or("Missing actual raw payload")?;
                    for (i, actual) in bytes.iter().copied().enumerate() {
                        let expected = if code == 0 {
                            [9, 17, 23][i % 3]
                        } else {
                            let value = i * 29 + depth * 17 + channels * 7 + code;
                            u8::try_from(if depth == 9 {
                                value % 2
                            } else {
                                value % 251 + 1
                            })?
                        };
                        if actual != expected {
                            return Err(format!(
                                "Custom actual {side} raw byte differs at {index}/{i}"
                            )
                            .into());
                        }
                    }
                    offset = end;
                }
                index += 1;
            }
        }
    }
    if type_rows.next().is_some()
        || case_rows.next().is_some()
        || captured.len() != offset
        || index != 4992
    {
        return Err("Extra/missing Custom raw records or payload bytes".into());
    }
    if read("stats.tsv")? != b"types\t1664\ncases\t4992\ncaptures\t9984\noriginal_raw_setter_calls\t9984\ncontexts\t13\nrust_scopes_entered\t4992\nrust_scopes_exited\t4992\nretained_native_images\t9984\nnative_images_after_release\t0\nnative_contexts_after_release\t0\n" {
        return Err("Custom raw native helper/callback/owner/scope counters differ".into());
    }
    let report = serde_json::json!({"layout_version":1,"types":1664,"cases":4992,"captures":9984,
        "payload_bytes":offset,"retained_native_images":9984,"original_raw_setter_calls":9984,
        "artifacts":artifacts,"scope":"All 13 depths and 128 channel counts from actual OpenCV 5 header catalog, original raw setter and real ImageBuffer in both managers, callback 0/1/255, cloned raw representations after input overwrite, false output preservation, real held native owners after callback/context destruction. No color conversion, encoded codec coverage, public SDK type values or complete P1 claim."});
    let output = directory.join("custom-raw-validation.json");
    if check_only && output.exists() {
        let existing: serde_json::Value = records::read(&output)?;
        if existing != report {
            return Err("Custom raw saved validation differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom raw types=1664 cases=4992 actual_captures=9984 native_setter_calls=9984 retained_native_images=9984 actual_payload_bytes={offset} passed"
    );
    Ok(())
}
