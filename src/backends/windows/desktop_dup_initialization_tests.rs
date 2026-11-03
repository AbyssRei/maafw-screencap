// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Creation failures with actual device/context notifications, no desktop.
use super::super::*;
use crate::{platform::windows::desktop_dup::init_probe, test_support::native_module::Module};
use std::{
    error::Error as StdError,
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Watch {
    attach: init_probe::Attach,
    read: init_probe::Read,
    clock: unsafe extern "C" fn() -> u64,
}
impl Watch {
    fn load() -> Result<Self, Box<dyn StdError>> {
        let path = std::env::var_os("MFS_D3D_LIFETIME_LIBRARY")
            .ok_or("missing initialization observer")?;
        // Native callbacks and their static records must remain loaded even if
        // a test fails before all observed native owners finish destruction.
        let module = std::mem::ManuallyDrop::new(Module::load(&path)?);
        // SAFETY: Exact d3d-lifetime.h, process-lifetime module and writable ABI widths.
        unsafe {
            let version =
                module.symbol::<unsafe extern "C" fn() -> u32>(c"mfs_d3d_lifetime_version")?;
            assert_eq!(version(), 1);
            let widths = module.symbol::<unsafe extern "C" fn(*mut u32, usize) -> i32>(
                c"mfs_d3d_lifetime_widths",
            )?;
            let mut values = [0; 6];
            assert_eq!(widths(values.as_mut_ptr(), values.len()), 0);
            assert_eq!(values, [8, 8, 8, 4, 4, 4]);
            Ok(Self {
                attach: module.symbol(c"mfs_d3d_lifetime_attach")?,
                read: module.symbol(c"mfs_d3d_lifetime_read")?,
                clock: module.symbol(c"mfs_d3d_lifetime_clock")?,
            })
        }
    }
    fn now(&self) -> u64 {
        // SAFETY: Exact no-argument function in the retained native module.
        unsafe { (self.clock)() }
    }
    fn record(&self, index: u32) -> [u64; 12] {
        let mut row = [0; 12];
        // SAFETY: In-range immutable native slot and exact writable output span.
        assert_eq!(
            unsafe { (self.read)(index, row.as_mut_ptr(), row.len()) },
            0
        );
        row
    }
}
fn slots(c: &DesktopDupCapture) -> u32 {
    [
        c.device.is_some(),
        c.context.is_some(),
        c.factory.is_some(),
        c.adapter.is_some(),
        c.output.is_some(),
        c.duplication.is_some(),
        c.readable.is_some(),
    ]
    .into_iter()
    .enumerate()
    .fold(0, |mask, (index, present)| {
        mask | (u32::from(present) << index)
    })
}
fn life(
    out: &mut fs::File,
    cycle: u32,
    mode: u32,
    phase: u32,
    rows: [[u64; 12]; 2],
) -> std::io::Result<()> {
    for (kind, row) in rows.into_iter().enumerate() {
        write!(out, "{cycle}\t{mode}\t{phase}\t{kind}")?;
        for value in row {
            write!(out, "\t{value}")?;
        }
        writeln!(out)?;
    }
    out.flush()
}
#[derive(Default)]
struct NeverConvert(usize);
impl BgraConverter for NeverConvert {
    type Image = ();
    type Error = ();
    fn bgra_to_bgr(&mut self, _: BgraView<'_>) -> Result<(), ()> {
        self.0 += 1;
        Err(())
    }
}
#[test]
#[ignore = "explicit original/Rust initialization errors and real D3D notifications; zero desktop screenshots"]
fn releases_partial_device_context_after_creation_failure_without_capturing()
-> Result<(), Box<dyn StdError>> {
    let root = PathBuf::from(
        std::env::var_os("MFS_DESKTOP_INITIALIZATION_ROOT")
            .ok_or("missing initialization records root")?,
    )
    .join("rust");
    fs::create_dir(&root)?;
    let watch = Watch::load()?;
    let mut abi = fs::File::create_new(root.join("abi.tsv"))?;
    write!(abi, "2\t8\t8\t8\t4\t4\t4")?;
    for code in init_probe::CODES {
        write!(abi, "\t{}", code.0.cast_unsigned())?;
    }
    writeln!(abi)?;
    drop(abi);
    let mut calls = fs::File::create_new(root.join("calls.tsv"))?;
    let mut rows = fs::File::create_new(root.join("lifetimes.tsv"))?;
    writeln!(
        calls,
        "cycle\tmode\tcontrol\tsuccess\torder\tdevice_calls\tnative_devices\tfactory_calls\tnative_factories\tinjected_code\tsurface_code\tbefore_close_slots\tafter_close_slots\tconversions\tsamples\tcaller\tbegin\treturned\tobserved\tdevice_result\tfactory_result\tviolation"
    )?;
    writeln!(
        rows,
        "cycle\tmode\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr"
    )?;
    for cycle in 0..20_u32 {
        for mode in 0..5_u32 {
            let first = if (2..4).contains(&mode) {
                (cycle * 2 + mode - 2) * 2
            } else {
                0
            };
            let mut capture = DesktopDupCapture::new(std::ptr::null_mut());
            assert_eq!(slots(&capture), 0);
            let guard = init_probe::Guard::begin(mode, first, watch.attach, watch.read);
            let mut converter = NeverConvert::default();
            let begin = watch.now();
            let mut surface_code = 0;
            let mut samples = 0;
            let success = if mode == 4 {
                capture.init().is_ok()
            } else {
                let attempt = capture.capture(&mut converter);
                samples = attempt.evidence.sample_count;
                match attempt.image {
                    Err(Error::Capture(CaptureError::Native(name, code))) => {
                        assert_eq!(
                            name,
                            if mode < 2 {
                                "D3D11CreateDevice"
                            } else {
                                "CreateDXGIFactory"
                            }
                        );
                        surface_code = code.0.cast_unsigned();
                        false
                    }
                    value => value.is_ok(),
                }
            };
            let returned = watch.now();
            let before = slots(&capture);
            let trace = guard.trace();
            drop(guard);
            if (2..4).contains(&mode) {
                let at_return = [watch.record(first), watch.record(first + 1)];
                life(&mut rows, cycle, mode, 0, trace.live)?;
                life(&mut rows, cycle, mode, 1, at_return)?;
                let end = Instant::now() + Duration::from_secs(2);
                let final_rows = loop {
                    let values = [watch.record(first), watch.record(first + 1)];
                    if values.iter().all(|r| r[4] == 1) || Instant::now() >= end {
                        break values;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                };
                life(&mut rows, cycle, mode, 2, final_rows)?;
                assert!(trace.live.iter().all(|r| r[4] == 0));
                assert!(
                    final_rows.iter().all(|r| r[4] == 1),
                    "partial initialization native objects still live"
                );
            }
            let observed = watch.now();
            capture.uninit();
            let after = slots(&capture);
            let injected = if mode < 4 {
                init_probe::CODES[usize::try_from(mode)?].0.cast_unsigned()
            } else {
                0
            };
            writeln!(
                calls,
                "{cycle}\t{mode}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{injected}\t{surface_code}\t{before}\t{after}\t{}\t{samples}\t{}\t{begin}\t{returned}\t{observed}\t{}\t{}\t{}",
                u32::from(mode == 4),
                u32::from(success),
                trace.order,
                trace.device_calls,
                trace.native_devices,
                trace.factory_calls,
                trace.native_factories,
                converter.0,
                trace.caller,
                trace.device_result.cast_unsigned(),
                trace.factory_result.cast_unsigned(),
                u32::from(trace.violation)
            )?;
            calls.flush()?;
            assert!(!trace.violation);
            assert_eq!(success, mode == 4);
            assert_eq!((before, after), (if mode == 4 { 7 } else { 0 }, 0));
            assert_eq!((converter.0, samples), (0, 0));
            assert_eq!(surface_code, injected);
        }
    }
    println!(
        "native Rust DesktopDup initialization cycles=20 public_rejections=80 direct_init_controls=20 native_devices=60 native_factories=20 d3d_notifications=80 captures=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
