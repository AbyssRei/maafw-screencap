// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual first-display capture: independently known owned patch, full-image
//! conversion/retention, real native geometry and callback ownership witnesses.
use crate::{
    Result,
    macos_display_geometry::{self, Geometry},
    records,
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scene {
    window_id: u32,
    width: u32,
    height: u32,
    colour: String,
    owner_pid: u32,
    display_patch: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalStatus {
    original_oracle_passed: bool,
    expected_source_files: u32,
    recorded_source_files: u32,
}
pub fn run(args: &[String]) -> Result<()> {
    let (path,check_only)=match args {
        [path]=>(path,false),[path,flag] if flag=="--check-only"=>(path,true),
        _=>return Err("expected upstream-trace macos-display-patch <actual external directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let scene: Scene = serde_json::from_slice(&read("scene/ready.json")?)?;
    if scene.width != 47
        || scene.height != 31
        || scene.colour != "green"
        || scene.display_patch != 1
    {
        return Err("wrong owned display scene premise".into());
    }
    let original: OriginalStatus = serde_json::from_slice(&read("source-oracle-status.json")?)?;
    original_status(original)?;
    let geometry_bytes = read("scene/display-geometry.tsv")?;
    let g = macos_display_geometry::parse(std::str::from_utf8(&geometry_bytes)?)?;
    if g.window != scene.window_id || g.pid != scene.owner_pid {
        return Err("native first-display geometry substituted the owned window/process".into());
    }
    for name in ["source-display-geometry.tsv", "rust-display-geometry.tsv"] {
        if read(name)? != geometry_bytes {
            return Err("source/Rust native first-display identities differ".into());
        }
    }
    if read("scene/stopped.txt")? != b"window_closed=1\n" {
        return Err("display scene did not acknowledge native window release".into());
    }
    traces(std::str::from_utf8(&read("rust-display-trace.tsv")?)?, &g)?;
    for call in 0..3 {
        returned(std::str::from_utf8(&read(&format!(
            "display-call-{call}.rust-status.txt"
        ))?)?)?;
        for side in ["source", "rust"] {
            let bgr = read(&format!("display-call-{call}.{side}.bgr"))?;
            g.pixels(&bgr, 3)?;
            if read(&format!("display-held-{call}.{side}.bgr"))? != bgr {
                return Err("full-display BGR owner changed after capture closure".into());
            }
            if side == "rust" {
                let raw = read(&format!("display-call-{call}.rust.raw"))?;
                g.pixels(&raw, 4)?;
                if read(&format!("display-held-{call}.rust.raw"))? != raw {
                    return Err(
                        "full-display raw CPU owner changed after scene/native shutdown".into(),
                    );
                }
                let (bgra, raw_tail) = raw.as_chunks::<4>();
                let (bgr, bgr_tail) = bgr.as_chunks::<3>();
                if !raw_tail.is_empty() || !bgr_tail.is_empty() || bgra.len() != bgr.len() {
                    return Err("full-image native channel spans differ".into());
                }
                for (bgra, bgr) in bgra.iter().zip(bgr) {
                    if bgra[..3] != *bgr {
                        return Err("full-image BGRA-to-BGR conversion differs".into());
                    }
                }
            }
        }
    }
    let report = serde_json::json!({"layout_version":1,"source_captures":3,"rust_captures":3,"payloads":18,
        "original_first_display_id":g.display,"owned_window_id":g.window,"owned_pid":g.pid,"width":g.width,"height":g.height,
        "known_owned_patch_region":g.region,"artifacts":artifacts,
        "scope":"Actual unchanged original first-enumerated-display capture and same migrated Rust path, original display width/height configuration preserved. Separate AppKit owner places a 47x31 opaque green window on that actual display using matching NSScreen identity. Native SCDisplay/SCWindow point frames, real pid/window IDs and unchanged source observations fix the interior pixel region. All six source/Rust captures assert the known green patch; complete Rust BGRA-to-BGR conversion and all eighteen immediate/held files are checked. Rust callback/lock/native-input owners end before explicit source semaphore fixture reclamation and acknowledged scene shutdown; CPU owners are read on a joined thread afterward. Other full-desktop pixels are not prescribed or compared across different capture times. Not zero/multiple-display, mixed-scale/Retina, complete format/resource/late-callback coverage or full macOS/P1 acceptance."});
    let output = root.join("display-patch-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved macOS display patch report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS display_patch source_captures=3 rust_captures=3 payloads=18 actual_native_geometry=1 known_owned_patch=6 full_image_rust_conversion=3 retained_cpu_verified=1 full_desktop_known_pixels=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
fn original_status(value: OriginalStatus) -> Result<()> {
    if !value.original_oracle_passed
        || value.expected_source_files != 7
        || value.recorded_source_files != 7
    {
        return Err("original display oracle failed/incomplete; independent Rust observations cannot accept this pair".into());
    }
    Ok(())
}
fn returned(text: &str) -> Result<()> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("invalid native display return snapshot")?;
        if fields.insert(key, value).is_some() {
            return Err("duplicate native display return field".into());
        }
    }
    let expected = BTreeMap::from([
        ("result", "1"),
        ("timed_out", "0"),
        ("content_calls", "1"),
        ("sample_calls", "1"),
        ("completed", "1"),
        ("source_failure", "0"),
        ("native_error", "0"),
        ("fault", "0"),
        ("wait_result", "0"),
        ("error_debug", "None"),
    ]);
    let elapsed = fields
        .remove("elapsed_ns")
        .ok_or("missing native display elapsed time")?;
    if elapsed.is_empty()
        || !elapsed.bytes().all(|b| b.is_ascii_digit())
        || elapsed.parse::<u64>()? == 0
        || fields != expected
    {
        return Err(
            "native display returned None/error/timeout or has incomplete callback evidence".into(),
        );
    }
    Ok(())
}
fn traces(text: &str, g: &Geometry) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 3 {
        return Err("incomplete native display callback records".into());
    }
    for (call, row) in rows.iter().enumerate() {
        let v = row
            .split('\t')
            .map(|value| -> Result<u64> {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid display callback decimal".into());
                }
                Ok(value.parse()?)
            })
            .collect::<Result<Vec<_>>>()?;
        if v.len() != 17
            || v[0] != u64::try_from(call)?
            || v[1..9] != [1; 8]
            || v[9] != u64::try_from(g.span(4)?)?
            || v[10..14] != [1, 1, 0, 0]
            || v[14] < u64::try_from(g.width.checked_mul(4).ok_or("display stride overflow")?)?
            || v[15] != u64::try_from(g.width)?
            || v[16] != u64::try_from(g.height)?
        {
            return Err(
                "native display callback/lock/retained source reference evidence differs".into(),
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_callbacks_unreleased_inputs_wrong_spans_and_cleanup_relabeling() {
        let g = macos_display_geometry::parse(
            "1\t17\t640\t480\t0\t0\t640\t480\t91\t7\t100\t100\t47\t31\t1\t2\n",
        )
        .unwrap();
        let rows = (0..3)
            .map(|i| format!("{i}\t1\t1\t1\t1\t1\t1\t1\t1\t1228800\t1\t1\t0\t0\t2560\t640\t480\n"))
            .collect::<String>();
        traces(&rows, &g).unwrap();
        assert!(
            original_status(OriginalStatus {
                original_oracle_passed: false,
                expected_source_files: 7,
                recorded_source_files: 1
            })
            .is_err()
        );
        assert!(
            original_status(OriginalStatus {
                original_oracle_passed: false,
                expected_source_files: 7,
                recorded_source_files: 7
            })
            .is_err()
        );
        assert!(
            original_status(OriginalStatus {
                original_oracle_passed: true,
                expected_source_files: 7,
                recorded_source_files: 1
            })
            .is_err()
        );
        original_status(OriginalStatus {
            original_oracle_passed: true,
            expected_source_files: 7,
            recorded_source_files: 7,
        })
        .unwrap();
        let returned_ok = "result=1\nelapsed_ns=7\ntimed_out=0\ncontent_calls=1\nsample_calls=1\ncompleted=1\nsource_failure=0\nnative_error=0\nfault=0\nwait_result=0\nerror_debug=None\n";
        returned(returned_ok).unwrap();
        assert!(returned(&returned_ok.replace("result=1", "result=0")).is_err());
        assert!(returned(&returned_ok.replace("timed_out=0", "timed_out=1")).is_err());
        assert!(returned(&(returned_ok.to_owned() + "result=1\n")).is_err());
        for bad in [
            rows.lines().take(2).collect::<Vec<_>>().join("\n"),
            rows.replacen("\t1\t1\t0\t0\t2560", "\t1\t0\t0\t0\t2560", 1),
            rows.replacen("1228800", "1228799", 1),
            rows.replacen("\t1\t1\t1\t1", "\t1\t0\t1\t1", 1),
        ] {
            assert!(traces(&bad, &g).is_err());
        }
    }
}
