// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Independent owned-window fixture metadata, never a display pixel waiver.
use crate::Result;
use crate::macos_display_geometry::Geometry;
use crate::macos_same_binary::{fields, number};
use std::{collections::BTreeMap, fs, path::Path};

fn control(
    values: &BTreeMap<String, String>,
    pid: u64,
    window: u64,
    observed: u64,
) -> Result<serde_json::Value> {
    if values.len() != 12
        || values.get("level_source").map(String::as_str) != Some("CGWindowLevelForKey")
        || values.get("key").map(String::as_str) != Some("kCGMaximumWindowLevelKey")
    {
        return Err("owned maximum-level fixture fields/API identity differ".into());
    }
    let maximum = number(values, "maximum_level")?;
    let ordinary = number(values, "screen_saver_level")?;
    if ordinary == 0 || maximum <= ordinary || maximum > i32::MAX as u64 || observed != maximum {
        return Err("owned maximum-level fixture ordering/native level differs".into());
    }
    for (key, value) in [
        ("schema", 1),
        ("level_queries", 2),
        ("requested_level", maximum),
        ("observed_level", observed),
        ("main_thread", 1),
        ("owner_pid", pid),
        ("window_id", window),
        ("window_closed", 1),
    ] {
        if number(values, key)? != value {
            return Err("owned maximum-level fixture owner/query/closure differs".into());
        }
    }
    Ok(
        serde_json::json!({"schema":1,"level_source":"CGWindowLevelForKey",
        "key":"kCGMaximumWindowLevelKey","level_queries":2,"screen_saver_level":ordinary,
        "maximum_level":maximum,"requested_level":maximum,"observed_level":observed,
        "main_thread":1,"owner_pid":pid,"window_id":window,"window_closed":1}),
    )
}

pub fn validate(arm: &Path, geometry: &Geometry, observed: u64) -> Result<serde_json::Value> {
    if fs::read(arm.join("maximum-level-enabled.txt"))?
        != b"schema=1\nmaximum_window_level_fixture=1\n"
    {
        return Err("maximum-level fixture was not explicitly selected".into());
    }
    let values = fields(&fs::read_to_string(
        arm.join("scene/maximum-level-control.txt"),
    )?)?;
    for slot in 0..7 {
        for kind in ["view", "census"] {
            let value: serde_json::Value =
                crate::records::read(&arm.join(format!("scene/{kind}-{slot}.json")))?;
            if value["schema"] != 2 {
                return Err(
                    "maximum-level fixture requires current prepared view/census schemas".into(),
                );
            }
        }
    }
    control(
        &values,
        u64::from(geometry.pid),
        u64::from(geometry.window),
        observed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> BTreeMap<String, String> {
        fields("schema=1\nlevel_source=CGWindowLevelForKey\nkey=kCGMaximumWindowLevelKey\nlevel_queries=2\nscreen_saver_level=1000\nmaximum_level=2147483631\nrequested_level=2147483631\nobserved_level=2147483631\nmain_thread=1\nowner_pid=21\nwindow_id=37\nwindow_closed=1\n").unwrap()
    }
    #[test]
    fn accepts_matching_public_level_and_owned_window_identity() {
        let value = control(&fixture(), 21, 37, 2147483631).unwrap();
        assert_eq!(value["maximum_level"], 2147483631_u64);
        assert_eq!(value["screen_saver_level"], 1000);
    }
    #[test]
    fn rejects_substituted_level_owner_query_and_closure() {
        for (key, value) in [
            ("key", "kCGScreenSaverWindowLevelKey"),
            ("level_source", "invented"),
            ("level_queries", "1"),
            ("maximum_level", "1000"),
            ("maximum_level", "2147483648"),
            ("screen_saver_level", "0"),
            ("requested_level", "1000"),
            ("observed_level", "1000"),
            ("main_thread", "0"),
            ("owner_pid", "22"),
            ("window_id", "38"),
            ("window_closed", "0"),
        ] {
            let mut values = fixture();
            values.insert(key.into(), value.into());
            assert!(control(&values, 21, 37, 2147483631).is_err(), "{key}");
        }
        assert!(control(&fixture(), 21, 37, 1000).is_err());
        let mut values = fixture();
        values.insert("extra".into(), "1".into());
        assert!(control(&values, 21, 37, 2147483631).is_err());
    }
}
