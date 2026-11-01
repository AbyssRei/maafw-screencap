// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fresh-process metadata arms; never substitutes for an original oracle.
use super::*;
use crate::macos_libtest_host::same_binary_protocol as protocol;
use std::{
    fs,
    process::{Child, Command, Stdio},
};

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
    use std::io::Write;
    let root = PathBuf::from(env::var_os("MFS_P1_MACOS_SAME_BINARY_ROOT").unwrap());
    let mode = env::var("MFS_P1_MACOS_SAME_BINARY_MODE")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(mode <= 1 && root.is_absolute() && root.is_dir());
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
    let child = Command::new(&executable)
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
    let geometry = display_geometry::parse(std::str::from_utf8(&geometry_bytes).unwrap()).unwrap();
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
    let capture = Capture::new(0);
    for call in 0..3 {
        // A separate registry prevents another timed-out call's late raw sample
        // being relabeled as this call's current result.
        let trace = trace();
        let samples = held();
        let start = Instant::now();
        let result = capture.invoke(
            converter(&adapter, &trace, &samples, false, Duration::ZERO),
            trace.clone(),
        );
        let elapsed = start.elapsed().as_nanos();
        let selection = *trace.display_selection.lock().unwrap();
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
            assert_eq!((raw.width, raw.height), (geometry.width, geometry.height));
            let selection = selection.expect("successful native call has no actual selection");
            assert_eq!(
                (
                    selection.display_count,
                    selection.display_id,
                    selection.width,
                    selection.height
                ),
                (1, geometry.display, geometry.width, geometry.height)
            );
            fs::write(root.join(format!("call-{call}.raw")), raw.pixels()).unwrap();
            fs::write(root.join(format!("call-{call}.bgr")), image.pixels()).unwrap();
            fs::write(
                root.join(format!("call-{call}-layout.tsv")),
                format!("{}\t{}\t{}\n", raw.width, raw.height, raw.native_stride),
            )
            .unwrap();
            correct = geometry.pixels(raw.pixels(), 4).is_ok()
                && geometry.pixels(image.pixels(), 3).is_ok();
            usable += 1;
            known += usize::from(correct);
        }
        writeln!(rows, "{call}\t{status}\t{}\t{elapsed}", u8::from(correct)).unwrap();
        rows.flush().unwrap();
        pending.push((call, trace, samples));
        images.push(result.ok().flatten());
    }
    // This observation budget follows all three original five-second waits.
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
    fs::write(root.join("summary.txt"),format!("schema=1\nmode={mode}\ninvocations=3\nusable_images={usable}\nknown_patch_images={known}\nscene_closed=1\nscene_child_exit=0\nsame_binary=1\nfaults={faults}\nno_capture_retry=1\noriginal_oracle_replaced=0\nfull_P1_acceptance=0\n")).unwrap();
    // Keep capture failure observations, but require the CG metadata arm as a
    // strict positive control. Neither arm is a replacement source oracle.
    assert_eq!(faults, 0);
    if mode == 1 {
        assert_eq!(
            (usable, known),
            (3, 3),
            "same-binary CG positive control failed"
        );
    }
    eprintln!(
        "native Rust macOS same_binary mode={mode} invocations=3 usable_images={usable} known_patch_images={known} scene_closed=1 diagnostic_only=1 no_capture_retry=1 full_P1_acceptance=0 recorded"
    );
}
