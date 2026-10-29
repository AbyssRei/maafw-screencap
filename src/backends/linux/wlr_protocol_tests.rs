// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[path = "../../../tests/upstream-parity/linux-wlr/protocol-cases.rs"]
pub(super) mod cases;

#[test]
#[ignore = "requires the explicit native Wayland protocol/SHM fixture and original source records"]
fn matches_native_protocol_formats_padding_and_ignored_yinvert() {
    let socket = PathBuf::from(env::var_os("MFS_P1_WLR_PROTOCOL_SOCKET").unwrap());
    let version: u32 = env::var("MFS_P1_WLR_PROTOCOL_VERSION")
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=3).contains(&version));
    let directory = PathBuf::from(env::var_os("MFS_P1_WLR_PROTOCOL_ARTIFACTS").unwrap());
    let adapter = Adapter::open();
    // Establish the actual native conversion runtime before resource baselines.
    {
        for (index, conversion) in [
            (0, Conversion::Bgra),
            (8, Conversion::Rgba),
            (16, Conversion::BgrCopy),
            (20, Conversion::Rgb),
        ] {
            let warm = Sample {
                width: 7,
                height: 5,
                native_stride: u32::try_from(cases::stride(index)).unwrap(),
                conversion,
                ready_time: None,
                flags: None,
                bytes: Arc::try_new(cases::raw(index)).unwrap(),
            };
            drop(adapter.convert(warm, 0).unwrap());
        }
    }
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    let mut held_bgr = Vec::new();
    let mut rows = String::new();
    for cycle in 0..20 {
        {
            let mut capture = Capture::connect(&socket).unwrap();
            assert_eq!(
                capture
                    .context
                    .state()
                    .unwrap()
                    .manager
                    .as_ref()
                    .unwrap()
                    .proxy
                    .version(),
                version
            );
            for index in 0..24 {
                // Only scalar/refcount observations surround the actual
                // conversion. It runs while the source frame remains alive;
                // all file I/O and assertions follow native frame release.
                let (raw, metadata, image) = capture
                    .capture_with(|sample| {
                        let metadata = (
                            sample.width,
                            sample.height,
                            sample.native_stride,
                            sample.flags,
                            sample.ready_time,
                        );
                        let raw = sample.bytes.clone();
                        adapter
                            .convert(sample, 0)
                            .map(|image| (raw, metadata, image))
                    })
                    .unwrap();
                let (width, height, native_stride, flags, ready_time) = metadata;
                assert_eq!(
                    (width, height, native_stride),
                    (7, 5, u32::try_from(cases::stride(index)).unwrap())
                );
                assert_eq!(flags, Some(u32::try_from(index % 2).unwrap()));
                assert_eq!(
                    ready_time,
                    Some((
                        0,
                        2000 + u32::try_from(index).unwrap(),
                        1000 + u32::try_from(index).unwrap()
                    ))
                );
                std::fs::write(
                    directory.join(format!("case-{cycle}-{index}.rust.raw")),
                    raw.as_slice(),
                )
                .unwrap();
                assert_eq!(raw.as_slice(), cases::raw(index));
                held_raw.push((raw, cases::raw(index)));
                let pixels = image.pixels_for(7, 5);
                std::fs::write(
                    directory.join(format!("case-{cycle}-{index}.rust.bgr")),
                    pixels,
                )
                .unwrap();
                assert_eq!(pixels, cases::legacy(index));
                let source =
                    std::fs::read(directory.join(format!("case-{cycle}-{index}.source.bgr")))
                        .unwrap();
                assert_eq!(pixels, source);
                let target_matches = pixels == cases::target(index);
                assert_eq!(target_matches, index % 4 == 0);
                rows.push_str(&format!(
                    "{cycle}\t{index}\t{version}\t{}\t{}\t{}\t{}\n",
                    cases::format(index),
                    cases::stride(index),
                    index % 2,
                    u8::from(target_matches)
                ));
                held_bgr.push(image);
            }
            capture.close();
            let evidence = capture.evidence();
            assert_eq!(
                (evidence.cleanup, evidence.fault, evidence.buffer_error),
                (0, 0, 0)
            );
            assert_eq!(evidence.data_created, evidence.data_dropped);
            assert_eq!((evidence.copied, evidence.frames), (24, 24));
        }
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        println!(
            "rust protocol cycle={cycle} held={} fds={baseline_fds} balanced=1",
            held_raw.len()
        );
    }
    for (index, (raw, expected)) in held_raw.iter().enumerate() {
        assert_eq!(raw.as_slice(), expected);
        assert_eq!(held_bgr[index].pixels_for(7, 5), cases::legacy(index % 24));
    }
    assert_eq!(adapter.owners(), 480);
    drop(held_bgr);
    drop(held_raw);
    assert_eq!(adapter.owners(), 0);
    std::fs::write(directory.join("rust-cases.tsv"), rows).unwrap();
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    println!(
        "native Rust Wlr protocol version={version} cycles=20 captures=480 retained_raw=480 retained_bgr=480 formats=6 layouts=4 resources_balanced=1 legacy_packed=1 native_copy=1 passed"
    );
}
