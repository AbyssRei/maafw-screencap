// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fixed-call alpha transitions, compared with the unchanged source diagnostic.
//! Transparent black/prior results are observations, never known-color success.
use super::*;
use crate::backends::windows::{
    parity_adapter::{Adapter, Converter as CpuConverter, Image},
    print_window::PrintWindowCapture,
};
use std::{error::Error as StdError, fs, io::Write, path::Path};

struct Observed<'a> {
    inner: CpuConverter<'a>,
    raw: Option<Vec<u8>>,
}
impl BgraConverter for Observed<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Image, i32> {
        assert!(self.raw.is_none());
        assert_eq!((input.width, input.height, input.stride), (96, 64, 384));
        assert_eq!(input.pixels.len(), 24576);
        // Test-only extra CPU copy at the existing conversion boundary. Native
        // storage never escapes; the original conversion still runs before Unmap.
        self.raw = Some(input.pixels.to_vec());
        self.inner.bgra_to_bgr(input)
    }
}
impl Converter for Observed<'_> {
    fn clone_image(&mut self, image: &Image) -> Result<Image, i32> {
        self.inner.clone_image(image)
    }
}
fn palette(alternative: bool) -> Vec<u8> {
    let colors = if alternative {
        [[0xe3, 0x42, 0x19], [0x57, 0x8b, 0xc1], [0x9d, 0x23, 0xf5]]
    } else {
        [[0x1f, 0x53, 0xa7], [0xb9, 0x4d, 0x27], [0x3b, 0xe2, 0x81]]
    };
    (0..64)
        .flat_map(|y| {
            (0..96).flat_map(move |x| {
                colors[if x < 32 {
                    0
                } else if y < 32 {
                    1
                } else {
                    2
                }]
            })
        })
        .collect()
}
fn save(root: &Path, name: &str, image: Option<&Image>) -> Result<(), Box<dyn StdError>> {
    let (layout, bytes) = image.map_or_else(
        || ("0\t0\t0\t0\n".to_owned(), &[][..]),
        |i| {
            (
                format!(
                    "{}\t{}\t{}\t{}\n",
                    i.dimensions().0,
                    i.dimensions().1,
                    i.stride(),
                    i.bytes().len()
                ),
                i.bytes(),
            )
        },
    );
    fs::write(root.join(format!("{name}.bgr")), bytes)?;
    fs::write(root.join(format!("{name}-layout.tsv")), layout)?;
    Ok(())
}
#[test]
#[ignore = "explicit paired FramePool alpha source data, native windows and PrintWindow content witnesses"]
fn records_original_alpha_sequence_pixels_and_held_owners() -> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_ALPHA_RUST_ROOT").ok_or("missing alpha Rust root")?,
    );
    assert!(root.is_absolute());
    fs::create_dir(&root)?;
    // SAFETY: Own exactly one explicit MTA reference on this fresh test worker.
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let mut dispatcher = HostDispatcher::new()?;
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let close_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut rows = fs::File::create_new(root.join("observations.tsv"))?;
    writeln!(
        rows,
        "arm\tphase\talpha\tframe_image\tframe_known\tframe_zero\tprint_image\tprint_known\tpaint_before\tpaint_after\tframe_prior\telapsed_us\treplayed\tconversions\tclones\tnative_pitch\tcopied_bytes\traw_present"
    )?;
    let mut identities = fs::File::create_new(root.join("owners.tsv"))?;
    let mut branches = fs::File::create_new(root.join("poll-branches.tsv"))?;
    writeln!(
        branches,
        "arm\tphase\tdrain_calls\tdrain_frames\tpoll_calls\tpoll_frames\tcache_clones\terrors"
    )?;
    let mut held = Vec::new();
    let mut raw_held = Vec::new();
    let mut positive_controls = true;
    let mut raw_count = 0;
    let mut replays = 0;
    for arm in 0..2 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.prepare_pseudo(false)?;
        let info = fixture.thread_info()?;
        writeln!(
            identities,
            "{arm}\t{}\t{}\t{}\t{}\t{}",
            fixture.hwnd().addr(),
            info[0],
            info[1],
            info[2],
            info[3]
        )?;
        let mut persistent: Option<FramePoolCapture<Image>> = None;
        for (phase, alpha) in [255, 0, 255, 0, 255].into_iter().enumerate() {
            let alternative = alpha == 0;
            fixture.alpha_scene(alternative)?;
            let before_paint = fixture.stats()?[0];
            fixture.restore_alpha_scene(alpha)?;
            let after_paint = fixture.stats()?[0];
            let name = format!("{}-{phase}", if arm == 0 { "fresh" } else { "persistent" });
            let directory = root.join(&name);
            fs::create_dir(&directory)?;
            let before = fixture.pseudo_state()?;
            let mut fresh;
            let capture = if arm == 0 {
                fresh = FramePoolCapture::new(fixture.hwnd());
                &mut fresh
            } else {
                persistent.get_or_insert_with(|| FramePoolCapture::new(fixture.hwnd()))
            };
            let mut converter = Observed {
                inner: adapter.converter(0),
                raw: None,
            };
            let began = Instant::now();
            let result = capture.capture(&mut converter); // one distinct call, no screenshot retry
            let elapsed = began.elapsed().as_micros();
            let after = fixture.pseudo_state()?;
            let mut states = fs::File::create_new(directory.join("states.tsv"))?;
            for (label, values) in [("before_capture", before), ("after_capture", after)] {
                writeln!(
                    states,
                    "{label}\t{}",
                    values.map(|v| v.to_string()).join("\t")
                )?;
            }
            fs::write(
                directory.join("frame-error.txt"),
                format!("{:?}\n", result.image.as_ref().err()),
            )?;
            let image = result.image.ok();
            save(&directory, "frame", image.as_ref())?;
            if let Some(raw) = converter.raw.take() {
                fs::write(directory.join("frame.raw"), &raw)?;
                raw_held.push((name.clone(), raw));
                raw_count += 1;
            }
            let current = palette(alternative);
            let prior = palette(!alternative);
            let known = image.as_ref().is_some_and(|i| i.bytes() == current);
            let zero = image
                .as_ref()
                .is_some_and(|i| !i.bytes().is_empty() && i.bytes().iter().all(|v| *v == 0));
            let previous = image.as_ref().is_some_and(|i| i.bytes() == prior);
            let printed =
                PrintWindowCapture::new(fixture.hwnd()).capture(&mut adapter.converter(0));
            fs::write(
                directory.join("print-error.txt"),
                format!("{:?}\n", printed.image.as_ref().err()),
            )?;
            let print_clean = printed
                .main_cleanup
                .0
                .iter()
                .chain(&printed.popup_cleanup.0)
                .all(Option::is_none)
                && printed.popup_failure.is_none();
            let print = printed.image.ok();
            save(&directory, "print", print.as_ref())?;
            let print_known = print.as_ref().is_some_and(|i| i.bytes() == current);
            positive_controls &= image.is_some()
                && print_known
                && print_clean
                && (alpha == 0 || known)
                && result.evidence.cleanup.iter().all(Option::is_none);
            replays += usize::from(result.evidence.replayed);
            writeln!(
                rows,
                "{arm}\t{phase}\t{alpha}\t{}\t{}\t{}\t{}\t{}\t{before_paint}\t{after_paint}\t{}\t{elapsed}\t{}\t{}\t{}\t{}\t{}\t{}",
                u8::from(image.is_some()),
                u8::from(known),
                u8::from(zero),
                u8::from(print.is_some()),
                u8::from(print_known),
                u8::from(previous),
                u8::from(result.evidence.replayed),
                converter.inner.conversions,
                converter.inner.clones,
                result.evidence.mapped_pitch,
                result.evidence.target_copy_bytes,
                u8::from(directory.join("frame.raw").exists())
            )?;
            rows.flush()?;
            writeln!(
                branches,
                "{arm}\t{phase}\t{}\t{}\t{}\t{}\t{}\t0",
                result
                    .evidence
                    .drained_frames
                    .checked_add(1)
                    .ok_or("drain call overflow")?,
                result.evidence.drained_frames,
                result.evidence.polls,
                u8::from(image.is_some() && !result.evidence.replayed),
                u8::from(result.evidence.replayed)
            )?;
            branches.flush()?;
            if let Some(image) = image {
                held.push((format!("{name}-frame"), image));
            }
            if let Some(print) = print {
                held.push((format!("{name}-print"), print));
            }
        }
        drop(persistent);
        fixture.close()?;
    }
    dispatcher.close()?;
    for (name, image) in &held {
        save(&root, &format!("held-{name}"), Some(image))?;
    }
    for (name, raw) in &raw_held {
        fs::write(root.join(format!("held-{name}.raw")), raw)?;
    }
    let held_count = held.len();
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), close_errors);
    assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
    assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
    fs::write(
        root.join("summary.tsv"),
        format!(
            "schema\t1\nphases\t10\nframe_images\t10\nprint_images\t10\nheld_images\t{held_count}\nclosed_windows\t2\ndispatcher_completed\t1\npositive_controls\t{}\nraw_frames\t{raw_count}\nreplayed_frames\t{replays}\ncontext_owners\t0\n",
            u8::from(positive_controls)
        ),
    )?;
    drop(adapter);
    drop(owned);
    assert!(positive_controls);
    assert_eq!(held_count, 20);
    println!(
        "Rust FramePool alpha phases=10 frame_images=10 print_images=10 held_images=20 closed_windows=2 positive_controls=1 source_oracle_replaced=0 capture_acceptance=0 passed"
    );
    Ok(())
}
