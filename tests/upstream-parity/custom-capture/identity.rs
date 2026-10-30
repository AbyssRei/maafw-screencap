// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real original StringBuffer callbacks and byte-string owners. No input actions.
use super::*;
use std::io::Write;

pub(super) struct UuidBuffer(NonNull<OpaqueString>);
impl Native<'_> {
    fn request_uuid(&self, buffer: &mut UuidBuffer) -> Result<u8, i32> {
        let mut value = 0;
        // SAFETY: Exact synchronous C declaration, live context/callback state
        // and native stack StringBuffer. Native boundary contains exceptions.
        let status =
            unsafe { (self.lib.uuid_invoke)(self.context.as_ptr(), buffer.0.as_ptr(), &mut value) };
        if status == 0 { Ok(value) } else { Err(status) }
    }
}
impl IdentityBoundary for Native<'_> {
    type UuidBuffer = UuidBuffer;
    type Uuid = Vec<u8>;
    fn uuid_callback(&self) -> Option<UuidCallback<Self>> {
        // SAFETY: Reads current actual upstream callback table, on its owner channel.
        (unsafe { (self.lib.has_uuid)(self.context.as_ptr()) } != 0).then_some(Self::request_uuid)
    }
    fn features(&self) -> Option<Result<u64, i32>> {
        let mut value = 0;
        // SAFETY: Exact u64 native feature field, callback state retained by test.
        let status = unsafe { (self.lib.features)(self.context.as_ptr(), 0, &mut value) };
        (status != -1).then_some(if status == 0 { Ok(value) } else { Err(status) })
    }
    fn uuid(&self, buffer: &UuidBuffer) -> Result<Vec<u8>, i32> {
        let mut view = StringView::default();
        // SAFETY: Native StringBuffer remains live throughout this callback;
        // its immutable backing string is not modified while bytes are read.
        let status = unsafe { (self.lib.string_view)(buffer.0.as_ptr(), &mut view) };
        if status != 0 {
            return Err(status);
        }
        let bytes = string_bytes(&view);
        let mut owned = Vec::new();
        owned.try_reserve_exact(bytes.len()).map_err(|_| -101)?;
        owned.extend_from_slice(bytes);
        Ok(owned)
    }
    fn with_uuid_buffer<T>(
        &self,
        action: impl FnOnce(&mut UuidBuffer) -> Result<T, i32>,
    ) -> Result<T, i32> {
        struct Call<F, T> {
            action: Option<F>,
            result: Option<Result<T, i32>>,
        }
        unsafe extern "C" fn execute<F: FnOnce(&mut UuidBuffer) -> Result<T, i32>, T>(
            argument: *mut c_void,
            buffer: *mut OpaqueString,
        ) -> i32 {
            // SAFETY: Exact stack state, one synchronous call; never retained.
            let call = unsafe { &mut *argument.cast::<Call<F, T>>() };
            let Some(action) = call.action.take() else {
                return 1;
            };
            let Some(buffer) = NonNull::new(buffer) else {
                return 1;
            };
            call.result = Some(
                catch_unwind(AssertUnwindSafe(|| action(&mut UuidBuffer(buffer))))
                    .unwrap_or(Err(-100)),
            );
            0
        }
        fn run<F: FnOnce(&mut UuidBuffer) -> Result<T, i32>, T>(
            lib: &Lib,
            action: F,
        ) -> Result<T, i32> {
            let mut call = Call {
                action: Some(action),
                result: None,
            };
            // SAFETY: Monomorphized callback matches stack Call. It contains
            // Rust panics; C++ destroys its StringBuffer after returning.
            let status = unsafe {
                (lib.string_scope)(Some(execute::<F, T>), (&mut call as *mut Call<F, T>).cast())
            };
            if status != 0 {
                return Err(status);
            }
            call.result.ok_or(1)?
        }
        run(self.lib, action)
    }
}

fn string_bytes(view: &StringView) -> &[u8] {
    assert!(view.length <= isize::MAX as usize);
    if view.length == 0 {
        return &[];
    }
    assert!(!view.data.is_null());
    // SAFETY: Only used while the concrete native StringBuffer/std::string
    // owner is borrowed. Caller does not mutate or destroy it during this read.
    unsafe { std::slice::from_raw_parts(view.data, view.length) }
}
struct Output<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
}
impl<'a> Output<'a> {
    fn new(lib: &'a Lib, seed: &[u8]) -> Self {
        let mut output = std::ptr::null_mut();
        // SAFETY: Readable seed with explicit length; success transfers one
        // actual std::string owner, whose matching destructor borrows Lib.
        assert_eq!(
            unsafe { (lib.uuid_output)(seed.as_ptr(), seed.len(), &mut output) },
            0
        );
        Self {
            pointer: NonNull::new(output).unwrap(),
            lib,
        }
    }
    fn view(&self) -> StringView {
        let mut view = StringView::default();
        // SAFETY: Owned live std::string, initialized writable view output.
        assert_eq!(
            unsafe { (self.lib.uuid_view)(self.pointer.as_ptr(), &mut view) },
            0
        );
        view
    }
    fn bytes(&self) -> &[u8] {
        let view = self.view();
        assert!(view.length <= isize::MAX as usize);
        if view.length == 0 {
            return &[];
        }
        assert!(!view.data.is_null());
        // SAFETY: This returned borrow retains self and its real native owner;
        // no mutable view exists, and the module outlives self.
        unsafe { std::slice::from_raw_parts(view.data, view.length) }
    }
    fn request(&mut self, native: &Native<'_>) -> Result<bool, i32> {
        let mut value = 0;
        // SAFETY: Distinct owned output string; actual original function body
        // receives it by mutable reference, and native exceptions are contained.
        let status = unsafe {
            (self.lib.uuid_source)(native.context.as_ptr(), self.pointer.as_ptr(), &mut value)
        };
        if status == 0 {
            Ok(value != 0)
        } else {
            Err(status)
        }
    }
}
impl Drop for Output<'_> {
    fn drop(&mut self) {
        // SAFETY: Sole owner, matching native destructor before library unload.
        unsafe { (self.lib.uuid_release)(self.pointer.as_ptr()) }
    }
}
struct State<'a> {
    lib: &'a Lib,
    thread: ThreadId,
    code: u8,
    expected: Vec<u8>,
    input: RefCell<Vec<u8>>,
    features: Cell<u64>,
    uuid_calls: Cell<u32>,
    feature_calls: Cell<u32>,
    panic: Cell<bool>,
}
impl State<'_> {
    fn reset_input(&self) {
        self.input.borrow_mut().copy_from_slice(&self.expected)
    }
}
unsafe extern "C" fn uuid_callback(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Test retains this exact state until callbacks are cleared or
    // context destruction completes. No native buffer pointer escapes.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        state
            .uuid_calls
            .set(state.uuid_calls.get().checked_add(1).unwrap());
        let mut bytes = state.input.borrow_mut();
        // SAFETY: Explicit readable span, copied into actual MaaUtils StringBuffer.
        assert_eq!(
            unsafe { (state.lib.string_set)(buffer, bytes.as_ptr(), bytes.len()) },
            0
        );
        bytes.fill(0xee); // Native set must own the original complete bytes.
        state.code
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe extern "C" fn features_callback(argument: *mut c_void) -> u64 {
    // SAFETY: Same retained exact callback state; no input action is performed.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        state
            .feature_calls
            .set(state.feature_calls.get().checked_add(1).unwrap());
        state.features.get()
    }))
    .unwrap_or_else(|_| {
        state.panic.set(true);
        0
    })
}
unsafe fn configure(native: &Native<'_>, state: &State<'_>, present: bool) {
    // SAFETY: Caller holds state at this exact address on its channel until
    // context drop, or clears both callbacks before allowing state to end.
    assert_eq!(
        unsafe {
            (native.lib.identity_configure)(
                native.context.as_ptr(),
                present.then_some(uuid_callback as UuidFunction),
                present.then_some(features_callback as FeaturesFunction),
                (state as *const State<'_>).cast_mut().cast(),
            )
        },
        0
    );
}
fn payload(index: usize) -> Vec<u8> {
    match index {
        0 => Vec::new(),
        1 => b"custom-capture-id".to_vec(),
        2 => vec![0, 0xff, b'a', 0, 0x80, b'z'],
        3 => (0..1025)
            .map(|n| u8::try_from((n * 31 + 7) % 256).unwrap())
            .collect(),
        _ => unreachable!(),
    }
}
fn state(lib: &Lib, code: u8, bytes: Vec<u8>, features: u64) -> State<'_> {
    State {
        lib,
        thread: std::thread::current().id(),
        code,
        input: RefCell::new(bytes.clone()),
        expected: bytes,
        features: Cell::new(features),
        uuid_calls: Cell::new(0),
        feature_calls: Cell::new(0),
        panic: Cell::new(false),
    }
}
fn original_features(native: &Native<'_>) -> Result<u64, i32> {
    let mut value = 0;
    // SAFETY: Exact declaration, retained context and callback state.
    let status = unsafe { (native.lib.features)(native.context.as_ptr(), 1, &mut value) };
    if status == 0 { Ok(value) } else { Err(status) }
}

#[test]
#[ignore = "requires explicit original Custom manager and real MaaUtils StringBuffer adapter"]
fn preserves_uuid_bytes_features_failures_and_borrowed_callbacks() {
    let directory =
        PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("identity");
    std::fs::create_dir(&directory).unwrap();
    let lib = Lib::open();
    let expected_abi = [
        size_of::<StringView>(),
        align_of::<StringView>(),
        offset_of!(StringView, data),
        offset_of!(StringView, length),
        size_of::<Option<UuidFunction>>(),
        size_of::<Option<FeaturesFunction>>(),
        size_of::<Option<StringScopeFunction>>(),
        size_of::<u64>(),
        2 * size_of::<usize>(),
        3 * size_of::<usize>(),
    ];
    let mut actual_abi = Vec::new();
    for (index, expected) in expected_abi.iter().enumerate() {
        // SAFETY: Scalar layout observation from actual C header-compiled declarations.
        let actual = unsafe { (lib.identity_abi)(u32::try_from(index).unwrap()) };
        assert_eq!(actual, *expected);
        actual_abi.push(actual);
    }
    assert_eq!(lib.count(1), 0);
    let seed = (0..257)
        .map(|n| u8::try_from((n * 11 + 3) % 256).unwrap())
        .collect::<Vec<_>>();
    let mut rows = std::fs::File::create_new(directory.join("cases.tsv")).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(64).unwrap();
    let mut index = 0;
    for table in [false, true] {
        for present in [false, true] {
            for code in [0, 1, 2, 255] {
                for kind in 0..4 {
                    let features = [0, 1 << 63, u64::MAX, (1 << 32) | 7][kind];
                    let state = state(&lib, code, payload(kind), features);
                    let mut source = Output::new(&lib, &seed);
                    let mut rust = seed.clone();
                    let source_pointer = source.view().data;
                    let rust_pointer = rust.as_ptr();
                    let native = Native::new(&lib, table);
                    let manager = Manager::new(Some(&native));
                    // SAFETY: State has a stable stack address through all calls and native drop.
                    unsafe { configure(&native, &state, present) };
                    let success = table && present && code != 0;
                    assert_eq!(source.request(&native), Ok(success));
                    state.reset_input();
                    assert_eq!(manager.request_uuid(&mut rust), Ok(success));
                    let expected = if success { &state.expected } else { &seed };
                    assert_eq!(source.bytes(), expected);
                    assert_eq!(rust, *expected);
                    if !success {
                        assert_eq!(source.view().data, source_pointer);
                        assert_eq!(rust.as_ptr(), rust_pointer);
                    }
                    let source_features = original_features(&native).unwrap();
                    let rust_features = manager.get_features().unwrap();
                    assert_eq!(source_features, if table && present { features } else { 0 });
                    assert_eq!(rust_features, source_features);
                    assert_eq!(
                        (state.uuid_calls.get(), state.feature_calls.get()),
                        if table && present { (2, 2) } else { (0, 0) }
                    );
                    assert!(!state.panic.get());
                    std::fs::write(
                        directory.join(format!("{index}-source.uuid")),
                        source.bytes(),
                    )
                    .unwrap();
                    std::fs::write(directory.join(format!("{index}-rust.uuid")), &rust).unwrap();
                    writeln!(rows, "{index}\t{}\t{}\t{code}\t{kind}\t{features}\t{}\t{}\t{source_features}\t{rust_features}\t{}\t{}",
                        u8::from(table),u8::from(present),u8::from(success),rust.len(),state.uuid_calls.get(),state.feature_calls.get()).unwrap();
                    drop(native);
                    held.push((source, rust, expected.to_vec()));
                    assert_eq!(lib.count(1), 0);
                    index += 1;
                }
            }
        }
    }
    assert_eq!(index, 64);
    controls(&lib, &directory, &seed);
    for (source, rust, expected) in &held {
        assert_eq!(source.bytes(), expected);
        assert_eq!(rust, expected);
    }
    let cpu = held
        .into_iter()
        .map(|(source, rust, expected)| {
            drop(source);
            (rust, expected)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        (
            lib.count(0),
            lib.count(1),
            lib.count(2),
            lib.count(3),
            lib.count(6)
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(lib.count(4), lib.count(5));
    assert_eq!(lib.count(4), 20);
    std::fs::write(
        directory.join("abi.txt"),
        actual_abi
            .iter()
            .map(|n| format!("{n}\n"))
            .collect::<String>(),
    )
    .unwrap();
    std::fs::write(directory.join("stats.tsv"), format!("uuid_scopes_entered\t{}\nuuid_scopes_exited\t{}\nnative_uuid_outputs\t{}\nnative_contexts\t{}\nretained_uuid_pairs\t64\n",lib.count(4),lib.count(5),lib.count(6),lib.count(1))).unwrap();
    drop(lib);
    std::thread::spawn(move || {
        for (bytes, expected) in cpu {
            assert_eq!(bytes, expected);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "Custom identity cases=64 source_rust_uuid_queries=128 feature_queries=128 actual_stringbuffer=1 nonutf8_nul=1 false_preserves_owner=1 exceptions_contained=1 retained_pairs=64 input_actions=0 passed"
    );
}

fn controls(lib: &Lib, directory: &std::path::Path, seed: &[u8]) {
    let state = state(lib, 1, payload(2), u64::MAX);
    let native = Native::new(lib, true);
    let manager = Manager::new(Some(&native));
    let mut rust = seed.to_vec();
    let mut source = Output::new(lib, seed);
    let mut records = std::fs::File::create_new(directory.join("controls.tsv")).unwrap();
    macro_rules! query {
        ($name:expr, $expected:expr) => {{
            state.reset_input();
            let source_uuid = source.request(&native).unwrap();
            state.reset_input();
            let rust_uuid = manager.request_uuid(&mut rust).unwrap();
            let source_features = original_features(&native).unwrap();
            let rust_features = manager.get_features().unwrap();
            assert_eq!(
                (source_uuid, rust_uuid, source_features, rust_features),
                $expected
            );
            writeln!(
                records,
                "{}\t{}\t{}\t{source_features}\t{rust_features}",
                $name,
                u8::from(source_uuid),
                u8::from(rust_uuid)
            )
            .unwrap();
        }};
    }
    // SAFETY: The state remains stable until both callbacks are cleared below.
    unsafe { configure(&native, &state, false) };
    query!("table-absent", (false, false, 0, 0));
    // SAFETY: Same live state and owner channel.
    unsafe { configure(&native, &state, true) };
    query!("table-present", (true, true, u64::MAX, u64::MAX));
    // SAFETY: Clears both borrowed callbacks while manager remains alive.
    unsafe { configure(&native, &state, false) };
    query!("table-removed", (false, false, 0, 0));
    assert_eq!(source.bytes(), state.expected);
    assert_eq!(rust, state.expected);
    let null_manager = Manager::<Native<'_>>::new(None);
    let null_uuid = null_manager.request_uuid(&mut rust).unwrap();
    let null_features = null_manager.get_features().unwrap();
    assert_eq!((null_uuid, null_features), (false, 0));
    writeln!(
        records,
        "null-manager\t{}\t{null_features}",
        u8::from(null_uuid)
    )
    .unwrap();
    for operation in 0..2 {
        // SAFETY: Installs a compiled C++ fault callback. Context remains owned;
        // guarded C ABI catches the throw before returning to Rust.
        unsafe {
            configure(&native, &state, true);
            assert_eq!((lib.identity_fault)(native.context.as_ptr(), operation), 0);
        }
        if operation == 0 {
            let pointer = rust.as_ptr();
            let native_pointer = source.view().data;
            let source_error = source.request(&native).unwrap_err();
            let rust_error = manager.request_uuid(&mut rust).unwrap_err();
            assert_eq!((source_error, rust_error), (3, 3));
            writeln!(records, "uuid-exception\t{source_error}\t{rust_error}").unwrap();
            assert_eq!(source.bytes(), state.expected);
            assert_eq!(source.view().data, native_pointer);
            assert_eq!(rust.as_ptr(), pointer);
            assert_eq!(rust, state.expected);
            std::fs::write(directory.join("exception-source.uuid"), source.bytes()).unwrap();
            std::fs::write(directory.join("exception-rust.uuid"), &rust).unwrap();
        } else {
            let source_error = original_features(&native).unwrap_err();
            let rust_error = manager.get_features().unwrap_err();
            assert_eq!((source_error, rust_error), (3, 3));
            writeln!(records, "features-exception\t{source_error}\t{rust_error}").unwrap();
        }
        // SAFETY: Restore actual callbacks before the next independent query.
        unsafe { configure(&native, &state, true) };
        query!(
            format!("recovery-{operation}"),
            (true, true, u64::MAX, u64::MAX)
        );
        assert_eq!(source.bytes(), state.expected);
        assert_eq!(rust, state.expected);
    }
    assert!(!state.panic.get());
    // SAFETY: Remove callbacks before the retained state can end.
    unsafe { configure(&native, &state, false) };
    std::fs::write(directory.join("recovered-source.uuid"), source.bytes()).unwrap();
    std::fs::write(directory.join("recovered-rust.uuid"), &rust).unwrap();
}
