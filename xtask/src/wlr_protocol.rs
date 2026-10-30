// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Byte/record verifier for the actual native IPC/SHM protocol fixture.
use crate::{Result, records};
use std::{fs, path::Path};
#[path = "../../tests/upstream-parity/linux-wlr/protocol-cases.rs"]
pub(super) mod cases;

pub fn run(args: &[String]) -> Result<()> {
    if args.first().is_some_and(|kind| kind == "wlr-failures") {
        return crate::wlr_failures::run(&args[1..]);
    }
    if args.first().is_some_and(|kind| kind == "wlr-registry") {
        return crate::wlr_registry::run(&args[1..]);
    }
    let (path, check_only) = match args {
        [kind, path] if kind == "wlr" => (path, false),
        [kind, path, flag] if kind == "wlr" && flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-protocol wlr <external matrix directory> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for version in 1..=3 {
        let directory = root.join(format!("v{version}"));
        for index in 0..24 {
            let path = directory.join("producer").join(format!("case-{index}.shm"));
            if fs::read(&path)? != cases::native(index) {
                return Err(format!("Native producer differs at v{version} case{index}").into());
            }
            artifacts.push(records::artifact(&root, &path)?);
        }
        for side in ["source", "rust"] {
            let path = directory.join(format!("{side}-cases.tsv"));
            table(version, &fs::read_to_string(&path)?)?;
            artifacts.push(records::artifact(&root, &path)?);
        }
        for cycle in 0..20 {
            for index in 0..24 {
                for (suffix, expected) in [
                    ("source.bgr", cases::legacy(index)),
                    ("rust.raw", cases::raw(index)),
                    ("rust.bgr", cases::legacy(index)),
                ] {
                    let path = directory.join(format!("case-{cycle}-{index}.{suffix}"));
                    let actual = fs::read(&path)?;
                    if actual != expected {
                        return Err(format!(
                            "Protocol payload differs: v{version} {cycle}/{index} {suffix}"
                        )
                        .into());
                    }
                    if suffix.ends_with(".bgr")
                        && (actual == cases::target(index)) != (index % 4 == 0)
                    {
                        return Err(
                            "Protocol intended-target/legacy-bug classification differs".into()
                        );
                    }
                    artifacts.push(records::artifact(&root, &path)?);
                }
            }
        }
        let path = directory.join("server-stdout.log");
        let log = fs::read_to_string(&path)?;
        if !log.contains(&format!("protocol ready version={version} formats=6 cases=24 native_shm=1 compositor=0"))
            || !log.contains(&format!("protocol stopped version={version} managers=40/40 frames=960/960 copies=960 graceful=1 error=0")) {
            return Err("Protocol fixture native lifetime/stop evidence differs".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"versions":[1,2,3],"formats":6,"layouts_per_format":4,
        "producer_inputs":72,"source_images":1440,"rust_raw_images":1440,"rust_bgr_images":1440,
        "positive_target_images_per_side":360,"legacy_stride_yinvert_images_per_side":1080,"artifacts":artifacts,
        "scope":"Actual native Wayland IPC/SHM protocol fixture only, with passive input-manager globals and no input actions or compositor/host desktop. Original source/Rust preserve the implicit packed Mat row and ignored YInvert: no-padding/no-inversion pixels match the intended target; other layouts match observed legacy bytes while differing from the intended target. This is not correct target capture acceptance for the legacy-bug cases, general compositor support, full format/failure/close behavior, SDK or P1 acceptance. Rust retains an extra safety CPU copy before real OpenCV conversion/copyTo at the source boundary."});
    let output = root.join("protocol-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved protocol validation differs from actual inputs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Wlr protocol versions=3 formats=6 layouts=4 producer_inputs=72 payloads=4320 bytes_verified=1 legacy_bugs_not_fixed=1 compositor_acceptance=0 passed"
    );
    Ok(())
}

fn table(version: u32, text: &str) -> Result<()> {
    if !(1..=3).contains(&version) {
        return Err("Unsupported protocol matrix version".into());
    }
    let rows: Vec<_> = text.lines().collect();
    if rows.len() != 480 {
        return Err("Incomplete native protocol matrix".into());
    }
    for (position, row) in rows.iter().enumerate() {
        let cycle = position / 24;
        let index = position % 24;
        let expected = format!(
            "{cycle}\t{index}\t{version}\t{}\t{}\t{}\t{}",
            cases::format(index),
            cases::stride(index),
            index % 2,
            usize::from(index % 4 == 0)
        );
        if *row != expected {
            return Err(format!("Protocol case row differs at {position}").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_native_padding_inversion_and_legacy_target_failures() {
        for index in 0..24 {
            assert_eq!(cases::native(index).len(), 5 * cases::stride(index));
            assert_eq!(cases::raw(index).len(), 7 * 5 * cases::channels(index));
            assert_eq!(cases::legacy(index) == cases::target(index), index % 4 == 0);
            if (index / 2) % 2 == 1 {
                assert!(
                    cases::native(index)[7 * cases::channels(index)..cases::stride(index)]
                        .iter()
                        .all(|byte| *byte == 0xa5)
                );
            }
        }
        let valid: String = (0..480)
            .map(|position| {
                let index = position % 24;
                format!(
                    "{}\t{index}\t1\t{}\t{}\t{}\t{}\n",
                    position / 24,
                    cases::format(index),
                    cases::stride(index),
                    index % 2,
                    usize::from(index % 4 == 0)
                )
            })
            .collect();
        assert!(table(1, &valid).is_ok());
        assert!(table(2, &valid).is_err());
        assert!(table(1, &valid.replacen("\t28\t", "\t27\t", 1)).is_err());
        assert!(table(1, &valid.replacen("\t1\n", "\t0\n", 1)).is_err());
        assert!(
            table(
                1,
                valid
                    .lines()
                    .take(479)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_str()
            )
            .is_err()
        );
    }
}
