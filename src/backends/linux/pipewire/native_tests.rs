// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::linux::pipewire::{Consumer, Conversion, Converter};
use std::{env, ffi::CStr, fs, path::Path, time::Duration};

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
fn task_ids() -> Vec<std::ffi::OsString> {
    let mut tasks: Vec<_> = fs::read_dir("/proc/self/task")
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    tasks.sort();
    tasks
}
fn task_snapshot(directory: &Path, label: &str, tasks: &[std::ffi::OsString]) {
    let mut output = format!("tasks observed={} ids={tasks:?}\n", tasks.len());
    for tid in tasks {
        for name in ["comm", "stat", "status", "wchan"] {
            let path = Path::new("/proc/self/task").join(tid).join(name);
            output.push_str(&format!("{path:?}: {:?}\n", fs::read_to_string(&path)));
        }
    }
    fs::write(directory.join(format!("tasks-{label}.txt")), output).unwrap();
}
fn assert_resources(directory: &Path, cycle: usize, fds: usize, threads: usize) {
    let actual_fds = count("/proc/self/fd");
    let tasks = task_ids();
    let actual_threads = tasks.len();
    if actual_fds != fds || actual_threads != threads {
        eprintln!(
            "native resource mismatch cycle={cycle} fds={actual_fds}/{fds} threads={actual_threads}/{threads} ids={tasks:?}; diagnostic observations cannot change this failure"
        );
        task_snapshot(directory, &format!("cycle-{cycle}-mismatch"), &tasks);
        // Observe disappearance/continued existence only AFTER fixing the failed
        // result. No recapture, cleanup retry, or later count can make it pass.
        for delay in [1, 10, 100] {
            std::thread::sleep(Duration::from_millis(delay));
            let tasks = task_ids();
            task_snapshot(directory, &format!("cycle-{cycle}-after-{delay}ms"), &tasks);
        }
    }
    assert_eq!(actual_fds, fds, "fd count after cycle={cycle}");
    assert_eq!(actual_threads, threads, "thread count after cycle={cycle}");
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
    task_snapshot(Path::new(&directory), "baseline", &task_ids());
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
        if cycle == 0 {
            task_snapshot(Path::new(&directory), "first-live", &task_ids());
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
        assert_resources(Path::new(&directory), cycle, fds, threads);
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
