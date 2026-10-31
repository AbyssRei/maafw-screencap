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
            assert!(client.current_channel().unwrap().is_none());
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

#[test]
#[ignore = "requires native original filesystem paths and owned compositor"]
fn preserves_source_path_components_and_native_reconnect_failures() {
    let inputs: &[&[u8]] = &[
        b"", b".", b"./", b"/", b"//", b"///", b"/.", b"/./", b"a", b"a/", b"a//", b"a/.", b"a/./",
        b"./a", b"./a/", b"a//b", b"a/b", b"a/./b", b"a//./b", b"a/../b", b"a/../b/", b"../a",
        b"a\0x", b"a\0x/", b"a\0x//b", b"a\0x/b", b"a/\0b", b"a/./\0b", b"a\xff",
    ];
    let socket = PathBuf::from(env::var_os("MFS_P0_WLR_SOCKET").unwrap());
    let parent = socket.parent().unwrap().as_os_str().as_bytes();
    let filename = socket.file_name().unwrap().as_bytes();
    let dot = [parent, b"/./", filename].concat();
    let repeated = [parent, b"//./", filename].concat();
    let trailing = [socket.as_os_str().as_bytes(), b"/"].concat();
    fn path(bytes: &[u8]) -> &Path {
        Path::new(OsStr::from_bytes(bytes))
    }
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("path-names");
    let oracle =
        PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap()).join("path-names");
    std::fs::create_dir(&directory).unwrap();
    let matrix = std::fs::read_to_string(oracle.join("path-equality.tsv")).unwrap();
    let lines: Vec<_> = matrix.lines().collect();
    assert_eq!(lines.len(), inputs.len() * inputs.len());
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut observed = String::new();
    for (index, line) in lines.iter().enumerate() {
        let left = index / inputs.len();
        let right = index % inputs.len();
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], left.to_string());
        assert_eq!(fields[1], right.to_string());
        assert_eq!(fields[2], hex(inputs[left]));
        assert_eq!(fields[3], hex(inputs[right]));
        assert!(matches!(fields[4], "0" | "1"));
        let equal = same_source_path(path(inputs[left]), path(inputs[right]));
        assert_eq!(equal, fields[4] == "1", "native path pair {left},{right}");
        observed.push_str(&format!(
            "{left}\t{right}\t{}\t{}\t{}\n",
            hex(inputs[left]),
            hex(inputs[right]),
            u8::from(equal)
        ));
    }
    std::fs::write(directory.join("path-equality.tsv"), observed).unwrap();
    // These Rust Path comparisons intentionally differ from the native source
    // condition; the actual C++ matrix and real connection probes are the oracle.
    assert_eq!(socket.as_path(), path(&dot));
    assert_eq!(socket.as_path(), path(&trailing));
    assert!(!same_source_path(&socket, path(&dot)));
    assert!(!same_source_path(&socket, path(&trailing)));
    assert!(same_source_path(path(&dot), path(&repeated)));
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    let mut held_bgr = Vec::new();
    held_raw.try_reserve_exact(20).unwrap();
    held_bgr.try_reserve_exact(20).unwrap();
    for cycle in 0..20 {
        let mut client = Client::new().unwrap();
        let counts = client.context.counts.clone();
        assert!(client.connect(&socket).unwrap());
        let before = counts.outer.load(Ordering::Acquire);
        assert!(client.connect(path(&dot)).unwrap());
        assert!(counts.outer.load(Ordering::Acquire) > before);
        let before = counts.outer.load(Ordering::Acquire);
        assert!(client.connect(path(&repeated)).unwrap());
        assert_eq!(counts.outer.load(Ordering::Acquire), before);
        let shared = client.into_shared().unwrap();
        let mut capture = Capture::from_client(shared.clone()).unwrap();
        held_bgr.push(context_capture(
            &mut capture,
            &adapter,
            &directory,
            &oracle,
            &format!("path-{cycle}"),
            &mut held_raw,
        ));
        drop(capture);
        client = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("old path capture owner"));
        assert!(client.connect(path(&trailing)).is_err());
        assert!(!client.connected().unwrap());
        assert!(client.current_channel().unwrap().is_none());
        assert!(client.context.state().unwrap().globals.output.is_none());
        assert!(client.connect(&socket).unwrap());
        drop(client);
        released_counts(&counts);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        eprintln!(
            "native path-name cycle={cycle} held={} fds={baseline_fds} resources_balanced=1",
            held_raw.len()
        );
    }
    for sample in &held_raw {
        raw(sample);
    }
    for image in &held_bgr {
        bgr(image.pixels());
    }
    drop(held_raw);
    drop(held_bgr);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr path_names matrix_cases=841 cycles=20 captures=20 retained_raw=20 retained_bgr=20 dot_reconnect=20 repeated_separator_noop=20 trailing_separator_rejected=20 explicit_recovery=20 resources_balanced=1 passed"
    );
}
