// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit 120-second native U caller, not a new capture policy or benchmark.
use super::*;
use crate::backends::linux::manager::{self, native as managed};
use std::io::Write as _;

struct Convert<'a> {
    adapter: &'a Adapter,
    latest: Option<RawFrame>,
}
impl<'a> managed::Converter for Convert<'a> {
    type Image = Option<Bgr<'a>>;
    type Error = AdapterError;
    #[cfg(feature = "linux-wlr")]
    fn wlr(&mut self, _: crate::backends::linux::wlr::Sample) -> Result<Self::Image, AdapterError> {
        unreachable!("PipeWire sustained caller cannot select Wlr")
    }
    fn pipewire(
        &mut self,
        raw: Option<RawFrame>,
        mode: Conversion,
    ) -> Result<Option<Self::Image>, AdapterError> {
        assert_eq!(mode, Conversion::Rgb);
        assert!(self.latest.is_none(), "one caller sample at a time");
        self.latest = raw;
        self.adapter
            .convert(
                self.latest.as_ref().ok_or(AdapterError::InvalidLayout)?,
                mode,
            )
            .map(|image| Some(Some(Bgr::Converted(image))))
    }
    fn pipewire_reuse(&mut self, _: Option<RawFrame>) -> Option<Self::Image> {
        unreachable!("owned RGB source must not negotiate BGR")
    }
    fn empty(&mut self) -> Result<Self::Image, AdapterError> {
        panic!("owned RGB source must not produce an empty output")
    }
}

fn rss_kib() -> u64 {
    let text = fs::read_to_string("/proc/self/statm").unwrap();
    let pages = text
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse::<u64>()
        .unwrap();
    // SAFETY: Scalar process page-size query; no pointer or ownership transfer.
    let page_size = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap();
    assert!(page_size > 0 && page_size % 1024 == 0);
    pages.checked_mul(page_size / 1024).unwrap()
}
fn record_resources(file: &mut fs::File, phase: &str, elapsed: u128, calls: u64) {
    writeln!(
        file,
        "{phase}\t{elapsed}\t{calls}\t{}\t{}\t{}",
        count("/proc/self/fd"),
        task_ids().len(),
        rss_kib()
    )
    .unwrap();
    file.flush().unwrap();
}

#[test]
#[ignore = "native owned RGB producer, OpenCV adapter, and >=120 seconds after warmup"]
fn sustains_native_manager_for_two_minutes_with_bounded_retained_owners() {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    assert_eq!(
        Path::new(&env::var("PIPEWIRE_RUNTIME_DIR").unwrap())
            .join(env::var("PIPEWIRE_REMOTE").unwrap()),
        Path::new(&socket)
    );
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(node, 0, "owned daemon source must have an explicit node");
    let directory =
        Path::new(&env::var("MFS_P1_PIPEWIRE_BGR_ARTIFACTS").unwrap()).join("sustained");
    fs::create_dir(&directory).unwrap();
    let frames = directory.join("frames");
    fs::create_dir(&frames).unwrap();
    let mut captures = fs::File::create_new(directory.join("captures.tsv")).unwrap();
    let mut resources = fs::File::create_new(directory.join("resources.tsv")).unwrap();
    writeln!(
        captures,
        "index\telapsed_ns\tcapture_ns\tmarker\tmarker_changed\treplayed"
    )
    .unwrap();
    writeln!(resources, "phase\telapsed_ns\tcalls\tfds\ttasks\trss_kib").unwrap();
    let adapter = Adapter::open();
    // Warm the actual conversion layout before accounting native resources.
    let mut cache = RawCache::default();
    let warm = [1_u8; 28 * 24 * 3];
    let layout = Layout::checked(
        28,
        24,
        Format::Rgb,
        u32::try_from(warm.len()).unwrap(),
        None,
    )
    .unwrap();
    // SAFETY: All input rows are initialized and copied into separate storage.
    unsafe { cache.copy_native(warm.as_ptr(), layout) }.unwrap();
    drop(
        adapter
            .convert(&cache.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    cache.clear();
    let baseline_fds = count("/proc/self/fd");
    let baseline_tasks = task_ids();
    fs::write(
        directory.join("baseline-task-ids.txt"),
        baseline_tasks
            .iter()
            .map(|id| id.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let mut manager = manager::NativeManager::new(manager::Config {
        method: manager::PIPEWIRE,
        wlr_socket: Vec::new(),
        pipewire_fd: -1,
        pipewire_node: node,
        ..manager::Config::default()
    });
    let connected = manager.connect_native().unwrap();
    let native_balance = BalanceInit;
    // Move the initialized manager after the balancing guard: unwind/normal
    // destruction must close native resources before fixture-only pw_deinit.
    let mut manager = manager;
    assert!(
        connected.connected && connected.failure.is_none(),
        "{connected:?}"
    );
    let close = connected.close.unwrap();
    let mut converter = Convert {
        adapter: &adapter,
        latest: None,
    };
    // These owners remain bounded independently of the number of captures.
    let mut held = Vec::new();
    held.try_reserve_exact(3).unwrap();
    let mut last = None;
    let mut warmup_replays = 0_u64;
    for _ in 0..3 {
        let mut output = None;
        let attempt = manager.capture_native(&mut output, &mut converter);
        assert!(attempt.result.unwrap());
        warmup_replays = warmup_replays
            .checked_add(u64::from(attempt.replayed.unwrap()))
            .unwrap();
        let raw = converter.latest.take().unwrap();
        assert_eq!(marker(&raw), known_bgr(output.as_ref().unwrap()));
    }
    let start = Instant::now();
    let mut calls = 0_u64;
    let mut replays = 0_u64;
    let mut next_resource = Duration::ZERO;
    let mut previous = None;
    while start.elapsed() < Duration::from_secs(120) {
        let capture_start = Instant::now();
        let mut output = None;
        let attempt = manager.capture_native(&mut output, &mut converter);
        let capture_ns = capture_start.elapsed().as_nanos();
        // Persist any available raw sample before interpreting a failed result.
        let stem = format!("frame-{calls:06}");
        if let Some(raw) = &converter.latest {
            fs::write(frames.join(format!("{stem}.rgb")), raw.pixels()).unwrap();
        }
        assert!(attempt.result.unwrap());
        let replayed = attempt.replayed.unwrap();
        replays = replays.checked_add(u64::from(replayed)).unwrap();
        let image = output.take().unwrap();
        let raw = converter.latest.take().unwrap();
        fs::write(frames.join(format!("{stem}.bgr")), image.pixels()).unwrap();
        let serial = marker(&raw);
        assert_eq!(known_bgr(&image), serial);
        let (rgb, rgb_tail) = raw.pixels().as_chunks::<3>();
        let (bgr, bgr_tail) = image.pixels().as_chunks::<3>();
        assert!(rgb_tail.is_empty() && bgr_tail.is_empty());
        assert_eq!(rgb.len(), bgr.len());
        for (rgb, bgr) in rgb.iter().zip(bgr) {
            assert_eq!(*bgr, [rgb[2], rgb[1], rgb[0]]);
        }
        let elapsed = start.elapsed();
        let changed = previous.map_or(-1, |old| i32::from(old != serial));
        writeln!(
            captures,
            "{calls}\t{}\t{capture_ns}\t{serial}\t{changed}\t{}",
            elapsed.as_nanos(),
            u8::from(replayed)
        )
        .unwrap();
        previous = Some(serial);
        calls = calls.checked_add(1).unwrap();
        if held.len() < 3 {
            held.push((raw, image, serial));
        } else {
            last = Some((raw, image, serial));
        }
        assert!(adapter.owner_count() <= 4, "bounded retained Mat owners");
        if elapsed >= next_resource {
            for (raw, image, serial) in held.iter().chain(last.iter()) {
                assert_eq!(marker(raw), *serial);
                assert_eq!(known_bgr(image), *serial);
            }
            record_resources(&mut resources, "stream", elapsed.as_nanos(), calls);
            captures.flush().unwrap();
            next_resource = elapsed.checked_add(Duration::from_secs(1)).unwrap();
        }
    }
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(120) && calls >= 4);
    record_resources(&mut resources, "stream", elapsed.as_nanos(), calls);
    let live = live_tasks();
    drop(manager);
    let managed::Evidence::PipeWire(e) = close.take().unwrap() else {
        panic!("missing completed close evidence")
    };
    assert_eq!(
        (
            e.fault,
            e.queue_error,
            e.core_disconnect_error,
            e.mapping_error,
            e.unmap_error
        ),
        (0, 0, 0, 0, 0),
        "{e:?}"
    );
    assert_eq!(e.dequeued, e.returned, "{e:?}");
    assert_eq!(e.owned_maps, e.unmap_calls, "{e:?}");
    assert_eq!(
        e.copies
            .checked_add(replays)
            .unwrap()
            .checked_add(warmup_replays)
            .unwrap(),
        calls.checked_add(3).unwrap(),
        "requested copies and source replay decisions"
    );
    drop(native_balance);
    assert_resources(&directory, 0, baseline_fds, &baseline_tasks, &live);
    let closed_tasks = task_ids();
    assert_eq!(closed_tasks, baseline_tasks);
    fs::write(
        directory.join("closed-task-ids.txt"),
        closed_tasks
            .iter()
            .map(|id| id.to_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    record_resources(&mut resources, "closed", start.elapsed().as_nanos(), calls);
    for (raw, image, serial) in held.iter().chain(last.iter()) {
        assert_eq!(marker(raw), *serial);
        assert_eq!(known_bgr(image), *serial);
    }
    assert_eq!(adapter.owner_count(), 4);
    drop(last);
    drop(held);
    drop(converter);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    captures.flush().unwrap();
    resources.flush().unwrap();
    fs::write(directory.join("summary.txt"), format!("trace_version=1\nwarmup=3\nwarmup_replays={warmup_replays}\nsteady_calls={calls}\nsteady_replays={replays}\nsteady_elapsed_ns={}\nretained_at_shutdown=4\nbaseline_fds={baseline_fds}\nerrors=0\nmarker_changes_are_not_native_freshness=1\nnative_copies_include_warmup=1\nnative_copies={}\nnative_dequeued={}\nnative_returned={}\nnative_allocated={}\ngraphics=owned_CPU_RGB_fixture_no_GPU_claim\nresources_balanced=1\n", elapsed.as_nanos(),e.copies,e.dequeued,e.returned,e.allocated)).unwrap();
    eprintln!(
        "native Rust PipeWire sustained warmup=3 steady_ns={} calls={calls} retained=4 errors=0 resources_balanced=1 passed",
        elapsed.as_nanos()
    );
}
