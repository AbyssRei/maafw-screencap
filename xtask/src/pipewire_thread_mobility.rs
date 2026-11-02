// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Finite source/Rust thread-handoff records, not general Send/Sync acceptance.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const SOURCE: &str = "cycle\tcreator_tid\tcapture_tid\tcloser_tid\tloop_tid\tloop_started\tfd_before\tfd_after\ttasks_before\ttasks_after\toffered_fd\toffered_fd_closed\tcaptures\tretained_frames";
const EXTRA: &str = "\tpanic_caught\tcopies\tdequeued\treturned\tqueue_error\tmap_error\tunmap_error\tfault\tcore_disconnect_error";

fn table(text: &str, header: &str, columns: usize) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("thread mobility table header differs".into());
    }
    lines
        .map(|line| {
            let row: Vec<i64> = line
                .split('\t')
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()?;
            if row.len() != columns {
                return Err("thread mobility table width differs".into());
            }
            Ok(row)
        })
        .collect()
}
fn ids(text: &str) -> Result<BTreeSet<i64>> {
    let rows = table(text, "id", 1)?;
    let values: BTreeSet<_> = rows.iter().map(|r| r[0]).collect();
    if values.is_empty()
        || values.len() != rows.len()
        || values
            .iter()
            .any(|&id| id <= 0 || u32::try_from(id).is_err())
    {
        return Err("mobility baseline/final task IDs differ".into());
    }
    Ok(values)
}
fn validate(
    row: &[i64],
    cycle: usize,
    rust: bool,
    mode: usize,
    baseline: &BTreeSet<i64>,
) -> Result<BTreeSet<i64>> {
    if row.len() != (if rust { 23 } else { 14 }) || row[0] != i64::try_from(cycle)? {
        return Err("mobility cycle/column count differs".into());
    }
    let callers: BTreeSet<_> = row[1..5].iter().copied().collect();
    if callers.len() != 4
        || callers
            .iter()
            .any(|&id| id <= 0 || u32::try_from(id).is_err())
        || !callers.is_disjoint(baseline)
        || row[5] <= 0
        || row[6] <= 0
        || row[6] != row[7]
        || row[8] != i64::try_from(baseline.len())?
        || row[9] != row[8]
        || row[12..14] != [3, 3]
        || (mode == 0 && row[10..12] != [-1, 0])
        || (mode == 1 && (row[10] < 0 || row[11] != 1))
        || i32::try_from(row[10]).is_err()
    {
        return Err("mobility identities, ownership or resource counts differ".into());
    }
    if rust
        && (row[14] != 1
            || row[15] != 4
            || row[16] < 4
            || row[16] != row[17]
            || row[18..23] != [0; 5])
    {
        return Err("Rust mobility unwind/copy/return/error evidence differs".into());
    }
    Ok(callers)
}
fn live(text: &str, expected: &BTreeSet<i64>, loop_id: i64, started: i64) -> Result<()> {
    let mut lines = text.lines();
    if lines.next() != Some("id\tname\tstarted\tflags\taddress_space") {
        return Err("mobility live-task header differs".into());
    }
    let mut observed = BTreeSet::new();
    for line in lines {
        let values: Vec<_> = line.split('\t').collect();
        if values.len() != 5 || values[1] != "MaaScreencap" {
            return Err("unknown mobility task kind".into());
        }
        let id: i64 = values[0].parse()?;
        let begin: i64 = values[2].parse()?;
        let flags: u64 = values[3].parse()?;
        let space: u64 = values[4].parse()?;
        if !observed.insert(id)
            || begin <= 0
            || flags & 4 != 0
            || space == 0
            || (id == loop_id && begin != started)
        {
            return Err("mobility task generation was not live".into());
        }
    }
    if &observed != expected {
        return Err("mobility task population differs".into());
    }
    Ok(())
}
fn pixels(bytes: &[u8], bgr: bool) -> Result<u64> {
    if bytes.len() != 2016 {
        return Err("mobility target image span differs".into());
    }
    let [r, g, b] = if bgr {
        [bytes[2], bytes[1], bytes[0]]
    } else {
        [bytes[0], bytes[1], bytes[2]]
    };
    let serial = (u64::from(r) << 16) | (u64::from(g) << 8) | u64::from(b);
    if serial == 0 {
        return Err("mobility image has no producer marker".into());
    }
    crate::pipewire_trace::pixels(bytes, serial, bgr)?;
    Ok(serial)
}

pub fn run(args: &[String]) -> Result<()> {
    let (root, readonly) = match args {
        [root] => (Path::new(root), false),
        [root, flag] if flag == "--check-only" => (Path::new(root), true),
        _ => return Err("expected pipewire-thread-mobility <matrix> [--check-only]".into()),
    };
    let mut inputs = Vec::new();
    let mut rows_verified = 0;
    let mut payloads = 0;
    let mut read = |path: &Path| -> Result<Vec<u8>> {
        inputs.push(records::artifact(root, path)?);
        Ok(fs::read(path)?)
    };
    for mode in 0..2 {
        for side in ["source", "rust"] {
            let rust = side == "rust";
            let directory = root.join(format!("{side}/mode-{mode}"));
            let baseline = ids(std::str::from_utf8(&read(
                &directory.join("baseline.tsv"),
            )?)?)?;
            let header = if rust {
                format!("{SOURCE}{EXTRA}")
            } else {
                SOURCE.to_owned()
            };
            let rows = table(
                std::str::from_utf8(&read(&directory.join("rows.tsv"))?)?,
                &header,
                if rust { 23 } else { 14 },
            )?;
            if rows.len() != 20 {
                return Err("mobility requires twenty complete actual cycles per side/mode".into());
            }
            for (cycle, row) in rows.iter().enumerate() {
                let callers = validate(row, cycle, rust, mode, &baseline)?;
                live(
                    std::str::from_utf8(&read(
                        &directory.join(format!("cycle-{cycle}-live.tsv")),
                    )?)?,
                    &callers,
                    row[4],
                    row[5],
                )?;
                if ids(std::str::from_utf8(&read(
                    &directory.join(format!("cycle-{cycle}-final.tsv")),
                )?)?)?
                    != baseline
                {
                    return Err("mobility left a live or unknown final task".into());
                }
                for call in 0..3 {
                    let current = read(&directory.join(format!("cycle-{cycle}-call-{call}.bgr")))?;
                    let held = read(&directory.join(format!("cycle-{cycle}-held-{call}.bgr")))?;
                    let serial = pixels(&current, true)?;
                    if current != held {
                        return Err("mobility changed retained BGR pixels".into());
                    }
                    payloads += 2;
                    if rust {
                        let current =
                            read(&directory.join(format!("cycle-{cycle}-call-{call}.rgb")))?;
                        let held = read(&directory.join(format!("cycle-{cycle}-held-{call}.rgb")))?;
                        if pixels(&current, false)? != serial || current != held {
                            return Err("mobility raw/BGR/held sample differs".into());
                        }
                        payloads += 2;
                    }
                }
                rows_verified += 1;
            }
        }
    }
    let report = serde_json::json!({"layout_version":1,"cycles_per_side_mode":20,"verified_lifecycles":rows_verified,
        "distinct_live_callers_per_object":3,"same_native_loop_generation":true,
        "source_captures":120,"rust_captures":120,"rust_controlled_converter_unwinds":40,
        "current_and_held_payloads":payloads,"complete_target_pixels_verified":true,
        "records":inputs,"capture_performed_by_validator":false,"full_P1_accepted":false,
        "scope":"Finite original source-class/Rust complete-owner handoffs using daemon and successful unique FD offers, actual named caller/native task generations, foreign close and retained known CPU pixels. Rust-only converter unwinds add one raw capture per lifecycle; they are not a matched original conversion fault. Opaque Mat/loader owners stay local to each Rust caller; their mobility, general concurrent use, arbitrary setup/cleanup failures and final SDK Send/Sync remain unaccepted. Actual native execution/source identity requires its separate native checks."});
    let path = root.join("thread-mobility-validation.json");
    if readonly {
        if records::read::<serde_json::Value>(&path)? != report {
            return Err("saved mobility report differs".into());
        }
    } else {
        records::create_json(&path, &report)?;
    }
    println!(
        "PipeWire thread mobility records lifecycles=80 captures=240 payloads=720 rust_unwinds=40 full_P1_acceptance=0 verified"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(rust: bool) -> Vec<i64> {
        let mut row = vec![0, 10, 11, 12, 13, 100, 7, 7, 2, 2, -1, 0, 3, 3];
        if rust {
            row.extend([1, 4, 9, 9, 0, 0, 0, 0, 0]);
        }
        row
    }
    #[test]
    fn requires_distinct_live_handoffs_closed_fd_and_balanced_native_returns() {
        let baseline = BTreeSet::from([1, 2]);
        let source = fixture(false);
        let rust = fixture(true);
        assert!(validate(&source, 0, false, 0, &baseline).is_ok());
        assert!(validate(&rust, 0, true, 0, &baseline).is_ok());
        let mut row = rust.clone();
        row[2] = row[1];
        assert!(validate(&row, 0, true, 0, &baseline).is_err());
        let mut row = rust.clone();
        row[10] = 4;
        assert!(validate(&row, 0, true, 1, &baseline).is_err());
        row[11] = 1;
        assert!(validate(&row, 0, true, 1, &baseline).is_ok());
        row[17] = 8;
        assert!(validate(&row, 0, true, 1, &baseline).is_err());
        assert!(validate(&rust, 0, true, 0, &BTreeSet::from([10, 2])).is_err());
    }
    #[test]
    fn rejects_wrong_generation_retiring_tasks_unknown_ids_and_incomplete_images() {
        let callers = BTreeSet::from([10, 11, 12, 13]);
        let text = "id\tname\tstarted\tflags\taddress_space\n10\tMaaScreencap\t90\t0\t4096\n11\tMaaScreencap\t91\t0\t4096\n12\tMaaScreencap\t92\t0\t4096\n13\tMaaScreencap\t100\t0\t4096\n";
        assert!(live(text, &callers, 13, 100).is_ok());
        assert!(live(text, &callers, 13, 101).is_err());
        assert!(live(&text.replace("100\t0", "100\t4"), &callers, 13, 100).is_err());
        assert!(live(&text.replace("13\tMaa", "14\tMaa"), &callers, 13, 100).is_err());
        assert!(ids("id\n1\n1\n").is_err());
        assert!(pixels(&[0; 2016], true).is_err());
        assert!(pixels(&[0; 3], false).is_err());
    }
}
