// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Controlled failed HRESULTs at live interfaces; no physical device fault.
use super::super::*;
use super::native_map::save;
use crate::{
    backends::windows::parity_adapter::{Adapter, Converter, Image},
    platform::windows::desktop_dup::map_probe,
    test_support::native_module::Module,
};
use std::{
    error::Error as StdError,
    ffi::c_void,
    fs,
    io::Write,
    time::{Duration, Instant},
};
struct ObservedConverter<'a>(Converter<'a>);
impl BgraConverter for ObservedConverter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, view: BgraView<'_>) -> Result<Image, i32> {
        assert!(!map_probe::conversion());
        self.0.bgra_to_bgr(view)
    }
}
const CODES: [HRESULT; 3] = [
    DXGI_ERROR_ACCESS_LOST,
    DXGI_ERROR_WAIT_TIMEOUT,
    DXGI_ERROR_DEVICE_REMOVED,
];
struct Watch {
    attach: unsafe extern "C" fn(*mut c_void, u32, u32) -> i32,
    read: unsafe extern "C" fn(u32, *mut u64, usize) -> i32,
    clock: unsafe extern "C" fn() -> u64,
}
impl Watch {
    fn load() -> Result<Self, Box<dyn StdError>> {
        let path = std::env::var_os("MFS_D3D_LIFETIME_LIBRARY")
            .ok_or("missing native lifetime observer")?;
        let module = std::mem::ManuallyDrop::new(Module::load(&path)?);
        // SAFETY: Exact d3d-lifetime.h ABI; code/global callback records stay
        // loaded to process exit even on failed assertions or registrations.
        let version =
            unsafe { module.symbol::<unsafe extern "C" fn() -> u32>(c"mfs_d3d_lifetime_version")? };
        assert_eq!(unsafe { version() }, 1);
        let widths = unsafe {
            module.symbol::<unsafe extern "C" fn(*mut u32, usize) -> i32>(
                c"mfs_d3d_lifetime_widths",
            )?
        };
        let mut values = [0; 6];
        assert_eq!(unsafe { widths(values.as_mut_ptr(), values.len()) }, 0);
        assert_eq!(values, [8, 8, 8, 4, 4, 4]);
        Ok(Self {
            attach: unsafe { module.symbol(c"mfs_d3d_lifetime_attach")? },
            read: unsafe { module.symbol(c"mfs_d3d_lifetime_read")? },
            clock: unsafe { module.symbol(c"mfs_d3d_lifetime_clock")? },
        })
    }
    fn now(&self) -> u64 {
        unsafe { (self.clock)() }
    }
    fn record(&self, index: u32) -> [u64; 12] {
        let mut row = [0; 12];
        // SAFETY: Fixed process-lifetime atomic record and exact writable span.
        assert_eq!(
            unsafe { (self.read)(index, row.as_mut_ptr(), row.len()) },
            0
        );
        row
    }
    fn snapshot(&self, out: &mut fs::File, cycle: u32, phase: u32) -> std::io::Result<()> {
        for kind in 0..4 {
            write!(out, "{cycle}\t{phase}\t{kind}")?;
            for value in self.record(cycle * 4 + kind) {
                write!(out, "\t{value}")?;
            }
            writeln!(out)?;
        }
        out.flush()
    }
    fn wait(&self, cycle: u32, phase: u32, access_lost: bool) -> bool {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            let done = (0..4).all(|kind| {
                self.record(cycle * 4 + kind)[4]
                    == u64::from(phase == 3 || access_lost && phase >= 1 && kind >= 2)
            });
            if done || Instant::now() >= end {
                return done;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
fn state(c: &DesktopDupCapture) -> [u64; 12] {
    let address = |p: *mut c_void| p as usize as u64;
    let pointers = [
        c.device.as_ref().map_or(0, |v| address(v.as_raw())),
        c.context.as_ref().map_or(0, |v| address(v.as_raw())),
        c.factory.as_ref().map_or(0, |v| address(v.as_raw())),
        c.adapter.as_ref().map_or(0, |v| address(v.as_raw())),
        c.output.as_ref().map_or(0, |v| address(v.as_raw())),
        c.duplication.as_ref().map_or(0, |v| address(v.as_raw())),
        c.readable.as_ref().map_or(0, |v| address(v.as_raw())),
    ];
    let slots = pointers
        .iter()
        .enumerate()
        .fold(0_u64, |mask, (i, p)| mask | (u64::from(*p != 0) << i));
    [
        slots,
        address(c.monitor.0),
        u64::from(c.just_initialized),
        u64::from(c.description.Width),
        u64::from(c.description.Height),
        pointers[0],
        pointers[1],
        pointers[2],
        pointers[3],
        pointers[4],
        pointers[5],
        pointers[6],
    ]
}
fn save_state(out: &mut fs::File, cycle: u32, phase: u32, row: &[u64; 12]) -> std::io::Result<()> {
    write!(out, "{cycle}\t{phase}")?;
    for value in row {
        write!(out, "\t{value}")?;
    }
    writeln!(out)?;
    out.flush()
}
#[test]
#[ignore = "requires exact original controlled Acquire HRESULTs, live native ownership and serial visible desktop"]
fn matches_access_lost_partial_release_next_call_rebuild_and_retained_other_errors()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_ACQUIRE_ERRORS_ROOT").ok_or("missing Acquire errors root")?,
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
    let mut abi = fs::File::create_new(directory.join("abi.tsv"))?;
    write!(abi, "1\t8\t8\t8\t4\t4\t4")?;
    for code in CODES {
        write!(abi, "\t{}", code.0 as u32)?;
    }
    writeln!(abi)?;
    drop(abi);
    let mut calls = fs::File::create_new(directory.join("calls.tsv"))?;
    let mut states = fs::File::create_new(directory.join("states.tsv"))?;
    let mut lifetimes = fs::File::create_new(directory.join("lifetimes.tsv"))?;
    writeln!(
        calls,
        "cycle\tmode\tcode\tempty\torder\tacquires\tnative_acquires\tinjected\tcopies\tmaps\tunmaps\treleases\tconversions\ttimeout\tviolation\tcaller\tacquire_duplication\tbegin_fault\tend_fault\tend_observation\tend_recovery\tend_recovery_observation\tend_close\tend_close_observation"
    )?;
    writeln!(
        states,
        "cycle\tphase\tslots\tmonitor\tjust_initialized\twidth\theight\tdevice\tcontext\tfactory\tadapter\toutput\tduplication\tstaging"
    )?;
    writeln!(
        lifetimes,
        "cycle\tphase\tkind\tattempted\tmethod\tstored_kind\tregistered\tcallbacks\tattach_thread\tcallback_thread\tattach_qpc\tcallback_qpc\tcallback_id\tobject\tattach_hr"
    )?;
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let mut held = Vec::new();
    held.try_reserve_exact(40)?;
    for cycle in 0..20_u32 {
        let mode = cycle % 3;
        let lost = mode == 0;
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.start_inside_desktop_updates()?;
        let geometry = fixture.geometry([0; 6])?;
        let caller = fixture.thread_info()?[1];
        let mut capture = DesktopDupCapture::new(fixture.hwnd());
        let mut converter = ObservedConverter(adapter.converter(0));
        let image = capture
            .capture(&mut converter)
            .image
            .map_err(|e| format!("Acquire errors warm {e:?}"))?;
        assert!(!capture.just_initialized && capture.readable.is_some());
        let coordinates = capture
            .output_coordinates()
            .map_err(|e| format!("Acquire errors output {e:?}"))?;
        let x = usize::try_from(geometry[16].checked_sub(i64::from(coordinates[0])).unwrap())?;
        let y = usize::try_from(geometry[17].checked_sub(i64::from(coordinates[1])).unwrap())?;
        save(&directory, &format!("{cycle}-warm"), &image, x, y)?;
        held.push((format!("{cycle}-warm-held"), image, x, y));
        let before = state(&capture);
        let objects = [
            capture.device.as_ref().unwrap().as_raw(),
            capture.context.as_ref().unwrap().as_raw(),
            capture.duplication.as_ref().unwrap().as_raw(),
            capture.readable.as_ref().unwrap().as_raw(),
        ];
        for (kind, object) in objects.into_iter().enumerate() {
            // SAFETY: Same live concrete native objects on the owning caller;
            // registration retains no object reference after returning.
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
        save_state(&mut states, cycle, 0, &before)?;
        watch.snapshot(&mut lifetimes, cycle, 0)?;
        let guard = map_probe::Guard::begin(mode + 3);
        let begin = watch.now();
        let rejected = capture.capture(&mut converter);
        let end = watch.now();
        let trace = guard.trace();
        drop(guard);
        let failed = state(&capture);
        save_state(&mut states, cycle, 1, &failed)?;
        let fault_owners = watch.wait(cycle, 1, lost);
        let observed = watch.now();
        watch.snapshot(&mut lifetimes, cycle, 1)?;
        assert!(fault_owners);
        assert!(match rejected.image {
            Err(Error::Capture(CaptureError::AccessLost)) => lost,
            Err(Error::Capture(CaptureError::Native("AcquireNextFrame", code))) =>
                !lost && code == CODES[usize::try_from(mode)?],
            _ => false,
        });
        assert_eq!(
            (
                trace.order,
                trace.acquires,
                trace.native_acquires,
                trace.injected_acquire
            ),
            (1, 1, 0, true)
        );
        assert_eq!(
            (
                trace.copies,
                trace.maps,
                trace.unmaps,
                trace.releases,
                trace.conversions
            ),
            (0, 0, 0, 0, 0)
        );
        assert_eq!(trace.acquired, CODES[usize::try_from(mode)?].0);
        assert_eq!(trace.timeout, 2000);
        assert!(!trace.violation && !trace.held && !trace.mapped);
        assert_eq!(u64::from(trace.thread), caller);
        assert_eq!(trace.duplication as u64, before[10]);
        assert_eq!((trace.context, trace.resource), (0, 0));
        assert_eq!(rejected.evidence.sample_count, 1);
        assert_eq!(rejected.evidence.warmups, 0);
        assert_eq!(
            rejected.evidence.samples[0].acquire,
            Some(CODES[usize::try_from(mode)?])
        );
        assert!(
            rejected.evidence.samples[0].frame.is_none()
                && rejected.evidence.samples[0].release.is_none()
        );
        assert_eq!(rejected.evidence.samples[0].copied_bytes, 0);
        let image = capture
            .capture(&mut converter)
            .image
            .map_err(|e| format!("Acquire errors next-operation capture {e:?}"))?;
        let recovered = watch.now();
        assert!(!capture.just_initialized && capture.readable.is_some());
        save(&directory, &format!("{cycle}-recovery"), &image, x, y)?;
        held.push((format!("{cycle}-recovery-held"), image, x, y));
        save_state(&mut states, cycle, 2, &state(&capture))?;
        let recovery_owners = watch.wait(cycle, 2, lost);
        let recovery_observed = watch.now();
        watch.snapshot(&mut lifetimes, cycle, 2)?;
        assert!(recovery_owners);
        capture.uninit();
        let closed = watch.now();
        save_state(&mut states, cycle, 3, &state(&capture))?;
        let closed_owners = watch.wait(cycle, 3, lost);
        let close_observed = watch.now();
        watch.snapshot(&mut lifetimes, cycle, 3)?;
        assert!(closed_owners);
        drop(capture);
        fixture.close()?;
        writeln!(
            calls,
            "{cycle}\t{mode}\t{}\t1\t1\t1\t0\t1\t0\t0\t0\t0\t0\t2000\t0\t{caller}\t{}\t{begin}\t{end}\t{observed}\t{recovered}\t{recovery_observed}\t{closed}\t{close_observed}",
            trace.acquired as u32, trace.duplication
        )?;
        calls.flush()?;
        assert_eq!(adapter.image_owners()?, u64::from((cycle + 1) * 2));
    }
    for (name, image, x, y) in &held {
        save(&directory, name, image, *x, *y)?;
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust DesktopDup acquire_errors cycles=20 public_calls=60 controlled_failures=20 access_lost=7 retained_other_errors=13 recoveries=20 observed_old_objects=80 retained_cpu=40 physical_device_faults=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
