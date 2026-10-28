// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::print_window::{Overlay, PrintConverter};

#[derive(Default)]
#[repr(C)]
struct FixtureSpan {
    owner: *mut c_void,
    hwnd: *mut c_void,
    popup: *mut c_void,
    width: i32,
    height: i32,
    popup_width: i32,
    popup_height: i32,
}
type CreateFixture =
    unsafe extern "C" fn(i32, i32, i32, i32, i32, i32, u32, *mut FixtureSpan) -> i32;
type CloseFixture = unsafe extern "C" fn(*mut *mut c_void) -> i32;
type Stats = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
type OverlayImage =
    unsafe extern "C" fn(*mut c_void, *mut c_void, i32, i32, i32, i32, i32, i32) -> i32;

pub struct ConversionReference {
    pub image: Result<Image, i32>,
    pub calls: u64,
    pub baseline: u64,
    pub at_conversion: [u64; 2],
    pub after_capture: u64,
    pub image_owners: u64,
    pub overflow: u64,
    pub bgr_allocations: u64,
    pub bgr_releases: u64,
    pub bgr_bytes: u64,
    pub bgr_freed_bytes: u64,
    pub allocator_errors: u64,
}

impl Adapter {
    pub fn print_conversion_reference(
        &self,
        hwnd: *mut c_void,
        fail_at: u32,
    ) -> Result<ConversionReference, Box<dyn Error>> {
        type Reference =
            unsafe extern "C" fn(*mut c_void, u32, *mut ImageSpan, *mut u64, usize) -> i32;
        // SAFETY: Exact private print-parity.h signature, with the module held
        // through the source exception boundary and image adoption.
        let reference: Reference =
            unsafe { self.module.symbol(c"mfs_print_conversion_reference")? };
        let mut image = ImageSpan::default();
        let mut observations = [0_u64; 12];
        // SAFETY: Fully initialized writable outputs of their stated capacity.
        let status = unsafe {
            reference(
                hwnd,
                fail_at,
                &mut image,
                observations.as_mut_ptr(),
                observations.len(),
            )
        };
        if observations[11] & 2 != 0 {
            // An unexpected native BGR allocation still references the probe
            // allocator's code/data. Preserve the module before reporting the
            // failed ownership assertion; never unload a possible late release.
            std::mem::forget(Arc::clone(&self.module));
        }
        Ok(ConversionReference {
            image: self.adopt(status, image),
            calls: observations[0],
            baseline: observations[1],
            at_conversion: [observations[2], observations[3]],
            after_capture: observations[4],
            image_owners: observations[5],
            overflow: observations[6],
            bgr_allocations: observations[7],
            bgr_releases: observations[8],
            bgr_bytes: observations[9],
            bgr_freed_bytes: observations[10],
            allocator_errors: observations[11],
        })
    }
    pub fn print_intersection(
        &self,
        canvas: (i32, i32),
        popup: (i32, i32),
        offset: (i64, i64),
    ) -> Result<Option<Overlay>, Box<dyn Error>> {
        type Intersect = unsafe extern "C" fn(i32, i32, i32, i32, i64, i64, *mut i32, usize) -> i32;
        // SAFETY: Exact private header signature; OpenCV operates on int Rects
        // after validating source offsets, and writes only six supplied outputs.
        let intersect: Intersect = unsafe { self.module.symbol(c"mfs_print_parity_intersection")? };
        let mut output = [0_i32; 6];
        let status = unsafe {
            intersect(
                canvas.0,
                canvas.1,
                popup.0,
                popup.1,
                offset.0,
                offset.1,
                output.as_mut_ptr(),
                output.len(),
            )
        };
        match status {
            0 => Ok(Some(Overlay {
                source_x: output[0],
                source_y: output[1],
                destination_x: output[2],
                destination_y: output[3],
                width: output[4],
                height: output[5],
            })),
            1 => {
                assert_eq!(output, [0; 6]);
                Ok(None)
            }
            _ => Err("source geometry is not representable".into()),
        }
    }
    pub fn print_reference(&self, hwnd: *mut c_void) -> Result<Image, i32> {
        // SAFETY: Exact private C signature; module stays live through adoption.
        let reference: Reference =
            unsafe { self.module.symbol(c"mfs_print_parity_reference") }.map_err(|_| -1)?;
        let mut span = ImageSpan::default();
        // SAFETY: Original class validates opaque HWND and writes only the span.
        let status = unsafe { reference(hwnd, &mut span) };
        self.adopt(status, span)
    }

    pub fn print_fixture(
        &self,
        popup: (i32, i32),
        offset: (i32, i32),
        mode: u32,
    ) -> Result<Fixture, Box<dyn Error>> {
        self.create_print_fixture(popup, offset, mode, false)
    }
    pub fn print_child_fixture(
        &self,
        popup: (i32, i32),
        offset: (i32, i32),
        mode: u32,
    ) -> Result<Fixture, Box<dyn Error>> {
        self.create_print_fixture(popup, offset, mode, true)
    }
    fn create_print_fixture(
        &self,
        popup: (i32, i32),
        offset: (i32, i32),
        mode: u32,
        child_target: bool,
    ) -> Result<Fixture, Box<dyn Error>> {
        // SAFETY: Every signature follows print-parity.h; resolve close before
        // creating any callback/context so partial failures remain owned.
        let (layout, create, close, stats): (Layout, CreateFixture, CloseFixture, Stats) = unsafe {
            (
                self.module.symbol(c"mfs_print_parity_layout")?,
                self.module.symbol(if child_target {
                    c"mfs_print_thread_child_fixture"
                } else {
                    c"mfs_print_thread_fixture"
                })?,
                self.module.symbol(c"mfs_print_thread_close")?,
                self.module.symbol(c"mfs_print_thread_stats")?,
            )
        };
        let mut actual = [0_u64; 9];
        // SAFETY: Writable exact-width layout query with stated capacity.
        if unsafe { layout(actual.as_mut_ptr(), actual.len()) } != 0 {
            return Err("PrintWindow fixture layout query failed".into());
        }
        let expected = [
            size_of::<FixtureSpan>(),
            align_of::<FixtureSpan>(),
            offset_of!(FixtureSpan, owner),
            offset_of!(FixtureSpan, hwnd),
            offset_of!(FixtureSpan, popup),
            offset_of!(FixtureSpan, width),
            offset_of!(FixtureSpan, height),
            offset_of!(FixtureSpan, popup_width),
            offset_of!(FixtureSpan, popup_height),
        ]
        .map(|v| u64::try_from(v).unwrap());
        if actual != expected {
            return Err("PrintWindow C/Rust layout differs".into());
        }
        let mut fixture = Fixture {
            module: Arc::clone(&self.module),
            span: FixtureSpan::default(),
            close,
            stats,
            failed_close: false,
        };
        // SAFETY: Fully writable descriptor. Even an error may transfer a partial
        // native owner, already contained by this thread-affine fixture guard.
        let status = unsafe {
            create(
                37,
                23,
                popup.0,
                popup.1,
                offset.0,
                offset.1,
                mode,
                &mut fixture.span,
            )
        };
        if status != 0 {
            fixture.close()?;
            return Err(format!("PrintWindow fixture create: {status}").into());
        }
        if fixture.span.owner.is_null()
            || fixture.span.hwnd.is_null()
            || (fixture.span.width, fixture.span.height) != (37, 23)
            || (mode != 0 && (fixture.span.popup_width, fixture.span.popup_height) != popup)
        {
            return Err("invalid PrintWindow fixture descriptor".into());
        }
        let thread = fixture.thread_info()?;
        if thread[0] == 0 || thread[0] == thread[1] || thread[0] != thread[2] {
            return Err("fixture must have a distinct real UI/window owner".into());
        }
        Ok(fixture)
    }
}

impl PrintConverter for Converter<'_> {
    fn overlay(
        &mut self,
        destination: &mut Image,
        source: &Image,
        rectangle: Overlay,
    ) -> Result<(), i32> {
        if !self.adapter.module.same_instance(&destination.module)
            || !self.adapter.module.same_instance(&source.module)
        {
            return Err(-1);
        }
        // SAFETY: Exact private header signature, from this same retained DLL.
        let overlay: OverlayImage =
            unsafe { self.adapter.module.symbol(c"mfs_print_parity_overlay") }.map_err(|_| -1)?;
        // SAFETY: Both opaque Mat owners originate in this module; destination
        // is exclusively borrowed and the source is independently owned/read-only.
        let status = unsafe {
            overlay(
                destination.span.owner,
                source.span.owner,
                rectangle.source_x,
                rectangle.source_y,
                rectangle.destination_x,
                rectangle.destination_y,
                rectangle.width,
                rectangle.height,
            )
        };
        match status {
            0 => Ok(()),
            status => Err(status),
        }
    }
}

pub(crate) struct Fixture {
    // Native windows retain callback code outside Rust. An owned module lease
    // survives Adapter destruction, and forgetting this guard leaks the lease
    // together with its native context rather than unloading a live WindowProc.
    module: Arc<Module>,
    span: FixtureSpan,
    close: CloseFixture,
    stats: Stats,
    failed_close: bool,
}
impl Fixture {
    pub fn prepare_pseudo(&self, layered: bool) -> Result<(), Box<dyn Error>> {
        type Prepare = unsafe extern "C" fn(*mut c_void, u32) -> i32;
        // SAFETY: The command creates/activates its anchor on the actual UI
        // owner, then minimizes/styles this fixture without borrowing pointers.
        let prepare: Prepare = unsafe { self.module.symbol(c"mfs_print_thread_prepare")? };
        let status = unsafe { prepare(self.span.owner, u32::from(layered)) };
        if status != 0 {
            return Err(format!("UI-owner pseudo preparation failed: {status}").into());
        }
        Ok(())
    }
    pub fn thread_info(&self) -> Result<[u64; 4], Box<dyn Error>> {
        type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Exact print-thread.h signature and four writable u64 outputs.
        let info: Info = unsafe { self.module.symbol(c"mfs_print_thread_info")? };
        let mut result = [0; 4];
        if unsafe { info(self.span.owner, result.as_mut_ptr(), result.len()) } != 0 {
            return Err("PrintWindow thread identity query failed".into());
        }
        Ok(result)
    }
    pub fn activate(&self) -> Result<(), Box<dyn Error>> {
        type Activate = unsafe extern "C" fn(*mut c_void) -> i32;
        // SAFETY: Sends one bounded command to the live fixture UI owner.
        // Windows foreground policy remains observable; a rejection is an error.
        let activate: Activate = unsafe { self.module.symbol(c"mfs_print_thread_activate")? };
        let status = unsafe { activate(self.span.owner) };
        if status != 0 {
            type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
            // SAFETY: Matching native observation for this same fixture command.
            let info: Info = unsafe { self.module.symbol(c"mfs_print_thread_activation_info")? };
            let mut observed = [0; 5];
            let queried = unsafe { info(self.span.owner, observed.as_mut_ptr(), observed.len()) };
            let kind = if status == 5
                && queried == 0
                && observed[0] == 0
                && observed[1] == observed[3]
                && observed[2] != observed[3]
            {
                "MFS_MISSING_ENVIRONMENT:foreground_control"
            } else {
                "fixture foreground assertion failed"
            };
            return Err(format!("{kind}: status={status}, info_status={queried}, [SetForegroundWindow, active, foreground, target, foreground_pid]={observed:?}").into());
        }
        Ok(())
    }
    pub fn hwnd(&self) -> *mut c_void {
        self.span.hwnd
    }
    pub fn stats(&self) -> Result<[u64; 5], Box<dyn Error>> {
        let mut stats = [0; 5];
        // SAFETY: Live serial caller owner and exactly five writable outputs;
        // the native command reads Pattern counters only on the UI thread.
        if unsafe { (self.stats)(self.span.owner, stats.as_mut_ptr(), stats.len()) } != 0 {
            return Err("PrintWindow fixture stats failed".into());
        }
        Ok(stats)
    }
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        if self.span.owner.is_null() {
            return Ok(());
        }
        if self.failed_close {
            return Err("PrintWindow fixture previous close failed".into());
        }
        // SAFETY: Owns a possibly partial UI worker and native context. Native
        // close destroys/unregisters on that worker, joins, then deletes/nulls.
        let status = unsafe { (self.close)(&mut self.span.owner) };
        if status != 0 {
            self.failed_close = true;
            return Err(format!("PrintWindow fixture close: {status}").into());
        }
        assert!(self.span.owner.is_null());
        Ok(())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Native callback/context may remain registered. Retain its code
            // explicitly rather than unloading a live WindowProc; report failure
            // without retrying or claiming cleanup/fixture acceptance.
            std::mem::forget(Arc::clone(&self.module));
            eprintln!("{error}; partial fixture/module retained for safety");
        }
    }
}

#[test]
#[ignore = "explicit native window callback code/context ownership"]
fn keeps_callback_module_until_window_registration_is_closed() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let module = Arc::downgrade(&adapter.module);
    let mut fixture = adapter.print_child_fixture((25, 17), (5, 3), 1)?;
    let thread = fixture.thread_info()?;
    assert_ne!(thread[0], thread[1]);
    assert_eq!(thread[0], thread[2]);
    println!("PrintWindow UI owner/caller/window/process identity: {thread:?}");
    drop(adapter);
    assert!(
        module.upgrade().is_some(),
        "native window registration must retain its code"
    );
    assert_eq!(fixture.stats()?[4], 0);
    // DestroyWindow invokes this DLL's WindowProc for all three fixture windows;
    // successful close drains them and unregisters the class before data release.
    fixture.close()?;
    drop(fixture);
    assert!(
        module.upgrade().is_none(),
        "closed fixture must release its module lease"
    );
    Ok(())
}

#[test]
#[ignore = "explicit partial native UI-worker construction and module release"]
fn closes_partial_fixture_workers_before_releasing_callback_code() -> Result<(), Box<dyn Error>> {
    let adapter = Adapter::load()?;
    let module = Arc::downgrade(&adapter.module);
    let baseline = adapter.objects();
    // A worker/queue is created, then the original fixture rejects the negative
    // popup size. Its retained partial owner must still be closed and joined.
    assert!(adapter.print_fixture((-1, 17), (5, 3), 1).is_err());
    assert_eq!(adapter.objects(), baseline);
    drop(adapter);
    assert!(
        module.upgrade().is_none(),
        "failed creation must release its joined worker module"
    );
    Ok(())
}
