// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Consume real original C creation/destruction and actual public C++ vtables.
//! Rust callbacks route only capture/identity/info to the same private manager.
use super::*;
use metadata::{Info, Objects};
use std::{
    ffi::{CStr, c_char},
    io::Write,
    path::Path,
};
type Dispatch =
    unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut *mut c_void, *mut u64) -> i32;
type Release = unsafe extern "C" fn(*mut c_void);
pub(super) struct Api {
    version: unsafe extern "C" fn() -> *const c_char,
    abi: unsafe extern "C" fn(u32) -> usize,
    source: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    rust: unsafe extern "C" fn(
        Option<Dispatch>,
        Option<Release>,
        *mut c_void,
        *mut *mut c_void,
    ) -> i32,
    destroy: unsafe extern "C" fn(u8, *mut c_void) -> i32,
    query: unsafe extern "C" fn(*mut c_void, u32, *mut c_void, *mut u64) -> i32,
    bases: unsafe extern "C" fn(*mut c_void) -> u32,
    count: unsafe extern "C" fn(u32) -> u64,
    fault: unsafe extern "C" fn(*mut c_void, u32) -> i32,
    publish_image: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    publish_uuid: unsafe extern "C" fn(*mut c_void, *mut OpaqueString) -> i32,
    publication_probe: unsafe extern "C" fn() -> u32,
}
impl Api {
    pub(super) fn open(module: &loader::Module) -> Self {
        // SAFETY: Exact native private header declarations; Lib retains module
        // through all actual unit, callback state and native value destruction.
        unsafe {
            Self {
                version: module.symbol(c"MaaCustomControlUnitGetVersion"),
                abi: module.symbol(c"mfs_custom_unit_abi"),
                source: module.symbol(c"mfs_custom_unit_source_create"),
                rust: module.symbol(c"mfs_custom_unit_rust_create"),
                destroy: module.symbol(c"mfs_custom_unit_destroy"),
                query: module.symbol(c"mfs_custom_unit_query"),
                bases: module.symbol(c"mfs_custom_unit_bases"),
                count: module.symbol(c"mfs_custom_unit_count"),
                fault: module.symbol(c"mfs_custom_unit_fault"),
                publish_image: module.symbol(c"mfs_custom_unit_publish_image"),
                publish_uuid: module.symbol(c"mfs_custom_unit_publish_uuid"),
                publication_probe: module.symbol(c"mfs_custom_unit_publication_probe"),
            }
        }
    }
}
impl Native<'_> {
    pub(super) fn publish_cpp_image(&self, image: &mut Image<'_>) -> Result<(), i32> {
        let (op, target) = self.unit_destination.get();
        if target.is_null() {
            return Ok(());
        }
        if op != 5 {
            return Err(1);
        }
        // SAFETY: Exact caller Mat borrowed by the C++ virtual method through
        // this synchronous dispatch; no data view aliases it. Native scope
        // validation precedes assignment and pixel storage leaves this head.
        status(unsafe { (self.lib.unit.publish_image)(target, image.pointer.as_ptr()) })
    }
    pub(super) fn publish_cpp_uuid(&self, buffer: &identity::UuidBuffer) -> Result<(), i32> {
        let (op, target) = self.unit_destination.get();
        if target.is_null() {
            return Ok(());
        }
        if op != 4 {
            return Err(1);
        }
        // SAFETY: Exact borrowed C++ string and live actual StringBuffer.
        // Original string assignment happens before that buffer is destroyed.
        status(unsafe { (self.lib.unit.publish_uuid)(target, buffer.0.as_ptr()) })
    }
}
struct Destination<'a> {
    cell: &'a Cell<(u32, *mut c_void)>,
    previous: (u32, *mut c_void),
}
impl Drop for Destination<'_> {
    fn drop(&mut self) {
        self.cell.set(self.previous);
    }
}
fn status(code: i32) -> Result<(), i32> {
    if code == 0 { Ok(()) } else { Err(code) }
}
struct Host<'a, 'l> {
    manager: Manager<'a, Native<'l>>,
    native: &'a Native<'l>,
    dropped: Cell<u32>,
    operations: Cell<u32>,
    panic: Cell<bool>,
    thread: ThreadId,
}
unsafe extern "C" fn dispatch(
    argument: *mut c_void,
    operation: u32,
    destination: *mut c_void,
    output: *mut *mut c_void,
    value: *mut u64,
) -> i32 {
    if output.is_null() || value.is_null() {
        return 1;
    }
    // SAFETY: Initialized full C++ UnitResult outputs, exclusively writable
    // during this synchronous dispatch; failure transfers no native owner.
    unsafe {
        *output = std::ptr::null_mut();
        *value = 0;
    }
    // SAFETY: Exact stable Box<Host> lives until actual unit destruction returns.
    let host = unsafe { &*argument.cast::<Host<'_, '_>>() };
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), i32> {
        assert_eq!(std::thread::current().id(), host.thread);
        assert_eq!(host.dropped.get(), 0);
        host.operations
            .set(host.operations.get().checked_add(1).unwrap());
        if matches!(operation, 4 | 5) == destination.is_null() {
            return Err(1);
        }
        // Borrow destinations only until this dispatch ends; restore on every
        // return, error or contained panic without retaining a native pointer.
        let _destination = Destination {
            cell: &host.native.unit_destination,
            previous: host
                .native
                .unit_destination
                .replace((operation, destination)),
        };
        let lib = host.native.lib;
        let scalar = match operation {
            0 => u64::from(host.manager.connect()?),
            1 => u64::from(host.manager.connected()?),
            2 => u64::from(host.manager.inactive()?),
            3 => host.manager.get_features()?,
            4 => {
                let mut bytes = Vec::new();
                if !host.manager.request_uuid(&mut bytes)? {
                    return Ok(());
                }
                // Actual C++ string was assigned by publish_uuid inside the
                // StringBuffer scope. This independent Vec is the disclosed
                // extra acceptance representation copy, and can now end.
                1
            }
            5 => {
                let mut object = std::ptr::null_mut();
                // SAFETY: Null buffer explicitly creates an empty actual Mat
                // head. Only acceptance bookkeeping allocates here, no pixels.
                status(unsafe { (lib.image)(std::ptr::null_mut(), &mut object) })?;
                let mut image = Image {
                    pointer: NonNull::new(object).ok_or(1)?,
                    lib,
                };
                if !host.manager.capture(&mut image)? {
                    return Ok(());
                }
                // Actual caller already owns storage before buffer teardown;
                // publish_image leaves only an empty bookkeeping Mat head.
                assert!(image.pixels().is_empty());
                1
            }
            6 => {
                let info = host.manager.get_info(&Objects(lib))?;
                let pointer = info.pointer.as_ptr();
                std::mem::forget(info);
                // SAFETY: Sole actual json::object head transfers; C++ moves
                // its owned value into the real virtual return and releases it.
                unsafe {
                    *output = pointer;
                }
                1
            }
            _ => return Err(1),
        };
        // SAFETY: Full writable output, no more fallible work after transfer.
        unsafe {
            *value = scalar;
        }
        Ok(())
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(code)) => code,
        Err(_) => {
            host.panic.set(true);
            -100
        }
    }
}
unsafe extern "C" fn release(argument: *mut c_void) {
    // SAFETY: Actual C++ virtual destructor calls once before destroy returns;
    // host Box is still retained by the Rust owner and cannot move or end yet.
    let host = unsafe { &*argument.cast::<Host<'_, '_>>() };
    if catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), host.thread);
        host.dropped.set(host.dropped.get().checked_add(1).unwrap());
    }))
    .is_err()
    {
        host.panic.set(true);
    }
}
struct Unit<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
    rust: bool,
}
impl Drop for Unit<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one original public CustomControlUnitAPI handle;
        // unchanged MaaCustomControlUnitDestroy performs virtual deletion.
        assert_eq!(
            unsafe { (self.lib.unit.destroy)(u8::from(self.rust), self.pointer.as_ptr()) },
            0
        );
    }
}
impl Unit<'_> {
    /// Output is the matching uniquely borrowed native string/Mat/object head,
    /// or null for scalars. No data view may remain borrowed during this call.
    unsafe fn query(&self, operation: u32, output: *mut c_void) -> (i32, u64) {
        let mut value = 0;
        // SAFETY: Owned actual unit, and for value operations an exclusively
        // borrowed matching actual native value head. No pointer is retained.
        let code =
            unsafe { (self.lib.unit.query)(self.pointer.as_ptr(), operation, output, &mut value) };
        (code, value)
    }
}
struct StringOutput<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
}
impl<'a> StringOutput<'a> {
    fn new(lib: &'a Lib, bytes: &[u8]) -> Self {
        let mut object = std::ptr::null_mut();
        // SAFETY: Readable bytes; exactly one native string owner transferred.
        assert_eq!(
            unsafe { (lib.uuid_output)(bytes.as_ptr(), bytes.len(), &mut object) },
            0
        );
        Self {
            pointer: NonNull::new(object).unwrap(),
            lib,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        let mut view = StringView::default();
        // SAFETY: Owned actual string, no concurrent mutation during this copy.
        assert_eq!(
            unsafe { (self.lib.uuid_view)(self.pointer.as_ptr(), &mut view) },
            0
        );
        assert!(view.length <= isize::MAX as usize);
        if view.length == 0 {
            return Vec::new();
        }
        assert!(!view.data.is_null());
        // SAFETY: Checked readable native owner span; independent test bytes.
        unsafe { std::slice::from_raw_parts(view.data, view.length) }.to_vec()
    }
}
impl Drop for StringOutput<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one string owner, matching destructor before unload.
        unsafe { (self.lib.uuid_release)(self.pointer.as_ptr()) }
    }
}
const SEED: &[u8] = b"original\0\xffstate";
const UUID: &[u8] = b"custom-native\0\xff\x80uuid";
const INFO: &[u8]=b"{\"type\":\"foreign\",\"large\":1e1000,\"nested\":[true,null],\"nul\":\"a\\u0000b\",\"v\":\"\xff\"}//comment";
const EXPECTED_INFO: &[u8]=b"{\"large\":1e1000,\"nested\":[true,null],\"nul\":\"a\\u0000b\",\"type\":\"custom\",\"v\":\"\xff\"}";
const DEFAULT: &[u8] = br#"{"type":"custom"}"#;
struct Controller<'a> {
    lib: &'a Lib,
    thread: ThreadId,
    panic: Cell<bool>,
    calls: Cell<[u32; 7]>,
    code: Cell<u8>,
    boolean: Cell<u8>,
    populate: Cell<bool>,
    features: u64,
    uuid: RefCell<Vec<u8>>,
    pixels: RefCell<Vec<u8>>,
    info: RefCell<Vec<u8>>,
}
impl Controller<'_> {
    fn lib_kind(&self) -> i32 {
        // SAFETY: Scalar constant from the actual compiled OpenCV headers.
        i32::try_from(unsafe { (self.lib.abi)(12) }).unwrap()
    }
    fn enter(&self, op: usize) {
        assert_eq!(std::thread::current().id(), self.thread);
        let mut calls = self.calls.get();
        calls[op] = calls[op].checked_add(1).unwrap();
        self.calls.set(calls);
    }
    fn reset(&self, cycle: usize, empty: bool) {
        *self.uuid.borrow_mut() = if empty { Vec::new() } else { UUID.to_vec() };
        *self.pixels.borrow_mut() = pixels(cycle);
        *self.info.borrow_mut() = if empty {
            br#"{"duplicate":1,"duplicate":2}"#.to_vec()
        } else {
            INFO.to_vec()
        };
    }
}
unsafe fn controller<'a>(argument: *mut c_void) -> &'a Controller<'a> {
    // SAFETY: Exact Controller retained by caller through native context/unit
    // destruction, no pointer retained by synchronous callback functions.
    unsafe { &*argument.cast::<Controller<'a>>() }
}
fn boolean(state: &Controller<'_>, op: usize) -> u8 {
    catch_unwind(AssertUnwindSafe(|| {
        state.enter(op);
        boolean_value(state.boolean.get(), op)
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
fn boolean_value(code: u8, op: usize) -> u8 {
    match op {
        0 => code,
        1 => {
            if code == 0 {
                255
            } else {
                0
            }
        }
        2 => {
            if code == 1 {
                0
            } else {
                2
            }
        }
        _ => unreachable!(),
    }
}
unsafe extern "C" fn connect(argument: *mut c_void) -> u8 {
    // SAFETY: Retained exact callback state on the current owner channel.
    boolean(unsafe { controller(argument) }, 0)
}
unsafe extern "C" fn connected(argument: *mut c_void) -> u8 {
    // SAFETY: Same exact retained callback state.
    boolean(unsafe { controller(argument) }, 1)
}
unsafe extern "C" fn inactive(argument: *mut c_void) -> u8 {
    // SAFETY: Same exact retained callback state.
    boolean(unsafe { controller(argument) }, 2)
}
unsafe extern "C" fn features(argument: *mut c_void) -> u64 {
    // SAFETY: Same exact retained callback state.
    let state = unsafe { controller(argument) };
    catch_unwind(AssertUnwindSafe(|| {
        state.enter(3);
        state.features
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe extern "C" fn uuid(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Same exact retained callback state and live native stack buffer.
    let state = unsafe { controller(argument) };
    catch_unwind(AssertUnwindSafe(|| {
        state.enter(4);
        let mut bytes = state.uuid.borrow_mut();
        // SAFETY: Explicit readable bytes, actual StringBuffer takes a copy.
        assert_eq!(
            unsafe { (state.lib.string_set)(buffer, bytes.as_ptr(), bytes.len()) },
            0
        );
        bytes.fill(0xee);
        state.code.get()
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe extern "C" fn shot(argument: *mut c_void, buffer: *mut OpaqueBuffer) -> u8 {
    // SAFETY: Same exact retained callback state and actual stack ImageBuffer.
    let state = unsafe { controller(argument) };
    catch_unwind(AssertUnwindSafe(|| {
        state.enter(5);
        if state.populate.get() {
            let mut bytes = state.pixels.borrow_mut();
            let mut accepted = 0;
            // SAFETY: Full checked 7x5 packed BGR fixture span; actual compiled
            // OpenCV kind obtained through its native ABI, setter really clones.
            assert_eq!(
                unsafe {
                    (state.lib.raw_set)(
                        buffer,
                        bytes.as_mut_ptr().cast(),
                        bytes.len(),
                        7,
                        5,
                        i32::try_from((state.lib.abi)(12)).unwrap(),
                        &mut accepted,
                    )
                },
                0
            );
            assert_eq!(accepted, 1);
            bytes.fill(0xee);
        }
        state.code.get()
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe extern "C" fn info(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Same exact retained callback state and actual StringBuffer.
    let state = unsafe { controller(argument) };
    catch_unwind(AssertUnwindSafe(|| {
        state.enter(6);
        let mut bytes = state.info.borrow_mut();
        // SAFETY: Source parser receives these complete original raw bytes.
        assert_eq!(
            unsafe { (state.lib.string_set)(buffer, bytes.as_ptr(), bytes.len()) },
            0
        );
        bytes.fill(0xee);
        state.code.get()
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe fn configure(native: &Native<'_>, state: &Controller<'_>, present: bool) {
    let argument = (state as *const Controller<'_>).cast_mut().cast();
    // SAFETY: Caller retains Controller at this address until unit/context
    // destruction. Every reconfiguration retains that same original argument.
    unsafe {
        assert_eq!(
            (native.lib.configure)(
                native.context.as_ptr(),
                present.then_some(connect as BoolCallback),
                present.then_some(connected as BoolCallback),
                present.then_some(inactive as BoolCallback),
                present.then_some(shot as ShotCallback),
                argument
            ),
            0
        );
        assert_eq!(
            (native.lib.identity_configure)(
                native.context.as_ptr(),
                present.then_some(uuid as UuidFunction),
                present.then_some(features as FeaturesFunction),
                argument
            ),
            0
        );
        assert_eq!(
            (native.lib.metadata.configure)(
                native.context.as_ptr(),
                present.then_some(info as UuidFunction),
                argument
            ),
            0
        );
    }
}
fn pixels(cycle: usize) -> Vec<u8> {
    (0..105)
        .map(|i| u8::try_from((i * 17 + cycle * 23 + 7) % 256).unwrap())
        .collect()
}

#[test]
#[ignore = "requires exact complete original Custom API/manager and real public C++ vtables"]
fn consumes_original_creation_virtual_capture_and_destruction_with_rust_manager() {
    let directory =
        PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("native-api");
    std::fs::create_dir(&directory).unwrap();
    let lib = Lib::open();
    // SAFETY: Actual exported source version has static storage and no allocation.
    let version = unsafe { CStr::from_ptr((lib.unit.version)()) }.to_bytes();
    assert_eq!(version, b"mfs-p1-custom-api");
    std::fs::write(directory.join("version.txt"), version).unwrap();
    for index in 0..6 {
        // SAFETY: Scalar observations of actual native function/handle/u64 types.
        assert_eq!(unsafe { (lib.unit.abi)(index) }, 8);
    }
    std::fs::write(directory.join("abi.txt"), b"8\n8\n8\n8\n8\n8\n").unwrap();
    // SAFETY: Real valid native destinations outside wrapper scopes; both
    // publication functions must reject them before touching their values.
    let outside = unsafe { (lib.unit.publication_probe)() };
    assert_eq!(outside, 3);
    std::fs::write(
        directory.join("publication-controls.tsv"),
        format!("outside-scopes\t{outside}\n"),
    )
    .unwrap();
    {
        let native = Native::new(&lib, false);
        let mut output = std::ptr::null_mut();
        // SAFETY: Source create receives an actual null callback table; allocation is not attempted.
        assert_eq!(
            unsafe { (lib.unit.source)(native.context.as_ptr(), &mut output) },
            0
        );
        assert!(output.is_null());
        assert!(Manager::<Native<'_>>::create(None).is_none());
        std::fs::write(
            directory.join("creation-controls.tsv"),
            b"null-table\t0\t0\nnull-destroy\t2\n",
        )
        .unwrap();
        for side in [0, 1] {
            // SAFETY: Original destroy explicitly accepts a null public handle.
            assert_eq!(unsafe { (lib.unit.destroy)(side, std::ptr::null_mut()) }, 0);
        }
    }
    let mut rows = std::fs::File::create_new(directory.join("cases.tsv")).unwrap();
    let mut lifecycles = std::fs::File::create_new(directory.join("lifecycles.tsv")).unwrap();
    let mut held = Vec::new();
    for cycle in 0..20 {
        let code = [0, 1, 2, 255][cycle % 4];
        let state = Controller {
            lib: &lib,
            thread: std::thread::current().id(),
            panic: Cell::new(false),
            calls: Cell::new([0; 7]),
            code: Cell::new(code),
            boolean: Cell::new(code),
            populate: Cell::new(true),
            features: [1 << 63, u64::MAX, 0, (1 << 32) | 7][cycle % 4],
            uuid: RefCell::new(Vec::new()),
            pixels: RefCell::new(Vec::new()),
            info: RefCell::new(Vec::new()),
        };
        let native = Native::new(&lib, true);
        // SAFETY: Stable state is retained until after both public handles and context end.
        unsafe { configure(&native, &state, false) };
        let host = Box::try_new(Host {
            manager: Manager::create(Some(&native)).unwrap(),
            native: &native,
            dropped: Cell::new(0),
            operations: Cell::new(0),
            panic: Cell::new(false),
            thread: std::thread::current().id(),
        })
        .unwrap();
        let mut source = std::ptr::null_mut();
        let mut rust = std::ptr::null_mut();
        // SAFETY: Full initialized outputs and stable boxed Host; both private
        // boundaries catch C++ exceptions before Rust. No callback during create.
        unsafe {
            assert_eq!((lib.unit.source)(native.context.as_ptr(), &mut source), 0);
            assert_eq!(
                (lib.unit.rust)(
                    Some(dispatch),
                    Some(release),
                    (&*host as *const Host<'_, '_>).cast_mut().cast(),
                    &mut rust
                ),
                0
            );
        }
        let source = Unit {
            pointer: NonNull::new(source).unwrap(),
            lib: &lib,
            rust: false,
        };
        let rust = Unit {
            pointer: NonNull::new(rust).unwrap(),
            lib: &lib,
            rust: true,
        };
        for unit in [&source, &rust] {
            // SAFETY: Actual native RTTI cast across original public base/mixins.
            assert_eq!(unsafe { (lib.unit.bases)(unit.pointer.as_ptr()) }, 15);
        }
        let mut values = Values::new(&lib);
        for (phase, present, empty) in [
            ("absent", false, false),
            ("present", true, false),
            ("empty", true, true),
            ("removed", false, true),
        ] {
            state.code.set(if empty { 255 } else { code });
            state.boolean.set(code);
            state.populate.set(!empty);
            // SAFETY: Change borrowed callbacks while keeping original argument fixed.
            unsafe { configure(&native, &state, present) };
            for op in 0..7 {
                observe(
                    &directory,
                    &mut rows,
                    cycle,
                    phase,
                    op,
                    &state,
                    &source,
                    &rust,
                    &mut values,
                    0,
                );
            }
        }
        if cycle == 0 {
            state.code.set(255);
            state.boolean.set(255);
            state.populate.set(true);
            for op in 0..7 {
                // SAFETY: Same state, real C++ fault callback in current table.
                unsafe {
                    configure(&native, &state, true);
                    assert_eq!((lib.unit.fault)(native.context.as_ptr(), op), 0);
                }
                observe(
                    &directory,
                    &mut rows,
                    cycle,
                    &format!("fault-{op}"),
                    op,
                    &state,
                    &source,
                    &rust,
                    &mut values,
                    3,
                );
                // SAFETY: Subsequent independent normal invocation, no assertion retry.
                unsafe { configure(&native, &state, true) };
                observe(
                    &directory,
                    &mut rows,
                    cycle,
                    &format!("recovered-{op}"),
                    op,
                    &state,
                    &source,
                    &rust,
                    &mut values,
                    0,
                );
            }
        }
        state.code.set(255);
        state.boolean.set(255);
        state.populate.set(true);
        // SAFETY: Same original borrowed state through final capture and destroy.
        unsafe { configure(&native, &state, true) };
        for op in [1, 4, 5, 6] {
            observe(
                &directory,
                &mut rows,
                cycle,
                "hold",
                op,
                &state,
                &source,
                &rust,
                &mut values,
                0,
            );
        }
        drop(source);
        drop(rust);
        assert_eq!(host.dropped.get(), 1);
        assert!(!host.panic.get());
        let operations = host.operations.get();
        assert_eq!(operations, if cycle == 0 { 46 } else { 32 });
        drop(host);
        drop(native);
        assert!(!state.panic.get());
        assert_eq!(
            state.calls.get(),
            if cycle == 0 {
                [6, 8, 6, 6, 8, 8, 8]
            } else {
                [4, 6, 4, 4, 6, 6, 6]
            }
        );
        writeln!(
            lifecycles,
            "{cycle}\t15\t15\t1\t{operations}\t{:?}\t{}",
            state.calls.get(),
            lib.count(1)
        )
        .unwrap();
        held.push((values, pixels(cycle)));
    }
    let mut cpu = Vec::new();
    for (cycle, (values, expected)) in held.into_iter().enumerate() {
        for (side, image, uuid, info) in [
            (
                "source",
                &values.source_image,
                &values.source_uuid,
                &values.source_info,
            ),
            (
                "rust",
                &values.rust_image,
                &values.rust_uuid,
                &values.rust_info,
            ),
        ] {
            assert_eq!(image.pixels(), expected);
            assert_eq!(uuid.bytes(), UUID);
            assert_eq!(info.bytes().unwrap(), EXPECTED_INFO);
            std::fs::write(
                directory.join(format!("{cycle}-held-{side}.bgr")),
                image.pixels(),
            )
            .unwrap();
        }
        cpu.push((
            values.source_image.pixels().to_vec(),
            values.rust_image.pixels().to_vec(),
            expected,
        ));
    }
    for index in [0, 1, 6] {
        assert_eq!(lib.count(index), 0);
    }
    // SAFETY: All actual object heads released before reading the scalar ledger.
    assert_eq!(unsafe { (lib.metadata.count)(0) }, 0);
    let mut counts = Vec::new();
    for i in 0..7 {
        // SAFETY: Native ledger after all actual public destruction calls completed.
        counts.push(unsafe { (lib.unit.count)(i) });
    }
    assert_eq!(counts, [20, 20, 20, 20, 20, 56, 56]);
    counts.extend([lib.count(0), lib.count(1), lib.count(6)]);
    // SAFETY: Native scalar object ledger, all native value heads have ended.
    counts.push(unsafe { (lib.metadata.count)(0) });
    counts.extend([lib.count(2), lib.count(3), lib.count(4), lib.count(5)]);
    for index in 1..5 {
        // SAFETY: Scalar metadata counters in the retained native module.
        counts.push(unsafe { (lib.metadata.count)(index) });
    }
    assert_eq!(
        counts,
        [
            20, 20, 20, 20, 20, 56, 56, 0, 0, 0, 0, 62, 62, 124, 124, 142, 56, 36, 101
        ]
    );
    // SAFETY: Actual OpenCV header constant observed through its compiled ABI.
    std::fs::write(
        directory.join("pixel-kind.txt"),
        format!("{}\n", unsafe { (lib.abi)(12) }),
    )
    .unwrap();
    std::fs::write(
        directory.join("stats.tsv"),
        counts
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{i}\t{v}\n"))
            .collect::<String>(),
    )
    .unwrap();
    drop(lib);
    std::thread::spawn(move || {
        for (source, rust, expected) in cpu {
            assert_eq!(source, expected);
            assert_eq!(rust, expected);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "Custom native API lifecycles=20 source_rust_virtual_queries=1308 actual_create_destroy=1 public_bases=15 rust_drop_callbacks=20 scoped_caller_publications=112 retained_pixel_pairs=20 input_actions=0 passed"
    );
}
struct Values<'a> {
    source_image: Image<'a>,
    rust_image: Image<'a>,
    source_uuid: StringOutput<'a>,
    rust_uuid: StringOutput<'a>,
    source_info: Info<'a>,
    rust_info: Info<'a>,
}
impl<'a> Values<'a> {
    fn new(lib: &'a Lib) -> Self {
        Self {
            source_image: lib.seed(),
            rust_image: lib.seed(),
            source_uuid: StringOutput::new(lib, SEED),
            rust_uuid: StringOutput::new(lib, SEED),
            source_info: Objects(lib).empty_object().unwrap(),
            rust_info: Objects(lib).empty_object().unwrap(),
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn observe(
    directory: &Path,
    rows: &mut std::fs::File,
    cycle: usize,
    phase: &str,
    op: u32,
    state: &Controller<'_>,
    source: &Unit<'_>,
    rust: &Unit<'_>,
    values: &mut Values<'_>,
    expected_status: i32,
) {
    let mut results = Vec::new();
    for (side, unit, image, uuid, info) in [
        (
            "source",
            source,
            &mut values.source_image,
            &mut values.source_uuid,
            &mut values.source_info,
        ),
        (
            "rust",
            rust,
            &mut values.rust_image,
            &mut values.rust_uuid,
            &mut values.rust_info,
        ),
    ] {
        state.reset(cycle, phase == "empty" || phase == "removed");
        let output = match op {
            4 => uuid.pointer.as_ptr(),
            5 => image.pointer.as_ptr(),
            6 => info.pointer.as_ptr(),
            _ => std::ptr::null_mut(),
        };
        let before = match op {
            4 => uuid.bytes(),
            5 => image.pixels().to_vec(),
            6 => info.bytes().unwrap(),
            _ => Vec::new(),
        };
        // SAFETY: Matching native value type selected by operation, unique
        // wrapper borrow, and all previous byte views already copied and ended.
        let (status, value) = unsafe { unit.query(op, output) };
        assert_eq!(status, expected_status, "{cycle} {phase} {side} {op}");
        let present = phase != "absent" && phase != "removed";
        let wanted = if status != 0 {
            0
        } else {
            match op {
                0..=2 => {
                    if present {
                        u64::from(
                            boolean_value(state.boolean.get(), usize::try_from(op).unwrap()) != 0,
                        )
                    } else {
                        u64::from(op != 0)
                    }
                }
                3 => {
                    if present {
                        state.features
                    } else {
                        0
                    }
                }
                4 | 5 => u64::from(present && state.code.get() != 0),
                6 => 1,
                _ => unreachable!(),
            }
        };
        assert_eq!(value, wanted, "{cycle} {phase} {side} {op}");
        let bytes = match op {
            4 => uuid.bytes(),
            5 => image.pixels().to_vec(),
            6 => info.bytes().unwrap(),
            _ => Vec::new(),
        };
        if op == 5 {
            let view = image.view();
            let expected = if bytes.is_empty() {
                (0, 0, 1, 0, 0, 0, 0)
            } else {
                (7, 5, 3, 21, state.lib_kind(), 105, 1)
            };
            assert_eq!(
                (
                    view.width,
                    view.height,
                    view.channels,
                    view.stride,
                    view.kind,
                    view.length,
                    view.references
                ),
                expected
            );
            let file = directory.join(format!("{cycle}-{phase}-{side}.layout"));
            std::fs::write(
                file,
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    view.width,
                    view.height,
                    view.channels,
                    view.stride,
                    view.kind,
                    view.length,
                    view.references
                ),
            )
            .unwrap();
        }
        if status != 0 || ((op == 4 || op == 5) && value == 0) {
            assert_eq!(bytes, before);
        }
        if op >= 4 {
            std::fs::write(
                directory.join(format!("{cycle}-{phase}-{side}-{op}.bin")),
                &bytes,
            )
            .unwrap();
        }
        results.push((status, value, bytes));
    }
    assert_eq!(results[0], results[1]);
    writeln!(
        rows,
        "{cycle}\t{phase}\t{op}\t{}\t{}\t{}\t{}",
        results[0].0, results[1].0, results[0].1, results[1].1
    )
    .unwrap();
}
