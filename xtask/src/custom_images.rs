// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Recompute real encoded/resize callbacks from independent known pixels.
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) =
        match args {
            [path] => (path, false),
            [path, flag] if flag == "--check-only" => (path, true),
            _ => return Err(
                "expected upstream-trace custom-images <external images directory> [--check-only]"
                    .into(),
            ),
        };
    let directory = records::external_root(Path::new(path))?;
    let parent = directory.parent().ok_or("Custom images parent missing")?;
    crate::custom_capture::run(&[parent.to_string_lossy().into_owned(), "--check-only".into()])?;
    let abi = fs::read_to_string(parent.join("abi.txt"))?;
    let value = |name: &str| -> Result<usize> {
        Ok(abi
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .ok_or("Missing actual image format ABI")?
            .parse()?)
    };
    let kind = value("CV_8UC3")?;
    let empty_kind = value("CV_8UC1")?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = directory.join(name);
        if fs::metadata(&path)?.len() > 1024 * 1024 {
            return Err("Custom bounded image record exceeds one MiB".into());
        }
        artifacts.push(records::artifact(&directory, &path)?);
        Ok(fs::read(path)?)
    };
    let inputs = (0..6)
        .map(|index| read(&format!("input-{index}.bin")))
        .collect::<Result<Vec<_>>>()?;
    for (index, color) in [0, 2, 6].into_iter().enumerate() {
        png(&inputs[index], 8, 6, color)?;
    }
    if inputs[3] != b"invalid-encoded-image" || inputs[4] != [0x37] || !inputs[5].is_empty() {
        return Err("Custom invalid/zero/null image inputs differ".into());
    }
    if read("pixel-kind.txt")? != format!("{kind}\n").as_bytes() {
        return Err("Custom image constant differs from actual compiled ABI".into());
    }
    let cases = String::from_utf8(read("cases.tsv")?)?;
    let mut rows = cases.lines();
    let mut paired = BTreeMap::new();
    for cycle in 0..20 {
        for index in (0..144).filter(|index| index % 20 == cycle) {
            let input = index / 24;
            let resize = index / 3 % 8;
            let code = [0, 1, 255][index % 3];
            let empty = input == 3 || input == 4;
            let (width, height) = dimensions(input, resize);
            let pixels = pattern(input == 0, width, height);
            let base = if empty {
                Vec::new()
            } else {
                pattern(input == 0, 8, 6)
            };
            for side in ["source", "rust"] {
                let status = usize::from(!empty && resize == 7) * 2;
                let result = usize::from(!empty && (1..=5).contains(&resize));
                let expected = format!(
                    "{index}\t{cycle}\t{side}\t{input}\t{resize}\t{code}\t0\t{}\t{status}\t{result}\t{}",
                    usize::from(input != 5),
                    usize::from(code != 0)
                );
                if rows.next() != Some(expected.as_str()) {
                    return Err(
                        format!("Custom image callback/order differs at {index}/{side}").into(),
                    );
                }
                let name = format!("{index}-{side}");
                let output = if code == 0 {
                    [9, 17, 23].repeat(35)
                } else {
                    pixels.clone()
                };
                for suffix in ["pixels", "held"] {
                    if read(&format!("{name}.{suffix}"))? != output {
                        return Err(format!(
                            "Custom captured/held pixels differ at {name}/{suffix}"
                        )
                        .into());
                    }
                }
                if read(&format!("{name}.before-pixels"))? != base
                    || read(&format!("{name}.after-pixels"))? != pixels
                {
                    return Err(format!(
                        "Custom independently decoded cache pixels differ at {name}"
                    )
                    .into());
                }
                let before = read(&format!("{name}.before-encoded"))?;
                let after = read(&format!("{name}.after-encoded"))?;
                let expected_before = match input {
                    0..=3 => &inputs[input],
                    4 => &inputs[5],
                    _ => &inputs[1],
                };
                if &before != expected_before {
                    return Err(format!(
                        "Custom encoded copy/cache preservation differs at {name}"
                    )
                    .into());
                }
                if empty || resize == 0 || resize == 6 {
                    if before != after {
                        return Err(format!("Custom unchanged cache differs at {name}").into());
                    }
                } else {
                    // Resize marks dirty before the native call, even when
                    // negative dimensions subsequently throw. Lazy re-encode
                    // therefore emits actual decoded BGR, discarding old alpha.
                    png(&after, width, height, 2)?;
                }
                let layouts = String::from_utf8(read(&format!("{name}.layout"))?)?;
                let layouts: Vec<_> = layouts.lines().collect();
                if layouts.len() != 3 {
                    return Err("Custom image layout row count differs".into());
                }
                let (out_width, out_height) = if code == 0 { (7, 5) } else { (width, height) };
                layout(
                    layouts[0],
                    out_width,
                    out_height,
                    kind,
                    empty_kind,
                    code != 0 && input == 4,
                )?;
                layout(
                    layouts[1],
                    if empty { 0 } else { 8 },
                    if empty { 0 } else { 6 },
                    kind,
                    empty_kind,
                    false,
                )?;
                layout(layouts[2], width, height, kind, empty_kind, false)?;
                let cache = (
                    before,
                    after,
                    layouts
                        .iter()
                        .map(|line| line.to_string())
                        .collect::<Vec<_>>(),
                );
                if side == "source" {
                    paired.insert(index, cache);
                } else if paired.remove(&index) != Some(cache) {
                    return Err(format!("Custom actual source/Rust encoded bytes or native layouts differ at {index}").into());
                }
            }
        }
    }
    if rows.next().is_some() || !paired.is_empty() {
        return Err("Extra/incomplete Custom image records".into());
    }
    let stats = b"lifecycles\t20\ncaptures\t288\nrust_scopes_entered\t144\nrust_scopes_exited\t144\nretained_native_images\t288\ninspection_decodes\t576\nnative_images_after_release\t0\nnative_contexts_after_release\t0\n";
    if read("stats.tsv")? != stats {
        return Err("Custom image owner/scope counters differ".into());
    }
    let report = serde_json::json!({"layout_version":1,"lifecycles":20,"cases":144,"captures":288,
        "retained_native_images":288,"inspection_decodes":576,"contained_negative_resize_exceptions":24,
        "artifacts":artifacts,"compiled_BGR_kind":kind,
        "scope":"Actual fixed raw/encoded/resize helpers inside original/Rust manager callbacks; self-owned gray/BGR/BGRA PNG, malformed/zero/null input, cache copies and integer-ratio INTER_AREA/auto sizes, false output preservation, contained negative-size exceptions, native owners past context shutdown and CPU bytes after module unload. Codec diagnostic copies/decode are explicit test work. Does not establish every codec/raw type, arbitrary resampling, general native failures, public provider ABI or complete P1."});
    let output = directory.join("custom-images-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Custom image saved validation differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom images lifecycles=20 cases=144 actual_captures=288 retained_native_images=288 decoded_cache_payloads=576 passed"
    );
    Ok(())
}

fn dimensions(input: usize, resize: usize) -> (usize, usize) {
    if input == 3 || input == 4 {
        return (0, 0);
    }
    match resize {
        2 => (16, 12),
        3..=5 => (4, 3),
        _ => (8, 6),
    }
}
fn pattern(gray: bool, width: usize, height: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (x, y, half) = match width {
                16 => (x / 2, y / 2, false),
                4 => (x * 2, y * 2, true),
                _ => (x, y, false),
            };
            let values = if gray {
                [12 + 8 * x + 4 * y + usize::from(half) * 6; 3]
            } else {
                [
                    12 + 8 * x + usize::from(half) * 4,
                    20 + 8 * y + usize::from(half) * 4,
                    28 + 4 * (x + y) + usize::from(half) * 4,
                ]
            };
            bytes.extend(values.map(|value| u8::try_from(value).expect("bounded fixture byte")));
        }
    }
    bytes
}
fn layout(
    line: &str,
    width: usize,
    height: usize,
    kind: usize,
    empty_kind: usize,
    released_raw: bool,
) -> Result<()> {
    let values = line
        .split('\t')
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if values.len() != 7 {
        return Err("Custom image layout width differs".into());
    }
    let expected = if width == 0 && released_raw {
        // Actual ImageBuffer::clear uses Mat::release, which preserves the
        // former raw BGR type/channels/row step while its readable span is 0.
        [0, 0, kind, 3, 24, 0]
    } else if width == 0 {
        [0, 0, empty_kind, 1, 0, 0]
    } else {
        [
            width,
            height,
            kind,
            3,
            width.checked_mul(3).ok_or("row overflow")?,
            width
                .checked_mul(height)
                .and_then(|n| n.checked_mul(3))
                .ok_or("span overflow")?,
        ]
    };
    if values[..6] != expected || (width == 0 && values[6] != 0) || (width != 0 && values[6] == 0) {
        return Err("Custom native image geometry/type/span/owner differs".into());
    }
    Ok(())
}
fn png(bytes: &[u8], width: usize, height: usize, color: u8) -> Result<()> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Custom PNG signature differs".into());
    }
    let mut offset: usize = 8;
    let mut header = false;
    let mut data = false;
    let mut end = false;
    while offset < bytes.len() {
        let prefix = bytes
            .get(offset..offset.checked_add(8).ok_or("PNG offset overflow")?)
            .ok_or("Truncated PNG chunk")?;
        let length = usize::try_from(u32::from_be_bytes(prefix[..4].try_into()?))?;
        let finish = offset
            .checked_add(12)
            .and_then(|n| n.checked_add(length))
            .ok_or("PNG length overflow")?;
        let chunk = bytes.get(offset..finish).ok_or("Truncated PNG payload")?;
        let kind = &chunk[4..8];
        if kind.iter().any(|byte| !byte.is_ascii_alphabetic()) {
            return Err("Invalid PNG chunk type".into());
        }
        let mut crc = u32::MAX;
        for byte in &chunk[4..chunk.len() - 4] {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
            }
        }
        if !crc != u32::from_be_bytes(chunk[chunk.len() - 4..].try_into()?) {
            return Err("Custom PNG CRC differs".into());
        }
        if !header {
            if kind != b"IHDR" || length != 13 {
                return Err("Custom PNG header missing".into());
            }
            let w = usize::try_from(u32::from_be_bytes(chunk[8..12].try_into()?))?;
            let h = usize::try_from(u32::from_be_bytes(chunk[12..16].try_into()?))?;
            if (w, h) != (width, height) || chunk[16..21] != [8, color, 0, 0, 0] {
                return Err("Custom PNG dimensions/format differ".into());
            }
            header = true;
        } else if kind == b"IHDR" {
            return Err("Duplicate PNG header".into());
        }
        if kind == b"IDAT" {
            if length == 0 {
                return Err("Empty PNG data".into());
            }
            data = true;
        }
        if kind == b"IEND" {
            if length != 0 || !data || finish != bytes.len() {
                return Err("Custom PNG end differs".into());
            }
            end = true;
        }
        offset = finish;
    }
    if !header || !data || !end {
        return Err("Incomplete Custom PNG".into());
    }
    Ok(())
}
