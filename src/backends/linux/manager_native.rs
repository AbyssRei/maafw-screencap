// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native factories over the SAME private Wlr/PipeWire implementations. No
//! recovery, new scheduling, alternate capture backend or SDK FD API.
#[cfg(feature = "linux-pipewire")]
use super::super::pipewire::{self, RawFrame, native::Native};
#[cfg(feature = "linux-pipewire")]
use super::PIPEWIRE;
use super::{Config, Factory, NONE, WLR};
use std::sync::{Arc, Mutex};
#[cfg(feature = "linux-wlr")]
use {
    super::super::wlr,
    std::{os::unix::ffi::OsStrExt, path::Path},
};

#[cfg(feature = "linux-wlr")]
type Client = Arc<wlr::Client>;
#[cfg(not(feature = "linux-wlr"))]
type Client = ();

pub(in crate::backends::linux) enum Unit {
    None,
    #[cfg(feature = "linux-wlr")]
    Wlr(wlr::Capture),
    #[cfg(feature = "linux-pipewire")]
    PipeWire {
        driver: Native,
        consumer: pipewire::Consumer,
        node: u32,
    },
}
#[derive(Debug, Default)]
struct Trace {
    closed: Mutex<Option<Evidence>>,
}
#[derive(Clone, Debug)]
pub(in crate::backends::linux) struct CloseWatch(Arc<Trace>);
impl CloseWatch {
    pub fn take(&self) -> Option<Evidence> {
        self.0
            .closed
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
    }
}
pub(in crate::backends::linux) struct Backend {
    unit: Unit,
    trace: Arc<Trace>,
}
impl Drop for Backend {
    fn drop(&mut self) {
        // Stop/drain first, then retain scalar evidence without allocation or
        // a native owner. Embedded driver Drop is idempotent after this close.
        let evidence = match &mut self.unit {
            Unit::None => Evidence::None,
            #[cfg(feature = "linux-wlr")]
            Unit::Wlr(capture) => {
                capture.close();
                Evidence::Wlr(capture.evidence())
            }
            #[cfg(feature = "linux-pipewire")]
            Unit::PipeWire {
                driver, consumer, ..
            } => {
                driver.close();
                consumer.reset_after_close();
                Evidence::PipeWire(driver.evidence())
            }
        };
        *self.trace.closed.lock().unwrap_or_else(|p| p.into_inner()) = Some(evidence);
    }
}
pub(in crate::backends::linux) struct NoneInput;
#[derive(Debug)]
pub(in crate::backends::linux) enum Error {
    Allocation,
    BackendNotCompiled(u64),
    UnprovenFdOffer,
    InvalidPrivateOwner,
    HostInputRequired(u64),
    HostInput {
        operation: &'static str,
        code: i64,
    },
    #[cfg(feature = "linux-wlr")]
    Wayland(crate::platform::linux::wayland::Error),
}
#[derive(Debug)]
pub(in crate::backends::linux) enum SourceFailure {
    #[cfg(feature = "linux-wlr")]
    Wayland(crate::platform::linux::wayland::Error),
    #[cfg(feature = "linux-pipewire")]
    PipeWire(crate::platform::linux::pipewire::NativeError),
}
#[derive(Debug)]
pub(in crate::backends::linux) struct Connection {
    pub connected: bool,
    pub failure: Option<SourceFailure>,
    pub evidence: Evidence,
    pub close: Option<CloseWatch>,
}
#[derive(Default)]
pub(in crate::backends::linux) struct NativeFactory {
    // Only the unsafe immediate-connect facade below can create a factory with
    // a source-FD permission. It never returns/exposes that factory to safe code.
    offered_fd: Option<i32>,
    failure: Option<SourceFailure>,
}
impl Factory for NativeFactory {
    type Client = Client;
    type Unit = Box<Backend>;
    type Input = Arc<NoneInput>;
    type Error = Error;
    fn create_client(&mut self) -> Result<Client, Error> {
        #[cfg(feature = "linux-wlr")]
        {
            wlr::Client::new()
                .and_then(wlr::Client::into_shared)
                .map_err(Error::Wayland)
        }
        #[cfg(not(feature = "linux-wlr"))]
        {
            Err(Error::BackendNotCompiled(WLR))
        }
    }
    fn connect_client(&mut self, client: &mut Client, socket: &[u8]) -> Result<bool, Error> {
        #[cfg(feature = "linux-wlr")]
        {
            let client = Arc::get_mut(client).ok_or(Error::InvalidPrivateOwner)?;
            match client.connect(Path::new(std::ffi::OsStr::from_bytes(socket))) {
                Ok(value) => Ok(value),
                Err(error) => {
                    self.failure = Some(SourceFailure::Wayland(error));
                    Ok(false)
                }
            }
        }
        #[cfg(not(feature = "linux-wlr"))]
        {
            let _ = (client, socket);
            Err(Error::BackendNotCompiled(WLR))
        }
    }
    fn create_capture(
        &mut self,
        config: &Config,
        client: Option<&Client>,
    ) -> Result<Box<Backend>, Error> {
        #[cfg(feature = "linux-pipewire")]
        if config.method == PIPEWIRE
            && config.pipewire_fd >= 0
            && self.offered_fd != Some(config.pipewire_fd)
        {
            return Err(Error::UnprovenFdOffer);
        }
        // Allocate the owning slot BEFORE backend construction/global pw_init,
        // as source make_shared does. No aborting-OOM recovery is claimed.
        let slot = Box::<Backend>::try_new_uninit().map_err(|_| Error::Allocation)?;
        let trace = Arc::try_new(Trace::default()).map_err(|_| Error::Allocation)?;
        let unit = match config.method {
            NONE => Unit::None,
            #[cfg(feature = "linux-wlr")]
            WLR => Unit::Wlr(
                wlr::Capture::from_client(client.ok_or(Error::InvalidPrivateOwner)?.clone())
                    .map_err(Error::Wayland)?,
            ),
            #[cfg(feature = "linux-pipewire")]
            PIPEWIRE => {
                let driver = if config.pipewire_fd < 0 {
                    Native::new().map_err(|_| Error::Allocation)?
                } else {
                    // SAFETY: The only non-None permission is a stack-local
                    // factory in connect_source_native. That unsafe caller
                    // proves this exact pending offer for this immediate call.
                    unsafe { Native::with_fd(config.pipewire_fd) }.map_err(|_| Error::Allocation)?
                };
                Unit::PipeWire {
                    driver,
                    consumer: pipewire::Consumer::default(),
                    node: config.pipewire_node,
                }
            }
            method => return Err(Error::BackendNotCompiled(method)),
        };
        // The unused client argument also occurs in a PipeWire-only build.
        let _ = client;
        Ok(Box::write(slot, Backend { unit, trace }))
    }
    fn create_input(
        &mut self,
        config: &Config,
        _: Option<&Client>,
    ) -> Result<Arc<NoneInput>, Error> {
        if config.input_method != NONE {
            // This native factory still has no host input provider. Never
            // replace the caller's retained metadata with Input=None.
            return Err(Error::HostInputRequired(config.input_method));
        }
        Arc::try_new(NoneInput).map_err(|_| Error::Allocation)
    }
    fn init_capture(&mut self, unit: &mut Self::Unit) -> Result<bool, Error> {
        match &mut unit.unit {
            Unit::None => Ok(true),
            #[cfg(feature = "linux-wlr")]
            Unit::Wlr(_) => Ok(true), // Source inherited ScreencapBase::init.
            #[cfg(feature = "linux-pipewire")]
            Unit::PipeWire { driver, node, .. } => {
                let result = if let Some(fd) = driver.pending_source_fd() {
                    if self.offered_fd != Some(fd) {
                        return Err(Error::UnprovenFdOffer);
                    }
                    // SAFETY: Same immediate unsafe caller proof, no owner or
                    // descriptor reuse can intervene between construction/init.
                    unsafe { driver.init_source(*node) }
                } else {
                    driver.init_daemon(*node)
                };
                match result {
                    Ok(()) => Ok(true),
                    Err(error) => {
                        self.failure = Some(SourceFailure::PipeWire(error));
                        Ok(false)
                    }
                }
            }
        }
    }
    fn init_input(&mut self, _: &mut Arc<NoneInput>) -> Result<bool, Error> {
        Ok(true)
    }
    fn input_features(_: &Arc<NoneInput>) -> u64 {
        0
    }
    fn inactive_capture(unit: &mut Self::Unit) {
        match &mut unit.unit {
            Unit::None => {}
            #[cfg(feature = "linux-wlr")]
            Unit::Wlr(capture) => capture.inactive(),
            #[cfg(feature = "linux-pipewire")]
            Unit::PipeWire {
                driver, consumer, ..
            } => consumer.inactive(driver),
        }
    }
    fn inactive_input(_: &mut Arc<NoneInput>) {}
}

pub(in crate::backends::linux) trait Converter {
    type Image;
    type Error;
    #[cfg(feature = "linux-wlr")]
    fn wlr(&mut self, sample: wlr::Sample) -> Result<Self::Image, Self::Error>;
    #[cfg(feature = "linux-pipewire")]
    fn pipewire(
        &mut self,
        raw: Option<RawFrame>,
        mode: pipewire::Conversion,
    ) -> Result<Option<Self::Image>, Self::Error>;
    #[cfg(feature = "linux-pipewire")]
    fn pipewire_reuse(&mut self, raw: Option<RawFrame>) -> Option<Self::Image>;
    #[cfg(feature = "linux-pipewire")]
    fn empty(&mut self) -> Result<Self::Image, Self::Error>;
}
#[cfg(feature = "linux-pipewire")]
struct PipeConverter<'a, C>(&'a mut C);
#[cfg(feature = "linux-pipewire")]
impl<C: Converter> pipewire::Converter<RawFrame> for PipeConverter<'_, C> {
    type Image = C::Image;
    type Error = C::Error;
    fn convert(
        &mut self,
        raw: Option<RawFrame>,
        mode: pipewire::Conversion,
    ) -> Result<Option<C::Image>, C::Error> {
        self.0.pipewire(raw, mode)
    }
    fn reuse(&mut self, raw: Option<RawFrame>) -> Option<C::Image> {
        self.0.pipewire_reuse(raw)
    }
}
#[derive(Debug)]
pub(in crate::backends::linux) enum CaptureFailure<E> {
    // Explicit source-false native evidence, distinct from converter failure.
    #[cfg(feature = "linux-wlr")]
    Wayland(crate::platform::linux::wayland::Error),
    #[cfg(feature = "linux-pipewire")]
    PipeWire(pipewire::Error<E>),
    Conversion(E),
}
#[derive(Debug)]
pub(in crate::backends::linux) enum Evidence {
    None,
    #[cfg(feature = "linux-wlr")]
    Wlr(wlr::Evidence),
    #[cfg(feature = "linux-pipewire")]
    PipeWire(pipewire::native::Evidence),
}
pub(in crate::backends::linux) struct Attempt<E> {
    pub result: Result<bool, CaptureFailure<E>>,
    pub evidence: Evidence,
    // Original PipeWire consumer replay decision, without a new SDK policy.
    // Other methods do not supply this evidence.
    pub replayed: Option<bool>,
}
impl super::NativeManager {
    pub fn connect_native(&mut self) -> Result<Connection, Error> {
        let mut factory = NativeFactory::default();
        let connected = self.connect(&mut factory)?;
        Ok(Connection {
            connected,
            failure: factory.failure,
            evidence: self.native_evidence(),
            close: self.capture.as_ref().map(|b| CloseWatch(b.trace.clone())),
        })
    }
    /// # Safety
    /// Before EACH call, any configured nonnegative FD must be a valid uniquely
    /// offered socket with its owning handle detached. No independent close or
    /// numeric reuse may race construction/init/native access. Keep residual
    /// ownership unresolved until full native close and proven FD identity,
    /// including failure. A consumed/closed old config FD is not a new offer.
    /// This delegates the existing private U driver contract, not an SDK API.
    pub unsafe fn connect_source_native(&mut self) -> Result<Connection, Error> {
        let mut factory = NativeFactory {
            offered_fd: (self.config.pipewire_fd >= 0).then_some(self.config.pipewire_fd),
            failure: None,
        };
        let connected = self.connect(&mut factory)?;
        Ok(Connection {
            connected,
            failure: factory.failure,
            evidence: self.native_evidence(),
            close: self.capture.as_ref().map(|b| CloseWatch(b.trace.clone())),
        })
    }
    pub fn inactive_native(&mut self) -> bool {
        self.inactive()
    }
}
impl<F: Factory<Unit = Box<Backend>>> super::Manager<F> {
    pub fn capture_native<C: Converter>(
        &mut self,
        output: &mut C::Image,
        converter: &mut C,
    ) -> Attempt<C::Error> {
        #[cfg(feature = "linux-pipewire")]
        let mut replayed = None;
        #[cfg(not(feature = "linux-pipewire"))]
        let replayed = None;
        let result = self.capture_with(output, |unit, output| match &mut unit.unit {
            Unit::None => {
                let _ = converter;
                Ok(false)
            }
            #[cfg(feature = "linux-wlr")]
            Unit::Wlr(capture) => {
                let image = capture
                    .capture_with(|sample| Ok(converter.wlr(sample)))
                    .map_err(CaptureFailure::Wayland)?
                    .map_err(CaptureFailure::Conversion)?;
                *output = image;
                Ok(true)
            }
            #[cfg(feature = "linux-pipewire")]
            Unit::PipeWire {
                driver, consumer, ..
            } => {
                let result = consumer.capture(driver, &mut PipeConverter(converter));
                replayed = Some(result.replayed);
                let image = match result.image {
                    Ok(Some(image)) => image,
                    Ok(None) => converter.empty().map_err(CaptureFailure::Conversion)?,
                    Err(pipewire::Error::Conversion(error)) => {
                        return Err(CaptureFailure::Conversion(error));
                    }
                    Err(error) => return Err(CaptureFailure::PipeWire(error)),
                };
                *output = image;
                Ok(true)
            }
        });
        Attempt {
            result,
            evidence: self.native_evidence(),
            replayed,
        }
    }
    fn native_evidence(&self) -> Evidence {
        match self.capture.as_ref().map(|b| &b.unit) {
            None | Some(Unit::None) => Evidence::None,
            #[cfg(feature = "linux-wlr")]
            Some(Unit::Wlr(capture)) => Evidence::Wlr(capture.evidence()),
            #[cfg(feature = "linux-pipewire")]
            Some(Unit::PipeWire { driver, .. }) => Evidence::PipeWire(driver.evidence()),
        }
    }
}

#[path = "manager_native_host.rs"]
mod host;
pub(in crate::backends::linux) use host::HostConnection;
pub(in crate::backends::linux) type HostedFactory<H> =
    super::host::WithInput<NativeFactory, H>;

#[cfg(test)]
#[path = "manager_native_tests.rs"]
mod tests;
