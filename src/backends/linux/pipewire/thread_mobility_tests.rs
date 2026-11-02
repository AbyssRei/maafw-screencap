// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native class mobility only; opaque Mat/loader owners remain thread-local.
use super::*;
use std::{
    fmt::Write,
    sync::{Condvar, Mutex, atomic::AtomicU32, mpsc},
};

#[derive(Default)]
struct Gate {
    opened: Mutex<bool>,
    changed: Condvar,
}
impl Gate {
    fn open(&self) {
        *self.opened.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.changed.notify_all();
    }
    fn wait(&self) {
        let mut opened = self.opened.lock().unwrap_or_else(|p| p.into_inner());
        while !*opened {
            opened = self.changed.wait(opened).unwrap_or_else(|p| p.into_inner());
        }
    }
}
struct Release(Arc<Gate>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.open();
    }
}
struct GlobalInit(Arc<AtomicBool>);
impl GlobalInit {
    fn release(&self) {
        if self.0.swap(false, Ordering::AcqRel) {
            // SAFETY: All three caller threads have joined and their Capture
            // owner stopped/destroyed native state first, including on unwind.
            // This balances only the fixture's actually acquired raw init.
            unsafe { sys::pw_deinit() };
        }
    }
}
impl Drop for GlobalInit {
    fn drop(&mut self) {
        self.release();
    }
}
struct CpuImage {
    raw: RawFrame,
    bgr: Vec<u8>,
    serial: u32,
}
struct CpuConverter<'a>(&'a Adapter);
impl Converter<RawFrame> for CpuConverter<'_> {
    type Image = CpuImage;
    type Error = AdapterError;
    fn convert(
        &mut self,
        raw: Option<RawFrame>,
        mode: Conversion,
    ) -> Result<Option<CpuImage>, AdapterError> {
        let raw = raw.ok_or(AdapterError::InvalidLayout)?;
        assert_eq!(mode, Conversion::Rgb);
        let image = Bgr::Converted(self.0.convert(&raw, mode)?);
        let serial = known_bgr(&image);
        assert_eq!(marker(&raw), serial);
        // Explicit test-only CPU snapshot. The opaque Mat and loader do not
        // enter the channel or acquire an unproved Send/Sync implementation.
        let bgr = image.pixels().to_vec();
        drop(image);
        assert_eq!(self.0.owner_count(), 0);
        Ok(Some(CpuImage { raw, bgr, serial }))
    }
    fn reuse(&mut self, _: Option<RawFrame>) -> Option<CpuImage> {
        panic!("RGB mobility fixture unexpectedly selected BGR reuse");
    }
}
struct UnwindConverter;
impl Converter<RawFrame> for UnwindConverter {
    type Image = ();
    type Error = ();
    fn convert(&mut self, raw: Option<RawFrame>, mode: Conversion) -> Result<Option<()>, ()> {
        assert_eq!(mode, Conversion::Rgb);
        marker(&raw.unwrap());
        panic!("controlled mobility converter unwind after real native copy");
    }
    fn reuse(&mut self, _: Option<RawFrame>) -> Option<()> {
        unreachable!("RGB mobility fixture cannot reuse BGR");
    }
}
struct Packet {
    capture: Capture,
    frames: Vec<CpuImage>,
    live: Vec<Task>,
    loop_id: u32,
    loop_started: u64,
    offered: i32,
    panic_caught: bool,
}
struct Closed {
    frames: Vec<CpuImage>,
    live: Vec<Task>,
    loop_id: u32,
    loop_started: u64,
    offered: i32,
    fd_closed: bool,
    evidence: Evidence,
    panic_caught: bool,
}
fn identify(slot: usize, ids: &[AtomicU32; 3], ready: &mpsc::SyncSender<()>, start: &Gate) {
    // SAFETY: These Linux calls query/name this actual owned caller thread.
    let (id, named) = unsafe {
        (
            libc::gettid(),
            libc::pthread_setname_np(libc::pthread_self(), c"MaaScreencap".as_ptr()),
        )
    };
    ids[slot].store(u32::try_from(id).unwrap(), Ordering::Release);
    ready.send(()).unwrap();
    start.wait();
    assert_eq!(named, 0);
}
fn id(task: &Task) -> u32 {
    task.id.to_str().unwrap().parse().unwrap()
}
fn loop_generation(loop_id: u32, started: u64) {
    let tasks = live_tasks();
    let task = tasks.iter().find(|t| id(t) == loop_id).unwrap();
    assert_eq!(task.started, started);
    assert!(task.started > 0 && task.flags & 4 == 0 && task.address_space > 0);
    assert_eq!(task.name, "MaaScreencap");
}
fn take(
    capture: &mut Capture,
    adapter: &Adapter,
    directory: &Path,
    cycle: usize,
    call: usize,
) -> CpuImage {
    let result = capture.capture(&mut CpuConverter(adapter));
    assert!(!result.replayed);
    let frame = result.image.unwrap().unwrap();
    fs::write(
        directory.join(format!("cycle-{cycle}-call-{call}.bgr")),
        &frame.bgr,
    )
    .unwrap();
    fs::write(
        directory.join(format!("cycle-{cycle}-call-{call}.rgb")),
        frame.raw.pixels(),
    )
    .unwrap();
    frame
}
fn row_tasks(directory: &Path, cycle: usize, live: &[Task], baseline: &[std::ffi::OsString]) {
    let mut text = String::from("id\tname\tstarted\tflags\taddress_space\n");
    let mut count = 0;
    for task in live.iter().filter(|t| !baseline.contains(&t.id)) {
        writeln!(
            &mut text,
            "{}\t{}\t{}\t{}\t{}",
            id(task),
            task.name,
            task.started,
            task.flags,
            task.address_space
        )
        .unwrap();
        count += 1;
    }
    assert_eq!(count, 4);
    fs::write(directory.join(format!("cycle-{cycle}-live.tsv")), text).unwrap();
}
fn save_ids(path: &Path, ids: &[std::ffi::OsString]) {
    let mut text = String::from("id\n");
    for id in ids {
        writeln!(&mut text, "{}", id.to_str().unwrap()).unwrap();
    }
    fs::write(path, text).unwrap();
}

#[test]
#[ignore = "requires real original/Rust thread-handoff matrix, daemon and offered FD"]
fn moves_complete_capture_between_live_callers_and_unwinds_before_foreign_close() {
    use std::io::Write as IoWrite;
    let root = std::path::PathBuf::from(env::var_os("MFS_P1_PIPEWIRE_THREAD_MOBILITY").unwrap());
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    let node: u32 = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse()
        .unwrap();
    let mut held = Vec::new();
    for mode in 0..2 {
        let directory = root.join(format!("rust/mode-{mode}"));
        fs::create_dir_all(&directory).unwrap();
        let mut rows = fs::File::create(directory.join("rows.tsv")).unwrap();
        writeln!(rows,"cycle\tcreator_tid\tcapture_tid\tcloser_tid\tloop_tid\tloop_started\tfd_before\tfd_after\ttasks_before\ttasks_after\toffered_fd\toffered_fd_closed\tcaptures\tretained_frames\tpanic_caught\tcopies\tdequeued\treturned\tqueue_error\tmap_error\tunmap_error\tfault\tcore_disconnect_error").unwrap();
        let fds = count("/proc/self/fd");
        let baseline = task_ids();
        save_ids(&directory.join("baseline.tsv"), &baseline);
        for cycle in 0..20 {
            let ids: Arc<[AtomicU32; 3]> = Arc::new(std::array::from_fn(|_| AtomicU32::new(0)));
            let acquired = Arc::new(AtomicBool::new(false));
            let global = GlobalInit(acquired.clone());
            let start = Arc::new(Gate::default());
            let finished = Arc::new(Gate::default());
            let (ready, all_ready) = mpsc::sync_channel(3);
            let (created, to_capture) = mpsc::sync_channel::<Packet>(0);
            let (captured, to_close) = mpsc::sync_channel::<Packet>(0);
            let closed = std::thread::scope(|scope| {
                // Both gates open on startup failure before Scope joins any
                // already-created caller. Startup identities use atomics.
                let _start_on_exit = Release(start.clone());
                let _finish_on_exit = Release(finished.clone());
                let creator = {
                    let (ids, ready, start, finished) =
                        (ids.clone(), ready.clone(), start.clone(), finished.clone());
                    let socket = &socket;
                    let baseline = &baseline;
                    scope.spawn(move || {
                        identify(0, &ids, &ready, &start);
                        let callers = ids.each_ref().map(|v| v.load(Ordering::Acquire));
                        assert!(callers.iter().all(|&id| id > 0));
                        assert!(
                            callers[0] != callers[1]
                                && callers[0] != callers[2]
                                && callers[1] != callers[2]
                        );
                        let offered = (mode == 1).then(|| UnixStream::connect(socket).unwrap());
                        let mut capture = if let Some(stream) = &offered {
                            // SAFETY: Fresh sole-owned socket remains owned by
                            // stream through fallible construction; detach it
                            // immediately before any native transfer attempt.
                            unsafe { Capture::with_fd(stream.as_raw_fd()) }.unwrap()
                        } else {
                            Capture::new().unwrap()
                        };
                        acquired.store(true, Ordering::Release);
                        let offered = offered.map_or(-1, IntoRawFd::into_raw_fd);
                        if mode == 1 {
                            // SAFETY: Same exact detached live offer; no other
                            // thread owns/closes it, and attempts are serialized.
                            unsafe { capture.init_source(node) }.unwrap();
                        } else {
                            capture.init_daemon(node).unwrap();
                        }
                        assert!(capture.connected());
                        let live = live_tasks();
                        assert_eq!(live.len(), baseline.len() + 4);
                        let extra: Vec<_> =
                            live.iter().filter(|t| !baseline.contains(&t.id)).collect();
                        assert!(extra.iter().all(|t| t.name == "MaaScreencap"
                            && t.started > 0
                            && t.flags & 4 == 0
                            && t.address_space > 0));
                        let loop_task = extra
                            .into_iter()
                            .find(|t| !callers.contains(&id(t)))
                            .unwrap();
                        let (loop_id, loop_started) = (id(loop_task), loop_task.started);
                        created
                            .send(Packet {
                                capture,
                                frames: Vec::new(),
                                live,
                                loop_id,
                                loop_started,
                                offered,
                                panic_caught: false,
                            })
                            .unwrap_or_else(|_| panic!("capture caller closed before handoff"));
                        finished.wait();
                    })
                };
                let capturer = {
                    let (ids, ready, start, finished) =
                        (ids.clone(), ready.clone(), start.clone(), finished.clone());
                    let directory = &directory;
                    scope.spawn(move || {
                        identify(1, &ids, &ready, &start);
                        let mut packet = to_capture.recv().unwrap();
                        loop_generation(packet.loop_id, packet.loop_started);
                        {
                            let adapter = Adapter::open();
                            assert_eq!(adapter.owner_count(), 0);
                            packet.frames.push(take(
                                &mut packet.capture,
                                &adapter,
                                directory,
                                cycle,
                                0,
                            ));
                            packet.capture.inactive();
                            packet.frames.push(take(
                                &mut packet.capture,
                                &adapter,
                                directory,
                                cycle,
                                1,
                            ));
                            let panic =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    packet.capture.capture(&mut UnwindConverter)
                                }));
                            assert!(panic.is_err());
                            packet.panic_caught = true;
                            // Next caller must acquire the real native lock
                            // while this unwinding caller remains alive.
                            packet.capture.evidence();
                            assert_eq!(adapter.owner_count(), 0);
                        } // every opaque adapter/Mat ends BEFORE handoff
                        captured
                            .send(packet)
                            .unwrap_or_else(|_| panic!("closer closed before handoff"));
                        finished.wait();
                    })
                };
                let closer = {
                    let (ids, ready, start, finished) =
                        (ids.clone(), ready, start.clone(), finished.clone());
                    let directory = &directory;
                    scope.spawn(move || {
                        let _finish = Release(finished);
                        identify(2, &ids, &ready, &start);
                        let mut packet = to_close.recv().unwrap();
                        loop_generation(packet.loop_id, packet.loop_started);
                        {
                            let adapter = Adapter::open();
                            assert_eq!(adapter.owner_count(), 0);
                            packet.frames.push(take(
                                &mut packet.capture,
                                &adapter,
                                directory,
                                cycle,
                                2,
                            ));
                            assert_eq!(adapter.owner_count(), 0);
                        }
                        packet.capture.close();
                        assert!(!packet.capture.connected());
                        let evidence = packet.capture.evidence();
                        drop(packet.capture);
                        let fd_closed = if mode == 1 {
                            // SAFETY: Scalar F_GETFD observation immediately
                            // after native teardown, before any artifact I/O.
                            let result = unsafe { libc::fcntl(packet.offered, libc::F_GETFD) };
                            result == -1
                                && std::io::Error::last_os_error().raw_os_error()
                                    == Some(libc::EBADF)
                        } else {
                            false
                        };
                        Closed {
                            frames: packet.frames,
                            live: packet.live,
                            loop_id: packet.loop_id,
                            loop_started: packet.loop_started,
                            offered: packet.offered,
                            fd_closed,
                            evidence,
                            panic_caught: packet.panic_caught,
                        }
                    })
                };
                for _ in 0..3 {
                    all_ready.recv().unwrap();
                }
                start.open();
                let closed = closer.join();
                let captured = capturer.join();
                let created = creator.join();
                created.unwrap();
                captured.unwrap();
                closed.unwrap()
            });
            global.release();
            assert!(mode == 0 || closed.fd_closed);
            assert_resources(&directory, cycle, fds, &baseline, &closed.live);
            assert_eq!(closed.frames.len(), 3);
            assert!(closed.panic_caught);
            let evidence = &closed.evidence;
            assert_eq!(evidence.copies, 4);
            assert_eq!(evidence.dequeued, evidence.returned);
            assert!(evidence.dequeued >= 4);
            assert_eq!(
                (
                    evidence.fault,
                    evidence.queue_error,
                    evidence.mapping_error,
                    evidence.unmap_error,
                    evidence.core_disconnect_error
                ),
                (0, 0, 0, 0, 0)
            );
            row_tasks(&directory, cycle, &closed.live, &baseline);
            save_ids(
                &directory.join(format!("cycle-{cycle}-final.tsv")),
                &task_ids(),
            );
            for (call, frame) in closed.frames.into_iter().enumerate() {
                assert_eq!(marker(&frame.raw), frame.serial);
                let (rgb, rgb_remainder) = frame.raw.pixels().as_chunks::<3>();
                let (bgr, bgr_remainder) = frame.bgr.as_chunks::<3>();
                assert!(rgb_remainder.is_empty() && bgr_remainder.is_empty());
                assert_eq!(rgb.len(), bgr.len());
                assert!(
                    rgb.iter()
                        .zip(bgr)
                        .all(|(rgb, bgr)| rgb[0] == bgr[2] && rgb[1] == bgr[1] && rgb[2] == bgr[0])
                );
                fs::write(
                    directory.join(format!("cycle-{cycle}-held-{call}.bgr")),
                    &frame.bgr,
                )
                .unwrap();
                fs::write(
                    directory.join(format!("cycle-{cycle}-held-{call}.rgb")),
                    frame.raw.pixels(),
                )
                .unwrap();
                held.push(frame);
            }
            let callers = ids.each_ref().map(|v| v.load(Ordering::Acquire));
            writeln!(rows,"{cycle}\t{}\t{}\t{}\t{}\t{}\t{fds}\t{}\t{}\t{}\t{}\t{}\t3\t3\t1\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                callers[0],callers[1],callers[2],closed.loop_id,closed.loop_started,count("/proc/self/fd"),baseline.len(),task_ids().len(),closed.offered,u8::from(closed.fd_closed),
                evidence.copies,evidence.dequeued,evidence.returned,evidence.queue_error,evidence.mapping_error,evidence.unmap_error,evidence.fault,evidence.core_disconnect_error).unwrap();
        }
    }
    for frame in held {
        assert_eq!(marker(&frame.raw), frame.serial);
    }
    eprintln!(
        "native PipeWire thread mobility objects=40 callers=3 captures=120 retained=120 controlled_unwinds=40 fd_tasks_balanced=40 passed"
    );
}
