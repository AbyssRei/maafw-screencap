// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual GPU destruction after distinct uninit, owning drop and explicit holds.
use super::*;
use crate::{
    backends::windows::parity_adapter::{Adapter, Image},
    test_support::native_module::Module,
};
use std::{
    error::Error as StdError,
    ffi::c_void,
    fs,
    io::Write,
    time::{Duration, Instant},
};
const KINDS: [u32; 3] = [0, 1, 3];
struct Watch {
    attach: unsafe extern "C" fn(*mut c_void, u32, u32) -> i32,
    read: unsafe extern "C" fn(u32, *mut u64, usize) -> i32,
    clock: unsafe extern "C" fn() -> u64,
}
impl Watch {
    fn load() -> Result<Self, Box<dyn StdError>> {
        let path = std::env::var_os("MFS_D3D_LIFETIME_LIBRARY")
            .ok_or("missing actual D3D lifetime observer")?;
        let module = std::mem::ManuallyDrop::new(Module::load(&path)?);
        // SAFETY: Exact private d3d-lifetime.h ABI, on actual live concrete
        // interfaces. Pin fixture code/global atomic records until process
        // exit even on failed registration/assertion; callbacks may run later.
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
        // SAFETY: Valid fixed atomic native record and complete writable array.
        assert_eq!(
            unsafe { (self.read)(index, row.as_mut_ptr(), row.len()) },
            0
        );
        row
    }
    fn snapshot(&self, out: &mut fs::File, cycle: u32, phase: u32) -> std::io::Result<()> {
        for (index, kind) in KINDS.into_iter().enumerate() {
            write!(out, "{cycle}\t{phase}\t{kind}")?;
            for value in self.record(cycle * 3 + u32::try_from(index).unwrap()) {
                write!(out, "\t{value}")?;
            }
            writeln!(out)?;
        }
        out.flush()
    }
    fn wait_all(&self, cycle: u32) {
        let until = Instant::now() + Duration::from_secs(2);
        while (0..3).any(|i| self.record(cycle * 3 + i)[4] != 1) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
fn slots(c: &FramePoolCapture<Image>) -> u32 {
    u32::from(c.device.is_some())
        | (u32::from(c.context.is_some()) << 1)
        | (u32::from(c.swap_chain.is_some()) << 2)
        | (u32::from(c.readable.is_some()) << 3)
        | (u32::from(c.item.is_some()) << 4)
        | (u32::from(c.pool.is_some()) << 5)
        | (u32::from(c.session.is_some()) << 6)
}
#[test]
#[ignore = "requires complete original FramePool lifetimes and serial visible desktop"]
fn retains_gpu_owners_at_uninit_and_releases_them_after_owner_drop_and_explicit_holds()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_FRAMEPOOL_LIFETIME_ROOT").ok_or("missing lifetime root")?,
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
        "cycle\theld\tbefore_slots\tstopped_slots\tcaller\tbegin_stop\tend_stop\tbegin_drop\tend_drop\trelease_qpc\tfinal_qpc\timage\tcache\tstopped_cache"
    )?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Fresh serial caller acquires/releases exactly its own MTA ref.
    assert_eq!(
        unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) },
        host::CO_E_NOTINITIALIZED
    );
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let close_errors = DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut held_images = Vec::new();
    held_images.try_reserve_exact(20)?;
    for cycle in 0..20_u32 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        let geometry = fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        fixture.start_inside_desktop_updates()?;
        let threads = fixture.thread_info()?;
        let mut capture: FramePoolCapture<Image> = FramePoolCapture::new(fixture.hwnd());
        let mut converter = adapter.converter(0);
        let attempt = capture.capture(&mut converter);
        assert!(attempt.evidence.cleanup.iter().all(Option::is_none));
        let image = attempt
            .image
            .map_err(|e| format!("FramePool lifetime capture: {e:?}"))?;
        queries::save(&directory, &format!("{cycle}-current"), Some(&image))?;
        let result_address = image.address();
        let cache_address = capture
            .cached
            .as_ref()
            .ok_or("missing warm cache")?
            .address();
        assert_ne!(result_address, cache_address);
        let objects = [
            capture.device.as_ref().unwrap().as_raw(),
            capture.context.as_ref().unwrap().as_raw(),
            capture.readable.as_ref().unwrap().as_raw(),
        ];
        for (index, (kind, object)) in KINDS.into_iter().zip(objects).enumerate() {
            // SAFETY: Borrow exact live concrete interface on this same caller.
            // Observer retains no reference to the observed native object.
            assert_eq!(
                unsafe { (watch.attach)(object, kind, cycle * 3 + u32::try_from(index)?) },
                0
            );
        }
        watch.snapshot(&mut rows, cycle, 0)?;
        for index in 0..3 {
            assert_eq!(watch.record(cycle * 3 + index)[4], 0);
        }
        let hold = cycle % 2 != 0;
        let retained = hold.then(|| {
            (
                capture.readable.as_ref().unwrap().clone(),
                capture.device.as_ref().unwrap().clone(),
            )
        });
        let before = slots(&capture);
        let begin = watch.now();
        assert!(capture.uninit().cleanup.iter().all(Option::is_none));
        let end = watch.now();
        let stopped = slots(&capture);
        let stopped_cache = capture
            .cached
            .as_ref()
            .ok_or("uninit lost source cache")?
            .address();
        watch.snapshot(&mut rows, cycle, 1)?;
        assert_eq!((before, stopped, stopped_cache), (127, 7, cache_address));
        assert_eq!(watch.record(cycle * 3)[4], 0);
        assert_eq!(watch.record(cycle * 3 + 1)[4], 0);
        if hold {
            assert_eq!(watch.record(cycle * 3 + 2)[4], 0);
        }
        let begin_drop = watch.now();
        drop(capture);
        let end_drop = watch.now();
        if !hold {
            watch.wait_all(cycle);
        }
        watch.snapshot(&mut rows, cycle, 2)?;
        if hold {
            assert_eq!(watch.record(cycle * 3)[4], 0);
            assert_eq!(watch.record(cycle * 3 + 2)[4], 0);
        } else {
            for index in 0..3 {
                assert_eq!(watch.record(cycle * 3 + index)[4], 1);
            }
        }
        let release = watch.now();
        drop(retained);
        watch.wait_all(cycle);
        let final_time = watch.now();
        watch.snapshot(&mut rows, cycle, 3)?;
        writeln!(
            calls,
            "{cycle}\t{}\t{before}\t{stopped}\t{}\t{begin}\t{end}\t{begin_drop}\t{end_drop}\t{release}\t{final_time}\t{result_address}\t{cache_address}\t{stopped_cache}",
            u32::from(hold),
            threads[1]
        )?;
        calls.flush()?;
        for index in 0..3 {
            assert_eq!(watch.record(cycle * 3 + index)[4], 1);
        }
        fixture.close()?;
        assert_eq!(adapter.image_owners()?, u64::from(cycle + 1));
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(wgc::EVENT_CLOSE_ERROR.load(Ordering::Acquire), 0);
        assert_eq!(DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire), close_errors);
        held_images.push((cycle, image));
    }
    drop(owned);
    for (cycle, image) in &held_images {
        queries::save(&directory, &format!("{cycle}-held"), Some(image))?;
    }
    drop(held_images);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust FramePool lifetime cycles=20 public_calls=20 d3d_destruction_notifications=60 explicit_reference_pairs=10 uninit_gpu_slots=7 retained_cpu=20 extra_flush=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
