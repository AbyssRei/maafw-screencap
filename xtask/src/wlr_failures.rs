// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual native Failed/EOF records, distinct from scripted Rust unit models.
use crate::{Result, records, wlr_protocol::cases};
use std::{fs, path::Path};

pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-protocol wlr-failures <external directory> [--check-only]"
                    .into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut cleanup_errors = Vec::new();
    let control = root.join("disconnect-control/disconnect-control.tsv");
    disconnect_controls(&fs::read_to_string(&control)?)?;
    artifacts.push(records::artifact(&root, &control)?);
    for side in ["source", "rust"] {
        let directory = root.join(side);
        if side == "rust" {
            let path = directory.join("rust-errors.tsv");
            errors(&fs::read_to_string(&path)?)?;
            artifacts.push(records::artifact(&root, &path)?);
        }
        let path = directory.join(format!("{side}-cases.tsv"));
        observations(&fs::read_to_string(&path)?)?;
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("producer/failure-requests.tsv");
        requests(&fs::read_to_string(&path)?)?;
        artifacts.push(records::artifact(&root, &path)?);
        for cycle in 0..20 {
            for phase in 0..5 {
                let index = if phase < 3 { 0 } else { 4 };
                let expected = if phase == 0 {
                    [201, 73, 17].repeat(35)
                } else {
                    cases::target(index)
                };
                let path = directory.join(format!("case-{cycle}-{phase}.{side}.bgr"));
                payload(&root, &path, &expected, &mut artifacts)?;
                if (1..=3).contains(&phase) {
                    let path = directory.join(format!("producer/case-{cycle}-{phase}.shm"));
                    payload(
                        &root,
                        &path,
                        &cases::native(if phase == 1 { 0 } else { 4 }),
                        &mut artifacts,
                    )?;
                }
                if side == "rust" && (phase == 1 || phase == 3) {
                    let path = directory.join(format!("case-{cycle}-{phase}.rust.raw"));
                    payload(&root, &path, &cases::raw(index), &mut artifacts)?;
                } else if side == "rust"
                    && directory
                        .join(format!("case-{cycle}-{phase}.rust.raw"))
                        .exists()
                {
                    return Err("Failed native operation unexpectedly published raw pixels".into());
                }
            }
        }
        for held in 0..40 {
            let path = directory.join(format!("held-{held}.{side}.bgr"));
            payload(
                &root,
                &path,
                &cases::target(if held % 2 == 0 { 0 } else { 4 }),
                &mut artifacts,
            )?;
        }
        let path = directory.join(format!("{side}-resources.tsv"));
        let cleanups = resources(&fs::read_to_string(&path)?, side == "rust")?;
        cleanup_errors.extend(cleanups);
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("server-stdout.log");
        let log = fs::read_to_string(&path)?;
        if !log.contains("failures ready cycles=20 operations=100 scripted_failed=40 disconnects=20 compositor=0")
            || !log.contains("failures stopped clients=20/20 frames=80/80 copies=60 failed=40 disconnected=20 graceful=1 error=0")
            || !log.contains("protocol stopped version=3 managers=20/20 frames=80/80 copies=60 graceful=1 error=0")
            || !log.lines().any(|line| line == "deferred disconnect scheduled=20 executed=20 cancelled=0 pending=0") {
            return Err("Native Failed/EOF fixture lifetime/stop evidence differs".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"operations_per_side":100,
        "ready_per_side":40,"failed_events_per_side":40,"disconnects_per_side":20,
        "deferred_disconnect_control_cases":3,"deferred_disconnect_executed_per_side":20,
        "native_producer_inputs":120,"caller_outputs":200,"held_bgr_images":80,"rust_raw_images":40,
        "rust_cleanup_codes":cleanup_errors,"artifacts":artifacts,
        "scope":"Actual native Wayland IPC/SHM fixture: Failed before Buffer, Ready, Failed after an actual SHM overwrite, Ready using the same wl_buffer, and deliberate server client destruction while waiting. No retry of failed assertions. Original source/Rust preserve old output, do no conversion for failed operations, retain the source connected flag after EOF and retain independent successful CPU images through complete native teardown. Rust's absent output API is exercised by a caller retaining its previous owning image. Source host explicitly detaches its output before later successful writes to retain original Mat owners. EOF cleanup error codes are preserved separately from primary failure; balanced fd/task counts do not prove successful protocol destruction. Passive input managers perform no actions. Not a compositor, host desktop, arbitrary disconnect timing, explicit client-close with live snapshots, complete error/resource matrix, public SDK or full P1 acceptance."});
    let path = root.join("failures-validation.json");
    if check_only && path.exists() {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("Saved Failed/EOF report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&path, &report)?;
    }
    println!(
        "Wlr native failures sides=2 cycles=40 operations=200 ready=80 failed_events=80 disconnects=40 payloads=440 actual_bytes_requests_resources_verified=1 cleanup_errors_preserved=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn disconnect_controls(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 3 {
        return Err("Incomplete native disconnect execution/cancellation controls".into());
    }
    let mut baseline = None;
    for (cycle, line) in rows.iter().enumerate() {
        let v = row(line)?;
        let cycle = i64::try_from(cycle)?;
        if v.len() != 8
            || v[..5] != [cycle, cycle + 1, 1, cycle, 0]
            || v[5] <= 0
            || v[5] != v[6]
            || v[7] != 1
            || baseline.is_some_and(|previous| previous != v[5])
        {
            return Err("Native deferred disconnect control owner/counter mismatch".into());
        }
        baseline = Some(v[5]);
    }
    Ok(())
}
fn payload(
    root: &Path,
    path: &Path,
    expected: &[u8],
    artifacts: &mut Vec<records::Artifact>,
) -> Result<()> {
    if fs::read(path)? != expected {
        return Err(format!("Failed/EOF payload differs: {}", path.display()).into());
    }
    artifacts.push(records::artifact(root, path)?);
    Ok(())
}
fn row(text: &str) -> Result<Vec<i64>> {
    Ok(text
        .split('\t')
        .map(str::parse)
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn observations(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 100 {
        return Err("Incomplete native Failed/EOF observations".into());
    }
    for (position, line) in rows.iter().enumerate() {
        let cycle = position / 5;
        let phase = position % 5;
        let success = phase == 1 || phase == 3;
        if *line
            != format!(
                "{cycle}\t{phase}\t{}\t1\t{}",
                u8::from(success),
                u8::from(!success)
            )
        {
            return Err("Failed/EOF result, output preservation or connected state differs".into());
        }
    }
    Ok(())
}
fn errors(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 60 {
        return Err("Missing native primary failure records".into());
    }
    for (position, line) in rows.iter().enumerate() {
        let cycle = position / 3;
        let phase = [0, 2, 4][position % 3];
        let (operation, code) = if phase == 4 {
            ("wl_display_roundtrip", -1)
        } else {
            ("native Wlr frame failed", -6)
        };
        if *line != format!("{cycle}\t{phase}\t{operation}\t{code}") {
            return Err("Native primary failure differs from expected Failed/EOF".into());
        }
    }
    Ok(())
}
fn requests(text: &str) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 100 {
        return Err("Incomplete native Failed/EOF requests".into());
    }
    let mut buffer = 0;
    for (position, line) in rows.iter().enumerate() {
        let cycle = i64::try_from(position / 5)?;
        let phase = position % 5;
        let values = row(line)?;
        let result = if phase == 4 {
            2
        } else {
            i64::from(phase == 1 || phase == 3)
        };
        if values.len() != 5
            || values[0] != cycle
            || values[1] != i64::try_from(phase)?
            || u32::try_from(values[2])
                .ok()
                .filter(|id| *id != 0)
                .is_none()
            || u32::try_from(values[3]).is_err()
            || values[4] != result
        {
            return Err("Native failure request order/identity/result differs".into());
        }
        if phase == 0 || phase == 4 {
            if values[3] != 0 {
                return Err("Unexpected copy before Buffer/after disconnect".into());
            }
        } else if phase == 1 {
            buffer = values[3];
            if buffer == 0 {
                return Err("Missing actual SHM copy".into());
            }
        } else if values[3] != buffer {
            return Err("Failed frame changed the unchanged-layout SHM owner".into());
        }
    }
    Ok(())
}
fn resources(text: &str, rust: bool) -> Result<Vec<i64>> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 20 {
        return Err("Incomplete native Failed/EOF resource rows".into());
    }
    let mut cleanup = Vec::new();
    for (cycle, line) in rows.iter().enumerate() {
        let v = row(line)?;
        if v.len() != if rust { 6 } else { 5 }
            || v[0] != i64::try_from(cycle)?
            || v[1] <= 0
            || v[1] != v[2]
            || v[3] <= 0
            || v[3] != v[4]
        {
            return Err("Failed/EOF fd/task owners did not close".into());
        }
        if rust {
            if !matches!(v[5], 0 | -1) {
                return Err("Unexpected Rust native cleanup failure".into());
            }
            cleanup.push(v[5]);
        }
    }
    Ok(cleanup)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_uncancelled_idle_or_unclosed_native_control_owners() {
        let rows = "0\t1\t1\t0\t0\t8\t8\t1\n1\t2\t1\t1\t0\t8\t8\t1\n2\t3\t1\t2\t0\t8\t8\t1\n";
        assert!(disconnect_controls(rows).is_ok());
        for bad in [
            rows.replace("\t8\t8", "\t8\t9"),
            rows.replacen("1\t2\t1\t1\t0", "1\t2\t1\t0\t1", 1),
            rows.replacen("2\t3\t1\t2", "2\t3\t2\t1", 1),
            rows.lines().take(2).collect::<Vec<_>>().join("\n"),
        ] {
            assert!(disconnect_controls(&bad).is_err());
        }
    }
    #[test]
    fn rejects_failure_relabeling_copied_before_buffer_and_changed_buffer_owners() {
        let observations_text = (0..100)
            .map(|p| {
                let phase = p % 5;
                let success = phase == 1 || phase == 3;
                format!(
                    "{}\t{phase}\t{}\t1\t{}\n",
                    p / 5,
                    u8::from(success),
                    u8::from(!success)
                )
            })
            .collect::<String>();
        assert!(observations(&observations_text).is_ok());
        assert!(
            observations(&observations_text.replacen("0\t0\t0\t1\t1", "0\t0\t1\t1\t0", 1)).is_err()
        );
        let native = (0..100)
            .map(|p| {
                let phase = p % 5;
                format!(
                    "{}\t{phase}\t{}\t{}\t{}\n",
                    p / 5,
                    phase + 10,
                    if phase == 0 || phase == 4 { 0 } else { 20 },
                    if phase == 4 {
                        2
                    } else {
                        i32::from(phase == 1 || phase == 3)
                    }
                )
            })
            .collect::<String>();
        assert!(requests(&native).is_ok());
        for bad in [
            native.replacen("0\t0\t10\t0", "0\t0\t10\t20", 1),
            native.replacen("0\t2\t12\t20", "0\t2\t12\t21", 1),
            native.replacen("0\t4\t14\t0\t2", "0\t4\t14\t0\t1", 1),
        ] {
            assert!(requests(&bad).is_err());
        }
    }
    #[test]
    fn preserves_expected_eof_cleanup_errors_and_rejects_unclosed_resources() {
        let rows = (0..20)
            .map(|i| format!("{i}\t4\t4\t2\t2\t-1\n"))
            .collect::<String>();
        assert_eq!(resources(&rows, true).unwrap(), vec![-1; 20]);
        assert!(resources(&rows.replacen("4\t4", "4\t5", 1), true).is_err());
        assert!(resources(&rows.replacen("2\t2\t-1", "2\t2\t-2", 1), true).is_err());
        assert!(resources(&rows.lines().take(19).collect::<Vec<_>>().join("\n"), true).is_err());
        let primary = (0..60)
            .map(|i| {
                let phase = [0, 2, 4][i % 3];
                let (op, code) = if phase == 4 {
                    ("wl_display_roundtrip", -1)
                } else {
                    ("native Wlr frame failed", -6)
                };
                format!("{}\t{phase}\t{op}\t{code}\n", i / 3)
            })
            .collect::<String>();
        assert!(errors(&primary).is_ok());
        assert!(
            errors(&primary.replacen("wl_display_roundtrip\t-1", "native Wlr frame failed\t-6", 1))
                .is_err()
        );
    }
}
