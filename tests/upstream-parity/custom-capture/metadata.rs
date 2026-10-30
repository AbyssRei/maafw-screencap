// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual StringBuffer and source JSON objects. No SDK or core JSON dependency.
use super::*;
use identity::UuidBuffer;
use std::{io::Write, path::Path};
#[path = "metadata_cases.rs"]
mod corpus;

type InfoFunction = unsafe extern "C" fn(*mut c_void, *mut OpaqueString) -> u8;
pub(super) struct Api {
    abi: unsafe extern "C" fn(u32) -> usize,
    configure: unsafe extern "C" fn(*mut c_void, Option<InfoFunction>, *mut c_void) -> i32,
    has: unsafe extern "C" fn(*mut c_void) -> u8,
    invoke: unsafe extern "C" fn(*mut c_void, *mut OpaqueString, *mut u8) -> i32,
    empty: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    parse: unsafe extern "C" fn(*mut OpaqueString, *mut *mut c_void) -> i32,
    set_type: unsafe extern "C" fn(*mut c_void) -> i32,
    source: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    release: unsafe extern "C" fn(*mut c_void),
    serialize: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    fault: unsafe extern "C" fn(*mut c_void) -> i32,
    count: unsafe extern "C" fn(u32) -> u64,
}
impl Api {
    pub(super) fn open(module: &loader::Module) -> Self {
        // SAFETY: Exact private bridge.h C declarations and live module. Lib
        // retains the module until all callback, object and string owners end.
        unsafe {
            Self {
                abi: module.symbol(c"mfs_custom_info_abi"),
                configure: module.symbol(c"mfs_custom_info_configure"),
                has: module.symbol(c"mfs_custom_has_info"),
                invoke: module.symbol(c"mfs_custom_info_invoke"),
                empty: module.symbol(c"mfs_custom_info_empty"),
                parse: module.symbol(c"mfs_custom_info_parse"),
                set_type: module.symbol(c"mfs_custom_info_type"),
                source: module.symbol(c"mfs_custom_info_source"),
                release: module.symbol(c"mfs_custom_info_release"),
                serialize: module.symbol(c"mfs_custom_info_serialize"),
                fault: module.symbol(c"mfs_custom_info_fault"),
                count: module.symbol(c"mfs_custom_info_count"),
            }
        }
    }
}
impl InfoBoundary for Native<'_> {
    type InfoBuffer = UuidBuffer;
    fn info_callback(&self) -> Option<InfoCallback<Self>> {
        // SAFETY: Current borrowed native callback table on its owner channel.
        (unsafe { (self.lib.metadata.has)(self.context.as_ptr()) } != 0).then_some(Self::info)
    }
    fn with_info_buffer<T>(
        &self,
        action: impl FnOnce(&mut UuidBuffer) -> Result<T, i32>,
    ) -> Result<T, i32> {
        self.with_uuid_buffer(action)
    }
}
impl Native<'_> {
    fn info(&self, buffer: &mut UuidBuffer) -> Result<u8, i32> {
        let mut value = 0;
        // SAFETY: Synchronous actual callback; native StringBuffer lives for
        // this scope and exceptions are contained before returning to Rust.
        status(unsafe {
            (self.lib.metadata.invoke)(self.context.as_ptr(), buffer.0.as_ptr(), &mut value)
        })?;
        Ok(value)
    }
}
fn status(code: i32) -> Result<(), i32> {
    if code == 0 { Ok(()) } else { Err(code) }
}
pub(super) struct Info<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
}
impl Drop for Info<'_> {
    fn drop(&mut self) {
        // SAFETY: Sole owner of the transferred native json::object head;
        // matching destructor runs before its borrowed module can unload.
        unsafe { (self.lib.metadata.release)(self.pointer.as_ptr()) }
    }
}
impl Info<'_> {
    fn bytes(&self) -> Result<Vec<u8>, i32> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Native object is borrowed immutably; serialization produces
        // a separate actual std::string owner, not a view into mutable scratch.
        status(unsafe { (self.lib.metadata.serialize)(self.pointer.as_ptr(), &mut pointer) })?;
        let output = Serialized {
            pointer: NonNull::new(pointer).ok_or(1)?,
            lib: self.lib,
        };
        let mut view = StringView::default();
        // SAFETY: Live owned native string and initialized full view output.
        status(unsafe { (self.lib.uuid_view)(output.pointer.as_ptr(), &mut view) })?;
        if view.length > isize::MAX as usize || (view.length != 0 && view.data.is_null()) {
            return Err(1);
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(view.length).map_err(|_| -101)?;
        if view.length != 0 {
            // SAFETY: Validated readable string span; the real string owner
            // remains alive and unmodified until this independent copy ends.
            bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(view.data, view.length) });
        }
        Ok(bytes)
    }
}
struct Serialized<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
}
impl Drop for Serialized<'_> {
    fn drop(&mut self) {
        // SAFETY: Matching destructor for exactly one serialization owner.
        unsafe { (self.lib.uuid_release)(self.pointer.as_ptr()) }
    }
}
struct Objects<'a>(&'a Lib);
impl<'a> InfoObjects for Objects<'a> {
    type Buffer = UuidBuffer;
    type Info = Info<'a>;
    type Error = i32;
    fn empty_object(&self) -> Result<Info<'a>, i32> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Full initialized output; success transfers one real object.
        status(unsafe { (self.0.metadata.empty)(&mut pointer) })?;
        Ok(Info {
            pointer: NonNull::new(pointer).ok_or(1)?,
            lib: self.0,
        })
    }
    fn parse_object(&self, buffer: &UuidBuffer) -> Result<Option<Info<'a>>, i32> {
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Actual StringBuffer is live. Native parse/as_object preserves
        // source syntax/bytes and copies the object before the stack scope ends.
        status(unsafe { (self.0.metadata.parse)(buffer.0.as_ptr(), &mut pointer) })?;
        Ok(NonNull::new(pointer).map(|pointer| Info {
            pointer,
            lib: self.0,
        }))
    }
    fn set_custom_type(&self, info: &mut Info<'a>) -> Result<(), i32> {
        // SAFETY: Unique object borrow. Adapter also rejects this operation
        // while a native stack StringBuffer scope is still active.
        status(unsafe { (self.0.metadata.set_type)(info.pointer.as_ptr()) })
    }
}
fn original<'a>(native: &Native<'a>) -> Result<Info<'a>, i32> {
    let mut pointer = std::ptr::null_mut();
    // SAFETY: Exact original method body, native callback state alive through
    // synchronous completion. Failure keeps the initialized output null.
    let code = unsafe { (native.lib.metadata.source)(native.context.as_ptr(), &mut pointer) };
    if code != 0 {
        assert!(pointer.is_null());
        return Err(code);
    }
    Ok(Info {
        pointer: NonNull::new(pointer).ok_or(1)?,
        lib: native.lib,
    })
}
struct State<'a> {
    lib: &'a Lib,
    input: RefCell<Vec<u8>>,
    code: u8,
    calls: Cell<u32>,
    panic: Cell<bool>,
    thread: ThreadId,
}
unsafe extern "C" fn callback(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Stable exact State retained through context destruction or clear.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        state.calls.set(state.calls.get().checked_add(1).unwrap());
        let mut bytes = state.input.borrow_mut();
        // SAFETY: Explicit readable byte span copied into real StringBuffer.
        assert_eq!(
            unsafe { (state.lib.string_set)(buffer, bytes.as_ptr(), bytes.len()) },
            0
        );
        bytes.fill(0xee);
        state.code
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe fn configure(native: &Native<'_>, state: &State<'_>, present: bool) {
    // SAFETY: Caller retains exact state on this channel until clear/drop.
    assert_eq!(
        unsafe {
            (native.lib.metadata.configure)(
                native.context.as_ptr(),
                present.then_some(callback as InfoFunction),
                (state as *const State<'_>).cast_mut().cast(),
            )
        },
        0
    );
}
fn state<'a>(lib: &'a Lib, input: &[u8], code: u8) -> State<'a> {
    State {
        lib,
        input: RefCell::new(input.to_vec()),
        code,
        calls: Cell::new(0),
        panic: Cell::new(false),
        thread: std::thread::current().id(),
    }
}
fn count(lib: &Lib, index: u32) -> u64 {
    // SAFETY: Scalar native object ledger, module retained.
    unsafe { (lib.metadata.count)(index) }
}

#[test]
#[ignore = "requires explicit original Custom metadata/StringBuffer/meojson adapter"]
fn preserves_metadata_parser_bytes_scope_order_defaults_and_exceptions() {
    let directory =
        PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("metadata");
    std::fs::create_dir(&directory).unwrap();
    let lib = Lib::open();
    let objects = Objects(&lib);
    // SAFETY: Layout observed from the actual complete upstream callback header.
    assert_eq!(
        unsafe { (lib.metadata.abi)(0) },
        size_of::<Option<InfoFunction>>()
    );
    assert_eq!(unsafe { (lib.metadata.abi)(1) }, 20 * size_of::<usize>());
    assert_eq!((lib.count(1), count(&lib, 0)), (0, 0));
    let cases = corpus::cases();
    assert_eq!(cases.len(), 37);
    for case in &cases {
        std::fs::write(directory.join(format!("{}.input", case.name)), case.input).unwrap();
    }
    let mut rows = std::fs::File::create_new(directory.join("cases.tsv")).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(592).unwrap();
    let mut index = 0;
    for table in [false, true] {
        for present in [false, true] {
            for code in [0, 1, 2, 255] {
                for case in &cases {
                    let state = state(&lib, case.input, code);
                    let native = Native::new(&lib, table);
                    let manager = Manager::new(Some(&native));
                    // SAFETY: Stable State through native drop below.
                    unsafe { configure(&native, &state, present) };
                    let source = original(&native).unwrap();
                    state.input.borrow_mut().copy_from_slice(case.input);
                    let rust = manager.get_info(&objects).unwrap();
                    let expected = if table && present && code != 0 {
                        case.expected
                    } else {
                        corpus::DEFAULT
                    };
                    let source_bytes = source.bytes().unwrap();
                    let rust_bytes = rust.bytes().unwrap();
                    assert_eq!(
                        source_bytes, expected,
                        "source {} {table} {present} {code}",
                        case.name
                    );
                    assert_eq!(
                        rust_bytes, expected,
                        "Rust {} {table} {present} {code}",
                        case.name
                    );
                    assert_eq!(state.calls.get(), if table && present { 2 } else { 0 });
                    assert!(!state.panic.get());
                    std::fs::write(
                        directory.join(format!("{index}-source.json.bin")),
                        source_bytes,
                    )
                    .unwrap();
                    std::fs::write(directory.join(format!("{index}-rust.json.bin")), rust_bytes)
                        .unwrap();
                    writeln!(
                        rows,
                        "{index}\t{}\t{}\t{code}\t{}\t{}\t{}\t{}",
                        u8::from(table),
                        u8::from(present),
                        case.name,
                        expected.len(),
                        state.calls.get(),
                        u8::from(table && present && code != 0 && case.object)
                    )
                    .unwrap();
                    drop(native);
                    held.push((source, rust, expected.to_vec()));
                    assert_eq!(lib.count(1), 0);
                    index += 1;
                }
            }
        }
    }
    assert_eq!(index, 592);
    controls(&lib, &objects, &directory);
    let mut cpu = Vec::new();
    for (source, rust, expected) in held {
        assert_eq!(source.bytes().unwrap(), expected);
        let bytes = rust.bytes().unwrap();
        assert_eq!(bytes, expected);
        cpu.push((bytes, expected));
    }
    let parsed = u64::try_from(cases.iter().filter(|c| c.object).count()).unwrap() * 3 + 2;
    let ledger = [
        count(&lib, 0),
        count(&lib, 1),
        count(&lib, 2),
        count(&lib, 3),
        count(&lib, 4),
        lib.count(4),
        lib.count(5),
        lib.count(6),
        lib.count(1),
    ];
    assert_eq!(ledger, [0, 599, 113, parsed, 597, 152, 152, 0, 0]);
    std::fs::write(
        directory.join("stats.tsv"),
        ledger
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{i}\t{v}\n"))
            .collect::<String>(),
    )
    .unwrap();
    std::fs::write(
        directory.join("abi.txt"),
        format!(
            "{}\n{}\n",
            size_of::<Option<InfoFunction>>(),
            20 * size_of::<usize>()
        ),
    )
    .unwrap();
    drop(lib);
    std::thread::spawn(move || {
        for (bytes, expected) in cpu {
            assert_eq!(bytes, expected);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "Custom metadata cases=592 payloads=37 source_rust_outputs=1184 actual_stringbuffer_meojson=1 parse_inside_scope=1 type_after_scope=1 borrowed_table_changes=1 exceptions_contained=1 retained_pairs=592 input_actions=0 passed"
    );
}
fn controls(lib: &Lib, objects: &Objects<'_>, directory: &Path) {
    let bytes = b"{\"v\":\"\xff\"}";
    let expected = b"{\"type\":\"custom\",\"v\":\"\xff\"}";
    let state = state(lib, bytes, 255);
    let native = Native::new(lib, true);
    let manager = Manager::new(Some(&native));
    let mut records = std::fs::File::create_new(directory.join("controls.tsv")).unwrap();
    for (label, present) in [("absent", false), ("present", true), ("removed", false)] {
        // SAFETY: Same live state; changes current borrowed table without rebuilding manager.
        unsafe { configure(&native, &state, present) };
        let wanted: &[u8] = if present { expected } else { corpus::DEFAULT };
        state.input.borrow_mut().copy_from_slice(bytes);
        let source = original(&native).unwrap().bytes().unwrap();
        state.input.borrow_mut().copy_from_slice(bytes);
        let rust = manager.get_info(objects).unwrap().bytes().unwrap();
        assert_eq!(source, wanted);
        assert_eq!(rust, wanted);
        for (side, value) in [("source", source), ("rust", rust)] {
            std::fs::write(directory.join(format!("{label}-{side}.json.bin")), value).unwrap();
        }
        writeln!(records, "{label}\t{}", state.calls.get()).unwrap();
    }
    // SAFETY: Compiled C++ callback throws after populating the real buffer;
    // both private C boundaries contain it and transfer no object on failure.
    assert_eq!(unsafe { (lib.metadata.fault)(native.context.as_ptr()) }, 0);
    assert_eq!(original(&native).err(), Some(3));
    assert_eq!(manager.get_info(objects).err(), Some(3));
    writeln!(records, "exception\t3\t3").unwrap();
    // SAFETY: Restore actual callback for a subsequent independent query.
    unsafe { configure(&native, &state, true) };
    state.input.borrow_mut().copy_from_slice(bytes);
    let source = original(&native).unwrap().bytes().unwrap();
    state.input.borrow_mut().copy_from_slice(bytes);
    let rust = manager.get_info(objects).unwrap().bytes().unwrap();
    assert_eq!(source, expected);
    assert_eq!(rust, expected);
    for (side, value) in [("source", source), ("rust", rust)] {
        std::fs::write(directory.join(format!("recovered-{side}.json.bin")), value).unwrap();
    }
    writeln!(records, "recovered\t{}", state.calls.get()).unwrap();
    let null = Manager::<Native<'_>>::new(None).get_info(objects).unwrap();
    assert_eq!(null.bytes().unwrap(), corpus::DEFAULT);
    std::fs::write(directory.join("null.json.bin"), null.bytes().unwrap()).unwrap();
    writeln!(records, "null\t{}", corpus::DEFAULT.len()).unwrap();
    let mut object = objects.empty_object().unwrap();
    assert_eq!(
        native.with_info_buffer(|_| objects.set_custom_type(&mut object)),
        Err(1)
    );
    writeln!(records, "type-inside-scope\t1").unwrap();
    assert!(!state.panic.get());
    // SAFETY: Clear borrowed callback before State can end.
    unsafe { configure(&native, &state, false) };
}
