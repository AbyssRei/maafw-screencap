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
#[cfg(feature = "windows-screen-dc")]
type QueueGate = unsafe extern "C" fn(*mut c_void, u32, *mut u64, usize) -> i32;
#[cfg(feature = "windows-screen-dc")]
pub struct QueueHold<'a> {
    fixture: &'a mut Fixture,
    control: QueueGate,
    released: bool,
}
#[cfg(feature = "windows-screen-dc")]
impl QueueHold<'_> {
    pub fn hwnd(&self) -> *mut c_void {
        self.fixture.hwnd()
    }
    fn operation(&self, operation: u32) -> Result<[u64; 7], Box<dyn Error>> {
        let mut output = [0; 7];
        // SAFETY: Exact print-thread.h declaration. This direct gate operation
        // never posts a synchronous command to the blocked UI owner. The
        // exclusive fixture borrow retains its context/worker/module and
        // prevents ordinary fixture commands and close until release/drop.
        let status = unsafe {
            (self.control)(
                self.fixture.span.owner,
                operation,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        if status != 0 {
            return Err(
                format!("native queue gate {operation}: {status}, state={output:?}").into(),
            );
        }
        Ok(output)
    }
    pub fn state(&self) -> Result<[u64; 7], Box<dyn Error>> {
        self.operation(1)
    }
    pub fn release(mut self) -> Result<[u64; 7], Box<dyn Error>> {
        self.released = true;
        self.operation(2)
    }
}
#[cfg(feature = "windows-screen-dc")]
impl Drop for QueueHold<'_> {
    fn drop(&mut self) {
        if !self.released {
            self.released = true;
            let _ = self.operation(2);
        }
    }
}
pub struct Fixture {
    module: Arc<Module>,
    span: Span,
    close: Close,
    failed_close: bool,
}
impl Adapter {
    #[cfg(feature = "windows-screen-dc")]
    pub fn screen_ensure_geometry(&self, window: *mut c_void) -> Result<(), Box<dyn Error>> {
        type Ensure = unsafe extern "C" fn(*mut c_void) -> i32;
        // SAFETY: Exact screen-parity.h signature, live module and owned HWND.
        let ensure: Ensure = unsafe { self.module.symbol(c"mfs_screen_parity_ensure_geometry")? };
        if unsafe { ensure(window) } != 0 {
            return Err("original native window geometry helper failed".into());
        }
        Ok(())
    }
    pub fn screen_fixture(&self) -> Result<Fixture, Box<dyn Error>> {
        self.render_fixture(37, 23)
    }
    pub fn render_fixture(&self, width: i32, height: i32) -> Result<Fixture, Box<dyn Error>> {
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
        let mut observed = [0_u64; 11];
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
            0x200,
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
        let status = unsafe { create(width, height, 0, 0, 0, 0, 0x300, &mut fixture.span) };
        if status != 0 {
            fixture.close()?;
            return Err(format!("render fixture create {status}").into());
        }
        if fixture.span.owner.is_null()
            || fixture.span.hwnd.is_null()
            || !fixture.span.popup.is_null()
            || (fixture.span.width, fixture.span.height) != (width, height)
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
        if fixture.stats()?[12] != 1 {
            return Err("ScreenDC fixture did not request an unobscured topmost scene".into());
        }
        Ok(fixture)
    }
}
impl Fixture {
    #[cfg(feature = "windows-screen-dc")]
    pub fn activate(&self) -> Result<(), Box<dyn Error>> {
        type Activate = unsafe extern "C" fn(*mut c_void) -> i32;
        type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Exact print-thread.h owner command, before any queue gate.
        let activate: Activate = unsafe { self.module.symbol(c"mfs_print_thread_activate")? };
        let info: Info = unsafe { self.module.symbol(c"mfs_print_thread_activation_info")? };
        let status = unsafe { activate(self.span.owner) };
        let mut observed = [0; 5];
        // SAFETY: Direct mutex-protected snapshot of the completed owner command;
        // no window callback borrows this exact initialized output storage.
        let info_status = unsafe { info(self.span.owner, observed.as_mut_ptr(), observed.len()) };
        if status != 0 || info_status != 0 {
            let boundary = if status == 5
                && info_status == 0
                && observed[3] != 0
                && observed[1] == observed[3]
                && observed[2] != observed[3]
            {
                "missing_environment: owned window is active on its UI queue but lacks actual foreground"
            } else {
                "owned fixture activation failed"
            };
            return Err(format!(
                "{boundary}; status={status} info_status={info_status} observation={observed:?}"
            )
            .into());
        }
        Ok(())
    }
    #[cfg(feature = "windows-screen-dc")]
    pub fn hold_queue(&mut self) -> Result<QueueHold<'_>, Box<dyn Error>> {
        // SAFETY: Resolve before arming; an armed gate always has a release
        // guard, including a native error or a Rust assertion unwind.
        let control: QueueGate = unsafe { self.module.symbol(c"mfs_print_thread_queue_gate")? };
        let hold = QueueHold {
            fixture: self,
            control,
            released: false,
        };
        let state = hold.operation(0)?;
        if state[..5] != [1, 1, 0, 0, 0] || state[5] == 0 || state[5] == state[6] {
            return Err(
                format!("native queue gate did not hold a separate owner: {state:?}").into(),
            );
        }
        Ok(hold)
    }
    #[cfg(any(feature = "windows-screen-dc", feature = "windows-desktop-dup-window"))]
    pub fn geometry(&self, input: [i64; 6]) -> Result<[i64; 31], Box<dyn Error>> {
        type Geometry =
            unsafe extern "C" fn(*mut c_void, *const i64, usize, *mut i64, usize) -> i32;
        // SAFETY: Exact print-thread.h declaration and fixed array lengths.
        // The bridge copies the request before posting to its retained UI owner
        // and waits for that owner to finish before writing this output.
        let geometry: Geometry = unsafe { self.module.symbol(c"mfs_print_thread_geometry")? };
        let mut output = [0; 31];
        if unsafe {
            geometry(
                self.span.owner,
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
            )
        } != 0
        {
            return Err("owned native fixture geometry request failed".into());
        }
        Ok(output)
    }
    #[cfg(feature = "windows-desktop-dup")]
    pub fn start_desktop_updates(&mut self) -> Result<(), Box<dyn Error>> {
        type Start = unsafe extern "C" fn(*mut c_void) -> i32;
        // SAFETY: Exact print-thread.h fixture-only owner command signature.
        let start: Start = unsafe { self.module.symbol(c"mfs_desktop_fixture_updates")? };
        let code = unsafe { start(self.span.owner) };
        if code != 0 {
            return Err(format!("desktop update fixture failed: {code}").into());
        }
        Ok(())
    }
    #[cfg(feature = "windows-desktop-dup-window")]
    pub fn start_inside_desktop_updates(&mut self) -> Result<(), Box<dyn Error>> {
        type Start = unsafe extern "C" fn(*mut c_void) -> i32;
        // SAFETY: Exact print-thread.h owner command. The UI owner retains the
        // timer/context until close kills it, destroys the target and joins.
        let start: Start = unsafe { self.module.symbol(c"mfs_desktop_fixture_inside_updates")? };
        let code = unsafe { start(self.span.owner) };
        if code != 0 {
            return Err(format!("in-target desktop update fixture failed: {code}").into());
        }
        Ok(())
    }
    #[cfg(feature = "windows-desktop-dup")]
    pub fn desktop_update_info(&self) -> Result<[u64; 3], Box<dyn Error>> {
        type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        let info: Info = unsafe { self.module.symbol(c"mfs_desktop_fixture_update_info")? };
        let mut values = [0; 3];
        // SAFETY: Live owner, exact three-element initialized output.
        if unsafe { info(self.span.owner, values.as_mut_ptr(), values.len()) } != 0 {
            return Err("desktop update info failed".into());
        }
        Ok(values)
    }
    pub fn hwnd(&self) -> *mut c_void {
        self.span.hwnd
    }
    #[cfg(feature = "windows-frame-pool")]
    pub fn prepare_pseudo(&mut self, layered: bool) -> Result<(), Box<dyn Error>> {
        type Prepare = unsafe extern "C" fn(*mut c_void, u32) -> i32;
        // SAFETY: Exact print-thread.h ABI; setup runs on the actual UI owner.
        let prepare: Prepare = unsafe { self.module.symbol(c"mfs_print_thread_prepare")? };
        let code = unsafe { prepare(self.span.owner, u32::from(layered)) };
        if code != 0 {
            return Err(format!("pseudo fixture preparation failed: {code}").into());
        }
        Ok(())
    }
    #[cfg(feature = "windows-frame-pool")]
    pub fn pseudo_state(&self) -> Result<[u64; 6], Box<dyn Error>> {
        type State = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Exact print-pseudo.h ABI, initialized six-element output.
        let state: State = unsafe { self.module.symbol(c"mfs_print_pseudo_state")? };
        let mut output = [0; 6];
        if unsafe { state(self.span.hwnd, output.as_mut_ptr(), output.len()) } != 0 {
            return Err("pseudo fixture state query failed".into());
        }
        Ok(output)
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
    pub fn stats(&self) -> Result<[u64; 13], Box<dyn Error>> {
        type Stats = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: UI owner reads its own counters and replies synchronously.
        let stats: Stats = unsafe { self.module.symbol(c"mfs_print_thread_stats")? };
        let mut output = [0; 13];
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
