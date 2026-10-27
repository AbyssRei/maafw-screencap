// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Private P0 projection/ownership probes. Explicit hardware/WARP real-window
// cases live in framepool; none of these tests is a migrated product backend.
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/windows_com_prototype.rs"));
}

use native::*;
use std::{error::Error, marker::PhantomData, rc::Rc};
use windows_core::{IUnknown, Interface};

#[path = "windows_framepool.rs"]
mod framepool;

fn create_frame_pool(
    device: &IDirect3DDevice,
    size: SizeInt32,
) -> windows_core::Result<Direct3D11CaptureFramePool> {
    // Generated class statics cache factories for the process. Keep an explicit
    // factory on this initialized MTA instead, and release it before uninit.
    let factory: IDirect3D11CaptureFramePoolStatics =
        windows_core::factory::<Direct3D11CaptureFramePool, _>()?;
    let mut output = std::ptr::null_mut();
    // SAFETY: Valid factory/device on this MTA, exact generated signature and
    // writable output. Adopt the one returned reference only after success.
    unsafe {
        (Interface::vtable(&factory).Create)(
            factory.as_raw(),
            device.as_raw(),
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            size,
            &mut output,
        )
        .ok()?;
        if output.is_null() {
            return Err(windows_core::Error::empty());
        }
        Ok(Direct3D11CaptureFramePool::from_raw(output))
    }
}

struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    fn mta() -> windows_core::Result<Self> {
        // SAFETY: Called only on this prototype's fresh worker. Each successful
        // acquisition, including S_FALSE, has one same-thread RoUninitialize.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED).ok()? };
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: The !Send/!Sync guard owns one successful initialization.
        unsafe { RoUninitialize() };
    }
}

struct Mapping<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one successful Map on this context, subresource 0.
        // The guard keeps both interfaces alive; no borrowed pixels escape it.
        unsafe { self.context.Unmap(self.texture, 0) };
    }
}

#[test]
fn balances_each_successful_winrt_initialization_on_the_owning_thread() {
    std::thread::spawn(|| {
        let first = Apartment::mta().unwrap();
        let second = Apartment::mta().unwrap();
        drop(second);
        let mut apartment = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        // SAFETY: Valid outputs. One initialization is still owned by first.
        unsafe {
            CoGetApartmentType(&mut apartment, &mut qualifier)
                .ok()
                .unwrap()
        };
        drop(first);
        // SAFETY: Query only; verifies both successful counts were balanced.
        assert_eq!(
            unsafe { CoGetApartmentType(&mut apartment, &mut qualifier) },
            CO_E_NOTINITIALIZED
        );
    })
    .join()
    .unwrap();
}

#[test]
fn preserves_host_sta_when_mta_initialization_is_rejected() {
    std::thread::spawn(|| {
        // SAFETY: Fresh test thread, null reserved pointer. Balance only success.
        unsafe {
            CoInitializeEx(None, u32::try_from(COINIT_APARTMENTTHREADED).unwrap())
                .ok()
                .unwrap()
        };
        struct Sta;
        impl Drop for Sta {
            fn drop(&mut self) {
                // SAFETY: Owns exactly the successful CoInitializeEx above.
                unsafe { CoUninitialize() };
            }
        }
        let sta = Sta;
        let mut before = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        // SAFETY: Both outputs are valid and this thread owns the STA.
        unsafe { CoGetApartmentType(&mut before, &mut qualifier).unwrap() };
        // SAFETY: Deliberate incompatible request. Failure owns no init count.
        let error = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        assert_eq!(error, RPC_E_CHANGED_MODE);
        let mut after = APTTYPE::default();
        // SAFETY: Valid outputs; failed initialization did not change the STA.
        unsafe { CoGetApartmentType(&mut after, &mut qualifier).unwrap() };
        assert_eq!(before, after);
        drop(sta);
        // SAFETY: Valid outputs. Verifies no hidden initialization remains.
        assert_eq!(
            unsafe { CoGetApartmentType(&mut after, &mut qualifier) },
            CO_E_NOTINITIALIZED
        );
    })
    .join()
    .unwrap();
}

#[test]
fn preserves_framepool_empty_result_and_releases_unstarted_event_registration() {
    std::thread::spawn(|| -> windows_core::Result<()> {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let _apartment = Apartment::mta()?;
        let mut device = None;
        // SAFETY: Explicit software-device feasibility probe, valid owned output.
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                u32::try_from(D3D11_CREATE_DEVICE_BGRA_SUPPORT).unwrap(),
                None,
                u32::try_from(D3D11_SDK_VERSION).unwrap(),
                Some(&mut device),
                None,
                None,
            )
            .ok()?;
        }
        let device = device.unwrap();
        let dxgi: IDXGIDevice = device.cast()?;
        // SAFETY: Valid live DXGI interface; generated wrapper adopts its result.
        let runtime: IDirect3DDevice =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        let pool = create_frame_pool(
            &runtime,
            SizeInt32 {
                Width: 32,
                Height: 24,
            },
        )?;
        struct Pool {
            object: Direct3D11CaptureFramePool,
            closed: bool,
        }
        impl Pool {
            fn close(&mut self) -> windows_core::Result<()> {
                self.object.Close()?;
                self.closed = true;
                Ok(())
            }
        }
        impl Drop for Pool {
            fn drop(&mut self) {
                if !self.closed {
                    let _ = self.object.Close();
                }
            }
        }
        let mut pool = Pool {
            object: pool,
            closed: false,
        };
        let mut frame = std::ptr::null_mut();
        // SAFETY: Pool is live on its owning MTA thread; output is writable.
        // No session was started, so no native frame can be pending.
        let result = unsafe {
            (Interface::vtable(&pool.object).TryGetNextFrame)(pool.object.as_raw(), &mut frame)
        };
        println!(
            "Unstarted FramePool: empty HRESULT={result:?}, null_output={}",
            frame.is_null()
        );
        result.ok()?;
        // Adopt any non-null successful output before making assertions.
        let unexpected = if frame.is_null() {
            None
        } else {
            // SAFETY: The successful ABI output transfers one COM reference.
            Some(unsafe { Direct3D11CaptureFrame::from_raw(frame) })
        };
        assert!(unexpected.is_none());
        assert_eq!(
            pool.object.TryGetNextFrame().unwrap_err().code(),
            windows_core::Error::empty().code()
        );
        // The rich wrapper translates native success + null to Error::empty().
        // P1 must preserve upstream's empty-pool meaning via the narrow ABI,
        // rather than treating this generated error as native capture failure.
        let calls = Arc::try_new(AtomicUsize::new(0)).unwrap();
        let weak = Arc::downgrade(&calls);
        let retained = calls.clone();
        let registration = pool.object.FrameArrived(move |_, _| {
            // No native object access, locks, waits or panicking work in callback.
            retained.fetch_add(1, Ordering::Relaxed);
        })?;
        drop(calls);
        assert!(weak.upgrade().is_some());
        let token = registration.into_token();
        // SAFETY: Token belongs to this live pool. Check the actual unregister
        // HRESULT instead of relying on EventRevoker's best-effort Drop.
        unsafe {
            (Interface::vtable(&pool.object).RemoveFrameArrived)(pool.object.as_raw(), token)
                .ok()?;
        }
        pool.close()?;
        drop(pool);
        assert!(weak.upgrade().is_none());
        // No capture session/events were started. This is registration release
        // evidence only; it does not establish in-flight callback draining.
        Ok(())
    })
    .join()
    .unwrap()
    .unwrap();
}

#[test]
fn reads_owned_d3d_pixels_through_dxgi_and_winrt_without_changing_host_context()
-> Result<(), Box<dyn Error + Send + Sync>> {
    let pixels = std::thread::spawn(|| -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let apartment = Apartment::mta()?;
        // SAFETY: Query only; does not change the process/thread DPI context.
        let dpi = unsafe { GetThreadDpiAwarenessContext() };
        let mut device = None;
        let mut context = None;
        // SAFETY: Explicit WARP adapter; outputs remain owned on this thread.
        // BGRA support is required for the WinRT Direct3D interop path.
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                u32::try_from(D3D11_CREATE_DEVICE_BGRA_SUPPORT)?,
                None,
                u32::try_from(D3D11_SDK_VERSION)?,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .ok()?;
        }
        let device = device.ok_or("D3D11 device output missing")?;
        let context = context.ok_or("D3D11 context output missing")?;
        let dxgi: IDXGIDevice = device.cast()?;
        // SAFETY: Live DXGI device; returned adapter is an owned COM reference.
        let adapter = unsafe { dxgi.GetAdapter()? };
        // SAFETY: Adapter reference is live and generated wrapper owns output.
        let mut description = DXGI_ADAPTER_DESC::default();
        unsafe { adapter.GetDesc(&mut description).ok()? };
        assert_ne!(description.VendorId, 0);
        // SAFETY: Valid DXGI device; wrapper adopts one owned IInspectable.
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
        let runtime: IDirect3DDevice = inspectable.cast()?;
        let access: IDirect3DDxgiInterfaceAccess = runtime.cast()?;
        // SAFETY: Correct IID/type pair selected by the generated generic method.
        let roundtrip: ID3D11Device = unsafe { access.GetInterface()? };
        assert_eq!(device.cast::<IUnknown>()?, roundtrip.cast::<IUnknown>()?);
        let retained = roundtrip.clone();
        drop(roundtrip);
        drop(device);
        drop(access);
        drop(runtime);
        drop(inspectable);
        drop(dxgi);
        drop(adapter);

        // Asymmetric rows establish pitch/row order/channel order, rather than
        // permitting a uniform-color fixture to hide a transpose or inversion.
        let expected = [
            1_u8, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 11, 12, 13, 255, 14, 15, 16, 255, 17, 18,
            19, 255,
        ];
        let mut descriptor = D3D11_TEXTURE2D_DESC {
            Width: 3,
            Height: 2,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            ..Default::default()
        };
        let source = D3D11_SUBRESOURCE_DATA {
            pSysMem: expected.as_ptr().cast(),
            SysMemPitch: 12,
            SysMemSlicePitch: 24,
        };
        let mut texture = None;
        // SAFETY: Descriptor describes exactly the two initialized 12-byte rows.
        // CreateTexture2D copies input synchronously; it never retains this slice.
        unsafe {
            retained
                .CreateTexture2D(&descriptor, Some(&source), Some(&mut texture))
                .ok()?
        };
        let texture = texture.ok_or("source texture output missing")?;
        descriptor.Usage = D3D11_USAGE_STAGING;
        descriptor.CPUAccessFlags = u32::try_from(D3D11_CPU_ACCESS_READ)?;
        let mut staging = None;
        // SAFETY: Staging allocation has matching dimensions/format and CPU read.
        unsafe {
            retained
                .CreateTexture2D(&descriptor, None, Some(&mut staging))
                .ok()?
        };
        let staging = staging.ok_or("staging texture output missing")?;
        // SAFETY: Same device and matching texture description; neither mapped.
        unsafe { context.CopyResource(&staging, &texture) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: CPU-readable subresource 0. Successful Map is paired by guard.
        unsafe {
            context
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .ok()?
        };
        let mapping = Mapping {
            context: &context,
            texture: &staging,
        };
        let row = usize::try_from(descriptor.Width)?
            .checked_mul(4)
            .ok_or("row overflow")?;
        let pitch = usize::try_from(mapped.RowPitch)?;
        let height = usize::try_from(descriptor.Height)?;
        if mapped.pData.is_null() || pitch < row {
            return Err("invalid mapped layout".into());
        }
        let length = row.checked_mul(height).ok_or("length overflow")?;
        println!("Explicit WARP: vendor={:#x}, device={:#x}, BGRA={}x{}, row_bytes={row}, mapped_pitch={pitch}",
            description.VendorId, description.DeviceId, descriptor.Width, descriptor.Height);
        let mut owned = Vec::new();
        owned.try_reserve_exact(length)?;
        for y in 0..height {
            let offset = y.checked_mul(pitch).ok_or("offset overflow")?;
            let end = offset.checked_add(row).ok_or("span overflow")?;
            if end > isize::MAX as usize {
                return Err("mapped span too large".into());
            }
            // SAFETY: Map guarantees RowPitch and readable rows until Unmap.
            // Only initialized target pixels are read, never driver padding.
            let bytes =
                unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(offset), row) };
            owned.extend_from_slice(bytes);
        }
        drop(mapping);
        drop(staging);
        drop(texture);
        drop(context);
        drop(retained);
        assert_eq!(owned, expected);
        // SAFETY: Query/comparison only; no DPI context was installed.
        assert!(
            unsafe { AreDpiAwarenessContextsEqual(dpi, GetThreadDpiAwarenessContext()) }.as_bool()
        );
        drop(apartment);
        Ok(owned)
    })
    .join()
    .unwrap()?;
    assert_eq!(&pixels[..4], &[1, 2, 3, 255]);
    std::thread::spawn(move || assert_eq!(&pixels[12..16], &[11, 12, 13, 255]))
        .join()
        .unwrap();
    Ok(())
}
