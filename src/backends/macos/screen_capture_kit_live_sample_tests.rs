// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Separate real-sample conversion diagnostic; old scene assertions stay strict.
use super::*;

type Output = (Sample, Image, u64);
struct Fanout {
    adapter: Arc<Adapter>,
    calls: [AtomicUsize; 3],
    evidence: [pixel_buffer::Evidence; 3],
    outputs: Mutex<[Option<Output>; 3]>,
    fault: AtomicI32,
    released: Arc<AtomicUsize>,
}
static RELEASE_PANIC: AtomicBool = AtomicBool::new(false);
fn thread_id() -> u64 {
    let mut value = 0;
    // SAFETY: Zero selects this thread and the initialized output is writable.
    assert_eq!(unsafe { libc::pthread_threadid_np(0, &mut value) }, 0);
    assert_ne!(value, 0);
    value
}
unsafe extern "C" fn visit(context: *mut c_void, sample: *const c_void, call: u32) -> i32 {
    // SAFETY: The native shared hook owns one Arc<Fanout> through this call.
    // It supplies exactly its live original CMSampleBuffer, with no ownership
    // transfer. The callback stores only independently owned CPU results.
    let state = unsafe { &*context.cast::<Fanout>() };
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), Error> {
        let call = usize::try_from(call).unwrap();
        assert!(call < 3);
        assert_eq!(state.calls[call].fetch_add(1, Ordering::AcqRel), 0);
        let sample = unsafe { sample.cast::<CMSampleBuffer>().as_ref() }
            .ok_or(Error::new("live diagnostic missing sample", -22))?;
        let evidence = &state.evidence[call];
        let output = pixel_buffer::with_sample(sample, evidence, |raw| {
            assert_eq!(evidence.locks.load(Ordering::Acquire), 1);
            assert_eq!(evidence.unlocks.load(Ordering::Acquire), 0);
            let raw = raw.ok_or(Error::new("live diagnostic missing image", -22))?;
            let image = state
                .adapter
                .convert(raw.pixels(), raw.width, raw.height, false)?;
            assert_eq!(evidence.unlocks.load(Ordering::Acquire), 0);
            Ok((raw, image, thread_id()))
        })?;
        let old = state.outputs.lock().unwrap()[call].replace(output);
        assert!(old.is_none());
        Ok(())
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(_)) => {
            state.fault.store(1, Ordering::Release);
            1
        }
        Err(_) => {
            state.fault.store(2, Ordering::Release);
            2
        }
    }
}
unsafe extern "C" fn release(context: *mut c_void) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: The installed native hook transfers this exact raw Arc back
        // once, after its last original/escaping observation owner has ended.
        let state = unsafe { Arc::from_raw(context.cast::<Fanout>()) };
        let released = state.released.clone();
        drop(state);
        released.fetch_add(1, Ordering::AcqRel);
    }));
    if result.is_err() {
        RELEASE_PANIC.store(true, Ordering::Release);
    }
}
fn save(root: &Path, stem: &str, raw: &Sample, image: &Image) {
    fs::write(root.join(format!("{stem}.bgra")), raw.pixels()).unwrap();
    fs::write(root.join(format!("{stem}.bgr")), image.pixels()).unwrap();
}

#[test]
#[ignore = "requires native Mac capture permission, original observed class and OpenCV adapter"]
fn converts_the_same_live_original_sample_and_retains_cpu_owners_after_source_shutdown() {
    let root = PathBuf::from(env::var_os("MFS_P1_MACOS_LIVE_SAMPLE_ARTIFACTS").unwrap());
    let adapter = Adapter::open();
    adapter.warm();
    let state = Arc::try_new(Fanout {
        adapter: adapter.clone(),
        calls: std::array::from_fn(|_| AtomicUsize::new(0)),
        evidence: std::array::from_fn(|_| pixel_buffer::Evidence::default()),
        outputs: Mutex::new(std::array::from_fn(|_| None)),
        fault: AtomicI32::new(0),
        released: Arc::try_new(AtomicUsize::new(0)).unwrap(),
    })
    .unwrap();
    let caller = thread_id();
    let original = original_host::Original::open(&root, true);
    fs::write(
        root.join("native-type.tsv"),
        format!("{}\n", original.live_bgr_type()),
    )
    .unwrap();
    // SAFETY: visit/release use this exact Fanout raw Arc and contain unwinding;
    // its adapter/code and state outlive every escaping native observation.
    assert_eq!(
        unsafe { original.install_live_sample(state.clone(), visit, release) },
        0
    );
    let mut codes = [0; 3];
    for (call, code) in codes.iter_mut().enumerate() {
        *code = original.capture_result(u32::try_from(call).unwrap(), &root);
        if let Some((raw, image, _)) = &state.outputs.lock().unwrap()[call] {
            save(&root, &format!("rust-call-{call}"), raw, image);
        }
    }
    original.finish(&root);
    let until = Instant::now() + Duration::from_secs(8);
    while state.released.load(Ordering::Acquire) == 0 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut rows = fs::File::create_new(root.join("live.tsv")).unwrap();
    use std::io::Write;
    writeln!(rows,"call\tinvoke_exit\tcallbacks\tcallback_thread\twidth\theight\tstride\tlocks\tunlocks\tcopies\tcopied_bytes\tconversions\tunlock_error").unwrap();
    for (call, code) in codes.iter().enumerate() {
        let outputs = state.outputs.lock().unwrap();
        let (width, height, stride, thread) = outputs[call]
            .as_ref()
            .map(|(raw, _, t)| (raw.width, raw.height, raw.native_stride, *t))
            .unwrap_or_default();
        let e = &state.evidence[call];
        writeln!(
            rows,
            "{call}\t{code}\t{}\t{thread}\t{width}\t{height}\t{stride}\t{}\t{}\t{}\t{}\t{}\t{}",
            state.calls[call].load(Ordering::Acquire),
            e.locks.load(Ordering::Acquire),
            e.unlocks.load(Ordering::Acquire),
            e.copies.load(Ordering::Acquire),
            e.copied_bytes.load(Ordering::Acquire),
            e.conversions.load(Ordering::Acquire),
            e.unlock_error.load(Ordering::Acquire)
        )
        .unwrap();
    }
    rows.flush().unwrap();
    let released = state.released.load(Ordering::Acquire);
    let fault = state.fault.load(Ordering::Acquire);
    fs::write(
        root.join("callback-owner.tsv"),
        format!("{released}\t{fault}\n"),
    )
    .unwrap();
    assert_eq!(released, 1, "native shared diagnostic context remains live");
    assert_eq!(Arc::strong_count(&state), 1);
    let outputs = state.outputs.lock().unwrap().each_mut().map(Option::take);
    let held_root = root.clone();
    let reader = std::thread::spawn(move || {
        let thread = thread_id();
        for (call, output) in outputs.into_iter().enumerate() {
            if let Some((raw, image, _)) = output {
                save(&held_root, &format!("rust-held-{call}"), &raw, &image);
            }
        }
        thread
    })
    .join()
    .unwrap();
    drop(state);
    let owners = adapter.owners();
    drop(adapter);
    fs::write(
        root.join("summary.tsv"),
        format!(
            "{}\t{caller}\t{reader}\t{released}\t{fault}\t{}\t{owners}\t{}\t{}\n",
            std::process::id(),
            u8::from(RELEASE_PANIC.load(Ordering::Acquire)),
            MODULE_CLOSED.load(Ordering::Acquire),
            MODULE_ERROR.load(Ordering::Acquire)
        ),
    )
    .unwrap();
    // Persist every actual result before asserting. This finite conversion
    // diagnostic does not replace known-scene or independent-capture results.
    assert_eq!(codes, [0; 3]);
    assert_eq!(fault, 0);
    assert!(!RELEASE_PANIC.load(Ordering::Acquire));
    assert_ne!(caller, reader);
    assert_eq!(owners, 0);
    assert_eq!(MODULE_CLOSED.load(Ordering::Acquire), 1);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    for call in 0..3 {
        let native = fs::read(root.join(format!("original-native-{call}-before.bgra"))).unwrap();
        let raw = fs::read(root.join(format!("rust-call-{call}.bgra"))).unwrap();
        assert_eq!(raw, native);
        assert_eq!(
            raw,
            fs::read(root.join(format!("rust-held-{call}.bgra"))).unwrap()
        );
        let image = fs::read(root.join(format!("rust-call-{call}.bgr"))).unwrap();
        assert_eq!(
            image,
            fs::read(root.join(format!("original-call-{call}.bgr"))).unwrap()
        );
        assert_eq!(
            image,
            fs::read(root.join(format!("original-held-{call}.bgr"))).unwrap()
        );
        assert_eq!(
            image,
            fs::read(root.join(format!("rust-held-{call}.bgr"))).unwrap()
        );
    }
    eprintln!(
        "native Rust macOS live_sample calls=3 original_samples=3 rust_conversions=3 retained_cpu=6 callback_owner_released=1 full_P1_acceptance=0 passed"
    );
}
