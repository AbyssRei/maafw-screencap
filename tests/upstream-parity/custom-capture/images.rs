// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual fixed encoded/resize helpers inside the same capture callback.
use super::*;
use std::{fs, io::Write, path::Path};

struct Api {
    fixture: unsafe extern "C" fn(u32, *mut *mut c_void) -> i32,
    set: unsafe extern "C" fn(*mut OpaqueBuffer, *mut u8, usize, *mut u8) -> i32,
    encoded: unsafe extern "C" fn(*mut OpaqueBuffer, *mut StringView) -> i32,
    resize: unsafe extern "C" fn(*mut OpaqueBuffer, i32, i32, *mut u8) -> i32,
    decode: unsafe extern "C" fn(*const u8, usize, *mut *mut c_void) -> i32,
}
impl Api {
    fn open(lib: &Lib) -> Self {
        // SAFETY: Exact private bridge.h types; lib retains the native module.
        unsafe {
            Self {
                fixture: lib._module.symbol(c"mfs_custom_codec_fixture"),
                set: lib._module.symbol(c"mfs_custom_encoded_set"),
                encoded: lib._module.symbol(c"mfs_custom_encoded_view"),
                resize: lib._module.symbol(c"mfs_custom_resize"),
                decode: lib._module.symbol(c"mfs_custom_codec_decode"),
            }
        }
    }
    fn fixture(&self, lib: &Lib, channels: u32) -> Vec<u8> {
        let mut output = std::ptr::null_mut();
        // SAFETY: Initialized owner output, small compiled self-owned fixture.
        assert_eq!(unsafe { (self.fixture)(channels, &mut output) }, 0);
        struct Owner<'a>(&'a Lib, NonNull<c_void>);
        impl Drop for Owner<'_> {
            fn drop(&mut self) {
                // SAFETY: Exact UuidOutput owner returned by codec_fixture.
                unsafe { (self.0.uuid_release)(self.1.as_ptr()) }
            }
        }
        let owner = Owner(lib, NonNull::new(output).unwrap());
        let mut view = StringView::default();
        // SAFETY: Live native encoded-byte owner through the completed copy.
        assert_eq!(unsafe { (lib.uuid_view)(owner.1.as_ptr(), &mut view) }, 0);
        copy(&view)
    }
    fn cache(&self, buffer: *mut OpaqueBuffer) -> Vec<u8> {
        let mut view = StringView::default();
        // SAFETY: Exact live callback buffer. No mutation until copy finishes.
        assert_eq!(unsafe { (self.encoded)(buffer, &mut view) }, 0);
        copy(&view)
    }
    fn decode<'a>(&self, lib: &'a Lib, bytes: &[u8]) -> Image<'a> {
        let mut output = std::ptr::null_mut();
        // SAFETY: Initialized readable CPU bytes retained through synchronous
        // diagnostic decode; native Image owns its separate decoded storage.
        assert_eq!(
            unsafe { (self.decode)(bytes.as_ptr(), bytes.len(), &mut output) },
            0
        );
        Image {
            pointer: NonNull::new(output).unwrap(),
            lib,
        }
    }
}
fn copy(view: &StringView) -> Vec<u8> {
    if view.length == 0 {
        return Vec::new();
    }
    assert!(!view.data.is_null() && view.length <= isize::MAX as usize);
    // SAFETY: Callers retain the actual immutable native cache/string owner
    // throughout this copy and do not mutate its backing storage concurrently.
    unsafe { std::slice::from_raw_parts(view.data, view.length) }.to_vec()
}

#[derive(Clone, Copy)]
struct Case {
    input: usize,
    resize: usize,
    code: u8,
}
impl Case {
    fn new(index: usize) -> Self {
        Self {
            input: index / 24,
            resize: index / 3 % 8,
            code: [0, 1, 255][index % 3],
        }
    }
    fn dimensions(self) -> (usize, usize) {
        if self.input == 3 || self.input == 4 {
            return (0, 0);
        }
        match self.resize {
            2 => (16, 12),
            3..=5 => (4, 3),
            _ => (8, 6),
        }
    }
    fn base_pixels(self) -> Vec<u8> {
        pattern(self.input == 0, 8, 6)
    }
    fn pixels(self) -> Vec<u8> {
        let (width, height) = self.dimensions();
        pattern(self.input == 0, width, height)
    }
}
fn pattern(gray: bool, width: usize, height: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            // Exact integer-ratio INTER_AREA: upsample repeats each source
            // cell; downsample averages 2x2 integers with no rounding ties.
            let (sx, sy, half) = match width {
                16 => (x / 2, y / 2, false),
                4 => (2 * x, 2 * y, true),
                _ => (x, y, false),
            };
            let pixel = if gray {
                let value = 12 + 8 * sx + 4 * sy + usize::from(half) * 6;
                [value; 3]
            } else {
                [
                    12 + 8 * sx + usize::from(half) * 4,
                    20 + 8 * sy + usize::from(half) * 4,
                    28 + 4 * (sx + sy) + usize::from(half) * 4,
                ]
            };
            bytes.extend(pixel.map(|value| u8::try_from(value).unwrap()));
        }
    }
    bytes
}
#[derive(Default)]
struct Observation {
    set_status: i32,
    set_result: u8,
    resize_status: i32,
    resize_result: u8,
    before: Vec<u8>,
    after: Vec<u8>,
    before_pixels: Vec<u8>,
    after_pixels: Vec<u8>,
    before_layout: String,
    after_layout: String,
}
struct State<'a> {
    lib: &'a Lib,
    api: &'a Api,
    case: Case,
    input: RefCell<Vec<u8>>,
    kind: i32,
    calls: Cell<usize>,
    panic: Cell<bool>,
    thread: ThreadId,
    observation: RefCell<Observation>,
}
fn layout(image: &Image<'_>) -> String {
    let view = image.view();
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        view.width,
        view.height,
        view.kind,
        view.channels,
        view.stride,
        view.length,
        view.references
    )
}
fn check(image: &Image<'_>, expected: &[u8], width: usize, height: usize, kind: i32) {
    let view = image.view();
    assert_eq!(
        (view.width, view.height),
        (
            i32::try_from(width).unwrap(),
            i32::try_from(height).unwrap()
        )
    );
    if !expected.is_empty() {
        assert_eq!(
            (view.kind, view.channels, view.stride),
            (kind, 3, width.checked_mul(3).unwrap())
        );
        assert!(view.references > 0);
    }
    assert_eq!(image.pixels(), expected);
}
unsafe extern "C" fn callback(argument: *mut c_void, buffer: *mut OpaqueBuffer) -> u8 {
    // SAFETY: Exact stable stack State retained until callbacks are cleared.
    let state = unsafe { &*argument.cast::<State<'_>>() };
    match catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(std::thread::current().id(), state.thread);
        state.calls.set(state.calls.get().checked_add(1).unwrap());
        let mut raw = pattern(false, 8, 6);
        let mut result = 0;
        // SAFETY: Fully initialized exact 8-bit BGR span, real buffer; native
        // setter clones storage before this CPU input is overwritten.
        assert_eq!(
            unsafe {
                (state.lib.raw_set)(
                    buffer,
                    raw.as_mut_ptr().cast(),
                    raw.len(),
                    8,
                    6,
                    state.kind,
                    &mut result,
                )
            },
            0
        );
        assert_eq!(result, 1);
        raw.fill(0xa5);
        let mut input = state.input.borrow_mut();
        let (pointer, length) = match state.case.input {
            4 => (input.as_mut_ptr(), 0),
            5 => (std::ptr::null_mut(), 0),
            _ => (input.as_mut_ptr(), input.len()),
        };
        let mut observation = Observation::default();
        // SAFETY: Real initialized readable encoded input through native copy;
        // zero length uses a nonnull sentinel, null explicitly tests rejection.
        observation.set_status =
            unsafe { (state.api.set)(buffer, pointer, length, &mut observation.set_result) };
        assert_eq!(
            (observation.set_status, observation.set_result),
            (0, u8::from(state.case.input != 5))
        );
        input.fill(0xa5);
        observation.before = state.api.cache(buffer);
        let before = state.api.decode(state.lib, &observation.before);
        let empty = state.case.input == 3 || state.case.input == 4;
        let base = if empty {
            Vec::new()
        } else {
            state.case.base_pixels()
        };
        check(
            &before,
            &base,
            if empty { 0 } else { 8 },
            if empty { 0 } else { 6 },
            state.kind,
        );
        observation.before_pixels = before.pixels().to_vec();
        observation.before_layout = layout(&before);
        let (width, height) = [
            (0, 0),
            (8, 6),
            (16, 12),
            (4, 3),
            (0, 3),
            (4, 0),
            (0, 0),
            (-1, 2),
        ][state.case.resize];
        if state.case.resize != 0 {
            // SAFETY: Actual live image buffer; bounded sizes, including a
            // negative-size exception control contained at the C++ boundary.
            observation.resize_status = unsafe {
                (state.api.resize)(buffer, width, height, &mut observation.resize_result)
            };
        }
        let expected_status = i32::from(!empty && state.case.resize == 7) * 2;
        let expected_result = u8::from(!empty && (1..=5).contains(&state.case.resize));
        assert_eq!(
            (observation.resize_status, observation.resize_result),
            (expected_status, expected_result)
        );
        observation.after = state.api.cache(buffer);
        let after = state.api.decode(state.lib, &observation.after);
        let (width, height) = state.case.dimensions();
        check(&after, &state.case.pixels(), width, height, state.kind);
        observation.after_pixels = after.pixels().to_vec();
        observation.after_layout = layout(&after);
        *state.observation.borrow_mut() = observation;
        state.case.code
    })) {
        Ok(code) => code,
        Err(_) => {
            state.panic.set(true);
            0
        }
    }
}
fn write(directory: &Path, name: &str, suffix: &str, bytes: &[u8]) {
    fs::write(directory.join(format!("{name}.{suffix}")), bytes).unwrap();
}

#[test]
#[ignore = "requires explicit original encoded/resize helpers and real PNG codec"]
fn preserves_encoded_callback_cache_resize_failures_and_native_owners() {
    let lib = Lib::open();
    let api = Api::open(&lib);
    // SAFETY: Actual selected header BGR constant; not a frozen type packing.
    let kind = i32::try_from(unsafe { (lib.abi)(12) }).unwrap();
    let directory =
        PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("images");
    fs::create_dir(&directory).unwrap();
    let mut inputs = [1, 3, 4]
        .map(|channels| api.fixture(&lib, channels))
        .to_vec();
    inputs.extend([b"invalid-encoded-image".to_vec(), vec![0x37], Vec::new()]);
    for (index, input) in inputs.iter().enumerate() {
        write(&directory, &format!("input-{index}"), "bin", input);
    }
    let mut cases = fs::File::create(directory.join("cases.tsv")).unwrap();
    let mut held = Vec::new();
    for cycle in 0..20 {
        let native = Native::new(&lib, true);
        for index in (0..144).filter(|index| index % 20 == cycle) {
            let case = Case::new(index);
            for side in ["source", "rust"] {
                let state = State {
                    lib: &lib,
                    api: &api,
                    case,
                    input: RefCell::new(inputs[case.input].clone()),
                    kind,
                    calls: Cell::new(0),
                    panic: Cell::new(false),
                    thread: std::thread::current().id(),
                    observation: RefCell::new(Observation::default()),
                };
                // SAFETY: Stable state is exclusively retained on this owner
                // channel until both native invocation and explicit clear end.
                assert_eq!(
                    unsafe {
                        (lib.configure)(
                            native.context.as_ptr(),
                            None,
                            None,
                            None,
                            Some(callback),
                            (&state as *const State<'_>).cast_mut().cast(),
                        )
                    },
                    0
                );
                let mut image = lib.seed();
                let passed = if side == "source" {
                    let mut value = 0;
                    // SAFETY: Exact real manager, owned Mat destination and
                    // live callback state; original method contains no retries.
                    assert_eq!(
                        unsafe {
                            (lib.source)(
                                native.context.as_ptr(),
                                image.pointer.as_ptr(),
                                &mut value,
                            )
                        },
                        0
                    );
                    value != 0
                } else {
                    Manager::new(Some(&native)).capture(&mut image).unwrap()
                };
                assert!(!state.panic.get());
                assert_eq!(state.calls.get(), 1);
                assert_eq!(passed, case.code != 0);
                // SAFETY: All synchronous calls ended, state is still alive.
                assert_eq!(
                    unsafe {
                        (lib.configure)(
                            native.context.as_ptr(),
                            None,
                            None,
                            None,
                            None,
                            std::ptr::null_mut(),
                        )
                    },
                    0
                );
                let expected = if passed {
                    case.pixels()
                } else {
                    [9, 17, 23].repeat(35)
                };
                let (width, height) = if passed { case.dimensions() } else { (7, 5) };
                check(&image, &expected, width, height, kind);
                let name = format!("{index}-{side}");
                let observation = state.observation.into_inner();
                write(&directory, &name, "pixels", image.pixels());
                write(&directory, &name, "before-encoded", &observation.before);
                write(&directory, &name, "after-encoded", &observation.after);
                write(
                    &directory,
                    &name,
                    "before-pixels",
                    &observation.before_pixels,
                );
                write(&directory, &name, "after-pixels", &observation.after_pixels);
                fs::write(
                    directory.join(format!("{name}.layout")),
                    format!(
                        "{}\n{}\n{}\n",
                        layout(&image),
                        observation.before_layout,
                        observation.after_layout
                    ),
                )
                .unwrap();
                writeln!(
                    cases,
                    "{index}\t{cycle}\t{side}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    case.input,
                    case.resize,
                    case.code,
                    observation.set_status,
                    observation.set_result,
                    observation.resize_status,
                    observation.resize_result,
                    u8::from(passed)
                )
                .unwrap();
                held.push((name, image, expected, width, height));
            }
        }
        drop(native);
    }
    assert_eq!(held.len(), 288);
    assert_eq!(lib.count(1), 0);
    let mut cpu = Vec::new();
    for (name, image, expected, width, height) in held {
        check(&image, &expected, width, height, kind);
        write(&directory, &name, "held", image.pixels());
        cpu.push((image.pixels().to_vec(), expected));
    }
    assert_eq!((lib.count(0), lib.count(1), lib.count(6)), (0, 0, 0));
    assert_eq!((lib.count(2), lib.count(3)), (144, 144));
    fs::write(directory.join("stats.tsv"), "lifecycles\t20\ncaptures\t288\nrust_scopes_entered\t144\nrust_scopes_exited\t144\nretained_native_images\t288\ninspection_decodes\t576\nnative_images_after_release\t0\nnative_contexts_after_release\t0\n").unwrap();
    fs::write(directory.join("pixel-kind.txt"), format!("{kind}\n")).unwrap();
    drop(lib);
    std::thread::spawn(move || {
        for (actual, expected) in cpu {
            assert_eq!(actual, expected);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "Custom images lifecycles=20 cases=144 source_rust_captures=288 retained_native_images=288 inspection_decodes=576 input_actions=0 passed"
    );
}
