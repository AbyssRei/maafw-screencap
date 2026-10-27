// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
#![cfg(target_os = "windows")]
#![feature(allocator_ext)]
#![deny(unsafe_op_in_unsafe_fn)]

#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_prototype.rs"));
}

use std::{
    error::Error,
    ffi::{CStr, c_void},
    mem::{align_of, offset_of, size_of},
    ptr::{NonNull, null_mut},
    sync::{
        Arc, Mutex,
        atomic::{AtomicI32, AtomicUsize, Ordering},
    },
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// Verified against interop.h by mfs_p0_layout; private to this P0 test target.
#[repr(C)]
struct Span {
    data: *const u8,
    length: usize,
    width: u32,
    height: u32,
    stride: u32,
    channels: u32,
}
type Callback = unsafe extern "C" fn(*mut c_void, *mut c_void, *const Span, i32);
type Start = unsafe extern "C" fn(*mut c_void, Option<Callback>, u32, *mut *mut c_void) -> i32;
type Stop = unsafe extern "C" fn(*mut c_void) -> i32;
type Layout = unsafe extern "C" fn(*mut u64, usize) -> i32;
type Info = unsafe extern "C" fn() -> *const i8;

struct Module(native::HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: This owns one LoadLibraryExW reference. Worker owners retain
        // another Arc until successful native join or an explicitly leaked error.
        assert_ne!(unsafe { native::FreeLibrary(self.0) }, 0);
    }
}

impl Module {
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T> {
        if size_of::<T>() != size_of::<native::FARPROC>() {
            return Err("unexpected function pointer representation".into());
        }
        // SAFETY: Module is loaded, name is NUL-terminated and borrowed by call.
        let pointer = unsafe { native::GetProcAddress(self.0, name.as_ptr().cast()) }
            .ok_or("native test symbol missing")?;
        // SAFETY: The caller must supply the exact exported C signature. This
        // is only used below for functions defined by the private interop.h.
        Ok(unsafe { std::mem::transmute_copy(&pointer) })
    }
}

struct Adapter {
    module: Arc<Module>,
    start: Start,
    stop: Stop,
    layout: Layout,
    info: Info,
}

impl Adapter {
    fn load() -> Result<Self> {
        let path = std::env::var_os("MFS_P0_NATIVE_LIBRARY")
            .ok_or("MFS_P0_NATIVE_LIBRARY is required; run cargo xtask native --desktop")?;
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = path.encode_wide().chain(Some(0)).collect();
        // SAFETY: Absolute path comes from explicit test configuration and is
        // NUL-terminated. No process-wide DLL search state is modified.
        let handle = unsafe { native::LoadLibraryExW(path.as_ptr(), null_mut(), 0x100 | 0x1000) };
        if handle.is_null() {
            return Err("LoadLibraryExW failed for native prototype".into());
        }
        let module = Arc::try_new(Module(handle))?;
        // SAFETY: Signatures and C calling convention match interop.h, and all
        // function pointers are retained together with the loaded module owner.
        unsafe {
            Ok(Self {
                start: module.symbol(c"mfs_p0_start")?,
                stop: module.symbol(c"mfs_p0_stop")?,
                layout: module.symbol(c"mfs_p0_layout")?,
                info: module.symbol(c"mfs_p0_opencv_info")?,
                module,
            })
        }
    }
}

struct Context {
    stop: Stop,
    panic_in_callback: bool,
    status: AtomicI32,
    calls: AtomicUsize,
    reentrant_stop: AtomicI32,
    pixels: Mutex<Option<Vec<u8>>>,
}

unsafe extern "C" fn callback(
    context: *mut c_void,
    worker: *mut c_void,
    span: *const Span,
    status: i32,
) {
    // SAFETY: Native start borrows this stable Box until successful stop/join.
    // Context contains only synchronized data and an immutable function pointer.
    let context = unsafe { &*context.cast::<Context>() };
    let result = std::panic::catch_unwind(|| -> std::result::Result<(), i32> {
        context.calls.fetch_add(1, Ordering::SeqCst);
        if context.panic_in_callback {
            panic!("injected Rust callback panic");
        }
        if status != 0 || span.is_null() {
            return Err(-1);
        }
        // SAFETY: Native callback supplies a live span for this invocation.
        let span = unsafe { &*span };
        if span.data.is_null()
            || span.width != 2
            || span.height != 2
            || span.channels != 3
            || span.stride != 6
            || span.length != 12
        {
            return Err(-2);
        }
        // SAFETY: This trusted C++ fixture provides 12 readable bytes, only for
        // this call. The exact declared layout and bounded length were checked.
        let input = unsafe { std::slice::from_raw_parts(span.data, span.length) };
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(input.len()).map_err(|_| -3)?;
        pixels.extend_from_slice(input);
        // SAFETY: The worker is live. This prototype explicitly supports asking
        // stop from its callback and must return 2 without consuming ownership.
        let reentrant = unsafe { (context.stop)(worker) };
        context.reentrant_stop.store(reentrant, Ordering::SeqCst);
        *context.pixels.lock().map_err(|_| -5)? = Some(pixels);
        Ok(())
    });
    // Completion publication needs no allocation, including panic/error paths.
    context.status.store(
        match result {
            Ok(Ok(())) => 0,
            Ok(Err(code)) => code,
            Err(_) => -4,
        },
        Ordering::SeqCst,
    );
}

struct Worker {
    handle: *mut c_void,
    context: Option<NonNull<Context>>,
    module: Option<Arc<Module>>,
    stop: Stop,
}

impl Worker {
    fn start(adapter: &Adapter, panic_in_callback: bool) -> Result<Self> {
        Self::start_with_fault(adapter, panic_in_callback, 0)
    }

    fn start_with_fault(adapter: &Adapter, panic_in_callback: bool, fault: u32) -> Result<Self> {
        let context = Box::try_new(Context {
            stop: adapter.stop,
            panic_in_callback,
            status: AtomicI32::new(-99),
            calls: AtomicUsize::new(0),
            reentrant_stop: AtomicI32::new(-99),
            pixels: Mutex::new(None),
        })?;
        // Transfer unique Box ownership before registration. No Box reborrow or
        // move may retag the pointee while a foreign thread holds shared access.
        let context = NonNull::new(Box::into_raw(context)).unwrap();
        let mut handle = null_mut();
        // SAFETY: Context has stable owned storage and synchronized callback data.
        // Native start either fails with null output or borrows it until join.
        let status =
            unsafe { (adapter.start)(context.as_ptr().cast(), Some(callback), fault, &mut handle) };
        if status != 0 || handle.is_null() {
            // SAFETY: The private C++ start contract guarantees that a failed
            // start leaves no thread/callback retaining this allocation.
            drop(unsafe { Box::from_raw(context.as_ptr()) });
            return Err(format!("native start failed: {status}").into());
        }
        Ok(Self {
            handle,
            context: Some(context),
            module: Some(Arc::clone(&adapter.module)),
            stop: adapter.stop,
        })
    }

    fn finish(mut self) -> Result<Box<Context>> {
        // SAFETY: This is the sole non-callback stop caller and owns the handle.
        // Successful return joins the callback thread and consumes native state.
        let status = unsafe { (self.stop)(self.handle) };
        if status != 0 {
            self.abandon();
            return Err(format!("native stop failed, context/module retained: {status}").into());
        }
        self.handle = null_mut();
        let context = self.context.take().ok_or("callback context missing")?;
        // SAFETY: Successful native stop joined the only callback thread. This
        // consumes the original Box allocation exactly once, after all aliases.
        Ok(unsafe { Box::from_raw(context.as_ptr()) })
    }

    fn abandon(&mut self) {
        // A failed join is not proof of callback drain. Retain every owner rather
        // than freeing context/module under a potentially active foreign thread.
        self.context.take(); // Deliberately retain the heap allocation on error.
        if let Some(owner) = self.module.take() {
            std::mem::forget(owner);
        }
        self.handle = null_mut();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: Same sole-owner stop contract as finish, for early errors.
            let status = unsafe { (self.stop)(self.handle) };
            if status != 0 {
                self.abandon();
            } else if let Some(context) = self.context.take() {
                // SAFETY: Successful stop drained all accesses; this is the
                // original unique allocation, still owned by the early-exit guard.
                drop(unsafe { Box::from_raw(context.as_ptr()) });
            }
        }
    }
}

#[test]
#[ignore = "requires the explicitly built C++/OpenCV P0 adapter"]
fn verifies_c_layout_and_failure_outputs() -> Result<()> {
    let adapter = Adapter::load()?;
    let mut layout = [u64::MAX; 8];
    // SAFETY: Writable capacity is accurate and the module is retained.
    assert_eq!(
        unsafe { (adapter.layout)(layout.as_mut_ptr(), layout.len()) },
        0
    );
    assert_eq!(
        layout,
        [
            size_of::<Span>() as u64,
            align_of::<Span>() as u64,
            offset_of!(Span, data) as u64,
            offset_of!(Span, length) as u64,
            offset_of!(Span, width) as u64,
            offset_of!(Span, height) as u64,
            offset_of!(Span, stride) as u64,
            offset_of!(Span, channels) as u64
        ]
    );
    let mut sentinel = u64::MAX;
    // SAFETY: Zero capacity requests no writes, despite a live output pointer.
    assert_eq!(unsafe { (adapter.layout)(&mut sentinel, 0) }, 1);
    assert_eq!(sentinel, u64::MAX);
    let mut output = NonNull::<c_void>::dangling().as_ptr();
    // SAFETY: Null context is a defined rejection; callback must not be invoked.
    assert_eq!(
        unsafe { (adapter.start)(null_mut(), Some(callback), 0, &mut output) },
        1
    );
    assert!(output.is_null());
    // SAFETY: Function returns a borrowed static string owned by the module.
    let info = unsafe { (adapter.info)() };
    if info.is_null() {
        return Err("OpenCV build information unavailable".into());
    }
    // SAFETY: Successful native result is NUL-terminated until module unload.
    println!("{}", unsafe { CStr::from_ptr(info) }.to_string_lossy());
    Ok(())
}

#[test]
#[ignore = "requires the explicitly built C++/OpenCV P0 adapter"]
fn drains_foreign_callback_and_keeps_independent_converted_pixels() -> Result<()> {
    fn assert_context_thread_safe<T: Send + Sync>() {}
    assert_context_thread_safe::<Context>();
    let adapter = Adapter::load()?;
    let context = Worker::start(&adapter, false)?.finish()?;
    assert_eq!(context.calls.load(Ordering::SeqCst), 1);
    assert_eq!(context.status.load(Ordering::SeqCst), 0);
    assert_eq!(context.reentrant_stop.load(Ordering::SeqCst), 2);
    let pixels = context
        .pixels
        .into_inner()
        .map_err(|_| "poisoned pixel state")?
        .ok_or("callback pixels missing")?;
    drop(adapter);
    std::thread::spawn(move || assert_eq!(pixels, [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]))
        .join()
        .unwrap();
    Ok(())
}

#[test]
#[ignore = "requires the explicitly built C++/OpenCV P0 adapter"]
fn seals_rust_panic_at_foreign_callback_boundary() -> Result<()> {
    let adapter = Adapter::load()?;
    let context = Worker::start(&adapter, true)?.finish()?;
    assert_eq!(context.calls.load(Ordering::SeqCst), 1);
    assert_eq!(context.status.load(Ordering::SeqCst), -4);
    assert!(
        context
            .pixels
            .into_inner()
            .map_err(|_| "poisoned pixel state")?
            .is_none()
    );
    Ok(())
}

#[test]
#[ignore = "requires the explicitly built C++/OpenCV P0 adapter"]
fn seals_native_constructor_and_conversion_exceptions() -> Result<()> {
    let adapter = Adapter::load()?;
    assert!(Worker::start_with_fault(&adapter, false, 1).is_err());
    let context = Worker::start_with_fault(&adapter, false, 2)?.finish()?;
    assert_eq!(context.calls.load(Ordering::SeqCst), 1);
    assert_eq!(context.status.load(Ordering::SeqCst), -1);
    assert!(
        context
            .pixels
            .into_inner()
            .map_err(|_| "poisoned pixel state")?
            .is_none()
    );
    Ok(())
}
