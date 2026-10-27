// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Feasibility only. No native object gains a Send/Sync implementation, and no
// SDK Session, public API or upstream timeout/replay implementation is added.
use block2::RcBlock;
use objc2::{AnyThread, MainThreadMarker, rc::Retained};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSWindow,
    NSWindowStyleMask,
};
use objc2_core_graphics::{CGPreflightScreenCaptureAccess, kCGColorSpaceSRGB};
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{
    CVPixelBuffer, CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetDataSize, CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType,
    CVPixelBufferGetWidth, CVPixelBufferIsPlanar, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSError, NSPoint, NSRect, NSRunLoop, NSSize};
use objc2_screen_capture_kit::{
    SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    process::ExitCode,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicUsize, Ordering},
    },
    thread::ThreadId,
    time::{Duration, Instant},
};

const WIDTH: usize = 47;
const HEIGHT: usize = 31;
type ProbeResult<T> = Result<T, String>;

#[derive(Default)]
struct Metrics {
    error: AtomicU32,
    native_error: AtomicIsize,
    unlock_error: AtomicI32,
    completed: AtomicUsize,
    discarded: AtomicUsize,
    inputs_dropped: AtomicUsize,
    content_thread: Mutex<Option<ThreadId>>,
    sample_thread: Mutex<Option<ThreadId>>,
    release_thread: Mutex<Option<ThreadId>>,
}
impl Metrics {
    fn failure(&self, code: u32) {
        let _ = self
            .error
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
    fn thread(&self, slot: &Mutex<Option<ThreadId>>) {
        if let Ok(mut value) = slot.lock() {
            *value = Some(std::thread::current().id());
        } else {
            self.failure(5);
        }
    }
}

struct Request {
    done: AtomicBool,
    abandoned: AtomicBool,
    pixels: Mutex<Option<Vec<u8>>>,
    metrics: Arc<Metrics>,
}
impl Request {
    fn new(metrics: Arc<Metrics>) -> Self {
        Self {
            done: AtomicBool::new(false),
            abandoned: AtomicBool::new(false),
            pixels: Mutex::new(None),
            metrics,
        }
    }
    fn complete(&self, result: Result<Option<Vec<u8>>, u32>) {
        if self.metrics.completed.fetch_add(1, Ordering::AcqRel) != 0 {
            self.metrics.failure(9);
        }
        match result {
            Ok(pixels) if !self.abandoned.load(Ordering::Acquire) => {
                if let Ok(mut output) = self.pixels.lock() {
                    *output = pixels;
                } else {
                    self.metrics.failure(5);
                }
            }
            Ok(_) => {
                self.metrics.discarded.fetch_add(1, Ordering::Release);
            }
            Err(code) => self.metrics.failure(code),
        }
        // This error/wakeup path requires no formatted error or allocation.
        self.done.store(true, Ordering::Release);
    }
}

// Native filter/configuration are created in the content callback and frozen
// before dispatch. Their retained block payload only keeps them alive through
// the native completion, matching upstream's callback-owned release. The block
// never mutates or exposes them on another thread. Capture/release identities
// are recorded; this does not establish general native-object Send/Sync.
struct Inputs {
    filter: Retained<SCContentFilter>,
    config: Retained<SCStreamConfiguration>,
    // Fields drop in declaration order. This guard reports release only after
    // both native retained owners have actually been released.
    released: ReleasedInputs,
}
impl Inputs {
    fn keep_alive(&self) {
        let _ = (&self.filter, &self.config, &self.released);
    }
}
struct ReleasedInputs {
    metrics: Arc<Metrics>,
}
impl Drop for ReleasedInputs {
    fn drop(&mut self) {
        self.metrics.thread(&self.metrics.release_thread);
        self.metrics.inputs_dropped.fetch_add(1, Ordering::Release);
    }
}

struct PixelLock<'a> {
    pixel: &'a CVPixelBuffer,
    metrics: &'a Metrics,
}
impl Drop for PixelLock<'_> {
    fn drop(&mut self) {
        // SAFETY: The retained image and successful ReadOnly lock outlive this
        // guard; all pixel borrows end first. Unlock exactly that successful lock.
        let code =
            unsafe { CVPixelBufferUnlockBaseAddress(self.pixel, CVPixelBufferLockFlags::ReadOnly) };
        if code != 0 {
            self.metrics.unlock_error.store(code, Ordering::Release);
        }
    }
}

fn copy_pixels(sample: &CMSampleBuffer, metrics: &Metrics) -> Result<Vec<u8>, u32> {
    // SAFETY: The native completion lends a valid video sample for this call.
    // This helper obtains its own retained image reference before accessing it.
    let image = unsafe { sample.image_buffer() }.ok_or(3_u32)?;
    if CVPixelBufferIsPlanar(&image)
        || CVPixelBufferGetPixelFormatType(&image) != kCVPixelFormatType_32BGRA
        || CVPixelBufferGetWidth(&image) != WIDTH
        || CVPixelBufferGetHeight(&image) != HEIGHT
    {
        return Err(3);
    }
    // SAFETY: The retained image is live and the flags request CPU read access.
    let code = unsafe { CVPixelBufferLockBaseAddress(&image, CVPixelBufferLockFlags::ReadOnly) };
    if code != 0 {
        return Err(7);
    }
    let lock = PixelLock {
        pixel: &image,
        metrics,
    };
    let stride = CVPixelBufferGetBytesPerRow(&image);
    let row = WIDTH.checked_mul(4).ok_or(3_u32)?;
    let span = HEIGHT
        .checked_sub(1)
        .and_then(|n| n.checked_mul(stride))
        .and_then(|n| n.checked_add(row))
        .ok_or(3_u32)?;
    if stride < row || span > CVPixelBufferGetDataSize(&image) {
        return Err(3);
    }
    isize::try_from(span).map_err(|_| 3_u32)?;
    let pointer = CVPixelBufferGetBaseAddress(&image).cast::<u8>();
    if pointer.is_null() {
        return Err(3);
    }
    let length = row.checked_mul(HEIGHT).ok_or(3_u32)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(length).map_err(|_| 4_u32)?;
    for y in 0..HEIGHT {
        let offset = y.checked_mul(stride).ok_or(3_u32)?;
        // SAFETY: Successful CPU lock and the checked native readable span cover
        // each target row. Inter-row and last-row allocation padding stay private.
        pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(pointer.add(offset), row) });
    }
    drop(lock);
    if metrics.unlock_error.load(Ordering::Acquire) != 0 {
        return Err(8);
    }
    Ok(pixels)
}

fn begin(window_id: u32, request: &Arc<Request>) {
    let state = request.clone();
    let content = RcBlock::new(move |raw: *mut SCShareableContent, error: *mut NSError| {
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), u32> {
            state.metrics.thread(&state.metrics.content_thread);
            // SAFETY: Both nullable pointers are borrowed only in this completion.
            if let Some(error) = unsafe { error.as_ref() } {
                state
                    .metrics
                    .native_error
                    .store(error.code(), Ordering::Release);
                return Err(2);
            }
            let content = unsafe { raw.as_ref() }.ok_or(2_u32)?;
            // SAFETY: Content is live for this callback; the returned immutable
            // array owns the enumerated SCWindow references.
            let windows = unsafe { content.windows() };
            let mut target = None;
            for index in 0..windows.count() {
                let window = windows.objectAtIndex(index);
                // SAFETY: Enumerated SCWindow remains retained in this scope.
                if unsafe { window.windowID() } == window_id {
                    target = Some(window);
                    break;
                }
            }
            let target = target.ok_or(6_u32)?;
            // SAFETY: Framework objects are created/accessed in this native
            // callback, before any asynchronous use. Input storage is retained
            // in the copied completion block until the native request ends.
            let filter = unsafe {
                SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &target)
            };
            let config = unsafe { SCStreamConfiguration::new() };
            unsafe {
                config.setWidth(WIDTH);
                config.setHeight(HEIGHT);
                config.setPixelFormat(kCVPixelFormatType_32BGRA);
                config.setColorSpaceName(kCGColorSpaceSRGB);
                config.setShowsCursor(false);
            }
            let inputs = Inputs {
                filter: filter.clone(),
                config: config.clone(),
                released: ReleasedInputs {
                    metrics: state.metrics.clone(),
                },
            };
            let sample_state = state.clone();
            let sample = RcBlock::new(move |raw: *mut CMSampleBuffer, error: *mut NSError| {
                // Borrow the captured fields so they survive every success/error
                // callback, including when the host request was abandoned.
                inputs.keep_alive();
                let result = catch_unwind(AssertUnwindSafe(|| {
                    sample_state
                        .metrics
                        .thread(&sample_state.metrics.sample_thread);
                    // SAFETY: Native completion lends these pointers only for
                    // this invocation; no borrowed result escapes it.
                    if let Some(error) = unsafe { error.as_ref() } {
                        sample_state
                            .metrics
                            .native_error
                            .store(error.code(), Ordering::Release);
                        return Err(2);
                    }
                    let sample = unsafe { raw.as_ref() }.ok_or(3_u32)?;
                    if sample_state.abandoned.load(Ordering::Acquire) {
                        return Ok(None);
                    }
                    copy_pixels(sample, &sample_state.metrics).map(Some)
                }));
                sample_state.complete(result.unwrap_or(Err(1)));
            });
            // SAFETY: Block payloads use atomic/Mutex state, never caller stack
            // pointers. Native inputs are frozen, kept by the copied block, and
            // released according to the same completion ownership as upstream.
            unsafe {
                SCScreenshotManager::captureSampleBufferWithFilter_configuration_completionHandler(
                    &filter,
                    &config,
                    Some(&sample),
                );
            }
            Ok(())
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(code)) => state.complete(Err(code)),
            Err(_) => state.complete(Err(1)),
        }
    });
    // SAFETY: Escaping closure captures only owned Arc state and a copied native
    // window identifier. It may run on a framework queue; no AppKit calls occur.
    unsafe {
        SCShareableContent::getShareableContentWithCompletionHandler(&content);
    }
}

fn pump() {
    objc2::rc::autoreleasepool(|_| {
        // SAFETY: Foundation owns this immutable process-lifetime mode string.
        let mode = unsafe { NSDefaultRunLoopMode };
        NSRunLoop::currentRunLoop()
            .runMode_beforeDate(mode, &NSDate::dateWithTimeIntervalSinceNow(0.01));
    });
}
fn wait(predicate: impl Fn() -> bool) -> ProbeResult<()> {
    let until = Instant::now() + Duration::from_secs(8);
    while !predicate() {
        if Instant::now() >= until {
            return Err(
                "native callback/result ownership did not drain within the probe budget".into(),
            );
        }
        pump();
    }
    Ok(())
}
fn check_metrics(metrics: &Metrics) -> ProbeResult<()> {
    let error = metrics.error.load(Ordering::Acquire);
    if error != 0 || metrics.unlock_error.load(Ordering::Acquire) != 0 {
        return Err(format!(
            "ScreenCaptureKit failure={error}, native={}, unlock={}",
            metrics.native_error.load(Ordering::Acquire),
            metrics.unlock_error.load(Ordering::Acquire)
        ));
    }
    if metrics.completed.load(Ordering::Acquire) != 1
        || metrics.inputs_dropped.load(Ordering::Acquire) != 1
    {
        return Err("native completion/retained-input release did not occur exactly once".into());
    }
    Ok(())
}
fn validate(pixels: &[u8], expected: [u8; 4]) -> ProbeResult<()> {
    if pixels.len() != WIDTH * HEIGHT * 4 {
        return Err("independent CPU image length mismatch".into());
    }
    // Window edge/shadow geometry is outside this small feasibility assertion.
    for y in 2..HEIGHT - 2 {
        for x in 2..WIDTH - 2 {
            let offset = y
                .checked_mul(WIDTH)
                .and_then(|v| v.checked_add(x))
                .and_then(|v| v.checked_mul(4))
                .ok_or("pixel offset overflow")?;
            let end = offset.checked_add(4).ok_or("pixel end overflow")?;
            if pixels.get(offset..end) != Some(expected.as_slice()) {
                return Err(format!(
                    "controlled-window BGRA mismatch at ({x},{y}): {:?}, expected {expected:?}",
                    pixels.get(offset..end)
                ));
            }
        }
    }
    Ok(())
}
struct Window(Retained<NSWindow>);
impl Drop for Window {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn probe() -> ProbeResult<()> {
    let mtm =
        MainThreadMarker::new().ok_or("AppKit probe did not start on the process main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    if !app.setActivationPolicy(NSApplicationActivationPolicy::Accessory) {
        return Err("fixture activation policy was rejected".into());
    }
    app.finishLaunching();
    // SAFETY: Only this process-main-thread owner creates/accesses/closes the
    // AppKit fixture; disabling release-on-close preserves its Retained owner.
    let window = Window(unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(100.0, 100.0),
                NSSize::new(WIDTH as f64, HEIGHT as f64),
            ),
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    });
    unsafe {
        window.0.setReleasedWhenClosed(false);
    }
    window.0.setHasShadow(false);
    window.0.setOpaque(true);
    window.0.orderFrontRegardless();
    let id = u32::try_from(window.0.windowNumber()).map_err(|e| e.to_string())?;
    let colors = [
        NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 1.0, 0.0, 1.0),
        NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.0, 0.0, 1.0),
        NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 1.0, 1.0),
    ];
    let expected = [[0, 255, 0, 255], [0, 0, 255, 255], [255, 0, 0, 255]];
    let mut final_pixels = None;
    for cycle in 0..20 {
        window.0.setBackgroundColor(Some(&colors[cycle % 3]));
        window.0.display();
        app.updateWindows();
        let painted = Instant::now() + Duration::from_millis(100);
        while Instant::now() < painted {
            pump();
        }
        let metrics = Arc::try_new(Metrics::default()).map_err(|e| e.to_string())?;
        let request = Arc::try_new(Request::new(metrics.clone())).map_err(|e| e.to_string())?;
        begin(id, &request);
        if let Err(error) = wait(|| {
            request.done.load(Ordering::Acquire)
                && Arc::strong_count(&request) == 1
                && (metrics.error.load(Ordering::Acquire) != 0
                    || metrics.inputs_dropped.load(Ordering::Acquire) == 1)
        }) {
            request.abandoned.store(true, Ordering::Release);
            return Err(error);
        }
        check_metrics(&metrics)?;
        let pixels = request
            .pixels
            .lock()
            .map_err(|_| "poisoned pixel slot")?
            .take()
            .ok_or("native capture published no CPU pixels")?;
        drop(request);
        validate(&pixels, expected[cycle % 3])?;
        println!(
            "ScreenCaptureKit cycle {}/20: {}x{} BGRA, owner={:?}, content={:?}, sample={:?}, release={:?}, retained inputs released=1",
            cycle + 1,
            WIDTH,
            HEIGHT,
            std::thread::current().id(),
            metrics.content_thread.lock().unwrap(),
            metrics.sample_thread.lock().unwrap(),
            metrics.release_thread.lock().unwrap()
        );
        final_pixels = Some((pixels, expected[cycle % 3]));
    }
    // Abandon the caller's request owner before native completion. The block
    // payload must keep its state/input lifetime, discard the result, then drain.
    let metrics = Arc::try_new(Metrics::default()).map_err(|e| e.to_string())?;
    let request = Arc::try_new(Request::new(metrics.clone())).map_err(|e| e.to_string())?;
    request.abandoned.store(true, Ordering::Release);
    let weak = Arc::downgrade(&request);
    begin(id, &request);
    drop(request);
    wait(|| weak.upgrade().is_none() && metrics.inputs_dropped.load(Ordering::Acquire) == 1)?;
    check_metrics(&metrics)?;
    if metrics.discarded.load(Ordering::Acquire) != 1 {
        return Err("abandoned native result was not discarded exactly once".into());
    }
    println!(
        "Abandoned caller owner: native completion discarded, weak state released, retained inputs released=1"
    );
    drop(window);
    drop(colors);
    drop(app);
    let (pixels, color) = final_pixels.ok_or("no final frame")?;
    std::thread::spawn(move || validate(&pixels, color))
        .join()
        .map_err(|_| "independent pixel worker panicked")??;
    println!(
        "ScreenCaptureKit: 20 captured frames, one abandoned native request, CPU pixels survive native/fixture teardown and cross-thread ownership"
    );
    Ok(())
}

pub fn run() -> ExitCode {
    if std::env::var("MFS_P0_MACOS_CAPTURE").as_deref() != Ok("1") {
        eprintln!("Explicit native capture is required: use cargo xtask macos-prototype");
        return ExitCode::FAILURE;
    }
    let permission = CGPreflightScreenCaptureAccess();
    println!(
        "native screen-capture permission preflight={permission}; no authorization dialog is requested"
    );
    let result = objc2::rc::autoreleasepool(|_| probe());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            if !permission && error.contains("native=-3801") {
                eprintln!(
                    "missing_environment: macOS Screen Recording authorization is unavailable"
                );
                ExitCode::from(77)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
