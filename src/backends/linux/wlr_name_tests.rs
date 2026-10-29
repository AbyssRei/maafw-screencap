// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::linux::manager::{self, native::Evidence as ManagerEvidence};
use std::ffi::OsStr;

#[test]
#[ignore = "requires native owned compositor and original embedded-NUL oracle"]
fn preserves_embedded_nul_native_name_and_full_reconnect_identity() {
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let bytes = socket.as_os_str().as_bytes();
    assert!(bytes.is_ascii() && !bytes.contains(&0));
    let mut first = bytes.to_vec();
    first.extend_from_slice(b"\0first");
    let mut second = bytes.to_vec();
    second.extend_from_slice(b"\0second");
    let first_path = Path::new(OsStr::from_bytes(&first));
    let second_path = Path::new(OsStr::from_bytes(&second));
    assert_ne!(first_path, second_path);
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("nul-names");
    let oracle =
        PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap()).join("nul-names");
    std::fs::create_dir(&directory).unwrap();
    // Encode the owned ASCII fixture path as JSON escapes, including a decoded
    // NUL. This exercises Config.h migration through actual native connect.
    let escaped: String = bytes.iter().map(|b| format!("\\u{b:04x}")).collect();
    let config = format!(
        "{{\"screencap_method\":1,\"input_method\":0,\"wlr_socket_path\":\"{escaped}\\u0000first\"}}"
    );
    std::fs::write(directory.join("manager-input.json"), &config).unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    let mut held_bgr = Vec::new();
    held_raw.try_reserve_exact(40).unwrap();
    held_bgr.try_reserve_exact(60).unwrap();
    let mut converter = ManagerConvert {
        adapter: &adapter,
        held: Vec::new(),
    };
    converter.held.try_reserve_exact(20).unwrap();
    for cycle in 0..20 {
        let mut client = Client::new().unwrap();
        let counts = client.context.counts.clone();
        assert!(client.connect(first_path).unwrap());
        let before = counts.outer.load(Ordering::Acquire);
        assert!(client.connect(first_path).unwrap());
        assert_eq!(counts.outer.load(Ordering::Acquire), before);
        for call in 0..2 {
            if call == 1 {
                let before = counts.outer.load(Ordering::Acquire);
                assert!(client.connect(second_path).unwrap());
                assert!(counts.outer.load(Ordering::Acquire) > before);
            }
            let shared = client.into_shared().unwrap();
            let mut capture = Capture::from_client(shared.clone()).unwrap();
            held_bgr.push(context_capture(
                &mut capture,
                &adapter,
                &directory,
                &oracle,
                &format!("nul-{cycle}-{call}"),
                &mut held_raw,
            ));
            drop(capture);
            client = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("old capture owner"));
        }
        for name in [b"\0mfs-unused".as_slice(), b""] {
            assert!(client.connect(Path::new(OsStr::from_bytes(name))).is_err());
            assert!(!client.connected().unwrap());
            assert!(client.channel.is_none());
            assert!(client.context.state().unwrap().globals.output.is_none());
        }
        drop(client);
        released_counts(&counts);
        let mut manager =
            manager::NativeManager::from_config_json(Some(config.as_bytes())).unwrap();
        let mut uuid = Vec::new();
        assert!(manager.request_uuid(&mut uuid).unwrap());
        assert_eq!(uuid, [first.as_slice(), b"_-1"].concat());
        let connected = manager.connect_native().unwrap();
        assert!(connected.connected && connected.failure.is_none());
        let close = connected.close.unwrap();
        let mut output = None;
        assert!(
            manager
                .capture_native(&mut output, &mut converter)
                .result
                .unwrap()
        );
        let image = output.take().unwrap();
        let stem = format!("nul-manager-{cycle}");
        bgr(image.pixels());
        assert_eq!(
            image.pixels(),
            std::fs::read(oracle.join(format!("{stem}.bgr"))).unwrap()
        );
        std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
        let sample = converter.held.last().unwrap();
        std::fs::write(directory.join(format!("{stem}.raw")), sample.pixels()).unwrap();
        std::fs::write(directory.join(format!("{stem}.meta.txt")), format!(
            "conversion={:?}\nwidth={}\nheight={}\nnative_stride={}\nready_time={:?}\nflags={:?}\n",
            sample.conversion, sample.width, sample.height, sample.native_stride,
            sample.ready_time, sample.flags,
        )).unwrap();
        held_bgr.push(image);
        drop(manager);
        let Some(ManagerEvidence::Wlr(closed)) = close.take() else {
            panic!("missing completed manager native cleanup")
        };
        assert_eq!(
            (closed.frames, closed.copied, closed.cleanup, closed.fault),
            (1, 1, 0, 0)
        );
        assert_eq!(closed.data_created, closed.destroyed);
        assert_eq!(closed.data_created, closed.data_dropped);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        eprintln!(
            "native NUL-name cycle={cycle} held={} fds={baseline_fds} resources_balanced=1",
            held_bgr.len()
        );
    }
    for sample in held_raw.iter().chain(converter.held.iter()) {
        raw(sample);
    }
    for image in &held_bgr {
        bgr(image.pixels());
    }
    drop(held_bgr);
    drop(held_raw);
    drop(converter);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr nul_names cycles=20 captures=60 retained_raw=60 retained_bgr=60 same_full_name_noop=20 suffix_reconnect=20 leading_nul_rejected=20 empty_rejected=20 json_manager_full_uuid=20 resources_balanced=1 passed"
    );
}
