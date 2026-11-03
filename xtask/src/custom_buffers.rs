// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read-only validation of actual retained native C string/image helper data.
use crate::{Result, records};
use std::{fs, path::Path};
const STRINGS: [&[u8]; 5] = [b"abc", b"a\0b", b"\xf0\x9f\xa7\xaa", b"", b"\xff\x80"];
fn validate(root: &Path) -> Result<serde_json::Value> {
    if fs::read(root.join("abi.tsv"))? != b"1\t8\t8\t1\t4\t8\n" {
        return Err("buffer helper native ABI widths differ".into());
    }
    if fs::read(root.join("counts.tsv"))?!=b"cycles\tstring_owners\timage_owners\tstring_bytes\timage_bytes\tfinal_images\tfinal_strings\n20\t100\t780\t240\t45120\t0\t0\n" {return Err("buffer helper owner/byte counts differ".into());}
    let string_rows = fs::read_to_string(root.join("strings.tsv"))?;
    let image_rows = fs::read_to_string(root.join("images.tsv"))?;
    let mut strings = string_rows.lines();
    let mut images = image_rows.lines();
    if strings.next() != Some("cycle\tcase\tvalues")
        || images.next() != Some("cycle\tdepth\tchannels\ttype\tscalar\tspan\toffset\tvalues")
    {
        return Err("buffer helper table headers differ".into());
    }
    let mut expected_strings = Vec::new();
    let mut expected_images = Vec::new();
    let scalars = [1, 1, 2, 2, 4, 4, 8, 2, 2, 1, 8, 8, 4];
    for cycle in 0..20 {
        for (case, body) in STRINGS.into_iter().enumerate() {
            let row = strings.next().ok_or("incomplete string helper cases")?;
            let fields = row.split('\t').collect::<Vec<_>>();
            if fields.len() != 3 || fields[0] != cycle.to_string() || fields[1] != case.to_string()
            {
                return Err("string helper case order differs".into());
            }
            let values: Vec<u64> = serde_json::from_str(fields[2])?;
            let n = u64::try_from(body.len())?;
            if values
                != [
                    1,
                    0,
                    1,
                    1,
                    0,
                    1,
                    1,
                    0,
                    0,
                    0,
                    1,
                    6,
                    1,
                    n,
                    u64::from(body.is_empty()),
                    0,
                    0,
                    n,
                    1,
                    1,
                    0,
                    1,
                    n,
                    1,
                ]
            {
                return Err("original string null/set/clear/retention semantics differ".into());
            }
            expected_strings.extend_from_slice(body);
        }
        for (depth, scalar) in scalars.into_iter().enumerate() {
            for channels in [1, 3, 4] {
                let kind = depth + ((channels - 1) << 5);
                let span = 6 * channels * scalar;
                let row = images.next().ok_or("incomplete image helper cases")?;
                let fields = row.split('\t').collect::<Vec<_>>();
                let prefix = [
                    cycle,
                    depth,
                    channels,
                    kind,
                    scalar,
                    span,
                    expected_images.len(),
                ]
                .map(|v| v.to_string());
                if fields.len() != 8 || fields[..7] != prefix {
                    return Err("image helper layout/order/offset differs".into());
                }
                let values: Vec<u64> = serde_json::from_str(fields[7])?;
                let k = u64::try_from(kind)?;
                let c = u64::try_from(channels)?;
                let n = u64::try_from(span)?;
                if values
                    != [
                        1, 1, 0, 0, 1, 0, 1, 0, 3, 2, c, k, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0, 1, 1,
                        1, 0, 0, c, k, n, 1,
                    ]
                {
                    return Err(
                        "original image null/clone/clear/type/retention semantics differ".into(),
                    );
                }
                for i in 0..span {
                    expected_images
                        .push(u8::try_from((cycle * 39 + depth * 3 + channels + i) % 251)?);
                }
            }
        }
    }
    if strings.next().is_some() || images.next().is_some() {
        return Err("additional buffer helper cases".into());
    }
    for (label, expected) in [("strings", expected_strings), ("images", expected_images)] {
        if fs::read(root.join(format!("{label}-current.bin")))? != expected
            || fs::read(root.join(format!("{label}-held.bin")))? != expected
        {
            return Err("buffer helper current/held bytes differ".into());
        }
    }
    let names = [
        "abi.tsv",
        "counts.tsv",
        "strings.tsv",
        "images.tsv",
        "strings-current.bin",
        "strings-held.bin",
        "images-current.bin",
        "images-held.bin",
    ];
    let mut inputs = names
        .into_iter()
        .map(|n| records::artifact(root, &root.join(n)))
        .collect::<Result<Vec<_>>>()?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        if !entry.file_type()?.is_file()
            || name != "buffer-validation.json" && !names.iter().any(|v| name == *v)
        {
            return Err("unregistered buffer helper file".into());
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(
        serde_json::json!({"layout_version":1,"cycles":20,"original_string_owners":100,"original_image_owners":780,"retained_strings":100,"retained_images":780,"string_bytes":240,"image_bytes":45120,"final_owners":0,"native_capture_frames":0,"inputs":inputs,"scope":"Actual full original Common/MaaBuffer.cpp C string/image helpers consumed through exception-contained native probes. Empty/null return distinctions, embedded NUL/arbitrary bytes, copied raw storage, clear preserving native Mat type and independently retained CPU values after original native source destruction. No Rust implementation of these legacy APIs, list/rect runtime acceptance, screenshot, arbitrary allocation fault or full P1 acceptance.","full_P1_accepted":false}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (root, only) = match args {
        [p] => (p, false),
        [p, flag] if flag == "--check-only" => (p, true),
        _ => {
            return Err(
                "expected upstream-trace custom-buffers <external directory> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(root))?;
    let report = validate(&root)?;
    let output = root.join("buffer-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved buffer helper report differs".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Custom buffer helpers cycles=20 original_string_owners=100 original_image_owners=780 retained_strings=100 retained_images=780 string_bytes=240 image_bytes=45120 final_owners=0 native_capture_frames=0 passed"
    );
    Ok(())
}
