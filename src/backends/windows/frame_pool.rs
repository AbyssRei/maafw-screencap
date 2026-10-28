// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed FramePoolScreencap.{h,cpp}; private P1 migration.
use super::pixels::{BgraConverter, BgraView};
use crate::platform::windows::wgc::{self, native::*};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::{
    collections::TryReserveError,
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};
use windows_core::Interface;

// Drop must preserve cleanup evidence without formatting, allocating or
// panicking on an I/O failure while another primary error is already unwinding.
pub(crate) static DROP_CLOSE_ERRORS: [AtomicI32; 2] = [AtomicI32::new(0), AtomicI32::new(0)];
pub(crate) static DROP_CLOSE_ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

pub(crate) trait Converter: BgraConverter {
    // Both source clone points are explicit and may fail independently.
    fn clone_image(&mut self, image: &Self::Image) -> Result<Self::Image, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum Error<E> {
    NullWindow,
    Native(&'static str, windows_core::HRESULT),
    WinRtException(&'static str, windows_core::HRESULT),
    Win32(&'static str, u32),
    InvalidWindow,
    InvalidLayout,
    Overflow,
    NoFrame,
    // Source returned Some(empty Mat), distinct from a timeout with no cache.
    EmptySample,
    Allocation(TryReserveError),
    OwnerAllocation(std::alloc::AllocError),
    Conversion(E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Roi {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

fn client_roi<E>(
    client: RECT,
    origin: POINT,
    frame: RECT,
    width: i32,
    height: i32,
) -> Result<Roi, Error<E>> {
    if width <= 0 || height <= 0 {
        return Err(Error::InvalidLayout);
    }
    let mut left = origin
        .x
        .checked_sub(frame.left)
        .ok_or(Error::Overflow)?
        .max(0);
    let mut top = origin
        .y
        .checked_sub(frame.top)
        .ok_or(Error::Overflow)?
        .max(0);
    let client_width = client
        .right
        .checked_sub(client.left)
        .ok_or(Error::Overflow)?
        .min(width);
    let client_height = client
        .bottom
        .checked_sub(client.top)
        .ok_or(Error::Overflow)?
        .min(height);
    if client_width < 0 || client_height < 0 {
        return Err(Error::InvalidLayout);
    }
    // Preserve source's clamp order, including shifting the left/top origin
    // when the client extends beyond the captured texture's right/bottom edge.
    if left.checked_add(client_width).ok_or(Error::Overflow)? > width {
        left = width - client_width;
    }
    if top.checked_add(client_height).ok_or(Error::Overflow)? > height {
        top = height - client_height;
    }
    Ok(Roi {
        left,
        top,
        width: client_width,
        height: client_height,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub operation: &'static str,
    pub code: windows_core::HRESULT,
}

#[derive(Default, Debug)]
pub(crate) struct Evidence {
    pub cleanup: [Option<Failure>; 4],
    cleanup_count: usize,
    pub properties: [Option<wgc::PropertyOutcome>; 6],
    property_count: usize,
    pub drained_frames: usize,
    pub polls: usize,
    pub replayed: bool,
    pub mapped_pitch: u32,
    pub target_copy_bytes: usize,
    pub frame_bounds_fallback: Option<windows_core::HRESULT>,
}
impl Evidence {
    fn closed(&mut self, operation: &'static str, result: windows_core::Result<()>) {
        if let Err(error) = result {
            // At most two uninit calls per capture, each closing session/pool.
            self.cleanup[self.cleanup_count] = Some(Failure {
                operation,
                code: error.code(),
            });
            self.cleanup_count = self
                .cleanup_count
                .checked_add(1)
                .expect("four close failure slots");
        }
    }
    fn property(&mut self, value: wgc::PropertyOutcome) {
        self.properties[self.property_count] = Some(value);
        self.property_count = self
            .property_count
            .checked_add(1)
            .expect("two sets of three properties");
    }
}

pub(crate) struct Attempt<T, E> {
    pub image: Result<T, Error<E>>,
    pub evidence: Evidence,
}

/// Same-thread owner. Construction does not initialize COM or start a worker;
/// original first-use initialization and process MTA effects remain observable.
pub(crate) struct FramePoolCapture<I> {
    window: HWND,
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    swap_chain: Option<IDXGISwapChain>,
    readable: Option<ID3D11Texture2D>,
    description: D3D11_TEXTURE2D_DESC,
    item: Option<GraphicsCaptureItem>,
    pool: Option<Direct3D11CaptureFramePool>,
    session: Option<GraphicsCaptureSession>,
    last_size: (i32, i32),
    cached: Option<I>,
    _thread: PhantomData<Rc<()>>,
}

fn native<T, E>(operation: &'static str, result: windows_core::Result<T>) -> Result<T, Error<E>> {
    result.map_err(|error| Error::Native(operation, error.code()))
}
fn present<T, E>(operation: &'static str, value: Option<T>) -> Result<T, Error<E>> {
    value.ok_or(Error::Native(
        operation,
        windows_core::HRESULT(0x8000_4003_u32 as i32),
    ))
}
fn exception<T, E>(
    operation: &'static str,
    result: windows_core::Result<T>,
) -> Result<T, Error<E>> {
    result.map_err(|error| Error::WinRtException(operation, error.code()))
}
fn property<E>(
    operation: &'static str,
    result: Result<wgc::PropertyOutcome, wgc::ConfigureError>,
) -> Result<wgc::PropertyOutcome, Error<E>> {
    result.map_err(|error| match error {
        wgc::ConfigureError::Metadata(error) => Error::WinRtException(operation, error.code()),
        wgc::ConfigureError::Allocation(error) => Error::OwnerAllocation(error),
    })
}
fn win32<E>(operation: &'static str) -> Error<E> {
    // SAFETY: Thread-local error observation, no ownership or pointer inputs.
    Error::Win32(operation, unsafe { GetLastError() })
}

impl<I> FramePoolCapture<I> {
    pub fn new(window: *mut std::ffi::c_void) -> Self {
        Self {
            window: HWND(window),
            device: None,
            context: None,
            swap_chain: None,
            readable: None,
            description: D3D11_TEXTURE2D_DESC::default(),
            item: None,
            pool: None,
            session: None,
            last_size: (0, 0),
            cached: None,
            _thread: PhantomData,
        }
    }

    /// Like source uninit, retains the image cache and device/context/swapchain.
    pub fn uninit(&mut self) -> Evidence {
        let mut evidence = Evidence::default();
        self.uninit_into(&mut evidence);
        evidence
    }
    fn uninit_into(&mut self, evidence: &mut Evidence) {
        if let Some(session) = self.session.take() {
            evidence.closed("GraphicsCaptureSession.Close", session.Close());
        }
        if let Some(pool) = self.pool.take() {
            evidence.closed("Direct3D11CaptureFramePool.Close", pool.Close());
        }
        self.readable.take();
        self.item.take();
        self.description = D3D11_TEXTURE2D_DESC::default();
        self.last_size = (0, 0);
    }

    pub fn capture<C: Converter<Image = I>>(&mut self, converter: &mut C) -> Attempt<I, C::Error> {
        let mut evidence = Evidence::default();
        let image = self.capture_inner(converter, &mut evidence);
        Attempt { image, evidence }
    }
    fn capture_inner<C: Converter<Image = I>>(
        &mut self,
        converter: &mut C,
        evidence: &mut Evidence,
    ) -> Result<I, Error<C::Error>> {
        if self.window.0.is_null() {
            return Err(Error::NullWindow);
        }
        if self.pool.is_none()
            && let Err(error) = self.init(evidence)
        {
            // Exceptions outside source init's local catches escape screencap;
            // only a false init return executes its immediate uninit branch.
            if !matches!(error, Error::WinRtException(..) | Error::OwnerAllocation(_)) {
                self.uninit_into(evidence);
            }
            return Err(error);
        }
        self.check_size(evidence)?;
        let frame = match self.wait_for_frame(evidence) {
            Ok(frame) => frame,
            Err(error) => {
                self.uninit_into(evidence);
                return Err(error);
            }
        };
        let Some(frame) = frame else {
            let cached = self.cached.as_ref().ok_or(Error::NoFrame)?;
            evidence.replayed = true;
            return converter.clone_image(cached).map_err(Error::Conversion);
        };
        // The current frame follows source's reference-release lifetime. Only
        // drained old frames receive an explicit Close call in wait_for_frame.
        self.read_frame(&frame, converter, evidence)
    }

    fn init<E>(&mut self, evidence: &mut Evidence) -> Result<(), Error<E>> {
        if self.window.0.is_null() {
            return Err(Error::NullWindow);
        }
        let descriptor = DXGI_SWAP_CHAIN_DESC {
            BufferCount: 1,
            BufferDesc: DXGI_MODE_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ..Default::default()
            },
            BufferUsage: DXGI_USAGE(DXGI_USAGE_RENDER_TARGET_OUTPUT),
            OutputWindow: self.window,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Windowed: true.into(),
            ..Default::default()
        };
        // Source's put outputs release the previous three slots before the call.
        self.context.take();
        self.device.take();
        self.swap_chain.take();
        // SAFETY: Owned empty outputs, original hardware driver/flags/swapchain,
        // nonowning HWND; all returned COM references stay on this caller.
        native("D3D11CreateDeviceAndSwapChain", unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                0,
                None,
                u32::try_from(D3D11_SDK_VERSION).expect("fixed SDK version"),
                Some(&descriptor),
                Some(&mut self.swap_chain),
                Some(&mut self.device),
                None,
                Some(&mut self.context),
            )
            .ok()
        })?;
        {
            let activation = native(
                "GraphicsCaptureItem activation",
                wgc::activation::<GraphicsCaptureItem>(),
            )?;
            let factory: IGraphicsCaptureItemInterop =
                native("QueryInterface item interop", activation.cast())?;
            // Source put_abi clears an item retained by an earlier escaping
            // initialization exception before this native output is written.
            self.item.take();
            // SAFETY: OS validates the target; generated IID adopts one item.
            self.item = Some(native("CreateForWindow", unsafe {
                factory.CreateForWindow(self.window)
            })?);
            // Both factory references leave the source try-block before Size.
        }
        let item = self.item.as_ref().unwrap();
        let size = exception("GraphicsCaptureItem.Size", item.Size())?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err(Error::InvalidLayout);
        }
        // SAFETY: Query only, no memory borrowed from the HWND.
        if !unsafe { IsWindow(Some(self.window)) }.as_bool()
            || !unsafe { IsWindowVisible(self.window) }.as_bool()
        {
            return Err(Error::InvalidWindow);
        }
        let device = present("ID3D11Device", self.device.as_ref())?;
        let dxgi: IDXGIDevice = native("QueryInterface IDXGIDevice", device.cast())?;
        // SAFETY: Live device, native wrapper adopts returned IInspectable.
        let inspectable = native("CreateDirect3D11DeviceFromDXGIDevice", unsafe {
            CreateDirect3D11DeviceFromDXGIDevice(&dxgi)
        })?;
        let runtime: IDirect3DDevice =
            native("QueryInterface IDirect3DDevice", inspectable.cast())?;
        // Keep the original second Size query at pool creation.
        // This argument is inside source's pool-creation try/catch; unlike
        // the first/last Size calls, its failure returns false from init.
        let size = native("GraphicsCaptureItem.Size for pool", item.Size())?;
        self.pool = Some(native(
            "Direct3D11CaptureFramePool.Create",
            wgc::create_pool(&runtime, size),
        )?);
        self.session = Some(native(
            "CreateCaptureSession",
            self.pool.as_ref().unwrap().CreateCaptureSession(item),
        )?);
        let session = self.session.as_ref().unwrap();
        evidence.property(property("border metadata", wgc::configure_border(session))?);
        evidence.property(property("cursor metadata", wgc::configure_cursor(session))?);
        evidence.property(property(
            "secondary metadata",
            wgc::configure_secondary(session),
        )?);
        native("StartCapture", session.StartCapture())?;
        let size = exception(
            "GraphicsCaptureItem.Size",
            self.item.as_ref().unwrap().Size(),
        )?;
        self.last_size = (size.Width, size.Height);
        Ok(())
    }

    fn check_size<E>(&mut self, evidence: &mut Evidence) -> Result<(), Error<E>> {
        let Some(item) = self.item.as_ref() else {
            return Ok(());
        };
        // SAFETY: Nonowning native validity query, matching source's short circuit.
        if !unsafe { IsWindow(Some(self.window)) }.as_bool() {
            return Err(Error::InvalidWindow);
        }
        let size = match item.Size() {
            Ok(size) => size,
            Err(error) => {
                self.uninit_into(evidence);
                return Err(Error::Native("GraphicsCaptureItem.Size", error.code()));
            }
        };
        if (size.Width, size.Height) == self.last_size {
            return Ok(());
        }
        self.uninit_into(evidence);
        // A size-change init failure leaves its partial state for the next call,
        // just like source check_and_handle_size_changed; do not add rollback.
        self.init(evidence)
    }

    fn wait_for_frame<E>(
        &self,
        evidence: &mut Evidence,
    ) -> Result<Option<Direct3D11CaptureFrame>, Error<E>> {
        let pool = present("Direct3D11CaptureFramePool", self.pool.as_ref())?;
        while let Some(frame) = native("TryGetNextFrame drain", wgc::next_frame(pool))? {
            native("old frame Close", frame.Close())?;
            evidence.drained_frames = evidence
                .drained_frames
                .checked_add(1)
                .ok_or(Error::Overflow)?;
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(2000) {
            std::thread::sleep(Duration::from_millis(2));
            evidence.polls = evidence.polls.checked_add(1).ok_or(Error::Overflow)?;
            if let Some(frame) = native("TryGetNextFrame wait", wgc::next_frame(pool))? {
                return Ok(Some(frame));
            }
        }
        Ok(None)
    }

    fn read_frame<C: Converter<Image = I>>(
        &mut self,
        frame: &Direct3D11CaptureFrame,
        converter: &mut C,
        evidence: &mut Evidence,
    ) -> Result<I, Error<C::Error>> {
        let surface = frame.Surface().map_err(|error| {
            if error.code().0 == 0 {
                Error::Native("frame.Surface null", error.code())
            } else {
                Error::WinRtException("frame.Surface", error.code())
            }
        })?;
        let access: IDirect3DDxgiInterfaceAccess =
            native("QueryInterface surface access", surface.cast())?;
        // SAFETY: Surface grants this generated texture IID and owned reference.
        let texture: ID3D11Texture2D =
            native("GetInterface texture", unsafe { access.GetInterface() })?;
        if self.readable.is_none() {
            self.init_texture(&texture)?;
        } else {
            let mut current = D3D11_TEXTURE2D_DESC::default();
            // SAFETY: Readonly descriptor query prevents an incompatible void
            // CopyResource from leaving undefined staging bytes readable.
            unsafe {
                texture.GetDesc(&mut current);
            }
            if current.Width != self.description.Width
                || current.Height != self.description.Height
                || current.Format != self.description.Format
                || current.MipLevels != self.description.MipLevels
                || current.ArraySize != self.description.ArraySize
                || current.SampleDesc != self.description.SampleDesc
            {
                return Err(Error::InvalidLayout);
            }
        }
        let staging = present("staging texture", self.readable.as_ref())?;
        let context = present("ID3D11DeviceContext", self.context.as_ref())?;
        // SAFETY: Matching original staging allocation/device, caller serialized.
        unsafe {
            context.CopyResource(staging, &texture);
        }
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: Original staging CPU-read mapping; guard owns one Unmap.
        native("Map", unsafe {
            context
                .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .ok()
        })?;
        evidence.mapped_pitch = mapped.RowPitch;
        let _mapping = Mapping {
            context,
            texture: staging,
        };
        let mut client = RECT::default();
        // SAFETY: Exact writable output. Query order remains inside the mapping.
        if !unsafe { GetClientRect(self.window, &mut client) }.as_bool() {
            return Err(win32("GetClientRect"));
        }
        let mut origin = POINT {
            x: client.left,
            y: client.top,
        };
        // SAFETY: Initialized exact point, original client.left/top input.
        if !unsafe { ClientToScreen(self.window, &mut origin) }.as_bool() {
            return Err(win32("ClientToScreen"));
        }
        let mut bounds = RECT::default();
        // SAFETY: Exact RECT buffer and matching attribute size.
        let code = unsafe {
            DwmGetWindowAttribute(
                self.window,
                u32::try_from(DWMWA_EXTENDED_FRAME_BOUNDS).expect("fixed DWM attribute"),
                (&mut bounds as *mut RECT).cast(),
                u32::try_from(size_of::<RECT>()).expect("fixed RECT size"),
            )
        };
        if code.is_err() {
            evidence.frame_bounds_fallback = Some(code);
            // SAFETY: Same exact output; fallback is present in fixed source.
            if !unsafe { GetWindowRect(self.window, &mut bounds) }.as_bool() {
                return Err(win32("GetWindowRect"));
            }
        }
        let width = i32::try_from(self.description.Width).map_err(|_| Error::Overflow)?;
        let height = i32::try_from(self.description.Height).map_err(|_| Error::Overflow)?;
        let roi = client_roi(client, origin, bounds, width, height)?;
        if roi.width == 0 || roi.height == 0 {
            // Source bgra_to_bgr(empty) returns an empty Mat and its empty
            // clone replaces the old cache. Preserve that invalidation instead
            // of replaying an older nonempty image on the following timeout.
            self.cached.take();
            return Err(Error::EmptySample);
        }
        // FramePool textures can contain space beyond current ContentSize.
        // Keep source ROI/clamping, but reject undefined trailing content rather
        // than reading it or silently cropping to a different target rectangle.
        let content = native("frame.ContentSize safety check", frame.ContentSize())?;
        if content.Width <= 0
            || content.Height <= 0
            || content.Width > width
            || content.Height > height
            || roi.left.checked_add(roi.width).ok_or(Error::Overflow)? > content.Width
            || roi.top.checked_add(roi.height).ok_or(Error::Overflow)? > content.Height
        {
            return Err(Error::InvalidLayout);
        }
        // SAFETY: This live successful Map owns the descriptor's native storage;
        // the guard below remains live through the full validated target copy.
        let pixels = unsafe { copy_target(&mapped, self.description, roi) }?;
        evidence.target_copy_bytes = pixels.len();
        // Independent initialized rows isolate target pixels and avoid exposing
        // unspecified mapped padding. Both original conversion/clone points
        // remain before Unmap; no image or cache borrows mapped/native memory.
        let result = converter
            .bgra_to_bgr(BgraView {
                pixels: &pixels,
                width: roi.width,
                height: roi.height,
                stride: usize::try_from(roi.width)
                    .unwrap()
                    .checked_mul(4)
                    .ok_or(Error::Overflow)?,
            })
            .map_err(Error::Conversion)?;
        self.cached = Some(converter.clone_image(&result).map_err(Error::Conversion)?);
        Ok(result)
    }

    fn init_texture<E>(&mut self, texture: &ID3D11Texture2D) -> Result<(), Error<E>> {
        let device = present("ID3D11Device", self.device.as_ref())?;
        // SAFETY: Live same-device texture, exact writable descriptor.
        unsafe {
            texture.GetDesc(&mut self.description);
        }
        self.description.BindFlags = 0;
        self.description.MiscFlags = 0;
        self.description.CPUAccessFlags =
            u32::try_from(D3D11_CPU_ACCESS_READ | D3D11_CPU_ACCESS_WRITE).expect("fixed CPU flags");
        self.description.Usage = D3D11_USAGE_STAGING;
        // SAFETY: Preserve complete source desc apart from original four edits.
        native("CreateTexture2D staging", unsafe {
            device
                .CreateTexture2D(&self.description, None, Some(&mut self.readable))
                .ok()
        })
    }
}

#[cfg(test)]
#[path = "frame_pool_tests.rs"]
mod tests;

impl<I> Drop for FramePoolCapture<I> {
    fn drop(&mut self) {
        let evidence = self.uninit();
        for failure in evidence.cleanup.into_iter().flatten() {
            let slot = usize::from(failure.operation != "GraphicsCaptureSession.Close");
            DROP_CLOSE_ERRORS[slot].store(failure.code.0, Ordering::Release);
            DROP_CLOSE_ERROR_COUNT
                .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    Some(n.saturating_add(1))
                })
                .expect("infallible saturated cleanup counter");
        }
        // Source destructor calls uninit then destroys members in reverse order.
        self.cached.take();
        self.swap_chain.take();
        self.context.take();
        self.device.take();
    }
}

struct Mapping<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        // SAFETY: Owns one successful same-thread Map of subresource zero;
        // both interfaces outlive this guard, including conversion unwind.
        unsafe {
            self.context.Unmap(self.texture, 0);
        }
    }
}

/// # Safety
/// mapped must be a live read mapping of the exact desc texture, with defined
/// BGRA pixels throughout its valid rows. The mapping must outlive this call.
unsafe fn copy_target<E>(
    mapped: &D3D11_MAPPED_SUBRESOURCE,
    desc: D3D11_TEXTURE2D_DESC,
    roi: Roi,
) -> Result<Vec<u8>, Error<E>> {
    if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
        || desc.SampleDesc.Count != 1
        || desc.ArraySize != 1
        || desc.MipLevels != 1
        || mapped.pData.is_null()
    {
        return Err(Error::InvalidLayout);
    }
    let pitch = usize::try_from(mapped.RowPitch).map_err(|_| Error::Overflow)?;
    let row = usize::try_from(roi.width)
        .map_err(|_| Error::InvalidLayout)?
        .checked_mul(4)
        .ok_or(Error::Overflow)?;
    let rows = usize::try_from(roi.height).map_err(|_| Error::InvalidLayout)?;
    let left = usize::try_from(roi.left)
        .map_err(|_| Error::InvalidLayout)?
        .checked_mul(4)
        .ok_or(Error::Overflow)?;
    let top = usize::try_from(roi.top).map_err(|_| Error::InvalidLayout)?;
    let full_row = usize::try_from(desc.Width)
        .map_err(|_| Error::Overflow)?
        .checked_mul(4)
        .ok_or(Error::Overflow)?;
    if row == 0
        || rows == 0
        || pitch < full_row
        || left.checked_add(row).ok_or(Error::Overflow)? > full_row
        || top.checked_add(rows).ok_or(Error::Overflow)?
            > usize::try_from(desc.Height).map_err(|_| Error::Overflow)?
    {
        return Err(Error::InvalidLayout);
    }
    let last = top
        .checked_add(rows - 1)
        .and_then(|n| n.checked_mul(pitch))
        .and_then(|n| n.checked_add(left))
        .and_then(|n| n.checked_add(row))
        .ok_or(Error::Overflow)?;
    isize::try_from(last).map_err(|_| Error::Overflow)?;
    let length = row.checked_mul(rows).ok_or(Error::Overflow)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(Error::Allocation)?;
    for y in 0..rows {
        let offset = top
            .checked_add(y)
            .and_then(|n| n.checked_mul(pitch))
            .and_then(|n| n.checked_add(left))
            .ok_or(Error::Overflow)?;
        // SAFETY: A successful Map guarantees live initialized BGRA pixel rows
        // in this validated native texture. ROI/span checks above exclude row
        // padding and out-of-target pixels; Map guard remains live for the copy.
        let bytes =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(offset), row) };
        output.extend_from_slice(bytes);
    }
    Ok(output)
}
