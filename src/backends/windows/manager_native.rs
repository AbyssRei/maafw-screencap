// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{Call, FailureClass, Image, Method, Sample};
use crate::backends::windows::{
    desktop_dup, desktop_dup_window, frame_pool, frame_pool_pseudo, gdi, gdi_types, print_window,
    print_window_pseudo, screen_dc,
};
use std::{ffi::c_void, time::Instant};

pub(crate) enum Backend<I> {
    Gdi(gdi::GdiCapture),
    FramePool(frame_pool_pseudo::FramePoolWithPseudoMinimize<I>),
    DesktopDup(desktop_dup::DesktopDupCapture),
    DesktopDupWindow(desktop_dup_window::DesktopDupWindowCapture),
    PrintWindow(print_window_pseudo::PrintWindowWithPseudoMinimize),
    ScreenDc(screen_dc::ScreenDcCapture),
}
impl<I> Backend<I> {
    pub(super) fn deactivate(&mut self) {
        match self {
            Self::PrintWindow(unit) => unit.inactive(),
            Self::FramePool(unit) => unit.inactive(),
            _ => {}
        }
    }
}
pub(crate) enum StartError {
    FramePool(frame_pool_pseudo::StartError),
    PrintWindow(print_window_pseudo::StartError),
}
pub(crate) struct Factory;
impl<I> super::Factory for FactoryFor<I> {
    type Unit = Backend<I>;
    type Error = StartError;
    fn create(&self, method: Method, window: *mut c_void) -> Result<Backend<I>, StartError> {
        Ok(match method {
            Method::Gdi => Backend::Gdi(gdi::GdiCapture::new(window)),
            Method::FramePool => Backend::FramePool(
                frame_pool_pseudo::FramePoolWithPseudoMinimize::new(window)
                    .map_err(StartError::FramePool)?,
            ),
            Method::DesktopDup => Backend::DesktopDup(desktop_dup::DesktopDupCapture::new(window)),
            Method::DesktopDupWindow => {
                Backend::DesktopDupWindow(desktop_dup_window::DesktopDupWindowCapture::new(window))
            }
            Method::PrintWindow => Backend::PrintWindow(
                print_window_pseudo::PrintWindowWithPseudoMinimize::new(window)
                    .map_err(StartError::PrintWindow)?,
            ),
            Method::ScreenDc => Backend::ScreenDc(screen_dc::ScreenDcCapture::new(window)),
        })
    }
}
pub(crate) struct FactoryFor<I>(std::marker::PhantomData<fn() -> I>);
impl Factory {
    pub fn for_image<I>(&self) -> FactoryFor<I> {
        FactoryFor(std::marker::PhantomData)
    }
}

pub(crate) enum Failure<E> {
    Gdi(gdi_types::CaptureError<E>),
    FramePool(frame_pool::Error<E>),
    Desktop(desktop_dup::Error<E>),
    DesktopWindow(desktop_dup_window::Error<E>),
}
pub(crate) enum Evidence<E> {
    Gdi(gdi_types::Cleanup),
    PrintWindow {
        main: gdi_types::Cleanup,
        popup: gdi_types::Cleanup,
        popup_failure: Option<gdi_types::CaptureError<E>>,
    },
    ScreenDc {
        coordination: Option<screen_dc::Coordination>,
        cleanup: gdi_types::Cleanup,
    },
    FramePool(frame_pool::Evidence),
    Desktop(desktop_dup::Evidence),
    DesktopWindow {
        capture: desktop_dup::Evidence,
        coordination: Option<desktop_dup_window::Coordination>,
        target_copy_bytes: usize,
    },
}
fn gdi_class<E>(error: &gdi_types::CaptureError<E>) -> FailureClass {
    use gdi_types::CaptureError::*;
    match error {
        Conversion(_) | Allocation(_) => FailureClass::Exception,
        SizeOverflow | UnsupportedBitmap | IncompleteRead { .. } | IncompleteScanlines { .. } => {
            FailureClass::Safety
        }
        _ => FailureClass::NoImage,
    }
}
fn desktop_class<E>(error: &desktop_dup::Error<E>) -> FailureClass {
    match error {
        desktop_dup::Error::Conversion(_) => FailureClass::Exception,
        desktop_dup::Error::Capture(desktop_dup::CaptureError::Allocation(_)) => {
            FailureClass::Exception
        }
        desktop_dup::Error::Capture(
            desktop_dup::CaptureError::Overflow | desktop_dup::CaptureError::InvalidLayout,
        ) => FailureClass::Safety,
        _ => FailureClass::NoImage,
    }
}
impl<C, I> super::Unit<C> for Backend<I>
where
    C: print_window::PrintConverter<Image = I>
        + frame_pool::Converter<Image = I>
        + desktop_dup_window::Converter<Image = I>,
{
    type Image = I;
    type Failure = Failure<C::Error>;
    type Evidence = Evidence<C::Error>;
    fn capture(&mut self, converter: &mut C) -> Call<I, Self::Failure, Self::Evidence> {
        let (sample, evidence) = match self {
            Self::Gdi(unit) => {
                let attempt = unit.capture(converter);
                (
                    attempt
                        .image
                        .map_err(|error| (gdi_class(&error), Failure::Gdi(error))),
                    Evidence::Gdi(attempt.cleanup),
                )
            }
            Self::PrintWindow(unit) => {
                let attempt = unit.capture(converter);
                (
                    attempt
                        .image
                        .map_err(|error| (gdi_class(&error), Failure::Gdi(error))),
                    Evidence::PrintWindow {
                        main: attempt.main_cleanup,
                        popup: attempt.popup_cleanup,
                        popup_failure: attempt.popup_failure,
                    },
                )
            }
            Self::ScreenDc(unit) => {
                let attempt = unit.capture(converter);
                (
                    attempt
                        .image
                        .map_err(|error| (gdi_class(&error), Failure::Gdi(error))),
                    Evidence::ScreenDc {
                        coordination: attempt.coordination,
                        cleanup: attempt.cleanup,
                    },
                )
            }
            Self::FramePool(unit) => {
                let attempt = unit.capture(converter);
                let image = match attempt.image {
                    // Source Some(empty Mat) succeeds in manager truth tests.
                    Err(frame_pool::Error::EmptySample) => {
                        return Call {
                            sample: Sample::Image(Image::Empty),
                            evidence: Evidence::FramePool(attempt.evidence),
                        };
                    }
                    result => result.map_err(|error| {
                        use frame_pool::Error::*;
                        let class = match &error {
                            WinRtException(..) | Conversion(_) | Allocation(_)
                            | OwnerAllocation(_) => FailureClass::Exception,
                            InvalidLayout | Overflow => FailureClass::Safety,
                            Native("frame.ContentSize safety check", _) => FailureClass::Safety,
                            _ => FailureClass::NoImage,
                        };
                        (class, Failure::FramePool(error))
                    }),
                };
                (image, Evidence::FramePool(attempt.evidence))
            }
            Self::DesktopDup(unit) => {
                let attempt = unit.capture(converter);
                (
                    attempt
                        .image
                        .map_err(|error| (desktop_class(&error), Failure::Desktop(error))),
                    Evidence::Desktop(attempt.evidence),
                )
            }
            Self::DesktopDupWindow(unit) => {
                let attempt = unit.capture(converter);
                let image = attempt.image.map_err(|error| {
                    use desktop_dup_window::Error::*;
                    let class = match &error {
                        Desktop(error) => desktop_class(error),
                        Conversion(_) => FailureClass::Exception,
                        Overflow => FailureClass::Safety,
                        _ => FailureClass::NoImage,
                    };
                    (class, Failure::DesktopWindow(error))
                });
                (
                    image,
                    Evidence::DesktopWindow {
                        capture: attempt.evidence,
                        coordination: attempt.coordination,
                        target_copy_bytes: attempt.target_copy_bytes,
                    },
                )
            }
        };
        Call {
            sample: match sample {
                Ok(image) => Sample::Image(Image::Pixels(image)),
                Err((class, error)) => Sample::Failure(class, error),
            },
            evidence,
        }
    }
    fn inactive(&mut self) {
        self.deactivate();
    }
}
pub(crate) struct Platform;
impl super::Platform for Platform {
    type Stamp = Instant;
    fn set_per_monitor_v2(&self) -> Result<isize, u32> {
        crate::platform::windows::manager::set_per_monitor_v2()
    }
    fn valid(&self, window: *mut c_void) -> bool {
        crate::platform::windows::manager::valid(window)
    }
    fn now(&self) -> Instant {
        Instant::now()
    }
    fn elapsed_millis(&self, start: Instant) -> u128 {
        start.elapsed().as_millis()
    }
}
