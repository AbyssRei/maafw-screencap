// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Same-object success lifecycle. Failed FD offers and Portal remain separate.
use super::*;

#[test]
#[ignore = "explicit native supplied-fd then daemon reinitialization, not failed-offer recovery or a safe SDK policy"]
fn reinitializes_after_consumed_fd_and_retains_both_sessions_cpu_images() {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    let remote = env::var("PIPEWIRE_REMOTE").unwrap();
    let runtime = env::var("PIPEWIRE_RUNTIME_DIR").unwrap();
    assert_eq!(Path::new(&runtime).join(remote), Path::new(&socket));
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let directory = Path::new(&env::var("MFS_P1_PIPEWIRE_FD_ARTIFACTS").unwrap()).join("reinit");
    fs::create_dir_all(&directory).unwrap();
    let fds = count("/proc/self/fd");
    let baseline = task_ids();
    task_snapshot(&directory, "baseline", &baseline);
    let mut held = Vec::new();
    held.try_reserve_exact(120).unwrap();
    for cycle in 0..20 {
        let offered = UnixStream::connect(&socket).unwrap();
        // SAFETY: Unique live private socket; constructor only stores its number.
        // Detach the sole Rust owner before the native transfer below.
        let native = unsafe { Native::with_fd(offered.as_raw_fd()) }.unwrap();
        let balance = BalanceInit;
        let mut native = native;
        let fd = offered.into_raw_fd();
        let mut consumer = Consumer::default();
        // Source close before its first init does not consume the pending FD.
        native.close();
        native.close();
        assert_eq!(native.input_fd, fd);
        // SAFETY: Read-only descriptor observation, no close or ownership claim.
        assert!(unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0);
        consumer.inactive(&mut native);
        assert!(matches!(
            consumer.capture(&mut native, &mut RawAudit).image,
            Err(crate::backends::linux::pipewire::Error::Disconnected)
        ));
        for session in 0..2 {
            if session == 0 {
                // SAFETY: The fresh socket's sole owner was detached above.
                // Failure stops this isolated probe; it does not guess a close.
                unsafe { native.init_source(node) }.unwrap();
            } else {
                // Exact U stored-FD transition after a successful core and close.
                // This is an explicit source-parity test, not SDK recovery policy.
                assert_eq!(native.input_fd, -1);
                native.init_daemon(node).unwrap();
            }
            assert_eq!(native.input_fd, -1);
            let objects = (
                native.resources.loop_,
                native.resources.core,
                native.resources.stream,
            );
            native.init_daemon(node).unwrap();
            assert_eq!(
                objects,
                (
                    native.resources.loop_,
                    native.resources.core,
                    native.resources.stream
                ),
                "already-connected init must not replace native objects"
            );
            for call in 0..3 {
                if call == 1 {
                    consumer.inactive(&mut native);
                    consumer.inactive(&mut native);
                }
                let result = consumer.capture(&mut native, &mut RawAudit);
                assert!(!result.replayed);
                let frame = result.image.unwrap().unwrap();
                fs::write(
                    directory.join(format!("cycle-{cycle}-session-{session}-call-{call}.rgb")),
                    frame.pixels(),
                )
                .unwrap();
                let serial = marker(&frame);
                held.push((frame, serial));
            }
            let live = live_tasks();
            native.close();
            consumer.reset_after_close();
            native.close();
            consumer.inactive(&mut native);
            assert!(!native.connected());
            assert_eq!(native.input_fd, -1);
            assert!(native.resources.loop_.is_null());
            assert!(native.resources.core.is_null());
            assert!(native.resources.stream.is_null());
            // SAFETY: Stop/join and native teardown completed, no callbacks run.
            assert!(unsafe { &*native.callbacks.state.get() }.cache.is_empty());
            assert!(matches!(
                consumer.capture(&mut native, &mut RawAudit).image,
                Err(crate::backends::linux::pipewire::Error::Disconnected)
            ));
            // Check the consumed input before any diagnostic FD can reuse it.
            if session == 0 {
                // SAFETY: Observation only; successful core owned this socket.
                assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EBADF)
                );
            }
            let evidence = native.evidence();
            assert_eq!(evidence.fault, 0, "{evidence:?}");
            assert_eq!(evidence.queue_error, 0, "{evidence:?}");
            assert_eq!(evidence.core_disconnect_error, 0, "{evidence:?}");
            assert_eq!((evidence.mapping_error, evidence.unmap_error), (0, 0));
            assert_eq!(evidence.owned_maps, evidence.unmap_calls);
            assert_eq!(evidence.dequeued, evidence.returned);
            assert_eq!(evidence.copies, u64::try_from((session + 1) * 3).unwrap());
            assert_eq!(
                evidence.allocated,
                u64::try_from((session + 1) * 3).unwrap()
            );
            assert_resources(&directory, cycle * 2 + session, fds, &baseline, &live);
            for (frame, serial) in &held {
                assert_eq!(marker(frame), *serial);
            }
            eprintln!(
                "native Rust reinit cycle={cycle} session={session} closed evidence={evidence:?}"
            );
        }
        drop(native);
        drop(balance);
        assert_resources(&directory, cycle + 40, fds, &baseline, &[]);
    }
    std::thread::spawn(move || {
        for (frame, serial) in held {
            assert_eq!(marker(&frame), serial);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "native Rust PipeWire reinit objects=20 sessions=40 captures=120 retained=120 consumed_sockets_closed=20 passed"
    );
}
