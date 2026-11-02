// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent native callback bytes; no waiver for strict RGB capture failures.
use crate::{Result, macos_display_geometry::Geometry};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    width: u64,
    height: u64,
    stride: u64,
    data_size: u64,
    row: u64,
    span: u64,
    format: u32,
    planar: u32,
    lock_code: i32,
    unlock_code: i32,
    packed_bytes: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    schema: u32,
    call: u32,
    serial: u64,
    pid: u32,
    caller_thread: u64,
    ready: u32,
    fault: u32,
    content_entries: u64,
    content_callbacks: u64,
    sample_entries: u64,
    sample_callbacks: u64,
    content_enter_thread: u64,
    content_callback_thread: u64,
    sample_enter_thread: u64,
    sample_callback_thread: u64,
    content_present: u32,
    content_error_present: u32,
    display_count: u64,
    display_id: u64,
    display_width: u64,
    display_height: u64,
    filter_present: u32,
    config_present: u32,
    config_width: u64,
    config_height: u64,
    config_format: u64,
    config_srgb: u32,
    config_cursor: u32,
    forwarded: u32,
    sample_present: u32,
    error_present: u32,
    before: Snapshot,
    after: Snapshot,
}
fn validate_record(
    record: &Observation,
    call: usize,
    g: &Geometry,
    pid: u64,
    caller: u64,
) -> Result<()> {
    if record.schema != 1
        || u64::from(record.call) != u64::try_from(call)?
        || record.serial
            != u64::try_from(call)?
                .checked_add(1)
                .ok_or("serial overflow")?
        || u64::from(record.pid) != pid
        || record.caller_thread != caller
        || caller == 0
        || record.content_enter_thread != caller
        || record.content_callback_thread == 0
        || record.sample_enter_thread != record.content_callback_thread
        || record.sample_callback_thread == 0
        || record.ready != 1
        || record.fault != 0
        || record.content_entries != 1
        || record.content_callbacks != 1
        || record.sample_entries != 1
        || record.sample_callbacks != 1
        || record.forwarded != 1
        || record.sample_present != 1
        || record.error_present != 0
        || record.content_present != 1
        || record.content_error_present != 0
        || record.display_count != u64::from(g.count)
        || record.display_id != u64::from(g.display)
        || record.display_width != u64::try_from(g.width)?
        || record.display_height != u64::try_from(g.height)?
        || record.filter_present != 1
        || record.config_present != 1
        || record.config_width != record.display_width
        || record.config_height != record.display_height
        || record.config_format != u64::from(u32::from_be_bytes(*b"BGRA"))
        || record.config_srgb != 1
        || record.config_cursor != 0
    {
        return Err("native sample call association/callback/premise differs".into());
    }
    let s = &record.before;
    let row = s.width.checked_mul(4).ok_or("native row overflow")?;
    let span = s
        .height
        .checked_sub(1)
        .and_then(|h| h.checked_mul(s.stride))
        .and_then(|prefix| prefix.checked_add(row))
        .ok_or("native readable span overflow")?;
    let packed = row.checked_mul(s.height).ok_or("native packed overflow")?;
    if record.before != record.after
        || s.width != u64::try_from(g.width)?
        || s.height != u64::try_from(g.height)?
        || s.row != row
        || s.stride < row
        || s.span != span
        || span > s.data_size
        || span > i64::MAX as u64
        || packed > i64::MAX as u64
        || s.packed_bytes != packed
        || s.format != u32::from_be_bytes(*b"BGRA")
        || s.planar != 0
        || s.lock_code != 0
        || s.unlock_code != 0
    {
        return Err("native sample phase/layout/lock evidence differs".into());
    }
    Ok(())
}
pub fn validate(
    arm: &Path,
    mode: u64,
    call: usize,
    g: &Geometry,
    pid: u64,
    caller: u64,
) -> Result<serde_json::Value> {
    let stem = format!("original-native-{call}");
    let record: Observation = crate::records::read(&arm.join(format!("{stem}.json")))?;
    validate_record(&record, call, g, pid, caller)?;
    let before = fs::read(arm.join(format!("{stem}-before.bgra")))?;
    let after = fs::read(arm.join(format!("{stem}-after.bgra")))?;
    let bgr = fs::read(arm.join(format!("original-call-{call}.bgr")))?;
    if before.len() != usize::try_from(record.before.packed_bytes)?
        || before != after
        || bgr.len()
            != g.width
                .checked_mul(g.height)
                .and_then(|n| n.checked_mul(3))
                .ok_or("BGR span overflow")?
        || before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(bgr.as_chunks::<3>().0)
            .any(|(a, b)| &a[..3] != b)
    {
        return Err(
            "native callback input changed or original BGR differs from actual native rows".into(),
        );
    }
    Ok(
        serde_json::json!({"mode":mode,"call":call,"serial":record.serial,
        "actual_native_pixels":g.width.checked_mul(g.height).ok_or("pixel overflow")?,
        "native_before_after_equal":true,"source_bgr_matches_native":true,
        "native_first_display_and_config_match":true,
        "native_full_rgb_differences":crate::macos_full_display_witness::differences(g,&before,4)?,
        "observation_wait_did_not_replace_source_return":true,"callback_drain_proved":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Geometry, serde_json::Value) {
        let g = crate::macos_display_geometry::parse_full(
            "1\t1\t3\t2\t0\t0\t3\t2\t4\t5\t0\t0\t3\t2\t1\t0\n",
        )
        .unwrap();
        let s = serde_json::json!({"width":3,"height":2,"stride":16,"data_size":32,"row":12,
            "span":28,"format":u32::from_be_bytes(*b"BGRA"),"planar":0,"lock_code":0,"unlock_code":0,"packed_bytes":24});
        (
            g,
            serde_json::json!({"schema":1,"call":0,"serial":1,"pid":7,"caller_thread":9,"ready":1,"fault":0,
            "content_entries":1,"content_callbacks":1,"sample_entries":1,"sample_callbacks":1,
            "content_enter_thread":9,"content_callback_thread":10,"sample_enter_thread":10,"sample_callback_thread":11,
            "forwarded":1,"sample_present":1,"error_present":0,
            "content_present":1,"content_error_present":0,"display_count":1,"display_id":1,"display_width":3,"display_height":2,
            "filter_present":1,"config_present":1,"config_width":3,"config_height":2,"config_format":u32::from_be_bytes(*b"BGRA"),
            "config_srgb":1,"config_cursor":0,"before":s,"after":s}),
        )
    }
    #[test]
    fn accepts_observed_native_call_and_checked_padded_layout() {
        let (g, value) = fixture();
        validate_record(&serde_json::from_value(value).unwrap(), 0, &g, 7, 9).unwrap();
    }
    #[test]
    fn refuses_wrong_association_duplicate_callbacks_and_incomplete_native_rows() {
        let (g, value) = fixture();
        for (key, bad) in [
            ("serial", 2),
            ("pid", 8),
            ("caller_thread", 10),
            ("content_enter_thread", 10),
            ("sample_enter_thread", 11),
            ("ready", 0),
            ("fault", 1),
            ("content_entries", 2),
            ("sample_callbacks", 2),
            ("forwarded", 0),
            ("sample_present", 0),
            ("error_present", 1),
            ("display_id", 2),
            ("display_count", 0),
            ("config_width", 4),
            ("config_srgb", 0),
            ("config_cursor", 1),
        ] {
            let mut v = value.clone();
            v[key] = bad.into();
            assert!(
                validate_record(&serde_json::from_value(v).unwrap(), 0, &g, 7, 9).is_err(),
                "{key}"
            );
        }
        for (key, bad) in [
            ("stride", 11_u64),
            ("data_size", 27),
            ("span", 29),
            ("packed_bytes", 23),
            ("width", u64::MAX),
            ("format", 0),
            ("planar", 1),
            ("unlock_code", 1),
        ] {
            let mut v = value.clone();
            v["before"][key] = bad.into();
            v["after"][key] = bad.into();
            assert!(
                validate_record(&serde_json::from_value(v).unwrap(), 0, &g, 7, 9).is_err(),
                "{key}"
            );
        }
        let mut v = value;
        v["after"]["width"] = 4.into();
        assert!(validate_record(&serde_json::from_value(v).unwrap(), 0, &g, 7, 9).is_err());
    }
}
