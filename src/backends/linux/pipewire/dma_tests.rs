// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real kernel DMA exporter checks and separately named non-DMA error controls.
//! Manual SPA descriptors do not establish an incoming PipeWire DMA stream.
use super::*;
use crate::platform::linux::pipewire::DmaSyncEvidence;
use std::{
    fs::File,
    os::{fd::FromRawFd, unix::fs::FileExt},
};

fn plane(fd: i64, maxsize: u32, chunk: *mut spa::sys::spa_chunk) -> spa::sys::spa_data {
    spa::sys::spa_data {
        type_: spa::sys::SPA_DATA_DmaBuf,
        flags: spa::sys::SPA_DATA_FLAG_READABLE,
        fd,
        maxsize,
        chunk,
        ..unsafe { mem::zeroed() }
    }
}
fn mappings(file: &File) -> Vec<String> {
    let stat = rustix::fs::fstat(file).unwrap();
    fs::read_to_string("/proc/self/maps")
        .unwrap()
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            let (major, minor) = fields[3].split_once(':').unwrap();
            (fields[4].parse::<u64>().unwrap() == stat.st_ino
                && u32::from_str_radix(major, 16).unwrap() == rustix::fs::major(stat.st_dev)
                && u32::from_str_radix(minor, 16).unwrap() == rustix::fs::minor(stat.st_dev))
            .then(|| fields[1].to_owned())
        })
        .collect()
}

#[test]
fn preserves_source_non_dma_ioctl_failure_and_borrowed_pointer_selection() {
    // An ordinary MemFd is deliberately an INVALID DMA descriptor. Its real
    // ENOTTY checks U's START-failure/no-END branch, never positive DMA support.
    let file = File::from(
        rustix::fs::memfd_create(c"mfs-invalid-dma-control", rustix::fs::MemfdFlags::CLOEXEC)
            .unwrap(),
    );
    let bytes: [u8; 24] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 0xed, 0xed, 0xed, 11, 12, 13, 14, 15, 16, 17, 18, 19, 0xed,
        0xed, 0xed,
    ];
    file.set_len(24).unwrap();
    file.write_all_at(&bytes, 0).unwrap();
    let native = Native::new().unwrap();
    let balance = BalanceInit;
    let native = native;
    let mut state = State {
        width: 3,
        height: 2,
        format: Format::Rgb,
        ..State::default()
    };
    let mut chunk = spa::sys::spa_chunk {
        stride: 12,
        size: 24,
        ..unsafe { mem::zeroed() }
    };
    let mut data = plane(i64::from(file.as_raw_fd()), 24, &mut chunk);
    // SAFETY: Real stable MemFd and exclusive initialized records/bytes. There
    // is no device DMA or producer; the negative ioctl cannot invalidate pixels.
    unsafe { copy_plane(&mut state, &data, &native.callbacks) }.unwrap();
    let first = state.cache.snapshot().unwrap();
    assert_eq!(first.pixels(), [&bytes[..9], &bytes[12..21]].concat());
    assert!(mappings(&file).is_empty());
    let evidence = native.evidence();
    assert_eq!((evidence.owned_maps, evidence.unmap_calls), (1, 1));
    assert_eq!(
        (
            evidence.dma.starts,
            evidence.dma.ends,
            evidence.dma.start_error,
            evidence.dma.end_error
        ),
        (1, 0, libc::ENOTTY, 0)
    );
    let observation = DmaSyncEvidence::default();
    let unmap_error = AtomicI32::new(0);
    let unmaps = AtomicU64::new(0);
    {
        // SAFETY: Same valid ordinary file, exact checked span, exclusive test
        // ownership. This extra map observes native READ/SHARED permissions only.
        let mapped =
            unsafe { DmaBufMap::new(&data, 21, 24, &observation, &unmap_error, &unmaps) }.unwrap();
        assert!(!mapped.pointer().is_null());
        assert_eq!(mappings(&file), ["r--s"]);
    }
    assert!(mappings(&file).is_empty());
    assert_eq!(unmaps.load(Ordering::Acquire), 1);
    for mode in 0..5 {
        file.set_len(24).unwrap();
        let mut invalid = plane(i64::from(file.as_raw_fd()), 24, &mut chunk);
        match mode {
            0 => invalid.fd = -1,
            1 => invalid.fd = i64::MAX,
            2 => invalid.mapoffset = 4096,
            3 => file.set_len(20).unwrap(),
            4 => invalid.type_ = spa::sys::SPA_DATA_MemPtr,
            _ => unreachable!(),
        }
        // SAFETY: Invalid descriptor scalars are rejected before borrowing;
        // every representable nonnegative descriptor remains owned/live here.
        assert!(
            unsafe { DmaBufMap::new(&invalid, 21, 24, &observation, &unmap_error, &unmaps) }
                .is_err()
        );
    }
    assert_eq!(observation.snapshot().starts, 1);
    assert_eq!(unmaps.load(Ordering::Acquire), 1);
    let mut existing = bytes;
    data.fd = -1;
    data.data = existing.as_mut_ptr().cast();
    // SAFETY: Existing initialized CPU bytes cover the span and stay immutable
    // during copy; source bypasses the unused fd and all sync in this branch.
    unsafe { copy_plane(&mut state, &data, &native.callbacks) }.unwrap();
    existing.fill(0);
    assert_eq!(state.cache.snapshot().unwrap().pixels(), first.pixels());
    assert_eq!(native.evidence().dma.borrowed, 1);
    state.cache.clear();
    drop(native);
    drop(balance);
    assert_eq!(first.pixels(), [&bytes[..9], &bytes[12..21]].concat());
    eprintln!(
        "native non-DMA error control START=ENOTTY END=skipped borrowed_pointer=skips_sync invalid_layouts=5 passed; not DMA acceptance"
    );
}

type Create = unsafe extern "C" fn(u32, u64) -> i32;
type Write = unsafe extern "C" fn(i32, *const u8, u64) -> i32;
type Abi = unsafe extern "C" fn(u32) -> u64;
struct Factory {
    _module: Module,
    create: Create,
    write: Write,
}
impl Factory {
    fn open() -> Self {
        let path = env::var_os("MFS_P1_PIPEWIRE_DMA_FIXTURE_LIBRARY").unwrap();
        let module = Module::open(Path::new(&path));
        // SAFETY: Exact dma-fixture.h C signatures, called only while this
        // module remains loaded. ABI values and actual fd/input use are checked.
        let (create, write, abi) = unsafe {
            (
                module.symbol::<Create>(c"mfs_pw_dma_create"),
                module.symbol::<Write>(c"mfs_pw_dma_write"),
                module.symbol::<Abi>(c"mfs_pw_dma_abi"),
            )
        };
        for (index, expected) in DmaBufMap::abi().into_iter().enumerate() {
            // SAFETY: Exact scalar ABI query, no foreign pointer input.
            assert_eq!(unsafe { abi(u32::try_from(index).unwrap()) }, expected);
        }
        Self {
            _module: module,
            create,
            write,
        }
    }
    fn create(&self, exporter: u32) -> File {
        // SAFETY: Fixed exporter/page-sized allocation; success transfers one
        // unique real DMA-BUF fd, no hidden native handle or retained input.
        let fd = unsafe { (self.create)(exporter, 4096) };
        assert!(fd >= 0, "native DMA create failed: {fd}");
        // SAFETY: Sole ownership transferred by the header contract above.
        unsafe { File::from_raw_fd(fd) }
    }
    fn write(&self, file: &File, bytes: &[u8]) {
        // SAFETY: Sole owned real DMA fd and independent initialized source;
        // native map/WRITE sync/copy ends before returning. No borrow escapes.
        assert_eq!(
            unsafe {
                (self.write)(
                    file.as_raw_fd(),
                    bytes.as_ptr(),
                    u64::try_from(bytes.len()).unwrap(),
                )
            },
            0
        );
    }
}
#[test]
#[ignore = "explicit isolated real kernel DMA exporter/owned mapping comparison, not incoming stream/GPU completion acceptance"]
fn matches_original_kernel_dma_maps_and_retains_independent_cpu_images() {
    let factory = Factory::open();
    let directory = env::var("MFS_P1_PIPEWIRE_DMA_ORACLE").unwrap();
    let directory = Path::new(&directory);
    let exporter = fs::read_to_string(directory.join("exporter.txt"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    if exporter == 0 {
        eprintln!("missing_environment: no accessible kernel DMA exporter; no DMA capture ran");
        std::process::exit(77);
    }
    assert!([1, 2].contains(&exporter));
    let adapter = Adapter::open();
    let mut warm = RawCache::default();
    let warm_bytes = [0_u8; 2016];
    let layout = Layout::checked(28, 24, Format::Rgb, 2016, None).unwrap();
    // SAFETY: Independent initialized warm-up rows.
    unsafe { warm.copy_native(warm_bytes.as_ptr(), layout) }.unwrap();
    drop(
        adapter
            .convert(&warm.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    warm.clear();
    let native = Native::new().unwrap();
    let balance = BalanceInit;
    let native = native;
    let baseline = count("/proc/self/fd");
    let mut state = State {
        width: 28,
        height: 24,
        format: Format::Rgb,
        ..State::default()
    };
    let mut raw_held = Vec::new();
    raw_held.try_reserve_exact(60).unwrap();
    let mut bgr_held = Vec::new();
    bgr_held.try_reserve_exact(60).unwrap();
    let observations = DmaSyncEvidence::default();
    let observation_error = AtomicI32::new(0);
    let observation_unmaps = AtomicU64::new(0);
    for cycle in 0..20 {
        let file = factory.create(exporter);
        assert_eq!(rustix::fs::fstat(&file).unwrap().st_size, 4096);
        // SAFETY: Read-only descriptor flag observation.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
        for mode in 0..3 {
            let index = cycle * 3 + mode;
            let stride = if mode == 2 { 84 } else { 96 };
            let mut input = fs::read(directory.join(format!("case-{index}.input"))).unwrap();
            assert_eq!(input.len(), stride * 24);
            factory.write(&file, &input);
            let mut chunk = spa::sys::spa_chunk {
                stride: i32::try_from(stride).unwrap(),
                size: u32::try_from(input.len()).unwrap(),
                ..unsafe { mem::zeroed() }
            };
            let data = plane(
                i64::from(file.as_raw_fd()),
                if mode == 1 { 0 } else { chunk.size },
                if mode == 2 {
                    ptr::null_mut()
                } else {
                    &mut chunk
                },
            );
            let span = stride * 23 + 84;
            {
                // SAFETY: Real owned DMA fd and exclusive producer WRITE END
                // completed. No producer mutates or reuses while mapped/read.
                let mapped = unsafe {
                    DmaBufMap::new(
                        &data,
                        span,
                        stride * 24,
                        &observations,
                        &observation_error,
                        &observation_unmaps,
                    )
                }
                .unwrap();
                assert!(!mapped.pointer().is_null());
                assert_eq!(mappings(&file), ["r--s"]);
            }
            assert!(mappings(&file).is_empty());
            // SAFETY: Same initialized native layout/real DMA lease; only
            // target rows copy. Owner finishes READ END/unmap before return.
            let copied = unsafe { copy_plane(&mut state, &data, &native.callbacks) }.unwrap();
            assert_eq!(copied.copied_bytes, 2016);
            assert!(copied.allocated_storage);
            assert!(mappings(&file).is_empty());
            let raw = state.cache.snapshot().unwrap();
            let serial = marker(&raw);
            assert_eq!(serial, index + 1);
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
            factory.write(&file, &input);
            raw_held.push((raw, serial));
            bgr_held.push((bgr, serial));
            for (image, serial) in &raw_held {
                assert_eq!(marker(image), *serial);
            }
            for (image, serial) in &bgr_held {
                assert_eq!(known_bgr(image), *serial);
            }
        }
        drop(file);
        assert_eq!(count("/proc/self/fd"), baseline);
    }
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
    assert_eq!(
        (
            evidence.dma.starts,
            evidence.dma.ends,
            evidence.dma.borrowed,
            evidence.dma.start_error,
            evidence.dma.end_error
        ),
        (60, 60, 0, 0, 0)
    );
    let observed = observations.snapshot();
    assert_eq!(
        (
            observed.starts,
            observed.ends,
            observed.start_error,
            observed.end_error
        ),
        (60, 60, 0, 0)
    );
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
        for (image, serial) in raw_held {
            assert_eq!(marker(&image), serial);
        }
    })
    .join()
    .unwrap();
    for (image, serial) in &bgr_held {
        assert_eq!(known_bgr(image), *serial);
    }
    assert_eq!(adapter.owner_count(), 60);
    drop(bgr_held);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    drop(factory);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native kernel DMA helper cycles=20 copies=60 observation_maps=60 READ_START=120 READ_END=120 unmaps=120 retained_rgb=60 retained_bgr=60 passed; incoming stream/GPU completion pending"
    );
}
