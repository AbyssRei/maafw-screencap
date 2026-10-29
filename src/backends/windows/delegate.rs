// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Capture-only optional Win32 delegation from the original outer manager.
//! Input/gamepad connection and the final SDK/loader ABI are outside this slice.
use std::{marker::PhantomData, rc::Rc};

/// Native owners/loans stay on the caller's channel. A loan's destructor must
/// not destroy the unit: the loader owns it, as in the original no-op deleter.
/// Loader destruction destroys its unit before releasing the module. The
/// boundary contains native exceptions and retains independent image owners.
pub(crate) trait Boundary {
    type Loader;
    type Handle;
    type View;
    type Image;
    type Error;
    fn new_loader(&mut self) -> Result<Self::Loader, Self::Error>;
    fn load(&mut self, loader: &mut Self::Loader) -> Result<bool, Self::Error>;
    fn create(
        &mut self,
        loader: &mut Self::Loader,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<Self::Handle>, Self::Error>;
    fn view(
        &mut self,
        loader: &Self::Loader,
        handle: Self::Handle,
    ) -> Result<Self::View, Self::Error>;
    fn connect(
        &mut self,
        loader: &mut Self::Loader,
        view: &Self::View,
    ) -> Result<bool, Self::Error>;
    fn capture(
        &mut self,
        loader: &mut Self::Loader,
        view: &Self::View,
        output: &mut Self::Image,
    ) -> Result<bool, Self::Error>;
    fn inactive(
        &mut self,
        loader: &mut Self::Loader,
        view: &Self::View,
    ) -> Result<bool, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum Error<E> {
    Boundary(E),
    // The source may leave a no-op shared view after destroying its real
    // loader owner on failed reconnect. Calling that view has no defined
    // source behavior. Retain its partial state but never dereference it.
    InvalidatedSourceView,
}

pub(crate) struct CaptureDelegate<B: Boundary> {
    // Preserve source member destruction order: the no-op view ends before
    // the loader/unit/module. The boundary remains available until both end.
    view: Option<B::View>,
    loader: Option<B::Loader>,
    boundary: B,
    hwnd: usize,
    method: u64,
    view_live: bool,
    _caller_channel: PhantomData<Rc<()>>,
}

impl<B: Boundary> CaptureDelegate<B> {
    pub(crate) fn new(boundary: B, hwnd: usize, method: u64) -> Self {
        Self {
            view: None,
            loader: None,
            boundary,
            hwnd,
            method,
            view_live: false,
            _caller_channel: PhantomData,
        }
    }

    /// None means the original optional capture condition was not selected.
    /// Some(false) remains a capture initialization failure; it says nothing
    /// about the excluded outer input manager's connection result.
    pub(crate) fn initialize(&mut self) -> Result<Option<bool>, Error<B::Error>> {
        if self.hwnd == 0 || self.method == 0 {
            return Ok(None);
        }
        // Construct first: an error here must leave the preceding owner live.
        let next = self.boundary.new_loader().map_err(Error::Boundary)?;
        let previous = self.loader.replace(next);
        self.view_live = false;
        drop(previous);
        let loader = self.loader.as_mut().unwrap();
        if !self.boundary.load(loader).map_err(Error::Boundary)? {
            self.loader = None;
            return Ok(Some(false));
        }
        let Some(handle) = self
            .boundary
            .create(loader, self.hwnd, self.method, 0, 0)
            .map_err(Error::Boundary)?
        else {
            self.loader = None;
            return Ok(Some(false));
        };
        // Source shared-view construction may fail before replacing the old
        // view. Keep the newly created loader/handle on that error path.
        let view = self
            .boundary
            .view(loader, handle)
            .map_err(Error::Boundary)?;
        self.view = Some(view);
        self.view_live = true;
        if !self
            .boundary
            .connect(loader, self.view.as_ref().unwrap())
            .map_err(Error::Boundary)?
        {
            self.view = None;
            self.view_live = false;
            self.loader = None;
            return Ok(Some(false));
        }
        Ok(Some(true))
    }

    pub(crate) fn capture(&mut self, output: &mut B::Image) -> Result<bool, Error<B::Error>> {
        let Some(view) = self.view.as_ref() else {
            return Ok(false);
        };
        if !self.view_live {
            return Err(Error::InvalidatedSourceView);
        }
        // Forward the caller's output directly, including a backend that
        // changes it before returning false/error. There is no connected gate.
        self.boundary
            .capture(self.loader.as_mut().unwrap(), view, output)
            .map_err(Error::Boundary)
    }

    pub(crate) fn inactive(&mut self) -> Result<bool, Error<B::Error>> {
        let Some(view) = self.view.as_ref() else {
            return Ok(true);
        };
        if !self.view_live {
            return Err(Error::InvalidatedSourceView);
        }
        self.boundary
            .inactive(self.loader.as_mut().unwrap(), view)
            .map_err(Error::Boundary)
    }
}

#[cfg(test)]
#[path = "delegate_tests.rs"]
mod tests;
