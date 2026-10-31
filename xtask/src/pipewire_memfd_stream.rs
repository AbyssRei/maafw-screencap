// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! External records for the controlled NULL-data real MemFd stream.
use crate::{Result, pipewire_trace, records};
use std::{fs, path::Path};

const HEADER: &str = "cycle\tdequeued\trestored\tlive\terrors\tcopies\town_maps\tunmaps";
fn row(text: &str, cycle: u64, rust: bool) -> Result<u64> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("MemFd stream header differs".into());
    }
    let fields: Vec<_> = lines
        .next()
        .ok_or("Missing MemFd stream row")?
        .split('\t')
        .collect();
    if fields.len() != 8 || lines.next().is_some() {
        return Err("MemFd stream row count/width differs".into());
    }
    let number = |value: &str| -> Result<u64> {
        if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
            return Err("Invalid MemFd stream count".into());
        }
        Ok(value.parse()?)
    };
    let observed: Vec<_> = fields[..5]
        .iter()
        .map(|v| number(v))
        .collect::<Result<_>>()?;
    if observed[0] != cycle
        || observed[1] < 3
        || observed[1] != observed[2]
        || observed[3..] != [0, 0]
        || fields[5..]
            != if rust {
                ["3", "3", "3"]
            } else {
                ["unknown", "unknown", "unknown"]
            }
    {
        return Err("MemFd stream intervention/return/owned mapping evidence differs".into());
    }
    Ok(observed[1])
}

pub fn run(args: &[String]) -> Result<()> {
    let [root, rest @ ..] = args else {
        return Err("usage: upstream-trace pipewire-memfd-stream ROOT [--check-only]".into());
    };
    if !rest.is_empty() && rest != ["--check-only"] {
        return Err("Invalid MemFd stream checker arguments".into());
    }
    let root = Path::new(root);
    let mut inputs = Vec::new();
    let mut dequeues = [0_u64; 2];
    for (index, side) in ["source", "rust"].into_iter().enumerate() {
        let rust = side == "rust";
        let extension = if rust { "rgb" } else { "bgr" };
        for cycle in 0..20 {
            let path = root.join(side).join(format!("probe-{cycle}.tsv"));
            inputs.push(records::artifact(root, &path)?);
            dequeues[index] = dequeues[index]
                .checked_add(row(&fs::read_to_string(path)?, cycle, rust)?)
                .ok_or("MemFd stream dequeues overflow")?;
            for call in 0..3 {
                let mut original = None;
                for held in ["", "-held"] {
                    let path = root
                        .join(side)
                        .join(format!("cycle-{cycle}-call-{call}{held}.{extension}"));
                    inputs.push(records::artifact(root, &path)?);
                    let bytes = fs::read(path)?;
                    if bytes.len() != 2016 {
                        return Err("Incomplete MemFd stream target pixels".into());
                    }
                    let marker = if rust {
                        u64::from(bytes[0]) << 16 | u64::from(bytes[1]) << 8 | u64::from(bytes[2])
                    } else {
                        u64::from(bytes[2]) << 16 | u64::from(bytes[1]) << 8 | u64::from(bytes[0])
                    };
                    if marker == 0 {
                        return Err("MemFd stream producer marker absent".into());
                    }
                    pipewire_trace::pixels(&bytes, marker, !rust)?;
                    if original.as_ref().is_some_and(|first| *first != bytes) {
                        return Err("MemFd stream held CPU pixels changed".into());
                    }
                    original = Some(bytes);
                }
            }
        }
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"captures_per_side":60,
        "pixel_files":240,"record_files":40,"source_dequeued_restored":dequeues[0],
        "rust_dequeued_restored":dequeues[1],"rust_own_maps_unmaps":60,"inputs":inputs,
        "scope":"Explicit isolated ELF intervention hides only data pointers on real dequeued MemFd leases; original pointers are restored before native queue. Actual original/Rust streams and CPU pixels, independent producer serials, exact FD/layout checks and native lifecycle premises are required by their own native commands. Source mapping/copy counts are unknown; Rust owns three maps/unmaps per lifecycle. No spontaneous NULL pointer, DMA/GPU, Portal, full format/failure matrix or complete P1 acceptance."});
    let output = root.join("memfd-stream-validation.json");
    if rest.is_empty() {
        records::create_json(&output, &report)?;
    } else if output.exists() && records::read::<serde_json::Value>(&output)? != report {
        return Err("Saved MemFd stream validation differs".into());
    }
    println!(
        "PipeWire MemFd stream records cycles=40 captures=120 pixels=240 intervention_rows=40 Rust_maps_unmaps=60 verified"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unrestored_live_corrupt_and_incomplete_native_rows() {
        let source = format!("{HEADER}\n7\t8\t8\t0\t0\tunknown\tunknown\tunknown\n");
        let rust = format!("{HEADER}\n7\t8\t8\t0\t0\t3\t3\t3\n");
        assert_eq!(row(&source, 7, false).unwrap(), 8);
        assert_eq!(row(&rust, 7, true).unwrap(), 8);
        for invalid in [
            rust.replace("7\t8\t8", "7\t8\t7"),
            rust.replace("8\t0\t0", "8\t1\t0"),
            rust.replace("8\t0\t0", "8\t0\t1"),
            rust.replace("3\t3\t3", "3\t0\t0"),
            rust.replace("7\t8", "6\t8"),
            rust.replace("7\t8\t8", "7\t2\t2"),
            rust.replace("7\t8", "7\t+8"),
            rust.replace("7\t8", "7\t18446744073709551616"),
            format!("{rust}unexpected\n"),
            format!("{HEADER}\n"),
            String::new(),
        ] {
            assert!(row(&invalid, 7, true).is_err(), "{invalid}");
        }
        assert!(row(&source, 7, true).is_err());
        assert!(row(&rust, 7, false).is_err());
    }
}
