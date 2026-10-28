// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed DesktopDupScreencap.{h,cpp}; private P1 implementation.
use super::pixels::{BgraConverter, BgraView};
use crate::platform::windows::desktop_dup::native::*;
use std::{collections::TryReserveError, marker::PhantomData, rc::Rc};
use windows_core::{HRESULT, Interface};

#[derive(Debug)]
pub(crate) enum CaptureError {
    Native(&'static str, HRESULT),
    Missing(&'static str),
    AccessLost,
    InvalidLayout,
    Overflow,
    Allocation(TryReserveError),
}
impl CaptureError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Native(name, _) | Self::Missing(name) => name,
            Self::AccessLost => "access lost",
            Self::InvalidLayout => "layout safety rejection",
            Self::Overflow => "arithmetic overflow",
            Self::Allocation(_) => "CPU allocation failure",
        }
    }
}
#[derive(Debug)]
pub(crate) enum Error<E> {
    Capture(CaptureError),
    Conversion(E),
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameInfo {
    pub present_time: i64,
    pub mouse_time: i64,
    pub accumulated: u32,
    pub protected_mask: bool,
}
#[derive(Default, Debug)]
pub(crate) struct SampleEvidence {
    pub acquire: Option<HRESULT>,
    pub frame: Option<FrameInfo>,
    pub release: Option<HRESULT>,
    pub pitch: u32,
    pub copied_bytes: usize,
    pub failure: Option<&'static str>,
}
#[derive(Default, Debug)]
pub(crate) struct Evidence {
    pub samples: [SampleEvidence; 4],
    pub sample_count: usize,
    pub warmup_alpha: [Option<u8>; 3],
    pub warmups: usize,
    pub monitor_query_failed: bool,
    pub primary_fallback: bool,
    pub lookup_error: Option<HRESULT>,
    pub adapters_seen: u32,
    pub outputs_seen: u32,
}
pub(crate) struct Attempt<T, E> {
    pub image: Result<T, Error<E>>,
    pub evidence: Evidence,
}

struct Raw {
    bytes: Vec<u8>,
    width: i32,
    height: i32,
    stride: usize,
}
impl Raw {
    fn view(&self) -> BgraView<'_> {
        BgraView {
            pixels: &self.bytes,
            width: self.width,
            height: self.height,
            stride: self.stride,
        }
    }
}

/// Caller-thread owner, including all native cleanup. A null input selects the
/// original adapter 0/output 0; a nonnull HWND follows the nearest monitor. This
/// private U target convention is distinct from window GDI/ScreenDC targets.
pub(crate) struct DesktopDupCapture {
    window: HWND,
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    factory: Option<IDXGIFactory>,
    adapter: Option<IDXGIAdapter>,
    output: Option<IDXGIOutput1>,
    duplication: Option<IDXGIOutputDuplication>,
    readable: Option<ID3D11Texture2D>,
    description: D3D11_TEXTURE2D_DESC,
    monitor: HMONITOR,
    just_initialized: bool,
    _caller: PhantomData<Rc<()>>,
}
fn native<T>(name: &'static str, result: windows_core::Result<T>) -> Result<T, CaptureError> {
    result.map_err(|error| CaptureError::Native(name, error.code()))
}
fn required<T>(name: &'static str, value: Option<T>) -> Result<T, CaptureError> {
    value.ok_or(CaptureError::Missing(name))
}

impl DesktopDupCapture {
    pub fn new(window: *mut std::ffi::c_void) -> Self {
        Self {
            window: HWND(window),
            device: None,
            context: None,
            factory: None,
            adapter: None,
            output: None,
            duplication: None,
            readable: None,
            description: D3D11_TEXTURE2D_DESC::default(),
            monitor: HMONITOR::default(),
            just_initialized: false,
            _caller: PhantomData,
        }
    }
    pub fn capture<C: BgraConverter>(&mut self, converter: &mut C) -> Attempt<C::Image, C::Error> {
        let mut evidence = Evidence::default();
        let image = self.capture_inner(converter, &mut evidence);
        Attempt { image, evidence }
    }
    fn capture_inner<C: BgraConverter>(
        &mut self,
        converter: &mut C,
        evidence: &mut Evidence,
    ) -> Result<C::Image, Error<C::Error>> {
        if self.device.is_none()
            && let Err(error) = self.init()
        {
            self.uninit();
            return Err(Error::Capture(error));
        }
        self.ensure_output(evidence).map_err(Error::Capture)?;
        if self.just_initialized {
            for index in 0..3 {
                evidence.warmups += 1;
                if let Ok(raw) = self.sample(evidence) {
                    // Source tests only the last BGRA pixel's alpha, then still
                    // acquires another frame for the actual returned image.
                    let alpha = *raw.bytes.last().expect("validated nonempty BGRA");
                    evidence.warmup_alpha[index] = Some(alpha);
                    if alpha == 255 {
                        self.just_initialized = false;
                        break;
                    }
                }
            }
        }
        let raw = self.sample(evidence).map_err(Error::Capture)?;
        // Original conversion point is AFTER Unmap, raw texture Release,
        // ReleaseFrame and desktop resource Release. Raw now owns safe CPU rows;
        // the fixed C++ source instead returns a dangling mapped cv::Mat view.
        converter.bgra_to_bgr(raw.view()).map_err(Error::Conversion)
    }
    fn init(&mut self) -> Result<(), CaptureError> {
        // SAFETY: Original default hardware/flags/feature-level selection, exact
        // owned out-slots. No device migration to a selected adapter in P1.
        native("D3D11CreateDevice", unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                0,
                None,
                u32::try_from(D3D11_SDK_VERSION).expect("fixed SDK version"),
                Some(&mut self.device),
                None,
                Some(&mut self.context),
            )
            .ok()
        })?;
        // SAFETY: The generated IID adopts one factory reference on this caller.
        self.factory = Some(native("CreateDXGIFactory", unsafe { CreateDXGIFactory() })?);
        Ok(())
    }
    fn ensure_output(&mut self, evidence: &mut Evidence) -> Result<(), CaptureError> {
        let target = if self.window.0.is_null() {
            HMONITOR::default()
        } else {
            // SAFETY: Nonowning HWND query; native API validates it.
            let monitor = unsafe {
                MonitorFromWindow(
                    self.window,
                    u32::try_from(MONITOR_DEFAULTTONEAREST).expect("fixed monitor flag"),
                )
            };
            evidence.monitor_query_failed = monitor.0.is_null();
            monitor
        };
        if target == self.monitor && self.duplication.is_some() {
            return Ok(());
        }
        // This order differs from source uninit and is deliberately preserved.
        self.duplication.take();
        self.readable.take();
        self.output.take();
        self.adapter.take();
        if !target.0.is_null() && self.find_output(target, evidence)? {
            self.monitor = target;
        } else {
            evidence.primary_fallback = true;
            self.init_primary()?;
            self.monitor = HMONITOR::default();
        }
        let output = required("IDXGIOutput1", self.output.as_ref())?;
        let device = required("ID3D11Device", self.device.as_ref())?;
        // SAFETY: Live output/device. Upstream permits different adapters here;
        // keep its native failure instead of changing device selection in U.
        self.duplication = Some(native("DuplicateOutput", unsafe {
            output.DuplicateOutput(device)
        })?);
        self.just_initialized = true;
        Ok(())
    }
    fn find_output(
        &mut self,
        target: HMONITOR,
        evidence: &mut Evidence,
    ) -> Result<bool, CaptureError> {
        let factory = required("IDXGIFactory", self.factory.as_ref())?;
        let mut adapter_index = 0_u32;
        loop {
            // SAFETY: Live factory; generated output adopts one adapter ref.
            let adapter = match unsafe { factory.EnumAdapters(adapter_index) } {
                Ok(adapter) => adapter,
                Err(error) => {
                    evidence.lookup_error = Some(error.code());
                    break;
                }
            };
            evidence.adapters_seen = evidence
                .adapters_seen
                .checked_add(1)
                .ok_or(CaptureError::Overflow)?;
            let mut output_index = 0_u32;
            loop {
                let mut output = None;
                // SAFETY: Live adapter, exact initialized owned output slot.
                let code = unsafe { adapter.EnumOutputs(output_index, &mut output) };
                if code.is_err() {
                    evidence.lookup_error = Some(code);
                    break;
                }
                let output = required("EnumOutputs null", output)?;
                evidence.outputs_seen = evidence
                    .outputs_seen
                    .checked_add(1)
                    .ok_or(CaptureError::Overflow)?;
                let mut description = DXGI_OUTPUT_DESC::default();
                // SAFETY: Live output and exact writable descriptor.
                let code = unsafe { output.GetDesc(&mut description) };
                if code.is_ok() && description.Monitor == target {
                    match output.cast::<IDXGIOutput1>() {
                        Ok(output1) => {
                            self.adapter = Some(adapter);
                            self.output = Some(output1);
                            return Ok(true); // original IDXGIOutput releases first
                        }
                        Err(error) => {
                            evidence.lookup_error = Some(error.code());
                            return Ok(false);
                        }
                    }
                }
                if code.is_err() {
                    evidence.lookup_error = Some(code);
                }
                output_index = output_index.checked_add(1).ok_or(CaptureError::Overflow)?;
            }
            adapter_index = adapter_index.checked_add(1).ok_or(CaptureError::Overflow)?;
        }
        Ok(false)
    }
    fn init_primary(&mut self) -> Result<(), CaptureError> {
        let factory = required("IDXGIFactory", self.factory.as_ref())?;
        // SAFETY: Live factory; the returned primary adapter reference is owned.
        self.adapter = Some(native("EnumAdapters primary", unsafe {
            factory.EnumAdapters(0)
        })?);
        let adapter = required("primary adapter null", self.adapter.as_ref())?;
        let mut output = None;
        // SAFETY: Live adapter and initialized output owned until function exit.
        native("EnumOutputs primary", unsafe {
            adapter.EnumOutputs(0, &mut output).ok()
        })?;
        let output = required("primary output null", output)?;
        self.output = Some(native(
            "primary QueryInterface IDXGIOutput1",
            output.cast(),
        )?);
        Ok(())
    }
    fn sample(&mut self, evidence: &mut Evidence) -> Result<Raw, CaptureError> {
        let index = evidence.sample_count;
        evidence.sample_count += 1;
        let sample = &mut evidence.samples[index];
        let result = self.sample_inner(sample);
        sample.failure = result.as_ref().err().map(CaptureError::kind);
        result
    }
    fn sample_inner(&mut self, evidence: &mut SampleEvidence) -> Result<Raw, CaptureError> {
        let context = required("ID3D11DeviceContext", self.context.as_ref())?;
        let duplication = required("IDXGIOutputDuplication", self.duplication.as_ref())?;
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource = std::ptr::null_mut();
        // SAFETY: Exact ABI and original 2000ms native wait, initialized outputs.
        let code = unsafe {
            (duplication.vtable().AcquireNextFrame)(
                duplication.as_raw(),
                2000,
                &mut info,
                &mut resource,
            )
        };
        evidence.acquire = Some(code);
        if code == DXGI_ERROR_ACCESS_LOST {
            self.duplication.take();
            self.readable.take();
            self.monitor = HMONITOR::default();
            return Err(CaptureError::AccessLost);
        }
        native("AcquireNextFrame", code.ok())?;
        evidence.frame = Some(FrameInfo {
            present_time: info.LastPresentTime,
            mouse_time: info.LastMouseUpdateTime,
            accumulated: info.AccumulatedFrames,
            protected_mask: info.ProtectedContentMaskedOut.as_bool(),
        });
        let acquisition = Acquisition {
            duplication,
            resource: if resource.is_null() {
                None
            } else {
                // SAFETY: Successful Acquire transfers one native resource reference.
                Some(unsafe { IDXGIResource::from_raw(resource) })
            },
            release: &mut evidence.release,
        };
        let resource = required("acquired resource null", acquisition.resource.as_ref())?;
        let texture: ID3D11Texture2D = native("QueryInterface ID3D11Texture2D", resource.cast())?;
        if self.readable.is_none() {
            // SAFETY: The acquired texture owns a live exact descriptor output.
            unsafe { texture.GetDesc(&mut self.description) };
            self.description.BindFlags = 0;
            self.description.MiscFlags = 0;
            self.description.CPUAccessFlags =
                u32::try_from(D3D11_CPU_ACCESS_READ | D3D11_CPU_ACCESS_WRITE)
                    .expect("fixed CPU flags");
            self.description.Usage = D3D11_USAGE_STAGING;
            let device = required("ID3D11Device", self.device.as_ref())?;
            // SAFETY: Original staging descriptor edits, empty owned output slot.
            native("CreateTexture2D staging", unsafe {
                device
                    .CreateTexture2D(&self.description, None, Some(&mut self.readable))
                    .ok()
            })?;
        } else {
            let mut current = D3D11_TEXTURE2D_DESC::default();
            // SAFETY: Same live acquired texture, exact descriptor; prevents an
            // incompatible void CopyResource from exposing undefined staging data.
            unsafe { texture.GetDesc(&mut current) };
            if current.Width != self.description.Width
                || current.Height != self.description.Height
                || current.Format != self.description.Format
                || current.ArraySize != self.description.ArraySize
                || current.MipLevels != self.description.MipLevels
                || current.SampleDesc != self.description.SampleDesc
            {
                return Err(CaptureError::InvalidLayout);
            }
        }
        let staging = required("staging texture null", self.readable.as_ref())?;
        // SAFETY: Same device, validated matching native texture layouts.
        unsafe { context.CopyResource(staging, &texture) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: CPU-readable same-context staging. Guard owns a successful Map.
        native("Map", unsafe {
            context
                .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .ok()
        })?;
        let _mapping = Mapping {
            context,
            texture: staging,
        };
        evidence.pitch = mapped.RowPitch;
        // SAFETY: This successful Map is live until the guard is dropped; copy
        // initializes independent packed BGRA rows before native storage expires.
        let raw = unsafe { copy_desktop(&mapped, self.description) }?;
        evidence.copied_bytes = raw.bytes.len();
        Ok(raw)
    }
    pub fn output_coordinates(&self) -> Result<[i32; 4], CaptureError> {
        let output = required("IDXGIOutput1", self.output.as_ref())?;
        let mut description = DXGI_OUTPUT_DESC::default();
        // SAFETY: Same owned output; exact writable descriptor on its caller.
        native("GetDesc output coordinates", unsafe {
            output.GetDesc(&mut description).ok()
        })?;
        let r = description.DesktopCoordinates;
        Ok([r.left, r.top, r.right, r.bottom])
    }
    pub fn uninit(&mut self) {
        self.readable.take();
        self.description = D3D11_TEXTURE2D_DESC::default();
        self.duplication.take();
        self.output.take();
        self.adapter.take();
        self.factory.take();
        self.context.take();
        self.device.take();
        // Fixed source does not reset current_monitor_/output_just_initialized_.
    }
}
impl Drop for DesktopDupCapture {
    fn drop(&mut self) {
        self.uninit();
    }
}

struct Acquisition<'a> {
    duplication: &'a IDXGIOutputDuplication,
    resource: Option<IDXGIResource>,
    release: &'a mut Option<HRESULT>,
}
impl Drop for Acquisition<'_> {
    fn drop(&mut self) {
        // SAFETY: One successful acquisition on this caller; mapped and raw
        // texture owners have already ended. Preserve release error separately.
        *self.release =
            Some(unsafe { (self.duplication.vtable().ReleaseFrame)(self.duplication.as_raw()) });
        self.resource.take();
    }
}
struct Mapping<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one successful same-context/subresource Map.
        unsafe { self.context.Unmap(self.texture, 0) };
    }
}
/// # Safety
/// A live read mapping of desc must supply initialized BGRA pixels in every
/// described row. The owning Map/Acquire guards must outlive this full copy.
unsafe fn copy_desktop(
    mapped: &D3D11_MAPPED_SUBRESOURCE,
    desc: D3D11_TEXTURE2D_DESC,
) -> Result<Raw, CaptureError> {
    if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
        || desc.MipLevels != 1
        || desc.ArraySize != 1
        || desc.SampleDesc.Count != 1
        || mapped.pData.is_null()
    {
        return Err(CaptureError::InvalidLayout);
    }
    let width = i32::try_from(desc.Width).map_err(|_| CaptureError::Overflow)?;
    let height = i32::try_from(desc.Height).map_err(|_| CaptureError::Overflow)?;
    if width <= 0 || height <= 0 {
        return Err(CaptureError::InvalidLayout);
    }
    let row = usize::try_from(width)
        .map_err(|_| CaptureError::Overflow)?
        .checked_mul(4)
        .ok_or(CaptureError::Overflow)?;
    let rows = usize::try_from(height).map_err(|_| CaptureError::Overflow)?;
    let pitch = usize::try_from(mapped.RowPitch).map_err(|_| CaptureError::Overflow)?;
    if pitch < row {
        return Err(CaptureError::InvalidLayout);
    }
    let span = (rows - 1)
        .checked_mul(pitch)
        .and_then(|n| n.checked_add(row))
        .ok_or(CaptureError::Overflow)?;
    isize::try_from(span).map_err(|_| CaptureError::Overflow)?;
    let length = row.checked_mul(rows).ok_or(CaptureError::Overflow)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(CaptureError::Allocation)?;
    for y in 0..rows {
        let offset = y.checked_mul(pitch).ok_or(CaptureError::Overflow)?;
        // SAFETY: Valid native pixel rows, bounded offset/span, Map still live.
        let pixels =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(offset), row) };
        bytes.extend_from_slice(pixels);
    }
    Ok(Raw {
        bytes,
        width,
        height,
        stride: row,
    })
}

#[cfg(test)]
#[path = "desktop_dup_tests.rs"]
mod tests;
