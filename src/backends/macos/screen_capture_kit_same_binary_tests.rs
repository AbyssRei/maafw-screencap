// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fresh-process metadata arms; never substitutes for an original oracle.
use super::*;
use crate::macos_libtest_host::same_binary_protocol as protocol;
use std::{
    fs,
    process::{Child, Command, Stdio},
};
#[path = "screen_capture_kit_live_sample_tests.rs"]
mod live_sample;
#[path = "screen_capture_kit_original_host.rs"]
mod original_host;

struct Scene {
    child: Child,
    root: PathBuf,
    finished: bool,
}
impl Scene {
    fn finish(&mut self) {
        assert!(!self.finished);
        fs::write(self.root.join("stop"), b"stop\n").unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "same-binary scene exited with {status}");
                break;
            }
            assert!(
                Instant::now() < until,
                "same-binary scene did not stop cooperatively"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            fs::read(self.root.join("stopped.txt")).unwrap(),
            b"window_closed=1\n"
        );
        self.finished = true;
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        // Failure cleanup owns exactly this child, never another GUI process.
        let _ = fs::write(self.root.join("stop"), b"stop\n");
        let until = Instant::now() + Duration::from_secs(15);
        while Instant::now() < until {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = fs::write(self.root.join("forced-stop.txt"), b"forced=1\n");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
#[ignore = "requires a native single-display Mac, prepared scene module and explicit diagnostic mode"]
fn records_first_display_with_same_executable_scene_host() {
    record(false);
}
#[test]
#[ignore = "requires a native single-display 1:1 Mac, full RGB scene and explicit call order"]
fn records_all_display_pixels_in_both_implementation_orders() {
    record(true);
}
#[test]
#[ignore = "requires a native Mac, separate CPU-view/census scene and explicit call order"]
fn records_full_display_native_witnesses_in_both_orders() {
    record_mode(true, true);
}
#[test]
#[ignore = "requires a native Mac and independent owned maximum-level RGB/witness scene"]
fn records_full_display_with_owned_maximum_level_scene_in_both_orders() {
    record_config(true, true, true);
}
#[test]
#[ignore = "requires a native Mac and separate original native-sample observation module"]
fn records_original_native_samples_before_and_after_source_conversion() {
    record_observed_config(true, true, false, true);
}
struct Timeline {
    start: Instant,
    rows: fs::File,
    rank: u32,
}
impl Timeline {
    fn now(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_nanos())
            .unwrap()
            .checked_add(1)
            .unwrap()
    }
    fn append(&mut self, side: u32, call: u32, begin: u64) {
        use std::io::Write;
        let end = self.now();
        writeln!(self.rows, "{}\t{side}\t{call}\t{begin}\t{end}", self.rank).unwrap();
        self.rows.flush().unwrap();
        self.rank = self.rank.checked_add(1).unwrap();
    }
}
fn original_calls(
    original: &original_host::Original,
    root: &Path,
    sequence: &mut Option<Timeline>,
    witness: &mut Option<Witness>,
) {
    for call in 0..3 {
        let begin = sequence.as_ref().map(Timeline::now);
        original.capture(call, root);
        if let Some(sequence) = sequence {
            sequence.append(0, call, begin.unwrap());
            if let Some(witness) = witness {
                witness.observe(sequence.rank);
            }
        }
    }
}
struct Witness {
    scene: PathBuf,
    start: Instant,
    rows: fs::File,
    next: u32,
}
impl Witness {
    fn observe(&mut self, slot: u32) {
        use std::io::Write;
        assert_eq!(slot, self.next);
        assert!(slot < 7);
        let now = || {
            u64::try_from(self.start.elapsed().as_nanos())
                .unwrap()
                .checked_add(1)
                .unwrap()
        };
        let begin = now();
        let temporary = self.scene.join(format!("witness-request-{slot}.tmp"));
        fs::write(&temporary, format!("slot={slot}\n")).unwrap();
        fs::rename(
            temporary,
            self.scene.join(format!("witness-request-{slot}")),
        )
        .unwrap();
        let acknowledgement = self.scene.join(format!("witness-{slot}-ready.txt"));
        let until = Instant::now() + Duration::from_secs(15);
        while !acknowledgement.exists() {
            assert!(
                Instant::now() < until,
                "native view/census witness timed out"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            fs::read(acknowledgement).unwrap(),
            format!("slot={slot}\nmain_thread=1\n").as_bytes()
        );
        writeln!(self.rows, "{slot}\t{begin}\t{}", now()).unwrap();
        self.rows.flush().unwrap();
        self.next = self.next.checked_add(1).unwrap();
    }
}
fn record(full: bool) {
    record_mode(full, false);
}
fn record_mode(full: bool, observe: bool) {
    record_config(full, observe, false);
}
fn record_config(full: bool, observe: bool, maximum: bool) {
    record_observed_config(full, observe, maximum, false);
}
fn record_observed_config(full: bool, observe: bool, maximum: bool, native_sample: bool) {
    use std::io::Write;
    assert!(!maximum || (full && observe));
    assert!(!native_sample || (full && observe && !maximum));
    let root = PathBuf::from(env::var_os("MFS_P1_MACOS_SAME_BINARY_ROOT").unwrap());
    let mode = env::var("MFS_P1_MACOS_SAME_BINARY_MODE")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(mode <= 1 && root.is_absolute() && root.is_dir());
    if native_sample {
        fs::write(
            root.join("native-sample-observer-enabled.txt"),
            b"schema=1\noriginal_native_sample_observer=1\n",
        )
        .unwrap();
    }
    assert_eq!(
        env::var("MFS_P1_MACOS_MAIN_THREAD_HOST").as_deref(),
        Ok("1")
    );
    // SAFETY: Read-only native caller identity; the existing explicit host pumps
    // AppKit on main while ordinary libtest executes this test on its worker.
    assert_eq!(unsafe { libc::pthread_main_np() }, 0);
    let adapter = Adapter::open();
    adapter.warm();
    let executable = std::env::current_exe().unwrap();
    let consumer =
        protocol::identity(&executable, std::process::id(), mode, "consumer", None).unwrap();
    fs::write(root.join("consumer-host.txt"), &consumer).unwrap();
    let consumer = protocol::fields(&consumer).unwrap();
    let frozen = root.parent().unwrap().join("same-binary-test-executable");
    if frozen.exists() {
        assert_eq!(
            protocol::fingerprint(&frozen).unwrap(),
            protocol::fingerprint(&executable).unwrap()
        );
    } else {
        let mut source = fs::File::open(&executable).unwrap();
        let mut output = fs::File::create_new(&frozen).unwrap();
        std::io::copy(&mut source, &mut output).unwrap();
        output.flush().unwrap();
        assert_eq!(
            protocol::fingerprint(&frozen).unwrap(),
            protocol::fingerprint(&executable).unwrap()
        );
    }
    let scene_root = root.join("scene");
    fs::create_dir(&scene_root).unwrap();
    let mut command = Command::new(&executable);
    if maximum {
        command.env("MFS_P1_MACOS_MAXIMUM_LEVEL_FIXTURE", "1");
        fs::write(
            root.join("maximum-level-enabled.txt"),
            b"schema=1\nmaximum_window_level_fixture=1\n",
        )
        .unwrap();
    } else {
        command.env_remove("MFS_P1_MACOS_MAXIMUM_LEVEL_FIXTURE");
    }
    if full {
        command.env("MFS_P1_MACOS_SAME_BINARY_FULL", "1");
    } else {
        command.env_remove("MFS_P1_MACOS_SAME_BINARY_FULL");
    }
    if observe {
        command.env("MFS_P1_MACOS_SAME_BINARY_WITNESS", "1");
    } else {
        command.env_remove("MFS_P1_MACOS_SAME_BINARY_WITNESS");
    }
    let child = command
        .env("MFS_P1_MACOS_SAME_BINARY_CHILD", "1")
        .env("MFS_P1_MACOS_SAME_BINARY_SCENE_ROOT", &scene_root)
        .env("MFS_P1_MACOS_SAME_BINARY_MODE", mode.to_string())
        .stdin(Stdio::null())
        .stdout(fs::File::create_new(root.join("scene-stdout.txt")).unwrap())
        .stderr(fs::File::create_new(root.join("scene-stderr.txt")).unwrap())
        .spawn()
        .unwrap();
    let mut scene = Scene {
        child,
        root: scene_root,
        finished: false,
    };
    fs::write(
        root.join("scene-child.txt"),
        format!(
            "pid={}\nmode={mode}\nsame_executable_requested=1\n",
            scene.child.id()
        ),
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(30);
    while !scene.root.join("ready.json").exists() {
        if let Some(status) = scene.child.try_wait().unwrap() {
            if status.code() == Some(77) {
                fs::write(
                    root.join("missing-environment.txt"),
                    b"scene_exit=77\nno_capture_attempted=1\n",
                )
                .unwrap();
                scene.finished = true;
                std::process::exit(77);
            }
            panic!("same-binary scene exited before ready: {status}");
        }
        assert!(
            Instant::now() < until,
            "same-binary scene readiness budget exceeded"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let producer =
        protocol::fields(&fs::read_to_string(scene.root.join("same-binary-host.txt")).unwrap())
            .unwrap();
    for field in [
        "program_path",
        "program_length",
        "program_fnv64",
        "mode",
        "schema",
    ] {
        assert_eq!(
            consumer[field], producer[field],
            "scene and consumer binary identities differ"
        );
    }
    assert_eq!(
        protocol::scalar(&producer["pid"]).unwrap(),
        u64::from(scene.child.id())
    );
    assert_ne!(consumer["pid"], producer["pid"]);
    let geometry_bytes = fs::read(scene.root.join("display-geometry.tsv")).unwrap();
    fs::write(root.join("consumer-expected-geometry.tsv"), &geometry_bytes).unwrap();
    let geometry = if full {
        display_geometry::parse_full(std::str::from_utf8(&geometry_bytes).unwrap())
    } else {
        display_geometry::parse(std::str::from_utf8(&geometry_bytes).unwrap())
    }
    .unwrap();
    assert_eq!(
        geometry.count, 1,
        "same-binary diagnostic requires one display"
    );
    assert_eq!(geometry.pid, scene.child.id());
    let mut rows = fs::File::create_new(root.join("calls.tsv")).unwrap();
    writeln!(rows, "call\tresult\tknown_patch\telapsed_ns").unwrap();
    let mut pending = Vec::new();
    pending.try_reserve_exact(3).unwrap();
    let mut images = Vec::new();
    images.try_reserve_exact(3).unwrap();
    let mut usable = 0;
    let mut known = 0;
    // Same parent process, executable identity, worker and still-live scene.
    // Legacy arms stay original-first. Full RGB arms declare both call orders.
    let original = original_host::Original::open(&root, native_sample);
    let mut sequence = if full {
        let mut rows = fs::File::create_new(root.join("sequence.tsv")).unwrap();
        writeln!(rows, "rank\timplementation\tcall\tbegin_ns\tend_ns").unwrap();
        Some(Timeline {
            start: Instant::now(),
            rows,
            rank: 0,
        })
    } else {
        None
    };
    let mut witness = if observe {
        assert!(full);
        fs::write(
            root.join("witness-enabled.txt"),
            b"schema=1\nrequests=7\nstrict_full_rgb_oracle=1\n",
        )
        .unwrap();
        let mut rows = fs::File::create_new(root.join("witness.tsv")).unwrap();
        writeln!(rows, "slot\tbegin_ns\tend_ns").unwrap();
        let mut value = Witness {
            scene: scene.root.clone(),
            start: sequence.as_ref().unwrap().start,
            rows,
            next: 0,
        };
        value.observe(0);
        Some(value)
    } else {
        None
    };
    let original_first = !full || mode == 0;
    if original_first {
        original_calls(&original, &root, &mut sequence, &mut witness);
    }
    let mut caller_results = Vec::new();
    let capture = Capture::new(0);
    for call in 0..3 {
        // A separate registry prevents another timed-out call's late raw sample
        // being relabeled as this call's current result.
        let trace = trace();
        let samples = held();
        let begin = sequence.as_ref().map(Timeline::now);
        let start = Instant::now();
        let result = capture.invoke(
            converter(&adapter, &trace, &samples, false, Duration::ZERO),
            trace.clone(),
        );
        let elapsed = start.elapsed().as_nanos();
        let selection = *trace.display_selection.lock().unwrap();
        if let Some(sequence) = &mut sequence {
            sequence.append(1, u32::try_from(call).unwrap(), begin.unwrap());
            if let Some(witness) = &mut witness {
                witness.observe(sequence.rank);
            }
        }
        let status = match &result {
            Ok(Some(image)) if !image.pixels().is_empty() => 1,
            Ok(Some(_)) => 3,
            Ok(None) => 0,
            Err(_) => 2,
        };
        fs::write(root.join(format!("call-{call}-status.txt")),format!(
            "schema=1\nresult={status}\ntimed_out={}\ncontent_calls={}\nsample_calls={}\ncompleted={}\nsource_failure={}\nnative_error={}\nfault={}\nwait_result={}\ninitial_inputs_released={}\n",
            u8::from(trace.timed_out.load(Ordering::Acquire)),trace.content_calls.load(Ordering::Acquire),trace.sample_calls.load(Ordering::Acquire),trace.completed.load(Ordering::Acquire),trace.source_failure.load(Ordering::Acquire),trace.native_error.load(Ordering::Acquire),trace.fault.load(Ordering::Acquire),trace.wait_result.load(Ordering::Acquire),trace.initial_inputs_released.load(Ordering::Acquire))).unwrap();
        fs::write(
            root.join(format!("call-{call}-selection.tsv")),
            display_patch::selection_record(selection),
        )
        .unwrap();
        fs::write(
            root.join(format!("call-{call}-error.txt")),
            format!("{:?}\n", result.as_ref().err()),
        )
        .unwrap();
        let mut correct = false;
        if status == 1 {
            let sample = samples.lock().unwrap();
            assert_eq!(sample.len(), 1);
            let raw = &sample[0];
            let image = result.as_ref().unwrap().as_ref().unwrap();
            let dimensions_match = (raw.width, raw.height) == (geometry.width, geometry.height);
            let selection_matches = selection.is_some_and(|s| {
                (s.display_count, s.display_id, s.width, s.height)
                    == (1, geometry.display, geometry.width, geometry.height)
            });
            if !full {
                assert!(dimensions_match, "successful native dimensions differ");
                assert!(selection_matches, "successful native selection differs");
            }
            // Full-display failures keep their actual returned CPU bytes and
            // native layout before any final known-pixel/selection assertion.
            fs::write(root.join(format!("call-{call}.raw")), raw.pixels()).unwrap();
            fs::write(root.join(format!("call-{call}.bgr")), image.pixels()).unwrap();
            fs::write(
                root.join(format!("call-{call}-layout.tsv")),
                format!("{}\t{}\t{}\n", raw.width, raw.height, raw.native_stride),
            )
            .unwrap();
            correct = dimensions_match
                && selection_matches
                && geometry.pixels(raw.pixels(), 4).is_ok()
                && geometry.pixels(image.pixels(), 3).is_ok();
            usable += 1;
            known += usize::from(correct);
        }
        writeln!(rows, "{call}\t{status}\t{}\t{elapsed}", u8::from(correct)).unwrap();
        rows.flush().unwrap();
        caller_results.push(status);
        pending.push((call, trace, samples));
        images.push(result.ok().flatten());
    }
    if !original_first {
        original_calls(&original, &root, &mut sequence, &mut witness);
    }
    if let Some(sequence) = &sequence {
        assert_eq!(sequence.rank, 6);
    }
    if let Some(witness) = &witness {
        assert_eq!(witness.next, 7);
    }
    // This observation budget follows all six calls, in the declared order.
    // It never resubmits a capture or converts a late frame into a caller result.
    let until = Instant::now() + Duration::from_secs(8);
    while pending
        .iter()
        .any(|(_, t, _)| t.request_released.load(Ordering::Acquire) != 1)
        && Instant::now() < until
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    scene.finish();
    original.finish(&root);
    let mut original_usable = 0;
    let mut original_known = 0;
    let mut equal_results = true;
    for (call, result) in caller_results.iter().enumerate() {
        let row: Vec<u64> = fs::read_to_string(root.join(format!("original-call-{call}.tsv")))
            .unwrap()
            .trim_end_matches('\n')
            .split('\t')
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(row.len(), 5);
        assert_eq!(row[0], u64::try_from(call).unwrap());
        equal_results &= row[1] == u64::try_from(*result).unwrap();
        if row[1] == 1 {
            let bytes = fs::read(root.join(format!("original-call-{call}.bgr"))).unwrap();
            assert_eq!(
                bytes,
                fs::read(root.join(format!("original-held-{call}.bgr"))).unwrap()
            );
            original_usable += 1;
            original_known += usize::from(geometry.pixels(&bytes, 3).is_ok());
        }
    }
    let mut faults = 0;
    for (call, trace, samples) in &pending {
        let released = trace.request_released.load(Ordering::Acquire) == 1
            && trace.semaphore_block_owner_released.load(Ordering::Acquire) == 1;
        if released {
            trace.reclaim_source_semaphore_for_fixture();
        }
        fs::write(root.join(format!("call-{call}-late.txt")),format!(
            "request_released={}\nsemaphore_block_owner_released={}\nfixture_reclaimed={}\ncompleted={}\nlate_completed={}\nfault={}\nraw_samples={}\n",
            trace.request_released.load(Ordering::Acquire),trace.semaphore_block_owner_released.load(Ordering::Acquire),trace.fixture_semaphore_reclaimed.load(Ordering::Acquire),trace.completed.load(Ordering::Acquire),u8::from(trace.late_completed.load(Ordering::Acquire)),trace.fault.load(Ordering::Acquire),samples.lock().unwrap().len())).unwrap();
        fs::write(
            root.join(format!("call-{call}-late-selection.tsv")),
            display_patch::selection_record(*trace.display_selection.lock().unwrap()),
        )
        .unwrap();
        faults += usize::from(trace.fault.load(Ordering::Acquire) != 0);
        if let Some(image) = &images[*call]
            && !image.pixels().is_empty()
        {
            fs::write(root.join(format!("held-{call}.bgr")), image.pixels()).unwrap();
            let samples = samples.lock().unwrap();
            assert_eq!(samples.len(), 1);
            fs::write(root.join(format!("held-{call}.raw")), samples[0].pixels()).unwrap();
        }
    }
    let schema = if full { 3 } else { 2 };
    fs::write(root.join("summary.txt"),format!("schema={schema}\nmode={mode}\ninvocations=3\nusable_images={usable}\nknown_patch_images={known}\nscene_closed=1\nscene_child_exit=0\nsame_binary=1\nfaults={faults}\nno_capture_retry=1\noriginal_oracle_replaced=0\nfull_P1_acceptance=0\noriginal_invocations=3\noriginal_usable_images={original_usable}\noriginal_known_patch_images={original_known}\noriginal_first={}\noriginal_owner_closed=1\npaired_caller_results={}\n{}",u8::from(original_first),u8::from(equal_results),if full {"full_display=1\n"} else {""})).unwrap();
    if observe {
        let captured = usable + original_usable;
        eprintln!(
            "native Rust macOS full_display_witness mode={mode} requests=7 captures={captured} held={captured} strict_full_rgb_oracle=1 recorded"
        );
    }
    // Keep capture failure observations, but require the CG metadata arm as a
    // strict positive control. Neither arm is a replacement source oracle.
    if maximum {
        eprintln!(
            "native Rust macOS maximum_level_fixture mode={mode} calls=6 witness_requests=7 owned_scene_only=1 strict_full_rgb_oracle=1 recorded"
        );
    }
    if native_sample {
        eprintln!(
            "native Rust macOS original_native_sample mode={mode} calls=6 native_samples=3 before_after_copies=6 strict_full_rgb_oracle=1 recorded"
        );
    }
    assert_eq!(faults, 0);
    assert!(
        equal_results,
        "same-process original/Rust caller results differ"
    );
    assert_eq!(
        original_usable, original_known,
        "original Some result has unknown target pixels"
    );
    assert_eq!(usable, known, "Rust Some result has unknown target pixels");
    if mode == 1 || full {
        assert_eq!(
            (usable, known),
            (3, 3),
            "same-binary CG positive control failed"
        );
        assert_eq!(
            (original_usable, original_known),
            (3, 3),
            "same-process original CG positive control failed"
        );
    }
    if full {
        eprintln!(
            "native Rust macOS full_display mode={mode} original_first={} calls=6 known_full_images=6 held_after_scene_close=6 full_rgb=1 no_capture_retry=1 full_P1_acceptance=0 passed",
            u8::from(original_first)
        );
    }
    if !full {
        eprintln!(
            "native Rust macOS same_binary mode={mode} invocations=3 usable_images={usable} known_patch_images={known} scene_closed=1 diagnostic_only=1 no_capture_retry=1 full_P1_acceptance=0 original_calls=3 original_first=1 caller_results_equal=1 recorded"
        );
    }
}
