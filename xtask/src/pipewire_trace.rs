// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual sustained owned-RGB trace format. This validates external records;
//! it neither performs capture nor introduces a general benchmark framework.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

const CAPTURES: &str = "index\telapsed_ns\tcapture_ns\tmarker\tmarker_changed\treplayed";
const RESOURCES: &str = "phase\telapsed_ns\tcalls\tfds\ttasks\trss_kib";
const MIN_NS: u64 = 120_000_000_000;

pub fn run(args: &[String]) -> Result<()> {
    let (mode, path, check_only) = match args {
        [kind, mode, path] if kind == "pipewire" => (mode, path, false),
        [kind, mode, path, flag] if kind == "pipewire" && flag == "--check-only" => {
            (mode, path, true)
        }
        _ => return Err(
            "expected upstream-trace pipewire <source|rust> <external directory> [--check-only]"
                .into(),
        ),
    };
    let rust = match mode.as_str() {
        "rust" => true,
        "source" => false,
        _ => return Err("trace owner must be source or rust".into()),
    };
    if !Path::new(path).is_dir() {
        return Err("sustained trace directory is missing".into());
    }
    let directory = records::external_root(Path::new(path))?;
    let report = validate(&directory, rust)?;
    let report_path = directory.join("trace-validation.json");
    if check_only && report_path.exists() {
        let saved: serde_json::Value = records::read(&report_path)?;
        if saved != serde_json::to_value(&report)? {
            return Err("saved sustained validation differs from actual inputs".into());
        }
    } else if !check_only {
        records::create_json(&report_path, &report)?;
    }
    println!(
        "PipeWire sustained trace owner={mode} calls={} payloads={} steady_ns={} resources={} pixels_verified=1 passed",
        report.calls, report.payloads, report.steady_ns, report.resource_rows
    );
    Ok(())
}

#[derive(Serialize)]
struct Report {
    layout_version: u32,
    owner: &'static str,
    warmup: u64,
    calls: u64,
    steady_ns: u64,
    payloads: u64,
    resource_rows: usize,
    marker_changes: u64,
    // A producer marker change is not a native sample/freshness timestamp.
    marker_changes_are_not_native_freshness: bool,
    steady_replays: Option<u64>,
    capture_p50_ns: u64,
    capture_p95_ns: u64,
    capture_p99_ns: u64,
    rss_kib: Range,
    fds: Range,
    tasks: Range,
    inputs: Vec<records::Artifact>,
    scope: &'static str,
}
#[derive(Serialize)]
struct Range {
    min: u64,
    max: u64,
}
impl Range {
    fn empty() -> Self {
        Self {
            min: u64::MAX,
            max: 0,
        }
    }
    fn include(&mut self, value: u64) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}
fn number(value: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("non-decimal trace integer".into());
    }
    Ok(value.parse()?)
}
fn table<'a>(text: &'a str, header: &str) -> Result<Vec<Vec<&'a str>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("unexpected sustained trace header".into());
    }
    lines
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 6 {
                return Err("sustained trace row must have six fields".into());
            }
            Ok(fields)
        })
        .collect()
}
struct Captures {
    markers: Vec<u64>,
    latencies: Vec<u64>,
    changes: u64,
    replays: u64,
}
fn captures(text: &str, rust: bool, end_ns: u64) -> Result<Captures> {
    let mut result = Captures {
        markers: Vec::new(),
        latencies: Vec::new(),
        changes: 0,
        replays: 0,
    };
    let mut elapsed = 0;
    for (index, row) in table(text, CAPTURES)?.into_iter().enumerate() {
        let now = number(row[1])?;
        let latency = number(row[2])?;
        let marker = number(row[3])?;
        let delta = now
            .checked_sub(elapsed)
            .filter(|v| *v > 0)
            .ok_or("non-increasing sustained capture clock")?;
        if number(row[0])? != u64::try_from(index)?
            || now > end_ns
            || latency > delta
            || !(1..=0xFF_FFFF).contains(&marker)
        {
            return Err("invalid sustained capture index/time/latency/marker".into());
        }
        let expected = result
            .markers
            .last()
            .map(|old| if *old == marker { "0" } else { "1" })
            .unwrap_or("-1");
        if row[4] != expected {
            return Err("false sustained producer marker-change evidence".into());
        }
        if row[4] == "1" {
            result.changes = result
                .changes
                .checked_add(1)
                .ok_or("marker count overflow")?;
        }
        if rust {
            match row[5] {
                "0" => {}
                "1" => {
                    result.replays = result
                        .replays
                        .checked_add(1)
                        .ok_or("replay count overflow")?
                }
                _ => return Err("missing actual Rust consumer replay decision".into()),
            }
        } else if row[5] != "-1" {
            return Err("original API replay evidence must remain unknown".into());
        }
        result.markers.push(marker);
        result.latencies.push(latency);
        elapsed = now;
    }
    Ok(result)
}
fn summary(text: &str) -> Result<BTreeMap<&str, &str>> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("invalid sustained summary field")?;
        if key.is_empty() || value.is_empty() || fields.insert(key, value).is_some() {
            return Err("empty or duplicate sustained summary field".into());
        }
    }
    Ok(fields)
}
fn pixels(bytes: &[u8], marker: u64, bgr: bool) -> Result<()> {
    if bytes.len() != 2016 {
        return Err("sustained image is not a complete 28x24 RGB/BGR target".into());
    }
    let actual = if bgr {
        [bytes[2], bytes[1], bytes[0]]
    } else {
        [bytes[0], bytes[1], bytes[2]]
    };
    if u64::from(actual[0]) << 16 | u64::from(actual[1]) << 8 | u64::from(actual[2]) != marker {
        return Err("sustained image marker differs from its record".into());
    }
    const BARS: [[u8; 3]; 7] = [
        [191, 191, 191],
        [191, 191, 0],
        [0, 191, 191],
        [0, 191, 0],
        [191, 0, 191],
        [191, 0, 0],
        [0, 0, 191],
    ];
    for y in 0..24 {
        for x in 0..28 {
            if x == 0 && y == 0 {
                continue;
            }
            for c in 0..3 {
                let rgb = if bgr { 2 - c } else { c };
                let delta =
                    u8::try_from((y * 7 + x * 3 + rgb * 13 + usize::try_from(marker & 63)?) & 63)?;
                if bytes[y * 84 + x * 3 + c] != BARS[x / 4][rgb] ^ delta {
                    return Err("sustained known target pixel mismatch".into());
                }
            }
        }
    }
    Ok(())
}
fn percentile(values: &[u64], percent: u64) -> Result<u64> {
    let rank = u64::try_from(values.len())?
        .checked_mul(percent)
        .ok_or("latency rank overflow")?
        .div_ceil(100);
    Ok(*values
        .get(usize::try_from(
            rank.checked_sub(1).ok_or("empty capture latencies")?,
        )?)
        .ok_or("invalid percentile")?)
}
fn task_identities(text: &str) -> Result<Vec<u64>> {
    let mut ids = text.lines().map(number).collect::<Result<Vec<_>>>()?;
    ids.sort_unstable();
    if ids.is_empty() || ids[0] == 0 || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("empty, zero or duplicate sustained task identity".into());
    }
    Ok(ids)
}
fn validate(directory: &Path, rust: bool) -> Result<Report> {
    let text = fs::read_to_string(records::resolve(directory, "summary.txt")?)?;
    let fields = summary(&text)?;
    let get = |key: &str| -> Result<&str> {
        fields
            .get(key)
            .copied()
            .ok_or_else(|| format!("missing sustained summary {key}").into())
    };
    let n = |key| -> Result<u64> { number(get(key)?) };
    for (key, value) in [
        ("trace_version", "1"),
        ("warmup", "3"),
        ("retained_at_shutdown", "4"),
        ("errors", "0"),
        ("marker_changes_are_not_native_freshness", "1"),
        ("graphics", "owned_CPU_RGB_fixture_no_GPU_claim"),
        ("resources_balanced", "1"),
    ] {
        if get(key)? != value {
            return Err(format!("sustained summary requirement failed: {key}").into());
        }
    }
    let calls = n("steady_calls")?;
    let steady_ns = n("steady_elapsed_ns")?;
    if calls < 4 || steady_ns < MIN_NS {
        return Err("sustained capture must cover >=120 seconds after warmup".into());
    }
    let mut captured = captures(
        &fs::read_to_string(records::resolve(directory, "captures.tsv")?)?,
        rust,
        steady_ns,
    )?;
    if u64::try_from(captured.markers.len())? != calls {
        return Err("missing sustained capture rows".into());
    }
    if rust {
        let samples = n("native_copies")?
            .checked_add(n("warmup_replays")?)
            .and_then(|v| v.checked_add(captured.replays))
            .ok_or("native sample/replay count overflow")?;
        let total_calls = calls
            .checked_add(3)
            .ok_or("sustained call count overflow")?;
        if n("steady_replays")? != captured.replays
            || n("native_dequeued")? != n("native_returned")?
            || get("native_copies_include_warmup")? != "1"
            || samples != total_calls
        {
            return Err("sustained native copy/replay/buffer accounting mismatch".into());
        }
    } else if get("native_buffer_counters")? != "not_exposed_by_original_API"
        || get("replayed")? != "-1_unknown_not_exposed_by_original_API"
    {
        return Err("original private counters/replay must remain explicitly unknown".into());
    }
    let resource_text = fs::read_to_string(records::resolve(directory, "resources.tsv")?)?;
    let rows = table(&resource_text, RESOURCES)?;
    if rows.len() < 3 {
        return Err("missing sustained resource series".into());
    }
    let baseline = task_identities(&fs::read_to_string(records::resolve(
        directory,
        "baseline-task-ids.txt",
    )?)?)?;
    let closed = task_identities(&fs::read_to_string(records::resolve(
        directory,
        "closed-task-ids.txt",
    )?)?)?;
    if baseline != closed {
        return Err("sustained shutdown changed task identities".into());
    }
    let baseline_fds = n("baseline_fds")?;
    let (mut rss_kib, mut fds, mut tasks) = (Range::empty(), Range::empty(), Range::empty());
    let (mut elapsed, mut seen_calls) = (0, 0);
    for (index, row) in rows.iter().enumerate() {
        let closed = index == rows.len() - 1;
        let now = number(row[1])?;
        let count = number(row[2])?;
        let (fd, task, rss) = (number(row[3])?, number(row[4])?, number(row[5])?);
        if row[0] != if closed { "closed" } else { "stream" }
            || now < elapsed
            || count < seen_calls
            || count > calls
            || fd == 0
            || task == 0
            || rss == 0
            || (closed
                && (count != calls
                    || elapsed < MIN_NS
                    || now < steady_ns
                    || fd != baseline_fds
                    || task != u64::try_from(baseline.len())?))
        {
            return Err("invalid sustained resource clock/fd/task/memory/shutdown series".into());
        }
        rss_kib.include(rss);
        fds.include(fd);
        tasks.include(task);
        elapsed = now;
        seen_calls = count;
    }
    let frames = records::resolve(directory, "frames")?;
    let mut actual_files = 0_u64;
    for entry in fs::read_dir(&frames)? {
        if !entry?.file_type()?.is_file() {
            return Err("unexpected sustained payload entry".into());
        }
        actual_files = actual_files
            .checked_add(1)
            .ok_or("payload count overflow")?;
    }
    let payloads = calls
        .checked_mul(if rust { 2 } else { 1 })
        .ok_or("payload count overflow")?;
    if actual_files != payloads {
        return Err("missing or extra sustained image payloads".into());
    }
    for (index, marker) in captured.markers.iter().copied().enumerate() {
        let stem = format!("frame-{index:06}");
        let bgr = fs::read(frames.join(format!("{stem}.bgr")))?;
        pixels(&bgr, marker, true)?;
        if rust {
            let rgb = fs::read(frames.join(format!("{stem}.rgb")))?;
            pixels(&rgb, marker, false)?;
            // The independent expected-color checks cover both complete spans;
            // also assert exact same-sample channel mapping, including marker.
            for (rgb, bgr) in rgb.as_chunks::<3>().0.iter().zip(bgr.as_chunks::<3>().0) {
                if *bgr != [rgb[2], rgb[1], rgb[0]] {
                    return Err("sustained same-sample RGB/BGR mismatch".into());
                }
            }
        }
    }
    captured.latencies.sort_unstable();
    Ok(Report {
        layout_version: 1,
        owner: if rust { "rust" } else { "source" },
        warmup: 3,
        calls,
        steady_ns,
        payloads,
        resource_rows: rows.len(),
        marker_changes: captured.changes,
        marker_changes_are_not_native_freshness: true,
        steady_replays: rust.then_some(captured.replays),
        capture_p50_ns: percentile(&captured.latencies, 50)?,
        capture_p95_ns: percentile(&captured.latencies, 95)?,
        capture_p99_ns: percentile(&captured.latencies, 99)?,
        rss_kib,
        fds,
        tasks,
        inputs: [
            "summary.txt",
            "captures.tsv",
            "resources.tsv",
            "baseline-task-ids.txt",
            "closed-task-ids.txt",
        ]
        .map(|file| records::artifact(directory, &directory.join(file)))
        .into_iter()
        .collect::<Result<_>>()?,
        scope: "Owned CPU RGB source/manager only. Trace validation is not a native run, GPU completion, general memory-growth proof, performance threshold or full P1 acceptance. Marker changes do not establish native freshness. Native resource/callback shutdown assertions run in the capture processes separately.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_fabricated_replay_marker_changes_and_time() {
        let valid = format!("{CAPTURES}\n0\t10\t5\t1\t-1\t0\n1\t20\t5\t1\t0\t1\n");
        let result = captures(&valid, true, 20).unwrap();
        assert_eq!((result.changes, result.replays), (0, 1));
        for invalid in [
            valid.replace("1\t0\t1", "1\t1\t1"),
            valid.replace("20\t5", "20\t11"),
            valid.replace("-1\t0", "-1\t-1"),
            valid.replace("1\t20", "1\t10"),
        ] {
            assert!(captures(&invalid, true, 20).is_err());
        }
    }
    #[test]
    fn preserves_unknown_original_replay_and_rejects_numeric_overflow() {
        let text = format!("{CAPTURES}\n0\t10\t5\t1\t-1\t-1\n");
        assert!(captures(&text, false, 10).is_ok());
        assert!(captures(&text.replace("-1\t-1", "-1\t0"), false, 10).is_err());
        for value in ["", "-1", "+1", "18446744073709551616"] {
            assert!(number(value).is_err());
        }
        assert!(summary("errors=0\nerrors=1\n").is_err());
        assert!(table("unknown\n", CAPTURES).is_err());
        assert_eq!(percentile(&[1, 2, 3, 4], 50).unwrap(), 2);
        assert_eq!(percentile(&[1, 2, 3, 4], 99).unwrap(), 4);
    }
    #[test]
    fn rejects_invalid_task_sets_and_incomplete_or_wrong_marker_pixels() {
        assert_eq!(task_identities("20\n3\n").unwrap(), [3, 20]);
        for text in ["", "0\n", "3\n3\n", "unknown\n"] {
            assert!(task_identities(text).is_err());
        }
        assert!(pixels(&[0; 2015], 1, true).is_err());
        assert!(pixels(&[0; 2016], 1, true).is_err());
    }
}
