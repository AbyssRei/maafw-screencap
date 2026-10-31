// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real native candidate rejection must preserve the original manager fallback.
use super::*;
use crate::{
    backends::windows::parity_adapter::{
        Adapter, Converter, Image as CpuImage,
        manager::{Script, Step},
    },
    platform::windows::wgc::{self, size_probe},
    test_support::windows_capture_host::{HostInitialization, host},
};
use std::{
    cell::RefCell, error::Error, fs::OpenOptions, io::Write, rc::Rc, sync::atomic::Ordering,
};

#[derive(Clone, Copy)]
struct Rejection {
    class: u32,
    queries: usize,
}
struct Candidate {
    backend: Backend<CpuImage>,
    method: Method,
    observed: Rc<RefCell<Vec<Rejection>>>,
}
impl Unit<Converter<'_>> for Candidate {
    type Image = CpuImage;
    type Failure = system::Failure<i32>;
    type Evidence = system::Evidence<i32>;
    fn capture(
        &mut self,
        converter: &mut Converter<'_>,
    ) -> Call<CpuImage, Self::Failure, Self::Evidence> {
        let guard = (self.method == Method::FramePool).then(|| size_probe::Guard::begin(8));
        let call = self.backend.capture(converter);
        if let Some(guard) = guard {
            let trace = guard.trace();
            drop(guard);
            assert_eq!(trace.calls, 1);
            assert_eq!(
                (
                    trace.queries[0].native_code,
                    trace.queries[0].width,
                    trace.queries[0].height
                ),
                (0, 96, 64)
            );
            assert_eq!(
                (
                    trace.queries[0].returned_width,
                    trace.queries[0].returned_height
                ),
                (0, 64)
            );
            let class = match &call.sample {
                Sample::Failure(FailureClass::NoImage, _) => 0,
                Sample::Failure(FailureClass::Exception, _) => 1,
                Sample::Failure(FailureClass::Safety, _) => 2,
                Sample::Image(_) => 3,
            };
            let system::Evidence::FramePool(evidence) = &call.evidence else {
                panic!("wrong candidate evidence")
            };
            assert_eq!((evidence.polls, evidence.target_copy_bytes), (0, 0));
            assert!(evidence.cleanup.iter().all(Option::is_none));
            self.observed.borrow_mut().push(Rejection {
                class,
                queries: trace.calls,
            });
        }
        call
    }
    fn inactive(&mut self) {
        self.backend.deactivate();
    }
}
struct CandidatesFactory(Rc<RefCell<Vec<Rejection>>>);
impl Factory for CandidatesFactory {
    type Unit = Candidate;
    type Error = system::StartError;
    fn create(&self, method: Method, window: *mut c_void) -> Result<Candidate, Self::Error> {
        Ok(Candidate {
            backend: NativeFactory
                .for_image::<CpuImage>()
                .create(method, window)?,
            method,
            observed: self.0.clone(),
        })
    }
}
fn save(root: &std::path::Path, name: &str, image: &CpuImage) -> Result<(), Box<dyn Error>> {
    let mut pixels = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join(format!("{name}.bgr")))?;
    pixels.write_all(image.bytes())?;
    let mut layout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join(format!("{name}.txt")))?;
    let (w, h) = image.dimensions();
    writeln!(
        layout,
        "{w}\t{h}\t{}\t{}",
        image.stride(),
        image.bytes().len()
    )?;
    Ok(())
}
fn known(image: &CpuImage) {
    assert_eq!(image.dimensions(), (96, 64));
    assert_eq!(image.stride(), 288);
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = y * 288 + x * 3;
            assert_eq!(&image.bytes()[offset..offset + 3], &expected);
        }
    }
}
#[test]
#[ignore = "explicit exact-source manager model plus real native GDI/FramePool rejection and desktop"]
fn continues_to_gdi_after_framepool_rejects_nonpositive_initial_item_size()
-> Result<(), Box<dyn Error>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_MANAGER_SIZE_ROOT").ok_or("missing manager Size root")?,
    );
    std::fs::create_dir(&root)?;
    let mut rows = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("manager.tsv"))?;
    writeln!(
        rows,
        "cycle\tsource_connected\tsource_selected\trust_connected\trust_selected\twarm_class\tmeasured_class\tqueries\tconversions\towners\tcaller_thread\tfixture_thread\tgdi_before\tgdi_after"
    )?;
    let mut finals = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("final.tsv"))?;
    writeln!(
        finals,
        "cycle\towners\tcompletion_live\tclose_errors\tconversions"
    )?;
    let mut apartment = host::APTTYPE::default();
    let mut qualifier = host::APTTYPEQUALIFIER::default();
    // SAFETY: Exact selected test on a fresh caller owns one MTA reference.
    assert_eq!(
        unsafe { host::CoGetApartmentType(&mut apartment, &mut qualifier) },
        host::CO_E_NOTINITIALIZED
    );
    unsafe { host::RoInitialize(host::RO_INIT_MULTITHREADED).ok()? };
    let owned = HostInitialization(true);
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let restore = crate::platform::windows::manager::probe::current();
    struct Dpi(isize);
    impl Drop for Dpi {
        fn drop(&mut self) {
            assert!(crate::platform::windows::manager::probe::set(self.0));
        }
    }
    let _dpi = Dpi(restore);
    let close_baseline =
        crate::backends::windows::frame_pool::DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire);
    let mut held = Vec::new();
    held.try_reserve_exact(20)?;
    for cycle in 0..20 {
        let mut script = Script {
            flags: 3,
            valid: 1,
            dpi_success: 1,
            ..Script::default()
        };
        for step in &mut script.steps[0] {
            *step = Step {
                outcome: 1,
                elapsed_ns: 1_000_000,
                ..Step::default()
            };
        }
        let source = adapter.manager_script(&script)?;
        assert_eq!((source.connect_status, source.selected), (1, 1));
        assert_eq!(source.capture_status, [1, 1]);
        crate::platform::windows::manager::set_per_monitor_v2()
            .map_err(|e| format!("fixture DPI {e}"))?;
        let mut fixture = adapter.render_fixture(96, 64)?;
        let geometry = fixture.geometry([1, i64::from(0x9000_0000_u32), 100, 100, 96, 64])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        let thread = fixture.thread_info()?;
        let objects = adapter.objects();
        let observed = Rc::new(RefCell::new(Vec::new()));
        observed.borrow_mut().try_reserve_exact(2)?;
        let factory = CandidatesFactory(observed.clone());
        let mut manager = Manager::<Candidate>::new(fixture.hwnd(), 3);
        let mut converter = adapter.converter(0);
        let report = manager.connect(&NativePlatform, &factory, &mut converter);
        let observations = observed.borrow();
        let class = |i: usize| observations.get(i).map_or(-1, |r| i64::from(r.class));
        let queries: usize = observations.iter().map(|r| r.queries).sum();
        writeln!(
            rows,
            "{cycle}\t{}\t{}\t{}\t{}\t{}\t{}\t{queries}\t{}\t{}\t{}\t{}\t{objects}\t{}",
            source.connect_status,
            source.selected,
            u8::from(report.failure.is_none()),
            report.selected.map_or(0, |m| m as u32),
            class(0),
            class(1),
            converter.conversions,
            adapter.image_owners()?,
            thread[1],
            thread[0],
            adapter.objects()
        )?;
        rows.flush()?;
        assert!(
            report.failure.is_none(),
            "ordinary initial item-size rejection aborted manager fallback"
        );
        assert_eq!(report.selected, Some(Method::Gdi));
        assert_eq!(observations.len(), 2);
        assert_eq!((class(0), class(1), queries), (0, 0, 2));
        drop(observations);
        assert_eq!(converter.conversions, 2);
        assert!(manager.connected(&NativePlatform));
        let mut output = Image::Empty;
        assert!(matches!(
            manager.screencap(&mut converter, &mut output),
            Publication::Called(Observation {
                outcome: Outcome::Pixels,
                ..
            })
        ));
        assert_eq!(converter.conversions, 3);
        let Image::Pixels(image) = output else {
            return Err("manager missing GDI pixels".into());
        };
        save(&root, &format!("{cycle}"), &image)?;
        known(&image);
        manager.inactive::<Converter<'_>>();
        drop(manager);
        fixture.close()?;
        assert_eq!(wgc::LIVE_COMPLETIONS.load(Ordering::Acquire), 0);
        assert_eq!(
            crate::backends::windows::frame_pool::DROP_CLOSE_ERROR_COUNT.load(Ordering::Acquire),
            close_baseline
        );
        held.push(image);
        assert_eq!(adapter.image_owners()?, u64::try_from(cycle + 1)?);
        writeln!(
            finals,
            "{cycle}\t{}\t0\t0\t{}",
            cycle + 1,
            converter.conversions
        )?;
        finals.flush()?;
    }
    drop(owned);
    for (i, image) in held.iter().enumerate() {
        save(&root, &format!("{i}-held"), image)?;
        known(image);
    }
    drop(held);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native manager initial_size_rejection cycles=20 source_model_cases=20 real_framepool_rejections=40 selected_gdi=20 gdi_conversions=60 retained=20 owners_released=1 passed"
    );
    Ok(())
}
