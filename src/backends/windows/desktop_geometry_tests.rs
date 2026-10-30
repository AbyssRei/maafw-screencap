// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native post-correction capture matrix; source runs in its own process first.
use crate::backends::windows::{
    desktop_dup_window::DesktopDupWindowCapture,
    parity_adapter::{Adapter, Image},
};
use crate::platform::windows::{geometry, window_api::SystemWindow};
use std::{
    error::Error,
    fs::File,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

fn create(root: &Path, name: &str) -> std::io::Result<File> {
    File::options()
        .write(true)
        .create_new(true)
        .open(root.join(name))
}
fn save(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    create(root, name)?.write_all(bytes)
}
fn client(row: &[i64; 31]) -> [i64; 4] {
    [
        row[16] + row[12],
        row[17] + row[13],
        row[16] + row[14],
        row[17] + row[15],
    ]
}
fn target(row: &[i64; 31]) -> [i64; 4] {
    // Coordinates are observed Win32 i32 values widened before arithmetic.
    let c = client(row);
    if row[18] & 0x8000_0000 != 0 && c == row[..4] {
        return c;
    }
    let width = (c[2] - c[0]).min(row[6] - row[4]);
    let height = (c[3] - c[1]).min(row[7] - row[5]);
    let left = c[0].clamp(row[4], row[6] - width);
    let top = c[1].clamp(row[5], row[7] - height);
    [left, top, left + width, top + height]
}
fn snapshot(file: &mut File, index: usize, stage: &str, row: [i64; 31]) -> std::io::Result<()> {
    write!(file, "{index}\trust\t{stage}")?;
    for value in row {
        write!(file, "\t{value}")?;
    }
    writeln!(file)?;
    file.flush()
}
fn known(image: &Image) -> bool {
    let (w, h) = image.dimensions();
    super::fullscreen::known(
        image.bytes(),
        w.try_into().unwrap(),
        h.try_into().unwrap(),
        image.stride(),
    )
}

#[test]
#[ignore = "explicit native F DesktopDupWindow 38-case matrix; original source records and serial visible desktop"]
fn captures_all_corrected_native_clients_and_keeps_independent_target_owners()
-> Result<(), Box<dyn Error>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_DUP_REFERENCE_ROOT").ok_or("source records required")?,
    )
    .join("window-geometry");
    if !root.is_absolute() || !root.is_dir() {
        return Err("absolute existing desktop geometry records required".into());
    }
    let text = std::fs::read_to_string(root.join("source-cases.tsv"))?;
    let definitions = text
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        definitions.len(),
        38,
        "source must complete all matrix cases first"
    );
    save(&root, "rust-cases.tsv", text.as_bytes())?;
    let mut snapshots = create(&root, "rust-snapshots.tsv")?;
    let mut resources = create(&root, "rust-resources.tsv")?;
    let mut markers = create(&root, "rust-markers.tsv")?;
    let mut owners = create(&root, "rust-owners.tsv")?;
    let adapter = Adapter::load()?;
    for (index, definition) in definitions.iter().enumerate() {
        assert_eq!(definition.len(), 7);
        assert_eq!(definition[0].parse::<usize>()?, index);
        let values = definition[2..]
            .iter()
            .map(|v| v.parse::<i64>())
            .collect::<Result<Vec<_>, _>>()?;
        let rectangle: [i64; 4] = values[1..].try_into()?;
        let mut window = adapter.screen_fixture()?;
        let before = window.geometry([
            1,
            values[0],
            rectangle[0],
            rectangle[1],
            rectangle[2] - rectangle[0],
            rectangle[3] - rectangle[1],
        ])?;
        snapshot(&mut snapshots, index, "setup", before)?;
        assert_eq!(client(&before), rectangle, "actual requested client");
        let desired = target(&before);
        geometry::ensure_on_screen(&SystemWindow, window.hwnd())
            .map_err(|e| format!("matrix helper {e:?}"))?;
        // Wait only for asynchronous fixture geometry, before capture. No
        // failed capture or pixel assertion is retried.
        let deadline = Instant::now() + Duration::from_secs(3);
        let corrected = loop {
            let row = window.geometry([0; 6])?;
            if client(&row) == desired {
                break row;
            }
            if Instant::now() >= deadline {
                return Err(format!("native geometry timeout case {index}").into());
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        snapshot(&mut snapshots, index, "corrected", corrected)?;
        assert_eq!(corrected[20], before[20] + i64::from(desired != rectangle));
        window.geometry([2, 0, 0, 0, 0, 0])?;
        window.start_inside_desktop_updates()?;
        snapshot(&mut snapshots, index, "presented", window.geometry([0; 6])?)?;
        let baseline = adapter.objects();
        let image_baseline = adapter.image_owners()?;
        let mut held = Vec::new();
        {
            let mut capture = DesktopDupWindowCapture::new(window.hwnd());
            for call in 0..2 {
                let mut converter = adapter.converter(0);
                let attempt = capture.capture(&mut converter);
                println!(
                    "DD geometry case={index} call={call} evidence={:?}",
                    attempt.evidence
                );
                assert!(attempt.coordination.as_ref().unwrap().geometry.is_ok());
                let image = attempt
                    .image
                    .map_err(|e| format!("matrix capture {index}/{call}: {e:?}"))?;
                let (width, height) = image.dimensions();
                assert_eq!(
                    (i64::from(width), i64::from(height)),
                    (desired[2] - desired[0], desired[3] - desired[1])
                );
                let row = usize::try_from(width)?
                    .checked_mul(3)
                    .ok_or("row overflow")?;
                let span = row
                    .checked_mul(usize::try_from(height)?)
                    .ok_or("span overflow")?;
                assert_eq!(
                    attempt.target_copy_bytes, span,
                    "explicit target safety copy"
                );
                assert_eq!(image.stride(), row);
                assert_eq!(image.bytes().len(), span);
                let name = format!("{index}-rust-{call}");
                save(&root, &format!("{name}.bgr"), image.bytes())?;
                save(
                    &root,
                    &format!("{name}.txt"),
                    format!("{width}\t{height}\t{row}\t{span}\n").as_bytes(),
                )?;
                assert!(known(&image), "known target/marker case {index}/{call}");
                let after = window.geometry([0; 6])?;
                snapshot(&mut snapshots, index, &format!("capture-{call}"), after)?;
                assert_eq!(client(&after), desired);
                assert_eq!(
                    after[20], corrected[20],
                    "capture must not repeat correction"
                );
                held.push(image);
            }
        }
        let marker = window.desktop_update_info()?;
        assert_eq!(marker[0], u64::try_from(window.hwnd() as usize)?);
        assert!(marker[1] >= 1);
        assert_eq!(marker[2], 0);
        writeln!(
            markers,
            "{index}\trust\t{}\t{}\t{}",
            marker[0], marker[1], marker[2]
        )?;
        markers.flush()?;
        window.close()?;
        let closed = adapter.objects();
        writeln!(resources, "{index}\trust\t{baseline}\t{closed}")?;
        resources.flush()?;
        assert!(closed <= baseline, "fixture GDI owners after close");
        let retained_count = adapter.image_owners()?;
        assert_eq!(retained_count, image_baseline + 2);
        for (call, image) in held.iter().enumerate() {
            assert!(known(image), "held CPU target after native close");
            assert_eq!(
                image.bytes(),
                std::fs::read(root.join(format!("{index}-rust-{call}.bgr")))?
            );
            save(
                &root,
                &format!("{index}-rust-{call}.held.bgr"),
                image.bytes(),
            )?;
        }
        drop(held);
        let released = adapter.image_owners()?;
        assert_eq!(released, image_baseline);
        writeln!(
            owners,
            "{index}\t{image_baseline}\t{retained_count}\t{released}"
        )?;
        owners.flush()?;
        println!(
            "native DD geometry case={index} name={} captures=2 retained=2 passed",
            definition[1]
        );
    }
    save(&root, "rust-result.txt", b"cases=38\ncaptures=76\nretained_after_close=76\nknown_pixels=1\nrepeated_geometry=0\nimage_owners_released=1\n")?;
    println!(
        "native DesktopDupWindow geometry cases=38 captures=76 retained_after_close=76 known_target_with_marker=1 repeated_geometry=0 image_owners_released=1 passed"
    );
    Ok(())
}
