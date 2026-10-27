// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// P0 feasibility only: an owned daemon/C source, not portal desktop capture
// or the migrated upstream backend. No native object is declared Send/Sync.
use pipewire as pw;
use pw::{properties::properties, spa};
use std::{
    error::Error,
    ffi::CStr,
    os::unix::net::UnixStream,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering},
    },
    thread::ThreadId,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const WIDTH: usize = 28;
const HEIGHT: usize = 24;

#[derive(Default)]
struct Evidence {
    first: Option<Vec<u8>>,
    callback_thread: Option<ThreadId>,
}
#[derive(Default)]
struct Shared {
    state: AtomicU32,
    error: AtomicU32,
    frames: AtomicUsize,
    dequeued: AtomicUsize,
    queued: AtomicUsize,
    queue_error: AtomicI32,
    first_buffer: AtomicUsize,
    reused: AtomicBool,
    evidence: Mutex<Evidence>,
}
struct UserData {
    shared: Arc<Shared>,
    format: spa::param::video::VideoInfoRaw,
    loop_ptr: *mut pw::sys::pw_thread_loop,
}
fn wake(data: &UserData) {
    // SAFETY: This pointer is the live owning loop. Listeners are removed while
    // holding its lock, then stop joins it before the loop can be destroyed.
    unsafe {
        pw::sys::pw_thread_loop_signal(data.loop_ptr, false);
    }
}
fn failure(data: &UserData, code: u32) {
    let _ = data
        .shared
        .error
        .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    wake(data);
}

struct Lease<'a> {
    stream: &'a pw::stream::Stream,
    buffer: NonNull<pw::sys::pw_buffer>,
    shared: &'a Shared,
}
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one dequeue transferred this lease, the stream and
        // native loop remain live/locked, and all pixel borrows have ended.
        let result = unsafe {
            pw::sys::pw_stream_queue_buffer(self.stream.as_raw_ptr(), self.buffer.as_ptr())
        };
        if result < 0 {
            let _ = self.shared.queue_error.compare_exchange(
                0,
                result,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        } else {
            self.shared.queued.fetch_add(1, Ordering::Release);
        }
    }
}

fn copy_pixels(
    lease: &Lease<'_>,
    format: &spa::param::video::VideoInfoRaw,
) -> std::result::Result<Vec<u8>, u32> {
    if format.format() != spa::param::video::VideoFormat::RGB
        || usize::try_from(format.size().width).map_err(|_| 2_u32)? != WIDTH
        || usize::try_from(format.size().height).map_err(|_| 2_u32)? != HEIGHT
    {
        return Err(2);
    }
    // SAFETY: The dequeued pw_buffer owns a valid SPA buffer for this lease.
    let buffer = unsafe { lease.buffer.as_ref().buffer.as_ref() }.ok_or(3_u32)?;
    if buffer.n_datas != 1 || buffer.datas.is_null() {
        return Err(3);
    }
    // SAFETY: The native buffer declares exactly one plane; the lease prevents
    // producer access/return until this copy is complete.
    let data = unsafe { &*buffer.datas };
    let kind = spa::buffer::DataType::from_raw(data.type_);
    if (kind != spa::buffer::DataType::MemFd && kind != spa::buffer::DataType::MemPtr)
        || !spa::buffer::DataFlags::from_bits_retain(data.flags)
            .contains(spa::buffer::DataFlags::READABLE)
    {
        return Err(3);
    }
    // SAFETY: A valid SPA plane owns its chunk descriptor throughout the lease.
    let chunk = unsafe { data.chunk.as_ref() }.ok_or(3_u32)?;
    if data.data.is_null() || chunk.flags != 0 || chunk.stride <= 0 {
        return Err(3);
    }
    let stride = usize::try_from(chunk.stride).map_err(|_| 3_u32)?;
    let row = WIDTH.checked_mul(3).ok_or(3_u32)?;
    let offset = usize::try_from(chunk.offset).map_err(|_| 3_u32)?;
    let span = HEIGHT
        .checked_sub(1)
        .and_then(|h| h.checked_mul(stride))
        .and_then(|v| v.checked_add(row))
        .ok_or(3_u32)?;
    let end = offset.checked_add(span).ok_or(3_u32)?;
    if stride < row
        || span > usize::try_from(chunk.size).map_err(|_| 3_u32)?
        || end > usize::try_from(data.maxsize).map_err(|_| 3_u32)?
    {
        return Err(3);
    }
    isize::try_from(end).map_err(|_| 3_u32)?;
    let length = row.checked_mul(HEIGHT).ok_or(3_u32)?;
    let mut owned = Vec::new();
    owned.try_reserve_exact(length).map_err(|_| 4_u32)?;
    for y in 0..HEIGHT {
        let start = y
            .checked_mul(stride)
            .and_then(|v| v.checked_add(offset))
            .ok_or(3_u32)?;
        // SAFETY: Checked span and native MAP_BUFFERS lease guarantee readable
        // initialized target rows. Never copy inter-row/native allocation padding.
        owned.extend_from_slice(unsafe {
            std::slice::from_raw_parts(data.data.cast::<u8>().add(start), row)
        });
    }
    Ok(owned)
}

fn process(stream: &pw::stream::Stream, data: &mut UserData) {
    let result = catch_unwind(AssertUnwindSafe(|| -> std::result::Result<(), u32> {
        // SAFETY: Called on the non-RT loop callback with its required lock.
        let Some(buffer) = NonNull::new(unsafe { stream.dequeue_raw_buffer() }) else {
            return Ok(());
        };
        data.shared.dequeued.fetch_add(1, Ordering::Relaxed);
        let lease = Lease {
            stream,
            buffer,
            shared: &data.shared,
        };
        let address = buffer.as_ptr().addr();
        if let Err(first) = data.shared.first_buffer.compare_exchange(
            0,
            address,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) && first == address
        {
            data.shared.reused.store(true, Ordering::Release);
        }
        let pixels = copy_pixels(&lease, &data.format)?;
        let mut evidence = data.shared.evidence.lock().map_err(|_| 5_u32)?;
        let thread = std::thread::current().id();
        if let Some(previous) = evidence.callback_thread
            && previous != thread
        {
            return Err(6);
        }
        evidence.callback_thread = Some(thread);
        if evidence.first.is_none() {
            evidence.first = Some(pixels);
        }
        drop(evidence);
        drop(lease);
        data.shared.frames.fetch_add(1, Ordering::Release);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => wake(data),
        Ok(Err(code)) => failure(data, code),
        Err(_) => failure(data, 1),
    }
}

struct Stop<'a> {
    loop_: &'a pw::thread_loop::ThreadLoop,
    running: bool,
}
impl Drop for Stop<'_> {
    fn drop(&mut self) {
        if self.running {
            self.loop_.stop();
        }
    }
}
fn wait(
    loop_: &pw::thread_loop::ThreadLoop,
    shared: &Shared,
    predicate: impl Fn() -> bool,
) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(8);
    while !predicate() {
        if shared.error.load(Ordering::Acquire) != 0
            || shared.queue_error.load(Ordering::Acquire) != 0
        {
            return Err(format!(
                "PipeWire callback failure={}, queue failure={}",
                shared.error.load(Ordering::Acquire),
                shared.queue_error.load(Ordering::Acquire)
            )
            .into());
        }
        if Instant::now() >= until {
            return Err("PipeWire fixture predicate did not complete within probe budget".into());
        }
        // Retain the upstream relative-wait entry point; absolute waiting belongs
        // to P3. This atomically releases/reacquires the native loop lock.
        loop_.timed_wait(Duration::from_secs(1));
    }
    Ok(())
}

fn one_cycle(socket: &str, target: u32) -> Result<Vec<u8>> {
    let shared = Arc::try_new(Shared::default())?;
    // SAFETY: This test owns all objects, serializes associated calls with the
    // loop lock, removes listeners under it, and joins without holding it.
    let loop_ = unsafe { pw::thread_loop::ThreadLoopBox::new(Some("mfs-p0-capture"), None)? };
    let mut stop = Stop {
        loop_: &loop_,
        running: false,
    };
    let lock = loop_.lock();
    let context = pw::context::ContextBox::new(loop_.loop_(), None)?;
    let connection = UnixStream::connect(socket)?;
    // The binding consumes OwnedFd on success and failure; no duplicate close or
    // implicit connection to the host's default remote is permitted here.
    let core = context.connect_fd(connection.into(), None)?;
    let stream = pw::stream::StreamBox::new(
        &core,
        "mfs-p0-capture",
        properties! {
            "media.type"=>"Video","media.category"=>"Capture","media.role"=>"Camera","node.name"=>"mfs-p0-rust",
        },
    )?;
    let listener = stream
        .add_local_listener_with_user_data(UserData {
            shared: shared.clone(),
            format: Default::default(),
            loop_ptr: loop_.as_raw_ptr(),
        })
        .state_changed(|_, data, _, new| {
            let result = catch_unwind(AssertUnwindSafe(|| match new {
                pw::stream::StreamState::Paused | pw::stream::StreamState::Streaming => {
                    data.shared.state.store(1, Ordering::Release);
                }
                pw::stream::StreamState::Error(_) => failure(data, 7),
                _ => {}
            }));
            if result.is_err() {
                failure(data, 1);
            } else {
                wake(data);
            }
        })
        .param_changed(|_, data, id, param| {
            let result = catch_unwind(AssertUnwindSafe(|| {
                if id == spa::param::ParamType::Format.as_raw()
                    && let Some(param) = param
                    && data.format.parse(param).is_err()
                {
                    failure(data, 2);
                }
            }));
            if result.is_err() {
                failure(data, 1);
            }
        })
        .process(process)
        .register()?;
    let format = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Id,
            spa::param::video::VideoFormat::RGB
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Rectangle,
            spa::utils::Rectangle {
                width: u32::try_from(WIDTH)?,
                height: u32::try_from(HEIGHT)?
            }
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Fraction,
            spa::utils::Fraction { num: 25, denom: 1 }
        ),
    );
    let bytes = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(format),
    )
    .map_err(|e| format!("POD serialization: {e:?}"))?
    .0
    .into_inner();
    let mut params = [spa::pod::Pod::from_bytes(&bytes).ok_or("invalid serialized POD layout")?];
    stream.connect(
        spa::utils::Direction::Input,
        Some(target),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;
    // SAFETY: The loop exists and is locked; no second start is attempted. The
    // high-level start discards its error, so preserve the actual native result.
    let started = unsafe { pw::sys::pw_thread_loop_start(loop_.as_raw_ptr()) };
    if started < 0 {
        return Err(format!("pw_thread_loop_start: {started}").into());
    }
    stop.running = true;
    wait(&loop_, &shared, || {
        shared.state.load(Ordering::Acquire) != 0
    })?;
    let node = stream.node_id();
    if node == u32::MAX {
        return Err("capture stream did not bind a node".into());
    }
    let link = core.create_object::<pw::link::Link>(
        "link-factory",
        &properties! {
            "link.output.node"=>target.to_string(),"link.output.port"=>"0",
            "link.input.node"=>node.to_string(),"link.input.port"=>"0",
        },
    )?;
    wait(&loop_, &shared, || {
        shared.frames.load(Ordering::Acquire) >= 32
    })?;
    stream.disconnect()?;
    drop(link);
    drop(listener);
    drop(stream);
    drop(core);
    drop(context);
    drop(lock);
    drop(stop);
    drop(loop_);
    assert_eq!(shared.error.load(Ordering::Acquire), 0);
    assert_eq!(shared.queue_error.load(Ordering::Acquire), 0);
    assert_eq!(
        shared.dequeued.load(Ordering::Acquire),
        shared.queued.load(Ordering::Acquire)
    );
    assert!(shared.reused.load(Ordering::Acquire));
    assert_eq!(
        Arc::strong_count(&shared),
        1,
        "listener retained shared state after stop"
    );
    let mut evidence = shared
        .evidence
        .lock()
        .map_err(|_| "poisoned callback evidence")?;
    assert_ne!(evidence.callback_thread, Some(std::thread::current().id()));
    println!(
        "PipeWire frames={}, dequeued={}, queued={}, callback={:?}, owner={:?}, native buffer reused=true",
        shared.frames.load(Ordering::Acquire),
        shared.dequeued.load(Ordering::Acquire),
        shared.queued.load(Ordering::Acquire),
        evidence.callback_thread,
        std::thread::current().id()
    );
    evidence
        .first
        .take()
        .ok_or_else(|| "no independent frame".into())
}

fn validate(pixels: &[u8]) -> Result<()> {
    let bars = [
        [191, 191, 191],
        [191, 191, 0],
        [0, 191, 191],
        [0, 191, 0],
        [191, 0, 191],
        [191, 0, 0],
        [0, 0, 191],
    ];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let start = y
                .checked_mul(WIDTH)
                .and_then(|v| v.checked_add(x))
                .and_then(|v| v.checked_mul(3))
                .ok_or("pixel offset overflow")?;
            let end = start.checked_add(3).ok_or("pixel span overflow")?;
            if pixels.get(start..end) != Some(&bars[x / 4]) {
                return Err(format!(
                    "owned native RGB fixture mismatch at ({x},{y}): {:?}",
                    pixels.get(start..end)
                )
                .into());
            }
        }
    }
    Ok(())
}

#[test]
#[ignore = "explicit private PipeWire daemon/owned native source; native Linux Actions only"]
fn captures_native_rgb_pixels_returns_every_buffer_and_drains_callbacks() -> Result<()> {
    let socket = std::env::var("MFS_P0_PIPEWIRE_SOCKET")?;
    let target = std::env::var("MFS_P0_PIPEWIRE_NODE_ID")?.parse::<u32>()?;
    let expected_version = std::env::var("MFS_P0_PIPEWIRE_VERSION")?;
    pw::init();
    struct Init;
    impl Drop for Init {
        fn drop(&mut self) {
            // SAFETY: This single explicit integration test owns all PipeWire use
            // in its process. All twenty loop/object lifetimes end before this drop.
            unsafe {
                pw::deinit();
            }
        }
    }
    let init = Init;
    // SAFETY: Native library owns a permanent terminated version string.
    let actual = unsafe { CStr::from_ptr(pw::sys::pw_get_library_version()) }.to_str()?;
    assert_eq!(actual, expected_version);
    let mut final_frame = None;
    for cycle in 1..=20 {
        println!("PipeWire lifecycle cycle {cycle}/20, native library={actual}");
        let pixels = one_cycle(&socket, target)?;
        validate(&pixels)?;
        final_frame = Some(pixels);
    }
    drop(init);
    let pixels = final_frame.ok_or("no captured frame")?;
    validate(&pixels)?;
    std::thread::spawn(move || {
        assert_eq!(pixels.len(), WIDTH * HEIGHT * 3);
        validate(&pixels).unwrap();
    })
    .join()
    .unwrap();
    Ok(())
}
