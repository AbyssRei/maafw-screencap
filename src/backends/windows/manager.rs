// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Fixed Win32ControlUnitMgr screenshot path only; input remains excluded.
use std::ffi::c_void;

#[path = "manager_native.rs"]
mod system;
pub(crate) use system::{Backend, Factory as NativeFactory, Platform as NativePlatform};
pub(crate) type NativeManager<I> = Manager<Backend<I>>;
impl<I> NativeManager<I> {
    pub fn connect_native<C>(
        &mut self,
        converter: &mut C,
    ) -> Connected<system::StartError, system::Failure<C::Error>, system::Evidence<C::Error>>
    where
        C: super::print_window::PrintConverter<Image = I>
            + super::frame_pool::Converter<Image = I>
            + super::desktop_dup_window::Converter<Image = I>,
    {
        self.connect(&NativePlatform, &NativeFactory.for_image(), converter)
    }
    pub fn connected_native(&self) -> bool {
        self.connected(&NativePlatform)
    }
    pub fn inactive_native(&mut self) {
        if let Some((_, unit)) = self.selected.as_mut() {
            unit.deactivate();
        }
    }
}

/// Fixed upstream values/order, private until the complete U/P migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum Method {
    Gdi = 1,
    FramePool = 2,
    DesktopDup = 3,
    DesktopDupWindow = 4,
    PrintWindow = 5,
    ScreenDc = 6,
}
impl Method {
    const ALL: [Self; 6] = [
        Self::Gdi,
        Self::FramePool,
        Self::DesktopDup,
        Self::DesktopDupWindow,
        Self::PrintWindow,
        Self::ScreenDc,
    ];
    fn index(self) -> usize {
        self as usize - 1
    }
    fn flag(self) -> u64 {
        1 << self.index()
    }
}

/// Some(empty Mat) is a successful source result, distinct from nullopt.
pub(crate) enum Image<I> {
    Empty,
    Pixels(I),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailureClass {
    NoImage,
    Exception,
    Safety,
}
pub(crate) enum Sample<I, F> {
    Image(Image<I>),
    Failure(FailureClass, F),
}
pub(crate) struct Call<I, F, E> {
    pub sample: Sample<I, F>,
    pub evidence: E,
}
pub(crate) trait Unit<C> {
    type Image;
    type Failure;
    type Evidence;
    fn capture(&mut self, converter: &mut C) -> Call<Self::Image, Self::Failure, Self::Evidence>;
    fn inactive(&mut self);
}
pub(crate) trait Factory {
    type Unit;
    type Error;
    fn create(&self, method: Method, window: *mut c_void) -> Result<Self::Unit, Self::Error>;
}
pub(crate) trait Platform {
    type Stamp;
    // U connect intentionally leaves the new caller DPI context in place.
    // P3's host-context contract is not introduced here.
    fn set_per_monitor_v2(&self) -> Result<isize, u32>;
    fn valid(&self, window: *mut c_void) -> bool;
    fn now(&self) -> Self::Stamp;
    fn elapsed_millis(&self, start: Self::Stamp) -> u128;
}

pub(crate) enum Outcome<F> {
    Pixels,
    Empty,
    Failure(FailureClass, F),
}
impl<F> Outcome<F> {
    fn succeeds(&self) -> bool {
        matches!(self, Self::Pixels | Self::Empty)
    }
    fn escapes(&self) -> bool {
        matches!(
            self,
            Self::Failure(FailureClass::Exception | FailureClass::Safety, _)
        )
    }
}
pub(crate) struct Observation<F, E> {
    pub outcome: Outcome<F>,
    pub evidence: E,
}
pub(crate) struct Trial<F, E> {
    pub warmup: Option<Observation<F, E>>,
    pub measured: Option<Observation<F, E>>,
    pub millis: Option<u128>,
}
impl<F, E> Default for Trial<F, E> {
    fn default() -> Self {
        Self {
            warmup: None,
            measured: None,
            millis: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Warmup,
    Measured,
}
pub(crate) enum ConnectFailure<S> {
    InvalidWindow,
    NoMethods,
    NoSuccessfulMethod,
    Construction(Method, S),
    Escaped(Method, Stage),
}
pub(crate) struct Connected<S, F, E> {
    pub failure: Option<ConnectFailure<S>>,
    pub dpi: Result<isize, u32>,
    pub selected: Option<Method>,
    pub trials: [Trial<F, E>; 6],
}
pub(crate) enum Publication<F, E> {
    NoUnit,
    Called(Observation<F, E>),
}

// All selected units are constructed before measurement. Explicit drop order
// follows the native MSVC source-body container observation, not Rust HashMap.
// A future toolchain must pass that observation before parity can be accepted.
struct Candidates<U>([Option<U>; 6]);
impl<U> Drop for Candidates<U> {
    fn drop(&mut self) {
        for slot in &mut self.0 {
            drop(slot.take());
        }
    }
}

pub(crate) struct Manager<U> {
    window: *mut c_void,
    flags: u64,
    connected: bool,
    selected: Option<(Method, U)>,
}
impl<U> Manager<U> {
    pub fn new(window: *mut c_void, flags: u64) -> Self {
        Self {
            window,
            flags,
            connected: false,
            selected: None,
        }
    }
    pub fn connected<P: Platform>(&self, platform: &P) -> bool {
        self.connected && (self.window.is_null() || platform.valid(self.window))
    }
    pub fn selected(&self) -> Option<Method> {
        self.selected.as_ref().map(|(method, _)| *method)
    }
    pub fn connect<C, P: Platform, F: Factory<Unit = U>>(
        &mut self,
        platform: &P,
        factory: &F,
        converter: &mut C,
    ) -> Connected<F::Error, U::Failure, U::Evidence>
    where
        U: Unit<C>,
    {
        self.connected = false;
        drop(self.selected.take());
        let mut report = Connected {
            failure: None,
            dpi: platform.set_per_monitor_v2(),
            selected: None,
            trials: std::array::from_fn(|_| Trial::default()),
        };
        if !self.window.is_null() && !platform.valid(self.window) {
            report.failure = Some(ConnectFailure::InvalidWindow);
            return report;
        }
        if self.flags == 0 {
            self.connected = true;
            return report;
        }
        let mut candidates = Candidates(std::array::from_fn(|_| None));
        for method in Method::ALL {
            if self.flags & method.flag() != 0 {
                match factory.create(method, self.window) {
                    Ok(unit) => candidates.0[method.index()] = Some(unit),
                    Err(error) => {
                        report.failure = Some(ConnectFailure::Construction(method, error));
                        return report;
                    }
                }
            }
        }
        if candidates.0.iter().all(Option::is_none) {
            report.failure = Some(ConnectFailure::NoMethods);
            return report;
        }
        let mut fastest = None;
        let mut cost = i64::MAX as u128; // source milliseconds::max()
        for method in Method::ALL {
            let Some(unit) = candidates.0[method.index()].as_mut() else {
                continue;
            };
            let trial = &mut report.trials[method.index()];
            let warmup = observe(unit.capture(converter));
            let escaped = warmup.outcome.escapes();
            trial.warmup = Some(warmup);
            if escaped {
                report.failure = Some(ConnectFailure::Escaped(method, Stage::Warmup));
                return report;
            }
            // Warmup temporary image has already been destroyed. The measured
            // temporary also dies before source check()/duration_since().
            let start = platform.now();
            let measured = observe(unit.capture(converter));
            let success = measured.outcome.succeeds();
            let escaped = measured.outcome.escapes();
            trial.measured = Some(measured);
            if escaped {
                report.failure = Some(ConnectFailure::Escaped(method, Stage::Measured));
                return report;
            }
            if !success {
                continue;
            }
            let elapsed = platform.elapsed_millis(start);
            trial.millis = Some(elapsed);
            if elapsed < cost {
                cost = elapsed;
                fastest = Some(method);
            }
        }
        let Some(fastest) = fastest else {
            report.failure = Some(ConnectFailure::NoSuccessfulMethod);
            return report;
        };
        self.selected = candidates.0[fastest.index()]
            .take()
            .map(|unit| (fastest, unit));
        report.selected = Some(fastest);
        // Source assigns the winner, then destroys the other candidates before
        // connect publishes true. Candidate/helper failures stay in evidence.
        drop(candidates);
        self.connected = true;
        report
    }
    pub fn screencap<C>(
        &mut self,
        converter: &mut C,
        output: &mut Image<U::Image>,
    ) -> Publication<U::Failure, U::Evidence>
    where
        U: Unit<C>,
    {
        let Some((_, unit)) = self.selected.as_mut() else {
            return Publication::NoUnit;
        };
        // Source does not guard this call with connected()/IsWindow(). A failed
        // optional or escaping call leaves the previous caller output intact.
        let Call { sample, evidence } = unit.capture(converter);
        let outcome = match sample {
            Sample::Image(image) => {
                let outcome = match &image {
                    Image::Empty => Outcome::Empty,
                    Image::Pixels(_) => Outcome::Pixels,
                };
                *output = image;
                outcome
            }
            Sample::Failure(class, error) => Outcome::Failure(class, error),
        };
        Publication::Called(Observation { outcome, evidence })
    }
    pub fn inactive<C>(&mut self)
    where
        U: Unit<C>,
    {
        if let Some((_, unit)) = self.selected.as_mut() {
            unit.inactive();
        }
    }
}
fn observe<I, F, E>(call: Call<I, F, E>) -> Observation<F, E> {
    let outcome = match call.sample {
        Sample::Image(Image::Pixels(image)) => {
            drop(image);
            Outcome::Pixels
        }
        Sample::Image(Image::Empty) => Outcome::Empty,
        Sample::Failure(class, error) => Outcome::Failure(class, error),
    };
    Observation {
        outcome,
        evidence: call.evidence,
    }
}

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "manager_tests.rs"]
mod tests;
