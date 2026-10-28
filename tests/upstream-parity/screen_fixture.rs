// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Reuse the real rendering/UI owner; no PrintWindow capture dependency.
use super::*;
#[derive(Default)]
#[repr(C)]
struct Span {
    owner: *mut c_void,
    hwnd: *mut c_void,
    popup: *mut c_void,
    width: i32,
    height: i32,
    popup_width: i32,
    popup_height: i32,
}
type Close = unsafe extern "C" fn(*mut *mut c_void) -> i32;
pub struct Fixture {
    module: Arc<Module>,
    span: Span,
    close: Close,
    failed_close: bool,
}
impl Adapter {
    pub fn screen_fixture(&self) -> Result<Fixture, Box<dyn Error>> {
        type Create = unsafe extern "C" fn(i32, i32, i32, i32, i32, i32, u32, *mut Span) -> i32;
        // SAFETY: Exact print-parity.h/print-thread.h signatures. Resolve the
        // close function before any partial owner/callback can be created.
        let (layout, create, close): (Layout, Create, Close) = unsafe {
            (
                self.module.symbol(c"mfs_print_parity_layout")?,
                self.module.symbol(c"mfs_print_thread_fixture")?,
                self.module.symbol(c"mfs_print_thread_close")?,
            )
        };
        let mut observed = [0_u64; 10];
        // SAFETY: Exact initialized writable layout array.
        if unsafe { layout(observed.as_mut_ptr(), observed.len()) } != 0 {
            return Err("render fixture layout query failed".into());
        }
        let expected = [
            size_of::<Span>(),
            align_of::<Span>(),
            offset_of!(Span, owner),
            offset_of!(Span, hwnd),
            offset_of!(Span, popup),
            offset_of!(Span, width),
            offset_of!(Span, height),
            offset_of!(Span, popup_width),
            offset_of!(Span, popup_height),
            0x100,
        ]
        .map(|n| u64::try_from(n).unwrap());
        if observed != expected {
            return Err("render fixture C/Rust layout differs".into());
        }
        let mut fixture = Fixture {
            module: Arc::clone(&self.module),
            span: Span::default(),
            close,
            failed_close: false,
        };
        // SAFETY: Exact writable descriptor; even an error may transfer a partial
        // owner, which the guard closes before releasing its callback module.
        let status = unsafe { create(37, 23, 0, 0, 0, 0, 0x100, &mut fixture.span) };
        if status != 0 {
            fixture.close()?;
            return Err(format!("render fixture create {status}").into());
        }
        if fixture.span.owner.is_null()
            || fixture.span.hwnd.is_null()
            || !fixture.span.popup.is_null()
            || (fixture.span.width, fixture.span.height) != (37, 23)
        {
            return Err("invalid render fixture descriptor".into());
        }
        let info = fixture.thread_info()?;
        if info[0] == 0 || info[0] == info[1] || info[0] != info[2] {
            return Err("render fixture must have a separate actual UI owner".into());
        }
        if fixture.stats()?[11] != 1 {
            return Err(
                "ScreenDC fixture did not disable owned-window transitions before first show"
                    .into(),
            );
        }
        Ok(fixture)
    }
}
impl Fixture {
    pub fn hwnd(&self) -> *mut c_void {
        self.span.hwnd
    }
    pub fn thread_info(&self) -> Result<[u64; 4], Box<dyn Error>> {
        type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Matching header signature; the live worker/owner is retained.
        let info: Info = unsafe { self.module.symbol(c"mfs_print_thread_info")? };
        let mut output = [0; 4];
        if unsafe { info(self.span.owner, output.as_mut_ptr(), output.len()) } != 0 {
            return Err("render owner identity query failed".into());
        }
        Ok(output)
    }
    pub fn stats(&self) -> Result<[u64; 12], Box<dyn Error>> {
        type Stats = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: UI owner reads its own counters and replies synchronously.
        let stats: Stats = unsafe { self.module.symbol(c"mfs_print_thread_stats")? };
        let mut output = [0; 12];
        if unsafe { stats(self.span.owner, output.as_mut_ptr(), output.len()) } != 0 {
            return Err("render owner statistics failed".into());
        }
        Ok(output)
    }
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.span.owner.is_null() {
            return Ok(());
        }
        if self.failed_close {
            return Err("render owner previously failed to close; no retry".into());
        }
        // SAFETY: Native close destroys/unregisters on the UI owner, joins the
        // worker and deletes/nulls its context only after completed shutdown.
        let status = unsafe { (self.close)(&mut self.span.owner) };
        if status != 0 || !self.span.owner.is_null() {
            self.failed_close = true;
            // Failed shutdown may retain callback code/context. Keep the module
            // loaded independently of this guard, never free or silently retry.
            std::mem::forget(Arc::clone(&self.module));
            return Err(format!(
                "render owner close {status}, residual owner={}",
                !self.span.owner.is_null()
            )
            .into());
        }
        self.span.hwnd = null_mut();
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.failed_close {
            let _ = self.close();
        }
    }
}
