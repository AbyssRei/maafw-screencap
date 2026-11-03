// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Test-only source caller. The module remains loaded until process exit.
use super::*;
use std::ffi::c_char;

type Create = unsafe extern "C" fn(*mut *mut c_void) -> i32;
type Invoke = unsafe extern "C" fn(*mut c_void, u32, *const c_char) -> i32;
type Finish = unsafe extern "C" fn(*mut c_void, *const c_char) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void) -> i32;
type Owners = unsafe extern "C" fn() -> u64;

pub(super) struct Original {
    module: *mut c_void,
    // Native source and CPU Mat owner; deliberately !Send/!Sync. Every access
    // and destruction occurs on this owning libtest caller thread.
    owner: Option<NonNull<c_void>>,
    invoke: Invoke,
    finish: Finish,
    destroy: Destroy,
    owners: Owners,
}
impl Original {
    pub(super) fn open(root: &Path, native_sample: bool) -> Self {
        let library = PathBuf::from(env::var_os("MFS_P1_MACOS_SAME_BINARY_SCENE_MODULE").unwrap());
        assert!(library.is_absolute());
        let native = CString::new(library.as_os_str().as_bytes()).unwrap();
        // SAFETY: Explicit absolute prepared fixture, with local symbol scope.
        let module = unsafe { libc::dlopen(native.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        assert!(!module.is_null());
        // Retain the dlopen reference through process exit. C++ source object
        // destruction is not proof its native completion blocks have drained.
        let version = unsafe {
            symbol::<unsafe extern "C" fn() -> u32>(module, c"mfs_mac_same_binary_original_version")
        };
        let create = unsafe { symbol::<Create>(module, c"mfs_mac_same_binary_original_create") };
        let invoke = unsafe { symbol::<Invoke>(module, c"mfs_mac_same_binary_original_capture") };
        let finish = unsafe { symbol::<Finish>(module, c"mfs_mac_same_binary_original_finish") };
        let destroy = unsafe { symbol::<Destroy>(module, c"mfs_mac_same_binary_original_destroy") };
        let owners = unsafe { symbol::<Owners>(module, c"mfs_mac_same_binary_original_owners") };
        // SAFETY: Exact private-header getters in the retained fixture image.
        assert_eq!(unsafe { version() }, 1);
        if native_sample {
            let observed = unsafe {
                symbol::<unsafe extern "C" fn() -> u32>(
                    module,
                    c"mfs_mac_native_sample_observer_version",
                )
            };
            assert_eq!(unsafe { observed() }, 1);
        }
        assert_eq!(unsafe { owners() }, 0);
        let mut info = unsafe { std::mem::zeroed::<libc::Dl_info>() };
        // SAFETY: Live function symbol; dladdr filename stays valid with image.
        assert_ne!(
            unsafe { libc::dladdr(invoke as *const c_void, &mut info) },
            0
        );
        assert!(!info.dli_fname.is_null());
        let loaded = unsafe { CStr::from_ptr(info.dli_fname) };
        let loaded = Path::new(std::ffi::OsStr::from_bytes(loaded.to_bytes()));
        assert_eq!(
            fs::canonicalize(loaded).unwrap(),
            fs::canonicalize(&library).unwrap()
        );
        fs::write(
            root.join("original-module.txt"),
            format!(
                "schema=1\nmodule_path={}\nmodule_retained_until_process_exit=1\n",
                loaded.to_str().unwrap()
            ),
        )
        .unwrap();
        if native_sample {
            fs::write(
                root.join("original-native-observer-module.txt"),
                format!(
                    "schema=1\nmodule_path={}\nversion=1\nalias_scope=original_class_only\nmodule_retained_until_process_exit=1\n",
                    loaded.to_str().unwrap()
                ),
            )
            .unwrap();
        }
        let mut pointer = std::ptr::null_mut();
        // SAFETY: Writable empty output; created context stays on this thread.
        assert_eq!(unsafe { create(&mut pointer) }, 0);
        let owner = NonNull::new(pointer).expect("original source context");
        assert_eq!(unsafe { owners() }, 1);
        Self {
            module,
            owner: Some(owner),
            invoke,
            finish,
            destroy,
            owners,
        }
    }
    pub(super) fn capture(&self, call: u32, root: &Path) {
        assert_eq!(self.capture_result(call, root), 0);
    }
    pub(super) fn capture_result(&self, call: u32, root: &Path) -> i32 {
        let directory = CString::new(root.as_os_str().as_bytes()).unwrap();
        // SAFETY: Live thread-local context, exact once-only index, synchronous
        // path borrow. Original class returns before the fixture saves bytes.
        unsafe { (self.invoke)(self.owner.unwrap().as_ptr(), call, directory.as_ptr()) }
    }
    /// The callbacks must match T, contain unwinding, and use only independent
    /// CPU owners after their synchronous native sample borrow ends.
    pub(super) unsafe fn install_live_sample<T: Send + Sync + 'static>(
        &self,
        state: Arc<T>,
        visit: unsafe extern "C" fn(*mut c_void, *const c_void, u32) -> i32,
        release: unsafe extern "C" fn(*mut c_void),
    ) -> i32 {
        type Install = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            unsafe extern "C" fn(*mut c_void, *const c_void, u32) -> i32,
            unsafe extern "C" fn(*mut c_void),
        ) -> i32;
        // SAFETY: Exact live-sample.h export in the explicitly selected observed
        // module. Resolve before transferring state so missing exports cannot
        // leak its Arc. Valid release consumes it on every native result.
        let install =
            unsafe { symbol::<Install>(self.module, c"mfs_mac_original_set_live_sample") };
        let context = Arc::into_raw(state).cast_mut().cast();
        unsafe { install(self.owner.unwrap().as_ptr(), context, visit, release) }
    }
    pub(super) fn live_bgr_type(&self) -> u64 {
        // SAFETY: Scalar query from the exact live-sample.h declaration in the
        // retained observation module, compiled against its actual OpenCV.
        let query = unsafe {
            symbol::<unsafe extern "C" fn() -> u64>(self.module, c"mfs_mac_original_live_bgr_type")
        };
        unsafe { query() }
    }
    pub(super) fn finish(mut self, root: &Path) {
        let directory = CString::new(root.as_os_str().as_bytes()).unwrap();
        // SAFETY: Three captures complete; scene closed. The source object is
        // destroyed and its independent CPU Mats saved before context release.
        assert_eq!(
            unsafe { (self.finish)(self.owner.unwrap().as_ptr(), directory.as_ptr()) },
            0
        );
        let owner = self.owner.take().unwrap();
        assert_eq!(unsafe { (self.destroy)(owner.as_ptr()) }, 0);
        assert_eq!(unsafe { (self.owners)() }, 0);
        fs::write(
            root.join("original-owner-closed.txt"),
            b"context_owners=0\nsource_callbacks_drained=0\n",
        )
        .unwrap();
    }
}
impl Drop for Original {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            // SAFETY: Failure cleanup on the owning caller thread; containing
            // module stays loaded independently of this context's lifetime.
            let code = unsafe { (self.destroy)(owner.as_ptr()) };
            if code != 0 {
                eprintln!("original source fixture cleanup failed: {code}");
            }
        }
    }
}
unsafe fn symbol<T: Copy>(module: *mut c_void, name: &CStr) -> T {
    // SAFETY: Retained explicit image/static terminated name; exact header
    // function type is supplied at each call site and checked by native calls.
    let pointer = unsafe { libc::dlsym(module, name.as_ptr()) };
    assert!(!pointer.is_null());
    assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    unsafe { std::mem::transmute_copy(&pointer) }
}
