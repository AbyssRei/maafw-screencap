// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
type Close = unsafe extern "C" fn(*mut *mut c_void) -> i32;
type Inactive = unsafe extern "C" fn(*mut c_void) -> i32;
type Capture = unsafe extern "C" fn(*mut c_void, *mut ImageSpan) -> i32;
pub struct Original {
    module: Arc<Module>,
    owner: *mut c_void,
    close: Close,
    capture: Capture,
    inactive: Inactive,
    release: Release,
    failed_close: bool,
}
pub struct HelperOriginal {
    module: Arc<Module>,
    owner: *mut c_void,
    close: Close,
    ensure: Inactive,
    status: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32,
    failed_close: bool,
}
impl Adapter {
    pub fn pseudo_helper_original(
        &self,
        hwnd: *mut c_void,
    ) -> Result<HelperOriginal, Box<dyn Error>> {
        type Open = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
        // SAFETY: Exact distinct helper-owner signatures from print-pseudo.h.
        // Resolve before construction and retain code through the final join.
        unsafe {
            let open: Open = self.module.symbol(c"mfs_pseudo_helper_open")?;
            let mut result = HelperOriginal {
                module: Arc::clone(&self.module),
                owner: null_mut(),
                close: self.module.symbol(c"mfs_pseudo_helper_close")?,
                ensure: self.module.symbol(c"mfs_pseudo_helper_ensure")?,
                status: self.module.symbol(c"mfs_pseudo_helper_status")?,
                failed_close: false,
            };
            let status = open(hwnd, &mut result.owner);
            if status != 0 || result.owner.is_null() {
                return Err(format!("original helper open: {status}").into());
            }
            Ok(result)
        }
    }
    pub fn pseudo_original(&self, hwnd: *mut c_void) -> Result<Original, Box<dyn Error>> {
        type Open = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
        // SAFETY: Exact private print-pseudo.h signatures. Resolve before open;
        // the owned module lease outlives its real C++ monitor and destruction.
        let (open, close, capture, inactive): (Open, Close, Capture, Inactive) = unsafe {
            (
                self.module.symbol(c"mfs_print_pseudo_open")?,
                self.module.symbol(c"mfs_print_pseudo_close")?,
                self.module.symbol(c"mfs_print_pseudo_capture")?,
                self.module.symbol(c"mfs_print_pseudo_inactive")?,
            )
        };
        let mut result = Original {
            module: Arc::clone(&self.module),
            owner: null_mut(),
            close,
            capture,
            inactive,
            release: self.release,
            failed_close: false,
        };
        // SAFETY: Live non-owning fixture HWND, initialized owned output.
        let status = unsafe { open(hwnd, &mut result.owner) };
        if status != 0 || result.owner.is_null() {
            return Err(format!("original pseudo wrapper open failed: {status}").into());
        }
        Ok(result)
    }
    pub fn pseudo_state(&self, hwnd: *mut c_void) -> Result<[u64; 6], Box<dyn Error>> {
        type State = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Exact fixed-size private output; no target memory is borrowed.
        let state: State = unsafe { self.module.symbol(c"mfs_print_pseudo_state")? };
        let mut result = [0; 6];
        if unsafe { state(hwnd, result.as_mut_ptr(), result.len()) } != 0 {
            return Err("pseudo window state query failed".into());
        }
        Ok(result)
    }
}
impl Original {
    pub fn capture(&mut self) -> Result<Image, Box<dyn Error>> {
        let mut image = ImageSpan::default();
        // SAFETY: Serial creating-thread owner; native exceptions are sealed.
        let status = unsafe { (self.capture)(self.owner, &mut image) };
        Image::adopt(&self.module, self.release, status, image)
            .map_err(|status| format!("original pseudo capture failed: {status}").into())
    }
    pub fn inactive(&mut self) -> Result<(), Box<dyn Error>> {
        // SAFETY: Serial creating-thread call retaining the module and owner.
        if unsafe { (self.inactive)(self.owner) } != 0 {
            return Err("original inactive failed".into());
        }
        Ok(())
    }
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.failed_close {
            return Err("original pseudo close previously failed".into());
        }
        // SAFETY: Original close joins before nulling this unique opaque owner.
        if unsafe { (self.close)(&mut self.owner) } != 0 || !self.owner.is_null() {
            // Any unresolved worker/object must retain its executable code.
            std::mem::forget(Arc::clone(&self.module));
            self.failed_close = true;
            return Err("original pseudo close failed".into());
        }
        Ok(())
    }
}
impl Drop for Original {
    fn drop(&mut self) {
        if !self.owner.is_null() && !self.failed_close {
            let _ = self.close();
        }
    }
}
impl HelperOriginal {
    pub fn ensure(&mut self) -> Result<(), Box<dyn Error>> {
        // SAFETY: Serial live owner, native exceptions do not cross this call.
        if unsafe { (self.ensure)(self.owner) } != 0 {
            return Err("original helper ensure failed".into());
        }
        Ok(())
    }
    pub fn is_pseudo(&self) -> Result<bool, Box<dyn Error>> {
        let mut pseudo = 0;
        // SAFETY: Exact initialized u32 output; source reads its atomic flag.
        if unsafe { (self.status)(self.owner, &mut pseudo) } != 0 || pseudo > 1 {
            return Err("original helper status failed".into());
        }
        Ok(pseudo == 1)
    }
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.failed_close {
            return Err("original helper close previously failed".into());
        }
        // SAFETY: Matching distinct owner; destructor stops/joins before nulling.
        if unsafe { (self.close)(&mut self.owner) } != 0 || !self.owner.is_null() {
            self.failed_close = true;
            std::mem::forget(Arc::clone(&self.module));
            return Err("original helper close failed".into());
        }
        Ok(())
    }
}
impl Drop for HelperOriginal {
    fn drop(&mut self) {
        if !self.owner.is_null() && !self.failed_close {
            let _ = self.close();
        }
    }
}

#[test]
#[ignore = "explicit original worker module ownership and closed-owner failure outputs"]
fn keeps_original_monitor_code_owned_until_close_after_adapter_destruction()
-> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let module = Arc::downgrade(&adapter.module);
    let mut original = adapter.pseudo_original(null_mut())?;
    drop(adapter);
    assert!(module.upgrade().is_some());
    // Original null HWND rejection has a fully empty image output; no Image
    // slice can be formed. This independently retained owner can still join.
    assert!(original.capture().is_err());
    original.close()?;
    assert!(original.capture().is_err());
    assert!(original.inactive().is_err());
    drop(original);
    assert!(module.upgrade().is_none());
    Ok(())
}
