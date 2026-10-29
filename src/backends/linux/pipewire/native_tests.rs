// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::linux::pipewire::{Consumer, Conversion, Converter};
use std::{
    env,
    ffi::CStr,
    fs,
    os::{
        fd::{AsRawFd, IntoRawFd},
        unix::net::UnixStream,
    },
    path::Path,
    time::Duration,
};

#[path = "opencv_tests.rs"]
mod opencv;

#[path = "reinit_tests.rs"]
mod reinit;

#[path = "fd_fault_tests.rs"]
mod fd_fault;

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
#[derive(Debug)]
struct Task {
    id: std::ffi::OsString,
    name: String,
    started: u64,
    flags: u64,
    address_space: u64,
    resident_pages: i64,
}
fn task(id: &std::ffi::OsStr) -> Option<Task> {
    let text = match fs::read_to_string(Path::new("/proc/self/task").join(id).join("stat")) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("cannot inspect task {id:?}: {error}"),
    };
    // proc stat fields 9/22/23/24, after the final ')' of field 2. The
    // comm field itself can contain whitespace/parentheses; split it first.
    let fields: Vec<_> = text
        .rsplit_once(')')
        .unwrap()
        .1
        .split_whitespace()
        .collect();
    Some(Task {
        id: id.to_owned(),
        name: text
            .split_once('(')
            .unwrap()
            .1
            .rsplit_once(')')
            .unwrap()
            .0
            .to_owned(),
        started: fields[19].parse().unwrap(),
        flags: fields[6].parse().unwrap(),
        address_space: fields[20].parse().unwrap(),
        resident_pages: fields[21].parse().unwrap(),
    })
}
fn live_tasks() -> Vec<Task> {
    task_ids().iter().filter_map(|id| task(id)).collect()
}
fn only_retiring_tasks(
    actual: &[std::ffi::OsString],
    baseline: &[std::ffi::OsString],
    live: &[Task],
) -> bool {
    baseline.iter().all(|id| actual.contains(id))
        && actual.iter().filter(|id| !baseline.contains(id)).all(|id| {
            let Some(before) = live.iter().find(|before| before.id == *id) else {
                return false; // Never forgive an unknown task or substituted TID.
            };
            if before.name != "MaaScreencap" || before.flags & 0x4 != 0 || before.address_space == 0
            {
                return false;
            }
            task(id).is_none_or(|after| {
                // PF_EXITING is kernel state, not a public capture capability.
                // No userspace address space may survive close. A still-live
                // worker is a failure, even if it might stop during a later wait.
                after.started == before.started
                    && after.flags & 0x4 != 0
                    && after.address_space == 0
                    && after.resident_pages == 0
            })
        })
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
fn assert_resources(
    directory: &Path,
    cycle: usize,
    fds: usize,
    baseline: &[std::ffi::OsString],
    live: &[Task],
) {
    let actual_fds = count("/proc/self/fd");
    let tasks = task_ids();
    let actual_threads = tasks.len();
    let mut settled = tasks == baseline;
    if actual_fds == fds && !settled && only_retiring_tasks(&tasks, baseline, live) {
        task_snapshot(
            directory,
            &format!("cycle-{cycle}-kernel-retirement"),
            &tasks,
        );
        fs::write(
            directory.join(format!("tasks-cycle-{cycle}-live-identities.txt")),
            format!("{live:#?}\n"),
        )
        .unwrap();
        // Native stop/join and full destruction have already completed. Wait
        // only for known PF_EXITING/no-mm tasks to leave proc, never for live
        // callbacks, asynchronous cleanup, an FD, or another capture attempt.
        let start = Instant::now();
        loop {
            let current = task_ids();
            if current == baseline {
                settled = true;
                break;
            }
            if !only_retiring_tasks(&current, baseline, live)
                || start.elapsed() >= Duration::from_millis(100)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        eprintln!(
            "kernel task retirement cycle={cycle} immediate_tasks={actual_threads} baseline={} settled={settled} elapsed_us={}",
            baseline.len(),
            start.elapsed().as_micros()
        );
    }
    if actual_fds != fds || !settled {
        eprintln!(
            "native resource mismatch cycle={cycle} fds={actual_fds}/{fds} threads={actual_threads}/{} ids={tasks:?}; diagnostic observations cannot change this failure",
            baseline.len()
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
    assert!(settled, "unretired/unknown/live task after cycle={cycle}");
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
    run_raw_probe(false, false);
}

#[test]
#[ignore = "explicit native supplied-socket success ownership, not Portal authorization/failure acceptance"]
fn captures_supplied_fd_shm_and_closes_twenty_consumed_sockets() {
    run_raw_probe(true, false);
}

#[test]
#[ignore = "requires the owned real kernel DMA producer and native source POD oracle"]
fn captures_null_data_dma_stream_with_paired_sync_and_retained_cpu_samples() {
    run_raw_probe(false, true);
}

fn run_raw_probe(supplied_fd: bool, dma_stream: bool) {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").expect("explicit private producer socket");
    let remote = env::var("PIPEWIRE_REMOTE").expect("explicit private daemon name");
    let runtime = env::var("PIPEWIRE_RUNTIME_DIR").expect("explicit private runtime");
    assert_eq!(Path::new(&runtime).join(&remote), Path::new(&socket));
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let directory = env::var(if dma_stream {
        "MFS_P1_PIPEWIRE_DMA_STREAM_ARTIFACTS"
    } else if supplied_fd {
        "MFS_P1_PIPEWIRE_FD_ARTIFACTS"
    } else {
        "MFS_P1_PIPEWIRE_RUST_ARTIFACTS"
    })
    .expect("explicit artifact directory");
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
    let baseline = task_ids();
    task_snapshot(Path::new(&directory), "baseline", &baseline);
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        // new is fallible only before it performs raw global initialization.
        let offered = supplied_fd.then(|| UnixStream::connect(&socket).unwrap());
        let native = if let Some(stream) = &offered {
            // SAFETY: Fresh uniquely owned private socket. No initialization
            // occurs yet; the owning handle is detached immediately below,
            // before any native transfer. A constructor error retains it.
            unsafe { Native::with_fd(stream.as_raw_fd()) }.unwrap()
        } else {
            Native::new().unwrap()
        };
        let balance_init = BalanceInit;
        let mut native = native;
        let input_fd = offered.map(IntoRawFd::into_raw_fd);
        if supplied_fd {
            // SAFETY: Sole ownership was detached for this one native attempt.
            // A failed attempt fails this isolated process; no generic residual
            // close or automatic FD retry is inferred from a null native core.
            unsafe { native.init_source(node) }.unwrap();
            assert_eq!(native.input_fd, -1, "non-null core consumed the offer");
        } else {
            native.init_daemon(node).unwrap();
        }
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
        let live = live_tasks();
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
        assert_eq!(
            (evidence.mapping_error, evidence.unmap_error),
            (0, 0),
            "{evidence:?}"
        );
        assert_eq!(evidence.owned_maps, evidence.unmap_calls, "{evidence:?}");
        if dma_stream {
            // Three actual callback copies, each with NULL incoming data, must
            // own a DMA map. Existing pointers bypass these sync counters in U;
            // normal mapped MemFd frames cannot satisfy these assertions.
            assert_eq!(evidence.owned_maps, 3, "{evidence:?}");
            assert_eq!(evidence.dma.borrowed, 0, "{evidence:?}");
            assert_eq!((evidence.dma.starts, evidence.dma.ends), (3, 3));
            assert_eq!((evidence.dma.start_error, evidence.dma.end_error), (0, 0));
        }
        assert_eq!(evidence.dequeued, evidence.returned, "{evidence:?}");
        assert_eq!(evidence.copies, 3, "{evidence:?}");
        assert_eq!(
            evidence.allocated, 3,
            "all three previous samples remain retained"
        );
        eprintln!("native Rust SHM closed cycle={cycle} evidence={evidence:?}");
        drop(native);
        drop(balance_init);
        if let Some(fd) = input_fd {
            // SAFETY: Observe only, before opening any diagnostic descriptors.
            // The input number was detached and consumed by the successful core;
            // no owning Rust handle can double-close it. No close occurs here.
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
        }
        assert_resources(Path::new(&directory), cycle, fds, &baseline, &live);
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
    if dma_stream {
        eprintln!(
            "native Rust PipeWire DMA stream cycles=20 captures=60 NULL_data=60 READ_START=60 READ_END=60 unmaps=60 retained=60 passed"
        );
    } else if supplied_fd {
        eprintln!(
            "native Rust PipeWire supplied fd SHM cycles=20 captures=60 retained=60 consumed_sockets_closed=20 passed"
        );
    } else {
        eprintln!(
            "native Rust PipeWire daemon SHM cycles=20 captures=60 retained=60 passed; OpenCV conversion/source parity pending"
        );
    }
}
