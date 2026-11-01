// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real Map rejection, exact Unmap/ReleaseFrame and conversion-after-release.
use super::super::*;
use crate::{
    backends::windows::parity_adapter::{Adapter, Converter, Image},
    platform::windows::desktop_dup::map_probe,
};
use std::{error::Error as StdError, fs::File, io::Write, path::Path};

struct TracedConverter<'a>(Converter<'a>);
impl BgraConverter for TracedConverter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, view: BgraView<'_>) -> Result<Image, i32> {
        if map_probe::conversion() {
            return Err(2);
        }
        self.0.bgra_to_bgr(view)
    }
}
fn create(root: &Path, name: &str) -> std::io::Result<File> {
    File::options()
        .write(true)
        .create_new(true)
        .open(root.join(name))
}
fn save(
    root: &Path,
    name: &str,
    image: &Image,
    x: usize,
    y: usize,
) -> Result<i32, Box<dyn StdError>> {
    let (width, height) = image.dimensions();
    let width = usize::try_from(width)?;
    let height = usize::try_from(height)?;
    assert!(x.checked_add(96).unwrap() <= width && y.checked_add(64).unwrap() <= height);
    let stride = image.stride();
    let first = y
        .checked_mul(stride)
        .unwrap()
        .checked_add(x.checked_mul(3).unwrap())
        .unwrap();
    let marker = &image.bytes()[first..first + 3];
    let phase = if marker == [78, 95, 96] {
        0
    } else if marker == [26, 43, 60] {
        1
    } else {
        -1
    };
    let mut bytes = create(root, &format!("{name}.bgr"))?;
    let mut layout = create(root, &format!("{name}.txt"))?;
    writeln!(layout, "96\t64\t{stride}\t18432")?;
    for row in 0_usize..64 {
        let offset = first.checked_add(row.checked_mul(stride).unwrap()).unwrap();
        let pixels = &image.bytes()[offset..offset + 288];
        bytes.write_all(pixels)?;
    }
    bytes.flush()?;
    assert!(phase >= 0, "desktop target marker differs");
    for row in 0_usize..64 {
        for column in 0_usize..96 {
            let offset = first + row * stride + column * 3;
            let expected = if row < 16 && column < 16 {
                if phase == 0 {
                    [78, 95, 96]
                } else {
                    [26, 43, 60]
                }
            } else if column < 32 {
                [31, 83, 167]
            } else if row < 32 {
                [185, 77, 39]
            } else {
                [59, 226, 129]
            };
            assert_eq!(&image.bytes()[offset..offset + 3], &expected);
        }
    }
    Ok(phase)
}

#[test]
#[ignore = "explicit original/Rust real DesktopDup Map rejection and exact native cleanup records required"]
fn unmaps_and_releases_each_real_acquisition_before_conversion_and_after_map_rejection()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_MAP_ROOT").ok_or("missing desktop Map root")?,
    );
    assert!(root.is_absolute());
    let original = std::fs::read_to_string(root.join("source/map.tsv"))?;
    assert_eq!(
        original.lines().count(),
        41,
        "complete source native matrix required"
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let mut rows = create(&directory, "map.tsv")?;
    writeln!(
        rows,
        "cycle\tfault\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\tmapped_result\treleased\ttimeout\tsubresource\tpitch\tthread\tcontext\tresource\tduplication\tmarker"
    )?;
    let mut copies = create(&directory, "cpu-copy.tsv")?;
    writeln!(
        copies,
        "cycle\tphase\tsamples\tcopied\tfull_width\tfull_height"
    )?;
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let mut held = Vec::new();
    held.try_reserve_exact(60)?;
    for cycle in 0_u32..20 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.start_inside_desktop_updates()?;
        let threads = fixture.thread_info()?;
        let geometry = fixture.geometry([0; 6])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        let mut capture = DesktopDupCapture::new(fixture.hwnd());
        let mut converter = TracedConverter(adapter.converter(0));
        let warm = capture
            .capture(&mut converter)
            .image
            .map_err(|e| format!("desktop warm {e:?}"))?;
        assert!(!capture.just_initialized && capture.readable.is_some());
        let coordinates = capture
            .output_coordinates()
            .map_err(|e| format!("desktop output {e:?}"))?;
        let x = usize::try_from(geometry[16].checked_sub(i64::from(coordinates[0])).unwrap())?;
        let y = usize::try_from(geometry[17].checked_sub(i64::from(coordinates[1])).unwrap())?;
        save(&directory, &format!("{cycle}-warm"), &warm, x, y)?;
        held.push((format!("{cycle}-warm-held"), warm, x, y));
        let mut identities = None;
        for phase in ["probe", "recovery"] {
            let fault = if phase == "probe" { cycle % 3 } else { 0 };
            let guard = map_probe::Guard::begin(fault);
            let attempt = capture.capture(&mut converter);
            let trace = guard.trace();
            drop(guard);
            assert!(!trace.violation && !trace.mapped && !trace.held);
            assert_eq!(trace.order, if fault == 1 { 1235 } else { 123456 });
            assert_eq!(
                (
                    trace.acquires,
                    trace.copies,
                    trace.maps,
                    trace.unmaps,
                    trace.releases,
                    trace.conversions
                ),
                (1, 1, 1, u32::from(fault != 1), 1, u32::from(fault != 1))
            );
            assert!(trace.acquired >= 0 && trace.released >= 0);
            assert_eq!(trace.timeout, 2000);
            assert_eq!(trace.subresource, if fault == 1 { u32::MAX } else { 0 });
            assert_eq!(u64::from(trace.thread), threads[1]);
            assert_ne!(trace.context, 0);
            assert_ne!(trace.resource, 0);
            assert_ne!(trace.duplication, 0);
            let actual = (trace.context, trace.resource, trace.duplication);
            if let Some(expected) = identities {
                assert_eq!(actual, expected);
            } else {
                identities = Some(actual);
            }
            assert!(!capture.just_initialized && capture.readable.is_some());
            assert_eq!(attempt.evidence.sample_count, 1);
            let sample = &attempt.evidence.samples[0];
            assert_eq!(sample.acquire, Some(HRESULT(trace.acquired)));
            assert_eq!(sample.release, Some(HRESULT(trace.released)));
            let (width, height) = (capture.description.Width, capture.description.Height);
            let expected_copy = if fault == 1 {
                0
            } else {
                usize::try_from(width)?
                    .checked_mul(usize::try_from(height)?)
                    .unwrap()
                    .checked_mul(4)
                    .unwrap()
            };
            assert_eq!(sample.copied_bytes, expected_copy);
            let image = match fault {
                1 => {
                    assert!(trace.mapped_result < 0 && trace.pitch == 0);
                    assert!(
                        matches!(attempt.image, Err(Error::Capture(CaptureError::Native("Map", code))) if code.0 == trace.mapped_result)
                    );
                    None
                }
                2 => {
                    assert!(trace.mapped_result >= 0 && trace.pitch >= width * 4);
                    assert!(matches!(attempt.image, Err(Error::Conversion(2))));
                    None
                }
                _ => {
                    assert!(trace.mapped_result >= 0 && trace.pitch >= width * 4);
                    Some(
                        attempt
                            .image
                            .map_err(|e| format!("desktop cycle={cycle} phase={phase} {e:?}"))?,
                    )
                }
            };
            let marker = if let Some(image) = image {
                let marker = save(&directory, &format!("{cycle}-{phase}"), &image, x, y)?;
                held.push((format!("{cycle}-{phase}-held"), image, x, y));
                marker
            } else {
                -1
            };
            writeln!(
                rows,
                "{cycle}\t{fault}\t{phase}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{marker}",
                if fault == 0 {
                    1
                } else if fault == 1 {
                    0
                } else {
                    2
                },
                trace.order,
                trace.acquires,
                trace.copies,
                trace.maps,
                trace.unmaps,
                trace.releases,
                trace.conversions,
                trace.acquired,
                trace.mapped_result,
                trace.released,
                trace.timeout,
                trace.subresource,
                trace.pitch,
                trace.thread,
                trace.context,
                trace.resource,
                trace.duplication
            )?;
            writeln!(
                copies,
                "{cycle}\t{phase}\t1\t{expected_copy}\t{width}\t{height}"
            )?;
            rows.flush()?;
            copies.flush()?;
        }
        capture.uninit();
        drop(capture);
        drop(fixture);
    }
    assert_eq!(held.len(), 47);
    assert_eq!(adapter.image_owners()?, held.len() as u64);
    for (name, image, x, y) in &held {
        save(&directory, name, image, *x, *y)?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust DesktopDup map_boundary cycles=20 warm=20 observed_calls=40 map_failures=7 conversion_failures=6 recovery=20 retained=47 native_forwarded=1 owners_released=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
