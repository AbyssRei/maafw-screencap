// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Test-only consumer of native/gdi-parity.h. Layout is verified in the real DLL.
use crate::backends::windows::pixels::{BgraConverter, BgraView};
use crate::test_support::native_module::Module;
use std::{
    error::Error,
    ffi::c_void,
    mem::{align_of, offset_of, size_of},
    ptr::null_mut,
    sync::Arc,
};

#[cfg(feature = "windows-print-window")]
#[path = "print_fault_probe.rs"]
pub(crate) mod print_fault;
#[cfg(feature = "windows-print-window")]
#[path = "print_pseudo_adapter.rs"]
pub(crate) mod print_pseudo;
#[cfg(feature = "windows-print-window")]
#[path = "print_query_probe.rs"]
pub(crate) mod print_query;
#[cfg(feature = "windows-print-window")]
#[path = "print_window_adapter.rs"]
pub(crate) mod print_window;
#[cfg(any(
    feature = "windows-gdi",
    feature = "windows-print-window",
    feature = "windows-screen-dc"
))]
#[path = "gdi_probe.rs"]
pub(crate) mod probe;
#[cfg(any(
    feature = "windows-screen-dc",
    feature = "windows-frame-pool",
    feature = "windows-desktop-dup"
))]
#[path = "screen_fixture.rs"]
pub(crate) mod screen_fixture;

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
    #[cfg(feature = "windows-screen-dc")]
    pub fn screen_window_observation(
        &self,
        window: *mut c_void,
    ) -> Result<[u64; 16], Box<dyn Error>> {
        type Observe = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> i32;
        // SAFETY: Exact screen-parity.h signature; read-only native queries and
        // an exact initialized writable array. No input pointers are retained.
        let observe: Observe = unsafe { self.module.symbol(c"mfs_screen_window_observation")? };
        let mut output = [0; 16];
        if unsafe { observe(window, output.as_mut_ptr(), output.len()) } != 0 {
            return Err("screen window observation failed".into());
        }
        Ok(output)
    }
    #[cfg(feature = "windows-screen-dc")]
    pub fn screen_present(&self) -> Result<[i32; 2], Box<dyn Error>> {
        type Present = unsafe extern "C" fn(*mut i32, usize) -> i32;
        // SAFETY: Exact private fixture signature, no object/context retained.
        let present: Present = unsafe { self.module.symbol(c"mfs_screen_fixture_present")? };
        let mut output = [0; 2];
        if unsafe { present(output.as_mut_ptr(), output.len()) } != 0 {
            return Err("screen presentation observation failed".into());
        }
        Ok(output)
    }
    #[cfg(feature = "windows-screen-dc")]
    pub fn screen_reference(&self, hwnd: *mut c_void) -> Result<Image, i32> {
        // SAFETY: Exact private image signature, synchronous source capture;
        // the image owner retains this module independently after adoption.
        let reference: Reference =
            unsafe { self.module.symbol(c"mfs_screen_parity_reference") }.map_err(|_| -1)?;
        let mut output = ImageSpan::default();
        let status = unsafe { reference(hwnd, &mut output) };
        self.adopt(status, output)
    }
    pub fn image_owners(&self) -> Result<u64, Box<dyn Error>> {
        type ImageOwners = unsafe extern "C" fn() -> u64;
        // SAFETY: Exact private signature; atomic query has no pointer inputs.
        let owners: ImageOwners = unsafe { self.module.symbol(c"mfs_gdi_parity_image_owners")? };
        Ok(unsafe { owners() })
    }
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path = std::env::var_os("MFS_GDI_PARITY_LIBRARY")
            .ok_or("MFS_GDI_PARITY_LIBRARY is required; run cargo xtask upstream-parity gdi")?;
        let module = Arc::try_new(Module::load(&path)?)?;
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
        Image::adopt(&self.module, self.release, status, span)
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
            conversions: 0,
            clones: 0,
            clone_fault_at: None,
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
    pub conversions: usize,
    pub clones: usize,
    pub clone_fault_at: Option<usize>,
}

impl BgraConverter for Converter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<Image, i32> {
        self.conversions = self.conversions.checked_add(1).ok_or(-1)?;
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

#[cfg(feature = "windows-desktop-dup-window")]
impl crate::backends::windows::desktop_dup_window::Converter for Converter<'_> {
    fn dimensions(&self, image: &Image) -> (i32, i32) {
        image.dimensions()
    }
    fn crop_bgr(
        &mut self,
        image: &Image,
        crop: crate::backends::windows::desktop_dup_window::Crop,
    ) -> Result<Image, i32> {
        type CropImage =
            unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32, *mut ImageSpan) -> i32;
        // SAFETY: Input owner and function belong to the same retained module;
        // exact private header ABI, checked geometry and writable output.
        let function: CropImage =
            unsafe { image.module.symbol(c"mfs_desktop_dup_crop") }.map_err(|_| -1)?;
        let mut output = ImageSpan::default();
        let code = unsafe {
            function(
                image.span.owner,
                crop.x,
                crop.y,
                crop.width,
                crop.height,
                &mut output,
            )
        };
        Image::adopt(&image.module, image.release, code, output)
    }
}

#[cfg(feature = "windows-frame-pool")]
impl crate::backends::windows::frame_pool::Converter for Converter<'_> {
    fn clone_image(&mut self, image: &Image) -> Result<Image, i32> {
        type CloneImage = unsafe extern "C" fn(*mut c_void, u32, *mut ImageSpan) -> i32;
        self.clones = self.clones.checked_add(1).ok_or(-1)?;
        // SAFETY: Resolve the exact private header signature in the input's
        // real owning module, which can differ from the active converter's.
        let clone: CloneImage =
            unsafe { image.module.symbol(c"mfs_gdi_parity_clone") }.map_err(|_| -1)?;
        let mut output = ImageSpan::default();
        let fail = u32::from(self.clone_fault_at == Some(self.clones));
        let status = unsafe { clone(image.span.owner, fail, &mut output) };
        Image::adopt(&image.module, image.release, status, output)
    }
}

pub(crate) struct Image {
    span: ImageSpan,
    module: Arc<Module>,
    release: Release,
}

impl Image {
    fn adopt(
        module: &Arc<Module>,
        release: Release,
        status: i32,
        span: ImageSpan,
    ) -> Result<Self, i32> {
        let image = Self {
            span,
            module: Arc::clone(module),
            release,
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
    #[cfg(any(feature = "windows-frame-pool", feature = "windows-desktop-dup"))]
    pub fn address(&self) -> usize {
        self.span.data as usize
    }
    #[cfg(any(feature = "windows-frame-pool", feature = "windows-desktop-dup"))]
    pub fn native_owners(&self) -> u64 {
        type Owners = unsafe extern "C" fn() -> u64;
        // SAFETY: Exact gdi-probe.h observation, module retained by this image.
        let owners: Owners = unsafe { self.module.symbol(c"mfs_gdi_parity_image_owners") }.unwrap();
        unsafe { owners() }
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
