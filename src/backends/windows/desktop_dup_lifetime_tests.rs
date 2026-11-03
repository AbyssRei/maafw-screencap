// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual D3D destruction and DXGI private-owner records after capture shutdown.
use super::super::*;
use crate::{backends::windows::parity_adapter::Adapter, test_support::native_module::Module};
use std::{
    error::Error as StdError,
    ffi::c_void,
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};
type Attach = unsafe extern "C" fn(*mut c_void, u32, u32) -> i32;
type Read = unsafe extern "C" fn(u32, *mut u64, usize) -> i32;
struct Watch {
    attach: Attach,
    read: Read,
    clock: unsafe extern "C" fn() -> u64,
}
impl Watch {
    fn load() -> Result<Self, Box<dyn StdError>> {
        let path = std::env::var_os("MFS_D3D_LIFETIME_LIBRARY")
            .ok_or("missing native lifetime observer")?;
        let module = std::mem::ManuallyDrop::new(Module::load(&path)?);
        // SAFETY: Exact d3d-lifetime.h declarations in this explicit native
        // fixture. Retain its code and global callback records until exit even
        // on a failed test; destruction can run on a driver/runtime thread.
        let version =
            unsafe { module.symbol::<unsafe extern "C" fn() -> u32>(c"mfs_d3d_lifetime_version")? };
        assert_eq!(unsafe { version() }, 1);
        let widths = unsafe {
            module.symbol::<unsafe extern "C" fn(*mut u32, usize) -> i32>(
                c"mfs_d3d_lifetime_widths",
            )?
        };
        let mut actual = [0; 6];
        assert_eq!(unsafe { widths(actual.as_mut_ptr(), actual.len()) }, 0);
        assert_eq!(actual, [8, 8, 8, 4, 4, 4]);
        Ok(Self {
            attach: unsafe { module.symbol(c"mfs_d3d_lifetime_attach")? },
            read: unsafe { module.symbol(c"mfs_d3d_lifetime_read")? },
            clock: unsafe { module.symbol(c"mfs_d3d_lifetime_clock")? },
        })
    }
    fn record(&self, index: u32) -> [u64; 12] {
        let mut values = [0; 12];
        // SAFETY: Fixed native atomic record, exact initialized writable span.
        assert_eq!(
            unsafe { (self.read)(index, values.as_mut_ptr(), values.len()) },
            0
        );
        values
    }
    fn now(&self) -> u64 {
        unsafe { (self.clock)() }
    }
    fn snapshot(&self, rows: &mut fs::File, cycle: u32, phase: u32) -> std::io::Result<()> {
        for kind in 0..4 {
            write!(rows, "{cycle}\t{phase}\t{kind}")?;
            for value in self.record(cycle * 4 + kind) {
                write!(rows, "\t{value}")?;
            }
            writeln!(rows)?;
        }
        rows.flush()
    }
}
fn slots(c: &DesktopDupCapture) -> u32 {
    u32::from(c.device.is_some())
        | (u32::from(c.context.is_some()) << 1)
        | (u32::from(c.factory.is_some()) << 2)
        | (u32::from(c.adapter.is_some()) << 3)
        | (u32::from(c.output.is_some()) << 4)
        | (u32::from(c.duplication.is_some()) << 5)
        | (u32::from(c.readable.is_some()) << 6)
}
#[test]
#[ignore = "requires original DesktopDup lifetime records, observed D3D objects and serial visible desktop"]
fn releases_observed_gpu_objects_while_cpu_images_survive_backend_and_window_shutdown()
-> Result<(), Box<dyn StdError>> {
    let root = PathBuf::from(
        std::env::var_os("MFS_DESKTOP_LIFETIME_ROOT").ok_or("missing lifetime root")?,
    );
    assert!(root.is_absolute());
    assert_eq!(
        fs::read_to_string(root.join("source/calls.tsv"))?
            .lines()
            .count(),
        21
    );
    let directory = root.join("rust");
    fs::create_dir(&directory)?;
    let watch = Watch::load()?;
    fs::write(directory.join("abi.tsv"), b"1\t8\t8\t8\t4\t4\t4\n")?;
    let mut rows = fs::File::create_new(directory.join("lifetimes.tsv"))?;
    let mut calls = fs::File::create_new(directory.join("calls.tsv"))?;
    writeln!(
        rows,
        "cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr"
    )?;
    writeln!(
        calls,
        "cycle\theld\tbefore_slots\tafter_slots\tcaller\tbegin_close\tend_close\trelease_qpc\tfinal_qpc"
    )?;
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let mut held = Vec::new();
    held.try_reserve_exact(20)?;
    for cycle in 0..20u32 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.start_inside_desktop_updates()?;
        let threads = fixture.thread_info()?;
        let geometry = fixture.geometry([0; 6])?;
        let mut capture = DesktopDupCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        let image = capture
            .capture(&mut converter)
            .image
            .map_err(|e| format!("lifetime warm {e:?}"))?;
        assert!(!capture.just_initialized && capture.readable.is_some());
        let coordinates = capture
            .output_coordinates()
            .map_err(|e| format!("lifetime output {e:?}"))?;
        let x = usize::try_from(geometry[16].checked_sub(i64::from(coordinates[0])).unwrap())?;
        let y = usize::try_from(geometry[17].checked_sub(i64::from(coordinates[1])).unwrap())?;
        super::native_map::save(&directory, &format!("{cycle}-current"), &image, x, y)?;
        let objects = [
            capture.device.as_ref().unwrap().as_raw(),
            capture.context.as_ref().unwrap().as_raw(),
            capture.duplication.as_ref().unwrap().as_raw(),
            capture.readable.as_ref().unwrap().as_raw(),
        ];
        for (kind, object) in objects.into_iter().enumerate() {
            // SAFETY: Borrow this exact live concrete interface on its caller;
            // successful registration retains no native object/interface ref.
            assert_eq!(
                unsafe {
                    (watch.attach)(
                        object,
                        u32::try_from(kind)?,
                        cycle * 4 + u32::try_from(kind)?,
                    )
                },
                0
            );
        }
        watch.snapshot(&mut rows, cycle, 0)?;
        for kind in 0..4 {
            assert_eq!(watch.record(cycle * 4 + kind)[4], 0);
        }
        let retained = (cycle % 2 != 0).then(|| {
            (
                capture.readable.as_ref().unwrap().clone(),
                capture.device.as_ref().unwrap().clone(),
            )
        });
        let before = slots(&capture);
        let begin = watch.now();
        capture.uninit();
        let end = watch.now();
        let after = slots(&capture);
        watch.snapshot(&mut rows, cycle, 1)?;
        if retained.is_some() {
            assert_eq!(watch.record(cycle * 4)[4], 0);
            assert_eq!(watch.record(cycle * 4 + 3)[4], 0);
        }
        let release_time = watch.now();
        drop(retained);
        let until = Instant::now() + Duration::from_secs(2);
        while (0..4).any(|kind| watch.record(cycle * 4 + kind)[4] != 1) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
        let final_time = watch.now();
        watch.snapshot(&mut rows, cycle, 2)?;
        writeln!(
            calls,
            "{cycle}\t{}\t{before}\t{after}\t{}\t{begin}\t{end}\t{release_time}\t{final_time}",
            cycle % 2,
            threads[1]
        )?;
        calls.flush()?;
        assert_eq!((before, after), (127, 0));
        for kind in 0..4 {
            assert_eq!(
                watch.record(cycle * 4 + kind)[4],
                1,
                "native destruction/association notification did not complete"
            );
        }
        fixture.close()?;
        drop(capture);
        held.push((cycle, image, x, y));
    }
    assert_eq!(adapter.image_owners()?, 20);
    for (cycle, image, x, y) in &held {
        super::native_map::save(&directory, &format!("{cycle}-held"), image, *x, *y)?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust DesktopDup lifetime cycles=20 native_objects=80 d3d_destruction_notifications=60 dxgi_private_owner_releases=20 held_texture_controls=10 held_device_controls=10 retained_cpu=20 extra_flush=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
