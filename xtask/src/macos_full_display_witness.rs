// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Verify independent CPU-view/window-list observations, never waive RGB parity.
use crate::macos_display_geometry::Geometry;
use crate::macos_same_binary::{fields, number, rows};
use crate::{Result, records};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct View {
    schema: u32,
    slot: u32,
    pid: u32,
    main_thread: u32,
    window_id: u32,
    width: usize,
    height: usize,
    source_bits_per_sample: u32,
    source_bits_per_pixel: u32,
    source_samples_per_pixel: u32,
    source_bytes_per_row: usize,
    source_bitmap_format: u64,
    source_has_alpha: bool,
    source_planar: bool,
    canonical_format: String,
    canonical_stride: usize,
    top_left_origin: u32,
    origin: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Census {
    schema: u32,
    slot: u32,
    pid: u32,
    main_thread: u32,
    window_id: u32,
    display_id: u32,
    windows: Vec<Window>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    rank: usize,
    id: u32,
    pid: u32,
    layer: i32,
    onscreen: bool,
    bounds: [f64; 4],
    owner_name: Option<String>,
    name: Option<String>,
}

pub fn differences(g: &Geometry, bytes: &[u8], channels: usize) -> Result<serde_json::Value> {
    let pixels = g.width.checked_mul(g.height).ok_or("pixel span overflow")?;
    if !matches!(channels, 3 | 4)
        || pixels.checked_mul(channels).ok_or("pixel bytes overflow")? != bytes.len()
    {
        return Err("full-display diagnostic pixel span differs".into());
    }
    let mut count = 0_u64;
    let mut changed_channels = 0_u64;
    let mut bounds = [g.width, g.height, 0, 0];
    let mut first = Vec::new();
    for (index, pixel) in bytes.chunks_exact(channels).enumerate() {
        let x = index % g.width;
        let y = index / g.width;
        let expected = g.expected_bgr(x, y);
        let changed = pixel[..3]
            .iter()
            .zip(expected)
            .filter(|(a, b)| **a != *b)
            .count()
            + usize::from(channels == 4 && pixel[3] != 255);
        if changed != 0 {
            count = count.checked_add(1).ok_or("difference count overflow")?;
            changed_channels = changed_channels
                .checked_add(u64::try_from(changed)?)
                .ok_or("channel count overflow")?;
            bounds = [
                bounds[0].min(x),
                bounds[1].min(y),
                bounds[2].max(x + 1),
                bounds[3].max(y + 1),
            ];
            if first.len() < 64 {
                first.push(serde_json::json!({"x":x,"y":y,"actual":pixel,"expected_bgr":expected}));
            }
        }
    }
    Ok(
        serde_json::json!({"differing_pixels":count,"differing_channels":changed_channels,
        "bounds":if count==0 {None} else {Some(bounds)},"first_64_differences":first,
        "all_pixels_examined":pixels,"known_full_rgb":count==0,"pixels_masked":0}),
    )
}

pub fn validate(
    arm: &Path,
    g: &Geometry,
    mode: u64,
    sequence: &[Vec<u64>],
    level: u64,
) -> Result<serde_json::Value> {
    if fs::read(arm.join("witness-enabled.txt"))?
        != b"schema=1\nrequests=7\nstrict_full_rgb_oracle=1\n"
        || sequence.len() != 6
    {
        return Err("missing explicit full-display witness scope".into());
    }
    let text = fs::read_to_string(arm.join("witness.tsv"))?;
    let mut lines = text.lines();
    if lines.next() != Some("slot\tbegin_ns\tend_ns") {
        return Err("invalid witness interval header".into());
    }
    let times = rows(&lines.collect::<Vec<_>>().join("\n"), 3)?;
    if times.len() != 7 {
        return Err("expected all seven view/census witnesses".into());
    }
    let scene = arm.join("scene");
    let geometry_text = fs::read_to_string(arm.join("consumer-expected-geometry.tsv"))?;
    let geometry_row = geometry_text.trim_end().split('\t').collect::<Vec<_>>();
    let origin = [
        geometry_row
            .get(4)
            .ok_or("missing display x")?
            .parse::<f64>()?,
        geometry_row
            .get(5)
            .ok_or("missing display y")?
            .parse::<f64>()?,
    ];
    let summary = fields(&fs::read_to_string(scene.join("witness-summary.txt"))?)?;
    if summary.len() != 7 {
        return Err("unexpected witness closure fields".into());
    }
    for (key, value) in [
        ("schema", 1),
        ("requests", 7),
        ("view_draws", 7),
        ("census_queries", 7),
        ("main_thread", 1),
        ("window_id", u64::from(g.window)),
        ("window_closed", 1),
    ] {
        if number(&summary, key)? != value {
            return Err("view/census closure differs".into());
        }
    }
    let mut observations = Vec::new();
    for (slot, row) in times.iter().enumerate() {
        if row[0] != slot as u64
            || row[1] == 0
            || row[2] <= row[1]
            || slot > 0 && row[1] < sequence[slot - 1][4]
            || slot < 6 && row[2] > sequence[slot][3]
        {
            return Err("witness does not lie between its actual capture calls".into());
        }
        if fs::read(scene.join(format!("witness-request-{slot}")))?
            != format!("slot={slot}\n").as_bytes()
            || fs::read(scene.join(format!("witness-{slot}-ready.txt")))?
                != format!("slot={slot}\nmain_thread=1\n").as_bytes()
        {
            return Err("native witness request/ack differs".into());
        }
        let view_path = scene.join(format!("view-{slot}.json"));
        let view: View = records::read(&view_path)?;
        if view.schema != 1
            || view.slot as usize != slot
            || view.pid != g.pid
            || view.main_thread != 1
            || view.window_id != g.window
            || (view.width, view.height) != (g.width, g.height)
            || view.source_bits_per_sample == 0
            || view.source_bits_per_pixel == 0
            || view.source_samples_per_pixel == 0
            || view.source_bytes_per_row == 0
            || view.canonical_format != "BGRA8_sRGB"
            || view.canonical_stride != g.width.checked_mul(4).ok_or("view stride overflow")?
            || view.top_left_origin != 1
            || view.origin != "NSView_cacheDisplay_semantic_NSColor"
        {
            return Err("native CPU view identity/layout differs".into());
        }
        let pixels = fs::read(scene.join(format!("view-{slot}.bgra")))?;
        g.pixels(&pixels, 4)?; // Strict known pixels apply to every CPU-view pixel.
        let census_path = scene.join(format!("census-{slot}.json"));
        let census: Census = records::read(&census_path)?;
        if census.schema != 1
            || census.slot as usize != slot
            || census.pid != g.pid
            || census.main_thread != 1
            || census.window_id != g.window
            || census.display_id != g.display
            || census.windows.is_empty()
        {
            return Err("native window census identity differs".into());
        }
        let mut unique = BTreeSet::new();
        let mut own = 0;
        let mut candidates = Vec::new();
        for (rank, window) in census.windows.iter().enumerate() {
            if window.rank != rank
                || window.id == 0
                || !unique.insert(window.id)
                || !window.onscreen
                || window.bounds.iter().any(|v| !v.is_finite())
                || window.bounds[2] < 0.0
                || window.bounds[3] < 0.0
            {
                return Err("invalid or repeated native window census row".into());
            }
            if window.id == g.window {
                own += 1;
                if window.pid != g.pid
                    || i64::from(window.layer) != i64::try_from(level)?
                    || window.bounds[..2] != origin
                    || window.bounds[2] != g.width as f64
                    || window.bounds[3] != g.height as f64
                {
                    return Err("census substituted the owned view".into());
                }
            } else {
                // Record all other actual windows. No guessed indicator owner,
                // overlap mask, attribution or capture exemption is applied.
                candidates.push(serde_json::json!({"rank":rank,"id":window.id,"pid":window.pid,
                    "layer":window.layer,"bounds":window.bounds,"owner_name":window.owner_name,"name":window.name}));
            }
        }
        if own != 1 {
            return Err("census does not contain the exact owned full-screen window".into());
        }
        observations.push(serde_json::json!({"slot":slot,"interval":row,"view_known_full_rgb":true,
            "view_metadata":serde_json::from_slice::<serde_json::Value>(&fs::read(&view_path)?)?,
            "view_source_bitmap_format":view.source_bitmap_format,"view_source_has_alpha":view.source_has_alpha,
            "view_source_planar":view.source_planar,"census":serde_json::from_slice::<serde_json::Value>(&fs::read(&census_path)?)?,
            "other_windows":candidates,"compositor_content_proved":false,"dot_owner_proved":false}));
    }
    Ok(
        serde_json::json!({"mode":mode,"view_images":7,"window_censuses":7,"observations":observations,
        "scope":"Native AppKit redraw into CPU storage plus onscreen CG metadata between calls. These are not WindowServer screenshots, simultaneous frame observations, proof of a dot producer or an exception to strict display RGB parity. Interleaving this work can affect scheduling."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_every_pixel_and_alpha_difference_without_masking_the_edge() {
        let g = crate::macos_display_geometry::parse_full(
            "1\t17\t3\t2\t0\t0\t3\t2\t91\t7\t0\t0\t3\t2\t1\t0\n",
        )
        .unwrap();
        let mut bytes = Vec::new();
        for y in 0..2 {
            for x in 0..3 {
                bytes.extend_from_slice(&g.expected_bgr(x, y));
                bytes.push(255);
            }
        }
        assert_eq!(differences(&g, &bytes, 4).unwrap()["differing_pixels"], 0);
        bytes[23] = 0;
        let result = differences(&g, &bytes, 4).unwrap();
        assert_eq!(result["differing_pixels"], 1);
        assert_eq!(result["bounds"], serde_json::json!([2, 1, 3, 2]));
        assert_eq!(result["pixels_masked"], 0);
        assert!(differences(&g, &bytes[..23], 4).is_err());
    }
}
