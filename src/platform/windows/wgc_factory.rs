// Copyright (c) Microsoft Corporation. All rights reserved.
// Copyright (C) 2026 AbyssRei (Rust adaptation)
// SPDX-License-Identifier: LGPL-3.0-or-later AND MIT
// Caller-thread adaptation of the selected SDK C++/WinRT activation loader.
// Original MIT permission notice: licenses/cppwinrt-MIT.txt.
use super::native::*;
use std::sync::atomic::{AtomicU32, Ordering};
use windows_core::Interface;

pub(super) static CLOSE_ERROR: AtomicU32 = AtomicU32::new(0);

// The SDK retries after CO_E_NOTINITIALIZED and tries the class-name DLL
// hierarchy after any remaining HRESULT failure. windows-core 0.100 only
// attempts that fallback for REGDB_E_CLASSNOTREG. Preserve the SDK branch here.
// Explicit owners avoid the projection's process-global raw factory cache.
pub(super) fn load<C: windows_core::RuntimeName, I: Interface>() -> windows_core::Result<I> {
    let name = windows_core::HSTRING::from(C::NAME);
    // SAFETY: Exact generated IID and immutable HSTRING; owns one output ref.
    let mut result = unsafe { RoGetActivationFactory::<I>(&name) };
    if result
        .as_ref()
        .err()
        .is_some_and(|error| error.code() == CO_E_NOTINITIALIZED)
    {
        // Like the SDK, retain combase load and MTA usage for the process. This
        // is the measured implicit MTA effect, not our own RoInitialize count.
        let combase = unsafe {
            LoadLibraryExW(
                windows_core::w!("combase.dll"),
                None,
                u32::try_from(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS).expect("fixed loader flags"),
            )
        };
        if combase.0.is_null() {
            return result;
        }
        let usage = unsafe { GetProcAddress(combase, windows_core::s!("CoIncrementMTAUsage")) };
        let Some(usage) = usage else {
            return result;
        };
        type Increment =
            unsafe extern "system" fn(*mut *mut std::ffi::c_void) -> windows_core::HRESULT;
        // SAFETY: This documented combase export has the exact cookie signature.
        let usage: Increment = unsafe { std::mem::transmute(usage) };
        let mut cookie = std::ptr::null_mut();
        // Source ignores this HRESULT and deliberately retains its cookie.
        let _ = unsafe { usage(&mut cookie) };
        result = unsafe { RoGetActivationFactory::<I>(&name) };
    }
    let original = match result {
        Ok(factory) => return Ok(factory),
        Err(error) => error,
    };
    // Projection null-output errors have code zero. The SDK's native S_OK
    // branch does not probe DLLs, even for an invalid success/null factory.
    if original.code().0 == 0 {
        return Err(original);
    }
    // Own the original Error/IErrorInfo across probes, as the SDK helper does.
    let mut prefix = C::NAME;
    while let Some(dot) = prefix.rfind('.') {
        prefix = &prefix[..dot];
        // This private path only receives the four fixed ASCII system classes.
        // A terminated stack buffer adds no allocation failure during fallback.
        if !prefix.is_ascii() || prefix.len().checked_add(5).is_none_or(|n| n > 128) {
            break;
        }
        let mut path = [0_u16; 128];
        for (slot, byte) in path.iter_mut().zip(prefix.bytes().chain(*b".dll")) {
            *slot = u16::from(byte);
        }
        // SAFETY: Terminated filename, original secure search flags.
        let module = unsafe {
            LoadLibraryExW(
                windows_core::PCWSTR(path.as_ptr()),
                None,
                u32::try_from(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS).expect("fixed loader flags"),
            )
        };
        if module.0.is_null() {
            continue;
        }
        let module = Library(module);
        let Some(function) =
            (unsafe { GetProcAddress(module.0, windows_core::s!("DllGetActivationFactory")) })
        else {
            continue;
        };
        type GetFactory = unsafe extern "system" fn(
            *mut std::ffi::c_void,
            *mut *mut std::ffi::c_void,
        ) -> windows_core::HRESULT;
        // SAFETY: Standard WinRT server export ABI: HSTRING, IActivationFactory**.
        let function: GetFactory = unsafe { std::mem::transmute(function) };
        let mut output = std::ptr::null_mut();
        let code = unsafe { function(std::mem::transmute_copy(&name), &mut output) };
        let returned = if output.is_null() {
            None
        } else {
            // SAFETY: Native export transfers a valid owned factory reference.
            Some(unsafe { IActivationFactory::from_raw(output) })
        };
        if code.0 != 0 {
            continue;
        }
        let Some(returned) = returned else {
            continue;
        };
        let found = if I::IID == IActivationFactory::IID {
            // SAFETY: Same IID; transfer without an extra QueryInterface.
            Ok(unsafe { I::from_raw(returned.into_raw()) })
        } else {
            returned.cast::<I>()
        };
        if let Ok(found) = found {
            // Match source library.detach(): keep code loaded for all future
            // interface/destructor calls, including after this factory drops.
            std::mem::forget(module);
            return Ok(found);
        }
    }
    Err(original)
}

struct Library(HMODULE);
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: Own exactly one failed-candidate load. Factory/interface locals
        // have been released before this guard; do not retry a failed unload.
        if !unsafe { FreeLibrary(self.0) }.as_bool() {
            CLOSE_ERROR.store(unsafe { GetLastError() }, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_activation_hresult_when_all_class_dll_candidates_are_missing() {
        struct Missing;
        impl windows_core::RuntimeName for Missing {
            const NAME: &'static str = "MfsAbsentCaptureFixture.Missing.Factory";
        }
        let result = load::<Missing, IActivationFactory>();
        assert!(result.is_err());
        // The loader has established the same upstream implicit MTA if needed.
        // Compare the activation error, not LoadLibrary's last Win32 error.
        let direct = unsafe {
            RoGetActivationFactory::<IActivationFactory>(&windows_core::HSTRING::from(
                <Missing as windows_core::RuntimeName>::NAME,
            ))
        };
        assert_eq!(result.unwrap_err().code(), direct.unwrap_err().code());
        assert_eq!(CLOSE_ERROR.load(Ordering::Acquire), 0);
    }
}
