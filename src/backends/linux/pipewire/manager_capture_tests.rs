// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real source manager forwarding over the existing private native implementation.
use super::*;
use crate::backends::linux::manager::{self, native as managed};

struct Convert<'a> {
    adapter: &'a Adapter,
    raw: Vec<RawFrame>,
}
impl<'a> managed::Converter for Convert<'a> {
    type Image = Option<Bgr<'a>>;
    type Error = AdapterError;
    #[cfg(feature = "linux-wlr")]
    fn wlr(&mut self, _: crate::backends::linux::wlr::Sample) -> Result<Self::Image, AdapterError> {
        unreachable!("PipeWire manager cannot select Wlr")
    }
    fn pipewire(
        &mut self,
        raw: Option<RawFrame>,
        mode: Conversion,
    ) -> Result<Option<Self::Image>, AdapterError> {
        let raw = raw.ok_or(AdapterError::InvalidLayout)?;
        assert_eq!(mode, Conversion::Rgb);
        let image = self.adapter.convert(&raw, mode)?;
        self.raw.push(raw);
        Ok(Some(Some(Bgr::Converted(image))))
    }
    fn pipewire_reuse(&mut self, _: Option<RawFrame>) -> Option<Self::Image> {
        unreachable!("owned RGB producer must not negotiate BGR")
    }
    fn empty(&mut self) -> Result<Self::Image, AdapterError> {
        panic!("owned RGB producer must supply a nonempty image")
    }
}

#[test]
#[ignore = "requires native owned PipeWire routing and actual C++/OpenCV adapter"]
fn reconnects_daemon_manager_and_retains_native_cpu_images() {
    run(false);
}
#[test]
#[ignore = "requires a unique supplied socket for each manager and native OpenCV adapter"]
fn forwards_supplied_fd_manager_and_closes_consumed_socket() {
    run(true);
}
fn check_close(evidence: managed::Evidence, expected: u64) {
    let managed::Evidence::PipeWire(e) = evidence else {
        panic!("missing native PipeWire close evidence")
    };
    assert_eq!(e.fault, 0, "{e:?}");
    assert_eq!((e.queue_error, e.core_disconnect_error), (0, 0), "{e:?}");
    assert_eq!((e.mapping_error, e.unmap_error), (0, 0), "{e:?}");
    assert_eq!(e.owned_maps, e.unmap_calls, "{e:?}");
    assert_eq!(e.dequeued, e.returned, "{e:?}");
    assert_eq!((e.copies, e.allocated), (expected, expected), "{e:?}");
}
fn run(supplied_fd: bool) {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    assert_eq!(
        Path::new(&env::var("PIPEWIRE_RUNTIME_DIR").unwrap())
            .join(env::var("PIPEWIRE_REMOTE").unwrap()),
        Path::new(&socket)
    );
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse()
        .unwrap();
    let mode = if supplied_fd { "fd" } else { "daemon" };
    let directory = Path::new(&env::var("MFS_P1_PIPEWIRE_BGR_ARTIFACTS").unwrap())
        .join(format!("manager-{mode}"));
    fs::create_dir(&directory).unwrap();
    let sessions = if supplied_fd { 1 } else { 2 };
    let total = 20 * sessions * 3;
    let adapter = Adapter::open();
    // Prepare conversion runtime before the exact native-resource baseline.
    let warm = [1_u8; 12];
    let mut cache = RawCache::default();
    let layout = Layout::checked(2, 2, Format::Rgb, 12, None).unwrap();
    // SAFETY: Complete initialized rows, copied into isolated owned CPU storage.
    unsafe { cache.copy_native(warm.as_ptr(), layout) }.unwrap();
    drop(
        adapter
            .convert(&cache.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    cache.clear();
    let fds = count("/proc/self/fd");
    let baseline = task_ids();
    let mut converter = Convert {
        adapter: &adapter,
        raw: Vec::new(),
    };
    converter.raw.try_reserve_exact(total).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(total).unwrap();
    let mut identities = Vec::new();
    identities.try_reserve_exact(total).unwrap();
    for cycle in 0..20 {
        let offered = supplied_fd.then(|| UnixStream::connect(&socket).unwrap());
        let fd = offered.map(IntoRawFd::into_raw_fd).unwrap_or(-1);
        struct Balance(u32);
        impl Drop for Balance {
            fn drop(&mut self) {
                for _ in 0..self.0 {
                    // SAFETY: Explicit fixture cleanup after manager/native
                    // destruction. One pw_init per successful backend creation.
                    unsafe { sys::pw_deinit() };
                }
            }
        }
        let mut balance = Balance(0);
        let json = format!(
            r#"{{"screencap_method":4,"input_method":0,"pw_socket_fd":{fd},"pw_node_id":{node}}}"#
        );
        let mut manager = manager::NativeManager::from_config_json(Some(json.as_bytes())).unwrap();
        let mut output = None;
        assert!(
            !manager
                .capture_native(&mut output, &mut converter)
                .result
                .unwrap()
        );
        assert!(output.is_none());
        let mut uuid = Vec::new();
        assert!(manager.request_uuid(&mut uuid).unwrap());
        assert_eq!(uuid, format!("{fd}_{node}_-1").as_bytes());
        let mut close: Option<managed::CloseWatch> = None;
        for session in 0..sessions {
            let connected = if supplied_fd {
                // SAFETY: Sole live offered private socket owner was detached
                // above. Exactly one connect occurs; no stale fd reuse/retry.
                unsafe { manager.connect_source_native() }.unwrap()
            } else {
                manager.connect_native().unwrap()
            };
            balance.0 += 1;
            assert!(
                connected.connected && connected.failure.is_none(),
                "{connected:?}"
            );
            if let Some(previous) = close.take() {
                check_close(
                    previous
                        .take()
                        .expect("old backend closed before reconstruction"),
                    3,
                );
            }
            close = connected.close;
            for call in 0..3 {
                if call == 1 {
                    assert!(manager.inactive_native());
                }
                assert!(
                    manager
                        .capture_native(&mut output, &mut converter)
                        .result
                        .unwrap()
                );
                let image = output.take().unwrap();
                let raw = converter.raw.last().unwrap();
                let stem = format!("manager-{cycle}-session-{session}-call-{call}");
                // Archive both outputs before strict pixel assertions so a
                // failed scene remains inspectable without another capture.
                fs::write(directory.join(format!("{stem}.rgb")), raw.pixels()).unwrap();
                fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
                let serial = known_bgr(&image);
                assert_eq!(serial, marker(raw));
                let (rgb_rows, rgb_tail) = raw.pixels().as_chunks::<3>();
                let (bgr_rows, bgr_tail) = image.pixels().as_chunks::<3>();
                assert!(rgb_tail.is_empty() && bgr_tail.is_empty());
                assert_eq!(rgb_rows.len(), bgr_rows.len());
                for (rgb, bgr) in rgb_rows.iter().zip(bgr_rows) {
                    assert_eq!(*bgr, [rgb[2], rgb[1], rgb[0]]);
                }
                identities.push(serial);
                held.push((image, serial));
            }
        }
        let live = live_tasks();
        drop(manager);
        check_close(
            close
                .unwrap()
                .take()
                .expect("completed manager destruction"),
            3,
        );
        drop(balance);
        if supplied_fd {
            // SAFETY: Read-only observation before opening any diagnostic fd.
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
        }
        assert_resources(&directory, cycle, fds, &baseline, &live);
        for (image, serial) in &held {
            assert_eq!(known_bgr(image), *serial);
        }
        eprintln!(
            "native Rust manager mode={mode} cycle={cycle} retained={} resources_balanced=1",
            held.len()
        );
    }
    assert_eq!(converter.raw.len(), total);
    assert_eq!(identities.len(), total);
    std::thread::spawn(move || {
        for (raw, serial) in converter.raw.into_iter().zip(identities) {
            assert_eq!(marker(&raw), serial);
        }
    })
    .join()
    .unwrap();
    for (image, serial) in &held {
        assert_eq!(known_bgr(image), *serial);
    }
    drop(held);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust PipeWire manager mode={mode} objects=20 sessions={} captures={total} retained_raw={total} retained_bgr={total} consumed_sockets_closed={} resources_balanced=1 passed",
        20 * sessions,
        if supplied_fd { 20 } else { 0 }
    );
}
