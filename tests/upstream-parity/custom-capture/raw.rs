// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Original raw C setter through both managers for every compiled raw type.
use super::*;
use std::{fs, io::Write};

struct Api {
    catalog: unsafe extern "C" fn(u32) -> usize,
    kind: unsafe extern "C" fn(u32, u32, *mut i32, *mut usize, *mut usize) -> i32,
}
impl Api {
    fn open(lib: &Lib) -> Self {
        // SAFETY: Exact private bridge.h types, retained native module.
        unsafe {
            Self {
                catalog: lib._module.symbol(c"mfs_custom_raw_catalog"),
                kind: lib._module.symbol(c"mfs_custom_raw_type"),
            }
        }
    }
    fn kind(&self, depth: u32, channels: u32) -> (i32, usize, usize) {
        let (mut kind, mut scalar, mut element) = (-1, 0, 0);
        // SAFETY: Writable initialized exact outputs, bounded header indices.
        assert_eq!(
            unsafe { (self.kind)(depth, channels, &mut kind, &mut scalar, &mut element) },
            0
        );
        (kind, scalar, element)
    }
}

struct RawState<'a> {
    lib: &'a Lib,
    thread: ThreadId,
    kind: i32,
    width: i32,
    height: i32,
    code: u8,
    input: RefCell<Vec<u64>>,
    length: usize,
    calls: Cell<u32>,
    panic: Cell<bool>,
}
unsafe extern "C" fn callback(argument: *mut c_void, buffer: *mut OpaqueBuffer) -> u8 {
    // SAFETY: configure's caller retains this exact stable state until the
    // synchronous callback ends and the native callback table is cleared.
    let state = unsafe { &*argument.cast::<RawState<'_>>() };
    match catch_unwind(AssertUnwindSafe(|| {
        assert_eq!(state.thread, std::thread::current().id());
        assert_eq!(state.calls.replace(1), 0);
        let mut input = state.input.borrow_mut();
        let mut result = 0;
        // SAFETY: Aligned initialized owned u64 storage covers the declared
        // checked span. No byte borrow overlaps native reads. The real setter
        // clones before return; overwrite verifies that independent ownership.
        assert_eq!(
            unsafe {
                (state.lib.raw_set)(
                    buffer,
                    input.as_mut_ptr().cast(),
                    state.length,
                    state.width,
                    state.height,
                    state.kind,
                    &mut result,
                )
            },
            0
        );
        assert_eq!(result, 1);
        input.fill(0xa5a5_a5a5_a5a5_a5a5);
        state.code
    })) {
        Ok(code) => code,
        Err(_) => {
            state.panic.set(true);
            0
        }
    }
}

fn pattern(depth: u32, channels: u32, length: usize, code: u8) -> Vec<u8> {
    (0..length)
        .map(|index| {
            // Bool bytes remain canonical; other types are compared as their
            // original raw representation without arithmetic/color conversion.
            let value = index * 29
                + usize::try_from(depth).unwrap() * 17
                + usize::try_from(channels).unwrap() * 7
                + usize::from(code);
            u8::try_from(if depth == 9 {
                value % 2
            } else {
                value % 251 + 1
            })
            .unwrap()
        })
        .collect()
}
fn state<'a>(lib: &'a Lib, kind: i32, code: u8, expected: &[u8]) -> RawState<'a> {
    let mut words = vec![0_u64; expected.len().div_ceil(size_of::<u64>())];
    // SAFETY: Writable initialized u64 allocation has this exact byte span;
    // byte writes preserve valid u64 representations and the borrow ends here.
    unsafe {
        std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), expected.len())
            .copy_from_slice(expected);
    }
    RawState {
        lib,
        thread: std::thread::current().id(),
        kind,
        width: 3,
        height: 2,
        code,
        input: RefCell::new(words),
        length: expected.len(),
        calls: Cell::new(0),
        panic: Cell::new(false),
    }
}

#[test]
#[ignore = "requires exact original raw helper, compiled type catalog and native image owners"]
fn preserves_every_compiled_raw_type_callback_bytes_and_native_owners() {
    let lib = Lib::open();
    let api = Api::open(&lib);
    // These values describe the actual selected header layout, not SDK enums.
    let catalog = [0, 1, 2, 3].map(|i| unsafe { (api.catalog)(i) });
    assert_eq!(
        catalog,
        [13, 128, 5, 4095],
        "review a changed native type catalog"
    );
    let directory = PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("raw");
    fs::create_dir(&directory).unwrap();
    // Invalid positive-sized spans/types must stop before the original setter.
    let guard_owner = Native::new(&lib, true);
    let kind = api.kind(0, 1).0;
    let widest = api.kind(6, 128).0;
    let mut guard_rows = String::new();
    guard_owner
        .with_buffer(|buffer| {
            let mut bytes = [1_u8, 2, 3, 4, 5, 6];
            let mut result = 0;
            // SAFETY: Exact initialized six-byte input and 3x2 8UC1 layout.
            assert_eq!(
                unsafe {
                    (lib.raw_set)(
                        buffer.0.as_ptr(),
                        bytes.as_mut_ptr().cast(),
                        bytes.len(),
                        3,
                        2,
                        kind,
                        &mut result,
                    )
                },
                0
            );
            assert_eq!(result, 1);
            let before = lib.count(7);
            for (label, length, width, height, raw_kind) in [
                ("short-span", 5, 3, 2, kind),
                ("negative-type", 6, 3, 2, -1),
                ("reserved-depth", 6, 3, 2, 13),
                ("max-reserved-depth", 6, 3, 2, 31),
                ("type-bits-outside-mask", 6, 3, 2, 4096),
                ("huge-span", 0, i32::MAX, i32::MAX, widest),
            ] {
                result = 255;
                // SAFETY: Real buffer and result stay valid. Inconsistent input
                // declarations are rejected before any native read, independently
                // confirmed by the actual original-helper invocation ledger.
                let status = unsafe {
                    (lib.raw_set)(
                        buffer.0.as_ptr(),
                        bytes.as_mut_ptr().cast(),
                        length,
                        width,
                        height,
                        raw_kind,
                        &mut result,
                    )
                };
                assert_eq!((status, result, lib.count(7)), (1, 0, before));
                assert_eq!(guard_owner.image(buffer).unwrap().pixels(), bytes);
                guard_rows.push_str(&format!("{label}\t{status}\t{result}\t0\n"));
            }
            Ok(())
        })
        .unwrap();
    drop(guard_owner);
    assert_eq!((lib.count(0), lib.count(1)), (0, 0));
    fs::write(directory.join("rejections.tsv"), guard_rows).unwrap();
    fs::write(
        directory.join("catalog.txt"),
        format!(
            "depths={}\nchannels={}\nchannel_shift={}\ntype_mask={}\n",
            catalog[0], catalog[1], catalog[2], catalog[3]
        ),
    )
    .unwrap();
    let mut types = fs::File::create(directory.join("types.tsv")).unwrap();
    let mut cases = fs::File::create(directory.join("cases.tsv")).unwrap();
    let mut payloads = fs::File::create(directory.join("captured.bin")).unwrap();
    let mut retained = fs::File::create(directory.join("held.bin")).unwrap();
    let mut held = Vec::new();
    let before_entered = lib.count(2);
    let before_exited = lib.count(3);
    let before_raw = lib.count(7);
    assert_eq!((lib.count(0), lib.count(1)), (0, 0));
    let mut index = 0;
    for depth in 0..u32::try_from(catalog[0]).unwrap() {
        let native = Native::new(&lib, true);
        for channels in 1..=u32::try_from(catalog[1]).unwrap() {
            let (kind, scalar, element) = api.kind(depth, channels);
            assert_eq!(
                element,
                scalar
                    .checked_mul(usize::try_from(channels).unwrap())
                    .unwrap()
            );
            let stride = element.checked_mul(3).unwrap();
            let span = stride.checked_mul(2).unwrap();
            assert!(span <= isize::MAX as usize);
            writeln!(
                types,
                "{depth}\t{channels}\t{kind}\t{scalar}\t{element}\t{stride}\t{span}"
            )
            .unwrap();
            for code in [0, 1, 255] {
                let expected = pattern(depth, channels, span, code);
                for side in ["source", "rust"] {
                    let state = state(&lib, kind, code, &expected);
                    // SAFETY: Stable state and owning module stay alive through
                    // all native calls; no concurrent table access/destruction.
                    assert_eq!(
                        unsafe {
                            (lib.configure)(
                                native.context.as_ptr(),
                                None,
                                None,
                                None,
                                Some(callback),
                                (&state as *const RawState<'_>).cast_mut().cast(),
                            )
                        },
                        0
                    );
                    let mut image = lib.seed();
                    let old = image.view().data;
                    let success = if side == "source" {
                        let mut result = 0;
                        // SAFETY: Separate owned destination; same real callback
                        // table and original manager body, invoked once.
                        assert_eq!(
                            unsafe {
                                (lib.source)(
                                    native.context.as_ptr(),
                                    image.pointer.as_ptr(),
                                    &mut result,
                                )
                            },
                            0
                        );
                        result != 0
                    } else {
                        Manager::new(Some(&native)).capture(&mut image).unwrap()
                    };
                    assert_eq!(success, code != 0);
                    assert_eq!(state.calls.get(), 1);
                    assert!(!state.panic.get());
                    // SAFETY: Calls ended; clear the native borrowed state before
                    // freeing its input. Later reads retain only real Mat owners.
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
                    let view = image.view();
                    let output = if code == 0 {
                        [9, 17, 23].repeat(35)
                    } else {
                        expected.clone()
                    };
                    assert_eq!(image.pixels(), output);
                    if code == 0 {
                        assert_eq!(view.data, old);
                    } else {
                        assert_ne!(view.data, old);
                        assert_eq!(
                            (
                                view.width,
                                view.height,
                                view.kind,
                                view.channels,
                                view.stride,
                                view.length
                            ),
                            (3, 2, kind, i32::try_from(channels).unwrap(), stride, span)
                        );
                    }
                    assert_eq!(view.references, 1);
                    payloads.write_all(image.pixels()).unwrap();
                    writeln!(cases, "{index}\t{side}\t{depth}\t{channels}\t{kind}\t{code}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        u8::from(success), view.width, view.height, view.kind, view.channels, view.stride, view.length, view.references).unwrap();
                    drop(state);
                    held.push((image, output));
                }
                index += 1;
            }
        }
        drop(native);
        assert_eq!(lib.count(1), 0);
        assert_eq!(lib.count(2), lib.count(3));
    }
    assert_eq!(index, 4992);
    assert_eq!(held.len(), 9984);
    assert_eq!(lib.count(2) - before_entered, 4992);
    assert_eq!(lib.count(3) - before_exited, 4992);
    assert_eq!(lib.count(7) - before_raw, 9984);
    for (image, expected) in &held {
        assert_eq!(image.pixels(), expected);
        assert_eq!(image.view().references, 1);
        retained.write_all(image.pixels()).unwrap();
    }
    let cpu = held
        .iter()
        .map(|(image, expected)| (image.pixels().to_vec(), expected.clone()))
        .collect::<Vec<_>>();
    drop(held);
    assert_eq!((lib.count(0), lib.count(1)), (0, 0));
    fs::write(directory.join("stats.tsv"), "types\t1664\ncases\t4992\ncaptures\t9984\noriginal_raw_setter_calls\t9984\ncontexts\t13\nrust_scopes_entered\t4992\nrust_scopes_exited\t4992\nretained_native_images\t9984\nnative_images_after_release\t0\nnative_contexts_after_release\t0\n").unwrap();
    drop(lib);
    let after_unload = std::thread::spawn(move || {
        cpu.into_iter()
            .map(|(bytes, expected)| {
                assert_eq!(bytes, expected);
                bytes.len()
            })
            .sum::<usize>()
    })
    .join()
    .unwrap();
    assert_eq!(
        after_unload,
        fs::metadata(directory.join("held.bin")).unwrap().len() as usize
    );
    println!(
        "Custom raw types=1664 cases=4992 source_rust_captures=9984 native_owners_retained=9984 rust_scopes=4992 exact_type_catalog=1 input_actions=0 passed"
    );
}
