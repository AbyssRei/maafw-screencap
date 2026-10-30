// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Typed host initialization, composed over the SAME capture factory. No input
//! action methods, native interface casts, authorization or public SDK contract.
use super::{Config, Factory};

pub(in crate::backends::linux) trait InputOwner {
    type Error;
    fn init(&mut self) -> Result<bool, Self::Error>;
    fn inactive(&mut self);
    fn features(&self) -> u64;
}

pub(in crate::backends::linux) trait InputProvider<C, E> {
    type Input: InputOwner<Error = E>;
    /// Returned input owns its real native resources/callback state. It may
    /// outlive this provider. Retain any shared Wayland client actually used by
    /// the owner; a copied raw pointer is not a lifetime proof.
    fn create(&mut self, config: &Config, client: Option<&C>) -> Result<Self::Input, E>;
}

pub(in crate::backends::linux) struct WithInput<F, H> {
    pub(super) capture: F,
    host: H,
}
impl<F, H> WithInput<F, H> {
    pub fn new(capture: F, host: H) -> Self {
        Self { capture, host }
    }
}
impl<F: Factory, H: InputProvider<F::Client, F::Error>> Factory for WithInput<F, H> {
    type Client = F::Client;
    type Unit = F::Unit;
    type Input = H::Input;
    type Error = F::Error;
    fn create_client(&mut self) -> Result<Self::Client, Self::Error> {
        self.capture.create_client()
    }
    fn connect_client(
        &mut self,
        client: &mut Self::Client,
        socket: &[u8],
    ) -> Result<bool, Self::Error> {
        self.capture.connect_client(client, socket)
    }
    fn create_capture(
        &mut self,
        config: &Config,
        client: Option<&Self::Client>,
    ) -> Result<Self::Unit, Self::Error> {
        self.capture.create_capture(config, client)
    }
    fn create_input(
        &mut self,
        config: &Config,
        client: Option<&Self::Client>,
    ) -> Result<Self::Input, Self::Error> {
        self.host.create(config, client)
    }
    fn init_capture(&mut self, unit: &mut Self::Unit) -> Result<bool, Self::Error> {
        self.capture.init_capture(unit)
    }
    fn init_input(&mut self, input: &mut Self::Input) -> Result<bool, Self::Error> {
        input.init()
    }
    fn input_features(input: &Self::Input) -> u64 {
        input.features()
    }
    fn inactive_capture(unit: &mut Self::Unit) {
        F::inactive_capture(unit);
    }
    fn inactive_input(input: &mut Self::Input) {
        input.inactive();
    }
}
