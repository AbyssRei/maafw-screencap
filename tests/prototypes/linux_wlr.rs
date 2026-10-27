// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Native P0 feasibility, not the migrated backend. Real wlroots performs the
// render/copy; system libwayland performs IPC. Only CPU pixels cross threads.
use memmap2::MmapOptions;
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fs::{MemfdFlags, ftruncate, memfd_create},
};
use std::{
    error::Error,
    fs::File,
    os::{fd::AsFd, unix::net::UnixStream},
    sync::Arc,
    thread::ThreadId,
    time::{Duration, Instant},
};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
    protocol::{wl_buffer, wl_callback, wl_output, wl_registry, wl_shm, wl_shm_pool},
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1 as frame, zwlr_screencopy_manager_v1 as manager,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Clone, Copy, Debug)]
struct Layout {
    format: wl_shm::Format,
    width: u32,
    height: u32,
    stride: u32,
}
#[derive(Debug, PartialEq)]
enum Phase {
    Buffer,
    BufferDone,
    Flags,
    Ready,
}
struct State {
    marker: Arc<()>,
    owner: ThreadId,
    shm: Option<wl_shm::WlShm>,
    output: Option<wl_output::WlOutput>,
    manager: Option<manager::ZwlrScreencopyManagerV1>,
    layout: Option<Layout>,
    buffer_done: bool,
    ready: bool,
    flags: Option<u32>,
    ready_time: Option<(u64, u32)>,
    error: Option<&'static str>,
    syncs: usize,
    reads: usize,
    cancelled: usize,
    events: Vec<Phase>,
}
impl State {
    fn event(&mut self) {
        if std::thread::current().id() != self.owner {
            self.error = Some("event queue ran on a foreign owner thread");
        }
    }
    fn record(&mut self, phase: Phase) {
        self.event();
        if self.events.len() == self.events.capacity() {
            self.error = Some("unexpected event count");
        } else {
            self.events.push(phase);
        }
    }
}
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        state.event();
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_shm" if state.shm.is_none() => {
                    state.shm = Some(registry.bind(name, 1, qh, state.marker.clone()))
                }
                "wl_output" if state.output.is_none() => {
                    state.output =
                        Some(registry.bind(name, version.min(3), qh, state.marker.clone()))
                }
                "zwlr_screencopy_manager_v1" if state.manager.is_none() && version >= 3 => {
                    state.manager = Some(registry.bind(name, 3, qh, state.marker.clone()))
                }
                _ => {}
            }
        }
    }
}
impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.event();
        state.syncs += 1;
    }
}
impl Dispatch<frame::ZwlrScreencopyFrameV1, Arc<()>> for State {
    fn event(
        state: &mut Self,
        _: &frame::ZwlrScreencopyFrameV1,
        event: frame::Event,
        _: &Arc<()>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.event();
        match event {
            frame::Event::Buffer {
                format: WEnum::Value(format),
                width,
                height,
                stride,
            } => {
                if state.layout.is_some() {
                    state.error = Some("duplicate SHM format event");
                }
                state.layout = Some(Layout {
                    format,
                    width,
                    height,
                    stride,
                });
                state.record(Phase::Buffer);
            }
            frame::Event::BufferDone => {
                state.buffer_done = true;
                state.record(Phase::BufferDone);
            }
            frame::Event::Flags {
                flags: WEnum::Value(flags),
            } => {
                state.flags = Some(flags.bits());
                state.record(Phase::Flags);
            }
            frame::Event::Ready {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                if tv_nsec >= 1_000_000_000 || state.flags.is_none() {
                    state.error = Some("invalid native ready evidence");
                }
                state.ready_time = Some((
                    u64::from(tv_sec_hi).checked_shl(32).unwrap() | u64::from(tv_sec_lo),
                    tv_nsec,
                ));
                state.ready = true;
                state.record(Phase::Ready);
            }
            frame::Event::Failed => state.error = Some("native compositor rejected screencopy"),
            frame::Event::LinuxDmabuf { .. } | frame::Event::Damage { .. } => {}
            _ => state.error = Some("unknown screencopy format/flags/event"),
        }
    }
}
macro_rules! ignore {
    ($interface:ty) => {
        impl Dispatch<$interface, Arc<()>> for State {
            fn event(
                state: &mut Self,
                _: &$interface,
                _: <$interface as Proxy>::Event,
                _: &Arc<()>,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                state.event();
            }
        }
    };
}
ignore!(wl_shm::WlShm);
ignore!(wl_output::WlOutput);
ignore!(wl_shm_pool::WlShmPool);
ignore!(wl_buffer::WlBuffer);
ignore!(manager::ZwlrScreencopyManagerV1);

#[derive(Default)]
struct Host {
    callbacks: usize,
}
impl Dispatch<wl_callback::WlCallback, ()> for Host {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.callbacks += 1;
    }
}

fn pump(
    queue: &mut EventQueue<State>,
    connection: &Connection,
    state: &mut State,
    predicate: impl Fn(&State) -> bool,
) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        queue.dispatch_pending(state)?;
        if let Some(error) = state.error {
            return Err(error.into());
        }
        if predicate(state) {
            return Ok(());
        }
        let remain = until
            .checked_duration_since(Instant::now())
            .ok_or("native Wayland predicate exceeded probe budget")?;
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let blocked = match connection.flush() {
            Ok(()) => false,
            Err(wayland_backend::client::WaylandError::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock =>
            {
                true
            }
            Err(error) => return Err(error.into()),
        };
        let events = PollFlags::IN
            | if blocked {
                PollFlags::OUT
            } else {
                PollFlags::empty()
            };
        let mut fds = [PollFd::from_borrowed_fd(guard.connection_fd(), events)];
        let timeout = Timespec {
            tv_sec: i64::try_from(remain.as_secs())?,
            tv_nsec: i64::from(remain.subsec_nanos()),
        };
        let count = match poll(&mut fds, Some(&timeout)) {
            Ok(count) => count,
            Err(rustix::io::Errno::INTR) => {
                drop(guard);
                state.cancelled += 1;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if count == 0 {
            drop(guard);
            state.cancelled += 1;
            return Err("native Wayland poll timed out".into());
        }
        let ready = fds[0].revents();
        if ready.contains(PollFlags::IN) {
            match guard.read() {
                Ok(_) => state.reads += 1,
                Err(wayland_backend::client::WaylandError::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
        } else {
            drop(guard);
            state.cancelled += 1;
            if ready.intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL) {
                return Err("native Wayland connection closed".into());
            }
        }
    }
}

fn validate(pixels: &[u8], format: wl_shm::Format) -> Result<()> {
    if pixels.len() != 64 * 48 * 4 {
        return Err("owned Wlr frame length mismatch".into());
    }
    for y in 0..48_usize {
        for x in 0..64_usize {
            let offset = y
                .checked_mul(64)
                .and_then(|n| n.checked_add(x))
                .and_then(|n| n.checked_mul(4))
                .ok_or("pixel offset overflow")?;
            let expected = if y >= 24 {
                [255, 0, 0]
            } else if x < 32 {
                [0, 0, 255]
            } else {
                [0, 255, 0]
            };
            if pixels.get(offset..offset.checked_add(3).ok_or("pixel span overflow")?)
                != Some(expected.as_slice())
            {
                return Err(format!("real compositor pixel mismatch at ({x},{y})").into());
            }
            if format == wl_shm::Format::Argb8888 && pixels[offset + 3] != 255 {
                return Err("native opaque ARGB alpha mismatch".into());
            }
        }
    }
    Ok(())
}

fn cycle(socket: &str) -> Result<(Vec<u8>, wl_shm::Format)> {
    let marker = Arc::try_new(())?;
    let connection = Connection::from_socket(UnixStream::connect(socket)?)?;
    // This method exists only for the explicitly selected system backend.
    assert!(!connection.backend().display_ptr().is_null());
    let library = std::fs::canonicalize(std::env::var("MFS_P0_WAYLAND_LIBRARY")?)?;
    let maps = std::fs::read_to_string("/proc/self/maps")?;
    let library = library
        .to_str()
        .ok_or("non-UTF-8 native library identity")?;
    assert!(
        maps.lines().any(|line| line.ends_with(library)),
        "capture process did not load the prepared native libwayland-client"
    );
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    let mut events = Vec::new();
    events.try_reserve_exact(16)?;
    let mut state = State {
        marker: marker.clone(),
        owner: std::thread::current().id(),
        shm: None,
        output: None,
        manager: None,
        layout: None,
        buffer_done: false,
        ready: false,
        flags: None,
        ready_time: None,
        error: None,
        syncs: 0,
        reads: 0,
        cancelled: 0,
        events,
    };
    let registry = connection.display().get_registry(&qh, ());
    let mut host_queue = connection.new_event_queue();
    let host_handle = host_queue.handle();
    let mut host = Host::default();
    let host_sync = connection.display().sync(&host_handle, ());
    let sync = connection.display().sync(&qh, ());
    pump(&mut queue, &connection, &mut state, |s| s.syncs == 1)?;
    assert_eq!(
        host.callbacks, 0,
        "capture queue dispatched a foreign host queue"
    );
    host_queue.dispatch_pending(&mut host)?;
    assert_eq!(host.callbacks, 1);
    // Exercise cancellation of a real native prepared read before the next
    // request. Dropping the guard invokes wl_display_cancel_read.
    let guard = queue
        .prepare_read()
        .ok_or("native prepare_read unavailable")?;
    drop(guard);
    state.cancelled += 1;
    let shm = state
        .shm
        .as_ref()
        .ok_or("private compositor has no wl_shm")?
        .clone();
    let output = state
        .output
        .as_ref()
        .ok_or("private compositor has no output")?
        .clone();
    let manager = state
        .manager
        .as_ref()
        .ok_or("private compositor has no screencopy v3")?
        .clone();
    let frame = manager.capture_output(0, &output, &qh, marker.clone());
    pump(&mut queue, &connection, &mut state, |s| s.buffer_done)?;
    let layout = state
        .layout
        .ok_or("native compositor supplied no SHM layout")?;
    if layout.width != 64
        || layout.height != 48
        || !matches!(
            layout.format,
            wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888
        )
    {
        return Err("unexpected real compositor format/dimensions".into());
    }
    let width = usize::try_from(layout.width)?;
    let height = usize::try_from(layout.height)?;
    let stride = usize::try_from(layout.stride)?;
    let row = width.checked_mul(4).ok_or("row size overflow")?;
    let length = stride.checked_mul(height).ok_or("native span overflow")?;
    if stride < row || length > 64 * 1024 * 1024 {
        return Err("native layout exceeds the owned fixture bounds".into());
    }
    let pool_size = i32::try_from(length)?;
    let file = File::from(memfd_create(c"mfs-p0-wlr", MemfdFlags::CLOEXEC)?);
    ftruncate(&file, u64::try_from(length)?)?;
    // SAFETY: Sole owned memfd, fixed checked size. No Rust byte reference is
    // created while the native producer writes; ready transfers read access.
    let mapping = unsafe { MmapOptions::new().len(length).map_mut(&file)? };
    let pool = shm.create_pool(file.as_fd(), pool_size, &qh, marker.clone());
    let buffer = pool.create_buffer(
        0,
        i32::try_from(width)?,
        i32::try_from(height)?,
        i32::try_from(stride)?,
        layout.format,
        &qh,
        marker.clone(),
    );
    frame.copy(&buffer);
    pump(&mut queue, &connection, &mut state, |s| s.ready)?;
    let flags = state.flags.ok_or("ready arrived without flags")?;
    if flags & !1 != 0 {
        return Err("unknown native orientation flags".into());
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(row.checked_mul(height).ok_or("CPU image overflow")?)?;
    for y in 0..height {
        let source_y = if flags & 1 != 0 {
            height
                .checked_sub(y)
                .and_then(|n| n.checked_sub(1))
                .ok_or("row inversion overflow")?
        } else {
            y
        };
        let offset = source_y.checked_mul(stride).ok_or("row offset overflow")?;
        let end = offset.checked_add(row).ok_or("row end overflow")?;
        pixels.extend_from_slice(
            mapping
                .get(offset..end)
                .ok_or("native mapping shorter than target span")?,
        );
    }
    assert_eq!(
        state.events,
        [Phase::Buffer, Phase::BufferDone, Phase::Flags, Phase::Ready]
    );
    frame.destroy();
    buffer.destroy();
    pool.destroy();
    manager.destroy();
    if output.version() >= 3 {
        output.release();
    }
    let cleanup_sync = connection.display().sync(&qh, ());
    pump(&mut queue, &connection, &mut state, |s| s.syncs == 2)?;
    println!(
        "Wlr system backend: format={:?}, stride={}, flags={}, ready_time={:?}, read={}, cancelled={}, foreign queue callbacks=1, owner={:?}",
        layout.format, stride, flags, state.ready_time, state.reads, state.cancelled, state.owner
    );
    drop((
        frame,
        buffer,
        pool,
        manager,
        output,
        shm,
        cleanup_sync,
        sync,
        host_sync,
        registry,
    ));
    drop(mapping);
    drop(file);
    drop(state);
    drop(qh);
    drop(host_handle);
    drop(queue);
    drop(host_queue);
    drop(connection);
    assert_eq!(
        Arc::strong_count(&marker),
        1,
        "native proxies or queued callbacks retained user data after disconnect"
    );
    validate(&pixels, layout.format)?;
    Ok((pixels, layout.format))
}

#[test]
#[ignore = "explicit private native wlroots/Pixman compositor; native Linux Actions only"]
fn captures_real_compositor_pixels_isolates_queues_and_releases_native_owners() -> Result<()> {
    let socket = std::env::var("MFS_P0_WLR_SOCKET")?;
    let mut final_frame = None;
    for index in 1..=20 {
        println!("Wlr lifecycle cycle {index}/20");
        final_frame = Some(cycle(&socket)?);
    }
    let (pixels, format) = final_frame.ok_or("no native Wlr sample")?;
    std::thread::spawn(move || validate(&pixels, format).map_err(|error| error.to_string()))
        .join()
        .unwrap()
        .map_err(|error| -> Box<dyn Error> { error.into() })?;
    Ok(())
}
