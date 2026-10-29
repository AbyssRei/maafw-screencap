// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Actual private bridge.h consumer; no C++ class calls across Rust directly.
use super::*;
use crate::{
    backends::windows::{delegate::CaptureDelegate, parity_adapter::Adapter},
    test_support::native_module::Module,
};
use std::{
    mem::{align_of, offset_of, size_of},
    path::PathBuf,
    sync::Arc,
};
#[path = "delegate_native_host_tests.rs"]
mod host;
#[cfg(feature = "prototype-windows-com")]
#[path = "delegate_runtime_tests.rs"]
mod runtime;
#[derive(Default)]
#[repr(C)]
struct Span {
    owner: *mut c_void,
    data: *const u8,
    length: usize,
    width: i32,
    height: i32,
    stride: usize,
}
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
struct Event {
    kind: u32,
    thread: u32,
    a: u64,
    b: u64,
    c: u64,
    d: u64,
}
type Create = unsafe extern "C" fn(usize, usize, u64, u64, u64, *mut *mut c_void) -> i32;
type Destroy = unsafe extern "C" fn(usize, *mut c_void) -> i32;
type Call = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;
type Capture = unsafe extern "C" fn(*mut c_void, *mut Span, *mut i32) -> i32;
type Release = unsafe extern "C" fn(*mut c_void);
struct Ops {
    module: Arc<Module>,
    create: Create,
    destroy: Destroy,
    connect: Call,
    capture: Capture,
    inactive: Call,
    release: Release,
}
struct Image {
    span: Span,
    module: Arc<Module>,
    release: Release,
}
impl Image {
    fn bytes(&self) -> &[u8] {
        if self.span.owner.is_null() {
            assert_eq!(self.span.length, 0);
            return &[];
        }
        let span = &self.span;
        assert!(span.width > 0 && span.height > 0 && !span.data.is_null());
        assert_eq!(
            span.stride,
            usize::try_from(span.width).unwrap().checked_mul(3).unwrap()
        );
        assert_eq!(
            span.length,
            span.stride
                .checked_mul(usize::try_from(span.height).unwrap())
                .unwrap()
        );
        assert!(span.length <= isize::MAX as usize);
        // SAFETY: Actual bridge header/layout verified on load. Bridge owns
        // exactly this initialized contiguous vector until release. &mut image
        // is required for capture; no shared view can overlap a native write.
        unsafe { std::slice::from_raw_parts(span.data, span.length) }
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: Exactly one bridge-owned output; this module owner remains
        // live through its release and never retains the provider module.
        unsafe { (self.release)(self.span.owner) };
    }
}
impl Ops {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let module = Arc::try_new(Module::load(path.as_os_str())?)?;
        // SAFETY: Exact private bridge.h signatures; every pointer user owns
        // the bridge module. Verify actual C++ layout before any native owner.
        unsafe {
            let layout: unsafe extern "C" fn(*mut usize, usize) -> i32 =
                module.symbol(c"mfs_dn_layout")?;
            let mut actual = [0; 18];
            assert_eq!(layout(actual.as_mut_ptr(), actual.len()), 0);
            assert_eq!(
                actual,
                [
                    size_of::<Span>(),
                    align_of::<Span>(),
                    offset_of!(Span, owner),
                    offset_of!(Span, data),
                    offset_of!(Span, length),
                    offset_of!(Span, width),
                    offset_of!(Span, height),
                    offset_of!(Span, stride),
                    size_of::<Event>(),
                    align_of::<Event>(),
                    size_of::<host::Table>(),
                    align_of::<host::Table>(),
                    offset_of!(host::Table, user),
                    offset_of!(host::Table, create),
                    offset_of!(host::Table, destroy),
                    offset_of!(host::Table, connect),
                    offset_of!(host::Table, capture),
                    offset_of!(host::Table, inactive)
                ]
            );
            Ok(Self {
                create: module.symbol(c"mfs_dn_create")?,
                destroy: module.symbol(c"mfs_dn_destroy")?,
                connect: module.symbol(c"mfs_dn_connect")?,
                capture: module.symbol(c"mfs_dn_capture")?,
                inactive: module.symbol(c"mfs_dn_inactive")?,
                release: module.symbol(c"mfs_dn_release")?,
                module,
            })
        }
    }
    fn image(&self) -> Image {
        Image {
            span: Span::default(),
            module: self.module.clone(),
            release: self.release,
        }
    }
    fn mode(&self, value: u32) {
        // SAFETY: Exact synchronous bridge header setter; no borrowed storage.
        let set: unsafe extern "C" fn(u32) =
            unsafe { self.module.symbol(c"mfs_dn_set_mode").unwrap() };
        unsafe { set(value) };
    }
    fn reset(&self) {
        // SAFETY: Exact header; rejects live output owners before resetting.
        let reset: unsafe extern "C" fn() -> i32 =
            unsafe { self.module.symbol(c"mfs_dn_reset").unwrap() };
        assert_eq!(unsafe { reset() }, 0);
    }
    fn events(&self) -> (Vec<Event>, u64) {
        type Read = unsafe extern "C" fn(*mut Event, usize, *mut usize, *mut u64) -> i32;
        // SAFETY: Exact header and initialized capacity. Overflow is rejected.
        let read: Read = unsafe { self.module.symbol(c"mfs_dn_events").unwrap() };
        let mut events = vec![Event::default(); 4096];
        let mut count = 0;
        let mut owners = 0;
        assert_eq!(
            unsafe { read(events.as_mut_ptr(), events.len(), &mut count, &mut owners) },
            0
        );
        assert!(count <= events.len());
        events.truncate(count);
        (events, owners)
    }
}
// SAFETY: Addresses always invoke the original factory type in C++ with all
// exceptions contained. Each synchronous virtual call uses actual class ABI.
// Bridge image bytes outlive the provider; Image retains only bridge code.
unsafe impl UnitOps for Ops {
    type Image = Image;
    unsafe fn create(
        &self,
        function: usize,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<NonNull<c_void>>, i32> {
        let mut unit = std::ptr::null_mut();
        // SAFETY: UnitOps caller proves live export address; exact bridge ABI.
        let status = unsafe { (self.create)(function, hwnd, method, mouse, keyboard, &mut unit) };
        if status == 0 {
            Ok(NonNull::new(unit))
        } else {
            assert!(unit.is_null());
            Err(status)
        }
    }
    unsafe fn destroy(&self, function: usize, unit: NonNull<c_void>) -> i32 {
        // SAFETY: Caller retains real unit/module through bridge destruction.
        unsafe { (self.destroy)(function, unit.as_ptr()) }
    }
    unsafe fn connect(&self, unit: NonNull<c_void>) -> Result<bool, i32> {
        let mut result = 0;
        // SAFETY: Live unit, exact bridge ABI, synchronous initialized output.
        let code = unsafe { (self.connect)(unit.as_ptr(), &mut result) };
        if code == 0 {
            Ok(result != 0)
        } else {
            Err(code)
        }
    }
    unsafe fn capture(&self, unit: NonNull<c_void>, output: &mut Image) -> Result<bool, i32> {
        assert!(Arc::ptr_eq(&self.module, &output.module));
        let mut result = 0;
        // SAFETY: Live unit; exclusive valid image including post-write errors.
        let code = unsafe { (self.capture)(unit.as_ptr(), &mut output.span, &mut result) };
        let _ = output.bytes();
        if code == 0 {
            Ok(result != 0)
        } else {
            Err(code)
        }
    }
    unsafe fn inactive(&self, unit: NonNull<c_void>) -> Result<bool, i32> {
        let mut result = 0;
        // SAFETY: Live unit and initialized synchronous output, exact header.
        let code = unsafe { (self.inactive)(unit.as_ptr(), &mut result) };
        if code == 0 {
            Ok(result != 0)
        } else {
            Err(code)
        }
    }
}
fn expected(bytes: &[u8]) {
    assert_eq!(bytes.len(), 37 * 23 * 3);
    for y in 0..23 {
        for x in 0..37 {
            let pixel = if x < 37 / 3 {
                [0x1f, 0x53, 0xa7]
            } else if y < 23 / 2 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            assert_eq!(&bytes[(y * 37 + x) * 3..(y * 37 + x + 1) * 3], &pixel);
        }
    }
}
#[test]
#[ignore = "requires explicitly built real delegate DLLs, bridge, source executable and live desktop"]
fn consumes_real_factory_and_releases_units_before_unloading_modules()
-> Result<(), Box<dyn std::error::Error>> {
    let build = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD")
            .ok_or("explicit native build directory required")?,
    );
    let records = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_RECORDS")
            .ok_or("new external records directory required")?,
    );
    assert!(build.is_absolute() && records.is_absolute());
    std::fs::create_dir(&records)?;
    let build = std::fs::canonicalize(build)?;
    let records = std::fs::canonicalize(records)?;
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))
        .map_err(|error| format!("bridge load: {error}"))?;
    ops.reset();
    let host = host::Guard::new(&ops);
    let adapter = Adapter::load().map_err(|error| format!("window adapter load: {error}"))?;
    let restore = crate::platform::windows::manager::probe::current();
    struct Restore(isize);
    impl Drop for Restore {
        fn drop(&mut self) {
            assert!(crate::platform::windows::manager::probe::set(self.0));
        }
    }
    let _restore = Restore(restore);
    crate::platform::windows::manager::set_per_monitor_v2()
        .map_err(|e| format!("fixture DPI {e}"))?;
    let mut window = adapter
        .window(37, 23)
        .map_err(|error| format!("fixture create {error}"))?;
    let thread = crate::platform::windows::manager::probe::thread();
    let object_baseline = adapter.objects();
    let mut held = Vec::new();
    let mut cycles = String::new();
    // Three real filesystem names exercise all Boost append_decorations
    // attempts. There is no PATH/global loader-search mutation.
    for (style, name, binary) in [
        (
            "normal",
            "MaaWin32ControlUnit.dll",
            "mfs_delegate_source.dll",
        ),
        (
            "prefixed",
            "libMaaWin32ControlUnit.dll",
            "mfs_delegate_source.dll",
        ),
        ("bare", "MaaWin32ControlUnit", "mfs_delegate_source.dll"),
        ("rust", "MaaWin32ControlUnit.dll", "mfs_delegate_rust.dll"),
    ] {
        let directory = records.join(style);
        std::fs::create_dir(&directory)?;
        let provider = directory.join(name);
        std::fs::copy(build.join(binary), &provider)?;
        let observation = if style == "bare" {
            directory.join("MaaWin32ControlUnit.")
        } else {
            provider.clone()
        };
        assert!(!library::loaded(&observation));
        for cycle in 0..20 {
            let mut first = CaptureDelegate::new(
                Native::new(&directory, &ops).unwrap(),
                window.hwnd() as usize,
                1,
            );
            let mut second = CaptureDelegate::new(
                Native::new(&directory, &ops).unwrap(),
                window.hwnd() as usize,
                1,
            );
            assert_eq!(first.initialize(), Ok(Some(true)));
            assert!(library::loaded(&observation));
            if style == "normal" && cycle == 0 {
                let mut different = Native::new(&records.join("different"), &ops).unwrap();
                let mut unused = different.new_loader().unwrap();
                assert!(!different.load(&mut unused).unwrap());
                assert_eq!(
                    different.last_load_error(),
                    Some(library::Error::DifferentPath)
                );
                drop(unused);
                assert!(library::loaded(&observation));
            }
            assert_eq!(second.initialize(), Ok(Some(true)));
            let mut image = ops.image();
            assert_eq!(first.capture(&mut image), Ok(true));
            expected(image.bytes());
            drop(first);
            let shared = library::loaded(&observation);
            assert!(shared);
            let mut other = ops.image();
            assert_eq!(second.capture(&mut other), Ok(true));
            expected(other.bytes());
            assert_eq!(image.bytes(), other.bytes());
            assert_eq!(second.inactive(), Ok(true));
            drop(second);
            let unloaded = !library::loaded(&observation);
            assert!(unloaded);
            expected(image.bytes());
            expected(other.bytes());
            std::fs::write(
                records.join(format!("{style}-{cycle}-first.bgr")),
                image.bytes(),
            )?;
            std::fs::write(
                records.join(format!("{style}-{cycle}-second.bgr")),
                other.bytes(),
            )?;
            held.push(image);
            held.push(other);
            let objects = adapter.objects();
            assert_eq!(objects, object_baseline);
            use std::fmt::Write;
            writeln!(
                cycles,
                "{style}\t{cycle}\t{}\t{}\t{object_baseline}\t{objects}",
                u8::from(shared),
                u8::from(unloaded)
            )?;
            std::fs::write(records.join("cycles.tsv"), &cycles)?;
        }
        if style == "rust" {
            let reference = std::fs::read(records.join("normal-source.bgr"))?;
            for image in &held[120..] {
                assert_eq!(image.bytes(), reference);
            }
            continue;
        }
        // Original executable independently uses REAL Boost/LibraryHolder,
        // original loader/outer bodies and the same real provider/interface.
        let source_output = records.join(format!("{style}-source.bgr"));
        let output = std::process::Command::new(build.join("mfs_delegate_native_source.exe"))
            .args([
                directory.as_os_str(),
                std::ffi::OsStr::new(&(window.hwnd() as usize).to_string()),
                source_output.as_os_str(),
            ])
            .output()?;
        std::fs::write(
            records.join(format!("{style}-source.stdout")),
            &output.stdout,
        )?;
        std::fs::write(
            records.join(format!("{style}-source.stderr")),
            &output.stderr,
        )?;
        assert!(
            output.status.success(),
            "source process {:?}",
            output.status
        );
        expected(&std::fs::read(source_output)?);
        assert!(!library::loaded(&observation));
    }
    // Real missing modules/exports produce false and release every load ref.
    for fixture in ["absent", "missing_create", "missing_destroy"] {
        let directory = records.join(fixture);
        std::fs::create_dir(&directory)?;
        let path = directory.join("MaaWin32ControlUnit.dll");
        if fixture != "absent" {
            std::fs::copy(build.join(format!("mfs_delegate_{fixture}.dll")), &path)?;
        }
        let mut native = Native::new(&directory, &ops).unwrap();
        let mut loader = native.new_loader().unwrap();
        assert!(!native.load(&mut loader).unwrap());
        if fixture == "absent" {
            assert!(matches!(
                native.last_load_error(),
                Some(library::Error::Kernel(_))
            ));
        }
        drop(loader);
        assert!(!library::loaded(&path));
    }
    let normal = records.join("normal");
    // Contained real C++ exceptions and direct output mutation after false/error.
    let mut delegate = CaptureDelegate::new(
        Native::new(&normal, &ops).unwrap(),
        window.hwnd() as usize,
        1,
    );
    ops.mode(1);
    assert_eq!(
        delegate.initialize(),
        Err(super::super::delegate::Error::Boundary(Error::Bridge(2)))
    );
    drop(delegate);
    assert!(!library::loaded(&normal.join("MaaWin32ControlUnit.dll")));
    ops.mode(0);
    let mut delegate = CaptureDelegate::new(
        Native::new(&normal, &ops).unwrap(),
        window.hwnd() as usize,
        1,
    );
    assert_eq!(delegate.initialize(), Ok(Some(true)));
    let mut output = ops.image();
    assert_eq!(delegate.capture(&mut output), Ok(true));
    expected(output.bytes());
    ops.mode(3);
    assert_eq!(delegate.capture(&mut output), Ok(false));
    assert_eq!(output.bytes(), [7, 9, 11].repeat(6));
    std::fs::write(records.join("post-false.bgr"), output.bytes())?;
    ops.mode(4);
    assert_eq!(
        delegate.capture(&mut output),
        Err(super::super::delegate::Error::Boundary(Error::Bridge(2)))
    );
    assert_eq!(output.bytes(), [7, 9, 11].repeat(6));
    std::fs::write(records.join("post-exception.bgr"), output.bytes())?;
    ops.mode(2);
    assert_eq!(
        delegate.initialize(),
        Err(super::super::delegate::Error::Boundary(Error::Bridge(2)))
    );
    drop(delegate);
    drop(output);
    ops.mode(0);
    assert!(!library::loaded(&normal.join("MaaWin32ControlUnit.dll")));
    window
        .close()
        .map_err(|error| format!("fixture close {error}"))?;
    drop(window);
    drop(adapter);
    for image in &held {
        expected(image.bytes());
    }
    let (events, owners) = ops.events();
    assert_eq!(owners, 160);
    assert!(events.iter().all(|event| event.thread == thread));
    let created = events.iter().filter(|event| event.kind == 1).count();
    let destroyed = events.iter().filter(|event| event.kind == 5).count();
    assert_eq!(created, 163);
    assert_eq!(destroyed, 162); // one create threw before ownership
    assert!(!events.iter().any(|event| event.kind == 99));
    host.verify(40, 40, 20);
    for event in events.iter().filter(|event| event.kind == 1) {
        assert_eq!(
            (event.a, event.b, event.c, event.d),
            (window_handle(&events), 1, 0, 0)
        );
    }
    std::fs::write(
        records.join("events.txt"),
        format!("{events:#?}\nretained_owners={owners}\n"),
    )?;
    let mut table = String::new();
    for event in &events {
        use std::fmt::Write;
        writeln!(
            table,
            "{}\t{}\t{}\t{}\t{}\t{}",
            event.kind, event.thread, event.a, event.b, event.c, event.d
        )?;
    }
    std::fs::write(records.join("events.tsv"), table)?;
    let host_counts = host.counts();
    drop(held);
    let after = ops.events().1;
    assert_eq!(after, 0);
    assert_eq!(DESTROY_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    std::fs::write(
        records.join("summary.json"),
        format!(
            "{{\"layout_version\":1,\"thread\":{thread},\"hwnd\":{},\"owners_before_release\":{owners},\"owners_after_release\":{after},\"created\":{created},\"destroyed\":{destroyed},\"host_created\":{},\"host_destroyed\":{},\"host_conversions\":{},\"host_captures\":{},\"host_inactive\":{}}}\n",
            window_handle(&events),
            host_counts[0],
            host_counts[1],
            host_counts[2],
            host_counts[3],
            host_counts[4]
        ),
    )?;
    println!(
        "real delegate native lifecycles=80; actual retained images=160; original Boost DLL paths=3; missing modules/exports=3; contained exception/output controls=5; actual Rust manager/GDI units=40; host conversions=120; full factories/framework pending"
    );
    Ok(())
}
fn window_handle(events: &[Event]) -> u64 {
    events.iter().find(|event| event.kind == 1).unwrap().a
}

#[test]
#[ignore = "requires explicit real provider DLLs and acceptance converter; construction only, no capture acceptance"]
fn constructs_original_selector_masks_without_connecting_or_capturing()
-> Result<(), Box<dyn std::error::Error>> {
    let build = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD").ok_or("explicit native build required")?,
    ))?;
    let records = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_FACTORY_RECORDS")
            .ok_or("new external factory records required")?,
    );
    assert!(records.is_absolute());
    std::fs::create_dir(&records)?;
    let records = std::fs::canonicalize(records)?;
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))?;
    ops.reset();
    let host = host::Guard::new(&ops);
    let mut cases = String::new();
    for name in ["source", "rust"] {
        let directory = records.join(name);
        std::fs::create_dir(&directory)?;
        let module = directory.join("MaaWin32ControlUnit.dll");
        std::fs::copy(build.join(format!("mfs_delegate_{name}.dll")), &module)?;
        // The original API constructor stores these selectors/handles without
        // connecting or validating the target. The high value is NEVER passed
        // to a native window operation or dereferenced by either implementation.
        for hwnd in [0, 0xfedc_ba98_7654_3210usize] {
            for method in [0, 1, 2, 4, 8, 16, 32, 63, u64::MAX] {
                assert!(!library::loaded(&module));
                let mut boundary = Native::new(&directory, &ops).unwrap();
                let mut loader = boundary.new_loader().unwrap();
                assert!(boundary.load(&mut loader).unwrap());
                let unit = boundary.create(&mut loader, hwnd, method, 0, 0).unwrap();
                assert!(unit.is_some());
                drop(loader);
                let unloaded = !library::loaded(&module);
                assert!(unloaded);
                use std::fmt::Write;
                writeln!(
                    cases,
                    "{name}\t{hwnd}\t{method}\t{}\t{}",
                    u8::from(unit.is_some()),
                    u8::from(unloaded)
                )?;
                std::fs::write(records.join("cases.tsv"), &cases)?;
            }
        }
    }
    assert_eq!(host.counts(), [18, 18, 0, 0, 0]);
    let counts = host.counts();
    std::fs::write(
        records.join("host-counts.tsv"),
        format!(
            "{}\t{}\t{}\t{}\t{}\n",
            counts[0], counts[1], counts[2], counts[3], counts[4]
        ),
    )?;
    let (events, owners) = ops.events();
    assert_eq!(owners, 0);
    assert_eq!(events.len(), 72);
    let mut table = String::new();
    let mut index = 0;
    for _name in ["source", "rust"] {
        for hwnd in [0, 0xfedc_ba98_7654_3210u64] {
            for method in [0, 1, 2, 4, 8, 16, 32, 63, u64::MAX] {
                let create = events[index];
                let destroy = events[index + 1];
                index += 2;
                assert_eq!(
                    (create.kind, create.a, create.b, create.c, create.d),
                    (1, hwnd, method, 0, 0)
                );
                assert_eq!(
                    (destroy.kind, destroy.a, destroy.b, destroy.c, destroy.d),
                    (5, 0, 0, 0, 0)
                );
            }
        }
    }
    for event in &events {
        use std::fmt::Write;
        assert_eq!(
            event.thread,
            crate::platform::windows::manager::probe::thread()
        );
        writeln!(
            table,
            "{}\t{}\t{}\t{}\t{}\t{}",
            event.kind, event.thread, event.a, event.b, event.c, event.d
        )?;
    }
    std::fs::write(records.join("events.tsv"), table)?;
    assert_eq!(DESTROY_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    println!(
        "real factory selector cases=36; native Rust manager contexts=18; actual captures=0; backend creation on connect not exercised; module/unit ownership closed=1"
    );
    Ok(())
}
