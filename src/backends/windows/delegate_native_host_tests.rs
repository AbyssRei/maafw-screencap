// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Acceptance-only Rust manager factory consumed through the actual C++ ABI.
use super::*;
use crate::backends::windows::{
    manager::{
        ConnectFailure, FailureClass, Image as ManagerImage, NativeManager, Observation, Outcome,
        Publication,
    },
    parity_adapter::Image as CpuImage,
};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
};
#[repr(C)]
pub(super) struct Table {
    pub user: *mut c_void,
    pub create: unsafe extern "C" fn(*mut c_void, usize, u64, *mut *mut c_void) -> i32,
    pub destroy: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
    pub connect: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut i32) -> i32,
    pub capture: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut Span, *mut i32) -> i32,
    pub inactive: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut i32) -> i32,
}
type Install = unsafe extern "C" fn(*const Table, usize) -> i32;
struct Stats {
    thread: u32,
    created: Cell<u32>,
    destroyed: Cell<u32>,
    conversions: Cell<usize>,
    captures: Cell<u32>,
    inactive: Cell<u32>,
}
struct Context {
    output: ManagerImage<CpuImage>,
    manager: NativeManager<CpuImage>,
    adapter: Adapter,
}
fn contained(call: impl FnOnce() -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or(90)
}
/// # Safety
/// pointer is the still-owned Stats from this synchronous registered table.
unsafe fn stats<'a>(pointer: *mut c_void) -> &'a Stats {
    // SAFETY: Registration owns a stable Box; every callback completes before
    // unregister. Caller-channel checks precede access to other mutable state.
    let stats = unsafe { &*pointer.cast::<Stats>() };
    assert_eq!(
        crate::platform::windows::manager::probe::thread(),
        stats.thread
    );
    stats
}
unsafe extern "C" fn create(
    user: *mut c_void,
    hwnd: usize,
    method: u64,
    output: *mut *mut c_void,
) -> i32 {
    contained(|| {
        // SAFETY: Actual facade passes registered user and initialized output;
        // all callbacks are synchronous and bound to the original caller.
        let stats = unsafe { stats(user) };
        unsafe { *output = std::ptr::null_mut() };
        let Ok(adapter) = Adapter::load() else {
            return 2;
        };
        let Ok(context) = Box::try_new(Context {
            output: ManagerImage::Empty,
            manager: NativeManager::new(std::ptr::without_provenance_mut(hwnd), method),
            adapter,
        }) else {
            return 3;
        };
        unsafe { *output = Box::into_raw(context).cast() };
        stats.created.set(stats.created.get() + 1);
        0
    })
}
unsafe extern "C" fn destroy(user: *mut c_void, context: *mut c_void) -> i32 {
    contained(|| {
        // SAFETY: Unique context returned by create, consumed exactly once by
        // the facade destructor before its unit module lease is released.
        let stats = unsafe { stats(user) };
        drop(unsafe { Box::from_raw(context.cast::<Context>()) });
        stats.destroyed.set(stats.destroyed.get() + 1);
        0
    })
}
unsafe extern "C" fn connect(user: *mut c_void, context: *mut c_void, output: *mut i32) -> i32 {
    contained(|| {
        // SAFETY: Caller retains registered context; native facade serializes
        // each call and exclusively borrows it for this callback only.
        let stats = unsafe { stats(user) };
        let context = unsafe { &mut *context.cast::<Context>() };
        let mut converter = context.adapter.converter(0);
        let report = context.manager.connect_native(&mut converter);
        stats
            .conversions
            .set(stats.conversions.get() + converter.conversions);
        if matches!(
            report.failure,
            Some(ConnectFailure::Construction(..) | ConnectFailure::Escaped(..))
        ) {
            return 2;
        }
        unsafe { *output = i32::from(report.failure.is_none()) };
        0
    })
}
unsafe extern "C" fn capture(
    user: *mut c_void,
    context: *mut c_void,
    span: *mut Span,
    output: *mut i32,
) -> i32 {
    contained(|| {
        // SAFETY: As above; returned bytes borrow Context.output until the next
        // operation. The provider copies them before this native call returns.
        let stats = unsafe { stats(user) };
        let context = unsafe { &mut *context.cast::<Context>() };
        let mut converter = context.adapter.converter(0);
        let result = context
            .manager
            .screencap(&mut converter, &mut context.output);
        stats
            .conversions
            .set(stats.conversions.get() + converter.conversions);
        stats.captures.set(stats.captures.get() + 1);
        if matches!(
            result,
            Publication::Called(Observation {
                outcome: Outcome::Failure(FailureClass::Exception | FailureClass::Safety, _),
                ..
            })
        ) {
            return 2;
        }
        let success = matches!(
            result,
            Publication::Called(Observation {
                outcome: Outcome::Pixels | Outcome::Empty,
                ..
            })
        );
        let borrowed = match &context.output {
            ManagerImage::Empty => Span::default(),
            ManagerImage::Pixels(image) => Span {
                owner: std::ptr::null_mut(),
                data: image.bytes().as_ptr(),
                length: image.bytes().len(),
                width: image.dimensions().0,
                height: image.dimensions().1,
                stride: image.stride(),
            },
        };
        unsafe {
            *span = borrowed;
            *output = i32::from(success)
        };
        0
    })
}
unsafe extern "C" fn inactive(user: *mut c_void, context: *mut c_void, output: *mut i32) -> i32 {
    contained(|| {
        // SAFETY: Unique synchronous context borrow on its verified channel.
        let stats = unsafe { stats(user) };
        let context = unsafe { &mut *context.cast::<Context>() };
        context.manager.inactive_native();
        stats.inactive.set(stats.inactive.get() + 1);
        unsafe { *output = 1 };
        0
    })
}
pub(super) struct Guard<'a> {
    stats: Option<Box<Stats>>,
    install: Install,
    _ops: &'a Ops,
}
impl<'a> Guard<'a> {
    pub(super) fn new(ops: &'a Ops) -> Self {
        let mut stats = Box::try_new(Stats {
            thread: crate::platform::windows::manager::probe::thread(),
            created: Cell::new(0),
            destroyed: Cell::new(0),
            conversions: Cell::new(0),
            captures: Cell::new(0),
            inactive: Cell::new(0),
        })
        .unwrap();
        // SAFETY: Header/layout verified by Ops; stable Box is kept through
        // unregister and all synchronous callback users/contexts must end first.
        let install: Install = unsafe { ops.module.symbol(c"mfs_dn_host_install").unwrap() };
        let table = Table {
            user: (&mut *stats as *mut Stats).cast(),
            create,
            destroy,
            connect,
            capture,
            inactive,
        };
        assert_eq!(unsafe { install(&table, size_of::<Table>()) }, 0);
        Self {
            stats: Some(stats),
            install,
            _ops: ops,
        }
    }
    pub(super) fn verify(&self, units: u32, captures: u32, inactive: u32) {
        let stats = self.stats.as_ref().unwrap();
        assert_eq!(stats.created.get(), units);
        assert_eq!(stats.destroyed.get(), units);
        assert_eq!(stats.captures.get(), captures);
        assert_eq!(stats.inactive.get(), inactive);
        assert_eq!(
            stats.conversions.get(),
            usize::try_from(units * 2 + captures).unwrap()
        );
    }
    pub(super) fn counts(&self) -> [u64; 5] {
        let stats = self.stats.as_ref().unwrap();
        [
            u64::from(stats.created.get()),
            u64::from(stats.destroyed.get()),
            u64::try_from(stats.conversions.get()).unwrap(),
            u64::from(stats.captures.get()),
            u64::from(stats.inactive.get()),
        ]
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        // SAFETY: Exact synchronous unregister; rejects any remaining context.
        // An unproved cleanup deliberately retains user storage; never free it
        // beneath still-registered callbacks. This is a failure, not a pass.
        let code = unsafe { (self.install)(std::ptr::null(), 0) };
        if code != 0 {
            std::mem::forget(self.stats.take());
            std::mem::forget(self._ops.module.clone());
            panic!("native host unregister failed: {code}");
        }
    }
}
