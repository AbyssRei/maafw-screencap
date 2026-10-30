// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private screenshot-relevant legacy Wlr/KWin creation flow. Input values are
//! retained as metadata, never changed to None or used to perform input actions.
use super::{Config, ConfigError};
use std::{borrow::Cow, collections::TryReserveError};

/// Host authorization boundary, not a Portal implementation or SDK provider.
/// A helper is an opaque nonowning scalar; the host retains its real owner until
/// destroy. In particular, U's failed Open/exception paths do not destroy it.
/// Getters only forward the source FD/node; consumption requires the existing
/// independently verified unsafe native connection boundary.
pub(crate) trait Portal {
    type Helper: Copy;
    type Error;
    fn create(&mut self) -> Result<Self::Helper, Self::Error>;
    fn open(&mut self, helper: Self::Helper) -> Result<u8, Self::Error>;
    fn fd(&mut self, helper: Self::Helper) -> Result<i32, Self::Error>;
    fn node(&mut self, helper: Self::Helper) -> Result<u32, Self::Error>;
    fn destroy(&mut self, helper: Self::Helper) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum Error<E> {
    Configuration(ConfigError),
    Host(E),
}
fn c_string(bytes: &[u8]) -> Result<Vec<u8>, TryReserveError> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    let mut output = Vec::new();
    output.try_reserve_exact(end)?;
    output.extend_from_slice(&bytes[..end]);
    Ok(output)
}
impl Config {
    pub fn from_legacy_wlr(path: Option<&[u8]>, use_vk: u8) -> Result<Option<Self>, ConfigError> {
        // The original checks only the pointer, despite its "null or empty"
        // diagnostic. Empty and leading-NUL names still construct a manager.
        let Some(path) = path else { return Ok(None) };
        Ok(Some(Self {
            method: 1,
            input_method: 1,
            wlr_socket: c_string(path)?,
            use_win32_vk_code: use_vk != 0,
            ..Self::default()
        }))
    }
    pub fn from_legacy_kwin<P: Portal>(
        device: Option<&[u8]>,
        width: i32,
        height: i32,
        use_vk: u8,
        portal: &mut P,
    ) -> Result<Option<Self>, Error<P::Error>> {
        let Some(device) = device else {
            return Ok(None);
        };
        if matches!(device.first(), None | Some(0)) || width <= 0 || height <= 0 {
            return Ok(None);
        }
        let mut config = Self {
            method: 4,
            input_method: 2,
            uinput_width: width,
            uinput_height: height,
            uinput_path: Cow::Owned(
                c_string(device).map_err(|e| Error::Configuration(ConfigError::Allocation(e)))?,
            ),
            use_win32_vk_code: use_vk != 0,
            ..Self::default()
        };
        let helper = portal.create().map_err(Error::Host)?;
        if portal.open(helper).map_err(Error::Host)? == 0 {
            // Keep U's residual helper visible in the host ledger. No silent
            // cleanup/authorization retry is introduced before U/P freeze.
            return Ok(None);
        }
        config.pipewire_fd = portal.fd(helper).map_err(Error::Host)?;
        config.pipewire_node = portal.node(helper).map_err(Error::Host)?;
        portal.destroy(helper).map_err(Error::Host)?;
        // Input dimensions do not describe capture pixels. Invalid FD/node
        // combinations are stored here; native capture checks them later.
        Ok(Some(config))
    }
}

#[cfg(test)]
#[path = "manager_compat_tests.rs"]
mod tests;
