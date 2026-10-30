// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual F exact-popup capture with an in-target update, without an outside
//! marker that would overlap a fullscreen target. This is one geometry case.
use crate::backends::windows::{
    desktop_dup_window::DesktopDupWindowCapture,
    parity_adapter::{Adapter, Image},
};
use std::{error::Error, fs::File, io::Write, path::Path};

fn known(bytes: &[u8], width: usize, height: usize, stride: usize) -> bool {
    let Some(row) = width.checked_mul(3) else {
        return false;
    };
    let Some(span) = stride.checked_mul(height) else {
        return false;
    };
    if width < 16 || height < 16 || stride != row || bytes.len() != span {
        return false;
    }
    let marker = &bytes[..3];
    if marker != [0x1a, 0x2b, 0x3c] && marker != [0x4e, 0x5f, 0x60] {
        return false;
    }
    for y in 0..height {
        for x in 0..width {
            let expected: &[u8] = if x < 16 && y < 16 {
                marker
            } else if x < width / 3 {
                &[0x1f, 0x53, 0xa7]
            } else if y < height / 2 {
                &[0xb9, 0x4d, 0x27]
            } else {
                &[0x3b, 0xe2, 0x81]
            };
            if &bytes[y * stride + x * 3..y * stride + x * 3 + 3] != expected {
                return false;
            }
        }
    }
    true
}
fn save(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = File::options()
        .create_new(true)
        .write(true)
        .open(root.join(name))?;
    file.write_all(bytes)
}
fn snapshot(file: &mut File, row: [i64; 31]) -> std::io::Result<()> {
    for (index, value) in row.into_iter().enumerate() {
        if index != 0 {
            write!(file, "\t")?;
        }
        write!(file, "{value}")?;
    }
    writeln!(file)?;
    file.flush()
}
#[test]
#[ignore = "explicit F fullscreen DesktopDupWindow capture, original source and serial visible desktop"]
fn captures_exact_popup_without_work_area_crop_and_keeps_owners_after_close()
-> Result<(), Box<dyn Error>> {
    let source_root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_DUP_REFERENCE_ROOT").ok_or("source records required")?,
    )
    .join("window-fullscreen");
    if !source_root.is_absolute() {
        return Err("absolute fullscreen source records required".into());
    }
    let root = source_root.join("rust");
    if !root.is_dir() {
        return Err("fresh fullscreen Rust evidence directory required".into());
    }
    let source_geometry = std::fs::read_to_string(source_root.join("geometry.txt"))?
        .split_whitespace()
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()?;
    if source_geometry.len() != 11 {
        return Err("incomplete original fullscreen geometry".into());
    }
    let width = usize::try_from(source_geometry[0])?;
    let height = usize::try_from(source_geometry[1])?;
    let stride = width.checked_mul(3).ok_or("row overflow")?;
    let source = std::fs::read(source_root.join("target.bgr"))?;
    assert!(
        known(&source, width, height, stride),
        "independently known source pixels"
    );
    assert_eq!(
        std::fs::read(source_root.join("held-after-close.bgr"))?,
        source
    );
    let adapter = Adapter::load()?;
    let image_baseline = adapter.image_owners()?;
    let mut fixture = adapter.screen_fixture()?;
    let initial = fixture.geometry([0; 6])?;
    assert_ne!(
        initial[..4],
        initial[4..8],
        "visible taskbar work-area distinction required"
    );
    assert_eq!(
        &initial[..4],
        &source_geometry[3..7],
        "same native monitor premise"
    );
    let setup = fixture.geometry([
        1,
        0x9000_0000,
        initial[0],
        initial[1],
        initial[2] - initial[0],
        initial[3] - initial[1],
    ])?;
    assert_eq!(setup[16..18], setup[..2]);
    assert_eq!(
        (setup[14], setup[15]),
        (i64::try_from(width)?, i64::try_from(height)?)
    );
    let mut snapshots = File::options()
        .create_new(true)
        .write(true)
        .open(root.join("snapshots.tsv"))?;
    snapshot(&mut snapshots, setup)?;
    fixture.start_inside_desktop_updates()?;
    let mut retained: Vec<Image> = Vec::new();
    {
        let mut capture = DesktopDupWindowCapture::new(fixture.hwnd());
        for call in 0..2 {
            let mut converter = adapter.converter(0);
            let attempt = capture.capture(&mut converter);
            println!(
                "fullscreen DesktopDupWindow call={call} evidence={:?}",
                attempt.evidence
            );
            assert!(attempt.coordination.as_ref().unwrap().geometry.is_ok());
            let image = attempt
                .image
                .map_err(|e| format!("fullscreen capture {call}: {e:?}"))?;
            assert_eq!(
                attempt.target_copy_bytes,
                width
                    .checked_mul(height)
                    .and_then(|n| n.checked_mul(3))
                    .ok_or("span overflow")?
            );
            assert_eq!(
                image.dimensions(),
                (i32::try_from(width)?, i32::try_from(height)?)
            );
            save(&root, &format!("capture-{call}.bgr"), image.bytes())?;
            save(
                &root,
                &format!("capture-{call}.txt"),
                format!(
                    "{width}\t{height}\t{}\t{}\n",
                    image.stride(),
                    image.bytes().len()
                )
                .as_bytes(),
            )?;
            assert!(
                known(image.bytes(), width, height, image.stride()),
                "full known target including changing marker"
            );
            let after = fixture.geometry([0; 6])?;
            snapshot(&mut snapshots, after)?;
            assert_eq!(after[16..18], setup[16..18]);
            assert_eq!(after[12..16], setup[12..16]);
            assert_eq!(
                after[20], setup[20],
                "exact popup must not trigger work-area correction"
            );
            retained.push(image);
        }
    }
    let marker = fixture.desktop_update_info()?;
    assert_eq!(marker[0], u64::try_from(fixture.hwnd() as usize)?);
    assert!(marker[1] >= 1);
    assert_eq!(marker[2], 0);
    save(
        &root,
        "marker.tsv",
        format!("{}\t{}\t{}\n", marker[0], marker[1], marker[2]).as_bytes(),
    )?;
    fixture.close()?;
    assert_eq!(adapter.image_owners()?, image_baseline + 2);
    for (call, image) in retained.iter().enumerate() {
        save(&root, &format!("held-{call}.bgr"), image.bytes())?;
        assert!(known(image.bytes(), width, height, image.stride()));
        assert_eq!(
            image.bytes(),
            std::fs::read(root.join(format!("capture-{call}.bgr")))?
        );
    }
    drop(retained);
    assert_eq!(adapter.image_owners()?, image_baseline);
    save(&root,"result.txt",b"cases=1\ncaptures=2\nretained_after_close=2\nknown_pixels=1\ngeometry_adjustments=0\nimage_owners_released=1\n")?;
    println!(
        "native DesktopDupWindow fullscreen cases=1 captures=2 known_target_with_marker=1 work_area_crop=0 geometry_adjustments=0 retained_after_close=2 image_owners_released=1 passed"
    );
    Ok(())
}

#[test]
fn rejects_corrupted_marker_and_other_target_pixels() {
    let mut bytes = vec![0; 24 * 24 * 3];
    for y in 0..24 {
        for x in 0..24 {
            let color = if x < 16 && y < 16 {
                [0x1a, 0x2b, 0x3c]
            } else if x < 8 {
                [0x1f, 0x53, 0xa7]
            } else if y < 12 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            bytes[(y * 24 + x) * 3..(y * 24 + x + 1) * 3].copy_from_slice(&color);
        }
    }
    assert!(known(&bytes, 24, 24, 72));
    for offset in [0, 3, (23 * 24 + 23) * 3] {
        let old = bytes[offset];
        bytes[offset] ^= 1;
        assert!(!known(&bytes, 24, 24, 72));
        bytes[offset] = old;
    }
    assert!(!known(&bytes, 24, 24, 73));
    assert!(!known(&bytes, usize::MAX, 24, 72));
}
