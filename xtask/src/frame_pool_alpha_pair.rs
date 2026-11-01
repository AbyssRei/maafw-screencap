// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Pair exact alpha transition outputs without accepting a black-only capture.
use super::*;
use std::collections::BTreeMap;

const RUST_HEADER: &str = "arm\tphase\talpha\tframe_image\tframe_known\tframe_zero\tprint_image\tprint_known\tpaint_before\tpaint_after\tframe_prior\telapsed_us\treplayed\tconversions\tclones\tnative_pitch\tcopied_bytes\traw_present";

fn collect(
    root: &Path,
    folder: &Path,
    output: &Path,
    artifacts: &mut Vec<records::Artifact>,
) -> Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if entry.path() == output
            || entry.path() == root.join("alpha-pair-validation.json")
            || entry.path() == root.join("alpha-poll-validation.json")
        {
            continue;
        }
        if kind.is_file() {
            artifacts.push(records::artifact(root, &entry.path())?);
        } else if kind.is_dir() {
            collect(root, &entry.path(), output, artifacts)?;
        } else {
            return Err("Alpha pair input contains a link or unsupported entry".into());
        }
    }
    Ok(())
}
pub(super) fn run(args: &[String], observe_poll: bool) -> Result<()> {
    let (source_path, rust_path, check_only) = match args {
        [source,rust] => (source,rust,false),
        [source,rust,flag] if flag == "--check-only" => (source,rust,true),
        _ => return Err("expected upstream-alpha frame-pool-pair <source directory> <Rust directory> [--check-only]".into()),
    };
    let source = records::external_root(Path::new(source_path))?;
    let rust = records::external_root(Path::new(rust_path))?;
    if source == rust || source.starts_with(&rust) || rust.starts_with(&source) {
        return Err("Alpha pair requires distinct non-nested source/Rust roots".into());
    }
    // Keep the historical source-only report and its strict controls unchanged.
    super::run(&[
        "frame-pool".into(),
        source_path.clone(),
        "--check-only".into(),
    ])?;
    let source_table = fs::read_to_string(source.join("observations.tsv"))?;
    let source_rows = source_table
        .lines()
        .skip(1)
        .enumerate()
        .map(|(i, l)| row(i, l))
        .collect::<Result<Vec<_>>>()?;
    let table = fs::read_to_string(rust.join("observations.tsv"))?;
    let mut lines = table.lines();
    if lines.next() != Some(RUST_HEADER) {
        return Err("Alpha pair Rust header differs".into());
    }
    let lines: Vec<_> = lines.collect();
    if lines.len() != 10 {
        return Err("Alpha pair requires ten Rust calls".into());
    }
    let owners = fs::read_to_string(rust.join("owners.tsv"))?
        .lines()
        .map(numbers)
        .collect::<Result<Vec<_>>>()?;
    if owners.len() != 2
        || owners.iter().enumerate().any(|(arm, v)| {
            v.len() != 6
                || v[0] != arm as u64
                || v[1..].contains(&0)
                || v[2] != v[4]
                || v[2] == v[3]
        })
    {
        return Err("Alpha pair native owner/caller identity differs".into());
    }
    if owners[0][3] != owners[1][3] || owners[0][5] != owners[1][5] {
        return Err("Alpha arms do not share one caller/process".into());
    }
    let mut observations = Vec::new();
    let mut paired_calls = 0;
    let (mut raw_count, mut replay_count) = (0, 0);
    for (index, line) in lines.iter().enumerate() {
        let values = numbers(line)?;
        if values.len() != 18 {
            return Err("Alpha pair Rust row width differs".into());
        }
        let row = row(
            index,
            &line.split('\t').take(12).collect::<Vec<_>>().join("\t"),
        )?;
        if values[12] > 1
            || values[13] > 1
            || values[14] != 1
            || values[17] > 1
            || values[12] + values[13] != 1
            || values[13] != values[17]
            || values[15] < if values[17] == 1 { 384 } else { 0 }
            || values[16] != if values[17] == 1 { 24576 } else { 0 }
            || values[17] == 0 && values[15] != 0
            || row[0] == 0 && values[12] != 0
        {
            return Err("Alpha pair replay/conversion/clone/native-copy evidence differs".into());
        }
        let name = format!(
            "{}-{}",
            if row[0] == 0 { "fresh" } else { "persistent" },
            row[1]
        );
        let native = source.join(&name);
        let current = rust.join(&name);
        let original = fs::read(native.join("source.bgr"))?;
        let original_held = fs::read(source.join(format!("held-frame-{index}/source.bgr")))?;
        let frame = fs::read(current.join("frame.bgr"))?;
        let held = fs::read(rust.join(format!("held-{name}-frame.bgr")))?;
        let print = fs::read(current.join("print.bgr"))?;
        let print_held = fs::read(rust.join(format!("held-{name}-print.bgr")))?;
        let expected_print = pattern(Some(row[2] == 0));
        let expected_frame = pattern(if row[4] == 1 {
            Some(row[2] == 0)
        } else if row[10] == 1 {
            Some(row[2] != 0)
        } else {
            None
        });
        if frame.len() != 18432
            || frame != expected_frame
            || frame != held
            || original != original_held
            || print != expected_print
            || print != print_held
            || print != fs::read(native.join("print/source.bgr"))?
        {
            return Err(
                "Alpha sequence pixels, classification or retained owner is invalid".into(),
            );
        }
        let paired_pixels = frame == original
            && row[3..8] == source_rows[index][3..8]
            && row[10] == source_rows[index][10];
        for path in [
            current.join("frame-layout.tsv"),
            current.join("print-layout.tsv"),
            rust.join(format!("held-{name}-frame-layout.tsv")),
            rust.join(format!("held-{name}-print-layout.tsv")),
        ] {
            if fs::read_to_string(path)? != "96\t64\t288\t18432\n" {
                return Err("Alpha pair CPU layout differs".into());
            }
        }
        for path in [
            current.join("frame-error.txt"),
            current.join("print-error.txt"),
        ] {
            if fs::read(path)? != b"None\n" {
                return Err("Alpha pair accepted a native/conversion error".into());
            }
        }
        let state = fs::read_to_string(current.join("states.tsv"))?;
        states(&state, row[2])?;
        // C++ text streams use CRLF on Windows; Rust records use LF. Compare
        // the already validated labels and numeric lines, preserving all bits.
        let paired_state = state
            .lines()
            .eq(fs::read_to_string(native.join("states.tsv"))?.lines());
        paired_calls += usize::from(paired_pixels && paired_state);
        let raw_path = current.join("frame.raw");
        let held_raw = rust.join(format!("held-{name}.raw"));
        let mut alpha_values = Vec::new();
        if values[17] == 1 {
            let raw = fs::read(raw_path)?;
            if raw.len() != 24576
                || raw != fs::read(held_raw)?
                || raw
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(frame.as_chunks::<3>().0)
                    .any(|(r, b)| r[..3] != *b)
            {
                return Err(
                    "Alpha pair raw/BGR channel conversion or held raw bytes differ".into(),
                );
            }
            alpha_values = raw.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
            alpha_values.sort_unstable();
            alpha_values.dedup();
            raw_count += 1;
        } else if raw_path.exists() || held_raw.exists() {
            return Err("Alpha pair replay has a substituted raw sample".into());
        }
        replay_count += values[12];
        observations.push(serde_json::json!({"arm":row[0],"phase":row[1],"alpha":row[2],"source":source_rows[index],"rust":values,"paired_pixels":paired_pixels,"paired_window_state":paired_state,"raw_alpha_values":alpha_values,"source_cache_branch_proved":false}));
    }
    let mut summary = BTreeMap::new();
    for line in fs::read_to_string(rust.join("summary.tsv"))?.lines() {
        let (key, value) = line.split_once('\t').ok_or("Bad alpha pair summary")?;
        if summary
            .insert(key.to_owned(), value.parse::<u64>()?)
            .is_some()
        {
            return Err("Duplicate alpha pair summary field".into());
        }
    }
    let expected = BTreeMap::from([
        ("schema", 1),
        ("phases", 10),
        ("frame_images", 10),
        ("print_images", 10),
        ("held_images", 20),
        ("closed_windows", 2),
        ("dispatcher_completed", 1),
        ("positive_controls", 1),
        ("raw_frames", raw_count),
        ("replayed_frames", replay_count),
        ("context_owners", 0),
    ])
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    if summary != expected {
        return Err("Alpha pair summary/owner/positive-control counts differ".into());
    }
    let output = rust.join(if observe_poll {
        "alpha-poll-validation.json"
    } else {
        "alpha-pair-validation.json"
    });
    let (mut source_inputs, mut rust_inputs) = (Vec::new(), Vec::new());
    collect(&source, &source, &output, &mut source_inputs)?;
    collect(&rust, &rust, &output, &mut rust_inputs)?;
    source_inputs.sort_by(|a, b| a.path.cmp(&b.path));
    rust_inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let mut report = serde_json::json!({"layout_version":1,"observations":observations,"paired_calls":paired_calls,"exact_sequence_match":paired_calls==10,"owners":owners,"summary":summary,"source_inputs":source_inputs,"rust_inputs":rust_inputs,
        "scope":"Ten fixed alpha transitions per implementation, fresh/persistent sessions, exact full target BGR comparison, independent PrintWindow current-content witnesses, raw conversion evidence and CPU bytes retained past source/window/queue closure. A complete valid record with any paired pixel/state mismatch is saved but remains a failed command. Opaque frames remain strict known-color controls; alpha0 black/prior classifications remain observations, not black-only capture acceptance. Source cache branch, frame freshness, arbitrary transparency/format/failure support, old pseudo failure reclassification and complete backend/P1 acceptance are not proved."});
    if observe_poll {
        report["polling_observation"] = super::poll::check(&source, &rust)?;
        report["scope"] = "Independent forwarded original drain/poll/cache observation with the full strict alpha pair. Complete valid mismatches remain failed and are saved. Interposition can affect scheduling; it does not reinterpret unobserved old source branches, prove freshness or a historical cause, replace old failed pseudo controls, or accept complete backend/P1.".into();
    }
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved alpha pair report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    if paired_calls != 10 {
        return Err(format!("Alpha pair exact sequence differs: {paired_calls}/10 calls match; complete valid observations retained, original failures unchanged").into());
    }
    if observe_poll {
        println!(
            "FramePool alpha polling phases=20 source_calls=10 rust_calls=10 source_branch_observed=1 capture_acceptance=0 passed"
        );
    }
    println!(
        "FramePool alpha paired phases=20 opaque_known=12 content_witnesses=20 retained_owners=30 bytes_verified=1 source_oracle_replaced=0 capture_acceptance=0 passed"
    );
    Ok(())
}
