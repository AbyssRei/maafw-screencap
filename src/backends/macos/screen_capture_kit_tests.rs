// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;

#[path = "../../../xtask/src/macos_display_geometry.rs"]
mod display_geometry;
#[path = "screen_capture_kit_display_tests.rs"]
mod display_patch;
#[path = "screen_capture_kit_sample_tests.rs"]
mod sample_matrix;
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
        Self::open_cpu()
    }
    // The native CPU sample fixture does not request capture or permission.
    // Existing actual capture probes keep the preflight above unchanged.
    fn open_cpu() -> Arc<Self> {
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
// SAFETY: Both exact private Image/SourceImage implementations own only an
// independent CPU Mat from synchronous cvtColor. No provider/CF/SC object, mapping, mutable alias
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

#[cfg(feature = "macos-manager")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ManagerScript {
    method: u64,
    window_id: u32,
    available: [u32; 2],
    fail_create: [u32; 2],
    outcomes: [u32; 3],
}
#[cfg(feature = "macos-manager")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ManagerEvent {
    kind: u32,
    unit: u32,
    call: u32,
    reserved: u32,
}
#[cfg(feature = "macos-manager")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ManagerReport {
    connect_status: [u32; 2],
    connected: [u32; 3],
    selected: [u32; 2],
    capture_status: [u32; 4],
    output_tokens: [u32; 4],
    inactive: u32,
    uuid_success: u32,
    uuid_length: u32,
    uuid: [u8; 11],
    reserved: u8,
    creates: u32,
    calls: u32,
    input_attempts: u32,
    overflow: u32,
    event_count: u32,
    events: [ManagerEvent; 16],
}
#[cfg(feature = "macos-manager")]
fn manager_event(report: &mut ManagerReport, kind: u32, unit: u32, call: u32) {
    let index = usize::try_from(report.event_count).unwrap();
    report.events[index] = ManagerEvent {
        kind,
        unit,
        call,
        reserved: 0,
    };
    report.event_count += 1;
}
#[cfg(feature = "macos-manager")]
fn manager_model(script: ManagerScript) -> ManagerReport {
    use crate::backends::macos::manager::Manager;
    use std::{cell::RefCell, rc::Rc};
    struct Unit {
        id: u32,
        script: ManagerScript,
        report: Rc<RefCell<ManagerReport>>,
    }
    impl Drop for Unit {
        fn drop(&mut self) {
            manager_event(&mut self.report.borrow_mut(), 3, self.id, 0);
        }
    }
    fn capture(
        manager: &Manager<Unit>,
        output: &mut u32,
        report: &Rc<RefCell<ManagerReport>>,
        index: usize,
    ) {
        let result = manager.capture_with(output, |unit| {
            let mut ledger = report.borrow_mut();
            if index == 1 {
                ledger.selected[0] = unit.id;
            }
            if index == 3 {
                ledger.selected[1] = unit.id;
            }
            let call = ledger.calls;
            ledger.calls += 1;
            manager_event(&mut ledger, 2, unit.id, call);
            match unit.script.outcomes[usize::try_from(call).unwrap()] {
                0 => Ok(None),
                1 => Ok(Some(20 + call)),
                2 => Ok(Some(0)),
                3 => Err(()),
                _ => panic!("unknown script outcome"),
            }
        });
        let mut report = report.borrow_mut();
        report.capture_status[index] = match result {
            Ok(value) => u32::from(value),
            Err(()) => 2,
        };
        report.output_tokens[index] = *output;
    }
    let report = Rc::new(RefCell::new(ManagerReport::default()));
    let mut output = 99;
    {
        let mut manager = Manager::new(script.window_id, script.method);
        report.borrow_mut().connected[0] = u32::from(manager.connected());
        capture(&manager, &mut output, &report, 0);
        for phase in 0..2 {
            let result = manager.connect_with(script.available[phase] != 0, |window| {
                assert_eq!(window, script.window_id);
                let id = {
                    let mut ledger = report.borrow_mut();
                    ledger.creates += 1;
                    let id = ledger.creates;
                    manager_event(&mut ledger, 1, id, 0);
                    id
                };
                if script.fail_create[phase] != 0 {
                    return Err(());
                }
                Ok(Unit {
                    id,
                    script,
                    report: report.clone(),
                })
            });
            report.borrow_mut().connect_status[phase] = match result {
                Ok(value) => u32::from(value),
                Err(()) => 2,
            };
            report.borrow_mut().connected[phase + 1] = u32::from(manager.connected());
            capture(
                &manager,
                &mut output,
                &report,
                if phase == 0 { 1 } else { 3 },
            );
            if phase == 0 {
                report.borrow_mut().inactive = u32::from(manager.inactive());
                capture(&manager, &mut output, &report, 2);
            }
        }
        let mut uuid = String::new();
        report.borrow_mut().uuid_success = u32::from(manager.request_uuid(&mut uuid).unwrap());
        let mut ledger = report.borrow_mut();
        ledger.uuid_length = u32::try_from(uuid.len()).unwrap();
        ledger.uuid[..uuid.len()].copy_from_slice(uuid.as_bytes());
    }
    let final_report = report.borrow();
    *final_report
}

#[cfg(feature = "macos-manager")]
#[test]
#[ignore = "exact compiled original manager bodies, scripted backend/platform and real cv::Mat"]
fn matches_original_manager_connect_forwarding_output_and_owner_scripts() {
    type Run = unsafe extern "C" fn(*const ManagerScript, *mut ManagerReport) -> i32;
    let module = Module::open();
    // SAFETY: manager-evidence.h is the private authority; actual layout and
    // every scalar/array field are consumed and compared across 320 source cases.
    let run = unsafe { module.symbol::<Run>(c"mfs_mac_manager_script") };
    let layout = unsafe { module.symbol::<Layout>(c"mfs_mac_manager_layout") };
    let mut owner = unsafe { std::mem::zeroed::<libc::Dl_info>() };
    assert_ne!(unsafe { libc::dladdr(run as *const c_void, &mut owner) }, 0);
    assert!(!owner.dli_fname.is_null());
    let owner = unsafe { CStr::from_ptr(owner.dli_fname) };
    assert_eq!(
        std::fs::canonicalize(Path::new(std::ffi::OsStr::from_bytes(owner.to_bytes()))).unwrap(),
        std::fs::canonicalize(env::var_os("MFS_P1_MACOS_OPENCV_ADAPTER").unwrap()).unwrap()
    );
    let reports = PathBuf::from(env::var_os("MFS_P1_MACOS_MANAGER_SCRIPTS").unwrap());
    std::fs::create_dir_all(&reports).unwrap();
    for (index, expected) in [
        1,
        size_of::<ManagerScript>(),
        align_of::<ManagerScript>(),
        offset_of!(ManagerScript, method),
        offset_of!(ManagerScript, window_id),
        offset_of!(ManagerScript, available),
        offset_of!(ManagerScript, fail_create),
        offset_of!(ManagerScript, outcomes),
        size_of::<ManagerReport>(),
        align_of::<ManagerReport>(),
        offset_of!(ManagerReport, connect_status),
        offset_of!(ManagerReport, connected),
        offset_of!(ManagerReport, selected),
        offset_of!(ManagerReport, capture_status),
        offset_of!(ManagerReport, output_tokens),
        offset_of!(ManagerReport, inactive),
        offset_of!(ManagerReport, uuid_success),
        offset_of!(ManagerReport, uuid_length),
        offset_of!(ManagerReport, uuid),
        offset_of!(ManagerReport, reserved),
        offset_of!(ManagerReport, creates),
        offset_of!(ManagerReport, calls),
        offset_of!(ManagerReport, input_attempts),
        offset_of!(ManagerReport, overflow),
        offset_of!(ManagerReport, event_count),
        offset_of!(ManagerReport, events),
        size_of::<ManagerEvent>(),
        align_of::<ManagerEvent>(),
        offset_of!(ManagerEvent, kind),
        offset_of!(ManagerEvent, unit),
        offset_of!(ManagerEvent, call),
        offset_of!(ManagerEvent, reserved),
        usize::try_from(crate::backends::macos::manager::NONE).unwrap(),
        usize::try_from(crate::backends::macos::manager::SCREEN_CAPTURE_KIT).unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(unsafe { layout(u32::try_from(index).unwrap()) }, expected);
    }
    let mut cases = 0_usize;
    for method in [0, 1, 2, u64::MAX] {
        for available in 0..4 {
            for failure in 0..4 {
                for outcomes in [[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 1, 2], [1, 0, 3]] {
                    let script = ManagerScript {
                        method,
                        window_id: if cases.is_multiple_of(2) { u32::MAX } else { 0 },
                        available: [available & 1, available >> 1],
                        fail_create: [failure & 1, failure >> 1],
                        outcomes,
                    };
                    let mut original = ManagerReport::default();
                    assert_eq!(unsafe { run(&script, &mut original) }, 0);
                    assert_eq!(original.overflow, 0);
                    assert_eq!(original.input_attempts, 0);
                    let expected = manager_model(script);
                    std::fs::write(reports.join(format!("case-{cases}.txt")), format!(
                        "method={}\nwindow_id={}\navailable={:?}\nfail_create={:?}\noutcomes={:?}\noriginal={original:?}\nrust={expected:?}\n",
                        script.method, script.window_id, script.available, script.fail_create, script.outcomes
                    )).unwrap();
                    assert_eq!(original, expected, "manager case {cases}");
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 320);
    drop(module);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "original Rust macOS manager scripts=320 exact_bodies=1 output_and_unit_events=1 input_none=1 passed"
    );
}

#[cfg(feature = "macos-manager")]
#[test]
#[ignore = "owned native scene plus unchanged manager/source oracle and real OpenCV owners"]
fn forwards_manager_captures_and_keeps_outputs_through_manager_and_window_shutdown() {
    use crate::backends::macos::manager::{NativeManager, SCREEN_CAPTURE_KIT};
    let oracle_path = env::var_os("MFS_P1_MACOS_EXTERNAL_ORACLE").unwrap();
    let original = Path::new(&oracle_path).parent().unwrap();
    let directory = directory();
    let adapter = Adapter::open();
    adapter.warm();
    warm_capture(&adapter);
    let samples = held();
    let mut images = Vec::new();
    images.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        {
            let mut manager = NativeManager::new(scene(), SCREEN_CAPTURE_KIT);
            assert!(!manager.connected());
            assert!(manager.connect_native().unwrap());
            assert!(manager.connected());
            let mut uuid = String::new();
            assert!(manager.request_uuid(&mut uuid).unwrap());
            assert_eq!(uuid, scene().to_string());
            for call in 0..3 {
                let trace = trace();
                let mut output = Image::Empty;
                let result = objc2::rc::autoreleasepool(|_| {
                    manager.capture_native(
                        &mut output,
                        converter(&adapter, &trace, &samples, false, Duration::ZERO),
                        trace.clone(),
                    )
                })
                .unwrap();
                assert!(result);
                drained(&trace, true);
                assert_eq!(trace.source_failure.load(Ordering::Acquire), 0);
                trace.reclaim_source_semaphore_for_fixture();
                bgr(output.pixels());
                assert_eq!(
                    output.pixels(),
                    std::fs::read(original.join(format!("manager-cycle-{cycle}-call-{call}.bgr")))
                        .unwrap()
                );
                std::fs::write(
                    directory.join(format!("rust-manager-cycle-{cycle}-call-{call}.bgr")),
                    output.pixels(),
                )
                .unwrap();
                images.push(output);
                assert!(manager.inactive());
            }
        }
        for image in &images {
            bgr(image.pixels());
        }
    }
    let mut missing = NativeManager::new(u32::MAX, SCREEN_CAPTURE_KIT);
    assert!(missing.connect_native().unwrap());
    let missing_trace = trace();
    let mut output = images.pop().unwrap();
    let expected = output.pixels().to_vec();
    let expected_pointer = output.pixels().as_ptr();
    let expected_owners = adapter.owners();
    assert!(
        !objc2::rc::autoreleasepool(|_| missing.capture_native(
            &mut output,
            converter(&adapter, &missing_trace, &samples, false, Duration::ZERO),
            missing_trace.clone()
        ))
        .unwrap()
    );
    drained(&missing_trace, false);
    assert_eq!(missing_trace.source_failure.load(Ordering::Acquire), 2);
    assert_eq!(output.pixels(), expected);
    assert_eq!(output.pixels().as_ptr(), expected_pointer);
    assert_eq!(adapter.owners(), expected_owners);
    missing_trace.reclaim_source_semaphore_for_fixture();
    drop(missing);
    images.push(output);
    stop_scene();
    std::thread::spawn(move || {
        for image in images {
            bgr(image.pixels());
        }
    })
    .join()
    .unwrap();
    for sample in samples.lock().unwrap().iter() {
        raw(sample);
    }
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust macOS manager cycles=20 captures=60 retained_bgr=60 missing_output_preserved=1 fixture_reclaimed_refs=61 input_none=1 passed"
    );
}
