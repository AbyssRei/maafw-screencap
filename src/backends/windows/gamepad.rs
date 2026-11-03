// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Screenshot-relevant outer Gamepad ownership over the SAME capture delegate.
//! Host input initialization/lifetime is opaque; no input actions or SDK ABI.
use super::delegate::{Boundary, CaptureDelegate, Error as CaptureError};
use std::{collections::TryReserveError, fmt::Write};

pub(crate) trait InputOwner {
    type Error;
    fn connect(&mut self) -> Result<bool, Self::Error>;
    fn connected(&self) -> bool;
    /// Same caller, before capture teardown. Must not unwind: the original
    /// manager destructor is noexcept. This is not an input action interface.
    fn reset(&mut self);
}
pub(crate) trait Host {
    type Error;
    type Input: InputOwner<Error = Self::Error>;
    /// Create before replacing the previous owner. The returned owner retains
    /// its real resources independently of later provider calls.
    fn create(&mut self, gamepad_type: u64) -> Result<Self::Input, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum Error<H, C> {
    Host(H),
    Capture(CaptureError<C>),
}
pub(crate) struct GamepadCapture<B: Boundary, H: Host> {
    // After the destructor's reset, Rust drops declaration order: delegate
    // (no-op view, then real loader) BEFORE the input owner, as in F.
    delegate: CaptureDelegate<B>,
    input: Option<H::Input>,
    host: H,
    gamepad_type: u64,
    hwnd: usize,
    screencap_method: u64,
    connected: bool,
}
impl<B: Boundary, H: Host> GamepadCapture<B, H> {
    pub fn new(boundary: B, host: H, hwnd: usize, gamepad_type: u64, method: u64) -> Self {
        Self {
            delegate: CaptureDelegate::new(boundary, hwnd, method),
            input: None,
            host,
            gamepad_type,
            hwnd,
            screencap_method: method,
            connected: false,
        }
    }
    pub fn connect(&mut self) -> Result<bool, Error<H::Error, B::Error>> {
        self.connected = false;
        let next = self.host.create(self.gamepad_type).map_err(Error::Host)?;
        let previous = self.input.replace(next);
        drop(previous);
        if !self
            .input
            .as_mut()
            .unwrap()
            .connect()
            .map_err(Error::Host)?
        {
            return Ok(false);
        }
        // A false optional capture initialization is only a source warning.
        // Errors still propagate before connected=true. Never reset/retry the
        // existing delegate when host creation/connect fails above.
        self.delegate.initialize().map_err(Error::Capture)?;
        self.connected = true;
        Ok(true)
    }
    pub fn connected(&self) -> bool {
        self.connected && self.input.as_ref().is_some_and(InputOwner::connected)
    }
    pub fn request_uuid(&self, output: &mut String) -> Result<bool, TryReserveError> {
        // F uses decimal HWND and treats every non-Xbox360 value as DS4,
        // independent of connection/input/capture state. Commit the new owner
        // only after fallible allocation and complete formatting succeed.
        let mut value = String::new();
        value.try_reserve_exact(64)?;
        let kind = if self.gamepad_type == 0 {
            "xbox360"
        } else {
            "ds4"
        };
        write!(value, "gamepad_{kind}_hwnd_{}", self.hwnd).expect("String formatting cannot fail");
        *output = value;
        Ok(true)
    }
    pub const fn get_features(&self) -> u64 {
        // Original metadata only: the three declared input flags. This private
        // correspondence neither implements input actions nor exposes an SDK
        // capability. F returns them before connection as well.
        1 | (1 << 1) | (1 << 2)
    }
    pub fn get_info(&self) -> Result<String, TryReserveError> {
        let mut value = String::new();
        value.try_reserve_exact(256)?;
        let hwnd = u64::try_from(self.hwnd).expect("planned Windows pointer width fits u64");
        let kind = i64::from_ne_bytes(self.gamepad_type.to_ne_bytes());
        let method = i64::from_ne_bytes(self.screencap_method.to_ne_bytes());
        write!(value, "{{\"gamepad_type\":{kind},\"hwnd\":{hwnd},\"screencap_method\":{method},\"type\":\"gamepad\"}}")
            .expect("String formatting cannot fail");
        Ok(value)
    }
    pub fn capture(&mut self, output: &mut B::Image) -> Result<bool, CaptureError<B::Error>> {
        self.delegate.capture(output)
    }
    pub fn inactive(&mut self) -> Result<bool, CaptureError<B::Error>> {
        self.delegate.inactive()
    }
}
impl<B: Boundary, H: Host> Drop for GamepadCapture<B, H> {
    fn drop(&mut self) {
        if let Some(input) = &mut self.input {
            input.reset();
        }
    }
}

#[cfg(test)]
#[path = "gamepad_tests.rs"]
mod tests;
