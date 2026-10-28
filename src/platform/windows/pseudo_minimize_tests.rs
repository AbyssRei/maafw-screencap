// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[cfg(feature = "prototype-windows")]
#[path = "../../../tests/upstream-parity/pseudo_adapter.rs"]
mod original;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Event {
    operation: u32,
    error: u32,
    argument: i64,
    result: i64,
}
struct Script {
    scenario: u32,
    style: isize,
    alpha: u8,
    valid: bool,
    iconic: bool,
    foreground: bool,
    reads: u32,
    writes: u32,
    alpha_writes: u32,
    events: Vec<Event>,
}
impl Script {
    fn new(scenario: u32) -> Self {
        let layered = matches!(scenario, 1 | 5 | 10 | 20 | 21);
        Self {
            scenario,
            style: if scenario == 0 { 0 } else { 0x40000 }
                | if layered { WS_EX_LAYERED as isize } else { 0 },
            alpha: if layered { 73 } else { 255 },
            valid: scenario != 3,
            iconic: !matches!(scenario, 2 | 15),
            foreground: false,
            reads: 0,
            writes: 0,
            alpha_writes: 0,
            events: Vec::new(),
        }
    }
    fn event(&mut self, operation: u32, argument: i64, result: i64, error: u32) {
        self.events.push(Event {
            operation,
            argument,
            result,
            error,
        });
    }
    fn error(&self, fail: bool) -> u32 {
        if fail { 0xe0120000 | self.scenario } else { 0 }
    }
}
impl Api for Arc<Mutex<Script>> {
    fn valid(&self, _: Window) -> bool {
        let mut s = locked(self);
        let value = s.valid;
        s.event(5, 0, i64::from(value), 0);
        value
    }
    fn iconic(&self, _: Window) -> bool {
        let mut s = locked(self);
        let value = s.iconic;
        s.event(6, 0, i64::from(value), 0);
        value
    }
    fn foreground(&self, _: Window) -> bool {
        let mut s = locked(self);
        let value = s.foreground;
        s.event(7, 0, i64::from(value), 0);
        value
    }
    fn style(&self, _: Window) -> Result<isize, u32> {
        let mut s = locked(self);
        s.reads += 1;
        let error = s.error((s.scenario == 4 && s.reads == 1) || (s.scenario == 6 && s.reads == 2));
        let value = if error == 0 { s.style } else { 0 };
        s.event(1, 0, i64::try_from(value).unwrap(), error);
        if error == 0 { Ok(value) } else { Err(error) }
    }
    fn set_style(&self, _: Window, value: isize) -> Result<(), u32> {
        let mut s = locked(self);
        s.writes += 1;
        let error = s.error(
            (s.scenario == 7 && s.writes == 1)
                || (matches!(s.scenario, 9 | 20 | 21) && s.writes == 2),
        );
        let previous = if error == 0 { s.style } else { 0 };
        s.event(
            2,
            i64::try_from(value).unwrap(),
            i64::try_from(previous).unwrap(),
            error,
        );
        if error == 0 {
            s.style = value;
            Ok(())
        } else {
            Err(error)
        }
    }
    fn alpha(&self, _: Window) -> Result<u8, u32> {
        let mut s = locked(self);
        let error = s.error(matches!(s.scenario, 5 | 21));
        let value = if error == 0 { s.alpha } else { 0 };
        s.event(3, 0, i64::from(value), error);
        if error == 0 { Ok(value) } else { Err(error) }
    }
    fn set_alpha(&self, _: Window, value: u8) -> Result<(), u32> {
        let mut s = locked(self);
        s.alpha_writes += 1;
        let error = s.error(
            (s.scenario == 8 && s.alpha_writes == 1)
                || (matches!(s.scenario, 10 | 20 | 21) && s.alpha_writes == 2),
        );
        s.event(4, i64::from(value), i64::from(error == 0), error);
        if error == 0 {
            s.alpha = value;
            Ok(())
        } else {
            Err(error)
        }
    }
    fn show(&self, _: Window, minimize: bool) -> bool {
        let mut s = locked(self);
        let result = s.scenario != 11;
        s.event(8, i64::from(minimize), i64::from(result), 0);
        s.iconic = minimize;
        result
    }
    fn sleep(&self, duration: Duration) {
        let mut s = locked(self);
        s.event(9, i64::try_from(duration.as_millis()).unwrap(), 0, 0);
        s.valid = false;
    }
}

fn run_script(scenario: u32) -> (Vec<Event>, [[u64; 5]; 4], Evidence) {
    let api = Arc::try_new(Mutex::new(Script::new(scenario))).unwrap();
    let helper = Helper::with_api(1_usize as HWND, Arc::clone(&api)).unwrap();
    let core = &helper.shared;
    // Explicitly driven scheduling matches the native script's fake std::thread.
    // begin/request_stop/monitor/restore are the production algorithms. Real OS
    // spawn/join ownership is tested separately, not inferred from these traces.
    let snapshot = || {
        let s = locked(&api);
        [
            u64::try_from(s.style).unwrap(),
            u64::from(s.alpha),
            u64::from(core.pseudo.load(Ordering::SeqCst)),
            u64::from(s.iconic),
            u64::from(s.valid),
        ]
    };
    let stop = || {
        if core.request_stop() {
            core.restore_on_stop();
        }
    };
    if scenario == 17 {
        core.revert();
    }
    assert!(core.begin());
    if scenario == 16 {
        assert!(!core.begin());
    }
    let first = snapshot();
    helper.ensure_not_minimized();
    let second = snapshot();
    if matches!(scenario, 12 | 20) && helper.is_pseudo_minimized() {
        helper.revert_pseudo_minimize();
    }
    if matches!(scenario, 13 | 14 | 15 | 19) {
        {
            let mut s = locked(&api);
            s.valid = scenario != 14;
            s.foreground = scenario == 13;
        }
        core.monitor();
        assert!(
            !core.begin(),
            "target loss must not silently restart monitoring"
        );
    }
    if scenario == 16 {
        helper.ensure_not_minimized();
        assert!(!core.begin());
    }
    if scenario == 18 {
        locked(&api).iconic = true;
        helper.ensure_not_minimized();
    }
    let third = snapshot();
    stop();
    if scenario == 16 {
        stop();
    }
    let fourth = snapshot();
    let evidence = helper.evidence();
    let events = std::mem::take(&mut locked(&api).events);
    (events, [first, second, third, fourth], evidence)
}

#[test]
fn preserves_partial_apply_failure_and_distinguishes_inactive_from_stop() {
    let (_, snapshots, evidence) = run_script(8);
    assert_eq!(
        snapshots[3],
        [
            0x40000 | WS_EX_LAYERED as u64 | WS_EX_TRANSPARENT as u64,
            255,
            0,
            1,
            1
        ]
    );
    assert_eq!(evidence.failures, 1);
    assert_eq!(evidence.first.unwrap().operation, Operation::ApplyAlpha);
    let (events, snapshots, _) = run_script(12);
    assert_eq!(snapshots[3], [0x40000, 0, 0, 0, 1]);
    assert!(
        !events
            .iter()
            .any(|event| event.operation == 8 && event.argument == 1)
    );
    let (_, snapshots, evidence) = run_script(20);
    assert_eq!(snapshots[3][2], 0);
    assert_eq!(evidence.failures, 2);
    assert_eq!(evidence.first.unwrap().operation, Operation::RevertStyle);
    assert_eq!(evidence.last.unwrap().operation, Operation::RevertAlpha);
    let (_, _, evidence) = run_script(21);
    assert_eq!(evidence.first.unwrap().operation, Operation::SaveAlpha);
    for operation in [
        Operation::SaveAlpha,
        Operation::RevertStyle,
        Operation::RevertAlpha,
    ] {
        let index = operation.index();
        assert_eq!(
            evidence.first_by_operation[index],
            Some(Failure {
                operation,
                code: 0xe0120015
            })
        );
        assert_eq!(
            evidence.last_by_operation[index],
            evidence.first_by_operation[index]
        );
        assert_eq!(evidence.counts_by_operation[index], 1);
    }
    assert_eq!(evidence.failures, 3);
}

#[test]
fn restores_saved_layered_alpha_and_keeps_monitor_target_loss_semantics() {
    let (_, snapshots, evidence) = run_script(1);
    assert_eq!(snapshots[3], [0x40000 | WS_EX_LAYERED as u64, 73, 0, 1, 1]);
    assert_eq!(evidence.failures, 0);
    for scenario in [13, 14, 15, 19] {
        let (events, snapshots, _) = run_script(scenario);
        assert_eq!(snapshots[3][2], 0);
        assert!(
            !events
                .iter()
                .any(|event| event.operation == 8 && event.argument == 1)
        );
        let sleeps: Vec<_> = events.iter().filter(|event| event.operation == 9).collect();
        assert_eq!(sleeps.len(), usize::from(scenario != 14));
        assert!(sleeps.iter().all(|event| event.argument == 100));
    }
}

struct Gate {
    entered: bool,
    release: bool,
}
#[derive(Clone)]
struct Blocking {
    script: Arc<Mutex<Script>>,
    gate: Arc<(Mutex<Gate>, std::sync::Condvar)>,
}
impl Api for Blocking {
    fn valid(&self, window: Window) -> bool {
        self.script.valid(window)
    }
    fn iconic(&self, window: Window) -> bool {
        self.script.iconic(window)
    }
    fn foreground(&self, window: Window) -> bool {
        self.script.foreground(window)
    }
    fn style(&self, window: Window) -> Result<isize, u32> {
        self.script.style(window)
    }
    fn set_style(&self, window: Window, value: isize) -> Result<(), u32> {
        self.script.set_style(window, value)
    }
    fn alpha(&self, window: Window) -> Result<u8, u32> {
        self.script.alpha(window)
    }
    fn set_alpha(&self, window: Window, value: u8) -> Result<(), u32> {
        self.script.set_alpha(window, value)
    }
    fn show(&self, window: Window, minimize: bool) -> bool {
        self.script.show(window, minimize)
    }
    fn sleep(&self, duration: Duration) {
        assert_eq!(duration, Duration::from_millis(100));
        let (gate, signal) = &*self.gate;
        let mut state = locked(gate);
        state.entered = true;
        signal.notify_all();
        while !state.release {
            state = signal.wait(state).unwrap();
        }
    }
}

#[test]
fn joins_owned_monitor_before_restoring_window_attributes() {
    let api = Blocking {
        script: Arc::try_new(Mutex::new(Script::new(1))).unwrap(),
        gate: Arc::try_new((
            Mutex::new(Gate {
                entered: false,
                release: false,
            }),
            std::sync::Condvar::new(),
        ))
        .unwrap(),
    };
    let mut helper = Helper::with_api(1_usize as HWND, api.clone()).unwrap();
    helper.start().unwrap();
    let (gate, signal) = &*api.gate;
    {
        let (state, timeout) = signal
            .wait_timeout_while(locked(gate), Duration::from_secs(5), |state| !state.entered)
            .unwrap();
        if timeout.timed_out() && !state.entered {
            drop(state);
            // Release even on a failed premise, so the owner can still join.
            locked(gate).release = true;
            signal.notify_all();
            panic!("monitor did not reach its original sleep boundary");
        }
    }
    helper.start().unwrap(); // no duplicate native snapshot or monitor
    helper.ensure_not_minimized();
    let was_pseudo = helper.is_pseudo_minimized();
    let core = Arc::clone(&helper.shared);
    let release_api = api.clone();
    let releaser = thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while core.running.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            thread::yield_now();
        }
        let stop_requested = !core.running.load(Ordering::SeqCst);
        // While the worker is deliberately blocked, a correctly ordered stop
        // has not yet restored styles or the pseudo flag.
        let retained = core.pseudo.load(Ordering::SeqCst) && locked(&release_api.script).alpha == 0;
        if !stop_requested {
            locked(&release_api.script).valid = false;
        }
        let (gate, signal) = &*release_api.gate;
        locked(gate).release = true;
        signal.notify_all();
        (stop_requested, retained)
    });
    helper.stop();
    assert!(was_pseudo);
    assert_eq!(releaser.join().unwrap(), (true, true));
    assert!(helper.monitor.is_none());
    assert!(!helper.is_pseudo_minimized());
    assert_eq!(locked(&api.script).alpha, 73);
    assert!(locked(&api.script).iconic);
    assert_eq!(helper.evidence(), Evidence::default());
    let calls = locked(&api.script).events.len();
    helper.stop();
    assert_eq!(locked(&api.script).events.len(), calls);
}

#[test]
#[cfg(feature = "prototype-windows")]
#[ignore = "explicit compiled original PseudoMinimizeHelper state/call parity"]
fn preserves_original_pseudo_minimize_scripted_native_calls()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = original::Adapter::load()?;
    for scenario in 0..22 {
        let source = adapter.pseudo_script(scenario)?;
        let (actual, snapshots, evidence) = run_script(scenario);
        let expected: Vec<_> = source
            .events()
            .iter()
            .map(|event| Event {
                operation: event.operation,
                error: event.error,
                argument: event.argument,
                result: event.result,
            })
            .collect();
        assert_eq!(actual, expected, "scenario={scenario}");
        assert_eq!(snapshots, source.snapshots, "scenario={scenario}");
        let errors: Vec<_> = actual.iter().filter(|event| event.error != 0).collect();
        assert_eq!(evidence.failures, u64::try_from(errors.len())?);
        assert_eq!(evidence.minimize_returned_zero, scenario == 11);
        assert!(!evidence.worker_panicked);
        println!(
            "PseudoMinimize script parity: scenario={scenario}, calls={}, native_failures={}",
            actual.len(),
            errors.len()
        );
    }
    Ok(())
}
