// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Saved actual SCK capture composition with controlled host input owners.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const VALUES: [u64; 18] = [0, 0, 1, 1, 1, 1, 1, 2, 0, 3, 1, 1, 1, 0, 1, 3, 2, 4];
const EVENTS: [[u64; 4]; 18] = [
    [1, 1, 0, 0],
    [4, 1, 0, 0],
    [5, 1, 0, 0],
    [2, 1, 0, 0],
    [5, 1, 0, 1],
    [1, 2, 1, 0],
    [3, 1, 1, 0],
    [4, 2, 1, 0],
    [5, 1, 1, 2],
    [2, 2, 1, 1],
    [1, 3, 2, 0],
    [3, 2, 2, 0],
    [4, 3, 2, 0],
    [6, 1, 2, 0],
    [5, 3, 2, 3],
    [2, 3, 2, 2],
    [3, 3, 2, 0],
    [6, 3, 2, 0],
];
const SUMMARY: &str = "schema=1\ncases=20\nsource_captures=60\nrust_captures=60\ninput_failure_captures=40\ninput_actions=0\nsource_module_process_pinned=1\nrust_fixture_reclaimed_refs=61\nscene_closed=1\n";

fn row(line: &str, case: usize, target: u32) -> Result<()> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 6
        || fields[0].parse::<usize>()? != case
        || fields[1].parse::<u64>()? != 1 + (case as u64) % 2
        || fields[2].parse::<u32>()? != target
        || target == 0
    {
        return Err("native host case/configuration differs".into());
    }
    if serde_json::from_str::<Vec<u64>>(fields[3])? != VALUES {
        return Err("native host failure state/features/capture/owner counts differ".into());
    }
    let expected = EVENTS
        .iter()
        .copied()
        .filter(|e| matches!(e[0], 4..=6))
        .collect::<Vec<_>>();
    if serde_json::from_str::<Vec<[u64; 4]>>(fields[4])? != expected {
        return Err("native host input events differ".into());
    }
    if serde_json::from_str::<serde_json::Value>(fields[5])?
        != serde_json::json!({
        "type":"macos","window_id":target,"screencap_method":1,"input_method":1+(case as u64)%2})
    {
        return Err("native host actual metadata differs".into());
    }
    Ok(())
}
fn pixels(bgr: &[u8], raw: &[u8]) -> Result<()> {
    if bgr.len() != 47 * 31 * 3 || raw.len() != 47 * 31 * 4 {
        return Err("native host complete image length differs".into());
    }
    for (index, (bgr, bgra)) in bgr
        .as_chunks::<3>()
        .0
        .iter()
        .zip(raw.as_chunks::<4>().0)
        .enumerate()
    {
        if bgr != &bgra[..3]
            || ((2..45).contains(&(index % 47))
                && (2..29).contains(&(index / 47))
                && *bgr != [0, 255, 0])
        {
            return Err("native host full conversion/known interior pixels differ".into());
        }
    }
    Ok(())
}
pub(crate) fn run(args: &[String]) -> Result<()> {
    let (directory, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected macos-native-host <external records> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(directory))?;
    let source = fs::read_to_string(root.join("source.tsv"))?;
    let rust = fs::read_to_string(root.join("rust.tsv"))?;
    let owners = fs::read_to_string(root.join("source-events.tsv"))?;
    let sources = source.lines().collect::<Vec<_>>();
    let rusts = rust.lines().collect::<Vec<_>>();
    let owner_rows = owners.lines().collect::<Vec<_>>();
    if sources.len() != 20 || rusts.len() != 20 || owner_rows.len() != 20 {
        return Err("native host matrix incomplete".into());
    }
    let target = sources[0]
        .split('\t')
        .nth(2)
        .ok_or("native host target absent")?
        .parse::<u32>()?;
    let mut names = [
        "source.tsv",
        "rust.tsv",
        "source-events.tsv",
        "summary.txt",
        "module-status.txt",
        "libmfs_macos_manager_queries.dylib",
    ]
    .into_iter()
    .map(String::from)
    .collect::<BTreeSet<_>>();
    for case in 0..20 {
        row(sources[case], case, target)?;
        row(rusts[case], case, target)?;
        let fields = owner_rows[case].split('\t').collect::<Vec<_>>();
        if fields.len() != 2
            || fields[0].parse::<usize>()? != case
            || serde_json::from_str::<Vec<[u64; 4]>>(fields[1])? != EVENTS
        {
            return Err("native host capture-before-input replacement/drop events differ".into());
        }
        for call in 0..3 {
            let mut images = Vec::new();
            for side in ["source", "rust"] {
                for moment in ["current", "held"] {
                    let name = format!("{side}-{moment}-{case}-{call}.bgr");
                    images.push(fs::read(root.join(&name))?);
                    names.insert(name);
                }
            }
            let raw_name = format!("rust-raw-{case}-{call}.bgra");
            let raw = fs::read(root.join(&raw_name))?;
            names.insert(raw_name);
            if images[1..].iter().any(|image| image != &images[0]) {
                return Err("native host original/Rust full current/held bytes differ".into());
            }
            pixels(&images[0], &raw)?;
        }
    }
    if fs::read_to_string(root.join("summary.txt"))? != SUMMARY
        || fs::read_to_string(root.join("module-status.txt"))?
            != "schema=1\nprocess_pinned=1\nnative_input_actions=0\n"
    {
        return Err("native host scope/summary differs".into());
    }
    let library = fs::read(root.join("libmfs_macos_manager_queries.dylib"))?;
    if library.len() < 32
        || library[..4] != [0xcf, 0xfa, 0xed, 0xfe]
        || library[4..8] != [12, 0, 0, 1]
        || library[12..16] != [6, 0, 0, 0]
    {
        return Err("native host source module is not an arm64 Mach-O dylib".into());
    }
    let mut actual = fs::read_dir(&root)?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<BTreeSet<_>>>()?;
    actual.remove("native-host-validation.json");
    if actual != names || names.len() != 306 {
        return Err("native host unexpected/missing record files".into());
    }
    let inputs = names
        .iter()
        .map(|name| records::artifact(&root, &root.join(name)))
        .collect::<Result<Vec<_>>>()?;
    let report = serde_json::json!({"layout_version":1,"cases":20,"captures":120,
        "input_failure_captures":40,"retained_bgr":120,"native_input_actions":0,
        "source_module_process_pinned":true,"files":306,"inputs":inputs,"full_P1_acceptance":false,
        "scope":"Actual unchanged original SCK and same Rust native capture helper through exact manager bodies with controlled input owners, native availability, three connects and dynamic features. Complete source/Rust/held BGR and raw Rust conversion bytes. Source Mat owners outlive managers; Rust images outlive scene shutdown. Source module is deliberately pinned for process lifetime because original callback disposal is unobserved; original semaphore create references remain unreleased. Rust fixture reclaims only after its observed owners end. No input actions/native input constructor, general failure/availability fault, source callback drain, module/global resource closure or full P1 acceptance."});
    let output = root.join("native-host-validation.json");
    if check_only {
        if output.exists() && records::read::<serde_json::Value>(&output)? != report {
            return Err("saved native host report differs".into());
        }
    } else {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS native host cases=20 captures=120 input_failure_captures=40 retained_bgr=120 input_actions=0 source_module_pinned=1 files=306 full_P1_acceptance=0 passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> String {
        let input = EVENTS
            .iter()
            .copied()
            .filter(|e| matches!(e[0], 4..=6))
            .collect::<Vec<_>>();
        format!(
            "0\t1\t7\t{}\t{}\t{}",
            serde_json::to_string(&VALUES).unwrap(),
            serde_json::to_string(&input).unwrap(),
            serde_json::json!({"type":"macos","window_id":7,"screencap_method":1,"input_method":1})
        )
    }
    #[test]
    fn rejects_connected_gate_feature_loss_and_input_event_mutations() {
        let text = valid();
        row(&text, 0, 7).unwrap();
        for index in [7, 8, 9, 10, 15, 16, 17] {
            let mut fields = text.split('\t').map(String::from).collect::<Vec<_>>();
            let mut values = VALUES;
            values[index] += 1;
            fields[3] = serde_json::to_string(&values).unwrap();
            assert!(row(&fields.join("\t"), 0, 7).is_err());
        }
        assert!(row(&text, 0, 8).is_err());
        assert!(row(&text.replace("\t7\t", "\t4294967296\t"), 0, 7).is_err());
        let mut fields = text.split('\t').map(String::from).collect::<Vec<_>>();
        let mut events: Vec<[u64; 4]> = serde_json::from_str(&fields[4]).unwrap();
        events.pop();
        fields[4] = serde_json::to_string(&events).unwrap();
        assert!(row(&fields.join("\t"), 0, 7).is_err());
    }
    #[test]
    fn checks_conversion_and_edge_bytes_outside_known_interior() {
        let bgr = [0, 255, 0].repeat(47 * 31);
        let mut raw = [0, 255, 0, 17].repeat(47 * 31);
        pixels(&bgr, &raw).unwrap();
        raw[0] = 1;
        assert!(pixels(&bgr, &raw).is_err());
    }
}
