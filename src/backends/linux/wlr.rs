// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private Wlr migration. Captures retain one shared client and independent
//! native buffer/result state; early-close and complete fault parity remain P1.
//! Callback-internal roundtrips and frame lifetime through conversion stay U.
use crate::platform::linux::wayland::{Channel, Error, callback_roundtrip, same_source_path};
use memmap2::{MmapMut, MmapOptions};
use std::{
    fs::File,
    marker::PhantomData,
    mem::ManuallyDrop,
    os::fd::{AsFd, AsRawFd, OwnedFd},
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicI32, AtomicU64, Ordering},
    },
    thread::ThreadId,
};
use wayland_backend::protocol::Message;
use wayland_client::{
    Connection, Proxy, WEnum,
    backend::{Backend, InvalidId, ObjectData, ObjectId},
    protocol::{wl_buffer, wl_display, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool},
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1 as keyboard;
use wayland_protocols_wlr::{
    screencopy::v1::client::{
        zwlr_screencopy_frame_v1 as frame, zwlr_screencopy_manager_v1 as manager,
    },
    virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1 as pointer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conversion {
    Bgra,
    Rgba,
    Rgb,
    BgrCopy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    format: u32,
    width: u32,
    height: u32,
    stride: u32,
}
impl Layout {
    fn size(self) -> Result<usize, Error> {
        if self.width == 0
            || self.height == 0
            || self.stride == 0
            || self.width > i32::MAX as u32
            || self.height > i32::MAX as u32
            || self.stride > i32::MAX as u32
        {
            return Err(Error::new(
                "Wlr native dimensions/stride",
                rustix::io::Errno::INVAL.raw_os_error(),
            ));
        }
        let size = self.height.checked_mul(self.stride).ok_or(Error::new(
            "Wlr mapping size",
            rustix::io::Errno::OVERFLOW.raw_os_error(),
        ))?;
        if size > i32::MAX as u32 {
            return Err(Error::new(
                "Wlr pool size narrowing",
                rustix::io::Errno::OVERFLOW.raw_os_error(),
            ));
        }
        usize::try_from(size).map_err(|_| {
            Error::new(
                "Wlr mapping size narrowing",
                rustix::io::Errno::OVERFLOW.raw_os_error(),
            )
        })
    }
    fn conversion(self) -> Result<(Conversion, usize), Error> {
        match wl_shm::Format::try_from(self.format) {
            Ok(wl_shm::Format::Xbgr8888 | wl_shm::Format::Abgr8888) => Ok((Conversion::Rgba, 4)),
            Ok(wl_shm::Format::Xrgb8888 | wl_shm::Format::Argb8888) => Ok((Conversion::Bgra, 4)),
            Ok(wl_shm::Format::Rgb888) => Ok((Conversion::BgrCopy, 3)),
            Ok(wl_shm::Format::Bgr888) => Ok((Conversion::Rgb, 3)),
            _ => Err(Error::new(
                "source Wlr unsupported conversion",
                rustix::io::Errno::INVAL.raw_os_error(),
            )),
        }
    }
}
#[derive(Default)]
struct Counts {
    data_created: AtomicU64,
    data_dropped: AtomicU64,
    destroyed: AtomicU64,
    recursive: AtomicU64,
    outer: AtomicU64,
    copied: AtomicU64,
    frames: AtomicU64,
    source_fd_zero_retained: AtomicU64,
    cleanup: AtomicI32,
}
impl Counts {
    fn increment(&self, value: &AtomicU64) {
        // Preserve an allocation-independent overflow error, never wrap.
        if value
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .is_err()
        {
            cleanup(self, rustix::io::Errno::OVERFLOW.raw_os_error());
        }
    }
}
fn cleanup(counts: &Counts, code: i32) {
    let _ = counts
        .cleanup
        .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
}
struct Object<T: Proxy> {
    proxy: T,
    destroy: fn(&T) -> Result<(), InvalidId>,
    counts: Arc<Counts>,
}
impl<T: Proxy> Drop for Object<T> {
    fn drop(&mut self) {
        match catch_unwind(AssertUnwindSafe(|| (self.destroy)(&self.proxy))) {
            Ok(Ok(())) => {}
            Ok(Err(_)) => cleanup(&self.counts, -1),
            Err(_) => cleanup(&self.counts, -2),
        }
    }
}
type Owned<T> = Arc<Object<T>>;
fn own<T: Proxy>(
    proxy: T,
    destroy: fn(&T) -> Result<(), InvalidId>,
    counts: &Arc<Counts>,
) -> Result<Owned<T>, Error> {
    // On allocation failure Object is dropped, releasing the new proxy once.
    Arc::try_new(Object {
        proxy,
        destroy,
        counts: counts.clone(),
    })
    .map_err(|_| Error::allocation())
}
fn local<T: Proxy>(proxy: &T) -> Result<(), InvalidId> {
    proxy
        .backend()
        .upgrade()
        .ok_or(InvalidId)?
        .destroy_object(&proxy.id())
}
#[derive(Default)]
struct Globals {
    registry: Option<Owned<wl_registry::WlRegistry>>,
    output: Option<Owned<wl_output::WlOutput>>,
    shm: Option<Owned<wl_shm::WlShm>>,
    seat: Option<Owned<wl_seat::WlSeat>>,
    manager: Option<Owned<manager::ZwlrScreencopyManagerV1>>,
    pointer: Option<Owned<pointer::ZwlrVirtualPointerManagerV1>>,
    keyboard: Option<Owned<keyboard::ZwpVirtualKeyboardManagerV1>>,
}
struct Buffer {
    // Mapping is independent of the Wayland proxy owner; close_buffer retains
    // it until after the original destruction roundtrip, including failure.
    // Explicit drop keeps source unmap-before-close after proxy/pool release;
    // its observed FD-zero residual is retained separately below.
    map: ManuallyDrop<MmapMut>,
    file: ManuallyDrop<File>,
    pool: Option<Owned<wl_shm_pool::WlShmPool>>,
    proxy: Option<Owned<wl_buffer::WlBuffer>>,
    counts: Arc<Counts>,
}
impl Drop for Buffer {
    fn drop(&mut self) {
        drop(self.proxy.take());
        drop(self.pool.take());
        if self.file.as_raw_fd() == 0 {
            // U's fixed MemfdBuffer destructor returns immediately for !fd_.
            // Preserve BOTH native residuals, as observed by the isolated
            // original FD-zero probe. This is not successful product cleanup;
            // P3 corrects ownership after U/P freeze. A fixture may reclaim
            // them only after buffer/proxy/callback/native access has ended.
            self.counts.increment(&self.counts.source_fd_zero_retained);
            return;
        }
        // SAFETY: These are the sole initialized owners. Protocol owners have
        // ended above, no pixel borrow escapes, and each manual owner is dropped
        // exactly once here, in source unmap-before-close order. ManuallyDrop's
        // field destruction does not release either owner a second time.
        unsafe {
            ManuallyDrop::drop(&mut self.map);
            ManuallyDrop::drop(&mut self.file);
        }
    }
}
fn buffer_name() -> Result<[u8; 53], Error> {
    let mut random = [0u8; 16];
    let mut offset = 0;
    while offset < random.len() {
        match rustix::rand::getrandom(&mut random[offset..], rustix::rand::GetRandomFlags::empty())
        {
            Ok(0) => return Err(Error::new("Wlr UUID empty random read", -1)),
            Ok(read) => {
                offset = offset.checked_add(read).ok_or(Error::new(
                    "Wlr UUID random length",
                    rustix::io::Errno::OVERFLOW.raw_os_error(),
                ))?
            }
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(Error::new("Wlr UUID random", error.raw_os_error())),
        }
    }
    random[6] = (random[6] & 15) | 64;
    random[8] = (random[8] & 63) | 128;
    let mut name = [0u8; 53];
    name[..16].copy_from_slice(b"maafw-wl-buffer-");
    let digits = b"0123456789abcdef";
    let mut cursor = 16;
    for (index, byte) in random.into_iter().enumerate() {
        if [4, 6, 8, 10].contains(&index) {
            name[cursor] = b'-';
            cursor += 1;
        }
        name[cursor] = digits[usize::from(byte >> 4)];
        name[cursor + 1] = digits[usize::from(byte & 15)];
        cursor += 2;
    }
    Ok(name)
}
#[derive(Default)]
struct State {
    globals: Globals,
    output: Option<Owned<wl_output::WlOutput>>,
    shm: Option<Owned<wl_shm::WlShm>>,
    manager: Option<Owned<manager::ZwlrScreencopyManagerV1>>,
    buffer: Option<Buffer>,
    layout: Option<Layout>,
    waiting: bool,
    successful: bool,
    connected: bool,
    ready_time: Option<(u32, u32, u32)>,
    flags: Option<u32>,
}
struct Context {
    owner: ThreadId,
    state: Mutex<State>,
    fault: AtomicI32,
    buffer_error: AtomicI32,
    counts: Arc<Counts>,
}
impl Context {
    fn state(&self) -> Result<MutexGuard<'_, State>, Error> {
        self.state
            .lock()
            .map_err(|_| Error::new("Wlr state poisoned", -2))
    }
    fn fail(&self, code: i32) {
        let _ = self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
    fn check(&self) -> Result<(), Error> {
        let code = self.fault.load(Ordering::Acquire);
        if code == 0 {
            Ok(())
        } else {
            Err(Error::new("Wlr callback", code))
        }
    }
}
#[derive(Clone, Copy)]
enum Kind {
    Registry,
    Frame,
    Ignore,
}
struct Data {
    context: Weak<Context>,
    counts: Arc<Counts>,
    kind: Kind,
}
impl Drop for Data {
    fn drop(&mut self) {
        self.counts.increment(&self.counts.data_dropped);
    }
}
fn data(context: &Arc<Context>, kind: Kind) -> Result<Arc<dyn ObjectData>, Error> {
    let value = Data {
        context: Arc::downgrade(context),
        counts: context.counts.clone(),
        kind,
    };
    context.counts.increment(&context.counts.data_created);
    let result = Arc::try_new(value).map_err(|_| Error::allocation())?;
    Ok(result)
}
fn bind<T: Proxy>(
    registry: &wl_registry::WlRegistry,
    name: u32,
    version: u32,
    ctx: &Arc<Context>,
) -> Result<T, Error> {
    // Retain advertised versions, never silently clamp as the P0 prototype did.
    // Reject an interface beyond this binding's layout instead of unsafe events.
    if version == 0 || version > T::interface().version {
        return Err(Error::new(
            "Wlr advertised interface beyond binding",
            rustix::io::Errno::INVAL.raw_os_error(),
        ));
    }
    registry
        .send_constructor(
            wl_registry::Request::Bind {
                name,
                id: (T::interface(), version),
            },
            data(ctx, Kind::Ignore)?,
        )
        .map_err(|_| Error::new("wl_registry_bind", -1))
}
fn global(
    ctx: &Arc<Context>,
    registry: &wl_registry::WlRegistry,
    name: u32,
    interface: &str,
    version: u32,
) -> Result<(), Error> {
    // Constructors/marshalling do not dispatch; native roundtrip never holds
    // this lock. Each replacement drops its old owner only after assignment.
    let mut state = ctx.state()?;
    let g = &mut state.globals;
    match interface {
        "wl_output" => {
            g.output = Some(own(
                bind(registry, name, version, ctx)?,
                local,
                &ctx.counts,
            )?)
        }
        "wl_shm" => {
            g.shm = Some(own(
                bind(registry, name, version, ctx)?,
                local,
                &ctx.counts,
            )?)
        }
        "wl_seat" => {
            g.seat = Some(own(
                bind(registry, name, version, ctx)?,
                local,
                &ctx.counts,
            )?)
        }
        "zwlr_screencopy_manager_v1" => {
            g.manager = Some(own(
                bind::<manager::ZwlrScreencopyManagerV1>(registry, name, version, ctx)?,
                |p| p.send_request(manager::Request::Destroy {}),
                &ctx.counts,
            )?)
        }
        "zwlr_virtual_pointer_manager_v1" => {
            g.pointer = Some(own(
                bind::<pointer::ZwlrVirtualPointerManagerV1>(registry, name, version, ctx)?,
                |p| p.send_request(pointer::Request::Destroy {}),
                &ctx.counts,
            )?)
        }
        "zwp_virtual_keyboard_manager_v1" => {
            g.keyboard = Some(own(
                bind(registry, name, version, ctx)?,
                local,
                &ctx.counts,
            )?)
        }
        _ => {}
    }
    Ok(())
}
fn recursive(
    ctx: &Context,
    backend: &Backend,
    frame: &frame::ZwlrScreencopyFrameV1,
) -> Result<(), Error> {
    ctx.counts.increment(&ctx.counts.recursive);
    // SAFETY: Only called by the synchronous Frame ObjectData handler below;
    // owner checked, backend TLS active, no State reference/guard survives here.
    unsafe { callback_roundtrip(backend, frame) }.map(|_| ())
}
fn prepare_buffer(
    ctx: &Arc<Context>,
    backend: &Backend,
    frame: &frame::ZwlrScreencopyFrameV1,
    layout: Layout,
) -> Result<(), Error> {
    let same = {
        let s = ctx.state()?;
        s.layout == Some(layout) && s.buffer.as_ref().is_some_and(|b| b.proxy.is_some())
    };
    if !same {
        let old = {
            let mut s = ctx.state()?;
            s.buffer.as_mut().map(|b| (b.proxy.take(), b.pool.take()))
        };
        if let Some(old) = old {
            drop(old.0);
            drop(old.1);
            // U logs a failed old-buffer close and continues new allocation.
            if recursive(ctx, backend, frame).is_ok() {
                drop(ctx.state()?.buffer.take());
            } else {
                cleanup(&ctx.counts, -3);
            }
        }
        let size = layout.size()?;
        // ftruncate provides defined zero-filled backing, including native
        // padding; no Rust references exist while the compositor writes it.
        let name = buffer_name()?;
        let name = std::ffi::CStr::from_bytes_with_nul(&name)
            .map_err(|_| Error::new("Wlr UUID name", rustix::io::Errno::INVAL.raw_os_error()))?;
        let file = File::from(
            rustix::fs::memfd_create(name, rustix::fs::MemfdFlags::empty())
                .map_err(|e| Error::new("Wlr memfd_create", e.raw_os_error()))?,
        );
        rustix::fs::ftruncate(
            &file,
            u64::try_from(size).map_err(|_| {
                Error::new("Wlr file size", rustix::io::Errno::OVERFLOW.raw_os_error())
            })?,
        )
        .map_err(|e| Error::new("Wlr ftruncate", e.raw_os_error()))?;
        // SAFETY: Owned correctly sized shared file. Server only writes a
        // requested frame; Rust borrows pixels after Ready and before next Copy.
        let map = unsafe { MmapOptions::new().len(size).map_mut(&file) }
            .map_err(|e| Error::new("Wlr mmap", e.raw_os_error().unwrap_or(-1)))?;
        let shm = {
            let mut s = ctx.state()?;
            s.buffer = Some(Buffer {
                map: ManuallyDrop::new(map),
                file: ManuallyDrop::new(file),
                pool: None,
                proxy: None,
                counts: ctx.counts.clone(),
            });
            s.layout = Some(layout);
            s.shm.clone().ok_or(Error::new(
                "Wlr retained shm",
                rustix::io::Errno::INVAL.raw_os_error(),
            ))?
        };
        let pool = {
            let s = ctx.state()?;
            shm.proxy
                .send_constructor::<wl_shm_pool::WlShmPool>(
                    wl_shm::Request::CreatePool {
                        fd: s.buffer.as_ref().unwrap().file.as_fd(),
                        size: i32::try_from(size).unwrap(),
                    },
                    data(ctx, Kind::Ignore)?,
                )
                .map_err(|_| Error::new("wl_shm_create_pool", -1))?
        };
        let pool = own(
            pool,
            |p| p.send_request(wl_shm_pool::Request::Destroy {}),
            &ctx.counts,
        )?;
        ctx.state()?.buffer.as_mut().unwrap().pool = Some(pool.clone());
        let buffer = pool
            .proxy
            .send_constructor::<wl_buffer::WlBuffer>(
                wl_shm_pool::Request::CreateBuffer {
                    offset: 0,
                    width: i32::try_from(layout.width).unwrap(),
                    height: i32::try_from(layout.height).unwrap(),
                    stride: i32::try_from(layout.stride).unwrap(),
                    format: WEnum::from(layout.format),
                },
                data(ctx, Kind::Ignore)?,
            )
            .map_err(|_| Error::new("wl_shm_pool_create_buffer", -1))?;
        ctx.state()?.buffer.as_mut().unwrap().proxy = Some(own(
            buffer,
            |p| p.send_request(wl_buffer::Request::Destroy {}),
            &ctx.counts,
        )?);
        drop(pool);
        recursive(ctx, backend, frame)?;
    }
    let buffer = ctx
        .state()?
        .buffer
        .as_ref()
        .and_then(|b| b.proxy.clone())
        .ok_or(Error::new(
            "Wlr copy buffer",
            rustix::io::Errno::INVAL.raw_os_error(),
        ))?;
    frame
        .send_request(frame::Request::Copy {
            buffer: buffer.proxy.clone(),
        })
        .map_err(|_| Error::new("zwlr_screencopy_frame_copy", -1))
}
impl ObjectData for Data {
    fn event(
        self: Arc<Self>,
        backend: &Backend,
        msg: Message<ObjectId, OwnedFd>,
    ) -> Option<Arc<dyn ObjectData>> {
        let ctx = self.context.upgrade()?;
        if std::thread::current().id() != ctx.owner {
            ctx.fail(-4);
            return None;
        }
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), Error> {
            let conn = Connection::from_backend(backend.clone());
            match self.kind {
                Kind::Registry => {
                    let (registry, event) = wl_registry::WlRegistry::parse_event(&conn, msg)
                        .map_err(|_| Error::new("Wlr registry event", -5))?;
                    if let wl_registry::Event::Global {
                        name,
                        interface,
                        version,
                    } = event
                    {
                        global(&ctx, &registry, name, &interface, version)?;
                    }
                    // Original GlobalRemove deliberately does nothing.
                }
                Kind::Frame => {
                    let (frame, event) = frame::ZwlrScreencopyFrameV1::parse_event(&conn, msg)
                        .map_err(|_| Error::new("Wlr frame event", -5))?;
                    match event {
                        frame::Event::Buffer {
                            format,
                            width,
                            height,
                            stride,
                        } => {
                            if let Err(error) = prepare_buffer(
                                &ctx,
                                backend,
                                &frame,
                                Layout {
                                    format: format.into(),
                                    width,
                                    height,
                                    stride,
                                },
                            ) {
                                // Source logs create_buffer failure, sends no
                                // Copy and leaves waiting unchanged. Do not add
                                // a P3 early-error wake or timeout here.
                                let _ = ctx.buffer_error.compare_exchange(
                                    0,
                                    error.code,
                                    Ordering::AcqRel,
                                    Ordering::Acquire,
                                );
                            }
                        }
                        frame::Event::Ready {
                            tv_sec_hi,
                            tv_sec_lo,
                            tv_nsec,
                        } => {
                            let mut s = ctx.state()?;
                            s.waiting = false;
                            s.successful = true;
                            s.ready_time = Some((tv_sec_hi, tv_sec_lo, tv_nsec));
                        }
                        frame::Event::Failed => {
                            let mut s = ctx.state()?;
                            s.waiting = false;
                            s.successful = false;
                        }
                        frame::Event::Flags { flags } => ctx.state()?.flags = Some(flags.into()),
                        // Damage, BufferDone and LinuxDmabuf are only logged in U.
                        _ => {}
                    }
                }
                Kind::Ignore => {}
            }
            Ok(())
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => ctx.fail(e.code),
            Err(_) => ctx.fail(-2),
        };
        None
    }
    fn destroyed(&self, _id: ObjectId) {
        self.counts.increment(&self.counts.destroyed);
    }
}

#[derive(Clone)]
pub(crate) struct Sample {
    pub width: u32,
    pub height: u32,
    pub native_stride: u32,
    pub conversion: Conversion,
    pub ready_time: Option<(u32, u32, u32)>,
    pub flags: Option<u32>,
    bytes: Arc<Vec<u8>>,
}
impl Sample {
    pub fn pixels(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Debug)]
pub(crate) struct Evidence {
    pub data_created: u64,
    pub data_dropped: u64,
    pub destroyed: u64,
    pub recursive: u64,
    pub outer: u64,
    pub copied: u64,
    pub frames: u64,
    pub source_fd_zero_retained: u64,
    pub cleanup: i32,
    pub fault: i32,
    pub buffer_error: i32,
}
// The client context receives registry events; each capture has another Context
// for its own frame callbacks. Only the scalar evidence ledger is shared. No
// state guard or exclusive client borrow survives native dispatch.
pub(crate) struct Client {
    context: Arc<Context>,
    channel: Option<Channel>,
    socket_path: PathBuf,
}
impl Client {
    pub fn into_shared(self) -> Result<Arc<Self>, Error> {
        // Standard shared ownership only. Channel's Rc marker keeps Client
        // and every Arc<Client> on this native channel; no Send/Sync is asserted.
        Arc::try_new(self).map_err(|_| Error::allocation())
    }
    pub fn new() -> Result<Self, Error> {
        let counts = Arc::try_new(Counts::default()).map_err(|_| Error::allocation())?;
        let context = Arc::try_new(Context {
            owner: std::thread::current().id(),
            state: Mutex::new(State::default()),
            fault: AtomicI32::new(0),
            buffer_error: AtomicI32::new(0),
            counts,
        })
        .map_err(|_| Error::allocation())?;
        Ok(Self {
            context,
            channel: None,
            socket_path: PathBuf::new(),
        })
    }
    pub fn connect(&mut self, path: &Path) -> Result<bool, Error> {
        if !same_source_path(&self.socket_path, path) {
            self.close();
            self.socket_path.clear();
            self.socket_path
                .try_reserve_exact(path.as_os_str().len())
                .map_err(|_| Error::allocation())?;
            self.socket_path.push(path);
        }
        if self.channel.is_some() {
            // Same live path is the source's no-op, including no roundtrip.
            return self.connected();
        }
        self.channel = Some(Channel::connect(&self.socket_path)?);
        if let Err(error) = self.bind_protocol() {
            self.close();
            return Err(error);
        }
        self.context.state()?.connected = true;
        Ok(true)
    }
    pub fn connected(&self) -> Result<bool, Error> {
        Ok(self.context.state()?.connected)
    }
    fn bind_protocol(&self) -> Result<(), Error> {
        // SAFETY: All connection clones/proxies are internal. Captures retain
        // Arc<Client> until their copied proxies/buffers have ended. connect
        // and close need &mut Client, unavailable while any capture retains it.
        // Closing/reconnecting with live snapshots is not yet migrated.
        let conn = unsafe { self.channel.as_ref().unwrap().connection() };
        let registry = conn
            .display()
            .send_constructor::<wl_registry::WlRegistry>(
                wl_display::Request::GetRegistry {},
                data(&self.context, Kind::Registry)?,
            )
            .map_err(|_| Error::new("wl_display_get_registry", -1))?;
        self.context.state()?.globals.registry = Some(own(registry, local, &self.context.counts)?);
        self.roundtrip()?;
        self.context.check()?;
        {
            let s = self.context.state()?;
            // Exact original core prerequisite is &&, not ||.
            if (s.globals.output.is_none() && s.globals.shm.is_none())
                || s.globals.manager.is_none()
                || s.globals.keyboard.is_none()
                || s.globals.pointer.is_none()
            {
                return Err(Error::new(
                    "Wlr original client prerequisites",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ));
            }
        }
        Ok(())
    }
    fn roundtrip(&self) -> Result<(), Error> {
        self.context.counts.increment(&self.context.counts.outer);
        // SAFETY: Temporary immutable connection borrow does not escape Client.
        unsafe {
            self.channel
                .as_ref()
                .ok_or(Error::new(
                    "closed Wlr channel",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ))?
                .connection()
        }
        .roundtrip()
        .map(|_| ())
        .map_err(|_| Error::new("wl_display_roundtrip", -1))
    }
    pub fn close(&mut self) {
        {
            let mut s = self
                .context
                .state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            s.connected = false;
            // Original client order; no capture can retain a proxy while this
            // unique client borrow exists. Registry callback data ends before
            // the native display/backend queue owner below.
            let g = &mut s.globals;
            g.pointer.take();
            g.keyboard.take();
            g.manager.take();
            g.seat.take();
            g.shm.take();
            g.output.take();
            g.registry.take();
        }
        drop(self.channel.take());
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.close();
    }
}
pub(crate) struct Capture {
    context: Arc<Context>,
    client: Option<Arc<Client>>,
    _owner: PhantomData<Rc<()>>,
}
impl Capture {
    pub fn connect(path: &Path) -> Result<Self, Error> {
        let mut client = Client::new()?;
        client.connect(path)?;
        Self::from_client(client.into_shared()?)
    }
    pub fn from_client(client: Arc<Client>) -> Result<Self, Error> {
        let state = {
            let s = client.context.state()?;
            State {
                output: s.globals.output.clone(),
                shm: s.globals.shm.clone(),
                manager: s.globals.manager.clone(),
                connected: true,
                ..State::default()
            }
        };
        let context = Arc::try_new(Context {
            owner: std::thread::current().id(),
            state: Mutex::new(state),
            fault: AtomicI32::new(0),
            buffer_error: AtomicI32::new(0),
            counts: client.context.counts.clone(),
        })
        .map_err(|_| Error::allocation())?;
        Ok(Self {
            context,
            client: Some(client),
            _owner: PhantomData,
        })
    }
    fn roundtrip(&self) -> Result<(), Error> {
        let client = self.client.as_ref().ok_or(Error::new(
            "closed Wlr client",
            rustix::io::Errno::INVAL.raw_os_error(),
        ))?;
        client.roundtrip()?;
        client.context.check()
    }
    pub fn inactive(&mut self) {} // Original inherited ScreencapBase no-op.
    pub fn capture_with<T>(
        &mut self,
        convert: impl FnOnce(Sample) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let (manager, output) = {
            let s = self.context.state()?;
            if !s.connected
                || !self
                    .client
                    .as_ref()
                    .ok_or(Error::new("closed Wlr source", -1))?
                    .connected()?
            {
                return Err(Error::new(
                    "closed Wlr source",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ));
            }
            (
                s.manager.clone().ok_or(Error::new(
                    "Wlr manager",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ))?,
                s.output.clone().ok_or(Error::new(
                    "Wlr output",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ))?,
            )
        };
        let frame = manager
            .proxy
            .send_constructor::<frame::ZwlrScreencopyFrameV1>(
                manager::Request::CaptureOutput {
                    overlay_cursor: 0,
                    output: output.proxy.clone(),
                },
                data(&self.context, Kind::Frame)?,
            )
            .map_err(|_| Error::new("zwlr_capture_output", -1))?;
        let frame = own(
            frame,
            |p| p.send_request(frame::Request::Destroy {}),
            &self.context.counts,
        )?;
        {
            let mut s = self.context.state()?;
            s.waiting = true;
            s.successful = false;
            s.ready_time = None;
            s.flags = None;
        }
        loop {
            if !self.context.state()?.waiting {
                break;
            }
            self.roundtrip()?;
            self.context.check()?;
        }
        let sample = {
            let s = self.context.state()?;
            if !s.successful {
                return Err(Error::new("native Wlr frame failed", -6));
            }
            let layout = s.layout.ok_or(Error::new(
                "Wlr missing layout",
                rustix::io::Errno::INVAL.raw_os_error(),
            ))?;
            let (conversion, channels) = layout.conversion()?;
            let row = usize::try_from(layout.width)
                .unwrap()
                .checked_mul(channels)
                .ok_or(Error::new(
                    "Wlr source Mat row",
                    rustix::io::Errno::OVERFLOW.raw_os_error(),
                ))?;
            if row > usize::try_from(layout.stride).unwrap() {
                return Err(Error::new(
                    "Wlr short native stride",
                    rustix::io::Errno::INVAL.raw_os_error(),
                ));
            }
            let size = row
                .checked_mul(usize::try_from(layout.height).unwrap())
                .ok_or(Error::new(
                    "Wlr source Mat span",
                    rustix::io::Errno::OVERFLOW.raw_os_error(),
                ))?;
            let buffer = s.buffer.as_ref().ok_or(Error::new(
                "Wlr missing mapping",
                rustix::io::Errno::INVAL.raw_os_error(),
            ))?;
            let pixels = buffer.map.get(..size).ok_or(Error::new(
                "Wlr short mapping",
                rustix::io::Errno::INVAL.raw_os_error(),
            ))?;
            // Preserve original Mat's implicit packed step; do not silently fix
            // stride/inversion in U. This owned copy adds work before conversion.
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(size)
                .map_err(|_| Error::allocation())?;
            bytes.extend_from_slice(pixels);
            let bytes = Arc::try_new(bytes).map_err(|_| Error::allocation())?;
            self.context.counts.increment(&self.context.counts.copied);
            self.context.counts.increment(&self.context.counts.frames);
            Sample {
                width: layout.width,
                height: layout.height,
                native_stride: layout.stride,
                conversion,
                ready_time: s.ready_time,
                flags: s.flags,
                bytes,
            }
        };
        // Native frame proxy stays alive THROUGH actual caller conversion, as
        // the original function-local unique_ptr does. No native lease escapes.
        let result = convert(sample);
        drop(frame);
        result
    }
    pub fn close(&mut self) {
        {
            // Rust unwinding leaves initialized Option owners even if a
            // callback poisoned the lock. Preserve its primary fault while
            // still releasing every native owner before the display closes.
            let mut s = self
                .context
                .state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            s.connected = false;
            if let Some(mut buffer) = s.buffer.take() {
                drop(buffer.proxy.take());
                drop(buffer.pool.take());
                drop(buffer);
            }
            // Capture snapshots end before the last shared client is released.
            s.output.take();
            s.shm.take();
            s.manager.take();
        }
        drop(self.client.take());
    }
    pub fn evidence(&self) -> Evidence {
        let c = &self.context.counts;
        Evidence {
            data_created: c.data_created.load(Ordering::Acquire),
            data_dropped: c.data_dropped.load(Ordering::Acquire),
            destroyed: c.destroyed.load(Ordering::Acquire),
            recursive: c.recursive.load(Ordering::Acquire),
            outer: c.outer.load(Ordering::Acquire),
            copied: c.copied.load(Ordering::Acquire),
            frames: c.frames.load(Ordering::Acquire),
            source_fd_zero_retained: c.source_fd_zero_retained.load(Ordering::Acquire),
            cleanup: c.cleanup.load(Ordering::Acquire),
            fault: self.context.fault.load(Ordering::Acquire),
            buffer_error: self.context.buffer_error.load(Ordering::Acquire),
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[path = "wlr_tests.rs"]
mod tests;
