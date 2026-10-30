// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{io::Write, marker::PhantomData, rc::Rc};
#[path = "../../../xtask/src/macos_sample_cases.rs"]
mod cases;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Info {
    width: u64,
    height: u64,
    stride: u64,
    span: u64,
    data_size: u64,
    format: u64,
    owner_thread: u64,
    source_thread: u64,
    mutation_thread: u64,
}
type Create = unsafe extern "C" fn(u32, u32, *mut *mut c_void) -> i32;
type NativeInfo = unsafe extern "C" fn(*const c_void, *mut Info) -> i32;
type SampleBuffer = unsafe extern "C" fn(*const c_void) -> *const CMSampleBuffer;
type Save = unsafe extern "C" fn(*const c_void, *const libc::c_char) -> i32;
type Mutate = unsafe extern "C" fn(*mut c_void) -> i32;
type Source = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
type Counter = unsafe extern "C" fn(u32) -> u64;
type NativeError = unsafe extern "C" fn(u32) -> i32;
type Thread = unsafe extern "C" fn() -> u64;
struct Bridge {
    adapter: Arc<Adapter>,
    create: Create,
    info: NativeInfo,
    sample: SampleBuffer,
    save: Save,
    mutate: Mutate,
    destroy: Release,
    source: Source,
    source_view: NativeView,
    source_release: Release,
    counter: Counter,
    error: NativeError,
    release_thread: Counter,
    thread: Thread,
}
impl Bridge {
    fn open() -> Self {
        let adapter = Adapter::open_cpu();
        let m = &adapter.module;
        // SAFETY: These are the exact sample-fixture.h declarations, loaded
        // from the selected acceptance image retained by adapter. Layout and
        // actual native consumption are checked below; no SDK export is added.
        let result = unsafe {
            Self {
                adapter: adapter.clone(),
                create: m.symbol(c"mfs_mac_sample_create"),
                info: m.symbol(c"mfs_mac_sample_info"),
                sample: m.symbol(c"mfs_mac_sample_buffer"),
                save: m.symbol(c"mfs_mac_sample_save"),
                mutate: m.symbol(c"mfs_mac_sample_mutate"),
                destroy: m.symbol(c"mfs_mac_sample_destroy"),
                source: m.symbol(c"mfs_mac_sample_source"),
                source_view: m.symbol(c"mfs_mac_sample_source_view"),
                source_release: m.symbol(c"mfs_mac_sample_source_release"),
                counter: m.symbol(c"mfs_mac_sample_counter"),
                error: m.symbol(c"mfs_mac_sample_error"),
                release_thread: m.symbol(c"mfs_mac_sample_release_thread"),
                thread: m.symbol(c"mfs_mac_sample_thread"),
            }
        };
        let layout: Layout = unsafe { m.symbol(c"mfs_mac_sample_layout") };
        for (index, expected) in [
            1,
            size_of::<Info>(),
            align_of::<Info>(),
            offset_of!(Info, width),
            offset_of!(Info, height),
            offset_of!(Info, stride),
            offset_of!(Info, span),
            offset_of!(Info, data_size),
            offset_of!(Info, format),
            offset_of!(Info, owner_thread),
            offset_of!(Info, source_thread),
            offset_of!(Info, mutation_thread),
        ]
        .into_iter()
        .enumerate()
        {
            // SAFETY: Scalar layout query from the retained private module.
            assert_eq!(unsafe { layout(u32::try_from(index).unwrap()) }, expected);
        }
        assert_eq!(unsafe { (result.counter)(8) }, u64::MAX);
        let mut pointer = std::ptr::dangling_mut::<c_void>();
        assert_eq!(unsafe { (result.create)(4, 0, &mut pointer) }, 1);
        assert!(pointer.is_null());
        assert_eq!(result.counts(), [0; 8]);
        result
    }
    fn counts(&self) -> [u64; 8] {
        // SAFETY: Atomic scalar observations, retained code, bounded indices.
        std::array::from_fn(|i| unsafe { (self.counter)(u32::try_from(i).unwrap()) })
    }
    fn fixture(&self, case: usize, cycle: usize) -> Fixture<'_> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Valid fixed case/cycle, initialized output; successful call
        // transfers one native fixture owner, returned only through this guard.
        assert_eq!(
            unsafe {
                (self.create)(
                    u32::try_from(case).unwrap(),
                    u32::try_from(cycle).unwrap(),
                    &mut pointer,
                )
            },
            0
        );
        Fixture {
            pointer: NonNull::new(pointer).unwrap(),
            bridge: self,
            _channel: PhantomData,
        }
    }
}
struct Fixture<'a> {
    pointer: NonNull<c_void>,
    bridge: &'a Bridge,
    // No mobility claim for CF/native samples. Creation, access, mutation and
    // final native release remain on this test's actual caller channel.
    _channel: PhantomData<Rc<()>>,
}
impl Fixture<'_> {
    fn info(&self) -> Info {
        let mut value = Info::default();
        // SAFETY: Live native owner and initialized full private C record.
        assert_eq!(
            unsafe { (self.bridge.info)(self.pointer.as_ptr(), &mut value) },
            0
        );
        value
    }
    fn sample(&self) -> &CMSampleBuffer {
        // SAFETY: The bridge checks the real CF sample type. This borrow is
        // bounded by the live fixture, never retained into mutation/destruction.
        unsafe { (self.bridge.sample)(self.pointer.as_ptr()).as_ref() }.unwrap()
    }
    fn save(&self, path: &Path) {
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: Owner remains live; C borrows the native bytes only under its
        // own synchronous read lock and closes the file before returning.
        assert_eq!(
            unsafe { (self.bridge.save)(self.pointer.as_ptr(), path.as_ptr()) },
            0
        );
    }
    fn mutate(&mut self) {
        // SAFETY: Exclusive fixture access, no outstanding sample/pixel borrow;
        // the native helper pairs a write lock while replacing initialized data.
        assert_eq!(unsafe { (self.bridge.mutate)(self.pointer.as_ptr()) }, 0);
    }
    fn source(&mut self) -> CpuMat {
        let info = self.info();
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Live sample, same native owner channel. The unchanged source
        // converts under its original read lock; only its independent CPU Mat
        // moves to the returned owner. No second full-frame copy is added here.
        assert_eq!(
            unsafe { (self.bridge.source)(self.pointer.as_ptr(), &mut pointer) },
            0
        );
        CpuMat {
            pointer: NonNull::new(pointer).unwrap(),
            module: self.bridge.adapter.module.clone(),
            view: self.bridge.source_view,
            release: self.bridge.source_release,
            width: usize::try_from(info.width).unwrap(),
            height: usize::try_from(info.height).unwrap(),
        }
    }
}
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        // SAFETY: Unique fixture owner, original caller channel, no borrowed
        // sample/pixels survive. CF refs end synchronously; CPU owners remain.
        unsafe { (self.bridge.destroy)(self.pointer.as_ptr()) };
    }
}
#[test]
#[ignore = "requires native CoreVideo/CoreMedia and explicit original-source adapter"]
fn preserves_padded_native_samples_and_cpu_images_after_overwrite_and_native_release() {
    let directory = PathBuf::from(env::var_os("MFS_P1_MACOS_SAMPLE_ARTIFACTS").unwrap());
    let bridge = Bridge::open();
    bridge.adapter.warm();
    let caller = unsafe { (bridge.thread)() };
    assert_ne!(caller, 0);
    let mut rows = std::fs::File::create_new(directory.join("samples.tsv")).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(cases::CYCLES * cases::CASES.len())
        .unwrap();
    for cycle in 0..cases::CYCLES {
        for (case, &(width, height, alignment)) in cases::CASES.iter().enumerate() {
            let mut fixture = bridge.fixture(case, cycle);
            fixture.save(&directory.join(format!("case-{case}-{cycle}.native.raw")));
            let source = fixture.source();
            let evidence = pixel_buffer::Evidence::default();
            let (raw, bgr) = pixel_buffer::with_sample(fixture.sample(), &evidence, |sample| {
                assert_eq!(evidence.locks.load(Ordering::Acquire), 1);
                assert_eq!(evidence.unlocks.load(Ordering::Acquire), 0);
                let sample = sample.unwrap();
                let bgr =
                    bridge
                        .adapter
                        .convert(sample.pixels(), sample.width, sample.height, false)?;
                Ok((sample, bgr))
            })
            .unwrap();
            assert_eq!((raw.width, raw.height), (width, height));
            let expected = cases::raw(case, cycle);
            let expected_bgr = cases::bgr(&expected);
            assert_eq!(raw.pixels(), expected);
            assert_eq!(source.pixels(), expected_bgr);
            assert_eq!(bgr.pixels(), expected_bgr);
            std::fs::write(
                directory.join(format!("case-{case}-{cycle}.rust.raw")),
                raw.pixels(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("case-{case}-{cycle}.source.bgr")),
                source.pixels(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("case-{case}-{cycle}.rust.bgr")),
                bgr.pixels(),
            )
            .unwrap();
            fixture.mutate();
            fixture.save(&directory.join(format!("case-{case}-{cycle}.mutated.raw")));
            let info = fixture.info();
            assert_eq!(info.owner_thread, caller);
            assert_eq!(info.source_thread, caller);
            assert_eq!(info.mutation_thread, caller);
            assert!(info.stride > info.width.checked_mul(4).unwrap());
            assert_eq!(info.stride % u64::try_from(alignment).unwrap(), 0);
            assert_eq!(raw.native_stride, usize::try_from(info.stride).unwrap());
            assert_eq!(
                info.format,
                u64::from(objc2_core_video::kCVPixelFormatType_32BGRA)
            );
            assert_eq!(
                (
                    evidence.locks.load(Ordering::Acquire),
                    evidence.unlocks.load(Ordering::Acquire),
                    evidence.copies.load(Ordering::Acquire),
                    evidence.conversions.load(Ordering::Acquire)
                ),
                (1, 1, 1, 1)
            );
            assert_eq!(evidence.unlock_error.load(Ordering::Acquire), 0);
            drop(fixture);
            let count = u64::try_from(held.len() + 1).unwrap();
            assert_eq!(bridge.counts()[0..2], [count, count]);
            writeln!(
                rows,
                "{cycle}\t{case}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t1",
                info.width,
                info.height,
                info.stride,
                info.span,
                info.data_size,
                info.format,
                info.owner_thread,
                info.source_thread,
                info.mutation_thread,
                evidence.locks.load(Ordering::Acquire),
                evidence.unlocks.load(Ordering::Acquire),
                evidence.copies.load(Ordering::Acquire),
                evidence.copied_bytes.load(Ordering::Acquire),
                evidence.conversions.load(Ordering::Acquire),
                evidence.unlock_error.load(Ordering::Acquire)
            )
            .unwrap();
            rows.flush().unwrap();
            // Native bytes were actually zeroed and all fixture refs ended first.
            assert_eq!(raw.pixels(), expected);
            assert_eq!(source.pixels(), expected_bgr);
            assert_eq!(bgr.pixels(), expected_bgr);
            held.push((case, cycle, raw, bgr, source));
        }
    }
    let output = directory.clone();
    let thread = bridge.thread;
    let reader = std::thread::spawn(move || {
        let reader = unsafe { thread() };
        for (case, cycle, raw, bgr, source) in held {
            std::fs::write(
                output.join(format!("case-{case}-{cycle}.held.raw")),
                raw.pixels(),
            )
            .unwrap();
            std::fs::write(
                output.join(format!("case-{case}-{cycle}.held.source.bgr")),
                source.pixels(),
            )
            .unwrap();
            std::fs::write(
                output.join(format!("case-{case}-{cycle}.held.rust.bgr")),
                bgr.pixels(),
            )
            .unwrap();
            let expected = cases::raw(case, cycle);
            let expected_bgr = cases::bgr(&expected);
            assert_eq!(raw.pixels(), expected);
            assert_eq!(source.pixels(), expected_bgr);
            assert_eq!(bgr.pixels(), expected_bgr);
        }
        reader
    })
    .join()
    .unwrap();
    assert_ne!(reader, caller);
    let mut releases = std::fs::File::create_new(directory.join("releases.tsv")).unwrap();
    for cycle in 0..cases::CYCLES {
        for case in 0..cases::CASES.len() {
            let thread =
                unsafe { (bridge.release_thread)(u32::try_from(cycle * 4 + case).unwrap()) };
            assert_eq!(thread, reader);
            writeln!(releases, "{cycle}\t{case}\t{thread}").unwrap();
        }
    }
    let counters = bridge.counts();
    assert_eq!(counters, [80, 80, 80, 80, 80, 80, 160, 160]);
    assert_eq!(unsafe { (bridge.error)(0) }, 0);
    assert_eq!(unsafe { (bridge.error)(1) }, 0);
    assert_eq!(bridge.adapter.owners(), 0);
    std::fs::write(
        directory.join("counters.tsv"),
        format!(
            "{}\t0\t0\t{caller}\t{reader}\t0\n",
            counters.map(|n| n.to_string()).join("\t")
        ),
    )
    .unwrap();
    drop(bridge);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(MODULE_CLOSED.load(Ordering::Acquire), 1);
    std::fs::write(directory.join("module.tsv"), "1\t0\n").unwrap();
    eprintln!(
        "native macOS sample_matrix cases=4 cycles=20 samples=80 source_images=80 rust_raw=80 rust_bgr=80 retained_cpu=240 native_overwrite=80 native_refs_closed=80 cross_thread_cpu_release=1 capture_permission_requested=0 passed"
    );
}
