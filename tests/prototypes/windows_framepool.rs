// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// P0 only. Real WGC capture via the upstream Create entry point, not a migrated
// product backend or the P3 CreateFreeThreaded implementation.
use super::{Apartment, Mapping, create_frame_pool, native::*};
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use windows_core::{Interface, PCWSTR};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

fn pump() -> Result<()> {
    let mut message = MSG::default();
    // SAFETY: Runs only on this probe's dedicated MTA/window thread. The output
    // is writable and only this thread's messages are dispatched.
    while unsafe { PeekMessageW(&mut message, None, 0, 0, u32::try_from(PM_REMOVE)?) }.as_bool() {
        if message.message == u32::try_from(WM_QUIT)? {
            return Err("unexpected WM_QUIT".into());
        }
        // SAFETY: The message was obtained for the current thread's own queue.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

struct Dispatcher(Option<DispatcherQueueController>);
impl Dispatcher {
    fn new() -> Result<Self> {
        // SAFETY: A fresh MTA probe thread has no dispatcher. COM is already
        // initialized by the !Send apartment guard; no apartment change requested.
        let controller = unsafe {
            CreateDispatcherQueueController(DispatcherQueueOptions {
                dwSize: u32::try_from(size_of::<DispatcherQueueOptions>())?,
                threadType: DQTYPE_THREAD_CURRENT,
                apartmentType: DQTAT_COM_NONE,
            })?
        };
        Ok(Self(Some(controller.cast()?)))
    }

    fn close(&mut self) -> Result<()> {
        let Some(controller) = self.0.as_ref() else {
            return Ok(());
        };
        let operation = controller.ShutdownQueueAsync()?;
        let until = Instant::now() + Duration::from_secs(5);
        let mut exceeded_budget = false;
        while operation.Status()? == windows_future::AsyncStatus::Started {
            pump()?;
            exceeded_budget |= Instant::now() >= until;
            std::thread::sleep(Duration::from_millis(1));
        }
        let result = operation.GetResults();
        self.0.take();
        result?;
        // Continue pumping until real completion even after the probe budget.
        // A timeout is not permission to abandon the owned native queue/thread.
        if exceeded_budget {
            return Err("dispatcher shutdown exceeded probe budget".into());
        }
        Ok(())
    }
}
impl Drop for Dispatcher {
    fn drop(&mut self) {
        // Explicit close reports errors. This fallback owns the same apartment
        // and pumps shutdown on early returns; it does not claim cancellation.
        if let Err(error) = self.close() {
            eprintln!("dispatcher cleanup: {error}");
        }
    }
}

struct Window(Option<HWND>);
impl Window {
    fn new() -> Result<Self> {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let title = [0_u16];
        // SAFETY: Built-in class, terminated strings, own window on this thread.
        let window = unsafe {
            CreateWindowExW(
                0,
                PCWSTR(class.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP | u32::try_from(WS_VISIBLE)?,
                240,
                240,
                96,
                64,
                None,
                None,
                None,
                None,
            )
        };
        if window.0.is_null() {
            return Err(windows_core::Error::from_thread().into());
        }
        let result = Self(Some(window));
        pump()?;
        let mut rectangle = RECT::default();
        // SAFETY: Live owned HWND and writable output.
        unsafe {
            GetClientRect(window, &mut rectangle).ok()?;
        }
        // SAFETY: The HWND is live and its DC is released on this thread.
        let dc = unsafe { GetDC(Some(window)) };
        if dc.0.is_null() {
            return Err("GetDC failed".into());
        }
        // SAFETY: Creates a solid brush with red=0x11, green=0x22, blue=0x33.
        let brush = unsafe { CreateSolidBrush(COLORREF(0x00332211)) };
        if brush.0.is_null() {
            // SAFETY: Owns the successful DC acquisition above.
            unsafe {
                ReleaseDC(Some(window), dc);
            }
            return Err("CreateSolidBrush failed".into());
        }
        // SAFETY: Live DC/brush and valid client rectangle. FillRect retains none.
        let filled = unsafe { FillRect(dc, &rectangle, brush) };
        // SAFETY: The brush was never selected and FillRect has returned.
        let deleted = unsafe { DeleteObject(HGDIOBJ(brush.0)).as_bool() };
        // SAFETY: Same-thread, same-window matching release.
        let released = unsafe { ReleaseDC(Some(window), dc) };
        if filled == 0 || !deleted || released != 1 {
            return Err("fixture drawing/cleanup failed".into());
        }
        // SAFETY: Finish fixture GDI and composition before starting capture.
        unsafe {
            GdiFlush().ok()?;
            DwmFlush().ok()?;
        }
        Ok(result)
    }

    fn handle(&self) -> HWND {
        self.0.unwrap()
    }
    fn close(&mut self) -> Result<()> {
        if let Some(window) = self.0 {
            // SAFETY: Only this thread owns/destroys the window, after capture.
            unsafe {
                DestroyWindow(window).ok()?;
            }
            self.0.take();
        }
        Ok(())
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("window cleanup: {error}");
        }
    }
}

#[derive(Default)]
struct Notifications {
    calls: AtomicUsize,
    thread: AtomicU32,
}

struct Capture {
    pool: Option<Direct3D11CaptureFramePool>,
    session: Option<GraphicsCaptureSession>,
    token: Option<i64>,
}
impl Capture {
    fn close(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        if let (Some(pool), Some(token)) = (&self.pool, self.token) {
            // SAFETY: Exact token from this pool; same owning MTA thread.
            let status =
                unsafe { (Interface::vtable(pool).RemoveFrameArrived)(pool.as_raw(), token) };
            if let Err(error) = status.ok() {
                failures.push(format!("remove event: {error}"));
            } else {
                self.token.take();
            }
        }
        if let Some(session) = self.session.take()
            && let Err(error) = session.Close()
        {
            failures.push(format!("session Close: {error}"));
        }
        if let Some(pool) = self.pool.take()
            && let Err(error) = pool.Close()
        {
            failures.push(format!("pool Close: {error}"));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; ").into())
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("capture cleanup: {error}");
        }
    }
}

struct OwnedPixels {
    width: usize,
    height: usize,
    bytes: Vec<u8>,
}

fn read_pixels(
    context: &ID3D11DeviceContext,
    device: &ID3D11Device,
    frame: &Direct3D11CaptureFrame,
    expected_size: SizeInt32,
) -> Result<OwnedPixels> {
    let size = frame.ContentSize()?;
    let surface = frame.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    // SAFETY: Native surface grants the interface for the generated correct IID.
    let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
    let mut description = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: Live texture, writable description output.
    unsafe {
        texture.GetDesc(&mut description);
    }
    if size != expected_size
        || size.Width <= 16
        || size.Height <= 16
        || description.Width < u32::try_from(size.Width)?
        || description.Height < u32::try_from(size.Height)?
        || description.Format != DXGI_FORMAT_B8G8R8A8_UNORM
    {
        return Err(
            format!("unexpected WGC layout: content={size:?}, texture={description:?}").into(),
        );
    }
    description.Usage = D3D11_USAGE_STAGING;
    description.BindFlags = 0;
    description.CPUAccessFlags = u32::try_from(D3D11_CPU_ACCESS_READ)?;
    description.MiscFlags = 0;
    let mut staging = None;
    // SAFETY: Same size/format/device, no retained initial memory.
    unsafe {
        device
            .CreateTexture2D(&description, None, Some(&mut staging))
            .ok()?;
    }
    let staging = staging.ok_or("staging output missing")?;
    // SAFETY: Matching textures, unmapped, same device and owning thread.
    unsafe {
        context.CopyResource(&staging, &texture);
    }
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: CPU-readable staging subresource 0, paired by mapping guard.
    unsafe {
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .ok()?;
    }
    let _mapping = Mapping {
        context,
        texture: &staging,
    };
    let pitch = usize::try_from(mapped.RowPitch)?;
    let width = usize::try_from(size.Width)?;
    let height = usize::try_from(size.Height)?;
    let row = width.checked_mul(4).ok_or("row overflow")?;
    let length = row.checked_mul(height).ok_or("length overflow")?;
    if mapped.pData.is_null() || pitch < row {
        return Err("invalid native mapped layout".into());
    }
    let mut owned = Vec::new();
    owned.try_reserve_exact(length)?;
    for y in 0..height {
        let offset = y.checked_mul(pitch).ok_or("offset overflow")?;
        let end = offset.checked_add(row).ok_or("span overflow")?;
        isize::try_from(end)?;
        // SAFETY: Map guarantees these content rows through Unmap. Copy exactly
        // the target row; do not expose texture padding or the native pointer.
        let bytes =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(offset), row) };
        owned.extend_from_slice(bytes);
    }
    println!(
        "WGC texture={}x{}, content={}x{}, row_bytes={row}, pitch={pitch}",
        description.Width, description.Height, size.Width, size.Height
    );
    Ok(OwnedPixels {
        width,
        height,
        bytes: owned,
    })
}

fn capture(driver: D3D_DRIVER_TYPE) -> Result<()> {
    let pixels = std::thread::spawn(move || -> Result<OwnedPixels> {
        let _apartment = Apartment::mta()?;
        // SAFETY: Thread-local queries, no pointer inputs or state changes.
        let dpi = unsafe { GetThreadDpiAwarenessContext() };
        // SAFETY: Returns the current native thread identity without side effects.
        let owner = unsafe { GetCurrentThreadId() };
        let mut dispatcher = Dispatcher::new()?;
        let mut window = Window::new()?;
        let support: IGraphicsCaptureSessionStatics =
            windows_core::factory::<GraphicsCaptureSession, _>()?;
        let mut supported = false;
        // SAFETY: Live owned factory, exact generated ABI, writable bool output.
        unsafe {
            (Interface::vtable(&support).IsSupported)(support.as_raw(), &mut supported).ok()?;
        }
        if !supported {
            return Err("missing environment: WGC IsSupported=false".into());
        }
        let mut device = None;
        let mut context = None;
        // SAFETY: Explicit driver selection, no fallback. Native outputs owned
        // by this MTA thread and released before its apartment guard.
        let status = unsafe {
            D3D11CreateDevice(
                None,
                driver,
                HMODULE::default(),
                u32::try_from(D3D11_CREATE_DEVICE_BGRA_SUPPORT)?,
                None,
                u32::try_from(D3D11_SDK_VERSION)?,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        };
        println!("WGC driver={driver:?}, device_create={status:?}, owner_thread={owner}");
        status.ok()?;
        let device = device.ok_or("device output missing")?;
        let context = context.ok_or("context output missing")?;
        let dxgi: IDXGIDevice = device.cast()?;
        // SAFETY: Live DXGI device, generated wrapper adopts WinRT output.
        let runtime: IDirect3DDevice =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        let factory: IGraphicsCaptureItemInterop =
            windows_core::factory::<GraphicsCaptureItem, _>()?;
        // SAFETY: Own live window, correct generated IID/result type.
        let item: GraphicsCaptureItem = unsafe { factory.CreateForWindow(window.handle())? };
        let target_size = item.Size()?;
        let pool = create_frame_pool(&runtime, target_size)?;
        let mut capture = Capture {
            pool: Some(pool),
            session: None,
            token: None,
        };
        let notifications = Arc::try_new(Notifications::default())?;
        let weak = Arc::downgrade(&notifications);
        let retained = notifications.clone();
        let registration = capture.pool.as_ref().unwrap().FrameArrived(move |_, _| {
            // SAFETY: Thread identity query has no pointer/ownership requirements.
            retained
                .thread
                .store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
            retained.calls.fetch_add(1, Ordering::Release);
        })?;
        capture.token = Some(registration.into_token());
        let session = capture.pool.as_ref().unwrap().CreateCaptureSession(&item)?;
        capture.session = Some(session);
        let session = capture.session.as_ref().unwrap();
        session.SetIsCursorCaptureEnabled(false)?;
        session.StartCapture()?;
        let until = Instant::now() + Duration::from_secs(5);
        while notifications.calls.load(Ordering::Acquire) == 0 {
            pump()?;
            if Instant::now() >= until {
                return Err("no FrameArrived event within probe budget".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let pool = capture.pool.as_ref().unwrap();
        let mut raw = std::ptr::null_mut();
        // SAFETY: Owning MTA thread and valid initialized out-pointer. Use raw
        // ABI to preserve native success/null, which the rich wrapper conflates.
        unsafe {
            (Interface::vtable(pool).TryGetNextFrame)(pool.as_raw(), &mut raw).ok()?;
        }
        if raw.is_null() {
            return Err("FrameArrived delivered no native frame".into());
        }
        // SAFETY: Successful ABI call transfers one owned, non-null frame ref.
        let frame = unsafe { Direct3D11CaptureFrame::from_raw(raw) };
        let result = read_pixels(&context, &device, &frame, target_size);
        let close_result = frame.Close();
        drop(frame);
        let pixels = match (result, close_result) {
            (Ok(pixels), Ok(())) => pixels,
            (Err(error), Ok(())) => return Err(error),
            (Ok(_), Err(error)) => return Err(error.into()),
            (Err(primary), Err(cleanup)) => {
                return Err(format!("{primary}; frame Close: {cleanup}").into());
            }
        };
        println!(
            "WGC callbacks={}, callback_thread={}, owner_thread={owner}",
            notifications.calls.load(Ordering::Acquire),
            notifications.thread.load(Ordering::Relaxed)
        );
        if notifications.thread.load(Ordering::Relaxed) != owner {
            return Err("Create event used an unexpected execution channel".into());
        }
        capture.close()?;
        drop(capture);
        drop(notifications);
        drop(item);
        drop(factory);
        drop(runtime);
        drop(dxgi);
        drop(context);
        drop(device);
        window.close()?;
        dispatcher.close()?;
        if weak.upgrade().is_some() {
            return Err("native event delegate retained context after queue shutdown".into());
        }
        // SAFETY: Query/comparison only. The probe never installs a DPI context.
        if !unsafe { AreDpiAwarenessContextsEqual(dpi, GetThreadDpiAwarenessContext()) }.as_bool() {
            return Err("DPI context changed".into());
        }
        Ok(pixels)
    })
    .join()
    .map_err(|_| "capture worker panicked")??;
    // Native frames, sessions, device, window and queue have all been released.
    // Validate an interior region to exclude WGC's system capture border.
    for y in 8_usize..pixels.height.checked_sub(8).ok_or("short image")? {
        for x in 8_usize..pixels.width.checked_sub(8).ok_or("narrow image")? {
            let offset = y
                .checked_mul(pixels.width)
                .and_then(|v| v.checked_add(x))
                .and_then(|v| v.checked_mul(4))
                .ok_or("pixel offset overflow")?;
            if pixels
                .bytes
                .get(offset..offset.checked_add(3).ok_or("pixel end overflow")?)
                != Some(&[0x33, 0x22, 0x11])
            {
                return Err(format!(
                    "WGC pixel mismatch at ({x},{y}): {:?}",
                    pixels
                        .bytes
                        .get(offset..offset.checked_add(4).ok_or("pixel end overflow")?)
                )
                .into());
            }
        }
    }
    let length = pixels
        .width
        .checked_mul(pixels.height)
        .and_then(|v| v.checked_mul(4))
        .ok_or("owned length overflow")?;
    std::thread::spawn(move || assert_eq!(pixels.bytes.len(), length))
        .join()
        .unwrap();
    Ok(())
}

#[test]
#[ignore = "explicit desktop WGC probe; hardware driver, no software fallback"]
fn captures_owned_window_with_hardware_and_drains_dispatcher() -> Result<()> {
    for cycle in 1..=20 {
        println!("WGC hardware lifecycle cycle {cycle}/20");
        capture(D3D_DRIVER_TYPE_HARDWARE)?;
    }
    Ok(())
}

#[test]
#[ignore = "explicit desktop WGC probe; WARP selected as a separate case"]
fn captures_owned_window_with_warp_and_drains_dispatcher() -> Result<()> {
    for cycle in 1..=20 {
        println!("WGC WARP lifecycle cycle {cycle}/20");
        capture(D3D_DRIVER_TYPE_WARP)?;
    }
    Ok(())
}
