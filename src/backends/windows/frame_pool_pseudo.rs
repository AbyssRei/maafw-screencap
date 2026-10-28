// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed FramePoolWithPseudoMinimizeScreencap.h; private P1 path.
use super::frame_pool::{Attempt, Converter, FramePoolCapture};
use crate::platform::windows::pseudo_minimize::Helper;
use std::{alloc::AllocError, ffi::c_void};

#[derive(Debug)]
pub(crate) enum StartError {
    Allocation(AllocError),
    Thread(std::io::Error),
}

pub(crate) struct FramePoolWithPseudoMinimize<I> {
    inner: FramePoolCapture<I>,
    helper: Option<Helper>,
}

impl<I> FramePoolWithPseudoMinimize<I> {
    pub fn new(hwnd: *mut c_void) -> Result<Self, StartError> {
        let inner = FramePoolCapture::new(hwnd);
        let mut helper = Helper::new(hwnd).map_err(StartError::Allocation)?;
        helper.start().map_err(StartError::Thread)?;
        Ok(Self {
            inner,
            helper: Some(helper),
        })
    }

    pub fn capture<C: Converter<Image = I>>(&mut self, converter: &mut C) -> Attempt<I, C::Error> {
        self.helper.as_ref().unwrap().ensure_not_minimized();
        self.inner.capture(converter)
    }

    pub fn inactive(&mut self) {
        let helper = self.helper.as_ref().unwrap();
        if helper.is_pseudo_minimized() {
            helper.revert_pseudo_minimize();
        }
    }
}

impl<I> Drop for FramePoolWithPseudoMinimize<I> {
    fn drop(&mut self) {
        // C++ destroys helper_ before inner_. Fully drop the helper (including
        // stop/join/restoration and its shared owner) before Rust drops inner's
        // session/pool/device/cache. Field declaration order alone is different.
        drop(self.helper.take());
    }
}

#[cfg(test)]
#[path = "frame_pool_pseudo_tests.rs"]
mod tests;
