// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{
    env,
    ffi::{CStr, CString, c_void},
    mem::{align_of, offset_of, size_of},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[repr(C)]
#[derive(Default)]
struct View {
    pixels: *const u8,
    length: usize,
    stride: usize,
    width: i32,
    height: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Resources {
    descriptors: u64,
    threads: u64,
    ports: u64,
}
type Convert =
    unsafe extern "C" fn(*const u8, usize, i32, i32, usize, i32, i32, *mut *mut c_void) -> i32;
type NativeView = unsafe extern "C" fn(*const c_void, *mut View) -> i32;
type Release = unsafe extern "C" fn(*mut c_void);
type Owners = unsafe extern "C" fn() -> u64;
type Layout = unsafe extern "C" fn(u32) -> usize;
type Snapshot = unsafe extern "C" fn(*mut Resources) -> i32;
static MODULE_CLOSED: AtomicUsize = AtomicUsize::new(0);
static MODULE_ERROR: AtomicI32 = AtomicI32::new(0);
struct Module(NonNull<c_void>);
// SAFETY: Only this explicitly built C++ acceptance image is loaded. Its handle
// is immutable after resolving symbols, every call/image owns an Arc, and final
// dlclose occurs after all native calls and CPU Mat owners have ended. This is
// a module lifetime proof, not Send/Sync for arbitrary native capture objects.
unsafe impl Send for Module {}
unsafe impl Sync for Module {}
impl Module {
    fn open() -> Arc<Self> {
        let path = env::var_os("MFS_P1_MACOS_OPENCV_ADAPTER").unwrap();
        assert!(Path::new(&path).is_absolute());
        let path = CString::new(path.as_bytes()).unwrap();
        // SAFETY: Explicit absolute prepared module path; immediate/local scope.
        let pointer = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        Arc::try_new(Self(
            NonNull::new(pointer).expect("load macOS acceptance adapter"),
        ))
        .unwrap()
    }
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        // SAFETY: Live module/static symbol; caller supplies exact private header
        // signature and verifies native layout plus actual consumption below.
        let pointer = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        assert!(!pointer.is_null());
        // SAFETY: POSIX callable symbol representation, exact header signature.
        unsafe { std::mem::transmute_copy(&pointer) }
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Last Arc, no active call/view/CPU Mat owner remains. Native
        // static destructors execute before image removal in this dlclose.
        let code = unsafe { libc::dlclose(self.0.as_ptr()) };
        if code != 0 {
            MODULE_ERROR.store(code, Ordering::Release);
        }
        if MODULE_CLOSED
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            // Final ownership may end in a native block-dispose trampoline;
            // record failure without unwinding through that C ABI.
            let _ = MODULE_ERROR.compare_exchange(0, -2, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}
struct Adapter {
    module: Arc<Module>,
    convert: Convert,
    view: NativeView,
    release: Release,
    owners: Owners,
    snapshot: Snapshot,
}
impl Adapter {
    fn open() -> Arc<Self> {
        if !objc2_core_graphics::CGPreflightScreenCaptureAccess() {
            eprintln!(
                "missing_environment: native Rust caller lacks Screen Recording permission; no prompt requested"
            );
            std::process::exit(77);
        }
        let module = Module::open();
        // SAFETY: conversion-adapter.h and acceptance-evidence.h are the ABI
        // authorities. Their exact sources compile into this selected image.
        let (convert, view, release, owners, layout, snapshot, resources) = unsafe {
            (
                module.symbol::<Convert>(c"mfs_pw_convert"),
                module.symbol::<NativeView>(c"mfs_pw_bgr_view"),
                module.symbol::<Release>(c"mfs_pw_bgr_release"),
                module.symbol::<Owners>(c"mfs_pw_bgr_owners"),
                module.symbol::<Layout>(c"mfs_pw_adapter_layout"),
                module.symbol::<Snapshot>(c"mfs_mac_resource_snapshot"),
                module.symbol::<Layout>(c"mfs_mac_resource_layout"),
            )
        };
        // SAFETY: Actual retained callable address, initialized system record;
        // dyld's filename borrow remains valid while this module owner lives.
        let mut owner = unsafe { std::mem::zeroed::<libc::Dl_info>() };
        assert_ne!(
            unsafe { libc::dladdr(convert as *const c_void, &mut owner) },
            0
        );
        assert!(!owner.dli_fname.is_null());
        let loaded = unsafe { CStr::from_ptr(owner.dli_fname) };
        let loaded = Path::new(std::ffi::OsStr::from_bytes(loaded.to_bytes()));
        let expected = env::var_os("MFS_P1_MACOS_OPENCV_ADAPTER").unwrap();
        assert_eq!(
            std::fs::canonicalize(loaded).unwrap(),
            std::fs::canonicalize(expected).unwrap()
        );
        for (index, expected) in [
            1,
            size_of::<View>(),
            align_of::<View>(),
            offset_of!(View, pixels),
            offset_of!(View, length),
            offset_of!(View, stride),
            offset_of!(View, width),
            offset_of!(View, height),
        ]
        .into_iter()
        .enumerate()
        {
            // SAFETY: Scalar query from the live selected native module.
            assert_eq!(unsafe { layout(u32::try_from(index).unwrap()) }, expected);
        }
        for (index, expected) in [
            1,
            size_of::<Resources>(),
            align_of::<Resources>(),
            offset_of!(Resources, descriptors),
            offset_of!(Resources, threads),
            offset_of!(Resources, ports),
        ]
        .into_iter()
        .enumerate()
        {
            // SAFETY: Exact private header scalar record, consumed by snapshot.
            assert_eq!(
                unsafe { resources(u32::try_from(index).unwrap()) },
                expected
            );
        }
        Arc::try_new(Self {
            module,
            convert,
            view,
            release,
            owners,
            snapshot,
        })
        .unwrap()
    }
    fn resources(&self) -> Resources {
        let mut output = Resources::default();
        // SAFETY: Initialized complete native header record, live module.
        assert_eq!(unsafe { (self.snapshot)(&mut output) }, 0);
        output
    }
    fn owners(&self) -> u64 {
        // SAFETY: Module is retained; native CPU image ledger is atomic.
        unsafe { (self.owners)() }
    }
    fn convert(
        &self,
        bytes: &[u8],
        width: usize,
        height: usize,
        fault: bool,
    ) -> Result<Image, Error> {
        let mut pointer = std::ptr::null_mut();
        let stride = width.checked_mul(4).unwrap();
        // A valid two-channel Mat triggers the real BGRA cv::Exception. The
        // unchanged C ABI has eight arguments and contains all C++ exceptions.
        let channels = if fault { 2 } else { 4 };
        // SAFETY: Initialized owned packed input retained synchronously through
        // conversion; C++ creates an independent CPU cv::Mat on success.
        let code = unsafe {
            (self.convert)(
                bytes.as_ptr(),
                bytes.len(),
                i32::try_from(width).unwrap(),
                i32::try_from(height).unwrap(),
                stride,
                channels,
                1,
                &mut pointer,
            )
        };
        if code != 0 {
            assert!(pointer.is_null());
            return Err(Error::new("native macOS conversion", i64::from(code)));
        }
        Ok(Image::Native(CpuMat {
            pointer: NonNull::new(pointer).unwrap(),
            module: self.module.clone(),
            view: self.view,
            release: self.release,
            width,
            height,
        }))
    }
    fn warm(&self) {
        let bytes = vec![255; 47 * 31 * 4];
        drop(self.convert(&bytes, 47, 31, false).unwrap());
        assert_eq!(self.owners(), 0);
    }
}
struct CpuMat {
    pointer: NonNull<c_void>,
    module: Arc<Module>,
    view: NativeView,
    release: Release,
    width: usize,
    height: usize,
}
// SAFETY: The exact native Image implementation owns only an independent CPU
// Mat from synchronous cvtColor. No provider/CF/SC object, mapping, mutable alias
// or borrowed input escapes. Exclusive ownership may transfer after conversion;
// Mat/allocator release is observed on a second thread. Every owner retains the
// code module through release. No Sync or arbitrary opaque native mobility.
unsafe impl Send for CpuMat {}
impl CpuMat {
    fn pixels(&self) -> &[u8] {
        let mut output = View::default();
        // SAFETY: Unique immutable CPU Mat and retained module, exact C record.
        assert_eq!(
            unsafe { (self.view)(self.pointer.as_ptr(), &mut output) },
            0
        );
        let row = self.width.checked_mul(3).unwrap();
        let length = row.checked_mul(self.height).unwrap();
        assert_eq!(
            (output.width, output.height, output.stride, output.length),
            (
                i32::try_from(self.width).unwrap(),
                i32::try_from(self.height).unwrap(),
                row,
                length
            )
        );
        assert!(!output.pixels.is_null());
        // SAFETY: Complete checked span owned by this actual CPU Mat; Rust
        // borrow prevents releasing it while this immutable view is used.
        unsafe { std::slice::from_raw_parts(output.pixels, length) }
    }
}
impl Drop for CpuMat {
    fn drop(&mut self) {
        let _ = &self.module;
        // SAFETY: Sole heap Image owner, no outstanding Rust views. Module field
        // releases only after this native destructor returns.
        unsafe { (self.release)(self.pointer.as_ptr()) };
    }
}
enum Image {
    Empty,
    Native(CpuMat),
}
impl Image {
    fn pixels(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Native(value) => value.pixels(),
        }
    }
}
struct ConvertAudit {
    adapter: Arc<Adapter>,
    trace: Arc<Trace>,
    held: Arc<Mutex<Vec<Sample>>>,
    exception: bool,
    delay: Duration,
    late_gate: Option<Arc<AtomicBool>>,
    successes: Arc<AtomicUsize>,
}
impl Converter for ConvertAudit {
    type Image = Image;
    fn empty(&self) -> Result<Image, Error> {
        Ok(Image::Empty)
    }
    fn convert(&self, sample: Sample) -> Result<Image, Error> {
        assert_eq!(self.trace.pixels.locks.load(Ordering::Acquire), 1);
        assert_eq!(self.trace.pixels.unlocks.load(Ordering::Acquire), 0);
        assert_eq!(
            *self.trace.sample_thread.lock().unwrap(),
            Some(std::thread::current().id())
        );
        self.held.lock().unwrap().push(sample.clone());
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        if let Some(gate) = &self.late_gate {
            // Fixture-only synchronization: let the caller finish scene close
            // and release its converter/module owners before conversion ends.
            // Six seconds of real delay still precede this gate. A loaded
            // runner cannot turn slow scene shutdown into a lifetime race.
            wait(|| gate.load(Ordering::Acquire));
        }
        assert_eq!(self.trace.pixels.unlocks.load(Ordering::Acquire), 0);
        let image =
            self.adapter
                .convert(sample.pixels(), sample.width, sample.height, self.exception)?;
        if (sample.width, sample.height) == (47, 31) {
            bgr(image.pixels());
        }
        assert_eq!(self.trace.pixels.unlocks.load(Ordering::Acquire), 0);
        self.successes.fetch_add(1, Ordering::AcqRel);
        Ok(image)
    }
}
fn trace() -> Arc<Trace> {
    Arc::try_new(Trace::default()).unwrap()
}
fn held() -> Arc<Mutex<Vec<Sample>>> {
    let mut samples = Vec::new();
    samples.try_reserve_exact(64).unwrap();
    Arc::try_new(Mutex::new(samples)).unwrap()
}
fn converter(
    adapter: &Arc<Adapter>,
    trace: &Arc<Trace>,
    held: &Arc<Mutex<Vec<Sample>>>,
    exception: bool,
    delay: Duration,
) -> Arc<ConvertAudit> {
    Arc::try_new(ConvertAudit {
        adapter: adapter.clone(),
        trace: trace.clone(),
        held: held.clone(),
        exception,
        delay,
        late_gate: None,
        successes: Arc::try_new(AtomicUsize::new(0)).unwrap(),
    })
    .unwrap()
}
trait FixtureCapture {
    fn invoke(
        &self,
        converter: Arc<ConvertAudit>,
        trace: Arc<Trace>,
    ) -> Result<Option<Image>, Error>;
}
impl FixtureCapture for Capture {
    fn invoke(
        &self,
        converter: Arc<ConvertAudit>,
        trace: Arc<Trace>,
    ) -> Result<Option<Image>, Error> {
        // Match the original fixture's caller-provided autorelease pool. The
        // capture implementation itself does not alter the host pool/run loop.
        objc2::rc::autoreleasepool(|_| self.capture_with(converter, trace))
    }
}
fn scene() -> u32 {
    env::var("MFS_P1_MACOS_WINDOW_ID").unwrap().parse().unwrap()
}
fn directory() -> PathBuf {
    PathBuf::from(env::var_os("MFS_P1_MACOS_RUST_ARTIFACTS").unwrap())
}
fn stop_scene() {
    std::fs::write(env::var_os("MFS_P1_MACOS_SCENE_STOP").unwrap(), b"stop\n").unwrap();
    let acknowledged = PathBuf::from(env::var_os("MFS_P1_MACOS_SCENE_STOPPED").unwrap());
    wait(|| acknowledged.is_file());
    assert_eq!(
        std::fs::read_to_string(acknowledged).unwrap(),
        "window_closed=1\n"
    );
}
fn wait(predicate: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(
            Instant::now() < until,
            "native callback owners did not drain within fixture budget"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn drained(trace: &Trace, sample: bool) {
    wait(|| {
        trace.request_released.load(Ordering::Acquire) == 1
            && (!sample || trace.block_inputs_released.load(Ordering::Acquire) == 1)
    });
    assert_eq!(trace.fault.load(Ordering::Acquire), 0);
    assert_eq!(trace.completed.load(Ordering::Acquire), 1);
    assert_eq!(
        trace.semaphore_block_owner_released.load(Ordering::Acquire),
        1
    );
    assert_eq!(trace.source_semaphore_retained.load(Ordering::Acquire), 1);
    assert_eq!(trace.fixture_semaphore_reclaimed.load(Ordering::Acquire), 0);
    if sample {
        assert_eq!(trace.initial_inputs_released.load(Ordering::Acquire), 1);
        assert_eq!(trace.pixels.locks.load(Ordering::Acquire), 1);
        assert_eq!(trace.pixels.unlocks.load(Ordering::Acquire), 1);
        assert_eq!(trace.pixels.unlock_error.load(Ordering::Acquire), 0);
        assert_eq!(trace.pixels.copies.load(Ordering::Acquire), 1);
        assert_eq!(trace.pixels.conversions.load(Ordering::Acquire), 1);
        assert!(trace.pixels.copied_bytes.load(Ordering::Acquire) > 0);
    }
}
fn bgr(bytes: &[u8]) {
    assert_eq!(bytes.len(), 47 * 31 * 3);
    for y in 2..29 {
        for x in 2..45 {
            assert_eq!(&bytes[(y * 47 + x) * 3..(y * 47 + x + 1) * 3], &[0, 255, 0]);
        }
    }
}
fn raw(sample: &Sample) {
    assert_eq!((sample.width, sample.height), (47, 31));
    assert!(sample.native_stride >= 47 * 4);
    assert_eq!(sample.pixels().len(), 47 * 31 * 4);
    for y in 2..29 {
        for x in 2..45 {
            assert_eq!(
                &sample.pixels()[(y * 47 + x) * 4..(y * 47 + x + 1) * 4],
                &[0, 255, 0, 255]
            );
        }
    }
}
fn warm_capture(adapter: &Arc<Adapter>) {
    let trace = trace();
    let held = held();
    let image = Capture::new(scene())
        .invoke(
            converter(adapter, &trace, &held, false, Duration::ZERO),
            trace.clone(),
        )
        .unwrap()
        .unwrap();
    bgr(image.pixels());
    drop(image);
    drained(&trace, true);
    trace.reclaim_source_semaphore_for_fixture();
    assert_eq!(adapter.owners(), 0);
}

#[test]
fn rejects_unrepresentable_source_config_dimensions() {
    assert_eq!(dimension(47.9), Ok(47));
    assert_eq!(dimension(0.0), Ok(0));
    for value in [-1.0, f64::NAN, f64::INFINITY, i32::MAX as f64 + 1.0] {
        assert!(dimension(value).is_err());
    }
}

#[test]
#[ignore = "explicit native macOS scene, unchanged original oracle and C++/OpenCV adapter"]
fn captures_original_window_with_callback_conversion_and_retained_cpu_owners() {
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    let oracle = std::fs::read(env::var_os("MFS_P1_MACOS_EXTERNAL_ORACLE").unwrap()).unwrap();
    bgr(&oracle);
    let adapter = Adapter::open();
    adapter.warm();
    warm_capture(&adapter);
    let baseline = adapter.resources();
    let samples = held();
    let mut images = Vec::new();
    images.try_reserve_exact(60).unwrap();
    let mut observations = format!(
        "cycle\tfd_count\tthread_count\tport_names\tfixture_reclaimed_since_baseline\nbaseline\t{}\t{}\t{}\t0\n",
        baseline.descriptors, baseline.threads, baseline.ports
    );
    for cycle in 0..20 {
        {
            let capture = Capture::new(scene());
            for call in 0..3 {
                let trace = trace();
                let image = capture
                    .invoke(
                        converter(&adapter, &trace, &samples, false, Duration::ZERO),
                        trace.clone(),
                    )
                    .unwrap()
                    .unwrap();
                drained(&trace, true);
                assert!(!trace.timed_out.load(Ordering::Acquire));
                assert_eq!(trace.wait_result.load(Ordering::Acquire), 0);
                assert_eq!(trace.source_failure.load(Ordering::Acquire), 0);
                assert_eq!(trace.native_error.load(Ordering::Acquire), 0);
                bgr(image.pixels());
                assert_eq!(image.pixels(), oracle);
                let last = samples.lock().unwrap().last().unwrap().clone();
                raw(&last);
                let stem = format!("cycle-{cycle}-call-{call}");
                std::fs::write(directory.join(format!("{stem}.raw")), last.pixels()).unwrap();
                std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
                std::fs::write(directory.join(format!("{stem}.txt")),format!("width={}\nheight={}\nnative_stride={}\nlocks=1\nunlocks=1\nconversion_before_unlock=1\nsource_semaphore_retained=1\nfixture_reclaim_follows=1\ncaller={:?}\ncontent={:?}\nsample={:?}\ninitial_release={:?}\nblock_release={:?}\n",last.width,last.height,last.native_stride,trace.caller.lock().unwrap(),trace.content_thread.lock().unwrap(),trace.sample_thread.lock().unwrap(),trace.initial_release_thread.lock().unwrap(),trace.block_release_thread.lock().unwrap())).unwrap();
                // Source create +1 is deliberately retained by U. Reclamation
                // belongs ONLY to this acceptance fixture after all real owners
                // ended; never count it as successful product cleanup.
                trace.reclaim_source_semaphore_for_fixture();
                assert_eq!(trace.fixture_semaphore_reclaimed.load(Ordering::Acquire), 1);
                images.push(image);
            }
        }
        let resources = adapter.resources();
        observations.push_str(&format!(
            "{cycle}\t{}\t{}\t{}\t{}\n",
            resources.descriptors,
            resources.threads,
            resources.ports,
            (cycle + 1) * 3
        ));
        std::fs::write(
            directory.join("resources-after-fixture-reclaim.tsv"),
            &observations,
        )
        .unwrap();
        assert_eq!(
            resources.descriptors, baseline.descriptors,
            "FD count after explicit fixture reclaim; not source product cleanup"
        );
        for sample in samples.lock().unwrap().iter() {
            raw(sample);
        }
    }
    stop_scene();
    std::thread::spawn(move || {
        for image in images {
            bgr(image.pixels());
        }
    })
    .join()
    .unwrap();
    std::thread::spawn(move || {
        for sample in samples.lock().unwrap().iter() {
            raw(sample);
        }
    })
    .join()
    .unwrap();
    assert_eq!(adapter.owners(), 0);
    assert_eq!(MODULE_CLOSED.load(Ordering::Acquire), 0);
    drop(adapter);
    assert_eq!(MODULE_CLOSED.load(Ordering::Acquire), 1);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust macOS cycles=20 captures=60 retained_raw=60 retained_bgr=60 callback_conversion=60 lock_pairs=60 source_create_refs_retained=60 fixture_reclaimed_refs=60 cross_thread_cpu_release=1 passed"
    );
}

#[test]
#[ignore = "explicit native macOS scene and real native conversion exception"]
fn releases_native_lock_after_conversion_exception_and_rejects_missing_window() {
    let adapter = Adapter::open();
    adapter.warm();
    warm_capture(&adapter);
    let capture = Capture::new(scene());
    let samples = held();
    let failed = trace();
    let result = capture.invoke(
        converter(&adapter, &failed, &samples, true, Duration::ZERO),
        failed.clone(),
    );
    assert_eq!(result.err(), Some(Error::new("native macOS conversion", 2)));
    drained(&failed, true);
    failed.reclaim_source_semaphore_for_fixture();
    let next = trace();
    let image = capture
        .invoke(
            converter(&adapter, &next, &samples, false, Duration::ZERO),
            next.clone(),
        )
        .unwrap()
        .unwrap();
    bgr(image.pixels());
    drained(&next, true);
    next.reclaim_source_semaphore_for_fixture();
    let missing = trace();
    let result = Capture::new(u32::MAX)
        .invoke(
            converter(&adapter, &missing, &samples, false, Duration::ZERO),
            missing.clone(),
        )
        .unwrap();
    assert!(result.is_none());
    drained(&missing, false);
    assert_eq!(missing.source_failure.load(Ordering::Acquire), 2);
    assert_eq!(missing.native_error.load(Ordering::Acquire), 0);
    assert_eq!(missing.sample_calls.load(Ordering::Acquire), 0);
    missing.reclaim_source_semaphore_for_fixture();
    stop_scene();
    bgr(image.pixels());
    drop(image);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust macOS conversion_exception=1 unlock_after_exception=1 subsequent_capture=1 missing_window=1 fixture_reclaimed_refs=3 passed"
    );
}

#[test]
#[ignore = "explicit native macOS scene; actual five-second wait and delayed conversion"]
fn retains_timed_out_callback_work_inputs_converter_and_late_result() {
    let adapter = Adapter::open();
    adapter.warm();
    warm_capture(&adapter);
    let trace = trace();
    let samples = held();
    let late_gate = Arc::try_new(AtomicBool::new(false)).unwrap();
    let mut converter = converter(&adapter, &trace, &samples, false, Duration::from_secs(6));
    Arc::get_mut(&mut converter).unwrap().late_gate = Some(late_gate.clone());
    let successes = converter.successes.clone();
    let start = Instant::now();
    let image = Capture::new(scene())
        .invoke(converter.clone(), trace.clone())
        .unwrap();
    assert!(image.is_none());
    assert!(trace.timed_out.load(Ordering::Acquire));
    assert!(start.elapsed() >= Duration::from_secs(4));
    assert_ne!(trace.wait_result.load(Ordering::Acquire), 0);
    assert_eq!(trace.pixels.locks.load(Ordering::Acquire), 1);
    assert_eq!(trace.pixels.unlocks.load(Ordering::Acquire), 0);
    assert_eq!(trace.initial_inputs_released.load(Ordering::Acquire), 0);
    assert_eq!(trace.request_released.load(Ordering::Acquire), 0);
    stop_scene();
    drop(converter);
    drop(adapter);
    assert_eq!(
        MODULE_CLOSED.load(Ordering::Acquire),
        0,
        "callback must own converter/module past caller timeout"
    );
    late_gate.store(true, Ordering::Release);
    drained(&trace, true);
    assert!(trace.late_completed.load(Ordering::Acquire));
    assert_eq!(trace.pixels.conversions.load(Ordering::Acquire), 1);
    assert_eq!(successes.load(Ordering::Acquire), 1);
    trace.reclaim_source_semaphore_for_fixture();
    wait(|| MODULE_CLOSED.load(Ordering::Acquire) == 1);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    for sample in samples.lock().unwrap().iter() {
        raw(sample);
    }
    eprintln!(
        "native Rust macOS timeout_seconds=5 delayed_conversion=1 late_conversion=1 native_inputs_retained=1 module_retained=1 late_owners_drained=1 fixture_reclaimed_refs=1 passed"
    );
}

#[test]
#[ignore = "explicit native macOS display and OpenCV conversion; known full-display pixels unasserted"]
fn captures_original_first_display_layout_and_retains_its_cpu_owners() {
    let adapter = Adapter::open();
    adapter.warm();
    let trace = trace();
    let samples = held();
    let image = Capture::new(0)
        .invoke(
            converter(&adapter, &trace, &samples, false, Duration::ZERO),
            trace.clone(),
        )
        .unwrap()
        .unwrap();
    drained(&trace, true);
    trace.reclaim_source_semaphore_for_fixture();
    let raw = samples.lock().unwrap().last().unwrap().clone();
    assert!(raw.width > 0 && raw.height > 0);
    assert_eq!(
        image.pixels().len(),
        raw.width
            .checked_mul(raw.height)
            .unwrap()
            .checked_mul(3)
            .unwrap()
    );
    let directory = directory();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("first-display.bgr"), image.pixels()).unwrap();
    std::fs::write(
        directory.join("first-display.txt"),
        format!(
            "width={}\nheight={}\nchannels=3\nknown_pixels=not_asserted\n",
            raw.width, raw.height
        ),
    )
    .unwrap();
    stop_scene();
    std::thread::spawn(move || {
        assert!(!image.pixels().is_empty());
        drop(image);
    })
    .join()
    .unwrap();
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust macOS first_display=1 callback_conversion=1 retained_cpu=1 known_pixels=not_asserted fixture_reclaimed_refs=1 passed"
    );
}
