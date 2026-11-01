// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Main-thread child entry only. No capture implementation or SDK host policy.
use super::same_binary_protocol as protocol;
use std::{
    ffi::{CString, c_char, c_void},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

pub(crate) fn child() -> bool {
    std::env::var("MFS_P1_MACOS_SAME_BINARY_CHILD").as_deref() == Ok("1")
}
pub(crate) fn run() -> ! {
    assert_eq!(
        std::env::var("MFS_P1_MACOS_MAIN_THREAD_HOST").as_deref(),
        Ok("1")
    );
    let marker = objc2::MainThreadMarker::new().expect("scene must enter on process main thread");
    let _ = marker;
    let mode = std::env::var("MFS_P1_MACOS_SAME_BINARY_MODE")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(mode <= 1);
    let root = PathBuf::from(std::env::var_os("MFS_P1_MACOS_SAME_BINARY_SCENE_ROOT").unwrap());
    let library = PathBuf::from(std::env::var_os("MFS_P1_MACOS_SAME_BINARY_SCENE_MODULE").unwrap());
    assert!(root.is_absolute() && library.is_absolute() && root.is_dir());
    let native = CString::new(library.as_os_str().as_bytes()).unwrap();
    // SAFETY: Explicit prepared absolute image; no global symbol publication.
    let handle = unsafe { libc::dlopen(native.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    assert!(!handle.is_null(), "prepared scene image did not load");
    // Deliberately retain this one dlopen reference until this child exits.
    // Fixture query completion is not proof every native block has returned.
    // No callback can later execute in an explicitly unloaded scene image.
    let version = unsafe {
        symbol::<unsafe extern "C" fn() -> u32>(handle, c"mfs_mac_same_binary_scene_version")
    };
    let run = unsafe {
        symbol::<unsafe extern "C" fn(u32, *const c_char, *const c_char) -> i32>(
            handle,
            c"mfs_mac_same_binary_scene_run",
        )
    };
    // SAFETY: Exact private header version getter from the retained image.
    assert_eq!(unsafe { version() }, 1);
    // SAFETY: dladdr borrows its static filename while the image is retained.
    let mut info = unsafe { std::mem::zeroed::<libc::Dl_info>() };
    assert_ne!(unsafe { libc::dladdr(run as *const c_void, &mut info) }, 0);
    assert!(!info.dli_fname.is_null());
    let loaded = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
    assert_eq!(
        std::fs::canonicalize(Path::new(std::ffi::OsStr::from_bytes(loaded.to_bytes()))).unwrap(),
        std::fs::canonicalize(&library).unwrap()
    );
    let executable = std::env::current_exe().unwrap();
    std::fs::write(
        root.join("same-binary-host.txt"),
        protocol::identity(
            &executable,
            std::process::id(),
            mode,
            "producer",
            Some(&library),
        )
        .unwrap(),
    )
    .unwrap();
    let program = CString::new(executable.as_os_str().as_bytes()).unwrap();
    let directory = CString::new(root.as_os_str().as_bytes()).unwrap();
    // SAFETY: Private header signature; both strings live throughout the
    // synchronous main-thread fixture. The C++ boundary contains exceptions.
    let code = unsafe { run(mode, program.as_ptr(), directory.as_ptr()) };
    if code == 0 {
        let stopped =
            protocol::fields(&std::fs::read_to_string(root.join("stopped.txt")).unwrap()).unwrap();
        assert_eq!(stopped.len(), 1);
        assert_eq!(protocol::scalar(&stopped["window_closed"]).unwrap(), 1);
    }
    std::fs::write(root.join("same-binary-result.txt"),format!("schema=1\nmode={mode}\npid={}\nexit={code}\nmain_thread=1\nmodule_retained_until_process_exit=1\n",std::process::id())).unwrap();
    std::process::exit(code);
}
unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &std::ffi::CStr) -> T {
    // SAFETY: Live selected module and a static terminated symbol name.
    let pointer = unsafe { libc::dlsym(handle, name.as_ptr()) };
    assert!(!pointer.is_null(), "prepared scene symbol missing");
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of_val(&pointer));
    // SAFETY: Caller supplies the exact header function-pointer type. POSIX
    // dlsym's callable address representation is checked above.
    unsafe { std::mem::transmute_copy(&pointer) }
}
