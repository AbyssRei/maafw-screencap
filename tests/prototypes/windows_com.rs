// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// This private P0 prototype proves projection/ownership boundaries. WARP is
// selected explicitly; success does not establish hardware or capture support.
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
