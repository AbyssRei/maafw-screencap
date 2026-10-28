// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::linux::pipewire::{Consumer, Conversion, Converter};
use std::{env, ffi::CStr, fs, path::Path};

#[path = "opencv_tests.rs"]
mod opencv;

struct RawAudit;
impl Converter<RawFrame> for RawAudit {
    type Image = RawFrame;
    type Error = ();
    fn convert(&mut self, raw: Option<RawFrame>, mode: Conversion) -> Result<Option<RawFrame>, ()> {
        // This is explicitly an RGB raw-copy probe, not BGR conversion/parity.
        assert_eq!(mode, Conversion::Rgb);
        Ok(raw)
    }
    fn reuse(&mut self, raw: Option<RawFrame>) -> Option<RawFrame> {
        raw
    }
}
struct BalanceInit;
impl Drop for BalanceInit {
    fn drop(&mut self) {
        // SAFETY: Declared before the driver owner, so driver stop, hooks and
        // native resources are all destroyed first, including during unwinding.
        // This test-only balancing is outside the source-equivalent destructor.
        unsafe {
            sys::pw_deinit();
        }
    }
}
fn count(path: &str) -> usize {
    fs::read_dir(path).unwrap().count()
}
fn marker(frame: &RawFrame) -> u32 {
    assert_eq!(
        (frame.width, frame.height, frame.stride, frame.format),
        (28, 24, 84, Format::Rgb)
    );
    let pixels = frame.pixels();
    assert_eq!(pixels.len(), 28 * 24 * 3);
    let serial = (u32::from(pixels[0]) << 16) | (u32::from(pixels[1]) << 8) | u32::from(pixels[2]);
    assert_ne!(serial, 0);
    let bars = [
        [191, 191, 191],
        [191, 191, 0],
        [0, 191, 191],
        [0, 191, 0],
        [191, 0, 191],
        [191, 0, 0],
        [0, 0, 191],
    ];
    for y in 0..24 {
        for x in 0..28 {
            if x == 0 && y == 0 {
                continue;
            }
            for c in 0..3 {
                let delta = (y * 7 + x * 3 + c * 13 + usize::try_from(serial & 63).unwrap()) & 63;
                assert_eq!(
                    pixels[y * 84 + x * 3 + c],
                    bars[x / 4][c] ^ u8::try_from(delta).unwrap(),
                    "fixture row={y} x={x} channel={c} serial={serial}"
                );
            }
        }
    }
    serial
}

#[test]
#[ignore = "explicit native Linux owned-daemon SHM driver probe, not full source/OpenCV parity"]
fn captures_requested_shm_rows_and_retains_sixty_cpu_samples_after_shutdown() {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").expect("explicit private producer socket");
    let remote = env::var("PIPEWIRE_REMOTE").expect("explicit private daemon name");
    let runtime = env::var("PIPEWIRE_RUNTIME_DIR").expect("explicit private runtime");
    assert_eq!(Path::new(&runtime).join(&remote), Path::new(&socket));
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let directory =
        env::var("MFS_P1_PIPEWIRE_RUST_ARTIFACTS").expect("explicit artifact directory");
    fs::create_dir_all(&directory).unwrap();
    let expected = env::var("MFS_P0_PIPEWIRE_VERSION").expect("explicit native version");
    let oracle =
        env::var("MFS_P1_PIPEWIRE_POD_ORACLE").expect("explicit original native negotiation PODs");
    let params = pod::Params::new().unwrap();
    for (index, param) in params.pointers().into_iter().enumerate() {
        // SAFETY: Params owns these complete aligned initialized PODs throughout
        // this synchronous comparison. Their bounded builder checked all writes.
        let size = unsafe { (*param).size }
            .checked_add(u32::try_from(mem::size_of::<spa::sys::spa_pod>()).unwrap())
            .unwrap();
        assert!(size <= 4096);
        // SAFETY: Same owner and validated complete extent, no native mutation.
        let actual = unsafe {
            std::slice::from_raw_parts(param.cast::<u8>(), usize::try_from(size).unwrap())
        };
        assert_eq!(
            actual,
            fs::read(Path::new(&oracle).join(format!("param-{index}.pod"))).unwrap(),
            "native source POD index={index}"
        );
    }
    // SAFETY: Library-version APIs return static NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(sys::pw_get_library_version()) }
            .to_str()
            .unwrap(),
        expected
    );
    let fds = count("/proc/self/fd");
    let threads = count("/proc/self/task");
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        // new is fallible only before it performs raw global initialization.
        let native = Native::new().unwrap();
        let balance_init = BalanceInit;
        let mut native = native;
        native.init_daemon(node).unwrap();
        let mut consumer = Consumer::default();
        for call in 0..3 {
            if call == 1 {
                consumer.inactive(&mut native);
            }
            let result = consumer.capture(&mut native, &mut RawAudit);
            assert!(
                !result.replayed,
                "requested fixture should deliver a native frame"
            );
            let frame = result.image.unwrap().unwrap();
            // Persist bytes before strict color/identity assertions.
            fs::write(
                Path::new(&directory).join(format!("cycle-{cycle}-call-{call}.rgb")),
                frame.pixels(),
            )
            .unwrap();
            let serial = marker(&frame);
            held.push((frame, serial));
            eprintln!("native Rust SHM cycle={cycle} call={call} marker={serial} copied_rgb=1");
        }
        native.close();
        consumer.reset_after_close();
        assert!(!native.connected());
        let evidence = native.evidence();
        assert_eq!(evidence.fault, 0, "{evidence:?}");
        assert_eq!(evidence.queue_error, 0, "{evidence:?}");
        assert_eq!(evidence.core_disconnect_error, 0, "{evidence:?}");
        assert_eq!(evidence.dequeued, evidence.returned, "{evidence:?}");
        assert_eq!(evidence.copies, 3, "{evidence:?}");
        assert_eq!(
            evidence.allocated, 3,
            "all three previous samples remain retained"
        );
        eprintln!("native Rust SHM closed cycle={cycle} evidence={evidence:?}");
        drop(native);
        drop(balance_init);
        assert_eq!(count("/proc/self/fd"), fds, "fd count after cycle={cycle}");
        assert_eq!(
            count("/proc/self/task"),
            threads,
            "thread count after cycle={cycle}"
        );
        for (frame, serial) in &held {
            assert_eq!(marker(frame), *serial, "retained sample identity changed");
        }
    }
    std::thread::spawn(move || {
        for (frame, serial) in held {
            assert_eq!(
                marker(&frame),
                serial,
                "post-shutdown cross-thread sample identity changed"
            );
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "native Rust PipeWire daemon SHM cycles=20 captures=60 retained=60 passed; OpenCV conversion/source parity pending"
    );
}
