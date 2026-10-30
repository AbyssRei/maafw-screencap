// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Same native capture factory with an explicitly supplied typed host owner.
use super::super::{
    Manager,
    host::{InputProvider, WithInput},
};
use super::{Client, CloseWatch, Error, Evidence, NativeFactory, SourceFailure};

pub(in crate::backends::linux) type HostedFactory<H> = WithInput<NativeFactory, H>;
#[derive(Debug)]
pub(in crate::backends::linux) struct HostConnection {
    pub result: Result<bool, Error>,
    pub failure: Option<SourceFailure>,
    pub evidence: Evidence,
    pub close: Option<CloseWatch>,
}
impl<H: InputProvider<Client, Error>> Manager<HostedFactory<H>> {
    pub fn connect_host_native(&mut self, host: H) -> super::HostConnection {
        self.connect_host_factory(NativeFactory::default(), host)
    }
    /// # Safety
    /// Before EACH call the configured nonnegative fd is a valid uniquely
    /// offered socket whose owning handle is detached. No host construction,
    /// init, other owner or thread may close/reuse/retarget it during this call.
    /// A failed host construction can leave an UNCONSUMED offer: preserve its
    /// real ownership until native close and proven descriptor identity. An old
    /// consumed fd is not a new offer. This is the existing private U driver
    /// boundary, not an authorization API or safe SDK fd contract.
    pub unsafe fn connect_source_host_native(&mut self, host: H) -> super::HostConnection {
        // Permission is stack-local and never survives into a safe factory.
        let capture = NativeFactory {
            offered_fd: (self.config.pipewire_fd >= 0).then_some(self.config.pipewire_fd),
            failure: None,
        };
        self.connect_host_factory(capture, host)
    }
    fn connect_host_factory(&mut self, capture: NativeFactory, host: H) -> HostConnection {
        let mut factory = WithInput::new(capture, host);
        let result = self.connect(&mut factory);
        // Keep primary host/native failure and eventual native close evidence
        // independent, including a retained capture after host init throws.
        HostConnection {
            result,
            failure: factory.capture.failure,
            evidence: self.native_evidence(),
            close: self
                .capture
                .as_ref()
                .map(|backend| CloseWatch(backend.trace.clone())),
        }
    }
}
