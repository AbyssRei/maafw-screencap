// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read-only records for isolated kernel memfd/ftruncate failures.
use crate::{Result, records, wlr_protocol::cases};
use std::{fs, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Memfd,
    Truncate,
}
impl Kind {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "memfd" => Ok(Self::Memfd),
            "ftruncate" => Ok(Self::Truncate),
            _ => Err("expected memfd or ftruncate resource failure kind".into()),
        }
    }
    fn errno(self) -> u64 {
        if self == Self::Memfd { 24 } else { 27 }
    }
    fn signal(self) -> u64 {
        u64::from(self == Self::Truncate)
    }
}
fn table(text: &str, columns: usize, count: usize) -> Result<Vec<Vec<u64>>> {
    let data: Vec<Vec<u64>> = text
        .lines()
        .map(|line| {
            line.split('\t')
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()
        })
        .collect::<std::result::Result<_, _>>()?;
    if data.len() != count || data.iter().any(|r| r.len() != columns) {
        return Err("native resource failure table is incomplete or has unknown columns".into());
    }
    Ok(data)
}
fn observations(text: &str, kind: Kind) -> Result<Vec<Vec<u64>>> {
    let data = table(text, 22, 20)?;
    for (cycle, r) in data.iter().enumerate() {
        if r[0] != u64::try_from(cycle)?
            || r[1] == 0
            || r[2] < r[1]
            || r[3] != r[2]
            || r[2].checked_add(1) != Some(r[4])
            || r[5] != r[1]
            || r[6] != kind.errno()
            || r[7] == 0
            || (kind == Kind::Memfd && r[7] <= r[2])
            || (kind == Kind::Truncate && r[7] < 268_435_456)
            || r[7] > r[8]
            || r[7] != r[9]
            || r[8] != r[10]
            || r[11..14] != [0, 0, 140]
            || r[14] == 0
            || r[15..17] != [1, 0]
            || r[17] > 1
            || r[18..] != [1, kind.signal(), kind.signal(), 1]
        {
            return Err(
                "native resource FD/limit/signal/output/recovery observations differ".into(),
            );
        }
    }
    Ok(data)
}
fn attempt(text: &str, side: &str, kind: Kind, row: &[u64]) -> Result<()> {
    let expected = if side == "source" {
        let observed = table(text, 10, 1)?.pop().unwrap();
        if (kind == Kind::Memfd && observed[3] != u64::MAX)
            || (kind == Kind::Truncate && observed[3] > i32::MAX as u64)
        {
            return Err("native source failing syscall descriptor differs".into());
        }
        vec![
            1,
            1,
            kind.errno(),
            observed[3],
            1,
            row[17],
            1,
            kind.signal(),
            kind.signal(),
            1,
        ]
    } else {
        vec![
            kind.errno(),
            0,
            1,
            0,
            row[17],
            1,
            kind.signal(),
            kind.signal(),
            1,
        ]
    };
    if table(text, expected.len(), 1)?.pop().unwrap() != expected {
        return Err("native resource failed attempt conflicts with complete row".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (name,path,check_only) = match args {
        [name,path] => (name,path,false),
        [name,path,flag] if flag == "--check-only" => (name,path,true),
        _ => return Err("expected upstream-protocol wlr-resource-errors <memfd|ftruncate> <matrix> [--check-only]".into()),
    };
    let kind = Kind::parse(name)?;
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        let path = directory.join(format!("{side}-resource.tsv"));
        let data = observations(&fs::read_to_string(&path)?, kind)?;
        artifacts.push(records::artifact(&root, &path)?);
        let path = directory.join("producer/allocation-requests.tsv");
        for (index, row) in table(&fs::read_to_string(&path)?, 5, 40)?
            .iter()
            .enumerate()
        {
            let phase = u64::try_from(index % 2)?;
            if row[0] != u64::try_from(index / 2)?
                || row[1] != phase
                || row[2] == 0
                || row[2] > u32::MAX as u64
                || row[3] > u32::MAX as u64
                || row[4] != phase
                || (row[3] == 0) != (phase == 0)
            {
                return Err("resource producer offer/Copy sequence differs".into());
            }
        }
        artifacts.push(records::artifact(&root, &path)?);
        for (cycle, row) in data.iter().enumerate() {
            let path = directory.join(format!("attempt-{cycle}.{side}.tsv"));
            attempt(&fs::read_to_string(&path)?, side, kind, row)?;
            artifacts.push(records::artifact(&root, &path)?);
            let mut pixels = vec![
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
                pixels.push((
                    directory.join(format!("case-{cycle}.rust.raw")),
                    cases::raw(0),
                ));
            }
            for (path, expected) in pixels {
                if fs::read(&path)? != expected {
                    return Err(format!("native resource pixels differ: {}", path.display()).into());
                }
                artifacts.push(records::artifact(&root, &path)?);
            }
        }
        let path = directory.join("server-stdout.log");
        let text = fs::read_to_string(&path)?;
        for expected in [
            "allocation ready cycles=20 operations=40 sparse_span=268435456 compositor=0",
            "allocation stopped clients=20/20 frames=40/40 copies=20 failed=20 graceful=1 error=0",
            "protocol stopped version=3 managers=20/20 frames=40/40 copies=20 graceful=1 error=0",
        ] {
            if !text.lines().any(|line| line == expected) {
                return Err("native resource producer/stop evidence differs".into());
            }
        }
        artifacts.push(records::artifact(&root, &path)?);
    }
    let report = serde_json::json!({"layout_version":1,"kind":name,"errno":kind.errno(),"cycles_per_side":20,
        "kernel_failures_per_side":20,"retained_failed_fds":0,"recovery_per_side":20,"limits_and_signal_state_restored":true,
        "artifacts":artifacts,"full_P1_accepted":false,
        "scope":"Isolated native test-process kernel resource failures with caller sentinel, unavailable owner, fd release, original limit/signal state, recovery and held pixels. Source syscall wrappers observe actual arguments/results under temporary limits after UUID; Rust uses an owned outer limit. No arbitrary exhaustion, combined failures or complete Wlr/P1 acceptance."});
    let path = root.join("resource-error-report.json");
    if check_only {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("saved resource error report differs".into());
        }
    } else {
        records::create_json(&path, &report)?;
    }
    println!(
        "Native Wlr resource kind={name} cycles_per_side=20 kernel_failures=40 failed_fds_retained=0 recovery=40 outputs=80 full_P1_acceptance=0"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(cycle: u64, kind: Kind) -> Vec<u64> {
        let before = if kind == Kind::Memfd { 1024 } else { 1 << 30 };
        vec![
            cycle,
            5,
            8,
            8,
            9,
            5,
            kind.errno(),
            before,
            u64::MAX,
            before,
            u64::MAX,
            0,
            0,
            140,
            cycle + 100,
            1,
            0,
            0,
            1,
            kind.signal(),
            kind.signal(),
            1,
        ]
    }
    fn text(rows: &[Vec<u64>]) -> String {
        rows.iter()
            .map(|r| {
                r.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn accepts_exact_zero_failed_fd_and_restored_signal_records() {
        for kind in [Kind::Memfd, Kind::Truncate] {
            let rows: Vec<_> = (0..20).map(|i| row(i, kind)).collect();
            assert!(observations(&text(&rows), kind).is_ok());
            let fd = if kind == Kind::Memfd { u64::MAX } else { 9 };
            assert!(
                attempt(
                    &text(&[vec![
                        1,
                        1,
                        kind.errno(),
                        fd,
                        1,
                        0,
                        1,
                        kind.signal(),
                        kind.signal(),
                        1
                    ]]),
                    "source",
                    kind,
                    &rows[0]
                )
                .is_ok()
            );
            assert!(
                attempt(
                    &text(&[vec![
                        kind.errno(),
                        0,
                        1,
                        0,
                        0,
                        1,
                        kind.signal(),
                        kind.signal(),
                        1
                    ]]),
                    "rust",
                    kind,
                    &rows[0]
                )
                .is_ok()
            );
        }
    }
    #[test]
    fn rejects_retained_failed_fds_wrong_errors_and_unrestored_process_state() {
        for kind in [Kind::Memfd, Kind::Truncate] {
            for (field, value) in [
                (3, 9),
                (4, 8),
                (5, 6),
                (6, 12),
                (7, 0),
                (9, 10),
                (10, 0),
                (11, 1),
                (12, 1),
                (13, 268435456),
                (14, 0),
                (15, 0),
                (16, 1),
                (17, 2),
                (18, 0),
                (19, 1 - kind.signal()),
                (20, 1 - kind.signal()),
                (21, 0),
            ] {
                let mut data: Vec<_> = (0..20).map(|i| row(i, kind)).collect();
                data[7][field] = value;
                assert!(observations(&text(&data), kind).is_err(), "field {field}");
            }
        }
    }
    #[test]
    fn rejects_missing_rows_calls_wrong_fd_release_and_absent_unavailable_owner() {
        let kind = Kind::Truncate;
        let rows: Vec<_> = (0..19).map(|i| row(i, kind)).collect();
        assert!(observations(&text(&rows), kind).is_err());
        let good = row(0, kind);
        for data in [
            vec![0, 1, 27, 9, 1, 0, 1, 1, 1, 1],
            vec![1, 1, 27, 9, 0, 0, 1, 1, 1, 1],
            vec![1, 1, 27, u64::MAX, 1, 0, 1, 1, 1, 1],
        ] {
            assert!(attempt(&text(&[data]), "source", kind, &good).is_err());
        }
        assert!(attempt("27\t0\t0\t0\t0\t1\t1\t1\t1", "rust", kind, &good).is_err());
        assert!(Kind::parse("mmap").is_err());
    }
}
