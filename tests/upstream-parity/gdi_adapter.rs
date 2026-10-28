// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Test-only consumer of native/gdi-parity.h. Layout is verified in the real DLL.
use crate::backends::windows::gdi_types::{BgraConverter, BgraView};
use std::{
    error::Error,
    ffi::{CStr, c_void},
    mem::{align_of, offset_of, size_of},
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
    sync::Arc,
};

#[cfg(feature = "windows-print-window")]
#[path = "print_fault_probe.rs"]
pub(crate) mod print_fault;
#[cfg(feature = "windows-print-window")]
#[path = "print_query_probe.rs"]
pub(crate) mod print_query;
#[cfg(feature = "windows-print-window")]
#[path = "print_window_adapter.rs"]
pub(crate) mod print_window;
#[path = "gdi_probe.rs"]
pub(crate) mod probe;

#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_prototype.rs"));
}

#[derive(Default)]
#[repr(C)]
struct ImageSpan {
    owner: *mut c_void,
    data: *const u8,
    length: usize,
    width: i32,
    height: i32,
    stride: usize,
}

#[derive(Default)]
#[repr(C)]
struct WindowSpan {
    hwnd: *mut c_void,
    width: i32,
    height: i32,
}

type Reference = unsafe extern "C" fn(*mut c_void, *mut ImageSpan) -> i32;
type Convert = unsafe extern "C" fn(*const u8, usize, i32, i32, usize, u32, *mut ImageSpan) -> i32;
type Release = unsafe extern "C" fn(*mut c_void);
type Layout = unsafe extern "C" fn(*mut u64, usize) -> i32;
type OpenWindow = unsafe extern "C" fn(i32, i32, *mut WindowSpan) -> i32;
type Paint = unsafe extern "C" fn(*mut c_void, u32) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void) -> i32;
type Objects = unsafe extern "C" fn() -> u32;

struct Module(native::HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: This owns one loader reference. Every image/adapter retains
        // this Arc until its final native operation has completed.
        let _ = unsafe { native::FreeLibrary(self.0) };
    }
}

impl Module {
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T, Box<dyn Error>> {
        if size_of::<T>() != size_of::<native::FARPROC>() {
            return Err("unexpected native function pointer representation".into());
        }
        // SAFETY: Live loader reference and NUL-terminated name.
        let symbol = unsafe { native::GetProcAddress(self.0, name.as_ptr().cast()) }
            .ok_or("GDI parity symbol missing")?;
        // SAFETY: Caller supplies the exact signature from gdi-parity.h.
        Ok(unsafe { std::mem::transmute_copy(&symbol) })
    }
}

pub(crate) struct Adapter {
    module: Arc<Module>,
    reference: Reference,
    convert: Convert,
    release: Release,
    open_window: OpenWindow,
    paint: Paint,
    destroy: Destroy,
    objects: Objects,
}

impl Adapter {
    pub fn image_owners(&self) -> Result<u64, Box<dyn Error>> {
        type ImageOwners = unsafe extern "C" fn() -> u64;
        // SAFETY: Exact private signature; atomic query has no pointer inputs.
        let owners: ImageOwners = unsafe { self.module.symbol(c"mfs_gdi_parity_image_owners")? };
        Ok(unsafe { owners() })
    }
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = std::env::var_os("MFS_GDI_PARITY_LIBRARY")
            .ok_or("MFS_GDI_PARITY_LIBRARY is required; run cargo xtask upstream-parity gdi")?;
        if !std::path::Path::new(&path).is_absolute() {
            return Err("GDI parity DLL path must be absolute".into());
        }
        let wide: Vec<u16> = path.encode_wide().chain(Some(0)).collect();
        // SAFETY: Explicit absolute NUL-terminated path; restrict dependent DLL
        // discovery to its directory and default safe locations, without changing
        // the process-wide loader search configuration.
        let handle = unsafe { native::LoadLibraryExW(wide.as_ptr(), null_mut(), 0x100 | 0x1000) };
        if handle.is_null() {
            // SAFETY: Read the loader failure before any other native operation.
            let error = unsafe { native::GetLastError() };
            return Err(format!("LoadLibraryExW failed for GDI parity adapter: {error}").into());
        }
        let module = Arc::try_new(Module(handle))?;
        // SAFETY: Every signature below matches gdi-parity.h. The loader owner
        // is kept together with all function pointers, including image releases.
        unsafe {
            let layout: Layout = module.symbol(c"mfs_gdi_parity_layout")?;
            let mut observed = [0_u64; 13];
            if layout(observed.as_mut_ptr(), observed.len()) != 0 {
                return Err("native GDI layout query failed".into());
            }
            let expected = [
                size_of::<ImageSpan>(),
                align_of::<ImageSpan>(),
                offset_of!(ImageSpan, owner),
                offset_of!(ImageSpan, data),
                offset_of!(ImageSpan, length),
                offset_of!(ImageSpan, width),
                offset_of!(ImageSpan, height),
                offset_of!(ImageSpan, stride),
                size_of::<WindowSpan>(),
                align_of::<WindowSpan>(),
                offset_of!(WindowSpan, hwnd),
                offset_of!(WindowSpan, width),
                offset_of!(WindowSpan, height),
            ]
            .map(|value| u64::try_from(value).unwrap());
            if observed != expected {
                return Err("private C/Rust GDI layout mismatch".into());
            }
            Ok(Self {
                reference: module.symbol(c"mfs_gdi_parity_reference")?,
                convert: module.symbol(c"mfs_gdi_parity_convert")?,
                release: module.symbol(c"mfs_gdi_parity_release")?,
                open_window: module.symbol(c"mfs_gdi_parity_window")?,
                paint: module.symbol(c"mfs_gdi_parity_paint")?,
                destroy: module.symbol(c"mfs_gdi_parity_destroy")?,
                objects: module.symbol(c"mfs_gdi_parity_objects")?,
                module,
            })
        }
    }

    fn adopt(&self, status: i32, span: ImageSpan) -> Result<Image, i32> {
        let image = Image {
            span,
            module: Arc::clone(&self.module),
            release: self.release,
        };
        if status != 0 {
            // A failure may not publish a partially-owned output.
            assert!(
                image.span.owner.is_null() && image.span.data.is_null() && image.span.length == 0
            );
            return Err(status);
        }
        let width = usize::try_from(image.span.width).map_err(|_| -1)?;
        let height = usize::try_from(image.span.height).map_err(|_| -1)?;
        let row = width.checked_mul(3).ok_or(-1)?;
        let readable = height
            .checked_sub(1)
            .and_then(|rows| rows.checked_mul(image.span.stride))
            .and_then(|preceding| preceding.checked_add(row))
            .ok_or(-1)?;
        if image.span.owner.is_null()
            || image.span.data.is_null()
            || row == 0
            || image.span.stride < row
            || image.span.length != readable
            || image.span.length > isize::MAX as usize
        {
            return Err(-1);
        }
        Ok(image)
    }

    pub fn reference(&self, hwnd: *mut c_void) -> Result<Image, i32> {
        let mut span = ImageSpan::default();
        // SAFETY: Native code treats HWND as an OS handle, validates it through
        // the original class, and writes only this output descriptor.
        let status = unsafe { (self.reference)(hwnd, &mut span) };
        self.adopt(status, span)
    }

    pub fn objects(&self) -> u32 {
        // SAFETY: Queries this process's GDI count; no borrowed inputs.
        unsafe { (self.objects)() }
    }

    pub fn converter(&self, fault: u32) -> Converter<'_> {
        Converter {
            adapter: self,
            fault,
            observed_objects: None,
        }
    }

    pub fn window(&self, width: i32, height: i32) -> Result<Window<'_>, i32> {
        let mut span = WindowSpan::default();
        // SAFETY: Descriptor is writable; native function creates a current-
        // thread window and returns its actual client size, or zeroed failure.
        let status = unsafe { (self.open_window)(width, height, &mut span) };
        if status != 0 {
            return Err(status);
        }
        Ok(Window {
            adapter: self,
            span,
        })
    }
}

pub(crate) struct Converter<'a> {
    adapter: &'a Adapter,
    fault: u32,
    pub observed_objects: Option<u32>,
}

impl BgraConverter for Converter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Image, i32> {
        self.observed_objects = Some(self.adapter.objects());
        let mut span = ImageSpan::default();
        // SAFETY: Borrowed BGRA input stays valid through this synchronous call;
        // native conversion neither modifies nor retains it, and returns an
        // independently-owned CPU image. Output descriptor is fully writable.
        let status = unsafe {
            (self.adapter.convert)(
                input.pixels.as_ptr(),
                input.pixels.len(),
                input.width,
                input.height,
                input.stride,
                self.fault,
                &mut span,
            )
        };
        self.adapter.adopt(status, span)
    }
}

pub(crate) struct Image {
    span: ImageSpan,
    module: Arc<Module>,
    release: Release,
}

impl Image {
    pub fn dimensions(&self) -> (i32, i32) {
        (self.span.width, self.span.height)
    }
    pub fn stride(&self) -> usize {
        self.span.stride
    }
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: adopt checked the descriptor's complete readable span; the
        // private native owner and loader remain live through this borrow.
        unsafe { std::slice::from_raw_parts(self.span.data, self.span.length) }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: Owns exactly this opaque CPU Mat owner; no writable aliases or
        // borrowed spans outlive self. Module is retained until after release.
        unsafe { (self.release)(self.span.owner) };
        let _ = &self.module;
    }
}

pub(crate) struct Window<'a> {
    adapter: &'a Adapter,
    span: WindowSpan,
}

impl Window<'_> {
    pub fn hwnd(&self) -> *mut c_void {
        self.span.hwnd
    }
    pub fn dimensions(&self) -> (i32, i32) {
        (self.span.width, self.span.height)
    }
    pub fn paint(&self, phase: u32) -> Result<(), i32> {
        // SAFETY: This owns the HWND on its creation thread; phase is validated.
        match unsafe { (self.adapter.paint)(self.span.hwnd, phase) } {
            0 => Ok(()),
            status => Err(status),
        }
    }
    pub fn minimize(&self) -> Result<(), Box<dyn Error>> {
        type Iconic = unsafe extern "C" fn(*mut c_void) -> i32;
        // SAFETY: Exact private header signature; this fixture owns its HWND on
        // the current thread. No unrelated window or process context is changed.
        let iconic: Iconic = unsafe { self.adapter.module.symbol(c"mfs_gdi_parity_iconic")? };
        // SAFETY: Same live fixture HWND; ShowWindow/IsIconic completes locally.
        if unsafe { iconic(self.span.hwnd) } != 0 {
            return Err("owned window did not become iconic".into());
        }
        Ok(())
    }
    pub fn close(&mut self) -> Result<(), i32> {
        let hwnd = std::mem::replace(&mut self.span.hwnd, null_mut());
        if hwnd.is_null() {
            return Ok(());
        }
        // SAFETY: Consume the owning-thread destruction responsibility once.
        match unsafe { (self.adapter.destroy)(hwnd) } {
            0 => Ok(()),
            status => Err(status),
        }
    }
}

impl Drop for Window<'_> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
