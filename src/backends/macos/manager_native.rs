// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::{InputFeatures, Manager, NativeManager};
use crate::{
    backends::macos::screen_capture_kit::{Capture, Converter, Trace},
    platform::macos::Error,
};
use std::sync::Arc;

impl NativeManager {
    pub fn connect_native(&mut self) -> Result<bool, Error> {
        // Match __builtin_available(macOS 14.0, *) through the maintained
        // binding's actual runtime-version check, not class-name presence.
        self.connect_with(objc2::available!(macos = 14.0), |id| {
            Arc::try_new(Capture::new(id)).map_err(|_| Error::allocation())
        })
    }
}
impl<I: InputFeatures> Manager<Arc<Capture>, I> {
    pub fn connect_native_with_host_input(
        &mut self,
        create_input: impl FnOnce(u32, u64) -> Result<I, Error>,
    ) -> Result<bool, Error> {
        self.connect_with_host_input(
            objc2::available!(macos = 14.0),
            |id| Arc::try_new(Capture::new(id)).map_err(|_| Error::allocation()),
            create_input,
        )
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
