// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
type Create = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> i32;
type Capture = unsafe extern "C" fn(*mut c_void, *mut ImageSpan) -> i32;
type Close = unsafe extern "C" fn(*mut *mut c_void) -> i32;
pub(crate) struct Source<'a> {
    adapter: &'a Adapter,
    owner: *mut c_void,
    capture: Capture,
    close: Close,
}
impl Adapter {
    pub fn desktop_pending_source(
        &self,
        window: *mut c_void,
    ) -> Result<Source<'_>, Box<dyn Error>> {
        // SAFETY: Exact private desktop-pending.h declarations, resolved before
        // construction. Owner remains on this caller and borrows its module.
        let (create, capture, close): (Create, Capture, Close) = unsafe {
            (
                self.module.symbol(c"mfs_desktop_pending_create")?,
                self.module.symbol(c"mfs_desktop_pending_capture")?,
                self.module.symbol(c"mfs_desktop_pending_destroy")?,
            )
        };
        let mut source = Source {
            adapter: self,
            owner: null_mut(),
            capture,
            close,
        };
        let status = unsafe { create(window, &mut source.owner) };
        if status != 0 || source.owner.is_null() {
            return Err(format!("original DesktopDupWindow creation {status}").into());
        }
        Ok(source)
    }
}
impl Source<'_> {
    pub fn capture(&mut self) -> Result<Image, i32> {
        let mut output = ImageSpan::default();
        // SAFETY: Live same-caller owner and complete writable descriptor; the
        // adopted independent CPU owner retains its own actual module Arc.
        let status = unsafe { (self.capture)(self.owner, &mut output) };
        self.adapter.adopt(status, output)
    }
}
impl Drop for Source<'_> {
    fn drop(&mut self) {
        // SAFETY: Sole same-caller owner, exact consumed-pointer destroy ABI.
        // Native destructor is noexcept; outputs stay independently owned.
        let status = unsafe { (self.close)(&mut self.owner) };
        assert!(status == 0 && self.owner.is_null());
    }
}
