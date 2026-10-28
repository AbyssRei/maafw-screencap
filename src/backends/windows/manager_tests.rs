// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::windows::parity_adapter::{
    Adapter,
    manager::{Event, Report, Script, Step},
};
use std::{cell::RefCell, rc::Rc};

struct World {
    input: Script,
    report: Report,
    time: u64,
    valid: bool,
}
impl World {
    fn event(&mut self, kind: u32, method: u32, call: u32) {
        let index = usize::try_from(self.report.trace_count).unwrap();
        assert!(index < 256, "script trace capacity");
        self.report.events[index] = Event {
            kind,
            method,
            call,
            reserved: 0,
            time_ns: self.time,
        };
        self.report.trace_count += 1;
    }
}
struct Pixels {
    method: u32,
    call: u32,
    world: Rc<RefCell<World>>,
}
impl Drop for Pixels {
    fn drop(&mut self) {
        if self.method == 255 {
            return;
        } // uninstrumented caller sentinel Mat
        let mut world = self.world.borrow_mut();
        world.event(4, self.method, self.call);
        world.time += world.input.steps[usize::try_from(self.method - 1).unwrap()]
            [usize::try_from(self.call - 1).unwrap()]
        .image_drop_ns;
    }
}
struct FakeUnit {
    method: u32,
    calls: u32,
    world: Rc<RefCell<World>>,
}
impl Drop for FakeUnit {
    fn drop(&mut self) {
        self.world.borrow_mut().event(5, self.method, 0);
    }
}
impl Unit<()> for FakeUnit {
    type Image = Pixels;
    type Failure = ();
    type Evidence = ();
    fn capture(&mut self, _: &mut ()) -> Call<Pixels, (), ()> {
        self.calls += 1;
        assert!(self.calls <= 4);
        let mut world = self.world.borrow_mut();
        world.event(2, self.method, self.calls);
        let step = world.input.steps[usize::try_from(self.method - 1).unwrap()]
            [usize::try_from(self.calls - 1).unwrap()];
        world.time += step.elapsed_ns;
        let sample = match step.outcome {
            0 => Sample::Failure(FailureClass::NoImage, ()),
            1 => {
                world.event(3, self.method, self.calls);
                Sample::Image(Image::Pixels(Pixels {
                    method: self.method,
                    call: self.calls,
                    world: self.world.clone(),
                }))
            }
            2 => Sample::Image(Image::Empty),
            3 => Sample::Failure(FailureClass::Exception, ()),
            _ => unreachable!("known script outcome"),
        };
        Call {
            sample,
            evidence: (),
        }
    }
    fn inactive(&mut self) {
        self.world.borrow_mut().event(9, self.method, 0);
    }
}
struct ScriptPlatform(Rc<RefCell<World>>);
impl Platform for ScriptPlatform {
    type Stamp = u64;
    fn set_per_monitor_v2(&self) -> Result<isize, u32> {
        let mut world = self.0.borrow_mut();
        world.event(7, 0, 0);
        if world.input.dpi_success != 0 {
            Ok(1)
        } else {
            Err(5)
        }
    }
    fn valid(&self, _: *mut c_void) -> bool {
        let mut world = self.0.borrow_mut();
        world.event(8, 0, 0);
        world.valid
    }
    fn now(&self) -> u64 {
        let mut world = self.0.borrow_mut();
        world.event(6, 0, 0);
        world.time
    }
    fn elapsed_millis(&self, start: u64) -> u128 {
        let mut world = self.0.borrow_mut();
        world.event(6, 0, 0);
        u128::from((world.time - start) / 1_000_000)
    }
}
struct ScriptFactory(Rc<RefCell<World>>);
impl Factory for ScriptFactory {
    type Unit = FakeUnit;
    type Error = ();
    fn create(&self, method: Method, _: *mut c_void) -> Result<FakeUnit, ()> {
        let mut world = self.0.borrow_mut();
        world.event(1, method as u32, 0);
        if world.input.fail_create == method as u32 {
            return Err(());
        }
        Ok(FakeUnit {
            method: method as u32,
            calls: 0,
            world: self.0.clone(),
        })
    }
}
fn connect_status<S, F, E>(report: &Connected<S, F, E>) -> u32 {
    match &report.failure {
        None => 1,
        Some(ConnectFailure::Escaped(..) | ConnectFailure::Construction(..)) => 2,
        _ => 0,
    }
}
fn capture_status<F, E>(report: &Publication<F, E>) -> u32 {
    match report {
        Publication::NoUnit => 0,
        Publication::Called(Observation {
            outcome: Outcome::Pixels | Outcome::Empty,
            ..
        }) => 1,
        Publication::Called(Observation {
            outcome: Outcome::Failure(FailureClass::NoImage, _),
            ..
        }) => 0,
        _ => 2,
    }
}
fn token(image: &Image<Pixels>) -> u32 {
    match image {
        Image::Empty => 0,
        Image::Pixels(image) => image.method * 256 + image.call,
    }
}
fn rust_script(input: Script) -> Report {
    let world = Rc::new(RefCell::new(World {
        input,
        report: Report::default(),
        time: 0,
        valid: input.valid != 0,
    }));
    {
        let platform = ScriptPlatform(world.clone());
        let factory = ScriptFactory(world.clone());
        let window = if input.null_window != 0 {
            std::ptr::null_mut()
        } else {
            std::ptr::without_provenance_mut(1)
        };
        let mut manager = Manager::new(window, input.flags);
        let mut output = Image::Pixels(Pixels {
            method: 255,
            call: 255,
            world: world.clone(),
        });
        let report = manager.connect(&platform, &factory, &mut ());
        world.borrow_mut().report.connect_status = connect_status(&report);
        world.borrow_mut().report.selected = manager.selected().map_or(0, |m| m as u32);
        let before = manager.connected(&platform);
        world.borrow_mut().report.connected_before = u32::from(before);
        let capture = manager.screencap(&mut (), &mut output);
        world.borrow_mut().report.capture_status[0] = capture_status(&capture);
        world.borrow_mut().report.output_tokens[0] = token(&output);
        world.borrow_mut().valid = false;
        let after = manager.connected(&platform);
        world.borrow_mut().report.connected_after = u32::from(after);
        let capture = manager.screencap(&mut (), &mut output);
        world.borrow_mut().report.capture_status[1] = capture_status(&capture);
        world.borrow_mut().report.output_tokens[1] = token(&output);
        manager.inactive::<()>();
        world.borrow_mut().valid = input.valid != 0;
        let status = if input.reconnect != 0 {
            connect_status(&manager.connect(&platform, &factory, &mut ()))
        } else {
            3
        };
        world.borrow_mut().report.reconnect_status = status;
        world.borrow_mut().report.selected_after_reconnect =
            manager.selected().map_or(0, |m| m as u32);
    }
    Rc::try_unwrap(world)
        .ok()
        .expect("all script owners released")
        .into_inner()
        .report
}
fn base() -> Script {
    let mut script = Script {
        flags: 63,
        valid: 1,
        dpi_success: 1,
        ..Script::default()
    };
    for (index, steps) in script.steps.iter_mut().enumerate() {
        for step in steps {
            *step = Step {
                outcome: 1,
                elapsed_ns: u64::try_from(index + 1).unwrap() * 1_000_000,
                ..Step::default()
            };
        }
    }
    script
}
#[test]
#[ignore = "explicit fixed-source C++ manager function/Mat ownership scripts; no native desktop acceptance"]
fn matches_source_selection_calls_quantization_failures_outputs_and_owner_release()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = Adapter::load()?;
    let mut cases = Vec::new();
    for flags in 0..64 {
        cases.push((format!("subset-{flags}"), Script { flags, ..base() }));
    }
    cases.push((
        "unknown-only".into(),
        Script {
            flags: 1 << 63,
            ..base()
        },
    ));
    cases.push((
        "unknown-and-known".into(),
        Script {
            flags: u64::MAX,
            ..base()
        },
    ));
    cases.push(("invalid-window".into(), Script { valid: 0, ..base() }));
    cases.push((
        "invalid-none".into(),
        Script {
            flags: 0,
            valid: 0,
            ..base()
        },
    ));
    cases.push((
        "null-none".into(),
        Script {
            flags: 0,
            null_window: 1,
            ..base()
        },
    ));
    cases.push((
        "null-selected".into(),
        Script {
            null_window: 1,
            ..base()
        },
    ));
    cases.push((
        "dpi-failure".into(),
        Script {
            dpi_success: 0,
            ..base()
        },
    ));
    cases.push((
        "reconnect".into(),
        Script {
            reconnect: 1,
            ..base()
        },
    ));
    for method in 0..6 {
        let mut script = base();
        script.steps[method][0].outcome = 0;
        cases.push((format!("warmup-fails-{method}"), script));
        let mut script = base();
        script.steps[method][1].outcome = 0;
        cases.push((format!("measurement-fails-{method}"), script));
        let mut script = base();
        script.fail_create = u32::try_from(method + 1)?;
        cases.push((format!("create-throws-{method}"), script));
        for stage in 0..2 {
            let mut script = base();
            script.steps[method][stage].outcome = 3;
            cases.push((format!("capture-throws-{method}-{stage}"), script));
        }
    }
    let mut script = base();
    for steps in &mut script.steps {
        steps[1].outcome = 0;
    }
    cases.push(("all-measurements-fail".into(), script));
    let mut script = base();
    for steps in &mut script.steps {
        steps[1].outcome = 2;
    }
    cases.push(("empty-is-engaged-success".into(), script));
    let mut script = base();
    for steps in &mut script.steps {
        steps[1].elapsed_ns = 1_000_001;
    }
    // First is slower in nanoseconds but ties after source millisecond
    // truncation. A nanosecond comparator would incorrectly select method 2.
    script.steps[0][1].elapsed_ns = 1_999_999;
    cases.push(("millisecond-tie".into(), script));
    let mut script = base();
    script.steps[0][1].image_drop_ns = 20_000_000;
    cases.push(("temporary-drop-in-measurement".into(), script));
    for first in 0..4 {
        for second in 0..4 {
            let mut script = base();
            script.flags = 1;
            script.steps[0][2].outcome = first;
            script.steps[0][3].outcome = second;
            cases.push((format!("publication-{first}-{second}"), script));
        }
    }
    for (name, input) in &cases {
        println!("manager source script case={name}");
        let source = adapter
            .manager_script(input)
            .map_err(|error| format!("source manager case {name}: {error}"))?;
        let rust = rust_script(*input);
        assert_eq!(rust, source, "exact-source manager case {name}");
    }
    println!("exact-source manager script cases={}", cases.len());
    Ok(())
}

#[test]
#[ignore = "explicit real source/Rust GDI manager, caller DPI, native resource and CPU-owner lifecycles"]
fn matches_source_gdi_manager_across_twenty_complete_lifecycles()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::backends::windows::{
        desktop_dup_window, frame_pool,
        parity_adapter::{Converter, Image as CpuImage},
        pixels::{BgraConverter, BgraView},
        print_window,
    };
    use crate::platform::windows::manager::probe;
    struct Context(isize);
    impl Drop for Context {
        fn drop(&mut self) {
            if !probe::set(self.0) {
                eprintln!("manager test caller DPI restoration failed");
            }
        }
    }
    struct ConverterProbe<'a> {
        inner: Converter<'a>,
        thread: u32,
    }
    impl BgraConverter for ConverterProbe<'_> {
        type Image = CpuImage;
        type Error = i32;
        fn bgra_to_bgr(&mut self, input: BgraView<'_>) -> Result<CpuImage, i32> {
            assert_eq!(probe::thread(), self.thread);
            assert!(probe::is_v2());
            self.inner.bgra_to_bgr(input)
        }
    }
    impl print_window::PrintConverter for ConverterProbe<'_> {
        fn overlay(
            &mut self,
            dst: &mut CpuImage,
            src: &CpuImage,
            rect: print_window::Overlay,
        ) -> Result<(), i32> {
            self.inner.overlay(dst, src, rect)
        }
    }
    impl frame_pool::Converter for ConverterProbe<'_> {
        fn clone_image(&mut self, image: &CpuImage) -> Result<CpuImage, i32> {
            self.inner.clone_image(image)
        }
    }
    impl desktop_dup_window::Converter for ConverterProbe<'_> {
        fn dimensions(&self, image: &CpuImage) -> (i32, i32) {
            self.inner.dimensions(image)
        }
        fn crop_bgr(
            &mut self,
            image: &CpuImage,
            crop: desktop_dup_window::Crop,
        ) -> Result<CpuImage, i32> {
            self.inner.crop_bgr(image, crop)
        }
    }
    let restore = Context(probe::current());
    let thread = probe::thread();
    let adapter = Adapter::load()?;
    let owner_baseline = adapter.image_owners()?;
    let mut retained = Vec::new();
    for cycle in 0..20 {
        // Harness owns a physical-size PMv2 window, then deliberately restores
        // an unaware caller before EACH source/Rust manager connect.
        crate::platform::windows::manager::set_per_monitor_v2()
            .map_err(|e| format!("fixture DPI {e}"))?;
        let mut fixture = adapter.window(37, 23).map_err(|e| format!("fixture {e}"))?;
        let objects = adapter.objects();
        assert!(probe::set(probe::unaware()));
        let (source, evidence) = adapter.manager_gdi_reference(fixture.hwnd())?;
        assert_eq!(
            (evidence.connected, evidence.captured, evidence.selected),
            (1, 1, 1)
        );
        assert_eq!((evidence.calls_after_connect, evidence.calls), (2, 3));
        assert_eq!(evidence.threads, [thread; 3]);
        assert_eq!(evidence.dpi_v2, [1; 3]);
        assert!(probe::is_v2()); // U intentionally leaves the source context.
        assert_eq!(adapter.objects(), objects);
        assert!(probe::set(probe::unaware()));
        let mut converter = ConverterProbe {
            inner: adapter.converter(0),
            thread,
        };
        let mut manager = NativeManager::new(fixture.hwnd(), Method::Gdi.flag());
        let report = manager.connect_native(&mut converter);
        assert!(report.failure.is_none());
        assert!(report.dpi.is_ok());
        assert_eq!(report.selected, Some(Method::Gdi));
        assert!(manager.connected_native());
        assert!(probe::is_v2());
        assert_eq!(converter.inner.conversions, 2);
        let mut output = Image::Empty;
        let publication = manager.screencap(&mut converter, &mut output);
        assert!(matches!(
            publication,
            Publication::Called(Observation {
                outcome: Outcome::Pixels,
                ..
            })
        ));
        assert_eq!(converter.inner.conversions, 3);
        let Image::Pixels(image) = output else {
            return Err("missing manager pixels".into());
        };
        assert_eq!(image.dimensions(), (37, 23));
        assert_eq!(image.bytes(), source.bytes());
        for y in 0..23_usize {
            for x in 0..37_usize {
                let expected = if x < 37 / 3 {
                    [0x1f, 0x53, 0xa7]
                } else if y < 23 / 2 {
                    [0xb9, 0x4d, 0x27]
                } else {
                    [0x3b, 0xe2, 0x81]
                };
                let offset = y * image.stride() + x * 3;
                assert_eq!(&image.bytes()[offset..offset + 3], expected);
            }
        }
        assert_eq!(adapter.objects(), objects);
        manager.inactive_native();
        fixture.close().map_err(|e| format!("fixture close {e}"))?;
        assert!(!manager.connected_native());
        drop(manager);
        let saved = image.bytes().to_vec();
        assert_eq!(source.bytes(), saved);
        retained.push((image, source, saved));
        assert_eq!(
            adapter.image_owners()?,
            owner_baseline + u64::try_from(cycle + 1)? * 2
        );
    }
    drop(adapter);
    for (rust, source, saved) in retained {
        assert_eq!(rust.bytes(), saved);
        assert_eq!(source.bytes(), saved);
    }
    assert!(probe::set(restore.0));
    drop(restore);
    println!(
        "manager GDI source/Rust complete lifecycles=20; calls=2 warmup/measurement + 1 publication each"
    );
    Ok(())
}
