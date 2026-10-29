// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit test-only native error controls. The C header defines this private
//! ABI; live calls verify FD arguments, counters, errno and the loaded owner.
use super::*;
use std::ffi::c_void;

type Arm = unsafe extern "C" fn(i32) -> i32;
type Count = unsafe extern "C" fn() -> u32;
type ErrorCode = unsafe extern "C" fn() -> i32;

pub(super) struct FdFault {
    arm: Arm,
    attempts: Count,
    rejections: Count,
    native_error: ErrorCode,
    control_error: ErrorCode,
}
fn symbol(name: &CStr) -> *mut c_void {
    // SAFETY: Static initialized C symbol name; explicit LD_PRELOAD retains the
    // library for this isolated test process. No pointer outlives that process.
    let pointer = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    assert!(!pointer.is_null(), "missing native fault control {name:?}");
    // SAFETY: dladdr initializes this plain C record on successful lookup.
    let mut info: libc::Dl_info = unsafe { mem::zeroed() };
    // SAFETY: Pointer is a live dynamically loaded symbol and output is valid.
    assert_ne!(unsafe { libc::dladdr(pointer, &mut info) }, 0);
    assert!(!info.dli_fname.is_null());
    // SAFETY: dladdr returns an initialized NUL-terminated owner filename.
    let owner = unsafe { CStr::from_ptr(info.dli_fname) }.to_str().unwrap();
    assert_eq!(
        fs::canonicalize(owner).unwrap(),
        fs::canonicalize(env::var("MFS_P1_PIPEWIRE_FD_FAULT_LIBRARY").unwrap()).unwrap()
    );
    pointer
}
impl FdFault {
    pub fn new() -> Self {
        // SAFETY: Each non-null symbol has the exact fixed-width signature in
        // fd-fault.h. Owner identity is checked, and the preload remains loaded.
        unsafe {
            Self {
                arm: mem::transmute::<*mut c_void, Arm>(symbol(c"mfs_pw_fd_fault_arm")),
                attempts: mem::transmute::<*mut c_void, Count>(symbol(c"mfs_pw_fd_fault_attempts")),
                rejections: mem::transmute::<*mut c_void, Count>(symbol(
                    c"mfs_pw_fd_fault_rejections",
                )),
                native_error: mem::transmute::<*mut c_void, ErrorCode>(symbol(
                    c"mfs_pw_fd_fault_native_error",
                )),
                control_error: mem::transmute::<*mut c_void, ErrorCode>(symbol(
                    c"mfs_pw_fd_fault_control_error",
                )),
            }
        }
    }
    pub fn arm(&self, fd: i32) {
        // SAFETY: The calling channel owns this fresh socket and no native
        // context/loop is running yet. Only its exact add_io may be rejected.
        assert_eq!(unsafe { (self.arm)(fd) }, 0);
    }
    pub fn check(&self, attempts: u32) {
        // SAFETY: Loaded header-declared functions read calling-thread scalars;
        // they retain no callback/input and allocate no test control state.
        unsafe {
            assert_eq!((self.attempts)(), attempts);
            assert_eq!((self.rejections)(), 1);
            assert_eq!((self.native_error)(), libc::ENOMEM);
            assert_eq!((self.control_error)(), 0);
        }
    }
}
pub(super) fn socket_cookie(fd: i32) -> u64 {
    let mut cookie = 0_u64;
    let expected = libc::socklen_t::try_from(mem::size_of_val(&cookie)).unwrap();
    let mut length = expected;
    // SAFETY: The FD is only observed. Valid output storage and its byte length
    // cover this synchronous Linux socket identity query; no ownership changes.
    assert_eq!(
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_COOKIE,
                ptr::from_mut(&mut cookie).cast(),
                &mut length,
            )
        },
        0
    );
    assert_eq!(length, expected);
    cookie
}
