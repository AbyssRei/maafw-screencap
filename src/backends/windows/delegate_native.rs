// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real module/create/destroy ownership, with a narrow caller-supplied native
//! class ABI bridge. No product C++/OpenCV linkage or final SDK ABI.
use super::delegate::Boundary;
use crate::platform::windows::delegate::{self as library, Lease, Location};
use std::{
    ffi::c_void,
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
};

pub(crate) static DESTROY_ERROR: AtomicI32 = AtomicI32::new(0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Library(library::Error),
    Bridge(i32),
    Allocation,
    InvalidOwner,
}

/// # Safety
/// Every address/pointee must use the actual original Win32ControlUnitAPI
/// declarations. All C++ exceptions must be contained, and operations are
/// synchronous on the caller channel. Images must own CPU storage independent
/// of the unit DLL before capture returns, without retaining its native lease.
/// The implementation and its ABI bridge must outlive loaders and images.
/// If destroy cannot prove consumption, it must still make all callback/host
/// borrows unreachable before returning: only independently owned native
/// resources and module code may remain. Keeping a DLL loaded alone does not
/// extend the lifetime of the borrowed bridge or host objects.
/// Implementations must return errors rather than unwind while owning foreign
/// resources; unwinding must never bypass consumption/quiescence obligations.
pub(crate) unsafe trait UnitOps {
    type Image;
    unsafe fn create(
        &self,
        function: usize,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<NonNull<c_void>>, i32>;
    /// Zero proves consumption. A nonzero result has unknown consumption;
    /// the loader retains its module rather than unloading live native code.
    unsafe fn destroy(&self, function: usize, unit: NonNull<c_void>) -> i32;
    unsafe fn connect(&self, unit: NonNull<c_void>) -> Result<bool, i32>;
    unsafe fn capture(&self, unit: NonNull<c_void>, output: &mut Self::Image) -> Result<bool, i32>;
    unsafe fn inactive(&self, unit: NonNull<c_void>) -> Result<bool, i32>;
}
pub(crate) struct Native<'a, O: UnitOps> {
    location: Location,
    ops: &'a O,
    last_load: Option<library::Error>,
}
pub(crate) struct Loader<'a, O: UnitOps> {
    // Destroy the foreign unit explicitly before the lease field is released.
    unit: Option<NonNull<c_void>>,
    module: Option<Lease>,
    create: Option<usize>,
    destroy: Option<usize>,
    ops: &'a O,
    _caller_channel: PhantomData<Rc<()>>,
}
pub(crate) struct View(Arc<usize>);
impl<'a, O: UnitOps> Native<'a, O> {
    pub(crate) fn new(directory: &Path, ops: &'a O) -> Result<Self, Error> {
        Ok(Self {
            location: Location::new(directory).map_err(Error::Library)?,
            ops,
            last_load: None,
        })
    }
    pub(crate) fn last_load_error(&self) -> Option<library::Error> {
        self.last_load
    }
}
impl<O: UnitOps> Drop for Loader<'_, O> {
    fn drop(&mut self) {
        if let Some(unit) = self.unit.take() {
            // SAFETY: Unit belongs to this loaded factory; the bridge remains
            // borrowed and code remains loaded through this synchronous call.
            let code = unsafe { self.ops.destroy(self.destroy.unwrap(), unit) };
            if code != 0 {
                let _ =
                    DESTROY_ERROR.compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
                // No cleanup-success claim: retaining code is safer than
                // unloading an object whose destruction is unproved. Scope
                // diagnostics must report/reclaim such failures explicitly.
                if let Some(module) = self.module.take() {
                    std::mem::forget(module);
                }
            }
        }
    }
}
impl<'a, O: UnitOps> Boundary for Native<'a, O> {
    type Loader = Loader<'a, O>;
    type Handle = NonNull<c_void>;
    type View = View;
    type Image = O::Image;
    type Error = Error;
    fn new_loader(&mut self) -> Result<Self::Loader, Error> {
        Ok(Loader {
            unit: None,
            module: None,
            create: None,
            destroy: None,
            ops: self.ops,
            _caller_channel: PhantomData,
        })
    }
    fn load(&mut self, loader: &mut Self::Loader) -> Result<bool, Error> {
        self.last_load = None;
        let module = match Lease::load(&self.location) {
            Ok(module) => module,
            Err(error) => {
                self.last_load = Some(error);
                return Ok(false);
            }
        };
        // SAFETY: Only addresses are obtained, with exact terminated original
        // names. UnitOps contains native class invocation and exceptions.
        let create = unsafe { module.symbol(c"MaaWin32ControlUnitCreate") };
        let destroy = unsafe { module.symbol(c"MaaWin32ControlUnitDestroy") };
        if create.is_none() || destroy.is_none() {
            return Ok(false);
        }
        loader.create = create;
        loader.destroy = destroy;
        loader.module = Some(module);
        Ok(true)
    }
    fn create(
        &mut self,
        loader: &mut Self::Loader,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<Self::Handle>, Error> {
        if loader.unit.is_some() {
            return Err(Error::InvalidOwner);
        }
        // SAFETY: Successful export lookup and active module lease; exact
        // original scalar selector widths, no gamepad/input operation.
        let unit = unsafe {
            self.ops.create(
                loader.create.ok_or(Error::InvalidOwner)?,
                hwnd,
                method,
                mouse,
                keyboard,
            )
        }
        .map_err(Error::Bridge)?;
        loader.unit = unit;
        Ok(unit)
    }
    fn view(&mut self, loader: &Self::Loader, handle: Self::Handle) -> Result<Self::View, Error> {
        if loader.unit != Some(handle) {
            return Err(Error::InvalidOwner);
        }
        // Independent no-op control block, not a strong owner of native unit.
        Ok(View(
            Arc::try_new(handle.as_ptr() as usize).map_err(|_| Error::Allocation)?,
        ))
    }
    fn connect(&mut self, loader: &mut Self::Loader, view: &Self::View) -> Result<bool, Error> {
        let unit = checked(loader, view)?;
        // SAFETY: Matching live unit/module and synchronous ABI bridge.
        unsafe { self.ops.connect(unit) }.map_err(Error::Bridge)
    }
    fn capture(
        &mut self,
        loader: &mut Self::Loader,
        view: &Self::View,
        output: &mut Self::Image,
    ) -> Result<bool, Error> {
        let unit = checked(loader, view)?;
        // SAFETY: Matching live unit/module; UnitOps establishes CPU output
        // ownership and contains any C++ exception before returning to Rust.
        unsafe { self.ops.capture(unit, output) }.map_err(Error::Bridge)
    }
    fn inactive(&mut self, loader: &mut Self::Loader, view: &Self::View) -> Result<bool, Error> {
        let unit = checked(loader, view)?;
        // SAFETY: Matching live unit/module and borrowed ABI bridge.
        unsafe { self.ops.inactive(unit) }.map_err(Error::Bridge)
    }
}
fn checked<O: UnitOps>(loader: &Loader<'_, O>, view: &View) -> Result<NonNull<c_void>, Error> {
    loader
        .unit
        .filter(|unit| loader.module.is_some() && unit.as_ptr() as usize == *view.0)
        .ok_or(Error::InvalidOwner)
}

#[cfg(all(test, feature = "prototype-windows"))]
#[path = "delegate_native_tests.rs"]
mod tests;
