// Copyright (c) Microsoft Corporation. All rights reserved.
// Copyright (C) 2026 AbyssRei (Rust adaptation)
// SPDX-License-Identifier: LGPL-3.0-or-later AND MIT
// Completion wait adapts SDK C++/WinRT Windows.Foundation.h::wait_for_completed.
// Original MIT permission notice: licenses/cppwinrt-MIT.txt.
// P1 caller-thread WGC boundary. No implicit worker or apartment guard.
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
pub(crate) mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_frame_pool.rs"));
}

use native::*;
use windows_core::Interface;
#[path = "wgc_factory.rs"]
mod factory;

pub(crate) fn activation<C: windows_core::RuntimeName>() -> windows_core::Result<IActivationFactory>
{
    factory::load::<C, IActivationFactory>()
}

pub(crate) fn next_frame(
    pool: &Direct3D11CaptureFramePool,
) -> windows_core::Result<Option<Direct3D11CaptureFrame>> {
    let mut output = std::ptr::null_mut();
    // SAFETY: Serial caller owns the live pool and exact initialized out-slot.
    // Native success/null means no frame, not the rich projection's null error.
    unsafe {
        (Interface::vtable(pool).TryGetNextFrame)(pool.as_raw(), &mut output).ok()?;
        Ok(if output.is_null() {
            None
        } else {
            Some(Direct3D11CaptureFrame::from_raw(output))
        })
    }
}

pub(crate) fn create_pool(
    device: &IDirect3DDevice,
    size: SizeInt32,
) -> windows_core::Result<Direct3D11CaptureFramePool> {
    let factory: IDirect3D11CaptureFramePoolStatics =
        factory::load::<Direct3D11CaptureFramePool, _>()?;
    let mut output = std::ptr::null_mut();
    // SAFETY: Exact generated ABI; same caller and device, original Create,
    // BGRA and one buffer. No process-global generated factory cache is used.
    unsafe {
        (Interface::vtable(&factory).Create)(
            factory.as_raw(),
            device.as_raw(),
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            size,
            &mut output,
        )
        .ok()?;
        if output.is_null() {
            return Err(windows_core::Error::empty());
        }
        Ok(Direct3D11CaptureFramePool::from_raw(output))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropertyState {
    Unsupported,
    Applied,
    AsyncIncomplete(i32),
    Denied(i32),
    Failed(windows_core::HRESULT),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PropertyOutcome {
    pub property: &'static str,
    pub state: PropertyState,
    pub wait_result: Option<u32>,
    pub notify_error: u32,
}
impl PropertyOutcome {
    fn new(property: &'static str, state: PropertyState) -> Self {
        Self {
            property,
            state,
            wait_result: None,
            notify_error: 0,
        }
    }
}

pub(crate) enum ConfigureError {
    Metadata(windows_core::Error),
    Allocation(std::alloc::AllocError),
}
impl From<windows_core::Error> for ConfigureError {
    fn from(value: windows_core::Error) -> Self {
        Self::Metadata(value)
    }
}

fn info() -> windows_core::Result<IApiInformationStatics> {
    factory::load::<ApiInformation, _>()
}
fn property_present(property: &str) -> windows_core::Result<bool> {
    let factory = info()?;
    let name = windows_core::HSTRING::from("Windows.Graphics.Capture.GraphicsCaptureSession");
    let property = windows_core::HSTRING::from(property);
    let mut output = false;
    // SAFETY: Exact generated ABI; both immutable HSTRING owners remain live.
    unsafe {
        (factory.vtable().IsPropertyPresent)(
            factory.as_raw(),
            std::mem::transmute_copy(&name),
            std::mem::transmute_copy(&property),
            &mut output,
        )
        .ok()?;
    }
    Ok(output)
}
fn border_supported() -> windows_core::Result<bool> {
    let factory = info()?;
    let contract = windows_core::HSTRING::from("Windows.Foundation.UniversalApiContract");
    let mut output = false;
    // SAFETY: Exact ABI, initialized writable bool and live HSTRING owner.
    unsafe {
        (factory.vtable().IsApiContractPresentByMajor)(
            factory.as_raw(),
            std::mem::transmute_copy(&contract),
            10,
            &mut output,
        )
        .ok()?;
    }
    if !output {
        return Ok(false);
    }
    let name = windows_core::HSTRING::from("Windows.Graphics.Capture.GraphicsCaptureAccess");
    // SAFETY: Same valid factory; exact immutable HSTRING input and output.
    unsafe {
        (factory.vtable().IsTypePresent)(
            factory.as_raw(),
            std::mem::transmute_copy(&name),
            &mut output,
        )
        .ok()?;
    }
    if !output {
        return Ok(false);
    }
    property_present("IsBorderRequired")
}

use std::sync::{
    Arc,
    atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering},
};
pub(crate) static LIVE_COMPLETIONS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static EVENT_CLOSE_ERROR: AtomicU32 = AtomicU32::new(0);

// Kernel event ownership may be released by the final native delegate or caller.
// Store the opaque handle value, with no Rust pointer dereference or affinity.
struct Event(usize);
impl Drop for Event {
    fn drop(&mut self) {
        // SAFETY: Last Arc owner means no callback/waiter still borrows the event.
        // CloseHandle supports a different releasing thread; no retry on failure.
        if !unsafe { CloseHandle(HANDLE(self.0 as *mut std::ffi::c_void)) }.as_bool() {
            EVENT_CLOSE_ERROR.store(unsafe { GetLastError() }, Ordering::Release);
        }
        LIVE_COMPLETIONS
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1))
            .expect("one live completion per owned event");
    }
}
struct Completion {
    event: Event,
    status: AtomicI32,
    notify_error: AtomicU32,
}

pub(crate) fn configure_border(
    session: &GraphicsCaptureSession,
) -> Result<PropertyOutcome, ConfigureError> {
    const NAME: &str = "IsBorderRequired";
    // These metadata calls are outside the original property's try/catch.
    if !border_supported()? {
        return Ok(PropertyOutcome::new(NAME, PropertyState::Unsupported));
    }
    let factory: IGraphicsCaptureAccessStatics = match factory::load::<GraphicsCaptureAccess, _>() {
        Ok(factory) => factory,
        Err(error) => {
            return Ok(PropertyOutcome::new(
                NAME,
                PropertyState::Failed(error.code()),
            ));
        }
    };
    let operation =
        (|| -> windows_core::Result<windows_future::IAsyncOperation<AppCapabilityAccessStatus>> {
            let mut output = std::ptr::null_mut();
            // SAFETY: Exact borderless request ABI and initialized output pointer.
            unsafe {
                (factory.vtable().RequestAccessAsync)(
                    factory.as_raw(),
                    GraphicsCaptureAccessKind::Borderless,
                    &mut output,
                )
                .ok()?;
                if output.is_null() {
                    return Err(windows_core::Error::empty());
                }
                Ok(windows_future::IAsyncOperation::from_raw(output))
            }
        })();
    let operation = match operation {
        Ok(operation) => operation,
        Err(error) => {
            return Ok(PropertyOutcome::new(
                NAME,
                PropertyState::Failed(error.code()),
            ));
        }
    };
    // Match C++/WinRT wait_for: a manual-reset event and Completed handler,
    // five-second WaitForSingleObject, no pumping, polling Status or cancel.
    let handle = unsafe { CreateEventW(None, true, false, windows_core::PCWSTR::null()) };
    if handle.0.is_null() {
        return Ok(PropertyOutcome::new(
            NAME,
            PropertyState::Failed(windows_core::Error::from_thread().code()),
        ));
    }
    LIVE_COMPLETIONS
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
        .expect("completion count within native handle bound");
    let state = Arc::try_new(Completion {
        event: Event(handle.0 as usize),
        status: AtomicI32::new(windows_future::AsyncStatus::Started.0),
        notify_error: AtomicU32::new(0),
    })
    .map_err(ConfigureError::Allocation)?;
    let retained = Arc::clone(&state);
    let completed = windows_future::AsyncOperationCompletedHandler::new(move |_, status| {
        retained.status.store(status.0, Ordering::Release);
        // SAFETY: Callback independently owns the kernel event through its Arc.
        // No Session/FramePool/native frame access occurs on this unknown thread.
        if !unsafe { SetEvent(HANDLE(retained.event.0 as *mut std::ffi::c_void)) }.as_bool() {
            retained
                .notify_error
                .store(unsafe { GetLastError() }, Ordering::Release);
        }
        Ok(())
    });
    if let Err(error) = operation.SetCompleted(&completed) {
        return Ok(PropertyOutcome::new(
            NAME,
            PropertyState::Failed(error.code()),
        ));
    }
    // SAFETY: Live shared event; native timeout remains exactly 5000 ms.
    let waited = unsafe { WaitForSingleObject(handle, 5000) };
    let status = state.status.load(Ordering::Acquire);
    let outcome = if status != windows_future::AsyncStatus::Completed.0 {
        PropertyState::AsyncIncomplete(status)
    } else {
        match operation.GetResults() {
            Ok(value) if value == AppCapabilityAccessStatus::Allowed => {
                match session.SetIsBorderRequired(false) {
                    Ok(()) => PropertyState::Applied,
                    Err(error) => PropertyState::Failed(error.code()),
                }
            }
            Ok(value) => PropertyState::Denied(value.0),
            Err(error) => PropertyState::Failed(error.code()),
        }
    };
    // On timeout the native operation/delegate can retain the independent state.
    // Neither returning nor dropping this backend is claimed to cancel/drain it.
    Ok(PropertyOutcome {
        property: NAME,
        state: outcome,
        wait_result: Some(waited),
        notify_error: state.notify_error.load(Ordering::Acquire),
    })
}

pub(crate) fn configure_cursor(
    session: &GraphicsCaptureSession,
) -> Result<PropertyOutcome, ConfigureError> {
    const NAME: &str = "IsCursorCaptureEnabled";
    if !property_present(NAME)? {
        return Ok(PropertyOutcome::new(NAME, PropertyState::Unsupported));
    }
    Ok(PropertyOutcome::new(
        NAME,
        match session.SetIsCursorCaptureEnabled(false) {
            Ok(()) => PropertyState::Applied,
            Err(error) => PropertyState::Failed(error.code()),
        },
    ))
}
pub(crate) fn configure_secondary(
    session: &GraphicsCaptureSession,
) -> Result<PropertyOutcome, ConfigureError> {
    const NAME: &str = "IncludeSecondaryWindows";
    if !property_present(NAME)? {
        return Ok(PropertyOutcome::new(NAME, PropertyState::Unsupported));
    }
    Ok(PropertyOutcome::new(
        NAME,
        match session.SetIncludeSecondaryWindows(true) {
            Ok(()) => PropertyState::Applied,
            Err(error) => PropertyState::Failed(error.code()),
        },
    ))
}
