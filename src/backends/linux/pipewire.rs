// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private fixed-upstream consumer control flow. Native stream/setup, POD and
//! buffer processing are not implemented by this module yet.
use std::time::Duration;

/// Symbolic formats, not SPA numeric values or a public format/ABI contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Bgra,
    Bgrx,
    Rgba,
    Rgbx,
    Rgb,
    Bgr,
    Yuy2,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conversion {
    Bgra,
    Rgba,
    Rgb,
    Yuy2,
}
impl Format {
    fn conversion(self) -> Option<Option<Conversion>> {
        Some(match self {
            Self::Bgra | Self::Bgrx => Some(Conversion::Bgra),
            Self::Rgba | Self::Rgbx => Some(Conversion::Rgba),
            Self::Rgb => Some(Conversion::Rgb),
            Self::Bgr => None,
            Self::Yuy2 => Some(Conversion::Yuy2),
            Self::Unknown => return None,
        })
    }
}

/// Driver operations are performed on the owning consumer channel. A future
/// native driver must prove that lock excludes callbacks, timed_wait releases
/// and reacquires it, and callback storage is stable and separately owned. No
/// Rust reference into callback-mutated storage may survive timed_wait.
pub(crate) trait Driver {
    type Raw;
    fn connected(&self) -> bool;
    fn stream_and_loop_exist(&self) -> bool;
    fn lock(&mut self);
    fn unlock(&mut self);
    // Upstream ignores set_active's return value. Driver retains native evidence
    // separately; this method must not silently change the selection policy.
    fn set_active(&mut self, active: bool);
    fn set_wanted(&mut self, wanted: bool);
    fn wanted(&self) -> bool;
    fn latest_is_empty(&self) -> bool;
    // This clones only the raw owner, under the loop lock, as cv::Mat does.
    // Snapshot must not lend callback-owned or producer-owned storage.
    fn snapshot(&mut self) -> (Option<Self::Raw>, Format);
    fn now(&mut self) -> Duration;
    fn timed_wait(&mut self, seconds: i32) -> i32;
}
pub(crate) trait Converter<R> {
    type Image;
    type Error;
    fn convert(
        &mut self,
        raw: Option<R>,
        mode: Conversion,
    ) -> Result<Option<Self::Image>, Self::Error>;
    // BGR follows source move assignment, including successful empty Mat.
    fn reuse(&mut self, raw: Option<R>) -> Option<Self::Image>;
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error<E> {
    Disconnected,
    NoCachedFrame,
    ConsecutiveTimeouts,
    UnsupportedFormat,
    ClockOverflow,
    CounterOverflow,
    Conversion(E),
}
pub(crate) struct Attempt<I, E> {
    pub image: Result<Option<I>, Error<E>>,
    pub replayed: bool,
}

struct Locked<'a, D: Driver>(&'a mut D);
impl<D: Driver> Drop for Locked<'_, D> {
    fn drop(&mut self) {
        self.0.unlock();
    }
}

/// U intentionally retains relative whole-second waits, including rounding a
/// positive subsecond remainder up to one second. P3 absolute waits are separate.
fn wait_until<D: Driver>(
    driver: &mut D,
    seconds: u32,
    predicate: impl Fn(&D) -> bool,
) -> Result<bool, ()> {
    let deadline = driver
        .now()
        .checked_add(Duration::from_secs(u64::from(seconds)))
        .ok_or(())?;
    while !predicate(driver) {
        let remaining = deadline.saturating_sub(driver.now());
        if remaining.is_zero() {
            return Ok(predicate(driver));
        }
        let seconds = i32::try_from(remaining.as_secs().max(1)).map_err(|_| ())?;
        if driver.timed_wait(seconds) < 0 {
            return Ok(predicate(driver));
        }
    }
    Ok(true)
}

#[derive(Default)]
pub(crate) struct Consumer {
    timeouts: u32,
}
impl Consumer {
    // Native teardown must stop/drain callbacks and clear its own raw state
    // before reporting close complete. This only resets the consumer counter.
    pub fn reset_after_close(&mut self) {
        self.timeouts = 0;
    }
    pub fn inactive<D: Driver>(&mut self, driver: &mut D) {
        if !driver.stream_and_loop_exist() {
            return;
        }
        driver.lock();
        let lock = Locked(driver);
        lock.0.set_active(false);
        self.timeouts = 0;
    }
    pub fn capture<D: Driver, C: Converter<D::Raw>>(
        &mut self,
        driver: &mut D,
        converter: &mut C,
    ) -> Attempt<C::Image, C::Error> {
        let mut replayed = false;
        let image = self.capture_inner(driver, converter, &mut replayed);
        Attempt { image, replayed }
    }
    fn capture_inner<D: Driver, C: Converter<D::Raw>>(
        &mut self,
        driver: &mut D,
        converter: &mut C,
        replayed: &mut bool,
    ) -> Result<Option<C::Image>, Error<C::Error>> {
        if !driver.connected() {
            return Err(Error::Disconnected);
        }
        driver.lock();
        let lock = Locked(driver);
        lock.0.set_active(true);
        lock.0.set_wanted(true);
        let waited = wait_until(lock.0, 3, |d| !d.wanted());
        if waited != Ok(true) {
            lock.0.set_wanted(false);
        }
        let got_frame = waited.map_err(|()| Error::ClockOverflow)?;
        if !got_frame {
            if !lock.0.connected() {
                return Err(Error::Disconnected);
            }
            self.timeouts = self.timeouts.checked_add(1).ok_or(Error::CounterOverflow)?;
            if lock.0.latest_is_empty() {
                return Err(Error::NoCachedFrame);
            }
            if self.timeouts > 3 {
                return Err(Error::ConsecutiveTimeouts);
            }
            *replayed = true;
        } else {
            self.timeouts = 0;
        }
        let (raw, format) = lock.0.snapshot();
        drop(lock);
        match format.conversion().ok_or(Error::UnsupportedFormat)? {
            Some(mode) => converter.convert(raw, mode).map_err(Error::Conversion),
            None => Ok(converter.reuse(raw)),
        }
    }
}

#[cfg(test)]
#[path = "pipewire_tests.rs"]
mod tests;
