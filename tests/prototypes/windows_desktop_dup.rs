// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Native P0 feasibility only. Retain the upstream DuplicateOutput entry and
// capture only the owned fixture's pixels while the GPU/native leases are live.
use super::{Apartment, Mapping, framepool::Window, native::*};
use std::error::Error;
use windows_core::Interface;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

struct Acquisition<'a> {
    duplication: &'a IDXGIOutputDuplication,
    active: bool,
}
impl Acquisition<'_> {
    fn close(&mut self) -> windows_core::Result<()> {
        if self.active {
            self.active = false;
            // SAFETY: Exactly one successful acquisition, all native pixel
            // access/mapping has ended, on the owning D3D context thread.
            unsafe {
                (Interface::vtable(self.duplication).ReleaseFrame)(self.duplication.as_raw())
                    .ok()?;
            }
        }
        Ok(())
    }
}
impl Drop for Acquisition<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("Desktop Duplication cleanup: {error}");
        }
    }
}

struct Pixels {
    bytes: Vec<u8>,
    color: [u8; 3],
}
impl Pixels {
    fn validate(&self) {
        assert!(!self.bytes.is_empty());
        let (pixels, remainder) = self.bytes.as_chunks::<4>();
        assert!(remainder.is_empty());
        for pixel in pixels {
            assert_eq!(&pixel[..3], &self.color);
        }
    }
}

fn draw(window: HWND, index: u8) -> Result<(RECT, [u8; 3])> {
    // SAFETY: Only this fixture's live HWND is raised, with unchanged geometry.
    unsafe {
        SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            u32::try_from(SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)?,
        )
        .ok()?;
    }
    let mut client = RECT::default();
    let mut origin = POINT::default();
    // SAFETY: Live owned window and writable outputs; DPI context is unchanged.
    unsafe {
        GetClientRect(window, &mut client).ok()?;
        ClientToScreen(window, &mut origin).ok()?;
    }
    let color = [
        0x40_u8.checked_add(index).ok_or("color overflow")?,
        0x22,
        0x11,
    ];
    let value = u32::from(color[2]) | (u32::from(color[1]) << 8) | (u32::from(color[0]) << 16);
    // SAFETY: The DC/brush are local to this thread and released after drawing.
    let dc = unsafe { GetDC(Some(window)) };
    if dc.0.is_null() {
        return Err("fixture GetDC failed".into());
    }
    // SAFETY: No pointer inputs. FillRect borrows, never retains, the brush.
    let brush = unsafe { CreateSolidBrush(COLORREF(value)) };
    if brush.0.is_null() {
        // SAFETY: Balance the successful same-thread acquisition.
        unsafe { ReleaseDC(Some(window), dc) };
        return Err("fixture brush creation failed".into());
    }
    // SAFETY: All handles/rectangle are live; the brush was never selected.
    let filled = unsafe { FillRect(dc, &client, brush) };
    // SAFETY: Drawing has completed; matching same-thread cleanup.
    let deleted = unsafe { DeleteObject(HGDIOBJ(brush.0)).as_bool() };
    // SAFETY: Matching HWND/DC and owning thread.
    let released = unsafe { ReleaseDC(Some(window), dc) };
    if filled == 0 || !deleted || released != 1 {
        return Err("fixture drawing/cleanup failed".into());
    }
    // SAFETY: Finish GDI/composition so acquisition observes this color update.
    unsafe {
        GdiFlush().ok()?;
        DwmFlush().ok()?;
    }
    // Leave the fixture's edges out: this checks known client pixels and does
    // not assume that DPI virtualization preserves the CreateWindow dimensions.
    let width = client
        .right
        .checked_sub(client.left)
        .ok_or("width overflow")?;
    let height = client
        .bottom
        .checked_sub(client.top)
        .ok_or("height overflow")?;
    if width <= 16 || height <= 16 {
        return Err("fixture client area too small".into());
    }
    Ok((
        RECT {
            left: origin.x.checked_add(8).ok_or("left overflow")?,
            top: origin.y.checked_add(8).ok_or("top overflow")?,
            right: origin
                .x
                .checked_add(width)
                .and_then(|n| n.checked_sub(8))
                .ok_or("right overflow")?,
            bottom: origin
                .y
                .checked_add(height)
                .and_then(|n| n.checked_sub(8))
                .ok_or("bottom overflow")?,
        },
        color,
    ))
}

fn cycle(index: u8) -> Result<Pixels> {
    // Use this cycle's color from the first fixture presentation. Duplication
    // may expose that presentation before a later repaint; no alternate initial
    // color or pixel-mismatch retry is permitted to mask it.
    let mut window = Window::with_color(
        0x00002211 | (u32::from(0x40_u8.checked_add(index).ok_or("color overflow")?) << 16),
    )?;
    let mut device = None;
    let mut context = None;
    // SAFETY: Explicit default hardware adapter, no software fallback. All
    // interfaces and immediate-context operations remain on this owner thread.
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            0,
            None,
            u32::try_from(D3D11_SDK_VERSION)?,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .ok()?;
    }
    let device = device.ok_or("hardware device missing")?;
    let context = context.ok_or("hardware context missing")?;
    let dxgi: IDXGIDevice = device.cast()?;
    // SAFETY: Live default device; use its actual adapter for the native probe.
    let adapter = unsafe { dxgi.GetAdapter()? };
    let mut adapter_desc = DXGI_ADAPTER_DESC::default();
    // SAFETY: Valid adapter and writable descriptor.
    unsafe { adapter.GetDesc(&mut adapter_desc).ok()? };
    let (region, color) = draw(window.handle(), index)?;
    let mut selected = None;
    for number in 0..64 {
        let mut output = None;
        // SAFETY: Bounded enumeration, returned interface is owned locally.
        let status = unsafe { adapter.EnumOutputs(number, &mut output) };
        if status == DXGI_ERROR_NOT_FOUND {
            break;
        }
        status.ok()?;
        let output = output.ok_or("successful output enumeration returned null")?;
        let mut desc = DXGI_OUTPUT_DESC::default();
        // SAFETY: Live output and writable descriptor.
        unsafe { output.GetDesc(&mut desc).ok()? };
        let bounds = desc.DesktopCoordinates;
        if desc.AttachedToDesktop.as_bool()
            && region.left >= bounds.left
            && region.top >= bounds.top
            && region.right <= bounds.right
            && region.bottom <= bounds.bottom
        {
            selected = Some((output.cast::<IDXGIOutput1>()?, desc));
            break;
        }
    }
    let (output, output_desc) = selected.ok_or("missing_environment: default hardware adapter has no output containing the controlled fixture")?;
    if output_desc.Rotation != DXGI_MODE_ROTATION_IDENTITY {
        return Err("missing_environment: this P0 fixture requires an unrotated output".into());
    }
    // SAFETY: Device and output belong to the same actual adapter. This is the
    // original API, not the later DuplicateOutput1 migration.
    let duplication = unsafe { output.DuplicateOutput(&device)? };
    // Publish a new known color after duplication starts; no capture retry is
    // used to discard an assertion failure or a native error.
    let (_, redrawn) = draw(window.handle(), index)?;
    assert_eq!(redrawn, color);
    let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
    let mut resource = std::ptr::null_mut();
    // SAFETY: Exact generated ABI, finite upstream wait and writable outputs.
    unsafe {
        (Interface::vtable(&duplication).AcquireNextFrame)(
            duplication.as_raw(),
            2000,
            &mut info,
            &mut resource,
        )
        .ok()?;
    }
    let mut acquisition = Acquisition {
        duplication: &duplication,
        active: true,
    };
    if resource.is_null() {
        return Err("successful acquisition returned a null resource".into());
    }
    // SAFETY: Successful call transferred exactly one reference; the owned
    // interface stays inside the acquisition's valid native lifetime.
    let resource = unsafe { IDXGIResource::from_raw(resource) };
    let texture: ID3D11Texture2D = resource.cast()?;
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: Live acquired texture and writable output.
    unsafe { texture.GetDesc(&mut desc) };
    let x = usize::try_from(
        region
            .left
            .checked_sub(output_desc.DesktopCoordinates.left)
            .ok_or("x overflow")?,
    )?;
    let y = usize::try_from(
        region
            .top
            .checked_sub(output_desc.DesktopCoordinates.top)
            .ok_or("y overflow")?,
    )?;
    let width = usize::try_from(
        region
            .right
            .checked_sub(region.left)
            .ok_or("width overflow")?,
    )?;
    let height = usize::try_from(
        region
            .bottom
            .checked_sub(region.top)
            .ok_or("height overflow")?,
    )?;
    if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
        || x.checked_add(width).ok_or("x extent overflow")? > usize::try_from(desc.Width)?
        || y.checked_add(height).ok_or("y extent overflow")? > usize::try_from(desc.Height)?
    {
        return Err("acquired format or fixture bounds invalid".into());
    }
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = u32::try_from(D3D11_CPU_ACCESS_READ)?;
    let mut staging = None;
    // SAFETY: Matching format/dimensions, no initial pixel pointer.
    unsafe {
        device
            .CreateTexture2D(&desc, None, Some(&mut staging))
            .ok()?
    };
    let staging = staging.ok_or("staging texture missing")?;
    // SAFETY: Matching textures, same context, acquired surface still valid.
    unsafe { context.CopyResource(&staging, &texture) };
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: CPU-readable staging, same thread. Guard pairs successful Map.
    unsafe {
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .ok()?
    };
    let mapping = Mapping {
        context: &context,
        texture: &staging,
    };
    let pitch = usize::try_from(mapped.RowPitch)?;
    let native_row = usize::try_from(desc.Width)?
        .checked_mul(4)
        .ok_or("native row overflow")?;
    let row = width.checked_mul(4).ok_or("row overflow")?;
    if mapped.pData.is_null() || pitch < native_row {
        return Err("invalid mapped desktop row layout".into());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(row.checked_mul(height).ok_or("CPU frame size overflow")?)?;
    for row_number in 0..height {
        let offset = y
            .checked_add(row_number)
            .and_then(|n| n.checked_mul(pitch))
            .and_then(|n| x.checked_mul(4).and_then(|x| n.checked_add(x)))
            .ok_or("mapped offset overflow")?;
        if offset.checked_add(row).ok_or("mapped end overflow")? > isize::MAX as usize {
            return Err("mapped span exceeds pointer arithmetic bounds".into());
        }
        // SAFETY: Map grants these rows until Unmap; checked crop/pitch/span.
        // Read only the fixture's initialized pixels, never desktop padding.
        let pixels =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(offset), row) };
        bytes.extend_from_slice(pixels);
    }
    drop(mapping);
    drop(staging);
    drop(texture);
    acquisition.close()?;
    // SAFETY: Deliberate native state check: a second release owns no lease.
    assert_eq!(
        unsafe { (Interface::vtable(&duplication).ReleaseFrame)(duplication.as_raw()) },
        DXGI_ERROR_INVALID_CALL
    );
    drop(resource);
    drop(acquisition);
    drop(duplication);
    drop(output);
    drop(adapter);
    drop(dxgi);
    drop(context);
    drop(device);
    window.close()?;
    let pixels = Pixels { bytes, color };
    pixels.validate();
    println!(
        "Desktop Duplication cycle {index}/20: vendor={:#x}, device={:#x}, target={}x{}, pitch={pitch}, accumulated={}, present={:?}, released=1",
        adapter_desc.VendorId,
        adapter_desc.DeviceId,
        width,
        height,
        info.AccumulatedFrames,
        info.LastPresentTime
    );
    Ok(pixels)
}

#[test]
#[ignore = "explicit owned-window Desktop Duplication; native Windows desktop only"]
fn copies_fixture_pixels_before_unmap_and_balances_twenty_native_acquisitions() -> Result<()> {
    let (first, last) = std::thread::spawn(|| -> Result<(Pixels, Pixels)> {
        let _apartment = Apartment::mta()?;
        // SAFETY: Query only; this probe never installs a DPI context.
        let before = unsafe { GetThreadDpiAwarenessContext() };
        let first = cycle(1)?;
        let mut last = cycle(2)?;
        for index in 3..=20 {
            last = cycle(index)?;
            first.validate();
        }
        // SAFETY: Query/comparison only, after all D3D/window objects are gone.
        assert!(
            unsafe { AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()) }
                .as_bool()
        );
        Ok((first, last))
    })
    .join()
    .unwrap()?;
    std::thread::spawn(move || {
        first.validate();
        last.validate();
    })
    .join()
    .unwrap();
    Ok(())
}
