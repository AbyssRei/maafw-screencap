// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed MaaFramework PrintWindowWithPseudoMinimizeScreencap.h.
use super::print_window::{Attempt, PrintConverter, PrintWindowCapture};
use crate::platform::windows::pseudo_minimize::Helper;
use std::{alloc::AllocError, ffi::c_void};

#[derive(Debug)]
pub(crate) enum StartError {
    Allocation(AllocError),
    Thread(std::io::Error),
}

pub(crate) struct PrintWindowWithPseudoMinimize {
    inner: PrintWindowCapture,
    helper: Helper,
}

impl PrintWindowWithPseudoMinimize {
    pub fn new(hwnd: *mut c_void) -> Result<Self, StartError> {
        let inner = PrintWindowCapture::new(hwnd);
        let mut helper = Helper::new(hwnd).map_err(StartError::Allocation)?;
        helper.start().map_err(StartError::Thread)?;
        Ok(Self { inner, helper })
    }

    pub fn capture<C: PrintConverter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        self.helper.ensure_not_minimized();
        self.inner.capture(converter)
    }

    pub fn inactive(&mut self) {
        if self.helper.is_pseudo_minimized() {
            // U's inactive restores styles without re-minimizing; subsequent
            // stop therefore sees pseudo=false and leaves the window visible.
            self.helper.revert_pseudo_minimize();
        }
    }
}

impl Drop for PrintWindowWithPseudoMinimize {
    fn drop(&mut self) {
        // C++ destroys helper before inner. Preserve this order explicitly even
        // if the currently trivial inner later acquires native ownership.
        self.helper.stop();
    }
}
