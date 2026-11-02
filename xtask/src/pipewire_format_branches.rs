// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Integrity of actual native parser observations; strict Rust parity is separate.
use crate::{Result, records};
use std::{fs, path::Path};

const SOURCE: &str = "index\tfirst\thas_second\tsecond\tsecond_choice\tmedia_result\tmedia_type\tmedia_subtype\tnative_result\tnative_width\tnative_height\tnative_format\tsource_width\tsource_height\tsource_format\tsource_accepted";
const RUST: &str = "index\trust_accepted\trust_width\trust_height\trust_format";

fn rows(text: &str, header: &str, columns: usize) -> Result<Vec<Vec<i64>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("parser branch record header differs".into());
    }
    lines
        .map(|line| {
            let row: Vec<i64> = line
                .split('\t')
                .map(str::parse)
                .collect::<std::result::Result<_, _>>()?;
            if row.len() != columns {
                return Err("parser branch record width differs".into());
            }
            Ok(row)
        })
        .collect()
}

fn validate(ids: &[i64], source: &[Vec<i64>], rust: &[Vec<i64>]) -> Result<Vec<usize>> {
    let [bgra, nv12, unknown, video, raw] = <[i64; 5]>::try_from(ids)?;
    if bgra <= 0 || nv12 <= 0 || bgra == nv12 || unknown != 0 || video <= 0 || raw <= 0 {
        return Err("parser branch native IDs differ".into());
    }
    u32::try_from(bgra)?;
    u32::try_from(nv12)?;
    u32::try_from(video)?;
    u32::try_from(raw)?;
    if source.len() != 8 || rust.len() != 8 {
        return Err("parser branch matrix must contain eight actual cases per side".into());
    }
    let specifications = [
        [bgra, 0, 0, 0],
        [nv12, 0, 0, 0],
        [0, 1, bgra, 0],
        [nv12, 1, bgra, 0],
        [nv12, 1, bgra, 1],
        [bgra, 1, nv12, 0],
        [0, 1, bgra, 1],
        [bgra, 1, nv12, 1],
    ];
    let mut differences = Vec::new();
    for (index, ((source, rust), spec)) in source.iter().zip(rust).zip(specifications).enumerate() {
        let format = if spec[1] == 1 { spec[2] } else { spec[0] };
        let accepted = i64::from(format == bgra);
        if source.len() != 16
            || rust.len() != 5
            || source[0] != i64::try_from(index)?
            || rust[0] != source[0]
            || source[1..5] != spec
            || source[5] <= 0
            || source[6..8] != [video, raw]
            || source[8] <= 0
            || source[9..12] != [8, 3, spec[0]]
            || source[12..16] != [8, 3, format, accepted]
        {
            return Err("original native parser branch premise differs".into());
        }
        if rust[1..] != [0, 0, 0, 0] && rust[1..] != [1, 8, 3, bgra] {
            return Err("Rust native parser branch result is outside the actual fixture".into());
        }
        if rust[1] != accepted {
            differences.push(index);
        }
    }
    Ok(differences)
}

pub fn run(args: &[String]) -> Result<()> {
    let (root, check_only) = match args {
        [root] => (Path::new(root), false),
        [root, flag] if flag == "--check-only" => (Path::new(root), true),
        _ => return Err("expected pipewire-format-branches <matrix> [--check-only]".into()),
    };
    let id_rows = rows(
        &fs::read_to_string(root.join("source/ids.tsv"))?,
        "bgra\tnv12\tunknown\tvideo\traw",
        5,
    )?;
    if id_rows.len() != 1 {
        return Err("expected one native parser ID row".into());
    }
    let source = rows(
        &fs::read_to_string(root.join("source/cases.tsv"))?,
        SOURCE,
        16,
    )?;
    let rust = rows(&fs::read_to_string(root.join("rust.tsv"))?, RUST, 5)?;
    let differences = validate(&id_rows[0], &source, &rust)?;
    let mut pods = Vec::new();
    for index in 0..8 {
        let path = root.join(format!("source/case-{index}.pod"));
        let bytes = fs::read(&path)?;
        if !(16..=4096).contains(&bytes.len())
            || usize::try_from(u32::from_le_bytes(bytes[..4].try_into()?))?.checked_add(8)
                != Some(bytes.len())
        {
            return Err("native parser POD byte span differs".into());
        }
        pods.push(records::artifact(root, &path)?);
    }
    let report = serde_json::json!({"layout_version":2,"cases":8,"native_first_property":8,
        "positive_media_results":8,"positive_raw_results":8,"source_fallback_last_property":8,
        "media_parser_results":source.iter().map(|r|r[5]).collect::<Vec<_>>(),
        "raw_parser_results":source.iter().map(|r|r[8]).collect::<Vec<_>>(),
        "source_accepted":5,"source_unsupported":3,
        "source_rust_difference_cases":differences,"parser_parity_accepted":differences.is_empty(),
        "diagnostic_only":true,"capture_performed":false,"full_P1_accepted":false,"native_pods":pods,
        "source_ids":records::artifact(root,&root.join("source/ids.tsv"))?,
        "source_records":records::artifact(root,&root.join("source/cases.tsv"))?,
        "rust_records":records::artifact(root,&root.join("rust.tsv"))?,
        "scope":"Actual positive SPA media/raw parse results, first-property standard observations and unchanged original manual fallback selecting the final property. Input spans and eight premises are verified. Strict Rust equality is separate; diagnostic integrity may retain differences. No capture, adversarial POD safety or P1 acceptance. Historical schema1 reports use their archived validator and are not rewritten."});
    let output = root.join("parser-branches-validation.json");
    if check_only {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved parser branch report differs".into());
        }
    } else {
        records::create_json(&output, &report)?;
    }
    println!(
        "PipeWire parser branch records cases=8 source_rust_differences={} diagnostic_only=1 capture=0 full_P1_acceptance=0 verified",
        differences.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
        let specs = [
            [9, 0, 0, 0],
            [10, 0, 0, 0],
            [0, 1, 9, 0],
            [10, 1, 9, 0],
            [10, 1, 9, 1],
            [9, 1, 10, 0],
            [0, 1, 9, 1],
            [9, 1, 10, 1],
        ];
        let mut source = Vec::new();
        let mut rust = Vec::new();
        for (i, spec) in specs.into_iter().enumerate() {
            let last = if spec[1] == 1 { spec[2] } else { spec[0] };
            let accepted = i64::from(last == 9);
            let mut row = vec![i64::try_from(i).unwrap()];
            row.extend(spec);
            row.extend([2, 2, 1, 2, 8, 3, spec[0], 8, 3, last, accepted]);
            source.push(row);
            rust.push(if accepted == 1 {
                vec![i as i64, 1, 8, 3, 9]
            } else {
                vec![i as i64, 0, 0, 0, 0]
            });
        }
        (source, rust)
    }

    #[test]
    fn keeps_observed_differences_without_accepting_parser_parity() {
        let (source, mut rust) = fixture();
        assert!(
            validate(&[9, 10, 0, 2, 1], &source, &rust)
                .unwrap()
                .is_empty()
        );
        for index in [5, 7] {
            rust[index] = vec![index as i64, 1, 8, 3, 9];
        }
        assert_eq!(validate(&[9, 10, 0, 2, 1], &source, &rust).unwrap(), [5, 7]);
    }

    #[test]
    fn rejects_missing_cases_changed_source_branches_and_false_rust_results() {
        let (source, rust) = fixture();
        assert!(validate(&[9, 10, 0, 2, 1], &source[..7], &rust).is_err());
        let mut bad = source.clone();
        bad[3][15] = 0;
        assert!(validate(&[9, 10, 0, 2, 1], &bad, &rust).is_err());
        let mut bad = source.clone();
        bad[0][8] = 0;
        assert!(validate(&[9, 10, 0, 2, 1], &bad, &rust).is_err());
        let mut bad = source.clone();
        bad[0][5] = 0;
        assert!(validate(&[9, 10, 0, 2, 1], &bad, &rust).is_err());
        let mut bad = rust.clone();
        bad[0][4] = 10;
        assert!(validate(&[9, 10, 0, 2, 1], &source, &bad).is_err());
        assert!(validate(&[9, 9, 0, 2, 1], &source, &rust).is_err());
    }
}
