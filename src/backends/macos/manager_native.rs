// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::Manager;
use crate::{
    backends::macos::screen_capture_kit::{Capture, Converter, Trace},
    platform::macos::Error,
};
use std::sync::Arc;

pub(crate) type NativeManager = Manager<Arc<Capture>>;
impl NativeManager {
    pub fn connect_native(&mut self) -> Result<bool, Error> {
        // Match __builtin_available(macOS 14.0, *) through the maintained
        // binding's actual runtime-version check, not class-name presence.
        self.connect_with(objc2::available!(macos = 14.0), |id| {
            Arc::try_new(Capture::new(id)).map_err(|_| Error::allocation())
        })
    }
    pub fn capture_native<C: Converter>(
        &self,
        output: &mut C::Image,
        converter: Arc<C>,
        trace: Arc<Trace>,
    ) -> Result<bool, Error> {
        self.capture_with(output, |unit| unit.capture_with(converter, trace))
    }
}
