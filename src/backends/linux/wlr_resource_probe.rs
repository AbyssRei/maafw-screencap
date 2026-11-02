// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Libtest-only, same-thread syscall scope. Never compiled into the library.
use std::{cell::Cell, marker::PhantomData, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct Observation {
    pub scopes: u32,
    pub returned: bool,
    pub error: i32,
    pub before: libc::rlimit,
    pub limited: libc::rlim_t,
    pub after: libc::rlimit,
}
thread_local! {
    static ARMED: Cell<Option<Observation>> = const { Cell::new(None) };
}
fn current() -> libc::rlimit {
    // SAFETY: Initialized writable POD, valid selector; no fd is opened.
    let mut value = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut value) } != 0 {
        std::process::abort();
    }
    value
}
fn same(a: libc::rlimit, b: libc::rlimit) -> bool {
    (a.rlim_cur, a.rlim_max) == (b.rlim_cur, b.rlim_max)
}
pub(super) struct Arm {
    armed: bool,
    _caller: PhantomData<Rc<()>>,
}
pub(super) fn arm(before: libc::rlimit) -> Arm {
    assert!(same(current(), before) && before.rlim_cur > 0);
    ARMED.with(|slot| {
        assert!(slot.get().is_none(), "nested memfd resource probe");
        slot.set(Some(Observation {
            scopes: 0,
            returned: false,
            error: 0,
            before,
            limited: before.rlim_cur,
            after: before,
        }));
    });
    Arm {
        armed: true,
        _caller: PhantomData,
    }
}
impl Arm {
    pub fn finish(mut self) -> Observation {
        let value = ARMED.with(|slot| slot.take().expect("armed memfd probe"));
        self.armed = false;
        value
    }
}
impl Drop for Arm {
    fn drop(&mut self) {
        if self.armed {
            ARMED.with(|slot| slot.set(None));
        }
    }
}
struct Limit {
    before: libc::rlimit,
    active: bool,
}
impl Drop for Limit {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        // SAFETY: Restore only this owned test process's original soft/hard
        // limits before returning to the capture callback or unwinding it.
        if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &self.before) } != 0 {
            std::process::abort();
        }
        let after = current();
        if !same(after, self.before) {
            std::process::abort();
        }
        ARMED.with(|slot| {
            let Some(mut value) = slot.get() else {
                std::process::abort();
            };
            value.after = after;
            slot.set(Some(value));
        });
    }
}
pub(super) fn at_memfd<T>(call: impl FnOnce() -> rustix::io::Result<T>) -> rustix::io::Result<T> {
    let Some(mut value) = ARMED.with(Cell::get) else {
        return call();
    };
    assert_eq!(value.scopes, 0, "one syscall scope per armed capture");
    assert!(same(current(), value.before));
    let mut guard = Limit {
        before: value.before,
        active: false,
    };
    let zero = libc::rlimit {
        rlim_cur: 0,
        rlim_max: value.before.rlim_max,
    };
    // SAFETY: Only the owned serial test process's soft limit changes. The
    // guard is already installed and restores after the real call, on unwind
    // too. Nothing is dispatched or waited here; caller UUID is already built.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &zero) }, 0);
    guard.active = true;
    let limited = current();
    assert!(same(limited, zero));
    value.scopes = 1;
    value.limited = limited.rlim_cur;
    ARMED.with(|slot| slot.set(Some(value)));
    let result = call();
    value.returned = true;
    value.error = result
        .as_ref()
        .err()
        .map_or(0, |error| error.raw_os_error());
    ARMED.with(|slot| slot.set(Some(value)));
    drop(guard);
    result
}

#[test]
#[ignore = "serial owned-process real NOFILE/syscall/unwind controls; no capture"]
fn restores_real_memfd_scope_and_leaves_unarmed_calls_unchanged() {
    let before = current();
    let fd = || rustix::fs::memfd_create(c"mfs-wlr-scope-control", rustix::fs::MemfdFlags::empty());
    drop(at_memfd(fd).unwrap());
    assert!(same(current(), before));
    let armed = arm(before);
    let poll_error = Cell::new(0);
    let result = at_memfd(|| {
        let mut ignored = libc::pollfd {
            fd: -1,
            events: 0,
            revents: 0,
        };
        // SAFETY: One initialized ignored descriptor, exact length and zero
        // timeout. Linux rejects nfds1 above the current NOFILE soft0 limit;
        // this separate control observes errno before the actual memfd call.
        let returned = unsafe { libc::poll(&mut ignored, 1, 0) };
        poll_error.set(if returned == -1 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        } else {
            0
        });
        fd()
    });
    assert_eq!(result.unwrap_err().raw_os_error(), libc::EMFILE);
    assert_eq!(poll_error.get(), libc::EINVAL);
    let value = armed.finish();
    assert_eq!(
        (value.scopes, value.returned, value.error, value.limited),
        (1, true, libc::EMFILE, 0)
    );
    assert!(same(value.before, before) && same(value.after, before) && same(current(), before));
    let armed = arm(before);
    let result = std::panic::catch_unwind(|| {
        at_memfd::<std::os::fd::OwnedFd>(|| panic!("owned probe unwind control"))
    });
    assert!(result.is_err());
    let value = armed.finish();
    assert_eq!((value.scopes, value.returned, value.limited), (1, false, 0));
    assert!(same(value.after, before) && same(current(), before));
    drop(at_memfd(fd).unwrap());
    assert!(same(current(), before));
    println!(
        "native Wlr syscall scope controls real_EMFILE=1 poll_nofile_EINVAL=1 unarmed_native=2 unwind_restored=1 capture=0 passed"
    );
}
