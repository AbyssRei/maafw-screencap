// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Strict records for the owned native RLIMIT_AS/SHM failure experiment.
use crate::{Result, records, wlr_protocol::cases};
use std::{fs, path::Path};

fn rows(text: &str, columns: usize, count: usize) -> Result<Vec<Vec<u64>>> {
    let rows: Vec<Vec<u64>> = text
        .lines()
        .map(|line| {
            line.split('\t')
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()
        })
        .collect::<std::result::Result<_, _>>()?;
    if rows.len() != count || rows.iter().any(|row| row.len() != columns) {
        return Err("native allocation table is incomplete or has an unknown shape".into());
    }
    Ok(rows)
}
fn observations(text: &str) -> Result<Vec<Vec<u64>>> {
    let data = rows(text, 18, 20)?;
    for (cycle, row) in data.iter().enumerate() {
        if row[0] != u64::try_from(cycle)?
            || row[1] != row[5]
            || row[2] < row[1]
            || row[2].checked_add(1) != Some(row[3])
            || row[3] != row[4]
            || row[6] == 0
            || row[6].checked_add(33_554_432) != Some(row[7])
            || row[7] > row[8]
            || row[8] != row[9]
            || row[10] != 268_435_456
            || row[11] != 12
            || row[12] > i32::MAX as u64
            || row[13] == 0
            || row[14] == 0
            || row[13] == row[14]
            || row[15..] != [0, 1, 1]
        {
            return Err(
                "native failed-map FD, exact limit restoration, output or recovery differs".into(),
            );
        }
    }
    Ok(data)
}
fn requests(text: &str) -> Result<()> {
    for (index, row) in rows(text, 5, 40)?.iter().enumerate() {
        let phase = u64::try_from(index % 2)?;
        if row[0] != u64::try_from(index / 2)?
            || row[1] != phase
            || row[2] == 0
            || row[4] != phase
            || (row[3] == 0) != (phase == 0)
        {
            return Err("native allocation offer/Copy sequence differs".into());
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-protocol wlr-allocation <external matrix> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let path = directory.join(format!("{side}-allocation.tsv"));
        let data = observations(&fs::read_to_string(&path)?)?;
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("producer/allocation-requests.tsv");
        requests(&fs::read_to_string(&path)?)?;
        artifacts.push(records::artifact(&root, &path)?);
        for (cycle, row) in data.iter().enumerate() {
            let path = directory.join(format!("attempt-{cycle}.{side}.tsv"));
            let attempt = rows(
                &fs::read_to_string(&path)?,
                if side == "source" { 10 } else { 9 },
                1,
            )?
            .pop()
            .unwrap();
            let expected = if side == "source" {
                vec![row[6], row[7], row[8], 1, 12, row[12], row[2], row[3], 1, 1]
            } else {
                vec![
                    row[6], row[7], row[8], row[9], 12, row[2], row[3], 1, row[12],
                ]
            };
            if attempt != expected {
                return Err("native allocation attempt conflicts with completed row".into());
            }
            artifacts.push(records::artifact(&root, &path)?);
            let mut payloads = vec![
                (
                    directory.join(format!("failed-{cycle}.{side}.bgr")),
                    [201, 73, 17].repeat(35),
                ),
                (
                    directory.join(format!("held-{cycle}.{side}.bgr")),
                    cases::target(0),
                ),
                (
                    directory.join(format!("producer/case-{cycle}-1.shm")),
                    cases::native(0),
                ),
            ];
            if side == "rust" {
                payloads.push((
                    directory.join(format!("case-{cycle}.rust.raw")),
                    cases::raw(0),
                ));
            }
            for (path, expected) in payloads {
                if fs::read(&path)? != expected {
                    return Err(
                        format!("native allocation pixels differ: {}", path.display()).into(),
                    );
                }
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
        let path = directory.join("server-stdout.log");
        let log = fs::read_to_string(&path)?;
        if !log.lines().any(|line| line == "allocation ready cycles=20 operations=40 sparse_span=268435456 compositor=0")
            || !log.lines().any(|line| line == "allocation stopped clients=20/20 frames=40/40 copies=20 failed=20 graceful=1 error=0")
            || !log.lines().any(|line| line == "protocol stopped version=3 managers=20/20 frames=40/40 copies=20 graceful=1 error=0") {
            return Err("native allocation producer/stop evidence differs".into());
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"failed_maps_per_side":20,
        "restored_limits_per_side":20,"recovery_per_side":20,"artifacts":artifacts,
        "full_P1_accepted":false,"scope":"Read-only verification of actual owned IPC/native RLIMIT_AS mmap failures, retained FD identity, caller pixels, replacement and recovery. No desktop, arbitrary resource exhaustion, all Wlr paths or P1 acceptance."});
    let report_path = root.join("allocation-report.json");
    if check_only {
        let old: serde_json::Value = records::read(&report_path)?;
        if old != report {
            return Err("saved native allocation report differs".into());
        }
    } else {
        records::create_json(&report_path, &report)?;
    }
    println!(
        "Native Wlr allocation records verified: cycles_per_side=20 failed_maps=40 recovery=40 outputs=80; full_P1_acceptance=0"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Vec<u64>> {
        (0..20)
            .map(|cycle| {
                vec![
                    cycle,
                    5,
                    6,
                    7,
                    7,
                    5,
                    1_073_741_824,
                    1_107_296_256,
                    u64::MAX,
                    u64::MAX,
                    268_435_456,
                    12,
                    8,
                    41 + cycle * 2,
                    42 + cycle * 2,
                    0,
                    1,
                    1,
                ]
            })
            .collect()
    }
    fn text(data: &[Vec<u64>]) -> String {
        data.iter()
            .map(|r| r.iter().map(u64::to_string).collect::<Vec<_>>().join("\t") + "\n")
            .collect()
    }
    #[test]
    fn rejects_released_failure_fd_unrestored_limits_and_fabricated_recovery() {
        let data = fixture();
        assert!(observations(&text(&data)).is_ok());
        for column in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
            let mut bad = data.clone();
            bad[0][column] = match column {
                12 => u64::MAX,
                13 => 0,
                14 => bad[0][13],
                _ => bad[0][column].wrapping_add(1),
            };
            assert!(observations(&text(&bad)).is_err(), "column{column}");
        }
        assert!(observations(&text(&data[..19])).is_err());
        let mut duplicate = data.clone();
        duplicate[19] = duplicate[0].clone();
        assert!(observations(&text(&duplicate)).is_err());
    }
    #[test]
    fn rejects_copy_after_the_failed_large_offer_and_missing_native_requests() {
        let data: Vec<_> = (0..40)
            .map(|i| vec![i / 2, i % 2, i + 1, if i % 2 == 0 { 0 } else { 99 }, i % 2])
            .collect();
        assert!(requests(&text(&data)).is_ok());
        let mut bad = data.clone();
        bad[0][3] = 99;
        assert!(requests(&text(&bad)).is_err());
        assert!(requests(&text(&data[..39])).is_err());
    }
}
