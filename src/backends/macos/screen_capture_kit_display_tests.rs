// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[test]
#[ignore = "requires native first-display AppKit patch scene and unchanged source capture"]
fn captures_first_display_owned_patch_and_keeps_pixels_after_scene_shutdown() {
    use std::io::Write;
    let directory = directory();
    let bytes = std::fs::read(env::var_os("MFS_P1_MACOS_DISPLAY_GEOMETRY").unwrap()).unwrap();
    assert_eq!(
        bytes,
        std::fs::read(directory.join("source-display-geometry.tsv")).unwrap()
    );
    let geometry = display_geometry::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(geometry.window, scene());
    assert!(geometry.pid > 0 && geometry.display > 0);
    std::fs::write(directory.join("rust-display-geometry.tsv"), &bytes).unwrap();
    let adapter = Adapter::open();
    adapter.warm();
    let samples = held();
    let mut images = Vec::new();
    images.try_reserve_exact(3).unwrap();
    let mut traces = std::fs::File::create_new(directory.join("rust-display-trace.tsv")).unwrap();
    {
        let capture = Capture::new(0);
        for call in 0..3 {
            let trace = trace();
            let sample_start = samples.lock().unwrap().len();
            let started = Instant::now();
            let result = capture.invoke(
                converter(&adapter, &trace, &samples, false, Duration::ZERO),
                trace.clone(),
            );
            let status = match &result {
                Ok(Some(_)) => 1,
                Ok(None) => 0,
                Err(_) => 2,
            };
            // Persist return/failure BEFORE unwrap/drain. A late callback must
            // not erase the actual caller's None/error/timeout observation.
            std::fs::write(directory.join(format!("display-call-{call}.rust-status.txt")),format!(
                "result={status}\nelapsed_ns={}\ntimed_out={}\ncontent_calls={}\nsample_calls={}\ncompleted={}\nsource_failure={}\nnative_error={}\nfault={}\nwait_result={}\nerror_debug={:?}\n",
                started.elapsed().as_nanos(),u8::from(trace.timed_out.load(Ordering::Acquire)),trace.content_calls.load(Ordering::Acquire),trace.sample_calls.load(Ordering::Acquire),trace.completed.load(Ordering::Acquire),trace.source_failure.load(Ordering::Acquire),trace.native_error.load(Ordering::Acquire),trace.fault.load(Ordering::Acquire),trace.wait_result.load(Ordering::Acquire),result.as_ref().err())).unwrap();
            if status != 1 {
                // Observe THIS failed call's late owners without another
                // capture or a longer source wait. Its original Result remains
                // failed even if a native callback subsequently completes.
                let until = Instant::now() + Duration::from_secs(15);
                while Instant::now() < until
                    && (trace.request_released.load(Ordering::Acquire) != 1
                        || trace.sample_calls.load(Ordering::Acquire) != 0
                            && trace.block_inputs_released.load(Ordering::Acquire) != 1)
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                if trace.request_released.load(Ordering::Acquire) == 1 {
                    trace.reclaim_source_semaphore_for_fixture();
                }
                let (total, new_samples, late_sample) = {
                    let samples = samples.lock().unwrap();
                    (
                        samples.len(),
                        samples.len().checked_sub(sample_start).unwrap(),
                        samples.get(sample_start).cloned(),
                    )
                };
                std::fs::write(directory.join(format!("display-call-{call}.rust-late.txt")),format!(
                    "original_result={status}\nlate_completed={late}\ncontent_calls={content}\nsample_calls={callbacks}\ncompleted={completed}\nrequest_released={request}\nblock_inputs_released={inputs}\nsemaphore_block_released={semaphore}\nfixture_reclaimed={reclaimed}\nsource_failure={failure}\nnative_error={native}\nfault={fault}\nlocks={locks}\nunlocks={unlocks}\nretained_samples_total={total}\ncopied_samples_this_call={new_samples}\nsample_width={width}\nsample_height={height}\nsample_native_stride={stride}\n",
                    late=u8::from(trace.late_completed.load(Ordering::Acquire)),content=trace.content_calls.load(Ordering::Acquire),callbacks=trace.sample_calls.load(Ordering::Acquire),completed=trace.completed.load(Ordering::Acquire),request=trace.request_released.load(Ordering::Acquire),inputs=trace.block_inputs_released.load(Ordering::Acquire),semaphore=trace.semaphore_block_owner_released.load(Ordering::Acquire),reclaimed=trace.fixture_semaphore_reclaimed.load(Ordering::Acquire),failure=trace.source_failure.load(Ordering::Acquire),native=trace.native_error.load(Ordering::Acquire),fault=trace.fault.load(Ordering::Acquire),locks=trace.pixels.locks.load(Ordering::Acquire),unlocks=trace.pixels.unlocks.load(Ordering::Acquire),width=late_sample.as_ref().map_or(0,|s|s.width),height=late_sample.as_ref().map_or(0,|s|s.height),stride=late_sample.as_ref().map_or(0,|s|s.native_stride))).unwrap();
                // Never label a prior successful call's retained sample as a
                // late result of this failed call. Release the mutex before IO.
                if let Some(sample) = late_sample {
                    std::fs::write(
                        directory.join(format!("display-call-{call}.rust-late.raw")),
                        sample.pixels(),
                    )
                    .unwrap();
                }
            }
            let image = result.unwrap().unwrap();
            drained(&trace, true);
            trace.reclaim_source_semaphore_for_fixture();
            let guard = samples.lock().unwrap();
            let raw = guard.last().unwrap();
            std::fs::write(
                directory.join(format!("display-call-{call}.rust.raw")),
                raw.pixels(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("display-call-{call}.rust.bgr")),
                image.pixels(),
            )
            .unwrap();
            writeln!(
                traces,
                "{call}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                trace.content_calls.load(Ordering::Acquire),
                trace.sample_calls.load(Ordering::Acquire),
                trace.completed.load(Ordering::Acquire),
                trace.request_released.load(Ordering::Acquire),
                trace.block_inputs_released.load(Ordering::Acquire),
                trace.pixels.locks.load(Ordering::Acquire),
                trace.pixels.unlocks.load(Ordering::Acquire),
                trace.pixels.conversions.load(Ordering::Acquire),
                trace.pixels.copied_bytes.load(Ordering::Acquire),
                trace.source_semaphore_retained.load(Ordering::Acquire),
                trace.fixture_semaphore_reclaimed.load(Ordering::Acquire),
                trace.source_failure.load(Ordering::Acquire),
                trace.fault.load(Ordering::Acquire),
                raw.native_stride,
                raw.width,
                raw.height
            )
            .unwrap();
            traces.flush().unwrap();
            // Persist actual payloads/evidence before asserting target pixels,
            // so a native mismatch remains independently inspectable.
            assert_eq!((raw.width, raw.height), (geometry.width, geometry.height));
            assert!(raw.native_stride >= geometry.width.checked_mul(4).unwrap());
            geometry.pixels(raw.pixels(), 4).unwrap();
            geometry.pixels(image.pixels(), 3).unwrap();
            images.push(image);
        }
    }
    stop_scene();
    let raw = std::mem::take(&mut *samples.lock().unwrap());
    assert_eq!(raw.len(), 3);
    std::thread::spawn(move || {
        for (call, (sample, image)) in raw.iter().zip(&images).enumerate() {
            std::fs::write(
                directory.join(format!("display-held-{call}.rust.raw")),
                sample.pixels(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("display-held-{call}.rust.bgr")),
                image.pixels(),
            )
            .unwrap();
            geometry.pixels(sample.pixels(), 4).unwrap();
            geometry.pixels(image.pixels(), 3).unwrap();
        }
    })
    .join()
    .unwrap();
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust macOS display_patch first_enumerated=1 captures=3 retained_raw=3 retained_bgr=3 known_owned_patch=3 scene_closed_before_cpu_read=1 callback_lock_pairs=3 fixture_reclaimed_refs=3 full_display_pixels_unasserted=1 passed"
    );
}
