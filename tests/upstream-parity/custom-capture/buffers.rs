// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual retained native string/image C helpers, not the final SDK ABI.
use super::*;
use std::{fs, io::Write};
struct Api {
    string: unsafe extern "C" fn(*const u8, usize, *mut u64, usize, *mut *mut c_void) -> i32,
    image: unsafe extern "C" fn(*const u8, usize, i32, *mut u64, usize, *mut *mut c_void) -> i32,
}
struct Text<'a> {
    lib: &'a Lib,
    owner: *mut c_void,
}
impl Text<'_> {
    fn bytes(&self) -> &[u8] {
        let mut v = StringView::default();
        // SAFETY: Exact bridge.h, a live independent native string and module.
        assert_eq!(unsafe { (self.lib.uuid_view)(self.owner, &mut v) }, 0);
        assert!(!v.data.is_null() && v.length <= 64);
        // SAFETY: The view borrows this immutable native string, whose owner
        // and loaded release function remain live through self.
        unsafe { std::slice::from_raw_parts(v.data, v.length) }
    }
}
impl Drop for Text<'_> {
    fn drop(&mut self) {
        // SAFETY: This sole owned native value is released before its module;
        // bytes() cannot leave a live borrow across the mutable drop.
        unsafe { (self.lib.uuid_release)(self.owner) }
    }
}
const STRINGS: [&[u8]; 5] = [b"abc", b"a\0b", b"\xf0\x9f\xa7\xaa", b"", b"\xff\x80"];
#[test]
#[ignore = "requires the explicit full original MaaBuffer C++/OpenCV acceptance library; no desktop"]
fn preserves_original_string_image_helpers_nulls_clear_and_retained_storage() {
    let lib = Lib::open();
    // SAFETY: Exact private bridge.h declarations; each potentially throwing
    // original C operation remains inside the native exception boundary.
    let api = unsafe {
        let abi: unsafe extern "C" fn(u32) -> usize = lib._module.symbol(c"mfs_custom_buffer_abi");
        for (i, w) in [1, 8, 8, 1, 4, 8].into_iter().enumerate() {
            assert_eq!(abi(u32::try_from(i).unwrap()), w);
        }
        Api {
            string: lib._module.symbol(c"mfs_custom_string_buffer_probe"),
            image: lib._module.symbol(c"mfs_custom_image_buffer_probe"),
        }
    };
    assert_eq!((lib.count(0), lib.count(6)), (0, 0));
    let root = PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("buffers");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("abi.tsv"), "1\t8\t8\t1\t4\t8\n").unwrap();
    let mut strings = fs::File::create_new(root.join("strings.tsv")).unwrap();
    let mut images = fs::File::create_new(root.join("images.tsv")).unwrap();
    writeln!(strings, "cycle\tcase\tvalues").unwrap();
    writeln!(
        images,
        "cycle\tdepth\tchannels\ttype\tscalar\tspan\toffset\tvalues"
    )
    .unwrap();
    let scalar_sizes = [1, 1, 2, 2, 4, 4, 8, 2, 2, 1, 8, 8, 4];
    let mut held_strings = Vec::new();
    let mut held_images = Vec::new();
    let mut current_strings = Vec::new();
    let mut current_images = Vec::new();
    for cycle in 0..20 {
        for (case, expected) in STRINGS.into_iter().enumerate() {
            let mut input = expected.to_vec();
            input.push(0);
            let mut values = [u64::MAX; 24];
            let mut output = std::ptr::null_mut();
            assert_eq!(
                unsafe {
                    (api.string)(
                        input.as_ptr(),
                        expected.len(),
                        values.as_mut_ptr(),
                        values.len(),
                        &mut output,
                    )
                },
                0
            );
            assert!(!output.is_null());
            let text = Text {
                lib: &lib,
                owner: output,
            };
            let length = u64::try_from(expected.len()).unwrap();
            assert_eq!(
                values,
                [
                    1,
                    0,
                    1,
                    1,
                    0,
                    1,
                    1,
                    0,
                    0,
                    0,
                    1,
                    6,
                    1,
                    length,
                    u64::from(expected.is_empty()),
                    0,
                    0,
                    length,
                    1,
                    1,
                    0,
                    1,
                    length,
                    1
                ]
            );
            assert_eq!(text.bytes(), expected);
            input.fill(0xa5);
            assert_eq!(text.bytes(), expected);
            writeln!(strings, "{cycle}\t{case}\t{values:?}").unwrap();
            current_strings.extend_from_slice(text.bytes());
            held_strings.push((case, text));
        }
        for (depth, scalar) in scalar_sizes.into_iter().enumerate() {
            for channels in [1, 3, 4] {
                let kind = i32::try_from(depth + ((channels - 1) << 5)).unwrap();
                let span = 3 * 2 * channels * scalar;
                let expected = (0..span)
                    .map(|i| u8::try_from((cycle * 39 + depth * 3 + channels + i) % 251).unwrap())
                    .collect::<Vec<_>>();
                let mut input = vec![0_u64; span.div_ceil(8)];
                // SAFETY: Unique initialized u64 storage with complete writable
                // byte span; alignment covers every selected native scalar.
                let bytes = unsafe {
                    std::slice::from_raw_parts_mut(input.as_mut_ptr().cast::<u8>(), span)
                };
                bytes.copy_from_slice(&expected);
                let mut values = [u64::MAX; 32];
                let mut output = std::ptr::null_mut();
                assert_eq!(
                    unsafe {
                        (api.image)(
                            bytes.as_ptr(),
                            span,
                            kind,
                            values.as_mut_ptr(),
                            values.len(),
                            &mut output,
                        )
                    },
                    0
                );
                let image = Image {
                    pointer: NonNull::new(output).unwrap(),
                    lib: &lib,
                };
                let k = u64::try_from(kind).unwrap();
                let c = u64::try_from(channels).unwrap();
                let n = u64::try_from(span).unwrap();
                assert_eq!(
                    values,
                    [
                        1, 1, 0, 0, 1, 0, 1, 0, 3, 2, c, k, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0, 1, 1,
                        1, 0, 0, c, k, n, 1
                    ]
                );
                let view = image.view();
                assert_eq!(
                    (view.width, view.height, view.channels, view.kind),
                    (3, 2, i32::try_from(channels).unwrap(), kind)
                );
                assert_eq!(image.pixels(), expected);
                input.fill(u64::MAX);
                assert_eq!(image.pixels(), expected);
                writeln!(
                    images,
                    "{cycle}\t{depth}\t{channels}\t{kind}\t{scalar}\t{span}\t{}\t{values:?}",
                    current_images.len()
                )
                .unwrap();
                current_images.extend_from_slice(image.pixels());
                held_images.push((cycle, depth, channels, image));
            }
        }
    }
    assert_eq!((lib.count(0), lib.count(6)), (780, 100));
    assert_eq!(held_strings.len(), 100);
    assert_eq!(held_images.len(), 780);
    let mut retained_strings = Vec::new();
    let mut retained_images = Vec::new();
    for (case, text) in &held_strings {
        assert_eq!(text.bytes(), STRINGS[*case]);
        retained_strings.extend_from_slice(text.bytes());
    }
    for (cycle, depth, channels, image) in &held_images {
        let span = image.pixels().len();
        let expected = (0..span)
            .map(|i| u8::try_from((cycle * 39 + depth * 3 + channels + i) % 251).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(image.pixels(), expected);
        retained_images.extend_from_slice(image.pixels());
    }
    assert_eq!(retained_strings, current_strings);
    assert_eq!(retained_images, current_images);
    fs::write(root.join("strings-current.bin"), &current_strings).unwrap();
    fs::write(root.join("strings-held.bin"), &retained_strings).unwrap();
    fs::write(root.join("images-current.bin"), &current_images).unwrap();
    fs::write(root.join("images-held.bin"), &retained_images).unwrap();
    drop(held_strings);
    drop(held_images);
    assert_eq!((lib.count(0), lib.count(6)), (0, 0));
    fs::write(root.join("counts.tsv"),"cycles\tstring_owners\timage_owners\tstring_bytes\timage_bytes\tfinal_images\tfinal_strings\n20\t100\t780\t240\t45120\t0\t0\n").unwrap();
    println!(
        "native Custom buffer helpers cycles=20 original_string_owners=100 original_image_owners=780 retained_strings=100 retained_images=780 string_bytes=240 image_bytes=45120 final_owners=0 native_capture_frames=0 passed"
    );
}
