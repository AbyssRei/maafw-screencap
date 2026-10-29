// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real kernel MemFd/helper comparisons. No PipeWire stream/callback is faked.
use super::*;
use std::{ffi::CString, fs::File, os::unix::fs::FileExt};

fn mappings(name: &str) -> usize {
    let text = fs::read_to_string("/proc/self/maps").unwrap();
    let tag = format!("/memfd:{name} ");
    text.lines()
        .filter(|line| {
            if !line.contains(&tag) {
                return false;
            }
            assert_eq!(line.split_whitespace().nth(1), Some("r--p"));
            true
        })
        .count()
}
fn plane(fd: i64, maxsize: u32, chunk: *mut spa::sys::spa_chunk) -> spa::sys::spa_data {
    spa::sys::spa_data {
        type_: spa::sys::SPA_DATA_MemFd,
        flags: spa::sys::SPA_DATA_FLAG_READABLE,
        fd,
        maxsize,
        chunk,
        // SAFETY: The remaining native descriptor fields accept zero/null.
        ..unsafe { mem::zeroed() }
    }
}

#[test]
#[ignore = "explicit original helper and real MemFd mapping/CPU/OpenCV owner comparison, not live NULL-data stream acceptance"]
fn matches_original_memfd_maps_and_retains_cpu_images_after_unmap() {
    let directory = env::var("MFS_P1_PIPEWIRE_MEMFD_ORACLE").unwrap();
    let directory = Path::new(&directory);
    let layout: Vec<usize> = fs::read_to_string(directory.join("spa-data-layout.txt"))
        .unwrap()
        .split_whitespace()
        .map(|value| value.parse().unwrap())
        .collect();
    type Data = spa::sys::spa_data;
    assert_eq!(
        layout,
        [
            size_of::<Data>(),
            align_of::<Data>(),
            offset_of!(Data, type_),
            offset_of!(Data, flags),
            offset_of!(Data, fd),
            offset_of!(Data, mapoffset),
            offset_of!(Data, maxsize),
            offset_of!(Data, data),
            offset_of!(Data, chunk),
            size_of::<i64>(),
        ]
    );
    let adapter = Adapter::open();
    let bytes = [0_u8; 28 * 24 * 3];
    let mut warm = RawCache::default();
    let warm_layout = Layout::checked(
        28,
        24,
        Format::Rgb,
        u32::try_from(bytes.len()).unwrap(),
        None,
    )
    .unwrap();
    // SAFETY: Independent initialized tightly packed warm-up bytes.
    unsafe { warm.copy_native(bytes.as_ptr(), warm_layout) }.unwrap();
    drop(
        adapter
            .convert(&warm.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    warm.clear();
    let native = Native::new().unwrap();
    let balance = BalanceInit;
    let native = native; // drops before test-only global init balancing
    let fds = count("/proc/self/fd");
    let observation_error = AtomicI32::new(0);
    let observation_unmaps = AtomicU64::new(0);
    let mut state = State {
        width: 28,
        height: 24,
        format: Format::Rgb,
        ..State::default()
    };
    let mut raw_images = Vec::new();
    raw_images.try_reserve_exact(60).unwrap();
    let mut bgr_images = Vec::new();
    bgr_images.try_reserve_exact(60).unwrap();
    let records = fs::read_to_string(directory.join("cases.tsv")).unwrap();
    let mut records = records.lines();
    assert_eq!(
        records.next().unwrap(),
        "index\tstride\tmaxsize\tchunk_size\thas_chunk\tfile_length\tmarker"
    );
    for cycle in 0..20 {
        let name = format!("mfs-rust-map-{cycle}");
        let fd = rustix::fs::memfd_create(
            CString::new(name.as_str()).unwrap(),
            rustix::fs::MemfdFlags::CLOEXEC,
        )
        .unwrap();
        let file = File::from(fd);
        assert_eq!(mappings(&name), 0);
        for mode in 0..3 {
            let values: Vec<u32> = records
                .next()
                .unwrap()
                .split('\t')
                .map(|v| v.parse().unwrap())
                .collect();
            let [
                index,
                stride,
                maxsize,
                chunk_size,
                has_chunk,
                file_length,
                serial,
            ] = <[u32; 7]>::try_from(values).unwrap();
            assert_eq!(index, cycle * 3 + mode);
            assert_eq!(serial, index + 1);
            assert_eq!(stride, if mode == 2 { 84 } else { 96 });
            assert_eq!(chunk_size, stride.checked_mul(24).unwrap());
            assert_eq!(maxsize, if mode == 1 { 0 } else { chunk_size });
            assert_eq!(has_chunk, u32::from(mode != 2));
            let span = stride.checked_mul(23).unwrap().checked_add(84).unwrap();
            assert_eq!(file_length, if mode == 1 { span } else { chunk_size });
            let mut input = fs::read(directory.join(format!("case-{index}.input"))).unwrap();
            assert_eq!(input.len(), usize::try_from(chunk_size).unwrap());
            file.set_len(u64::from(file_length)).unwrap();
            let length = usize::try_from(file_length).unwrap();
            file.write_all_at(&input[..length], 0).unwrap();
            let mut chunk = spa::sys::spa_chunk {
                stride: i32::try_from(stride).unwrap(),
                size: chunk_size,
                // SAFETY: Zero flags/offset are initialized valid native fields.
                ..unsafe { mem::zeroed() }
            };
            let data = plane(
                i64::from(file.as_raw_fd()),
                maxsize,
                if has_chunk != 0 {
                    &mut chunk
                } else {
                    ptr::null_mut()
                },
            );
            {
                // SAFETY: A real live MemFd, valid descriptors and exact checked
                // target span. This owner is only for observing native mapping
                // permissions/lifetime; it never publishes native pixel borrows.
                let mapped = unsafe {
                    MemFdMap::new(
                        &data,
                        usize::try_from(span).unwrap(),
                        &observation_error,
                        &observation_unmaps,
                    )
                }
                .unwrap();
                assert!(!mapped.pointer().is_null());
                assert_eq!(mappings(&name), 1);
            }
            assert_eq!(mappings(&name), 0);
            // SAFETY: State and this real FD lease are exclusively owned here;
            // no stream/callback is running or producer writing/truncating.
            let copied = unsafe { copy_plane(&mut state, &data, &native.callbacks) }.unwrap();
            assert_eq!(copied.copied_bytes, 28 * 24 * 3);
            assert!(
                copied.allocated_storage,
                "all previous CPU images remain held"
            );
            assert_eq!(
                mappings(&name),
                0,
                "unmap precedes return from the copy helper"
            );
            assert!(
                rustix::fs::fstat(&file).is_ok(),
                "mapping must not consume the native fd"
            );
            let raw = state.cache.snapshot().unwrap();
            assert_eq!(marker(&raw), serial);
            assert_eq!(
                raw.pixels(),
                fs::read(directory.join(format!("case-{index}.rgb"))).unwrap()
            );
            let bgr = Bgr::Converted(adapter.convert(&raw, Conversion::Rgb).unwrap());
            assert_eq!(known_bgr(&bgr), serial);
            assert_eq!(
                bgr.pixels(),
                fs::read(directory.join(format!("case-{index}.bgr"))).unwrap()
            );
            input.fill(0);
            file.write_all_at(&input[..length], 0).unwrap();
            raw_images.push((raw, serial));
            bgr_images.push((bgr, serial));
            for (image, serial) in &raw_images {
                assert_eq!(marker(image), *serial);
            }
            for (image, serial) in &bgr_images {
                assert_eq!(known_bgr(image), *serial);
            }
        }
        drop(file);
        assert_eq!(mappings(&name), 0);
        assert_eq!(count("/proc/self/fd"), fds);
    }
    assert!(records.next().is_none());
    // Six real/pre-native failure boundaries must not map, unmap, or replace the
    // last owned CPU cache. Arbitrary closed positive FDs are not borrowed.
    let edge = File::from(
        rustix::fs::memfd_create(c"mfs-rust-map-errors", rustix::fs::MemfdFlags::CLOEXEC).unwrap(),
    );
    edge.set_len(2304).unwrap();
    let mut chunk = spa::sys::spa_chunk {
        stride: 96,
        size: 2304,
        ..unsafe { mem::zeroed() }
    };
    for mode in 0..6 {
        edge.set_len(2304).unwrap();
        chunk.size = 2304;
        let write_only = std::fs::OpenOptions::new()
            .write(true)
            .open(format!("/proc/self/fd/{}", edge.as_raw_fd()))
            .unwrap();
        let mut data = plane(i64::from(edge.as_raw_fd()), 2304, &mut chunk);
        match mode {
            0 => data.fd = -1,
            1 => data.fd = i64::MAX,
            2 => data.mapoffset = 4096,
            3 => {
                edge.set_len(2291).unwrap();
            }
            4 => {
                data.maxsize = 0;
                chunk.size = 0;
            }
            5 => data.fd = i64::from(write_only.as_raw_fd()),
            _ => unreachable!(),
        }
        // SAFETY: Invalid negative/unrepresentable descriptors are rejected
        // before borrowing; all representable nonnegative FDs remain live.
        // No input pointer exists, and failures occur before any CPU read.
        let error = unsafe { copy_plane(&mut state, &data, &native.callbacks) }.unwrap_err();
        if mode == 5 {
            match &error {
                PlaneError::Map(error) => {
                    assert_eq!(error.operation, "owned MemFd mmap");
                    assert_eq!(error.code, rustix::io::Errno::ACCES.raw_os_error());
                }
                _ => panic!("wrong failure {error:?}"),
            }
        }
        assert_eq!(marker(&state.cache.snapshot().unwrap()), 60);
        assert_eq!(native.callbacks.owned_maps.load(Ordering::Acquire), 60);
        eprintln!("MemFd helper rejected case={mode} error={error:?}");
    }
    drop(edge);
    assert_eq!(count("/proc/self/fd"), fds);
    state.cache.clear();
    let evidence = native.evidence();
    assert_eq!(
        (
            evidence.owned_maps,
            evidence.unmap_calls,
            evidence.mapping_error,
            evidence.unmap_error
        ),
        (60, 60, 0, 0)
    );
    // Permission observations have independent accounting; each performs one
    // additional real map/unmap outside the single-map production copy path.
    assert_eq!(
        (
            observation_unmaps.load(Ordering::Acquire),
            observation_error.load(Ordering::Acquire)
        ),
        (60, 0)
    );
    drop(native);
    drop(balance);
    std::thread::spawn(move || {
        for (image, serial) in raw_images {
            assert_eq!(marker(&image), serial);
        }
    })
    .join()
    .unwrap();
    for (image, serial) in &bgr_images {
        assert_eq!(known_bgr(image), *serial);
    }
    assert_eq!(adapter.owner_count(), 60);
    drop(bgr_images);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native MemFd helper cycles=20 copies=60 owning_maps=60 observed_maps=60 unmap_calls=120 rejected=6 retained_rgb=60 retained_bgr=60 passed; live NULL-data stream and DMA-BUF pending"
    );
}
