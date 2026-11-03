// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real hidden/destroyed HWND initialization failures and old-device releases.
use super::*;
use crate::backends::windows::pixels::{BgraConverter, BgraView};
use crate::test_support::native_module::Module;
use std::{
    convert::Infallible,
    error::Error as StdError,
    ffi::c_void,
    fs,
    io::Write,
    path::{Path, PathBuf},
    ptr::null_mut,
};
use windows_core::Interface;

type Status = i32;
type Widths = unsafe extern "C" fn(*mut u32, usize) -> Status;
type Create = unsafe extern "C" fn(*mut *mut c_void) -> Status;
type Destroy = unsafe extern "C" fn(*mut *mut c_void) -> Status;
type Info = unsafe extern "C" fn(*mut c_void, *mut u64, usize) -> Status;
type Attach = unsafe extern "C" fn(*mut c_void, u32) -> Status;
type Record = unsafe extern "C" fn(u32, *mut u64, usize) -> Status;
struct Fixture {
    clock: unsafe extern "C" fn() -> u64,
    create: Create,
    destroy: Destroy,
    info: Info,
    attach: Attach,
    record: Record,
}
impl Fixture {
    fn load(path: &Path) -> Result<Self, Box<dyn StdError>> {
        let module = Module::load(path.as_os_str())?;
        // Pin even on symbol/ABI/assertion failure. A native private-data final
        // Release can outlive a failed test caller, never the fixture's code.
        let module = std::mem::ManuallyDrop::new(module);
        // SAFETY: Exact private framepool-init-fixture.h scalar signatures in
        // the prepared process-retained DLL, verified by real native calls.
        let version = unsafe {
            module.symbol::<unsafe extern "C" fn() -> u32>(c"mfs_framepool_init_version")?
        };
        assert_eq!(unsafe { version() }, 1);
        let widths: Widths = unsafe { module.symbol(c"mfs_framepool_init_widths")? };
        let mut actual = [0; 6];
        assert_eq!(unsafe { widths(actual.as_mut_ptr(), actual.len()) }, 0);
        assert_eq!(actual, [8, 8, 8, 4, 4, 4]);
        Ok(Self {
            clock: unsafe { module.symbol(c"mfs_framepool_init_clock")? },
            create: unsafe { module.symbol(c"mfs_framepool_init_window_create")? },
            destroy: unsafe { module.symbol(c"mfs_framepool_init_window_destroy")? },
            info: unsafe { module.symbol(c"mfs_framepool_init_window_info")? },
            attach: unsafe { module.symbol(c"mfs_framepool_init_attach")? },
            record: unsafe { module.symbol(c"mfs_framepool_init_record")? },
        })
    }
    fn window(&self) -> Window<'_> {
        let mut value = null_mut();
        // SAFETY: Empty writable output; ownership remains on this caller.
        assert_eq!(unsafe { (self.create)(&mut value) }, 0);
        assert!(!value.is_null());
        Window { value, owner: self }
    }
    fn info(&self, window: *mut c_void) -> [u64; 4] {
        let mut values = [0; 4];
        // SAFETY: Native query treats the possibly destroyed HWND as a scalar.
        assert_eq!(
            unsafe { (self.info)(window, values.as_mut_ptr(), values.len()) },
            0
        );
        values
    }
    fn record(&self, index: u32) -> [u64; 6] {
        let mut values = [0; 6];
        // SAFETY: Fixed native process-lifetime atomic record and writable span.
        assert_eq!(
            unsafe { (self.record)(index, values.as_mut_ptr(), values.len()) },
            0
        );
        values
    }
}
struct Window<'a> {
    value: *mut c_void,
    owner: &'a Fixture,
}
impl Window<'_> {
    fn close(&mut self) {
        if !self.value.is_null() {
            // SAFETY: Exact caller-owned real hidden HWND. Success clears it.
            assert_eq!(unsafe { (self.owner.destroy)(&mut self.value) }, 0);
            assert!(self.value.is_null());
        }
    }
}
impl Drop for Window<'_> {
    fn drop(&mut self) {
        if !self.value.is_null() {
            // SAFETY: Same caller-owned native window. Failure is fatal to this
            // isolated test; never leak a window or unload its native module.
            if unsafe { (self.owner.destroy)(&mut self.value) } != 0 {
                std::process::abort();
            }
        }
    }
}
struct NeverConvert;
impl BgraConverter for NeverConvert {
    type Image = ();
    type Error = Infallible;
    fn bgra_to_bgr(&mut self, _: BgraView<'_>) -> Result<(), Infallible> {
        panic!("initialization failure reached conversion");
    }
}
impl Converter for NeverConvert {
    fn clone_image(&mut self, _: &()) -> Result<(), Infallible> {
        panic!("initialization failure reached clone/cache replay");
    }
}
fn slots(c: &FramePoolCapture<()>) -> u32 {
    u32::from(c.device.is_some())
        | (u32::from(c.context.is_some()) << 1)
        | (u32::from(c.swap_chain.is_some()) << 2)
        | (u32::from(c.readable.is_some()) << 3)
        | (u32::from(c.item.is_some()) << 4)
        | (u32::from(c.pool.is_some()) << 5)
        | (u32::from(c.session.is_some()) << 6)
}

#[test]
#[ignore = "requires prepared native hidden-window/device observer and unchanged source records"]
fn preserves_failed_initialization_state_and_releases_replaced_native_devices()
-> Result<(), Box<dyn StdError>> {
    use crate::test_support::windows_capture_host::{HostInitialization, host};
    let root = PathBuf::from(std::env::var_os("MFS_P1_FRAMEPOOL_INIT_ARTIFACTS").unwrap());
    let directory = root.join("rust");
    fs::create_dir(&directory)?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Writable scalar outputs on this isolated libtest caller thread.
    assert_eq!(
        unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) },
        host::CO_E_NOTINITIALIZED
    );
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let _initialization = HostInitialization(true);
    let fixture = Fixture::load(&PathBuf::from(
        std::env::var_os("MFS_P1_FRAMEPOOL_INIT_FIXTURE").unwrap(),
    ))?;
    fs::write(directory.join("abi.tsv"), b"1\t8\t8\t8\t4\t4\t4\n")?;
    let mut rows = fs::File::create_new(directory.join("calls.tsv"))?;
    let mut lifetimes = fs::File::create_new(directory.join("lifetimes.tsv"))?;
    writeln!(
        rows,
        "cycle\tcall\tvalid\tvisible\tbefore_slots\tafter_slots\tno_image\tprevious_final\tcaller\twindow_thread\tbegin_qpc\tend_qpc\tnative_hresult"
    )?;
    writeln!(
        lifetimes,
        "index\tattached\tfinal\tattach_thread\tfinal_thread\tattach_qpc\tfinal_qpc"
    )?;
    let mut converter = NeverConvert;
    let drop_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    for cycle in 0..20_u32 {
        let mut window = fixture.window();
        let target = window.value;
        let mut capture = FramePoolCapture::<()>::new(target);
        for call in 0..4_u32 {
            if call == 2 {
                window.close();
            }
            let info = fixture.info(target);
            assert_ne!(info[0], 0);
            assert_eq!(info[2..], [u64::from(call < 2), 0]);
            assert_eq!(info[1], if call < 2 { info[0] } else { 0 });
            let before = slots(&capture);
            // SAFETY: Scalar QPC query in the process-retained fixture image.
            let begin = unsafe { (fixture.clock)() };
            let attempt = capture.capture(&mut converter);
            let end = unsafe { (fixture.clock)() };
            let code = match attempt.image {
                Err(Error::Native("CreateForWindow", code)) if call < 2 => {
                    assert_eq!(code.0, -2147024809);
                    code.0
                }
                Err(Error::Native("D3D11CreateDeviceAndSwapChain", code)) if call >= 2 => {
                    assert_eq!(code.0, -2005270527);
                    code.0
                }
                other => panic!("unexpected native initialization result: {other:?}"),
            };
            let after = slots(&capture);
            let previous = if call == 1 || call == 2 {
                fixture.record(2 * cycle + call - 1)[1]
            } else {
                0
            };
            writeln!(
                rows,
                "{cycle}\t{call}\t{}\t{}\t{before}\t{after}\t1\t{previous}\t{}\t{}\t{begin}\t{end}\t{code}",
                info[2], info[3], info[0], info[1]
            )?;
            rows.flush()?;
            assert_eq!(before, if call == 0 || call == 3 { 0 } else { 7 });
            assert_eq!(after, if call < 2 { 7 } else { 0 });
            assert_eq!(previous, u64::from(call == 1 || call == 2));
            assert!(capture.cached.is_none());
            assert_eq!(capture.last_size, (0, 0));
            assert_eq!(
                (attempt.evidence.polls, attempt.evidence.target_copy_bytes),
                (0, 0)
            );
            assert!(!attempt.evidence.replayed);
            assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
            if call < 2 {
                let device = capture.device.as_ref().unwrap();
                // SAFETY: Live real ID3D11Device borrow on the source caller.
                // The pinned native fixture retains only its own IUnknown.
                assert_eq!(
                    unsafe { (fixture.attach)(device.as_raw(), 2 * cycle + call) },
                    0
                );
            }
        }
        drop(capture);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), drop_errors);
    }
    for index in 0..40 {
        let values = fixture.record(index);
        assert_eq!(values[..2], [1, 1]);
        assert!(values[2] != 0 && values[3] != 0 && values[4] != 0 && values[5] >= values[4]);
        writeln!(
            lifetimes,
            "{index}\t{}\t{}\t{}\t{}\t{}\t{}",
            values[0], values[1], values[2], values[3], values[4], values[5]
        )?;
    }
    assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
    assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
    println!(
        "Rust FramePool initialization cycles=20 calls=80 hidden_rejections=40 destroyed_rejections=40 final_device_notifications=40 native_captures=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
