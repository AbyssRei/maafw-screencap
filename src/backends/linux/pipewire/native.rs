// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! First native driver slice: daemon/unsafe supplied-FD source connection and
//! requested-frame copies from readable SHM/owned MemFd/DMA mappings. General FD
//! ownership, native incoming DMA/NULL-data streams and full P1 remain unfinished.
use super::{
    Driver, Format,
    buffer::{Chunk, CopyError, CopyEvidence, Layout, RawCache, RawFrame},
    wait_until,
};
use crate::platform::linux::pipewire::{
    DmaBufMap, DmaSyncEvidence, DmaSyncSnapshot, MemFdMap, NativeError, Resources, spa, sys,
};
use std::{
    cell::{Cell, UnsafeCell},
    ffi::{c_char, c_void},
    mem,
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
#[path = "native_pod.rs"]
mod pod;

struct State {
    width: u32,
    height: u32,
    format: Format,
    wanted: bool,
    cache: RawCache,
}
impl Default for State {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            format: Format::Unknown,
            wanted: false,
            cache: RawCache::default(),
        }
    }
}

struct Callbacks {
    loop_: Cell<*mut sys::pw_thread_loop>,
    stream: Cell<*mut sys::pw_stream>,
    connected: AtomicBool,
    // Consumer accesses this only while holding the actual native loop lock.
    // Callbacks run under that same lock. No state reference survives a native
    // timed_wait, which releases/reacquires the lock while callbacks can mutate.
    state: UnsafeCell<State>,
    fault: AtomicU32,
    queue_error: AtomicI32,
    mapping_error: AtomicI32,
    unmap_error: AtomicI32,
    owned_maps: AtomicU64,
    unmap_calls: AtomicU64,
    dma: DmaSyncEvidence,
    dequeued: Arc<AtomicU64>,
    returned: AtomicU64,
    copies: AtomicU64,
    allocated: AtomicU64,
}
impl Callbacks {
    fn fail(&self, code: u32) {
        let _ = self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
    fn add(&self, counter: &AtomicU64, amount: u64) {
        if counter
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                v.checked_add(amount)
            })
            .is_err()
        {
            self.fail(2);
        }
    }
    fn boundary(&self, operation: impl FnOnce()) {
        if catch_unwind(AssertUnwindSafe(operation)).is_err() {
            // No exception crosses native C. This scalar path cannot allocate;
            // source error callbacks do not signal, so U still uses its timeout.
            self.fail(1);
        }
    }
}

static CORE_EVENTS: sys::pw_core_events = sys::pw_core_events {
    version: sys::PW_VERSION_CORE_EVENTS,
    error: Some(on_core_error),
    // SAFETY: Unselected C callbacks are nullable function pointers.
    ..unsafe { mem::zeroed() }
};
static STREAM_EVENTS: sys::pw_stream_events = sys::pw_stream_events {
    version: sys::PW_VERSION_STREAM_EVENTS,
    state_changed: Some(on_state),
    param_changed: Some(on_param),
    process: Some(on_process),
    // SAFETY: Same nullable event-record invariant.
    ..unsafe { mem::zeroed() }
};

unsafe extern "C" fn on_core_error(
    data: *mut c_void,
    id: u32,
    _seq: i32,
    result: i32,
    _message: *const c_char,
) {
    // SAFETY: Registration retains this stable owner until stop/unregister and
    // native teardown finish. Atomic state is accessible from either channel.
    let callbacks = unsafe { &*data.cast::<Callbacks>() };
    // Keep source's exact core-error predicate;
    // do not turn unrelated server errors into a new termination policy.
    if id == pipewire::core::PW_ID_CORE && result == -rustix::io::Errno::PIPE.raw_os_error() {
        callbacks.connected.store(false, Ordering::Release);
    }
}
unsafe extern "C" fn on_state(
    data: *mut c_void,
    _old: sys::pw_stream_state,
    new: sys::pw_stream_state,
    _error: *const c_char,
) {
    // SAFETY: Same stable registration lifetime, including synchronous native
    // destruction notifications before callback storage is freed.
    let callbacks = unsafe { &*data.cast::<Callbacks>() };
    if new == sys::pw_stream_state_PW_STREAM_STATE_ERROR
        || new == sys::pw_stream_state_PW_STREAM_STATE_UNCONNECTED
    {
        callbacks.connected.store(false, Ordering::Release);
    }
}
unsafe extern "C" fn on_param(data: *mut c_void, id: u32, param: *const spa::sys::spa_pod) {
    if param.is_null() || id != spa::sys::SPA_PARAM_Format {
        return;
    }
    // SAFETY: Stable callback owner, dispatched with actual loop lock held.
    let callbacks = unsafe { &*data.cast::<Callbacks>() };
    callbacks.boundary(|| {
        // SAFETY: PipeWire retains this initialized POD throughout the callback.
        let Some((width, height, format)) = (unsafe { pod::parse(param) }) else {
            return;
        };
        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            callbacks.fail(3);
            return;
        }
        // SAFETY: Actual loop lock excludes consumer/other callbacks; this
        // reference ends before any native wait or reentrant resource operation.
        {
            let state = unsafe { &mut *callbacks.state.get() };
            state.width = width;
            state.height = height;
            state.format = format;
        }
        // SAFETY: Owning loop is live and native signal only wakes its waiter.
        unsafe {
            sys::pw_thread_loop_signal(callbacks.loop_.get(), false);
        }
    });
}

struct Lease<'a> {
    buffer: *mut sys::pw_buffer,
    callbacks: &'a Callbacks,
}
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        // SAFETY: One native dequeue transferred this buffer; this guard returns
        // it once, on the locked callback channel after all row references end.
        let code = unsafe { sys::pw_stream_queue_buffer(self.callbacks.stream.get(), self.buffer) };
        self.callbacks.add(&self.callbacks.returned, 1);
        if code < 0 {
            let _ = self.callbacks.queue_error.compare_exchange(
                0,
                code,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}
unsafe extern "C" fn on_process(data: *mut c_void) {
    // SAFETY: Stable owner and actual loop lock; the process callback is non-RT.
    let callbacks = unsafe { &*data.cast::<Callbacks>() };
    callbacks.boundary(|| {
        let mut latest = None;
        loop {
            // SAFETY: Live stream is accessed on its locked native loop channel.
            let buffer = unsafe { sys::pw_stream_dequeue_buffer(callbacks.stream.get()) };
            if buffer.is_null() {
                break;
            }
            callbacks.add(&callbacks.dequeued, 1);
            // Drop returns each predecessor immediately, retaining only latest.
            latest = Some(Lease { buffer, callbacks });
        }
        let Some(lease) = latest else {
            return;
        };
        // SAFETY: A dequeued lease retains its SPA buffer until this scope ends.
        let Some(buffer) = (unsafe { (*lease.buffer).buffer.as_ref() }) else {
            return;
        };
        if buffer.n_datas < 1 || buffer.datas.is_null() {
            return;
        }
        // SAFETY: Native loop lock excludes consumer access. Read only scalars,
        // ending this temporary reference before further native operations.
        let wanted = unsafe { (*callbacks.state.get()).wanted };
        if !wanted {
            return;
        }
        // SAFETY: The valid SPA buffer declares at least one plane.
        let plane = unsafe { &*buffer.datas };
        let kind = spa::buffer::DataType::from_raw(plane.type_);
        let readable = spa::buffer::DataFlags::from_bits_retain(plane.flags)
            .contains(spa::buffer::DataFlags::READABLE);
        if !readable
            || (kind != spa::buffer::DataType::MemFd
                && kind != spa::buffer::DataType::MemPtr
                && kind != spa::buffer::DataType::DmaBuf)
            || (plane.data.is_null() && kind == spa::buffer::DataType::MemPtr)
        {
            // Reject unsupported data kinds or a MemPtr with no readable data.
            callbacks.fail(4);
            return;
        }
        if kind == spa::buffer::DataType::DmaBuf && buffer.n_datas != 1 {
            callbacks.fail(4); // no implicit multi-plane/modifier interpretation
            return;
        }
        // No consumer wait/reentrant native operation occurs inside this block.
        let copied = {
            // SAFETY: Only this callback can mutate state under the native lock.
            let state = unsafe { &mut *callbacks.state.get() };
            // SAFETY: The native lease retains descriptors/FDs and excludes
            // producer writes until this synchronous copy and owned unmap end.
            unsafe { copy_plane(state, plane, callbacks) }
        };
        match copied {
            Ok(evidence) => {
                callbacks.add(&callbacks.copies, 1);
                if evidence.allocated_storage {
                    callbacks.add(&callbacks.allocated, 1);
                }
                // SAFETY: Actual loop lock is still held. No state reference
                // survives the signal or this callback's buffer return.
                unsafe {
                    (*callbacks.state.get()).wanted = false;
                }
                // SAFETY: Owning live loop, source success signal point.
                unsafe {
                    sys::pw_thread_loop_signal(callbacks.loop_.get(), false);
                }
            }
            Err(PlaneError::Copy(error)) => callbacks.fail(copy_fault(error)),
            Err(PlaneError::Map(error)) => {
                let _ = callbacks.mapping_error.compare_exchange(
                    0,
                    error.code,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                callbacks.fail(5);
            }
        }
    });
}
#[derive(Debug)]
enum PlaneError {
    Copy(CopyError),
    Map(NativeError),
}
enum PlaneMap<'a> {
    MemFd(MemFdMap<'a>),
    Dma(DmaBufMap<'a>),
}
impl PlaneMap<'_> {
    fn pointer(&self) -> *const u8 {
        match self {
            Self::MemFd(map) => map.pointer(),
            Self::Dma(map) => map.pointer(),
        }
    }
}
/// # Safety
/// Caller exclusively owns State (the actual loop lock for callback State).
/// All descriptor/chunk fields and representable nonnegative FDs stay live.
/// An existing pointer covers the declared readable span; producer writes and
/// truncation are excluded through copy and mapping destruction. Mapping may
/// fail without reading pixels. No native pointer escapes this call.
unsafe fn copy_plane(
    state: &mut State,
    plane: &spa::sys::spa_data,
    callbacks: &Callbacks,
) -> Result<CopyEvidence, PlaneError> {
    // SAFETY: Lease retains the optional initialized chunk for this call.
    let chunk = unsafe { plane.chunk.as_ref() }.map(|c| Chunk {
        stride: c.stride,
        offset: c.offset,
        size: c.size,
        flags: c.flags,
    });
    let capacity = if plane.data.is_null() && plane.maxsize == 0 {
        chunk.map_or(0, |c| c.size) // same no-pointer MemFd source fallback
    } else {
        plane.maxsize
    };
    let layout = Layout::checked(state.width, state.height, state.format, capacity, chunk)
        .map_err(PlaneError::Copy)?;
    let mapping = if plane.data.is_null() {
        // SAFETY: Same retained lease, checked target span and channel. The
        // platform owner verifies descriptor/file extent before its fresh mmap.
        let mapping = if plane.type_ == spa::sys::SPA_DATA_DmaBuf {
            // SAFETY: Same retained descriptor/native lease and exclusive
            // provider handoff; the DMA owner preserves U's one-shot sync path.
            PlaneMap::Dma(
                unsafe {
                    DmaBufMap::new(
                        plane,
                        layout.readable_span(),
                        layout.mapping_length().map_err(PlaneError::Copy)?,
                        &callbacks.dma,
                        &callbacks.unmap_error,
                        &callbacks.unmap_calls,
                    )
                }
                .map_err(PlaneError::Map)?,
            )
        } else {
            PlaneMap::MemFd(
                unsafe {
                    MemFdMap::new(
                        plane,
                        layout.readable_span(),
                        &callbacks.unmap_error,
                        &callbacks.unmap_calls,
                    )
                }
                .map_err(PlaneError::Map)?,
            )
        };
        callbacks.add(&callbacks.owned_maps, 1);
        Some(mapping)
    } else {
        if plane.type_ == spa::sys::SPA_DATA_DmaBuf {
            callbacks.dma.borrowed_pointer();
        }
        None
    };
    let pointer = mapping
        .as_ref()
        .map_or(plane.data.cast_const().cast(), PlaneMap::pointer);
    // SAFETY: MAP_BUFFERS retains existing SHM pointers (including native mapoffset)
    // or the owned map covers these checked target rows. No padding is borrowed.
    // Guard drop finishes unmap before wanted/signal and the outer buffer return.
    unsafe { state.cache.copy_native(pointer, layout) }.map_err(PlaneError::Copy)
}
fn copy_fault(error: CopyError) -> u32 {
    16 + error as u32
}

pub(super) struct Native {
    resources: Resources,
    callbacks: Box<Callbacks>,
    epoch: Instant,
    locked: bool,
    input_fd: i32,
}
#[derive(Debug)]
pub(super) struct Evidence {
    pub fault: u32,
    pub queue_error: i32,
    pub core_disconnect_error: i32,
    pub mapping_error: i32,
    pub unmap_error: i32,
    pub owned_maps: u64,
    pub unmap_calls: u64,
    pub dma: DmaSyncSnapshot,
    pub dequeued: u64,
    pub returned: u64,
    pub copies: u64,
    pub allocated: u64,
}
impl Native {
    pub fn new() -> Result<Self, std::alloc::AllocError> {
        let resources = Resources::new()?;
        let callbacks = Box::try_new(Callbacks {
            loop_: Cell::new(ptr::null_mut()),
            stream: Cell::new(ptr::null_mut()),
            connected: AtomicBool::new(false),
            state: UnsafeCell::new(State::default()),
            fault: AtomicU32::new(0),
            queue_error: AtomicI32::new(0),
            mapping_error: AtomicI32::new(0),
            unmap_error: AtomicI32::new(0),
            owned_maps: AtomicU64::new(0),
            unmap_calls: AtomicU64::new(0),
            dma: DmaSyncEvidence::default(),
            dequeued: Arc::try_new(AtomicU64::new(0))?,
            returned: AtomicU64::new(0),
            copies: AtomicU64::new(0),
            allocated: AtomicU64::new(0),
        })?;
        // SAFETY: Source initializes once per constructor and never deinitializes
        // in its destructor. Preserve that raw policy in U; no high-level binding
        // constructor/OnceLock initialization is mixed in. Explicit test cleanup
        // may balance it only after all native resources have been destroyed.
        unsafe {
            sys::pw_init(ptr::null_mut(), ptr::null_mut());
        }
        Ok(Self {
            resources,
            callbacks,
            epoch: Instant::now(),
            locked: false,
            input_fd: -1,
        })
    }
    /// # Safety
    /// `fd` must identify the caller's valid uniquely offered socket. Detach any
    /// owning handle before `init_source`; no independent close/reuse may race
    /// it. Before an init attempt the caller still owns that descriptor. After
    /// a null native core, consumption/residual ownership is protocol-specific;
    /// only act on it after complete close, using separately proven ownership.
    /// This private U entry intentionally is not a safe host FD contract.
    pub unsafe fn with_fd(fd: i32) -> Result<Self, std::alloc::AllocError> {
        assert!(fd >= 0);
        let mut result = Self::new()?;
        result.input_fd = fd;
        Ok(result)
    }
    pub fn init_daemon(&mut self, node: u32) -> Result<(), NativeError> {
        assert!(
            self.input_fd < 0,
            "pending supplied fd requires unsafe init_source"
        );
        self.initialize(node)
    }
    /// # Safety
    /// If a supplied descriptor remains pending, the `with_fd` contract must
    /// still hold for this attempt, including after a previous native failure.
    /// Successful native core creation consumes it; subsequent initialization
    /// follows source daemon mode after close. No new recovery policy is added.
    pub unsafe fn init_source(&mut self, node: u32) -> Result<(), NativeError> {
        self.initialize(node)
    }
    fn initialize(&mut self, node: u32) -> Result<(), NativeError> {
        if self.connected() {
            return Ok(());
        }
        self.close();
        let result = (|| {
            let data = ptr::from_ref(&*self.callbacks).cast_mut().cast();
            // SAFETY: Box address/event records stay fixed through stop, hook
            // removal and resource destruction; all setup calls are serialized.
            // A pending fd is reachable only through unsafe init_source and its
            // ownership contract; the safe daemon entry rejects pending offers.
            unsafe {
                self.resources.create_source(
                    &mut self.input_fd,
                    data,
                    &CORE_EVENTS,
                    &STREAM_EVENTS,
                )?;
            }
            self.callbacks.loop_.set(self.resources.loop_);
            self.callbacks.stream.set(self.resources.stream);
            let params = pod::Params::new().map_err(|()| NativeError {
                operation: "SPA fixed-capacity builder",
                code: -1,
            })?;
            // SAFETY: Source starts loop before connecting stream; no second
            // start, and every callback pointer is initialized before this call.
            let started = unsafe { sys::pw_thread_loop_start(self.resources.loop_) };
            if started < 0 {
                return Err(NativeError {
                    operation: "pw_thread_loop_start",
                    code: started,
                });
            }
            let mut pointers = params.pointers();
            let flags = pipewire::stream::StreamFlags::AUTOCONNECT
                | pipewire::stream::StreamFlags::DONT_RECONNECT
                | pipewire::stream::StreamFlags::MAP_BUFFERS;
            let connected = {
                self.lock();
                let lock = super::Locked(self);
                // SAFETY: Live locked stream and aligned complete POD owners
                // cover this call. Native copies the negotiation parameters.
                unsafe {
                    sys::pw_stream_connect(
                        lock.0.resources.stream,
                        spa::utils::Direction::Input.as_raw(),
                        node,
                        flags.bits(),
                        pointers.as_mut_ptr(),
                        3,
                    )
                }
            };
            if connected < 0 {
                return Err(NativeError {
                    operation: "pw_stream_connect",
                    code: connected,
                });
            }
            let negotiated = {
                self.lock();
                let lock = super::Locked(self);
                wait_until(lock.0, 3, |driver| {
                    // SAFETY: Actual lock held; borrow ends before timed_wait.
                    let state = unsafe { &*driver.callbacks.state.get() };
                    state.width > 0 && state.height > 0
                })
            };
            if negotiated != Ok(true) {
                return Err(NativeError {
                    operation: "format negotiation wait",
                    code: -1,
                });
            }
            self.callbacks.connected.store(true, Ordering::Release);
            Ok(())
        })();
        if result.is_err() {
            self.close();
        }
        result
    }
    pub fn close(&mut self) {
        assert!(
            !self.locked,
            "stop/join requires the native loop lock released"
        );
        // No mutable state borrow is held during stop/join or native callbacks.
        self.resources.close();
        self.callbacks.connected.store(false, Ordering::Release);
        self.callbacks.loop_.set(ptr::null_mut());
        self.callbacks.stream.set(ptr::null_mut());
        // SAFETY: Stop joined the loop and native hooks/resources are destroyed;
        // no callback can retain/access state. External frames own only CPU Arcs.
        unsafe {
            *self.callbacks.state.get() = State::default();
        }
    }
    pub fn evidence(&self) -> Evidence {
        Evidence {
            fault: self.callbacks.fault.load(Ordering::Acquire),
            queue_error: self.callbacks.queue_error.load(Ordering::Acquire),
            core_disconnect_error: self.resources.core_disconnect_error,
            mapping_error: self.callbacks.mapping_error.load(Ordering::Acquire),
            unmap_error: self.callbacks.unmap_error.load(Ordering::Acquire),
            owned_maps: self.callbacks.owned_maps.load(Ordering::Acquire),
            unmap_calls: self.callbacks.unmap_calls.load(Ordering::Acquire),
            dma: self.callbacks.dma.snapshot(),
            dequeued: self.callbacks.dequeued.load(Ordering::Acquire),
            returned: self.callbacks.returned.load(Ordering::Acquire),
            copies: self.callbacks.copies.load(Ordering::Acquire),
            allocated: self.callbacks.allocated.load(Ordering::Acquire),
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.close();
    }
}
impl Driver for Native {
    type Raw = RawFrame;
    fn connected(&self) -> bool {
        self.callbacks.connected.load(Ordering::Acquire)
    }
    fn stream_and_loop_exist(&self) -> bool {
        !self.resources.stream.is_null() && !self.resources.loop_.is_null()
    }
    fn lock(&mut self) {
        assert!(!self.locked && !self.resources.loop_.is_null());
        // SAFETY: Consumer operations require initialized live resources; no
        // concurrent consumer/close exists. This excludes all native callbacks.
        unsafe {
            sys::pw_thread_loop_lock(self.resources.loop_);
        }
        self.locked = true;
    }
    fn unlock(&mut self) {
        assert!(self.locked);
        self.locked = false;
        // SAFETY: Matching native lock on the owning consumer channel.
        unsafe {
            sys::pw_thread_loop_unlock(self.resources.loop_);
        }
    }
    fn set_active(&mut self, active: bool) {
        assert!(self.locked && !self.resources.stream.is_null());
        // SAFETY: Live locked stream; U intentionally ignores activation result.
        unsafe {
            sys::pw_stream_set_active(self.resources.stream, active);
        }
    }
    fn set_wanted(&mut self, wanted: bool) {
        assert!(self.locked);
        // SAFETY: Actual native lock excludes callback writes; no state reference
        // escapes this scalar write or survives the subsequent native wait.
        unsafe {
            (*self.callbacks.state.get()).wanted = wanted;
        }
    }
    fn wanted(&self) -> bool {
        assert!(self.locked);
        // SAFETY: Predicate runs under actual loop lock; scalar borrow ends here.
        unsafe { (*self.callbacks.state.get()).wanted }
    }
    fn latest_is_empty(&self) -> bool {
        assert!(self.locked);
        // SAFETY: Locked consumer state access, no borrow survives native wait.
        unsafe { (*self.callbacks.state.get()).cache.is_empty() }
    }
    fn snapshot(&mut self) -> (Option<RawFrame>, Format) {
        assert!(self.locked);
        // SAFETY: Actual lock is held. Snapshot retains independent CPU storage;
        // it borrows no native memory and does not let this reference escape.
        let state = unsafe { &*self.callbacks.state.get() };
        (state.cache.snapshot(), state.format)
    }
    fn now(&mut self) -> Duration {
        self.epoch.elapsed()
    }
    fn timed_wait(&mut self, seconds: i32) -> i32 {
        assert!(self.locked && seconds > 0);
        // SAFETY: The actual loop lock is held. This releases/reacquires it with
        // NO reference into callback-mutated state alive. Keep the native i32;
        // high-level timed_wait discards it and would change the U predicate.
        unsafe { sys::pw_thread_loop_timed_wait(self.resources.loop_, seconds) }
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
