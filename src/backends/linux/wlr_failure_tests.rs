// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::protocol::cases;
use super::*;
use std::cell::Cell;

#[test]
#[ignore = "requires explicit native Failed/SHM/EOF fixture and original observations"]
fn preserves_failed_outputs_and_buffer_reuse_before_native_disconnect() {
    let socket = PathBuf::from(env::var_os("MFS_P1_WLR_FAILURES_SOCKET").unwrap());
    let directory = PathBuf::from(env::var_os("MFS_P1_WLR_FAILURES_ARTIFACTS").unwrap());
    let source = PathBuf::from(env::var_os("MFS_P1_WLR_FAILURES_SOURCE").unwrap());
    let adapter = Adapter::open();
    let warm = Sample {
        width: 7,
        height: 5,
        native_stride: 28,
        conversion: Conversion::Bgra,
        ready_time: None,
        flags: None,
        bytes: Arc::try_new(cases::raw(0)).unwrap(),
    };
    drop(adapter.convert(warm, 0).unwrap());
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let sentinel = [201, 73, 17].repeat(35);
    let mut images: Vec<Image<'_>> = Vec::new();
    let mut raws = Vec::new();
    let mut rows = String::new();
    let mut resources = String::new();
    let mut errors = String::new();
    for cycle in 0..20 {
        let counts;
        {
            let mut client = Client::new().unwrap();
            assert!(client.connect(&socket).unwrap());
            let client = client.into_shared().unwrap();
            counts = client.context.counts.clone();
            let mut capture = Capture::from_client(client.clone()).unwrap();
            let mut current: Option<usize> = None;
            for phase in 0..5 {
                let before = current;
                let conversion_calls = Cell::new(0);
                let result = capture.capture_with(|sample| {
                    conversion_calls.set(conversion_calls.get() + 1);
                    let raw = sample.bytes.clone();
                    adapter.convert(sample, 0).map(|image| (raw, image))
                });
                let success = phase == 1 || phase == 3;
                assert_eq!(result.is_ok(), success);
                assert_eq!(conversion_calls.get(), u32::from(success));
                match result {
                    Ok((raw, image)) => {
                        let index = if phase == 1 { 0 } else { 4 };
                        assert_eq!(raw.as_slice(), cases::raw(index));
                        assert_eq!(image.pixels_for(7, 5), cases::target(index));
                        std::fs::write(
                            directory.join(format!("case-{cycle}-{phase}.rust.raw")),
                            raw.as_slice(),
                        )
                        .unwrap();
                        raws.push((raw, index));
                        current = Some(images.len());
                        images.push(image);
                    }
                    Err(error) => {
                        if phase == 4 {
                            assert_eq!(error.operation, "wl_display_roundtrip");
                        } else {
                            assert_eq!(
                                (error.operation, error.code),
                                ("native Wlr frame failed", -6)
                            );
                        }
                        assert_eq!(current, before);
                        errors.push_str(&format!(
                            "{cycle}\t{phase}\t{}\t{}\n",
                            error.operation, error.code
                        ));
                    }
                }
                // The host retains its previous real owning image on Err. No
                // SDK output API or native retry is introduced by this fixture.
                let pixels = current.map_or(sentinel.as_slice(), |i| images[i].pixels_for(7, 5));
                let expected = if phase == 0 {
                    sentinel.clone()
                } else {
                    cases::target(if phase < 3 { 0 } else { 4 })
                };
                std::fs::write(
                    directory.join(format!("case-{cycle}-{phase}.rust.bgr")),
                    pixels,
                )
                .unwrap();
                assert_eq!(pixels, expected);
                assert_eq!(
                    pixels,
                    std::fs::read(source.join(format!("case-{cycle}-{phase}.source.bgr"))).unwrap()
                );
                assert!(client.connected().unwrap());
                rows.push_str(&format!(
                    "{cycle}\t{phase}\t{}\t1\t{}\n",
                    u8::from(success),
                    u8::from(!success)
                ));
            }
            capture.close();
            drop(client);
            let evidence = capture.evidence();
            assert_eq!((evidence.fault, evidence.buffer_error), (0, 0));
            assert_eq!((evidence.copied, evidence.frames), (2, 2));
        }
        let actual_fds = fds();
        let actual_tasks = tasks();
        assert_eq!(actual_fds, baseline_fds);
        assert_eq!(actual_tasks, baseline_tasks);
        assert_eq!(
            counts.data_created.load(Ordering::Acquire),
            counts.data_dropped.load(Ordering::Acquire)
        );
        // Native send/destroy requests may fail after the deliberate EOF.
        // Preserve this cleanup evidence independently of the primary failure;
        // fd/task closure does not establish successful protocol destruction.
        let cleanup = counts.cleanup.load(Ordering::Acquire);
        assert!(matches!(cleanup, 0 | -1));
        resources.push_str(&format!(
            "{cycle}\t{baseline_fds}\t{actual_fds}\t{}\t{}\t{cleanup}\n",
            baseline_tasks.len(),
            actual_tasks.len()
        ));
    }
    for (i, (raw, index)) in raws.iter().enumerate() {
        assert_eq!(raw.as_slice(), cases::raw(*index));
        assert_eq!(images[i].pixels_for(7, 5), cases::target(*index));
        std::fs::write(
            directory.join(format!("held-{i}.rust.bgr")),
            images[i].pixels_for(7, 5),
        )
        .unwrap();
    }
    std::fs::write(directory.join("rust-cases.tsv"), rows).unwrap();
    std::fs::write(directory.join("rust-resources.tsv"), resources).unwrap();
    std::fs::write(directory.join("rust-errors.tsv"), errors).unwrap();
    assert_eq!(adapter.owners(), 40);
    drop(images);
    drop(raws);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    println!(
        "native Rust Wlr failures cycles=20 operations=100 ready=40 failed_events=40 disconnects=20 failed_conversions=0 connected_retained=20 held_raw=40 held_bgr=40 fd_tasks_balanced=1 cleanup_errors_recorded=1 passed"
    );
}
