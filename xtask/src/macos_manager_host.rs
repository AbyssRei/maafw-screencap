// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read controlled manager host records; no native input/capture acceptance.
use crate::{Result, records};
use std::{fs, path::Path};

fn cases() -> Vec<Vec<u64>> {
    let mut cases = Vec::new();
    for method in [0, 1, 2, u64::MAX] {
        for input in [0, 1, 2, 3, u64::MAX] {
            for available in 0..4 {
                for failure in 0..16 {
                    for features in [[0, u64::MAX], [1, 2], [4, 3]] {
                        for outcomes in [[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 1, 2], [1, 0, 3]] {
                            let mut row = vec![
                                method,
                                input,
                                if cases.len().is_multiple_of(2) {
                                    0
                                } else {
                                    u64::from(u32::MAX)
                                },
                                available & 1,
                                available >> 1,
                                failure & 1,
                                (failure >> 1) & 1,
                                (failure >> 2) & 1,
                                (failure >> 3) & 1,
                            ];
                            row.extend(features);
                            row.extend(outcomes);
                            cases.push(row);
                        }
                    }
                }
            }
        }
    }
    cases
}
struct Expected {
    values: Vec<u64>,
    events: Vec<[u64; 4]>,
    capture: u64,
    input: u64,
    token: u64,
}
impl Expected {
    fn event(&mut self, kind: u64, unit: u64, phase: usize, call: u64) {
        self.events.push([kind, unit, phase as u64, call]);
    }
    fn features(&mut self, s: &[u64], phase: usize) -> u64 {
        if self.input == 0 {
            return 0;
        }
        self.event(5, self.input, phase, self.values[24]);
        self.values[24] += 1;
        s[9 + phase] & 3
    }
    fn capture(&mut self, s: &[u64], phase: usize, slot: usize) {
        if self.capture != 0 {
            let call = self.values[23];
            self.values[23] += 1;
            self.event(2, self.capture, phase, call);
            self.values[13 + slot] = match s[11 + call as usize] {
                0 => 0,
                1 => {
                    self.token = 20 + call;
                    1
                }
                2 => {
                    self.token = 0;
                    1
                }
                3 => 2,
                _ => unreachable!(),
            };
        }
        self.values[17 + slot] = self.token;
    }
}
fn expected(s: &[u64]) -> Vec<u64> {
    let mut e = Expected {
        values: vec![0; 155],
        events: Vec::new(),
        capture: 0,
        input: 0,
        token: 99,
    };
    e.capture(s, 0, 0);
    for phase in 0..2 {
        let mut status = 1;
        if s[0] == 1 {
            if s[3 + phase] == 0 {
                status = 0;
            } else {
                e.values[21] += 1;
                let id = e.values[21];
                e.event(1, id, phase, 0);
                if s[5 + phase] != 0 {
                    status = 2;
                } else {
                    if e.capture != 0 {
                        e.event(3, e.capture, phase, 0);
                    }
                    e.capture = id;
                }
            }
        }
        if status == 1 && matches!(s[1], 1 | 2) {
            e.values[22] += 1;
            let id = e.values[22];
            e.event(4, id, phase, 0);
            if s[7 + phase] != 0 {
                status = 2;
            } else {
                if e.input != 0 {
                    e.event(6, e.input, phase, 0);
                }
                e.input = id;
            }
        }
        e.values[phase] = status;
        e.values[3 + phase] = u64::from(status == 1);
        e.values[5 + phase] = e.capture;
        e.values[7 + phase] = e.input;
        let features = e.features(s, phase);
        e.values[if phase == 0 { 10 } else { 12 }] = features;
        e.capture(s, phase, if phase == 0 { 1 } else { 3 });
        if phase == 0 {
            let features = e.features(s, phase);
            e.values[11] = features;
            e.capture(s, phase, 2);
        }
    }
    if e.capture != 0 {
        e.event(3, e.capture, 1, 0);
    }
    if e.input != 0 {
        e.event(6, e.input, 1, 0);
    }
    e.values[25] = e.events.len() as u64;
    for (i, event) in e.events.iter().enumerate() {
        e.values[27 + i * 4..31 + i * 4].copy_from_slice(event);
    }
    e.values
}
fn validate_row(script: &[u64], values: &[u64], info: &str) -> Result<()> {
    if script.len() != 14 || values.len() != 155 {
        return Err("manager host record width differs".into());
    }
    for (i, &n) in script.iter().enumerate() {
        if !matches!(i, 0 | 1 | 9 | 10) && u32::try_from(n).is_err() {
            return Err("manager host script exceeds native UINT width".into());
        }
    }
    if script[3..9].iter().any(|&n| n > 1)
        || script[11..].iter().any(|&n| n > 3)
        || values
            .iter()
            .enumerate()
            .any(|(i, &n)| !(9..13).contains(&i) && u32::try_from(n).is_err())
        || values != expected(script)
    {
        return Err("manager host state, native widths, events or retained owners differ".into());
    }
    let signed = |v: u64| i64::from_ne_bytes(v.to_ne_bytes());
    let expected = format!(
        "{{\"input_method\":{},\"screencap_method\":{},\"type\":\"macos\",\"window_id\":{}}}",
        signed(script[1]),
        signed(script[0]),
        script[2]
    );
    if info != expected {
        return Err("manager host metadata selector/identity differs".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-macos-manager-host <external directory> [--check-only]".into(),
            );
        }
    };
    let root = records::external_root(Path::new(path))?;
    let source = fs::read_to_string(root.join("source.tsv"))?;
    if source != fs::read_to_string(root.join("rust.tsv"))? {
        return Err("source/Rust host records differ".into());
    }
    let matrix = cases();
    let lines = source.lines().collect::<Vec<_>>();
    if matrix.len() != 19200 || lines.len() != matrix.len() {
        return Err("manager host matrix incomplete".into());
    }
    let (mut calls, mut queries) = (0_u64, 0_u64);
    for (case, (line, script)) in lines.iter().zip(&matrix).enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 4 || fields[0] != case.to_string() {
            return Err("manager host case order/fields differ".into());
        }
        let actual: Vec<u64> = serde_json::from_str(fields[1])?;
        if actual != *script {
            return Err("manager host case selection differs".into());
        }
        let values: Vec<u64> = serde_json::from_str(fields[2])?;
        validate_row(script, &values, fields[3])?;
        calls = calls
            .checked_add(values[23])
            .ok_or("capture count overflow")?;
        queries = queries
            .checked_add(values[24])
            .ok_or("query count overflow")?;
    }
    let summary = format!(
        "schema=1\ncases=19200\ncontrolled_source_capture_calls={calls}\nsource_feature_queries={queries}\npaired_reports=38400\ninput_actions=0\nnative_screenshots=0\njoined_reader=1\n"
    );
    if fs::read_to_string(root.join("summary.txt"))? != summary {
        return Err("manager host summary differs".into());
    }
    let libraries = [
        "mfs_macos_manager_queries.dll",
        "libmfs_macos_manager_queries.so",
        "libmfs_macos_manager_queries.dylib",
    ]
    .into_iter()
    .filter(|n| root.join(n).is_file())
    .collect::<Vec<_>>();
    if libraries.len() != 1 {
        return Err("expected one actual manager model library".into());
    }
    let names = ["source.tsv", "rust.tsv", "summary.txt", libraries[0]];
    let mut entries = fs::read_dir(&root)?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.retain(|n| n != "manager-host-validation.json");
    entries.sort();
    let mut expected_names = names.to_vec();
    expected_names.sort();
    if entries != expected_names {
        return Err("unexpected or missing manager host files".into());
    }
    let inputs = names
        .into_iter()
        .map(|n| records::artifact(&root, &root.join(n)))
        .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"cases":19200,"paired_reports":38400,
        "controlled_source_capture_calls":calls,"source_feature_queries":queries,"inputs":inputs,
        "input_actions":0,"native_screenshots":0,"full_P1_acceptance":false,
        "scope":"Exact fixed manager bodies with explicit availability, capture and host input-owner models; full events, partial construction, dynamic masked features, metadata and output retention. No real macOS input construction, screenshot, native availability failure, arbitrary allocator exhaustion, callback drain or full manager/P1 acceptance."});
    let output = root.join("manager-host-validation.json");
    if check_only {
        if output.exists() && records::read::<serde_json::Value>(&output)? != report {
            return Err("saved manager host report differs".into());
        }
    } else {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS manager host cases=19200 paired_reports=38400 exact_events=1 input_actions=0 native_screenshots=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_matched_record_changes_to_native_widths_and_cleanup() {
        let s = [0, 0, 7, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1];
        let v = expected(&s);
        let info = "{\"input_method\":0,\"screencap_method\":0,\"type\":\"macos\",\"window_id\":7}";
        validate_row(&s, &v, info).unwrap();
        for index in [0, 2, 5, 7, 9, 13, 17, 21, 22, 23, 24, 25, 26, 27, 154] {
            let mut bad = v.clone();
            bad[index] = 1_u64 << 32;
            assert!(validate_row(&s, &bad, info).is_err(), "field {index}");
        }
        let mut bad = s;
        bad[2] = 1_u64 << 32;
        assert!(validate_row(&bad, &v, info).is_err());
        assert!(validate_row(&s, &v, info.trim_end_matches('}')).is_err());
    }
    #[test]
    fn requires_live_old_input_after_failed_replacement_and_capture_first_drop() {
        let s = [1, 1, 7, 1, 1, 0, 0, 0, 1, u64::MAX, 2, 1, 0, 3];
        let v = expected(&s);
        assert_eq!(&v[0..2], &[1, 2]);
        assert_eq!(&v[5..9], &[1, 2, 1, 1]);
        assert_eq!(&v[9..13], &[0, 3, 3, 2]);
        let info = "{\"input_method\":1,\"screencap_method\":1,\"type\":\"macos\",\"window_id\":7}";
        validate_row(&s, &v, info).unwrap();
        let end = 27 + usize::try_from(v[25]).unwrap() * 4;
        assert_eq!(&v[end - 8..end], &[3, 2, 1, 0, 6, 1, 1, 0]);
        let mut bad = v.clone();
        bad[end - 8..end - 4].copy_from_slice(&v[end - 4..end]);
        bad[end - 4..end].copy_from_slice(&v[end - 8..end - 4]);
        assert!(validate_row(&s, &bad, info).is_err());
        bad = v.clone();
        bad[12] = 3;
        assert!(validate_row(&s, &bad, info).is_err());
    }
}
