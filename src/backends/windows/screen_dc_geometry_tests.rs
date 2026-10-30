// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! P1 native geometry completion followed by unchanged ScreenDC captures.
use super::*;
use crate::backends::windows::parity_adapter::{Adapter, Image};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Case {
    name: String,
    style: u32,
    client: [i64; 4],
}
const POPUP: u32 = 0x9000_0000; // WS_POPUP | WS_VISIBLE, matching source predicate.
const ORDINARY: u32 = 0x10cf_0000; // WS_VISIBLE | WS_OVERLAPPEDWINDOW.

fn cases(monitor: [i64; 4], work: [i64; 4]) -> Vec<Case> {
    let mut cases = vec![Case {
        name: "exact_popup".into(),
        style: POPUP,
        client: monitor,
    }];
    for edge in 0..4 {
        for delta in [-9, -8, -7, -1, 1, 7, 8, 9] {
            let mut client = monitor;
            client[edge] = client[edge].checked_add(delta).unwrap();
            cases.push(Case {
                name: format!("edge_{edge}_{delta}"),
                style: POPUP,
                client,
            });
        }
    }
    for (name, style, client) in [
        ("exact_bordered_popup", POPUP | 0x0004_0000, monitor),
        ("ordinary_monitor", ORDINARY, monitor),
        ("ordinary_work", ORDINARY, work),
        (
            "review_negative_one_translation",
            POPUP,
            [monitor[0] - 1, monitor[1], monitor[2] - 1, monitor[3]],
        ),
        (
            "oversized_popup",
            POPUP,
            [
                monitor[0] - 9,
                monitor[1] - 9,
                monitor[2] + 9,
                monitor[3] + 9,
            ],
        ),
    ] {
        cases.push(Case {
            name: name.into(),
            style,
            client,
        });
    }
    cases
}

fn client(snapshot: &[i64; 31]) -> [i64; 4] {
    [
        snapshot[16].checked_add(snapshot[12]).unwrap(),
        snapshot[17].checked_add(snapshot[13]).unwrap(),
        snapshot[16].checked_add(snapshot[14]).unwrap(),
        snapshot[17].checked_add(snapshot[15]).unwrap(),
    ]
}
fn expected(snapshot: &[i64; 31]) -> [i64; 4] {
    let original = client(snapshot);
    if snapshot[18] & 0x8000_0000 != 0 && original == snapshot[0..4] {
        return original;
    }
    // Native RECT values are i32, widened before differences and sums. The
    // contract clamps size first, then positions that size within rcWork.
    let width = (original[2] - original[0]).min(snapshot[6] - snapshot[4]);
    let height = (original[3] - original[1]).min(snapshot[7] - snapshot[5]);
    let left = original[0].clamp(snapshot[4], snapshot[6] - width);
    let top = original[1].clamp(snapshot[5], snapshot[7] - height);
    [left, top, left + width, top + height]
}
fn snapshot_row(file: &mut File, index: usize, side: &str, stage: &str, row: [i64; 31]) {
    write!(file, "{index}\t{side}\t{stage}").unwrap();
    for value in row {
        write!(file, "\t{value}").unwrap();
    }
    writeln!(file).unwrap();
    file.flush().unwrap();
}
fn save_image(root: &Path, index: usize, side: &str, cycle: usize, image: &Image) {
    let name = format!("{index}-{side}-{cycle}");
    let bytes = root.join(format!("{name}.bgr"));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(bytes)
        .unwrap();
    output.write_all(image.bytes()).unwrap();
    let (width, height) = image.dimensions();
    std::fs::write(
        root.join(format!("{name}.txt")),
        format!(
            "{width}\t{height}\t{}\t{}\n",
            image.stride(),
            image.bytes().len()
        ),
    )
    .unwrap();
}

#[test]
#[ignore = "explicit native F geometry/ScreenDC matrix; serial visible desktop and adapter required"]
fn captures_corrected_native_clients_without_repeated_adjustment() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(
        std::env::var_os("MFS_SCREEN_GEOMETRY_EVIDENCE")
            .ok_or("MFS_SCREEN_GEOMETRY_EVIDENCE required")?,
    );
    if !root.is_absolute() || !root.is_dir() {
        return Err("ScreenDC geometry evidence must be an existing absolute directory".into());
    }
    let mut definitions = File::options()
        .write(true)
        .create_new(true)
        .open(root.join("cases.tsv"))?;
    let mut observations = File::options()
        .write(true)
        .create_new(true)
        .open(root.join("snapshots.tsv"))?;
    let mut resources = File::options()
        .write(true)
        .create_new(true)
        .open(root.join("resources.tsv"))?;
    let adapter = Adapter::load()?;
    let mut control = adapter.screen_fixture()?;
    let initial = control.geometry([0; 6])?;
    control.close()?;
    let matrix = cases(
        initial[0..4].try_into().unwrap(),
        initial[4..8].try_into().unwrap(),
    );
    assert_eq!(matrix.len(), 38);
    for (index, case) in matrix.iter().enumerate() {
        writeln!(
            definitions,
            "{index}\t{}\t{}\t{}\t{}\t{}\t{}",
            case.name, case.style, case.client[0], case.client[1], case.client[2], case.client[3]
        )?;
        definitions.flush()?;
        for side in ["source", "rust"] {
            let mut window = adapter.screen_fixture()?;
            let before = window.geometry([
                1,
                i64::from(case.style),
                case.client[0],
                case.client[1],
                case.client[2] - case.client[0],
                case.client[3] - case.client[1],
            ])?;
            snapshot_row(&mut observations, index, side, "setup", before);
            assert_eq!(
                client(&before),
                case.client,
                "native fixture setup {} {side}",
                case.name
            );
            let target = expected(&before);
            if side == "source" {
                adapter.screen_ensure_geometry(window.hwnd())?;
            } else {
                geometry::ensure_on_screen(&SystemWindow, window.hwnd())
                    .map_err(|e| format!("Rust native helper {e:?}"))?;
            }
            // Only wait for the fixture's real asynchronous geometry messages.
            // No capture has occurred and no pixel/capture failure is retried.
            let deadline = Instant::now() + Duration::from_secs(3);
            let corrected = loop {
                let row = window.geometry([0; 6])?;
                if client(&row) == target {
                    break row;
                }
                if Instant::now() >= deadline {
                    snapshot_row(&mut observations, index, side, "timeout", row);
                    return Err(format!(
                        "native geometry completion {} {side}: {:?} != {target:?}",
                        case.name,
                        client(&row)
                    )
                    .into());
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            snapshot_row(&mut observations, index, side, "corrected", corrected);
            let changed = i64::from(target != case.client);
            assert_eq!(
                corrected[20],
                before[20] + changed,
                "native geometry messages {} {side}",
                case.name
            );
            let presented = window.geometry([2, 0, 0, 0, 0, 0])?;
            snapshot_row(&mut observations, index, side, "presented", presented);
            assert_eq!(adapter.screen_present()?, [1, 0]);
            let baseline = adapter.objects();
            let mut held = Vec::new();
            {
                let mut capture = ScreenDcCapture::new(window.hwnd());
                for cycle in 0..2 {
                    let image = if side == "source" {
                        adapter
                            .screen_reference(window.hwnd())
                            .map_err(|s| format!("source capture {s}"))?
                    } else {
                        let mut converter = adapter.converter(0);
                        let attempt = capture.capture(&mut converter);
                        assert!(
                            attempt.cleanup.0.iter().all(Option::is_none),
                            "native cleanup"
                        );
                        assert!(
                            attempt.coordination.unwrap().geometry.is_ok(),
                            "Rust geometry coordination"
                        );
                        attempt
                            .image
                            .map_err(|e| format!("Rust native ScreenDC {e:?}"))?
                    };
                    save_image(&root, index, side, cycle, &image);
                    assert_eq!(
                        image.dimensions(),
                        (
                            i32::try_from(target[2] - target[0])?,
                            i32::try_from(target[3] - target[1])?
                        )
                    );
                    assert_eq!(
                        first_nonfixture_pixel(&image),
                        None,
                        "known pixels {} {side} {cycle}",
                        case.name
                    );
                    let row = window.geometry([0; 6])?;
                    snapshot_row(
                        &mut observations,
                        index,
                        side,
                        &format!("capture-{cycle}"),
                        row,
                    );
                    assert_eq!(client(&row), target, "stable client geometry");
                    assert_eq!(
                        row[20], corrected[20],
                        "capture must not repeat geometry correction"
                    );
                    assert_eq!(adapter.objects(), baseline, "capture GDI resources");
                    held.push(image);
                }
            }
            window.close()?;
            let closed = adapter.objects();
            writeln!(resources, "{index}\t{side}\t{baseline}\t{closed}")?;
            resources.flush()?;
            assert!(
                closed <= baseline,
                "owned fixture GDI resources after close"
            );
            for image in &held {
                assert_eq!(
                    first_nonfixture_pixel(image),
                    None,
                    "CPU owner after native close"
                );
            }
        }
        println!("native ScreenDC geometry case {index} {} passed", case.name);
    }
    println!(
        "native ScreenDC geometry cases=38 sides=76 captures=152 retained_after_close=152 pixels=known repeated_geometry=0 passed"
    );
    Ok(())
}
