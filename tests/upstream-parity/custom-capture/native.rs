// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit native original callback/real ImageBuffer test. No SDK ABI.
use super::*;
use std::{
    cell::{Cell, RefCell},
    env,
    ffi::c_void,
    marker::PhantomData,
    mem::{align_of, offset_of, size_of},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    ptr::NonNull,
    rc::Rc,
    thread::ThreadId,
};

#[cfg(windows)]
use crate::platform::windows::test_library as loader;
#[cfg(unix)]
mod loader {
    use super::*;
    use std::{
        ffi::{CStr, CString},
        os::unix::ffi::OsStrExt,
        path::Path,
    };
    pub struct Module(NonNull<c_void>);
    impl Module {
        pub fn open(path: &Path) -> Self {
            let path = CString::new(path.as_os_str().as_bytes()).unwrap();
            // SAFETY: Terminated native path, local eager resolution; all
            // users borrow the containing Lib through actual native shutdown.
            Self(
                NonNull::new(unsafe {
                    libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL)
                })
                .expect("load native custom model"),
            )
        }
        pub unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
            assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
            // SAFETY: Live module and exact bridge.h function declaration.
            let pointer = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
            assert!(!pointer.is_null(), "missing custom symbol {name:?}");
            unsafe { std::mem::transmute_copy(&pointer) }
        }
    }
    impl Drop for Module {
        fn drop(&mut self) {
            // SAFETY: Every symbol/native owner has ended before this module owner.
            assert_eq!(unsafe { libc::dlclose(self.0.as_ptr()) }, 0);
        }
    }
}

#[repr(C)]
struct OpaqueBuffer {
    _opaque: [u8; 0],
}
#[repr(C)]
struct OpaqueString {
    _opaque: [u8; 0],
}
type UuidFunction = unsafe extern "C" fn(*mut c_void, *mut OpaqueString) -> u8;
type FeaturesFunction = unsafe extern "C" fn(*mut c_void) -> u64;
type StringScopeFunction = unsafe extern "C" fn(*mut c_void, *mut OpaqueString) -> i32;
#[repr(C)]
#[derive(Default)]
struct StringView {
    data: *const u8,
    length: usize,
}
type BoolCallback = unsafe extern "C" fn(*mut c_void) -> u8;
type ShotCallback = unsafe extern "C" fn(*mut c_void, *mut OpaqueBuffer) -> u8;
type ScopeCallback = unsafe extern "C" fn(*mut c_void, *mut OpaqueBuffer) -> i32;
#[repr(C)]
#[derive(Default)]
struct View {
    data: *const u8,
    length: usize,
    stride: usize,
    width: i32,
    height: i32,
    kind: i32,
    channels: i32,
    references: i32,
}
struct Lib {
    unit: native_api::Api,
    metadata: metadata::Api,
    _module: loader::Module,
    abi: unsafe extern "C" fn(u32) -> usize,
    create: unsafe extern "C" fn(u8, *mut *mut c_void) -> i32,
    destroy: unsafe extern "C" fn(*mut c_void),
    configure: unsafe extern "C" fn(
        *mut c_void,
        Option<BoolCallback>,
        Option<BoolCallback>,
        Option<BoolCallback>,
        Option<ShotCallback>,
        *mut c_void,
    ) -> i32,
    boolean: unsafe extern "C" fn(*mut c_void, u32, u8, *mut u8) -> i32,
    has_capture: unsafe extern "C" fn(*mut c_void) -> u8,
    invoke: unsafe extern "C" fn(*mut c_void, *mut OpaqueBuffer, *mut u8) -> i32,
    scope: unsafe extern "C" fn(Option<ScopeCallback>, *mut c_void) -> i32,
    raw_set:
        unsafe extern "C" fn(*mut OpaqueBuffer, *mut c_void, usize, i32, i32, i32, *mut u8) -> i32,
    image: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32,
    seed: unsafe extern "C" fn(*mut *mut c_void) -> i32,
    release: unsafe extern "C" fn(*mut c_void),
    view: unsafe extern "C" fn(*mut c_void, *mut View) -> i32,
    source: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut u8) -> i32,
    fault: unsafe extern "C" fn(*mut c_void) -> i32,
    count: unsafe extern "C" fn(u32) -> u64,
    identity_abi: unsafe extern "C" fn(u32) -> usize,
    identity_configure: unsafe extern "C" fn(
        *mut c_void,
        Option<UuidFunction>,
        Option<FeaturesFunction>,
        *mut c_void,
    ) -> i32,
    has_uuid: unsafe extern "C" fn(*mut c_void) -> u8,
    uuid_invoke: unsafe extern "C" fn(*mut c_void, *mut OpaqueString, *mut u8) -> i32,
    features: unsafe extern "C" fn(*mut c_void, u8, *mut u64) -> i32,
    string_scope: unsafe extern "C" fn(Option<StringScopeFunction>, *mut c_void) -> i32,
    string_set: unsafe extern "C" fn(*mut OpaqueString, *const u8, usize) -> i32,
    string_view: unsafe extern "C" fn(*mut OpaqueString, *mut StringView) -> i32,
    uuid_output: unsafe extern "C" fn(*const u8, usize, *mut *mut c_void) -> i32,
    uuid_release: unsafe extern "C" fn(*mut c_void),
    uuid_view: unsafe extern "C" fn(*mut c_void, *mut StringView) -> i32,
    uuid_source: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut u8) -> i32,
    identity_fault: unsafe extern "C" fn(*mut c_void, u32) -> i32,
}
impl Lib {
    fn open() -> Self {
        let module = loader::Module::open(&PathBuf::from(
            env::var_os("MFS_CUSTOM_CAPTURE_LIBRARY").unwrap(),
        ));
        // SAFETY: Every type is the exact bridge.h private C declaration;
        // optional callback pointers have C's null/function representation.
        unsafe {
            Self {
                unit: native_api::Api::open(&module),
                metadata: metadata::Api::open(&module),
                abi: module.symbol(c"mfs_custom_abi"),
                create: module.symbol(c"mfs_custom_create"),
                destroy: module.symbol(c"mfs_custom_destroy"),
                configure: module.symbol(c"mfs_custom_configure"),
                boolean: module.symbol(c"mfs_custom_boolean"),
                has_capture: module.symbol(c"mfs_custom_has_capture"),
                invoke: module.symbol(c"mfs_custom_invoke"),
                scope: module.symbol(c"mfs_custom_scope"),
                raw_set: module.symbol(c"mfs_custom_raw_set"),
                image: module.symbol(c"mfs_custom_image"),
                seed: module.symbol(c"mfs_custom_seed"),
                release: module.symbol(c"mfs_custom_release"),
                view: module.symbol(c"mfs_custom_view"),
                source: module.symbol(c"mfs_custom_source"),
                fault: module.symbol(c"mfs_custom_fault"),
                count: module.symbol(c"mfs_custom_count"),
                identity_abi: module.symbol(c"mfs_custom_identity_abi"),
                identity_configure: module.symbol(c"mfs_custom_identity_configure"),
                has_uuid: module.symbol(c"mfs_custom_has_uuid"),
                uuid_invoke: module.symbol(c"mfs_custom_uuid_invoke"),
                features: module.symbol(c"mfs_custom_features"),
                string_scope: module.symbol(c"mfs_custom_string_scope"),
                string_set: module.symbol(c"mfs_custom_string_set"),
                string_view: module.symbol(c"mfs_custom_string_view"),
                uuid_output: module.symbol(c"mfs_custom_uuid_output"),
                uuid_release: module.symbol(c"mfs_custom_uuid_release"),
                uuid_view: module.symbol(c"mfs_custom_uuid_view"),
                uuid_source: module.symbol(c"mfs_custom_uuid_source"),
                identity_fault: module.symbol(c"mfs_custom_identity_fault"),
                _module: module,
            }
        }
    }
    fn count(&self, kind: u32) -> u64 {
        // SAFETY: Retained library and scalar native ledger.
        unsafe { (self.count)(kind) }
    }
    fn seed(&self) -> Image<'_> {
        let mut out = std::ptr::null_mut();
        // SAFETY: Initialized failure output; success transfers one Mat head.
        assert_eq!(unsafe { (self.seed)(&mut out) }, 0);
        Image {
            pointer: NonNull::new(out).unwrap(),
            lib: self,
        }
    }
}

#[path = "identity.rs"]
mod identity;
#[path = "images.rs"]
mod images;
#[path = "metadata.rs"]
mod metadata;
#[path = "native-api.rs"]
mod native_api;
#[path = "raw.rs"]
mod raw;
struct Image<'a> {
    pointer: NonNull<c_void>,
    lib: &'a Lib,
}
impl Image<'_> {
    fn view(&self) -> View {
        let mut view = View::default();
        // SAFETY: Owned live Mat head and full writable C structure; native
        // view validates span. Storage is not mutated while bytes are borrowed.
        assert_eq!(
            unsafe { (self.lib.view)(self.pointer.as_ptr(), &mut view) },
            0
        );
        view
    }
    fn pixels(&self) -> &[u8] {
        let view = self.view();
        if view.length == 0 {
            assert!(view.data.is_null());
            return &[];
        }
        assert!(!view.data.is_null());
        // SAFETY: Native checked readable span, initialized owned clone, real
        // Mat owner and module retained for this borrow. No writable alias.
        unsafe { std::slice::from_raw_parts(view.data, view.length) }
    }
}
impl Drop for Image<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one native Mat head transferred to this owner. Module
        // remains borrowed until the matching native destructor finishes.
        unsafe { (self.lib.release)(self.pointer.as_ptr()) }
    }
}
struct Buffer(NonNull<OpaqueBuffer>);
struct Native<'a> {
    context: NonNull<c_void>,
    lib: &'a Lib,
    unit_destination: Cell<(u32, *mut c_void)>,
    _channel: PhantomData<Rc<()>>,
}
impl<'a> Native<'a> {
    fn new(lib: &'a Lib, table: bool) -> Self {
        let mut out = std::ptr::null_mut();
        // SAFETY: Full initialized output, one owned stable native context.
        assert_eq!(unsafe { (lib.create)(u8::from(table), &mut out) }, 0);
        Self {
            context: NonNull::new(out).unwrap(),
            lib,
            unit_destination: Cell::new((0, std::ptr::null_mut())),
            _channel: PhantomData,
        }
    }
    /// Caller keeps state alive at the same address on this channel until callbacks are cleared or
    /// this context is destroyed; no invocation may overlap that destruction.
    unsafe fn configure(&self, state: &State<'_>, present: bool) {
        let boolean = present.then_some(boolean_callback as BoolCallback);
        let shot = present.then_some(shot_callback as ShotCallback);
        // SAFETY: The caller establishes the retained callback state lifetime;
        // actual upstream C++ table and function types are compiled in bridge.
        assert_eq!(
            unsafe {
                (self.lib.configure)(
                    self.context.as_ptr(),
                    boolean,
                    boolean,
                    boolean,
                    shot,
                    (state as *const State<'_>).cast_mut().cast(),
                )
            },
            0
        );
    }
    fn boolean(&self, op: u32) -> Option<Result<u8, i32>> {
        let mut result = 0;
        // SAFETY: Context lives on this owner channel; callback state is held
        // by configure's caller. No Rust mutable state borrow spans invocation.
        let status = unsafe { (self.lib.boolean)(self.context.as_ptr(), op, 0, &mut result) };
        (status != -1).then_some(if status == 0 { Ok(result) } else { Err(status) })
    }
    fn shot(&self, buffer: &mut Buffer) -> Result<u8, i32> {
        let mut result = 0;
        // SAFETY: Synchronous invocation on actual live stack ImageBuffer and
        // borrowed callback table/state; private C++ boundary contains throws.
        let status =
            unsafe { (self.lib.invoke)(self.context.as_ptr(), buffer.0.as_ptr(), &mut result) };
        if status == 0 { Ok(result) } else { Err(status) }
    }
}
impl<'a> Boundary for Native<'a> {
    type Buffer = Buffer;
    type Image = Image<'a>;
    type Error = i32;
    fn connect(&self) -> Option<Result<u8, i32>> {
        self.boolean(0)
    }
    fn connected(&self) -> Option<Result<u8, i32>> {
        self.boolean(1)
    }
    fn inactive(&self) -> Option<Result<u8, i32>> {
        self.boolean(2)
    }
    fn screenshot(&self) -> Option<Screenshot<Self>> {
        // SAFETY: Read actual table presence on the retained native context.
        (unsafe { (self.lib.has_capture)(self.context.as_ptr()) } != 0).then_some(Self::shot)
    }
    fn image(&self, buffer: &Buffer) -> Result<Image<'a>, i32> {
        let mut output = std::ptr::null_mut();
        // SAFETY: Actual stack buffer remains alive here. The returned Mat
        // retains its real storage; only an opaque acceptance Mat head allocates.
        let status = unsafe { (self.lib.image)(buffer.0.as_ptr().cast(), &mut output) };
        if status != 0 {
            assert!(output.is_null());
            return Err(status);
        }
        Ok(Image {
            pointer: NonNull::new(output).ok_or(1)?,
            lib: self.lib,
        })
    }
    fn publish_image(&self, output: &mut Image<'a>, mut image: Image<'a>) -> Result<(), i32> {
        self.publish_cpp_image(&mut image)?;
        *output = image;
        Ok(())
    }
    fn with_buffer<T>(&self, action: impl FnOnce(&mut Buffer) -> Result<T, i32>) -> Result<T, i32> {
        struct Call<F, T> {
            action: Option<F>,
            result: Option<Result<T, i32>>,
        }
        unsafe extern "C" fn execute<F: FnOnce(&mut Buffer) -> Result<T, i32>, T>(
            argument: *mut c_void,
            buffer: *mut OpaqueBuffer,
        ) -> i32 {
            // SAFETY: One synchronous call with the exact live stack Call
            // type; no pointer/reference retained by the native scope.
            let call = unsafe { &mut *argument.cast::<Call<F, T>>() };
            let Some(action) = call.action.take() else {
                return 1;
            };
            let Some(buffer) = NonNull::new(buffer) else {
                return 1;
            };
            call.result = Some(
                catch_unwind(AssertUnwindSafe(|| action(&mut Buffer(buffer)))).unwrap_or(Err(-100)),
            );
            0
        }
        fn run<F: FnOnce(&mut Buffer) -> Result<T, i32>, T>(
            native: &Native<'_>,
            action: F,
        ) -> Result<T, i32> {
            let mut call = Call {
                action: Some(action),
                result: None,
            };
            // SAFETY: Stack state and closure remain exclusively borrowed
            // throughout one native stack ImageBuffer scope. execute catches
            // Rust panics; native scope contains C++ exceptions and releases it.
            let status = unsafe {
                (native.lib.scope)(Some(execute::<F, T>), (&mut call as *mut Call<F, T>).cast())
            };
            if status != 0 {
                return Err(status);
            }
            call.result.ok_or(1)?
        }
        run(self, action)
    }
}
impl Drop for Native<'_> {
    fn drop(&mut self) {
        // SAFETY: All synchronous calls ended on the owner channel. Callback
        // state still lives; default source manager destruction invokes none.
        unsafe { (self.lib.destroy)(self.context.as_ptr()) }
    }
}
struct State<'a> {
    lib: &'a Lib,
    thread: ThreadId,
    code: u8,
    populate: bool,
    kind: i32,
    expected: Vec<u8>,
    input: RefCell<Vec<u8>>,
    calls: Cell<u32>,
    panic: Cell<bool>,
}
unsafe extern "C" fn boolean_callback(argument: *mut c_void) -> u8 {
    // SAFETY: configure's caller retains this exact state for all callbacks.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    match catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        // A success screenshot with code=1 deliberately has false connection
        // callbacks, proving that capture has no manager-connected gate.
        if state.code == 1 { 0 } else { state.code }
    })) {
        Ok(value) => value,
        Err(_) => {
            state.panic.set(true);
            0
        }
    }
}
unsafe extern "C" fn shot_callback(argument: *mut c_void, buffer: *mut OpaqueBuffer) -> u8 {
    // SAFETY: Exact live state; original/Rust call passes a real borrowed
    // ImageBuffer. No native buffer pointer survives this invocation.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    match catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        state.calls.set(state.calls.get().checked_add(1).unwrap());
        if state.populate {
            let mut bytes = state.input.borrow_mut();
            let mut result = 0;
            // SAFETY: Sized initialized owned input, exact checked dimensions
            // and type; actual raw setter clones it before the overwrite below.
            assert_eq!(
                unsafe {
                    (state.lib.raw_set)(
                        buffer,
                        bytes.as_mut_ptr().cast(),
                        bytes.len(),
                        7,
                        5,
                        state.kind,
                        &mut result,
                    )
                },
                0
            );
            assert_eq!(result, 1);
            bytes.fill(0xa5);
        }
        state.code
    })) {
        Ok(value) => value,
        Err(_) => {
            state.panic.set(true);
            0
        }
    }
}

#[test]
#[ignore = "requires explicit compiled original Custom/ImageBuffer/OpenCV model"]
fn preserves_original_native_callbacks_pixels_empty_failure_and_owners() {
    let lib = Lib::open();
    let layouts = [
        size_of::<View>(),
        align_of::<View>(),
        offset_of!(View, data),
        offset_of!(View, length),
        offset_of!(View, stride),
        offset_of!(View, width),
        offset_of!(View, height),
        offset_of!(View, kind),
        offset_of!(View, channels),
        offset_of!(View, references),
        1,
    ];
    for (index, layout) in layouts.into_iter().enumerate() {
        // SAFETY: Scalar native header/compiler ABI query on retained module.
        assert_eq!(unsafe { (lib.abi)(u32::try_from(index).unwrap()) }, layout);
    }
    // CV type packing is owned by the actual selected OpenCV headers, not a
    // frozen OpenCV 4 integer layout. These private adapter values never become
    // SDK format/ABI enums. Every input below uses the real compiled constants.
    let kinds = [11, 12, 13].map(|index| {
        // SAFETY: Header-compiled scalar CV_8UC1/3/4 query on the live library.
        i32::try_from(unsafe { (lib.abi)(index) }).unwrap()
    });
    println!(
        "native Custom CV_8UC1={} CV_8UC3={} CV_8UC4={}",
        kinds[0], kinds[1], kinds[2]
    );
    assert_eq!(lib.count(0), 0);
    assert_eq!(lib.count(1), 0);
    let directory = PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut rows = String::new();
    let mut held = Vec::new();
    let mut cases = 0;
    for table in [false, true] {
        for present in [false, true] {
            for code in [0, 1, 2, 255] {
                for populate in [false, true] {
                    for (channels, kind) in [(1, kinds[0]), (3, kinds[1]), (4, kinds[2])] {
                        let expected: Vec<u8> = (0..7 * 5 * channels)
                            .map(|i| u8::try_from((i * 29 + 11) % 251 + 1).unwrap())
                            .collect();
                        let state = State {
                            lib: &lib,
                            thread: std::thread::current().id(),
                            code,
                            populate,
                            kind,
                            input: RefCell::new(expected.clone()),
                            expected,
                            calls: Cell::new(0),
                            panic: Cell::new(false),
                        };
                        let native = Native::new(&lib, table);
                        // SAFETY: state declared first and outlives native/manager/calls;
                        // every operation occurs synchronously on this same test channel.
                        unsafe { native.configure(&state, present) };
                        let manager = Manager::new(Some(&native));
                        let source = lib.seed();
                        let mut rust = lib.seed();
                        for operation in 0..3 {
                            let mut original = 0;
                            // SAFETY: Live original model, actual borrowed table/context.
                            assert_eq!(
                                unsafe {
                                    (lib.boolean)(
                                        native.context.as_ptr(),
                                        operation,
                                        1,
                                        &mut original,
                                    )
                                },
                                0
                            );
                            let result = match operation {
                                0 => manager.connect(),
                                1 => manager.connected(),
                                _ => manager.inactive(),
                            }
                            .unwrap();
                            assert_eq!(result, original != 0);
                        }
                        let source_old = source.view().data;
                        let rust_old = rust.view().data;
                        let mut original = 0;
                        // SAFETY: Distinct owning output Mat; callback input/state alive;
                        // original source body plus real stack buffer execute once.
                        assert_eq!(
                            unsafe {
                                (lib.source)(
                                    native.context.as_ptr(),
                                    source.pointer.as_ptr(),
                                    &mut original,
                                )
                            },
                            0
                        );
                        state.input.borrow_mut().copy_from_slice(&state.expected);
                        let result = manager.capture(&mut rust).unwrap();
                        let success = table && present && code != 0;
                        assert_eq!((original != 0, result), (success, success));
                        assert_eq!(state.calls.get(), if table && present { 2 } else { 0 });
                        assert!(!state.panic.get());
                        assert_eq!(source.pixels(), rust.pixels());
                        let a = source.view();
                        let b = rust.view();
                        assert_eq!(
                            (
                                a.width,
                                a.height,
                                a.kind,
                                a.channels,
                                a.length,
                                a.stride,
                                a.references
                            ),
                            (
                                b.width,
                                b.height,
                                b.kind,
                                b.channels,
                                b.length,
                                b.stride,
                                b.references
                            )
                        );
                        if !success {
                            assert_eq!(a.data, source_old);
                            assert_eq!(b.data, rust_old);
                        } else if populate {
                            assert_eq!(source.pixels(), state.expected);
                            assert_eq!(a.references, 1);
                            assert_eq!(
                                state.input.borrow().as_slice(),
                                vec![0xa5; state.expected.len()]
                            );
                        } else {
                            assert!(source.pixels().is_empty());
                            assert_eq!(a.references, 0);
                        }
                        std::fs::write(
                            directory.join(format!("case-{cases}.source.bin")),
                            source.pixels(),
                        )
                        .unwrap();
                        std::fs::write(
                            directory.join(format!("case-{cases}.rust.bin")),
                            rust.pixels(),
                        )
                        .unwrap();
                        rows.push_str(&format!(
                            "{cases}\t{}\t{}\t{code}\t{}\t{channels}\t{}\t{}\t{}\t{}\n",
                            u8::from(table),
                            u8::from(present),
                            u8::from(populate),
                            u8::from(success),
                            a.length,
                            a.references,
                            state.calls.get()
                        ));
                        let saved_pixels = source.pixels().to_vec();
                        drop(native);
                        drop(state);
                        assert_eq!(lib.count(1), 0);
                        assert_eq!(lib.count(2), lib.count(3));
                        if success {
                            held.push((source, saved_pixels.clone()));
                            held.push((rust, saved_pixels));
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 96);
    // Actual C++ callback exception in both paths. Preserve previous pixels and
    // require all native stack scopes to end before a separate subsequent call.
    {
        let expected = vec![71; 105];
        let state = State {
            lib: &lib,
            thread: std::thread::current().id(),
            code: 1,
            populate: true,
            kind: kinds[1],
            input: RefCell::new(expected.clone()),
            expected,
            calls: Cell::new(0),
            panic: Cell::new(false),
        };
        let native = Native::new(&lib, true);
        // SAFETY: State outlives context/calls, all on this owner channel.
        unsafe { native.configure(&state, true) };
        let manager = Manager::new(Some(&native));
        let source = lib.seed();
        let mut rust = lib.seed();
        let source_old = source.view().data;
        let rust_old = rust.view().data;
        let prior_pixels = source.pixels().to_vec();
        // SAFETY: Controlled native callback; valid context and output owner.
        assert_eq!(unsafe { (lib.fault)(native.context.as_ptr()) }, 0);
        let mut value = 0;
        assert_eq!(
            unsafe { (lib.source)(native.context.as_ptr(), source.pointer.as_ptr(), &mut value) },
            3
        );
        assert_eq!(manager.capture(&mut rust), Err(3));
        assert_eq!(source.view().data, source_old);
        assert_eq!(rust.view().data, rust_old);
        assert_eq!(source.pixels(), prior_pixels);
        assert_eq!(rust.pixels(), prior_pixels);
        assert_eq!(lib.count(2), lib.count(3));
        // Observe changes in the SAME borrowed upstream callback table.
        unsafe { native.configure(&state, false) };
        assert!(!manager.capture(&mut rust).unwrap());
        unsafe { native.configure(&state, true) };
        // SAFETY: New nonthrowing callback in the same live source context;
        // this is a distinct recovery case after the asserted native failure.
        assert_eq!(
            unsafe { (lib.source)(native.context.as_ptr(), source.pointer.as_ptr(), &mut value) },
            0
        );
        assert_eq!(value, 1);
        assert_eq!(source.pixels(), state.expected);
        state.input.borrow_mut().copy_from_slice(&state.expected);
        assert!(manager.capture(&mut rust).unwrap());
        assert_eq!(rust.pixels(), state.expected);
        assert_eq!(state.calls.get(), 2);
        assert!(!state.panic.get());
    }
    for (image, expected) in &held {
        assert_eq!(image.pixels(), expected);
        let view = image.view();
        if view.length != 0 {
            assert_eq!(view.references, 1);
        }
    }
    let retained = held.len();
    drop(held);
    assert_eq!(lib.count(0), 0);
    assert_eq!(lib.count(1), 0);
    assert_eq!(lib.count(2), lib.count(3));
    std::fs::write(directory.join("cases.tsv"), rows).unwrap();
    // SAFETY: Scalar actual original table width/field-offset observations.
    std::fs::write(
        directory.join("abi.txt"),
        format!(
            "callback_table_size={}\nscreencap_offset={}\npointer_size={}\nCV_8UC1={}\nCV_8UC3={}\nCV_8UC4={}\n",
            unsafe { (lib.abi)(14) },
            unsafe { (lib.abi)(15) },
            unsafe { (lib.abi)(16) },kinds[0],kinds[1],kinds[2]
        ),
    )
    .unwrap();
    std::fs::write(directory.join("stats.tsv"), format!(
        "native_mat_heads\t{}\nnative_contexts\t{}\nrust_adapter_scopes_entered\t{}\nrust_adapter_scopes_exited\t{}\nretained_mat_heads\t{retained}\n",
        lib.count(0),lib.count(1),lib.count(2),lib.count(3),
    )).unwrap();
    println!(
        "native Custom cases={cases} source_rust_captures=192 retained={retained} real_imagebuffer=1 caller_callbacks=1 false_output_preserved=1 true_empty=1 raw_setter_clone=1 borrowed_table_changes=1 native_exception=1 subsequent_capture=1 native_mat_heads_closed=1 passed"
    );
}
