// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only caller-local observation of primary-output COM failures.
use super::native::*;
use std::{cell::Cell, ffi::c_void, marker::PhantomData, rc::Rc};
use windows_core::{HRESULT, Interface};

pub(crate) type Created = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;
pub(crate) type Factory = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
pub(crate) type End = unsafe extern "C" fn() -> i32;
#[derive(Clone, Copy)]
struct Active {
    created: Created,
    factory: Factory,
}
thread_local! { static ACTIVE: Cell<Option<Active>> = const { Cell::new(None) }; }
pub(crate) struct Guard {
    end: Option<End>,
    _caller: PhantomData<Rc<()>>,
}
impl Guard {
    pub fn begin(created: Created, factory: Factory, end: End) -> Self {
        ACTIVE.with(|s| {
            assert!(s.get().is_none());
            s.set(Some(Active { created, factory }));
        });
        Self {
            end: Some(end),
            _caller: PhantomData,
        }
    }
    pub fn finish(mut self) -> windows_core::Result<()> {
        ACTIVE.with(|s| s.set(None));
        let end = self.end.take().unwrap();
        // SAFETY: Matching retained private module, all facade owners ended.
        HRESULT(unsafe { end() }).ok()
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|s| s.set(None));
        if let Some(end) = self.end.take() {
            // SAFETY: The retained module/once-only scope also outlives unwind.
            let code = unsafe { end() };
            if code != 0 {
                eprintln!("primary-output probe cleanup HRESULT={code:#x}");
            }
        }
    }
}
pub(crate) fn created(
    device: Option<&ID3D11Device>,
    context: Option<&ID3D11DeviceContext>,
) -> windows_core::Result<()> {
    let Some(active) = ACTIVE.with(Cell::get) else {
        return Ok(());
    };
    let pointers = [
        device.map_or(std::ptr::null_mut(), Interface::as_raw),
        context.map_or(std::ptr::null_mut(), Interface::as_raw),
    ];
    // SAFETY: Exact private desktop-output-probe.h contract; both borrowed COM
    // owners are live on this caller. Observer code remains loaded for process
    // lifetime, including error paths and destruction notifications.
    HRESULT(unsafe { (active.created)(pointers[0], pointers[1]) }).ok()
}
pub(crate) fn factory(
    result: windows_core::Result<IDXGIFactory>,
) -> windows_core::Result<IDXGIFactory> {
    let factory = result?;
    let Some(active) = ACTIVE.with(Cell::get) else {
        return Ok(factory);
    };
    let mut output = std::ptr::null_mut();
    // SAFETY: Borrows the live real factory and exact initialized output slot.
    // Success transfers one facade reference; failure leaves output null. The
    // facade retains its own real reference, so this original owner may end.
    let code = HRESULT(unsafe { (active.factory)(factory.as_raw(), &mut output) });
    code.ok()?;
    if output.is_null() {
        return Err(HRESULT(0x8000_4003_u32.cast_signed()).into());
    }
    // SAFETY: One nonnull transferred IDXGIFactory reference from the matching
    // native oracle; the test retains its module through all releases.
    Ok(unsafe { IDXGIFactory::from_raw(output) })
}
