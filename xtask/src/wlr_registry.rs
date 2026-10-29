// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independently recompute actual IPC registry selections and retained pixels.
use crate::{Result, records, wlr_protocol::cases};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-protocol wlr-registry <external directory> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let client_path = directory.join(format!("{side}-cases.tsv"));
        let client_rows = fs::read_to_string(&client_path)?;
        selections(&client_rows)?;
        let server_path = directory.join("producer/registry-requests.tsv");
        let server_rows = fs::read_to_string(&server_path)?;
        selections(&server_rows)?;
        if client_rows != server_rows {
            return Err("Native server output ids/versions differ from client snapshots".into());
        }
        artifacts.push(records::artifact(&root, &client_path)?);
        artifacts.push(records::artifact(&root, &server_path)?);
        for position in 0..80 {
            let generation = position / 4 + if position % 4 < 2 { 1 } else { 2 };
            let index = if generation % 2 == 1 { 0 } else { 4 };
            let mut payloads = vec![
                (
                    format!("producer/case-{position}.shm"),
                    cases::native(index),
                ),
                (format!("case-{position}.{side}.bgr"), cases::target(index)),
                (
                    format!("case-{position}.{side}.held.bgr"),
                    cases::target(index),
                ),
            ];
            if side == "rust" {
                payloads.push((format!("case-{position}.rust.raw"), cases::raw(index)));
            }
            for (name, expected) in payloads {
                let path = directory.join(name);
                if fs::read(&path)? != expected {
                    return Err(format!(
                        "Wrong registry output/retained bytes: {}",
                        path.display()
                    )
                    .into());
                }
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
        let path = directory.join(format!("{side}-resources.tsv"));
        let text = fs::read_to_string(&path)?;
        resources(&text)?;
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("server-stdout.log");
        let text = fs::read_to_string(&path)?;
        if !text.contains("registry ready rounds=20 captures=80 initial_outputs=2 compositor=0")
            || !text.contains("registry stopped rounds=20 globals=22 removed=21 outputs=22/22 frames=80/80 copies=80 graceful=1 error=0")
            || !text.contains("protocol stopped version=3 managers=1/1 frames=80/80 copies=80 graceful=1 error=0") {
            return Err("Registry fixture actual lifecycle/stop evidence differs".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"rounds_per_side":20,"captures_per_side":80,
        "native_producer_images":160,"source_images":80,"source_held_images":80,
        "rust_raw_images":80,"rust_bgr_images":80,"rust_held_images":80,"artifacts":artifacts,
        "scope":"Actual native Wayland registry/IPC/SHM fixture; two initial outputs, twenty replacement announcements and twenty-one global removals. Original source and Rust select the last announced output with its advertised version, retain prior capture snapshots, and ignore removal for later snapshots. Real native request ids, generation-specific pixels, CPU retention and full server/client closure are verified. Passive input-manager globals perform no input actions. Not a compositor, host desktop, live-client early close, complete failure/resource matrix, public SDK or full P1 acceptance."});
    let output = root.join("registry-validation.json");
    if check_only && output.exists() {
        let saved: serde_json::Value = records::read(&output)?;
        if saved != report {
            return Err("Saved registry report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "Wlr registry sides=2 rounds=40 captures=160 native_inputs=160 payloads=560 actual_ids_versions_pixels_retention_verified=1 compositor_acceptance=0 passed"
    );
    Ok(())
}

fn selections(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 80 {
        return Err("Incomplete native registry selections".into());
    }
    let mut old = 0;
    let mut next = 0;
    for (position, row) in rows.iter().enumerate() {
        let values = row
            .split('\t')
            .map(str::parse::<usize>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let generation = position / 4 + if position % 4 < 2 { 1 } else { 2 };
        let index = if generation % 2 == 1 { 0 } else { 4 };
        if values.len() != 5
            || values[0] != position
            || values[1] != generation
            || values[2] == 0
            || values[3] != index
            || values[4] != 1 + generation % 2
        {
            return Err("Native registry generation/order/version differs".into());
        }
        let id = values[2];
        match position % 4 {
            0 => {
                if position != 0 && id != next {
                    return Err("Client lost its prior current output".into());
                }
                old = id;
            }
            1 if id != old => return Err("Existing capture changed its output snapshot".into()),
            2 => {
                if id == old {
                    return Err("New capture retained the old registry output".into());
                }
                next = id;
            }
            3 if id != next => return Err("Global removal changed the retained output".into()),
            _ => {}
        }
    }
    Ok(())
}
fn resources(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err("Missing/duplicate registry resource row".into());
    }
    let values = rows[0]
        .split('\t')
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if values.len() != 4
        || values[0] == 0
        || values[2] == 0
        || values[0] != values[1]
        || values[2] != values[3]
    {
        return Err("Registry native fd/task resources differ".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_native_snapshot_generation_version_and_partial_shutdown() {
        let rows = (0..80)
            .map(|position| {
                let generation = position / 4 + if position % 4 < 2 { 1 } else { 2 };
                format!(
                    "{position}\t{generation}\t{}\t{}\t{}\n",
                    generation + 10,
                    if generation % 2 == 1 { 0 } else { 4 },
                    1 + generation % 2
                )
            })
            .collect::<String>();
        assert!(selections(&rows).is_ok());
        for invalid in [
            rows.replacen("1\t1\t11", "1\t1\t12", 1),
            rows.replacen("2\t2\t12", "2\t2\t11", 1),
            rows.replacen("3\t2\t12", "3\t2\t11", 1),
            rows.replacen("0\t1\t11\t0\t2", "0\t1\t11\t0\t1", 1),
            rows.lines().take(79).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(selections(&invalid).is_err());
        }
        assert!(resources("4\t4\t2\t2\n").is_ok());
        for text in [
            "",
            "4\t5\t2\t2\n",
            "4\t4\t2\t3\n",
            "0\t0\t0\t0\n",
            "4\t4\t2\t2\n4\t4\t2\t2\n",
        ] {
            assert!(resources(text).is_err());
        }
    }
}
