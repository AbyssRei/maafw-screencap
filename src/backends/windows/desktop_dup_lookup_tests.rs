// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual monitor lookup/fallback and temporary COM ownership, without frames.
use super::super::*;
use crate::{platform::windows::desktop_dup::output_probe, test_support::native_module::Module};
use std::{
    error::Error as StdError,
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

const FIELDS: usize = 38;
const E_FAIL_BITS: u32 = 0x8000_4005;
const E_NOINTERFACE_BITS: u32 = 0x8000_4002;
const E_ABORT_BITS: u32 = 0x8000_4004;
const NOT_FOUND_BITS: u32 = 0x887a_0002;
struct Oracle {
    begin: unsafe extern "C" fn(u32, u32, u32) -> i32,
    created: output_probe::Created,
    factory: output_probe::Factory,
    end: output_probe::End,
    trace: unsafe extern "C" fn(*mut u64, usize) -> i32,
    read: unsafe extern "C" fn(u32, *mut u64, usize) -> i32,
    clock: unsafe extern "C" fn() -> u64,
    target_create: unsafe extern "C" fn(*mut u64, usize) -> i32,
    target_close: unsafe extern "C" fn(*mut u64, usize) -> i32,
}
impl Oracle {
    fn load() -> Result<Self, Box<dyn StdError>> {
        let probe =
            std::env::var_os("MFS_DESKTOP_LOOKUP_LIBRARY").ok_or("missing output oracle module")?;
        let life =
            std::env::var_os("MFS_D3D_LIFETIME_LIBRARY").ok_or("missing output lifetime module")?;
        // Keep native code/records loaded even if a failed test retains a COM
        // reference or destruction callback beyond ordinary stack unwinding.
        let probe = std::mem::ManuallyDrop::new(Module::load(&probe)?);
        let life = std::mem::ManuallyDrop::new(Module::load(&life)?);
        // SAFETY: Exact private headers, retained modules and fixed writable widths.
        unsafe {
            let version =
                life.symbol::<unsafe extern "C" fn() -> u32>(c"mfs_d3d_lifetime_version")?;
            assert_eq!(version(), 1);
            let widths = life.symbol::<unsafe extern "C" fn(*mut u32, usize) -> i32>(
                c"mfs_d3d_lifetime_widths",
            )?;
            let mut values = [0; 6];
            assert_eq!(widths(values.as_mut_ptr(), values.len()), 0);
            assert_eq!(values, [8, 8, 8, 4, 4, 4]);
            Ok(Self {
                begin: probe.symbol(c"mfs_dd_lookup_begin")?,
                created: probe.symbol(c"mfs_dd_lookup_created")?,
                factory: probe.symbol(c"mfs_dd_lookup_factory")?,
                end: probe.symbol(c"mfs_dd_lookup_end")?,
                trace: probe.symbol(c"mfs_dd_lookup_trace")?,
                read: life.symbol(c"mfs_d3d_lifetime_read")?,
                clock: life.symbol(c"mfs_d3d_lifetime_clock")?,
                target_create: probe.symbol(c"mfs_dd_lookup_target_create")?,
                target_close: probe.symbol(c"mfs_dd_lookup_target_close")?,
            })
        }
    }
    fn now(&self) -> u64 {
        // SAFETY: Fixed scalar query on the process-lifetime module.
        unsafe { (self.clock)() }
    }
    fn trace(&self) -> [u64; FIELDS] {
        let mut values = [0; FIELDS];
        // SAFETY: Fixed header capacity, exact writable array, same caller scope.
        assert_eq!(
            unsafe { (self.trace)(values.as_mut_ptr(), values.len()) },
            0
        );
        values
    }
    fn life(&self, index: u32) -> [[u64; 12]; 2] {
        let mut rows = [[0; 12]; 2];
        for (kind, row) in rows.iter_mut().enumerate() {
            // SAFETY: Forty once-only contexts with two in-range native records.
            assert_eq!(
                unsafe {
                    (self.read)(
                        index * 2 + u32::try_from(kind).unwrap(),
                        row.as_mut_ptr(),
                        row.len(),
                    )
                },
                0
            );
        }
        rows
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
    .fold(0, |mask, (i, present)| mask | (u32::from(present) << i))
}
fn trace(
    out: &mut fs::File,
    cycle: u32,
    control: u32,
    phase: u32,
    row: [u64; FIELDS],
) -> std::io::Result<()> {
    write!(out, "{cycle}\t{control}\t{phase}")?;
    for value in row {
        write!(out, "\t{value}")?;
    }
    writeln!(out)?;
    out.flush()
}
fn lifetimes(
    out: &mut fs::File,
    cycle: u32,
    control: u32,
    phase: u32,
    rows: [[u64; 12]; 2],
) -> std::io::Result<()> {
    for (kind, row) in rows.into_iter().enumerate() {
        write!(out, "{cycle}\t{control}\t{phase}\t{kind}")?;
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

fn target_row(out: &mut fs::File, phase: u32, values: [u64; 7]) -> std::io::Result<()> {
    write!(out, "{phase}")?;
    for value in values {
        write!(out, "\t{value}")?;
    }
    writeln!(out)?;
    out.flush()
}

#[test]
#[ignore = "explicit real hidden HWND/monitor/COM lookup controls; zero screenshots"]
fn releases_temporary_lookup_owners_before_primary_fallback_and_retains_selected_monitor()
-> Result<(), Box<dyn StdError>> {
    let text = std::env::var("MFS_DESKTOP_LOOKUP_MODE")?;
    let mode: u32 = text.parse()?;
    assert!(mode <= 4 && text == mode.to_string());
    let root = PathBuf::from(
        std::env::var_os("MFS_DESKTOP_LOOKUP_ROOT").ok_or("missing lookup matrix root")?,
    )
    .join("rust");
    fs::create_dir(&root)?;
    let oracle = Oracle::load()?;
    let mut abi = fs::File::create_new(root.join("abi.tsv"))?;
    writeln!(
        abi,
        "1\t{mode}\t{FIELDS}\t8\t8\t8\t4\t4\t4\t{E_FAIL_BITS}\t{E_NOINTERFACE_BITS}\t{E_ABORT_BITS}\t{NOT_FOUND_BITS}"
    )?;
    drop(abi);
    let mut calls = fs::File::create_new(root.join("calls.tsv"))?;
    let mut traces = fs::File::create_new(root.join("trace.tsv"))?;
    let mut lives = fs::File::create_new(root.join("lifetimes.tsv"))?;
    let mut targets = fs::File::create_new(root.join("target.tsv"))?;
    let mut evidence = fs::File::create_new(root.join("evidence.tsv"))?;
    writeln!(
        calls,
        "cycle\tcontrol\tsuccess\tbefore_slots\tafter_slots\tbefore_monitor\tafter_monitor\tsamples\tconversions\tsurface_code\tsurface_error\treturned\tclosed\tobserved"
    )?;
    writeln!(traces, "cycle\tcontrol\tphase\tfields")?;
    writeln!(lives, "cycle\tcontrol\tphase\tkind\tfields")?;
    writeln!(
        targets,
        "phase\thwnd\tmonitor\tcaller\towner\tvisible\tvalid\tresolved"
    )?;
    writeln!(
        evidence,
        "cycle\tcontrol\tadapters\toutputs\tfallback\tlookup_error_present\tlookup_error\tmonitor_failed"
    )?;
    let mut target = [0_u64; 7];
    // SAFETY: Retained private DLL, exact seven initialized writable scalars.
    HRESULT(unsafe { (oracle.target_create)(target.as_mut_ptr(), target.len()) }).ok()?;
    target_row(&mut targets, 0, target)?;
    assert!(target[0] != 0 && target[1] != 0);
    assert_eq!(target[2], target[3]);
    assert_eq!((target[4], target[5], target[6]), (0, 1, target[1]));
    for cycle in 0..20_u32 {
        for control in 0..2_u32 {
            let index = cycle * 2 + control;
            // SAFETY: Once-only context and matching retained DLL on this caller.
            HRESULT(unsafe { (oracle.begin)(mode, control, index) }).ok()?;
            let guard = output_probe::Guard::begin(oracle.created, oracle.factory, oracle.end);
            let window = usize::try_from(target[0])? as *mut std::ffi::c_void;
            let mut capture = DesktopDupCapture::new(window);
            let mut converter = NeverConvert::default();
            let attempt = capture.capture(&mut converter);
            let returned = oracle.now();
            let before = slots(&capture);
            let before_monitor = capture.monitor.0 as usize as u64;
            let (name, code) = match attempt.image {
                Err(Error::Capture(CaptureError::Native(name, code))) => {
                    (name, code.0.cast_unsigned())
                }
                value => {
                    return Err(format!("expected lookup sentinel rejection: {value:?}").into());
                }
            };
            let before_trace = oracle.trace();
            trace(&mut traces, cycle, control, 0, before_trace)?;
            let live = oracle.life(index);
            lifetimes(&mut lives, cycle, control, 0, live)?;
            capture.uninit();
            let closed = oracle.now();
            let after_trace = oracle.trace();
            trace(&mut traces, cycle, control, 1, after_trace)?;
            let deadline = Instant::now() + Duration::from_secs(2);
            let final_rows = loop {
                let rows = oracle.life(index);
                if rows.iter().all(|r| r[4] == 1) || Instant::now() >= deadline {
                    break rows;
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            lifetimes(&mut lives, cycle, control, 1, final_rows)?;
            let observed = oracle.now();
            let after = slots(&capture);
            let after_monitor = capture.monitor.0 as usize as u64;
            guard.finish()?;
            writeln!(
                calls,
                "{cycle}\t{control}\t0\t{before}\t{after}\t{before_monitor}\t{after_monitor}\t{}\t{}\t{code}\t{name}\t{returned}\t{closed}\t{observed}",
                attempt.evidence.sample_count, converter.0
            )?;
            writeln!(
                evidence,
                "{cycle}\t{control}\t{}\t{}\t{}\t{}\t{}\t{}",
                attempt.evidence.adapters_seen,
                attempt.evidence.outputs_seen,
                u8::from(attempt.evidence.primary_fallback),
                u8::from(attempt.evidence.lookup_error.is_some()),
                attempt
                    .evidence
                    .lookup_error
                    .map_or(0, |e| e.0.cast_unsigned()),
                u8::from(attempt.evidence.monitor_query_failed)
            )?;
            calls.flush()?;
            evidence.flush()?;
            assert_eq!((name, code), ("DuplicateOutput", E_ABORT_BITS));
            assert_eq!((before, after), (31, 0));
            assert_eq!(before_monitor, if control == 1 { target[1] } else { 0 });
            assert_eq!(after_monitor, before_monitor);
            assert_eq!(
                (
                    attempt.evidence.sample_count,
                    converter.0,
                    attempt.evidence.warmups
                ),
                (0, 0, 0)
            );
            assert!(!capture.just_initialized && !attempt.evidence.monitor_query_failed);
            assert_eq!(attempt.evidence.primary_fallback, control == 0);
            assert_eq!(
                attempt.evidence.adapters_seen,
                u32::from(control == 1 || mode != 0)
            );
            assert_eq!(
                attempt.evidence.outputs_seen,
                u32::from(control == 1 || mode >= 2)
            );
            let lookup = if control == 1 {
                None
            } else {
                Some(HRESULT(
                    (match mode {
                        0 => E_FAIL_BITS,
                        3 => E_NOINTERFACE_BITS,
                        _ => NOT_FOUND_BITS,
                    })
                    .cast_signed(),
                ))
            };
            assert_eq!(attempt.evidence.lookup_error, lookup);
            assert_eq!((before_trace[3], after_trace[3]), (0, 0));
            assert!(live.iter().all(|r| r[0] == 1 && r[3] == 1 && r[4] == 0));
            assert!(final_rows.iter().all(|r| r[4] == 1));
        }
    }
    // SAFETY: All capture/facade scopes ended; same owning caller and module.
    HRESULT(unsafe { (oracle.target_close)(target.as_mut_ptr(), target.len()) }).ok()?;
    target_row(&mut targets, 1, target)?;
    assert_eq!(target[3..], [0, 0, 0, 0]);
    println!(
        "native Rust DesktopDup lookup mode={mode} cycles=20 public_rejections=40 fallbacks=20 matching_controls=20 d3d_notifications=80 captures=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
