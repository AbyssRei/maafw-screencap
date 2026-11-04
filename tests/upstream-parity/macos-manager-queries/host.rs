// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Exact original manager bodies versus the same Rust manager, controlled hosts.
use super::*;
#[cfg(unix)]
use std::ffi::{CStr, c_void};
use std::{
    cell::RefCell,
    fs,
    io::{BufWriter, Write as _},
    mem::{align_of, offset_of, size_of},
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Script {
    method: u64,
    input_method: u64,
    window_id: u32,
    available: [u32; 2],
    fail_capture: [u32; 2],
    fail_input: [u32; 2],
    features: [u64; 2],
    outcomes: [u32; 3],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
struct Event {
    kind: u32,
    unit: u32,
    phase: u32,
    call: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
struct Report {
    connect: [u32; 2],
    connected: [u32; 3],
    capture_owner: [u32; 2],
    input_owner: [u32; 2],
    features: [u64; 4],
    captures: [u32; 4],
    pixels: [u32; 4],
    capture_creates: u32,
    input_creates: u32,
    calls: u32,
    queries: u32,
    event_count: u32,
    overflow: u32,
    events: [Event; 32],
    info_length: u32,
    info: [u8; 256],
}
impl Default for Report {
    fn default() -> Self {
        // SAFETY: Every field is an integer or an array/struct of integers;
        // all-zero is valid. No padding is read or serialized as raw bytes.
        unsafe { std::mem::zeroed() }
    }
}
impl Script {
    fn numbers(&self) -> Vec<u64> {
        let mut v = vec![self.method, self.input_method, u64::from(self.window_id)];
        for a in [&self.available, &self.fail_capture, &self.fail_input] {
            v.extend(a.iter().copied().map(u64::from));
        }
        v.extend(self.features);
        v.extend(self.outcomes.map(u64::from));
        assert_eq!(v.len(), 14);
        v
    }
}
impl Report {
    fn numbers(&self) -> Vec<u64> {
        let mut v = Vec::new();
        for a in [
            self.connect.as_slice(),
            self.connected.as_slice(),
            self.capture_owner.as_slice(),
            self.input_owner.as_slice(),
        ] {
            v.extend(a.iter().copied().map(u64::from));
        }
        v.extend(self.features);
        for a in [self.captures, self.pixels] {
            v.extend(a.map(u64::from));
        }
        v.extend(
            [
                self.capture_creates,
                self.input_creates,
                self.calls,
                self.queries,
                self.event_count,
                self.overflow,
            ]
            .map(u64::from),
        );
        for e in self.events {
            v.extend([e.kind, e.unit, e.phase, e.call].map(u64::from));
        }
        assert_eq!(v.len(), 155);
        v
    }
    fn info(&self) -> &str {
        let n = usize::try_from(self.info_length).unwrap();
        assert!(n <= self.info.len() && self.info[n..].iter().all(|&b| b == 0));
        std::str::from_utf8(&self.info[..n]).unwrap()
    }
}
struct State {
    script: Script,
    phase: usize,
    report: Report,
}
fn event(s: &mut State, kind: u32, unit: u32, call: u32) {
    let index = usize::try_from(s.report.event_count).unwrap();
    assert!(index < s.report.events.len());
    s.report.events[index] = Event {
        kind,
        unit,
        phase: s.phase.try_into().unwrap(),
        call,
    };
    s.report.event_count += 1;
}
struct Unit {
    id: u32,
    state: Rc<RefCell<State>>,
}
impl Drop for Unit {
    fn drop(&mut self) {
        event(&mut self.state.borrow_mut(), 3, self.id, 0);
    }
}
struct Input {
    id: u32,
    state: Rc<RefCell<State>>,
}
impl InputFeatures for Input {
    fn features(&self) -> u64 {
        let mut s = self.state.borrow_mut();
        let call = s.report.queries;
        s.report.queries += 1;
        event(&mut s, 5, self.id, call);
        s.script.features[s.phase]
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        event(&mut self.state.borrow_mut(), 6, self.id, 0);
    }
}
fn create_capture(state: &Rc<RefCell<State>>, window: u32) -> Result<Unit, ()> {
    let id = {
        let mut s = state.borrow_mut();
        assert_eq!(window, s.script.window_id);
        s.report.capture_creates += 1;
        let id = s.report.capture_creates;
        event(&mut s, 1, id, 0);
        if s.script.fail_capture[s.phase] != 0 {
            return Err(());
        }
        id
    };
    Ok(Unit {
        id,
        state: state.clone(),
    })
}
fn create_input(state: &Rc<RefCell<State>>, window: u32, method: u64) -> Result<Input, ()> {
    let id = {
        let mut s = state.borrow_mut();
        assert_eq!(
            (window, method),
            (s.script.window_id, s.script.input_method)
        );
        s.report.input_creates += 1;
        let id = s.report.input_creates;
        event(&mut s, 4, id, 0);
        if s.script.fail_input[s.phase] != 0 {
            return Err(());
        }
        id
    };
    Ok(Input {
        id,
        state: state.clone(),
    })
}
fn capture(
    manager: &Manager<Unit, Input>,
    state: &Rc<RefCell<State>>,
    output: &mut Option<u8>,
    slot: usize,
) {
    let result = manager.capture_with(output, |unit| {
        let mut s = state.borrow_mut();
        let call = s.report.calls;
        s.report.calls += 1;
        event(&mut s, 2, unit.id, call);
        match s.script.outcomes[usize::try_from(call).unwrap()] {
            0 => Ok(None),
            1 => Ok(Some(Some(u8::try_from(20 + call).unwrap()))),
            2 => Ok(Some(None)),
            3 => Err(()),
            _ => unreachable!(),
        }
    });
    let mut s = state.borrow_mut();
    s.report.captures[slot] = result.map_or(2, u32::from);
    s.report.pixels[slot] = u32::from(output.unwrap_or(0));
}
fn model(script: Script) -> Report {
    let state = Rc::new(RefCell::new(State {
        script,
        phase: 0,
        report: Report::default(),
    }));
    {
        let mut manager = Manager::<Unit, Input>::with_host_input(
            script.window_id,
            script.method,
            script.input_method,
        );
        let mut output = Some(99);
        state.borrow_mut().report.connected[0] = u32::from(manager.connected());
        let cold = manager.get_features();
        state.borrow_mut().report.features[0] = cold;
        capture(&manager, &state, &mut output, 0);
        for phase in 0..2 {
            state.borrow_mut().phase = phase;
            let result = manager.connect_with_host_input(
                script.available[phase] != 0,
                |window| create_capture(&state, window),
                |window, method| create_input(&state, window, method),
            );
            {
                let mut s = state.borrow_mut();
                s.report.connect[phase] = result.map_or(2, u32::from);
                s.report.connected[phase + 1] = u32::from(manager.connected());
                s.report.capture_owner[phase] = manager.selected.as_ref().map_or(0, |unit| unit.id);
                s.report.input_owner[phase] = manager.input.as_ref().map_or(0, |input| input.id);
            }
            let features = manager.get_features();
            state.borrow_mut().report.features[if phase == 0 { 1 } else { 3 }] = features;
            capture(
                &manager,
                &state,
                &mut output,
                if phase == 0 { 1 } else { 3 },
            );
            if phase == 0 {
                assert!(manager.inactive());
                let features = manager.get_features();
                state.borrow_mut().report.features[2] = features;
                capture(&manager, &state, &mut output, 2);
            }
        }
        let info = manager.get_info().unwrap();
        let mut s = state.borrow_mut();
        s.report.info_length = info.len().try_into().unwrap();
        s.report.info[..info.len()].copy_from_slice(info.as_bytes());
    }
    assert_eq!(Rc::strong_count(&state), 1, "both manager owners must end");
    Rc::try_unwrap(state).ok().unwrap().into_inner().report
}

#[cfg(target_os = "windows")]
use crate::platform::windows::test_library::Module;
#[cfg(unix)]
struct Module(*mut c_void);
#[cfg(unix)]
impl Module {
    fn open(path: &Path) -> Self {
        use std::os::unix::ffi::OsStrExt;
        let p = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: Explicit absolute prepared library, local symbol scope.
        let value = unsafe { libc::dlopen(p.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        assert!(!value.is_null());
        Self(value)
    }
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        // SAFETY: Retained live image and terminated static export name. Caller
        // supplies the exact host.h signature and checks native layout first.
        let p = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        assert!(!p.is_null());
        assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        unsafe { std::mem::transmute_copy(&p) }
    }
}
#[cfg(unix)]
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Only synchronous model calls; no native/callback owner escapes.
        assert_eq!(unsafe { libc::dlclose(self.0) }, 0);
    }
}

#[test]
#[ignore = "explicit exact-body native model; controlled hosts, zero native input/capture"]
fn preserves_host_input_construction_features_and_partial_manager_owners() {
    let library = fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_MAC_MANAGER_QUERY_LIBRARY").unwrap(),
    ))
    .unwrap();
    let root = PathBuf::from(std::env::var_os("MFS_MAC_MANAGER_HOST_RECORDS").unwrap());
    assert!(root.is_absolute());
    let product = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    assert!(
        !root
            .ancestors()
            .find(|p| p.exists())
            .unwrap()
            .canonicalize()
            .unwrap()
            .starts_with(product)
    );
    fs::create_dir(&root).unwrap();
    fs::copy(&library, root.join(library.file_name().unwrap())).unwrap();
    let module = Module::open(&library);
    type Run = unsafe extern "C" fn(*const Script, *mut Report) -> i32;
    // SAFETY: Exact host.h declarations; layout values are checked before any
    // struct consumption. Module outlives all synchronous source calls.
    let (run, layout) = unsafe {
        (
            module.symbol::<Run>(c"mfs_mh_run"),
            module.symbol::<unsafe extern "C" fn(u32) -> usize>(c"mfs_mh_layout"),
        )
    };
    let expected = [
        1,
        size_of::<Script>(),
        align_of::<Script>(),
        offset_of!(Script, method),
        offset_of!(Script, input_method),
        offset_of!(Script, window_id),
        offset_of!(Script, available),
        offset_of!(Script, fail_capture),
        offset_of!(Script, fail_input),
        offset_of!(Script, features),
        offset_of!(Script, outcomes),
        size_of::<Report>(),
        align_of::<Report>(),
        offset_of!(Report, connect),
        offset_of!(Report, connected),
        offset_of!(Report, capture_owner),
        offset_of!(Report, input_owner),
        offset_of!(Report, features),
        offset_of!(Report, captures),
        offset_of!(Report, pixels),
        offset_of!(Report, capture_creates),
        offset_of!(Report, input_creates),
        offset_of!(Report, calls),
        offset_of!(Report, queries),
        offset_of!(Report, event_count),
        offset_of!(Report, overflow),
        offset_of!(Report, events),
        offset_of!(Report, info_length),
        offset_of!(Report, info),
        size_of::<Event>(),
        align_of::<Event>(),
        offset_of!(Event, kind),
        offset_of!(Event, unit),
        offset_of!(Event, phase),
        offset_of!(Event, call),
        0,
        1,
        2,
        1,
        2,
    ];
    for (i, value) in expected.into_iter().enumerate() {
        assert_eq!(
            unsafe { layout(i.try_into().unwrap()) },
            value,
            "layout {i}"
        );
    }
    assert_eq!(
        unsafe { layout(expected.len().try_into().unwrap()) },
        usize::MAX
    );
    let mut source = BufWriter::new(fs::File::create_new(root.join("source.tsv")).unwrap());
    let mut rust = BufWriter::new(fs::File::create_new(root.join("rust.tsv")).unwrap());
    let (mut cases, mut calls, mut queries) = (0_u32, 0_u64, 0_u64);
    for method in [NONE, SCREEN_CAPTURE_KIT, 2, u64::MAX] {
        for input_method in [NONE, GLOBAL_EVENT, POST_TO_PID, 3, u64::MAX] {
            for available in 0..4 {
                for failure in 0..16 {
                    for features in [[0, u64::MAX], [1, 2], [4, 3]] {
                        for outcomes in [[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 1, 2], [1, 0, 3]] {
                            let script = Script {
                                method,
                                input_method,
                                window_id: if cases.is_multiple_of(2) { 0 } else { u32::MAX },
                                available: [available & 1, available >> 1],
                                fail_capture: [failure & 1, (failure >> 1) & 1],
                                fail_input: [(failure >> 2) & 1, (failure >> 3) & 1],
                                features,
                                outcomes,
                            };
                            let mut original = Report::default();
                            assert_eq!(
                                unsafe { run(&script, &mut original) },
                                0,
                                "native case {cases}"
                            );
                            let actual = model(script);
                            assert_eq!(original, actual, "case {cases}: {script:?}");
                            assert_eq!(original.overflow, 0);
                            let signed = |v: u64| i64::from_ne_bytes(v.to_ne_bytes());
                            assert_eq!(
                                original.info(),
                                format!(
                                    "{{\"input_method\":{},\"screencap_method\":{},\"type\":\"macos\",\"window_id\":{}}}",
                                    signed(input_method),
                                    signed(method),
                                    script.window_id
                                )
                            );
                            writeln!(
                                source,
                                "{cases}\t{:?}\t{:?}\t{}",
                                script.numbers(),
                                original.numbers(),
                                original.info()
                            )
                            .unwrap();
                            writeln!(
                                rust,
                                "{cases}\t{:?}\t{:?}\t{}",
                                script.numbers(),
                                actual.numbers(),
                                actual.info()
                            )
                            .unwrap();
                            calls += u64::from(original.calls);
                            queries += u64::from(original.queries);
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 19200);
    source.flush().unwrap();
    rust.flush().unwrap();
    drop(source);
    drop(rust);
    let mut report = Report::default();
    assert_eq!(unsafe { run(std::ptr::null(), &mut report) }, 1);
    assert_eq!(report, Report::default());
    assert_eq!(unsafe { run(std::ptr::null(), std::ptr::null_mut()) }, 1);
    drop(module);
    let retained = fs::read(root.join("source.tsv")).unwrap();
    let copy = fs::read(root.join("rust.tsv")).unwrap();
    std::thread::spawn(move || assert_eq!(retained, copy))
        .join()
        .unwrap();
    fs::write(root.join("summary.txt"),format!("schema=1\ncases={cases}\ncontrolled_source_capture_calls={calls}\nsource_feature_queries={queries}\npaired_reports={}\ninput_actions=0\nnative_screenshots=0\njoined_reader=1\n",cases*2)).unwrap();
    eprintln!(
        "macOS manager host cases={cases} paired_reports={} exact_events=1 input_actions=0 native_screenshots=0 passed",
        cases * 2
    );
}
