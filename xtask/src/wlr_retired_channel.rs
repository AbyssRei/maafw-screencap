// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual Rust retired-channel resources and CPU outputs, not native execution.
use crate::{Result, records};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
const HEADER: &str = "cycle\tmode\tconnected\tglobals\tmanager_refs\tshm_refs\toutput_refs\tnative_fd_retained\tclosed_rejected\tstale_rejected\trebound\told_channel_released\told_fd_closed\tfds_before\tfds_after\ttasks_equal\tcallbacks_balanced\tcaptures\theld\tbusy_rejections\tdata_created\tdata_destroyed\tdata_dropped";
const MODES: [&str; 4] = ["control", "cold", "buffered", "reconnect"];

fn observations(text: &str) -> Result<u64> {
    let mut rows = text.lines();
    if rows.next() != Some(HEADER) {
        return Err("Wrong retired-channel record header".into());
    }
    let mut baseline = None;
    for cycle in 0..20 {
        for (mode, name) in MODES.iter().enumerate() {
            let fields: Vec<_> = rows
                .next()
                .ok_or("Incomplete retired-channel matrix")?
                .split('\t')
                .collect();
            if fields.len() != 23 || fields[0] != cycle.to_string() || fields[1] != *name {
                return Err("Wrong retired-channel case order/width".into());
            }
            let actual = fields[2..]
                .iter()
                .map(|n| n.parse::<u64>())
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if fields[2..]
                .iter()
                .zip(&actual)
                .any(|(s, n)| *s != n.to_string())
            {
                return Err("Noncanonical retired-channel scalar".into());
            }
            let fds = *baseline.get_or_insert(actual[11]);
            let rebound = u64::from(mode == 3);
            let retained = u64::from(mode != 0);
            let captures = [0, 0, 1, 2][mode];
            let expected = [
                rebound,
                rebound * 7,
                retained,
                retained,
                retained,
                retained,
                u64::from(mode == 1 || mode == 2),
                rebound,
                rebound,
                1,
                1,
                fds,
                fds,
                1,
                1,
                captures,
                captures,
                captures * 2,
            ];
            if fds < 4
                || actual[..18] != expected
                || actual[18] < 6
                || actual[18] != actual[19]
                || actual[19] != actual[20]
            {
                return Err(
                    "Retired-channel logical/native owners, rejections or resources differ".into(),
                );
            }
        }
    }
    if rows.next().is_some() {
        return Err("Extra retired-channel records".into());
    }
    baseline.ok_or_else(|| "Missing retired-channel baseline".into())
}
fn raw_pixels(meta: &str, bytes: &[u8], expected: &[u8]) -> Result<()> {
    let mut fields = BTreeMap::new();
    for line in meta.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("Invalid retired-channel raw metadata")?;
        if fields.insert(key, value).is_some() {
            return Err("Duplicate raw metadata field".into());
        }
    }
    if fields.len() != 6
        || fields.get("width") != Some(&"64")
        || fields.get("height") != Some(&"48")
    {
        return Err("Retired-channel raw dimensions differ".into());
    }
    let (channels, order) = match fields.get("conversion").copied() {
        Some("Bgra") => (4, [0, 1, 2]),
        Some("Rgba") => (4, [2, 1, 0]),
        Some("Rgb") => (3, [2, 1, 0]),
        Some("BgrCopy") => (3, [0, 1, 2]),
        _ => return Err("Unknown retired-channel source conversion".into()),
    };
    let stride = (64 * channels).to_string();
    if fields.get("native_stride").copied() != Some(stride.as_str())
        || bytes.len() != 64 * 48 * channels
        || expected.len() != 64 * 48 * 3
    {
        return Err("Retired-channel raw span/stride differs".into());
    }
    let time = fields
        .get("ready_time")
        .ok_or("Missing native capture time")?
        .strip_prefix("Some((")
        .and_then(|s| s.strip_suffix("))"))
        .ok_or("Missing native Ready evidence")?;
    if time
        .split(',')
        .map(|v| v.trim().parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()?
        .len()
        != 3
    {
        return Err("Malformed native Ready evidence".into());
    }
    let flags = fields.get("flags").ok_or("Missing native flag evidence")?;
    if *flags != "None" {
        flags
            .strip_prefix("Some(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or("Malformed native flags")?
            .parse::<u32>()?;
    }
    for (pixel, bgr) in bytes
        .chunks_exact(channels)
        .zip(expected.as_chunks::<3>().0)
    {
        if order
            .into_iter()
            .enumerate()
            .any(|(c, index)| pixel[index] != bgr[c])
        {
            return Err("Retired-channel raw target pixels differ".into());
        }
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (directory,source,check_only) = match args {
        [directory,source] => (directory,source,false), [directory,source,flag] if flag=="--check-only" => (directory,source,true),
        _ => return Err("expected upstream-trace wlr-retired-channel <Rust directory> <original prefix directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(directory))?;
    let oracle = records::external_root(Path::new(source))?;
    // Always check the actual original files, without modifying their record.
    crate::wlr_early_close::run(&[source.clone(), "--check-only".into()])?;
    let mut inputs = Vec::new();
    let mut names = BTreeSet::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        names.insert(name.to_string());
        inputs.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let rows = read("early-close.tsv")?;
    let fds = observations(std::str::from_utf8(&rows)?)?;
    let tasks = read("early-close-tasks.txt")?;
    let ids = std::str::from_utf8(&tasks)?.lines().collect::<Vec<_>>();
    if ids.is_empty()
        || ids.windows(2).any(|w| w[0] >= w[1])
        || ids.iter().any(|s| {
            s.parse::<u32>()
                .map_or(true, |n| n == 0 || n.to_string() != *s)
        })
    {
        return Err("Invalid retired-channel task identities".into());
    }
    let expected: Vec<u8> = (0..48)
        .flat_map(|y| {
            (0..64).flat_map(move |x| {
                if y >= 24 {
                    [255, 0, 0]
                } else if x < 32 {
                    [0, 0, 255]
                } else {
                    [0, 255, 0]
                }
            })
        })
        .collect();
    for cycle in 0..20 {
        for (mode, name) in MODES.iter().enumerate() {
            if read(&format!("{cycle}-{name}.tasks"))? != tasks {
                return Err("Retired-channel final task identities differ".into());
            }
            for phase in match mode {
                2 => &["before"][..],
                3 => &["before", "fresh"][..],
                _ => &[][..],
            } {
                let stem = format!("{cycle}-{name}-{phase}");
                let bgr = read(&format!("{stem}.bgr"))?;
                let held = read(&format!("{stem}.held"))?;
                if bgr != expected
                    || held != bgr
                    || fs::read(oracle.join(format!("{stem}.bgr")))? != bgr
                {
                    return Err("Retired-channel BGR/original/retained pixels differ".into());
                }
                let raw = read(&format!("{stem}.raw"))?;
                if read(&format!("{stem}.raw-held"))? != raw {
                    return Err("Retired-channel raw owner changed after close".into());
                }
                let meta = read(&format!("{stem}.meta.txt"))?;
                raw_pixels(std::str::from_utf8(&meta)?, &raw, &bgr)?;
            }
        }
    }
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("Unexpected retired-channel directory/link".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 retired-channel file")?;
        if name != "retired-channel-validation.json" {
            actual.insert(name);
        }
    }
    if actual != names || inputs.len() != 382 {
        return Err("Retired-channel records have missing or extra files".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles":20,"cases":80,"control_prefixes":80,
        "retained_native_channels":60,"closed_rejected":40,"stale_rejected":20,"captures":60,
        "retained_raw":60,"retained_bgr":60,"busy_control_rejections":120,"baseline_fds":fds,
        "original_prefix_validation_sha256":records::hash(&oracle.join("early-close-validation.json"))?,"artifacts":inputs,
        "scope":"Actual Rust logical close/reconnect with retained native channel owners, exact old proxy references, fd/task/callback release and CPU pixels compared to the original controlled prefix. Native disconnect is delayed by the Rust safety owner; stale old snapshots reject before marshalling instead of executing the original invalid cross-display combination. This does not accept original immediate disconnect, general reentry, native concurrent use or full P1."});
    let output = root.join("retired-channel-validation.json");
    if check_only {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved retired-channel report differs from actual inputs".into());
        }
    } else {
        records::create_json(&output, &report)?;
    }
    println!(
        "Validated Rust Wlr retired channels cases=80 captures=60 raw_and_bgr_files=240 task_files=80 busy_control_rejections=120 actual_files=382 full_P1_acceptance=0 passed"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_native_matrix_and_blank_raw_capture() {
        assert!(observations("").is_err());
        assert!(observations(HEADER).is_err());
        assert!(observations(&format!("{HEADER}\n0\tcontrol\t0\n")).is_err());
        assert!(raw_pixels("width=64\nheight=48\n", &[], &[]).is_err());
    }
}
