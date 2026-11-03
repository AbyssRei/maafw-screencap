// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Same real native sample, separate from unchanged known-scene capture gates.
use crate::{Result, records};
use std::{fs, path::Path};
const HEADER: &str = "call\tinvoke_exit\tcallbacks\tcallback_thread\twidth\theight\tstride\tlocks\tunlocks\tcopies\tcopied_bytes\tconversions\tunlock_error";
fn row(text: &str, count: usize) -> Result<Vec<u64>> {
    let values = text
        .split('\t')
        .map(|v| -> Result<u64> {
            if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                return Err("invalid live sample scalar".into());
            }
            Ok(v.parse()?)
        })
        .collect::<Result<Vec<_>>>()?;
    if values.len() != count {
        return Err("live sample scalar count differs".into());
    }
    Ok(values)
}
fn text(root: &Path, name: &str) -> Result<String> {
    Ok(fs::read_to_string(root.join(name))?)
}
fn check(root: &Path) -> Result<serde_json::Value> {
    let summary = text(root, "summary.tsv")?;
    let lines = summary.lines().collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err("live sample summary row count differs".into());
    }
    let s = row(lines[0], 9)?;
    let native_type = text(root, "native-type.tsv")?;
    let native_type = native_type.lines().collect::<Vec<_>>();
    if native_type.len() != 1 {
        return Err("native OpenCV type query count differs".into());
    }
    let native_type = row(native_type[0], 1)?[0];
    if native_type == 0 {
        return Err("native BGR type query missing".into());
    }
    if s[0] == 0 || s[1] == 0 || s[2] == 0 || s[1] == s[2] || s[3..] != [1, 0, 0, 0, 1, 0] {
        return Err("live sample context/CPU/module ownership differs".into());
    }
    if text(root, "callback-owner.tsv")? != "1\t0\n"
        || text(root, "original-owner-closed.txt")?
            != "context_owners=0\nsource_callbacks_drained=0\n"
    {
        return Err("live sample source or shared callback owner did not end".into());
    }
    if text(root, "original-finish.txt")?
        != format!(
            "schema=1\ncalls=3\ncaller_thread={}\npid={}\nsource_object_destroyed=1\nheld_images=3\ncallback_drain_proved=0\nmodule_retained_until_process_exit=1\n",
            s[1], s[0]
        )
    {
        return Err("original source did not close before the three held CPU reads".into());
    }
    let lines = text(root, "live.tsv")?;
    let lines = lines.lines().collect::<Vec<_>>();
    if lines.len() != 4 || lines[0] != HEADER {
        return Err("live sample invocation census differs".into());
    }
    let mut total = 0u64;
    for call in 0..3 {
        let v = row(lines[call + 1], 13)?;
        let packed = v[4]
            .checked_mul(v[5])
            .and_then(|p| p.checked_mul(4))
            .ok_or("live sample size overflow")?;
        if v[0] != u64::try_from(call)?
            || v[1] != 0
            || v[2] != 1
            || v[3] == 0
            || v[4] == 0
            || v[5] == 0
            || v[4] > i32::MAX as u64
            || v[5] > i32::MAX as u64
            || v[6] < v[4].checked_mul(4).ok_or("live row overflow")?
            || v[7..10] != [1; 3]
            || v[10] != packed
            || v[11..] != [1, 0]
        {
            return Err("live sample conversion/layout/lock evidence differs".into());
        }
        let (native_thread, native, bgr) =
            crate::macos_native_sample::validate_live(root, call, s[0], s[1], [v[4], v[5], v[6]])?;
        if native_thread != v[3] {
            return Err("Rust conversion was not on the same native callback thread".into());
        }
        let call_row = text(root, &format!("original-call-{call}.tsv"))?;
        let call_lines = call_row.lines().collect::<Vec<_>>();
        if call_lines.len() != 1 {
            return Err("original call row count differs".into());
        }
        let n = row(call_lines[0], 5)?;
        if n[0] != u64::try_from(call)? || n[1] != 1 || n[2] == 0 || n[3] != s[1] || n[4] != s[0] {
            return Err("original sample was not returned on its owning caller".into());
        }
        let layouts = text(root, &format!("original-call-{call}-layout.tsv"))?;
        let layouts = layouts.lines().collect::<Vec<_>>();
        if layouts.len() != 1 {
            return Err("original layout row count differs".into());
        }
        let layout = row(layouts[0], 5)?;
        if layout[..2] != v[4..6]
            || layout[2] != v[4].checked_mul(3).ok_or("original row overflow")?
            || layout[3..] != [native_type; 2]
        {
            return Err("original returned CPU Mat layout differs".into());
        }
        for (name, expected) in [
            (format!("rust-call-{call}.bgra"), &native),
            (format!("rust-held-{call}.bgra"), &native),
            (format!("rust-call-{call}.bgr"), &bgr),
            (format!("rust-held-{call}.bgr"), &bgr),
            (format!("original-held-{call}.bgr"), &bgr),
        ] {
            if fs::read(root.join(name))? != *expected {
                return Err("same-sample native/Rust or held full CPU bytes differ".into());
            }
        }
        total = total
            .checked_add(packed / 4)
            .ok_or("live pixel count overflow")?;
    }
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|n| n != "live-sample-validation.json")
        {
            artifacts.push(records::artifact(root, &path)?);
        }
    }
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(
        serde_json::json!({"layout_version":1,"calls":3,"native_samples":3,"rust_conversions":3,"actual_pixels":total,
        "callback_owner_released":true,"cpu_reader_thread":s[2],"artifacts":artifacts,"known_scene_acceptance":false,"callback_drain_proved":false,"full_P1_accepted":false,
        "scope":"Separate native-sample conversion diagnostic: exact original class callback is forwarded unchanged before Rust with_sample locks/copies/converts that same live CMSampleBuffer synchronously on the observed native thread. Complete native-before/after BGRA, original/Rust BGR, callback-context release and post-source CPU owners on a joined reader are checked. No retry, native sample lease escape, known-scene failure waiver, independent source-selection equivalence or full P1 acceptance."}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err(
            "expected upstream-trace macos-live-sample <actual external directory> [--check-only]"
                .into(),
        ),
    };
    let root = records::external_root(Path::new(path))?;
    let report = check(&root)?;
    let output = root.join("live-sample-validation.json");
    if only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("live sample report differs from actual records".into());
        }
    } else if !only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS live_sample calls=3 original_samples=3 same_callback_conversions=3 full_byte_pairs=3 retained_cpu=6 callback_owner_released=1 known_scene_acceptance=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture(root: &Path) {
        fs::write(root.join("summary.tsv"), "7\t9\t10\t1\t0\t0\t0\t1\t0\n").unwrap();
        fs::write(root.join("callback-owner.tsv"), "1\t0\n").unwrap();
        fs::write(root.join("native-type.tsv"), "16\n").unwrap();
        fs::write(
            root.join("original-owner-closed.txt"),
            "context_owners=0\nsource_callbacks_drained=0\n",
        )
        .unwrap();
        fs::write(root.join("original-finish.txt"),"schema=1\ncalls=3\ncaller_thread=9\npid=7\nsource_object_destroyed=1\nheld_images=3\ncallback_drain_proved=0\nmodule_retained_until_process_exit=1\n").unwrap();
        let mut rows = format!("{HEADER}\n");
        for call in 0..3 {
            rows.push_str(&format!("{call}\t0\t1\t11\t3\t2\t16\t1\t1\t1\t24\t1\t0\n"));
            let sample = json!({"width":3,"height":2,"stride":16,"data_size":32,"row":12,"span":28,
                "format":u32::from_be_bytes(*b"BGRA"),"planar":0,"lock_code":0,"unlock_code":0,"packed_bytes":24});
            let native = json!({"schema":1,"call":call,"serial":call+1,"pid":7,"caller_thread":9,"ready":1,"fault":0,
                "content_entries":1,"content_callbacks":1,"sample_entries":1,"sample_callbacks":1,
                "content_enter_thread":9,"content_callback_thread":12,"sample_enter_thread":12,"sample_callback_thread":11,
                "content_present":1,"content_error_present":0,"display_count":1,"display_id":4,"display_width":3,"display_height":2,
                "filter_present":1,"config_present":1,"config_width":3,"config_height":2,"config_format":u32::from_be_bytes(*b"BGRA"),
                "config_srgb":1,"config_cursor":0,"forwarded":1,"sample_present":1,"error_present":0,"before":sample,"after":sample});
            fs::write(
                root.join(format!("original-native-{call}.json")),
                serde_json::to_vec(&native).unwrap(),
            )
            .unwrap();
            fs::write(
                root.join(format!("original-call-{call}.tsv")),
                format!("{call}\t1\t20\t9\t7\n"),
            )
            .unwrap();
            fs::write(
                root.join(format!("original-call-{call}-layout.tsv")),
                "3\t2\t9\t16\t16\n",
            )
            .unwrap();
            let raw = (0..6).flat_map(|i| [i, 7, call, 255]).collect::<Vec<u8>>();
            let bgr = raw
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| p[..3].iter().copied())
                .collect::<Vec<_>>();
            for name in [
                format!("original-native-{call}-before.bgra"),
                format!("original-native-{call}-after.bgra"),
                format!("rust-call-{call}.bgra"),
                format!("rust-held-{call}.bgra"),
            ] {
                fs::write(root.join(name), &raw).unwrap();
            }
            for name in [
                format!("original-call-{call}.bgr"),
                format!("original-held-{call}.bgr"),
                format!("rust-call-{call}.bgr"),
                format!("rust-held-{call}.bgr"),
            ] {
                fs::write(root.join(name), &bgr).unwrap();
            }
        }
        fs::write(root.join("live.tsv"), rows).unwrap();
    }
    fn root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mfs-live-sample-records-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    #[test]
    fn accepts_complete_synthetic_pipeline_and_rejects_each_changed_full_payload() {
        let root = root();
        fixture(&root);
        let report = check(&root).unwrap();
        assert_eq!(report["actual_pixels"], 18);
        assert_eq!(report["known_scene_acceptance"], false);
        // Both observed tag encodings are external native declarations, not
        // literal Rust format constants. A different selected OpenCV may use
        // another tag while producing the same packed BGR contract.
        fs::write(root.join("native-type.tsv"), "64\n").unwrap();
        for call in 0..3 {
            fs::write(
                root.join(format!("original-call-{call}-layout.tsv")),
                "3\t2\t9\t64\t64\n",
            )
            .unwrap();
        }
        check(&root).unwrap();
        fixture(&root);
        for call in 0..3 {
            for name in [
                format!("original-native-{call}-before.bgra"),
                format!("original-native-{call}-after.bgra"),
                format!("original-call-{call}.bgr"),
                format!("original-held-{call}.bgr"),
                format!("rust-call-{call}.bgra"),
                format!("rust-held-{call}.bgra"),
                format!("rust-call-{call}.bgr"),
                format!("rust-held-{call}.bgr"),
            ] {
                let path = root.join(&name);
                let mut raw = fs::read(&path).unwrap();
                raw[0] ^= 1;
                fs::write(&path, raw).unwrap();
                assert!(check(&root).is_err(), "{name}");
                fixture(&root);
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_incomplete_invocations_wrong_threads_native_failures_and_owner_relabeling() {
        let root = root();
        fixture(&root);
        let good = text(&root, "live.tsv").unwrap();
        for (column, bad) in [
            (0, "1"),
            (1, "2"),
            (2, "2"),
            (3, "12"),
            (4, "0"),
            (5, "18446744073709551615"),
            (6, "11"),
            (7, "0"),
            (8, "0"),
            (9, "0"),
            (10, "23"),
            (11, "0"),
            (12, "1"),
        ] {
            let mut lines = good.lines().map(str::to_owned).collect::<Vec<_>>();
            let mut v = lines[1].split('\t').map(str::to_owned).collect::<Vec<_>>();
            v[column] = bad.into();
            lines[1] = v.join("\t");
            fs::write(root.join("live.tsv"), lines.join("\n") + "\n").unwrap();
            assert!(check(&root).is_err(), "column {column}");
            fixture(&root);
        }
        for (name, bad) in [
            ("summary.tsv", "7\t9\t9\t1\t0\t0\t0\t1\t0\n"),
            ("summary.tsv", "7\t9\t10\t0\t0\t0\t0\t1\t0\n"),
            ("summary.tsv", "7\t9\t10\t1\t0\t0\t1\t1\t0\n"),
            ("callback-owner.tsv", "0\t0\n"),
            ("native-type.tsv", "64\n"),
            (
                "original-owner-closed.txt",
                "context_owners=0\nsource_callbacks_drained=1\n",
            ),
            ("original-finish.txt", ""),
            ("live.tsv", HEADER),
            ("original-call-0.tsv", "0\t0\t20\t9\t7\n"),
            ("original-call-0-layout.tsv", "3\t2\t10\t16\t16\n"),
        ] {
            fs::write(root.join(name), bad).unwrap();
            assert!(check(&root).is_err(), "{name}");
            fixture(&root);
        }
        let path = root.join("original-native-0.json");
        for (key, bad) in [
            ("ready", 0),
            ("fault", 8),
            ("display_count", 0),
            ("display_id", 0),
            ("sample_callback_thread", 12),
        ] {
            let mut record: serde_json::Value = records::read(&path).unwrap();
            record[key] = bad.into();
            fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            assert!(check(&root).is_err(), "{key}");
            fixture(&root);
        }
        assert!(row("1\t-1", 2).is_err());
        assert!(row("1\t18446744073709551616", 2).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
