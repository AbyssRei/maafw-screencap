// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fixed Linux manager screenshot orchestration, restricted to Input=None.
//! Private U logic with selected native factories and legacy byte configuration.
//! Full parity remains P1; this module defines no public SDK.
#[path = "manager_config.rs"]
mod configuration;
pub(crate) use configuration::Config;
pub(crate) use configuration::Error as ConfigError;
#[path = "manager_compat.rs"]
mod compat;

#[cfg(all(
    target_os = "linux",
    any(feature = "linux-wlr", feature = "linux-pipewire")
))]
#[path = "manager_native.rs"]
pub(super) mod native;
#[cfg(all(
    target_os = "linux",
    any(feature = "linux-wlr", feature = "linux-pipewire")
))]
pub(super) type NativeManager = Manager<native::NativeFactory>;

// Original MaaDef.h selectors; these are not SDK ABI values.
pub(crate) const NONE: u64 = 0;
pub(crate) const WLR: u64 = 1;
pub(crate) const EXT_IMAGE: u64 = 2;
pub(crate) const PIPEWIRE: u64 = 4;

/// Boundaries mirror original construction and virtual init calls. Concrete
/// factories must retain the actual shared client in each constructed Wlr unit;
/// this manager does not replace native owners or implement capture itself.
pub(crate) trait Factory {
    type Client;
    type Unit;
    type Input;
    type Error;
    fn create_client(&mut self) -> Result<Self::Client, Self::Error>;
    fn connect_client(
        &mut self,
        client: &mut Self::Client,
        socket: &[u8],
    ) -> Result<bool, Self::Error>;
    fn create_capture(
        &mut self,
        config: &Config,
        client: Option<&Self::Client>,
    ) -> Result<Self::Unit, Self::Error>;
    fn create_none_input(&mut self) -> Result<Self::Input, Self::Error>;
    fn init_capture(&mut self, unit: &mut Self::Unit) -> Result<bool, Self::Error>;
    fn init_none_input(&mut self, input: &mut Self::Input) -> Result<bool, Self::Error>;
    fn inactive_capture(&mut self, unit: &mut Self::Unit);
    fn inactive_none_input(&mut self, input: &mut Self::Input);
}

pub(crate) struct Manager<F: Factory> {
    config: Config,
    connected: bool,
    capture: Option<F::Unit>,
    input: Option<F::Input>,
    client: Option<F::Client>,
}
impl<F: Factory> Manager<F> {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            connected: false,
            capture: None,
            input: None,
            client: None,
        }
    }
    pub fn from_config_json(input: Option<&[u8]>) -> Result<Self, ConfigError> {
        Ok(Self::new(Config::parse_c_string(input)?))
    }
    pub fn get_info(&self) -> Result<Vec<u8>, ConfigError> {
        self.config.info()
    }
    pub const fn get_features(&self) -> u64 {
        // Unconnected managers and the only implemented input unit (None)
        // expose no input features in the source.
        0
    }
    pub const fn connected(&self) -> bool {
        self.connected
    }
    pub fn connect(&mut self, factory: &mut F) -> Result<bool, F::Error> {
        self.connected = false;
        drop(self.capture.take());
        drop(self.input.take());
        // Only Input=None is in this screenshot migration. Other input values
        // remain queryable configuration; never silently perform input work.
        if self.config.input_method != NONE {
            return Ok(false);
        }
        if self.config.method == WLR && self.client.is_none() {
            self.client = Some(factory.create_client()?);
            if !factory.connect_client(self.client.as_mut().unwrap(), &self.config.wlr_socket)? {
                // Original keeps the failed client. A later manager connect
                // skips this call solely because that owner is non-null.
                return Ok(false);
            }
        }
        match self.config.method {
            NONE | WLR => {}
            PIPEWIRE => {
                // Preserve the original AND: a portal fd permits node zero.
                if self.config.pipewire_fd < 0 && self.config.pipewire_node == 0 {
                    return Ok(false);
                }
            }
            // U declares ExtImage but create_screencap does not implement it.
            _ => return Ok(false),
        }
        self.capture = Some(factory.create_capture(&self.config, self.client.as_ref())?);
        self.input = Some(factory.create_none_input()?);
        if !factory.init_capture(self.capture.as_mut().unwrap())? {
            return Ok(false);
        }
        if !factory.init_none_input(self.input.as_mut().unwrap())? {
            return Ok(false);
        }
        self.connected = true;
        Ok(true)
    }
    pub fn capture_with<I, E>(
        &mut self,
        output: &mut I,
        capture: impl FnOnce(&mut F::Unit, &mut I) -> Result<bool, E>,
    ) -> Result<bool, E> {
        let Some(unit) = &mut self.capture else {
            return Ok(false);
        };
        // Source forwards regardless of manager connected. It also does not
        // roll back a backend's output writes when that backend returns false.
        capture(unit, output)
    }
    pub fn inactive(&mut self, factory: &mut F) -> bool {
        if let Some(unit) = &mut self.capture {
            factory.inactive_capture(unit);
        }
        if let Some(input) = &mut self.input {
            factory.inactive_none_input(input);
        }
        true
    }
    pub fn request_uuid(&self, output: &mut Vec<u8>) -> Result<bool, ConfigError> {
        *output = self.config.uuid()?;
        Ok(true)
    }
}
impl<F: Factory> Drop for Manager<F> {
    fn drop(&mut self) {
        // Original member destruction releases the manager's client reference
        // first; each Wlr capture still retains its own shared client reference.
        drop(self.client.take());
        drop(self.capture.take());
        drop(self.input.take());
    }
}
#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
