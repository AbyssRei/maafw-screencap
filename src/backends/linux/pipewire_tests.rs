// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Step {
    elapsed: u64,
    result: i32,
    event: u32,
    token: u8,
    format: Format,
}
#[derive(Clone, Default)]
struct Plan {
    inactive: bool,
    throws: bool,
    steps: Vec<Step>,
}
#[derive(Default)]
struct Trace {
    locked: bool,
    events: Vec<(u32, i32)>,
    converted_token: u8,
}
struct Fake {
    connected: bool,
    native: bool,
    wanted: bool,
    raw: Option<Rc<u8>>,
    format: Format,
    time: Duration,
    plan: Plan,
    step: usize,
    trace: Rc<RefCell<Trace>>,
}
impl Driver for Fake {
    type Raw = Rc<u8>;
    fn connected(&self) -> bool {
        self.connected
    }
    fn stream_and_loop_exist(&self) -> bool {
        self.native
    }
    fn lock(&mut self) {
        let mut trace = self.trace.borrow_mut();
        assert!(!trace.locked);
        trace.locked = true;
        trace.events.push((1, 0));
    }
    fn unlock(&mut self) {
        let mut trace = self.trace.borrow_mut();
        assert!(trace.locked);
        trace.events.push((2, 0));
        trace.locked = false;
    }
    fn set_active(&mut self, active: bool) {
        let mut trace = self.trace.borrow_mut();
        assert!(trace.locked);
        trace.events.push((3, i32::from(active)));
    }
    fn set_wanted(&mut self, wanted: bool) {
        assert!(self.trace.borrow().locked);
        self.wanted = wanted;
    }
    fn wanted(&self) -> bool {
        self.wanted
    }
    fn latest_is_empty(&self) -> bool {
        self.raw.is_none()
    }
    fn snapshot(&mut self) -> (Option<Rc<u8>>, Format) {
        assert!(self.trace.borrow().locked);
        (self.raw.clone(), self.format)
    }
    fn now(&mut self) -> Duration {
        self.trace.borrow_mut().events.push((4, 0));
        self.time
    }
    fn timed_wait(&mut self, seconds: i32) -> i32 {
        assert!(self.trace.borrow().locked);
        self.trace.borrow_mut().events.push((5, seconds));
        let step = &self.plan.steps[self.step];
        self.step += 1;
        self.time += Duration::from_nanos(step.elapsed);
        match step.event {
            1 => {
                self.raw = (step.token != 0).then(|| Rc::new(step.token));
                self.format = step.format;
                self.wanted = false;
            }
            2 => self.connected = false,
            3 => {
                self.wanted = false;
                self.connected = false;
            }
            4 => self.format = step.format,
            _ => {}
        }
        step.result
    }
}
struct Convert {
    trace: Rc<RefCell<Trace>>,
    throws: bool,
}
impl Converter<Rc<u8>> for Convert {
    type Image = Rc<u8>;
    type Error = ();
    fn convert(&mut self, raw: Option<Rc<u8>>, mode: Conversion) -> Result<Option<Rc<u8>>, ()> {
        let mut trace = self.trace.borrow_mut();
        assert!(!trace.locked);
        trace.events.push((
            6,
            match mode {
                Conversion::Bgra => 1,
                Conversion::Rgba => 2,
                Conversion::Rgb => 3,
                Conversion::Yuy2 => 4,
            },
        ));
        trace.converted_token = raw.as_deref().copied().unwrap_or(0);
        if self.throws || raw.is_none() {
            Err(())
        } else {
            Ok(raw.map(|raw| Rc::new(*raw)))
        }
    }
    fn reuse(&mut self, raw: Option<Rc<u8>>) -> Option<Rc<u8>> {
        assert!(!self.trace.borrow().locked);
        raw
    }
}
fn fake(format: Format, token: u8) -> Fake {
    Fake {
        connected: true,
        native: true,
        wanted: false,
        raw: (token != 0).then(|| Rc::new(token)),
        format,
        time: Duration::ZERO,
        plan: Plan::default(),
        step: 0,
        trace: Rc::default(),
    }
}
fn converter(driver: &Fake) -> Convert {
    Convert {
        trace: driver.trace.clone(),
        throws: driver.plan.throws,
    }
}
fn timeout() -> Plan {
    Plan {
        steps: vec![Step {
            elapsed: 3_000_000_000,
            result: -110,
            event: 0,
            token: 0,
            format: Format::Unknown,
        }],
        ..Plan::default()
    }
}
fn fresh(format: Format, token: u8) -> Plan {
    Plan {
        steps: vec![Step {
            elapsed: 10_000_000,
            result: 0,
            event: 1,
            token,
            format,
        }],
        ..Plan::default()
    }
}
#[test]
fn replays_three_timeouts_then_rejects_and_inactive_resets_the_counter() {
    let mut consumer = Consumer::default();
    let mut driver = fake(Format::Bgr, 9);
    for expected in 1..=4 {
        driver.plan = timeout();
        driver.step = 0;
        let mut convert = converter(&driver);
        let attempt = consumer.capture(&mut driver, &mut convert);
        assert_eq!(consumer.timeouts, expected);
        if expected <= 3 {
            assert_eq!(attempt.image.unwrap().as_deref(), Some(&9));
            assert!(attempt.replayed);
        } else {
            assert_eq!(attempt.image, Err(Error::ConsecutiveTimeouts));
        }
        assert!(!driver.wanted);
        assert!(!driver.trace.borrow().locked);
    }
    consumer.inactive(&mut driver);
    assert_eq!(consumer.timeouts, 0);
    consumer.timeouts = 2;
    driver.native = false;
    consumer.inactive(&mut driver);
    assert_eq!(consumer.timeouts, 2);
    consumer.reset_after_close();
    assert_eq!(consumer.timeouts, 0);
}
#[test]
fn fresh_frame_replaces_the_cache_and_conversion_runs_after_unlock() {
    let mut consumer = Consumer { timeouts: 3 };
    let mut driver = fake(Format::Bgra, 9);
    driver.plan = fresh(Format::Rgba, 12);
    let mut convert = converter(&driver);
    let result = consumer.capture(&mut driver, &mut convert);
    let image = result.image.unwrap().unwrap();
    assert!(!result.replayed);
    assert_eq!(consumer.timeouts, 0);
    assert_eq!(*image, 12);
    assert!(!Rc::ptr_eq(&image, driver.raw.as_ref().unwrap()));
    assert_eq!(driver.trace.borrow().events.last(), Some(&(6, 2)));
    driver.plan = fresh(Format::Bgra, 19);
    driver.step = 0;
    convert.throws = true;
    assert_eq!(
        consumer.capture(&mut driver, &mut convert).image,
        Err(Error::Conversion(()))
    );
    assert!(!driver.trace.borrow().locked);
}
#[test]
fn retains_whole_second_waits_across_spurious_wakeups() {
    let mut driver = fake(Format::Bgr, 0);
    driver.plan = Plan {
        steps: vec![
            Step {
                elapsed: 2_100_000_000,
                result: 0,
                event: 0,
                token: 0,
                format: Format::Bgr,
            },
            Step {
                elapsed: 1_000_000_000,
                result: -110,
                event: 0,
                token: 0,
                format: Format::Bgr,
            },
        ],
        ..Plan::default()
    };
    let mut convert = converter(&driver);
    let result = Consumer::default().capture(&mut driver, &mut convert);
    assert_eq!(result.image, Err(Error::NoCachedFrame));
    let waits: Vec<_> = driver
        .trace
        .borrow()
        .events
        .iter()
        .filter(|(kind, _)| *kind == 5)
        .copied()
        .collect();
    assert_eq!(waits, [(5, 3), (5, 1)]);
}

#[test]
fn rejects_clock_and_counter_overflow_with_wanted_cleared_and_lock_released() {
    let mut driver = fake(Format::Bgr, 9);
    driver.time = Duration::MAX;
    let mut convert = converter(&driver);
    assert_eq!(
        Consumer::default().capture(&mut driver, &mut convert).image,
        Err(Error::ClockOverflow)
    );
    assert!(!driver.wanted);
    assert!(!driver.trace.borrow().locked);
    driver.time = Duration::ZERO;
    driver.plan = timeout();
    let mut consumer = Consumer { timeouts: u32::MAX };
    assert_eq!(
        consumer.capture(&mut driver, &mut convert).image,
        Err(Error::CounterOverflow)
    );
    assert!(!driver.wanted);
    assert!(!driver.trace.borrow().locked);
}

const FORMATS: [Format; 8] = [
    Format::Bgra,
    Format::Bgrx,
    Format::Rgba,
    Format::Rgbx,
    Format::Rgb,
    Format::Bgr,
    Format::Yuy2,
    Format::Unknown,
];
fn format_id(format: Format) -> usize {
    FORMATS.iter().position(|value| *value == format).unwrap()
}
struct Case {
    name: String,
    connected: bool,
    native: bool,
    format: Format,
    token: u8,
    timeouts: u32,
    plans: Vec<Plan>,
}
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for format in FORMATS {
        for token in [0, 13] {
            for throws in [false, true] {
                let mut plan = fresh(format, token);
                plan.throws = throws;
                cases.push(Case {
                    name: format!("fresh-{format:?}-{token}-{throws}"),
                    connected: true,
                    native: true,
                    format,
                    token: 4,
                    timeouts: 3,
                    plans: vec![plan],
                });
            }
        }
        cases.push(Case {
            name: format!("replay-limit-{format:?}"),
            connected: true,
            native: true,
            format,
            token: 14,
            timeouts: 0,
            plans: vec![timeout(); 5],
        });
        cases.push(Case {
            name: format!("empty-cache-{format:?}"),
            connected: true,
            native: true,
            format,
            token: 0,
            timeouts: 0,
            plans: vec![timeout(); 2],
        });
    }
    for event in 0..=4 {
        for result in [0, -110] {
            let plan = Plan {
                steps: vec![Step {
                    elapsed: 3_000_000_000,
                    result,
                    event,
                    token: 17,
                    format: Format::Rgb,
                }],
                ..Plan::default()
            };
            cases.push(Case {
                name: format!("wait-event-{event}-{result}"),
                connected: true,
                native: true,
                format: Format::Bgra,
                token: 6,
                timeouts: 2,
                plans: vec![plan],
            });
        }
    }
    for connected in [false, true] {
        for native in [false, true] {
            cases.push(Case {
                name: format!("inactive-{connected}-{native}"),
                connected,
                native,
                format: Format::Bgr,
                token: 8,
                timeouts: 3,
                plans: vec![Plan {
                    inactive: true,
                    ..Plan::default()
                }],
            });
        }
    }
    cases.push(Case {
        name: "disconnected-preserves-counter".into(),
        connected: false,
        native: true,
        format: Format::Bgr,
        token: 3,
        timeouts: 3,
        plans: vec![Plan::default()],
    });
    cases.push(Case {
        name: "inactive-reactivation-and-new-frame".into(),
        connected: true,
        native: true,
        format: Format::Bgr,
        token: 5,
        timeouts: 2,
        plans: vec![
            timeout(),
            Plan {
                inactive: true,
                ..Plan::default()
            },
            timeout(),
            fresh(Format::Bgr, 21),
            timeout(),
        ],
    });
    for first in [0, 1_000_000_001, 2_100_000_000, 3_000_000_001] {
        let mut plan = timeout();
        plan.steps.insert(
            0,
            Step {
                elapsed: first,
                result: 0,
                event: 0,
                token: 0,
                format: Format::Unknown,
            },
        );
        cases.push(Case {
            name: format!("spurious-{first}"),
            connected: true,
            native: true,
            format: Format::Bgr,
            token: 7,
            timeouts: 0,
            plans: vec![plan],
        });
    }
    cases
}

#[test]
#[ignore = "explicit exact-source C++ control oracle; no native Linux capture"]
fn matches_fixed_source_wait_replay_inactive_and_conversion_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{
        fmt::Write as _,
        io::Write as _,
        process::{Command, Stdio},
    };
    let executable = std::env::var_os("MFS_PIPEWIRE_CONTROL_MODEL")
        .ok_or("explicit source-model executable is required")?;
    assert!(std::path::Path::new(&executable).is_absolute());
    let cases = cases();
    let mut input = format!("{}\n", cases.len());
    let mut expected = Vec::new();
    for case in &cases {
        writeln!(
            input,
            "{} {} {} {} {} {}",
            u8::from(case.connected),
            case.token,
            format_id(case.format),
            case.timeouts,
            u8::from(case.native),
            case.plans.len()
        )?;
        let mut driver = fake(case.format, case.token);
        driver.connected = case.connected;
        driver.native = case.native;
        let mut consumer = Consumer {
            timeouts: case.timeouts,
        };
        for (index, plan) in case.plans.iter().enumerate() {
            writeln!(
                input,
                "{} {} {}",
                u8::from(plan.inactive),
                u8::from(plan.throws),
                plan.steps.len()
            )?;
            for step in &plan.steps {
                writeln!(
                    input,
                    "{} {} {} {} {}",
                    step.elapsed,
                    step.result,
                    step.event,
                    step.token,
                    format_id(step.format)
                )?;
            }
            driver.plan = plan.clone();
            driver.step = 0;
            *driver.trace.borrow_mut() = Trace::default();
            let mut convert = converter(&driver);
            let (status, image) = if plan.inactive {
                consumer.inactive(&mut driver);
                (1, None)
            } else {
                let attempt = consumer.capture(&mut driver, &mut convert);
                match attempt.image {
                    Ok(image) => (1, image),
                    Err(Error::Conversion(())) => (2, None),
                    Err(_) => (0, None),
                }
            };
            let trace = driver.trace.borrow();
            assert!(!trace.locked);
            let shared = image
                .as_ref()
                .zip(driver.raw.as_ref())
                .is_some_and(|(a, b)| Rc::ptr_eq(a, b));
            let mut report = format!(
                "{status} {} {} {} {} {} {} {}",
                consumer.timeouts,
                u8::from(driver.wanted),
                u8::from(driver.connected),
                u8::from(image.is_none()),
                u8::from(shared),
                trace.converted_token,
                driver.step
            );
            for (kind, value) in &trace.events {
                write!(report, " {kind}:{value}")?;
            }
            expected.push((format!("{} operation {index}", case.name), report));
        }
    }
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("oracle stdin missing")?;
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child.wait_with_output()?;
    writer
        .join()
        .map_err(|_| "oracle input writer panicked")??;
    assert!(
        output.status.success(),
        "source oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout)?;
    let lines: Vec<_> = actual.lines().collect();
    assert_eq!(lines.len(), expected.len());
    for ((name, expected), actual) in expected.iter().zip(lines) {
        assert_eq!(expected, actual, "source control case {name}");
    }
    println!(
        "exact-source PipeWire control cases={} operations={}",
        cases.len(),
        expected.len()
    );
    Ok(())
}
