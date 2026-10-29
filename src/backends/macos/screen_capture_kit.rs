// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private P1 migration of the fixed one-shot ScreenCaptureKit class. No public
//! SDK Session, worker, cancellation, cache or target policy is introduced.
use crate::platform::macos::{
    Error,
    pixel_buffer::{self, Sample},
};
use block2::RcBlock;
use dispatch2::{DispatchRetained, DispatchSemaphore, DispatchTime};
use objc2::{AnyThread, rc::Retained, runtime::AnyClass};
use objc2_core_graphics::kCGColorSpaceSRGB;
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::kCVPixelFormatType_32BGRA;
use objc2_foundation::{NSArray, NSError};
use objc2_screen_capture_kit::{
    SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration, SCWindow,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicUsize, Ordering},
    },
    thread::ThreadId,
};

pub(crate) trait Converter: Send + Sync + 'static {
    type Image: Send + 'static;
    fn empty(&self) -> Result<Self::Image, Error>;
    /// Called synchronously in the native sample callback BEFORE Unlock.
    /// The independent packed BGRA copy adds work; native storage never escapes.
    fn convert(&self, sample: Sample) -> Result<Self::Image, Error>;
}
#[derive(Default)]
pub(crate) struct Trace {
    pub pixels: pixel_buffer::Evidence,
    pub content_calls: AtomicUsize,
    pub sample_calls: AtomicUsize,
    pub completed: AtomicUsize,
    pub timed_out: AtomicBool,
    pub late_completed: AtomicBool,
    pub initial_inputs_released: AtomicUsize,
    pub block_inputs_released: AtomicUsize,
    pub request_released: AtomicUsize,
    pub semaphore_block_owner_released: AtomicUsize,
    pub source_semaphore_retained: AtomicUsize,
    pub fixture_semaphore_reclaimed: AtomicUsize,
    pub signals: AtomicUsize,
    pub wait_result: AtomicIsize,
    pub signal_result: AtomicIsize,
    pub native_error: AtomicIsize,
    // 0 success/empty; 1 content error; 2 missing target/display; 3 sample error;
    // 4 unavailable native screenshot class. Not public ABI/status values.
    pub source_failure: AtomicI32,
    pub fault: AtomicI32,
    pub caller: Mutex<Option<ThreadId>>,
    pub content_thread: Mutex<Option<ThreadId>>,
    pub sample_thread: Mutex<Option<ThreadId>>,
    pub initial_release_thread: Mutex<Option<ThreadId>>,
    pub block_release_thread: Mutex<Option<ThreadId>>,
    pub request_release_thread: Mutex<Option<ThreadId>>,
    source_semaphore: Mutex<Option<DispatchRetained<DispatchSemaphore>>>,
}
impl Drop for Trace {
    fn drop(&mut self) {
        // P1 preserves the original MRC function's unreleased create +1. This
        // is deliberately not a product cleanup success. P3 changes ownership
        // after U/P are frozen; explicit acceptance fixtures may reclaim it
        // after the real caller and all native callback owners have ended.
        if let Some(owner) = self
            .source_semaphore
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner())
            .take()
        {
            std::mem::forget(owner);
        }
    }
}
impl Trace {
    fn fault(&self, code: i32) {
        let _ = self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
    fn thread(&self, slot: &Mutex<Option<ThreadId>>) {
        match slot.lock() {
            Ok(mut value) => *value = Some(std::thread::current().id()),
            Err(_) => self.fault(-2),
        }
    }
    fn once(&self, counter: &AtomicUsize) -> bool {
        if counter
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            true
        } else {
            self.fault(-3);
            false
        }
    }
    #[cfg(test)]
    fn reclaim_source_semaphore_for_fixture(&self) {
        assert_eq!(
            self.request_released.load(Ordering::Acquire),
            1,
            "fixture must not reclaim a live caller/callback semaphore"
        );
        let owner = self.source_semaphore.lock().unwrap().take().unwrap();
        drop(owner);
        self.fixture_semaphore_reclaimed.store(1, Ordering::Release);
    }
}
struct ReleasedRequest(Arc<Trace>);
impl Drop for ReleasedRequest {
    fn drop(&mut self) {
        self.0.thread(&self.0.request_release_thread);
        self.0.request_released.store(1, Ordering::Release);
    }
}
struct ReleasedSemaphore(Arc<Trace>);
impl Drop for ReleasedSemaphore {
    fn drop(&mut self) {
        self.0
            .semaphore_block_owner_released
            .store(1, Ordering::Release);
    }
}
struct Semaphore {
    value: DispatchRetained<DispatchSemaphore>,
    // After the actual native +1 release. Request's caller and copied blocks
    // keep this owner through active wait/signal and every late callback.
    _released: ReleasedSemaphore,
}
type Completion<T> = Result<Option<T>, Error>;
struct Request<C: Converter> {
    output: Mutex<Option<Completion<C::Image>>>,
    converter: Arc<C>,
    trace: Arc<Trace>,
    semaphore: Semaphore,
    // Last: observed only after output, converter and other owned fields drop.
    _released: ReleasedRequest,
}
impl<C: Converter> Request<C> {
    fn finish(&self, result: Result<Option<C::Image>, Error>) {
        if !self.trace.once(&self.trace.completed) {
            return;
        }
        match self.output.lock() {
            Ok(mut output) => *output = Some(result),
            Err(_) => self.trace.fault(-2),
        }
        if self.trace.timed_out.load(Ordering::Acquire) {
            self.trace.late_completed.store(true, Ordering::Release);
        }
        // Scalar fault + original native signal remains available even if the
        // result mutex is poisoned. No formatting/allocation is required here.
        self.trace.signals.store(1, Ordering::Release);
        let result = self.semaphore.value.signal();
        self.trace.signal_result.store(result, Ordering::Release);
    }
    fn wait(&self) -> Result<Option<C::Image>, Error> {
        // Exact source dispatch_time(NOW, 5*NSEC_PER_SEC) and semaphore wait,
        // AFTER content submission. Keep the native kernel outcome/counting
        // semantics instead of emulating them with a different wait primitive.
        let result = self
            .semaphore
            .value
            .wait(DispatchTime::NOW.time(5_000_000_000));
        self.trace.wait_result.store(result, Ordering::Release);
        if result != 0 {
            self.trace.timed_out.store(true, Ordering::Release);
            if self.trace.completed.load(Ordering::Acquire) != 0 {
                self.trace.late_completed.store(true, Ordering::Release);
            }
            return Ok(None);
        }
        let fault = self.trace.fault.load(Ordering::Acquire);
        if fault != 0 {
            return Err(Error::new(
                "ScreenCaptureKit callback fault",
                i64::from(fault),
            ));
        }
        self.output
            .lock()
            .map_err(|_| Error::new("ScreenCaptureKit result poisoned", -2))?
            .take()
            .ok_or(Error::new("ScreenCaptureKit missing completion", -3))?
    }
}
struct ReleasedInputs(Arc<Trace>);
impl Drop for ReleasedInputs {
    fn drop(&mut self) {
        self.0.thread(&self.0.block_release_thread);
        self.0.block_inputs_released.store(1, Ordering::Release);
    }
}
type CreatedInputs = (Retained<SCContentFilter>, Retained<SCStreamConfiguration>);
struct Inputs {
    // Model the original +1 create owners separately from the copied block's
    // retained references. Release create owners before signalling completion.
    initial: Mutex<Option<CreatedInputs>>,
    filter: Retained<SCContentFilter>,
    config: Retained<SCStreamConfiguration>,
    trace: Arc<Trace>,
    _released: ReleasedInputs,
}
impl Inputs {
    fn release_initial(&self) {
        let value = self
            .initial
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        drop(value);
        self.trace.thread(&self.trace.initial_release_thread);
        self.trace
            .initial_inputs_released
            .store(1, Ordering::Release);
    }
    fn keep_alive(&self) {
        let _ = (&self.filter, &self.config);
    }
}
fn dimension(value: f64) -> Result<usize, Error> {
    if !value.is_finite() || value < 0.0 || value > i32::MAX as f64 {
        return Err(Error::new("ScreenCaptureKit config dimension", -22));
    }
    // Original CGFloat->NSUInteger truncation for its defined positive domain.
    Ok(value as usize)
}
fn content<C: Converter>(
    wid: u32,
    state: &Arc<Request<C>>,
    raw: *mut SCShareableContent,
    error: *mut NSError,
) -> Result<(), Error> {
    state.trace.thread(&state.trace.content_thread);
    if !state.trace.once(&state.trace.content_calls) {
        return Err(Error::new("repeated native content callback", -3));
    }
    // SAFETY: Nullable callback borrows; no pointer escapes this invocation.
    if let Some(error) = unsafe { error.as_ref() } {
        state
            .trace
            .native_error
            .store(error.code(), Ordering::Release);
        state.trace.source_failure.store(1, Ordering::Release);
        state.finish(Ok(None));
        return Ok(());
    }
    let Some(content) = (unsafe { raw.as_ref() }) else {
        state.trace.source_failure.store(1, Ordering::Release);
        state.finish(Ok(None));
        return Ok(());
    };
    let (filter, width, height) = if wid != 0 {
        // SAFETY: Retained native content owns the enumerated windows. Preserve
        // source first exact match, not a new selection/visibility policy.
        let windows = unsafe { content.windows() };
        let mut selected = None;
        for index in 0..windows.count() {
            let window = windows.objectAtIndex(index);
            if unsafe { window.windowID() } == wid {
                selected = Some(window);
                break;
            }
        }
        let Some(window) = selected else {
            state.trace.source_failure.store(2, Ordering::Release);
            state.finish(Ok(None));
            return Ok(());
        };
        // SAFETY: This callback owns the live selected object and creates its
        // frozen capture inputs here, as in the original content callback.
        let frame = unsafe { window.frame() };
        (
            unsafe {
                SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &window)
            },
            dimension(frame.size.width)?,
            dimension(frame.size.height)?,
        )
    } else {
        // SAFETY: Same live content; source takes displays[0] without a primary
        // monitor identity rule. Do not infer another target for empty results.
        let displays = unsafe { content.displays() };
        if displays.count() == 0 {
            state.trace.source_failure.store(2, Ordering::Release);
            state.finish(Ok(None));
            return Ok(());
        }
        let display = displays.objectAtIndex(0);
        let excluded = NSArray::<SCWindow>::new();
        (
            unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &excluded,
                )
            },
            usize::try_from(unsafe { display.width() })
                .map_err(|_| Error::new("negative source display width", -22))?,
            usize::try_from(unsafe { display.height() })
                .map_err(|_| Error::new("negative source display height", -22))?,
        )
    };
    // SAFETY: Configuration is mutable only in this content callback, then
    // frozen through the API's screenshot completion ownership channel.
    let config = unsafe { SCStreamConfiguration::new() };
    unsafe {
        config.setWidth(width);
        config.setHeight(height);
        config.setPixelFormat(kCVPixelFormatType_32BGRA);
        config.setColorSpaceName(kCGColorSpaceSRGB);
        config.setShowsCursor(false);
    }
    if AnyClass::get(c"SCScreenshotManager").is_none() {
        state.trace.source_failure.store(4, Ordering::Release);
        drop(filter);
        drop(config);
        state.finish(Ok(None));
        return Ok(());
    }
    let inputs = Inputs {
        filter: filter.clone(),
        config: config.clone(),
        initial: Mutex::new(Some((filter, config))),
        trace: state.trace.clone(),
        _released: ReleasedInputs(state.trace.clone()),
    };
    // These point to native objects, not the movable Rust Retained wrappers.
    // The local RcBlock below owns both copied references throughout submission,
    // including a possible synchronous callback that releases create owners.
    let filter_pointer = NonNull::from(inputs.filter.as_ref());
    let config_pointer = NonNull::from(inputs.config.as_ref());
    let sample_state = state.clone();
    let sample = RcBlock::new(move |raw: *mut CMSampleBuffer, error: *mut NSError| {
        inputs.keep_alive();
        let result = catch_unwind(AssertUnwindSafe(|| {
            sample_state.trace.thread(&sample_state.trace.sample_thread);
            if !sample_state.trace.once(&sample_state.trace.sample_calls) {
                return Err(Error::new("repeated native sample callback", -3));
            }
            // SAFETY: Only this synchronous completion borrows these pointers.
            if let Some(error) = unsafe { error.as_ref() } {
                sample_state
                    .trace
                    .native_error
                    .store(error.code(), Ordering::Release);
                sample_state
                    .trace
                    .source_failure
                    .store(3, Ordering::Release);
                return Ok(None);
            }
            let Some(sample) = (unsafe { raw.as_ref() }) else {
                sample_state
                    .trace
                    .source_failure
                    .store(3, Ordering::Release);
                return Ok(None);
            };
            // Do not skip conversion after caller timeout. U still performs
            // callback work and retains the late result until its block dies.
            pixel_buffer::with_sample(sample, &sample_state.trace.pixels, |sample| match sample {
                Some(sample) => sample_state.converter.convert(sample),
                None => sample_state.converter.empty(),
            })
            .map(Some)
        }))
        .unwrap_or(Err(Error::new("ScreenCaptureKit sample panic", -1)));
        inputs.release_initial();
        sample_state.finish(result);
    });
    // SAFETY: The source API copies this escaping block. Its mutable Rust state
    // uses owned Send/Sync Arc+Mutex/atomics, never caller stack borrows. The only
    // native payload is the frozen filter/config create+block ownership pair,
    // accessed/released in the framework's documented completion channel, as in
    // the original MRC source. No AppKit object or general native Send/Sync impl
    // is introduced. Native pointer borrows stay valid through the local block.
    unsafe {
        SCScreenshotManager::captureSampleBufferWithFilter_configuration_completionHandler(
            filter_pointer.as_ref(),
            config_pointer.as_ref(),
            Some(&sample),
        );
    }
    Ok(())
}
pub(crate) struct Capture {
    window_id: u32,
}
impl Capture {
    pub const fn new(window_id: u32) -> Self {
        Self { window_id }
    }
    pub fn capture_with<C: Converter>(
        &self,
        converter: Arc<C>,
        trace: Arc<Trace>,
    ) -> Result<Option<C::Image>, Error> {
        trace.thread(&trace.caller);
        if trace.source_semaphore_retained.load(Ordering::Acquire) != 0
            || trace.content_calls.load(Ordering::Acquire) != 0
        {
            return Err(Error::new("ScreenCaptureKit trace reused", -22));
        }
        let semaphore = catch_unwind(|| DispatchSemaphore::new(0))
            .map_err(|_| Error::new("native dispatch semaphore allocation", -12))?;
        let block_owner = semaphore.clone();
        *trace
            .source_semaphore
            .lock()
            .map_err(|_| Error::new("semaphore owner poisoned", -2))? = Some(semaphore);
        trace.source_semaphore_retained.store(1, Ordering::Release);
        let state = Arc::try_new(Request {
            output: Mutex::new(None),
            converter,
            trace: trace.clone(),
            semaphore: Semaphore {
                value: block_owner,
                _released: ReleasedSemaphore(trace.clone()),
            },
            _released: ReleasedRequest(trace),
        })
        .map_err(|_| Error::allocation())?;
        let callback_state = state.clone();
        let window_id = self.window_id;
        let callback = RcBlock::new(move |raw: *mut SCShareableContent, error: *mut NSError| {
            let result = catch_unwind(AssertUnwindSafe(|| {
                content(window_id, &callback_state, raw, error)
            }))
            .unwrap_or(Err(Error::new("ScreenCaptureKit content panic", -1)));
            if let Err(error) = result {
                callback_state.finish(Err(error));
            }
        });
        // SAFETY: Framework copies the block, which owns all request state and
        // copied ID, including through timeout/late completion. This method does
        // not create a worker or alter the host's run loop/thread context.
        unsafe {
            SCShareableContent::getShareableContentWithCompletionHandler(&callback);
        }
        state.wait()
    }
}

#[cfg(test)]
#[path = "screen_capture_kit_tests.rs"]
mod tests;
