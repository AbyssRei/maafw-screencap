// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::protocol::cases;
use super::*;

#[test]
#[ignore = "requires the explicit native registry/SHM fixture and original source records"]
fn preserves_last_output_and_existing_snapshots_through_global_removal() {
    let socket = PathBuf::from(env::var_os("MFS_P1_WLR_REGISTRY_SOCKET").unwrap());
    let directory = PathBuf::from(env::var_os("MFS_P1_WLR_REGISTRY_ARTIFACTS").unwrap());
    let source = PathBuf::from(env::var_os("MFS_P1_WLR_REGISTRY_SOURCE").unwrap());
    let adapter = Adapter::open();
    // Initialize the actual conversion runtime before native resource baselines.
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
    let mut client = Client::new().unwrap();
    assert!(client.connect(&socket).unwrap());
    let client = client.into_shared().unwrap();
    let counts = client.context.counts.clone();
    let mut raw_images = Vec::new();
    let mut images = Vec::new();
    let mut rows = String::new();
    for round in 0..20 {
        let old_output = client
            .context
            .state()
            .unwrap()
            .globals
            .output
            .clone()
            .unwrap();
        let mut old = Capture::from_client(client.clone()).unwrap();
        let mut shot = |capture: &mut Capture, output: &Owned<wl_output::WlOutput>, phase| {
            let generation = round + if phase < 2 { 1 } else { 2 };
            let index = if generation % 2 == 1 { 0 } else { 4 };
            let position = round * 4 + phase;
            let (raw, image) = capture
                .capture_with(|sample| {
                    assert_eq!(
                        (sample.width, sample.height, sample.native_stride),
                        (7, 5, 28)
                    );
                    let raw = sample.bytes.clone();
                    adapter.convert(sample, 0).map(|image| (raw, image))
                })
                .unwrap();
            let pixels = image.pixels_for(7, 5);
            std::fs::write(
                directory.join(format!("case-{position}.rust.raw")),
                raw.as_slice(),
            )
            .unwrap();
            std::fs::write(directory.join(format!("case-{position}.rust.bgr")), pixels).unwrap();
            assert_eq!(raw.as_slice(), cases::raw(index));
            assert_eq!(pixels, cases::target(index));
            assert_eq!(
                pixels,
                std::fs::read(source.join(format!("case-{position}.source.bgr"))).unwrap()
            );
            assert_eq!(
                output.proxy.version(),
                1 + u32::try_from(generation % 2).unwrap()
            );
            rows.push_str(&format!(
                "{position}\t{generation}\t{}\t{index}\t{}\n",
                output.proxy.id().protocol_id(),
                output.proxy.version()
            ));
            raw_images.push((raw, index));
            images.push(image);
        };
        shot(&mut old, &old_output, 0);
        client.roundtrip().unwrap();
        client.context.check().unwrap();
        let next_output = client
            .context
            .state()
            .unwrap()
            .globals
            .output
            .clone()
            .unwrap();
        assert_ne!(old_output.proxy.id(), next_output.proxy.id());
        let mut next = Capture::from_client(client.clone()).unwrap();
        shot(&mut old, &old_output, 1);
        shot(&mut next, &next_output, 2);
        client.roundtrip().unwrap();
        client.context.check().unwrap();
        assert_eq!(
            client
                .context
                .state()
                .unwrap()
                .globals
                .output
                .as_ref()
                .unwrap()
                .proxy
                .id(),
            next_output.proxy.id()
        );
        let mut after_remove = Capture::from_client(client.clone()).unwrap();
        shot(&mut after_remove, &next_output, 3);
    }
    drop(client);
    assert_eq!(fds(), baseline_fds);
    assert_eq!(tasks(), baseline_tasks);
    assert_eq!(counts.cleanup.load(Ordering::Acquire), 0);
    assert_eq!(
        counts.data_created.load(Ordering::Acquire),
        counts.data_dropped.load(Ordering::Acquire)
    );
    assert_eq!(
        (
            counts.copied.load(Ordering::Acquire),
            counts.frames.load(Ordering::Acquire)
        ),
        (80, 80)
    );
    for (position, (raw, index)) in raw_images.iter().enumerate() {
        assert_eq!(raw.as_slice(), cases::raw(*index));
        assert_eq!(images[position].pixels_for(7, 5), cases::target(*index));
        std::fs::write(
            directory.join(format!("case-{position}.rust.held.bgr")),
            images[position].pixels_for(7, 5),
        )
        .unwrap();
    }
    std::fs::write(directory.join("rust-cases.tsv"), rows).unwrap();
    std::fs::write(
        directory.join("rust-resources.tsv"),
        format!(
            "{baseline_fds}\t{}\t{}\t{}\n",
            fds(),
            baseline_tasks.len(),
            tasks().len()
        ),
    )
    .unwrap();
    assert_eq!(adapter.owners(), 80);
    drop(images);
    drop(raw_images);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    println!(
        "native Rust Wlr registry rounds=20 captures=80 retained_raw=80 retained_bgr=80 replacement_snapshots=20 removed_output_retained=20 advertised_versions=1,2 resources_balanced=1 passed"
    );
}
