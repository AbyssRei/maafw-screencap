// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{cell::RefCell, sync::Arc};

use super::super::delegate_script::*;

fn instance() -> (CaptureDelegate<Script>, Shared) {
    let state = Rc::new(RefCell::new(State::default()));
    (CaptureDelegate::new(Script(state.clone()), 1, 1), state)
}
#[test]
fn failed_reconnect_retains_an_invalid_view_without_calling_its_destroyed_unit() {
    let (mut capture, state) = instance();
    assert_eq!(capture.initialize(), Ok(Some(true)));
    let mut output = Some(Arc::try_new([99; 18]).unwrap());
    state.borrow_mut().outcome = 1;
    assert_eq!(capture.capture(&mut output), Ok(true));
    let held = output.as_ref().unwrap().clone();
    state.borrow_mut().flags = 1;
    assert_eq!(capture.initialize(), Ok(Some(false)));
    assert!(capture.view.is_some() && capture.loader.is_none() && !capture.view_live);
    let before = state.borrow().events.clone();
    assert_eq!(
        capture.capture(&mut output),
        Err(Error::InvalidatedSourceView)
    );
    assert_eq!(capture.inactive(), Err(Error::InvalidatedSourceView));
    assert!(Arc::ptr_eq(output.as_ref().unwrap(), &held));
    assert_eq!(state.borrow().events, before);
    state.borrow_mut().flags = 0;
    assert_eq!(capture.initialize(), Ok(Some(true)));
    assert_eq!(capture.capture(&mut output), Ok(true));
    assert_eq!(&held[..3], [7, 1, 11]);
    assert_eq!(&output.unwrap()[..3], [7, 2, 11]);
    drop(capture);
    assert!(state.borrow().alive.is_empty() && !state.borrow().module);
    assert_eq!(&held[..3], [7, 1, 11]);
}

#[test]
fn loader_construction_failure_preserves_the_old_owner_but_view_failure_keeps_the_new_one() {
    let (mut capture, state) = instance();
    assert_eq!(capture.initialize(), Ok(Some(true)));
    state.borrow_mut().fail_loader = true;
    let before = state.borrow().events.clone();
    assert_eq!(
        capture.initialize(),
        Err(Error::Boundary("loader allocation"))
    );
    assert!(capture.view_live);
    assert_eq!(state.borrow().events, before);
    state.borrow_mut().fail_loader = false;
    state.borrow_mut().fail_view = true;
    assert_eq!(
        capture.initialize(),
        Err(Error::Boundary("view allocation"))
    );
    assert!(capture.loader.is_some() && capture.view.is_some() && !capture.view_live);
    assert_eq!(
        state.borrow().alive.iter().copied().collect::<Vec<_>>(),
        [2]
    );
    drop(capture);
    assert!(state.borrow().alive.is_empty() && !state.borrow().module);
}

#[path = "../../../tests/upstream-parity/windows-delegate/cases.rs"]
mod cases;
use cases::{Case, matrix};
fn pixels(image: &Image) -> String {
    match image {
        Some(pixels) => pixels.iter().map(|b| format!("{b:02x}")).collect(),
        None => "empty".into(),
    }
}
fn status(value: Result<bool, Error<&'static str>>) -> u32 {
    match value {
        Ok(value) => u32::from(value),
        Err(Error::Boundary(_)) => 2,
        Err(Error::InvalidatedSourceView) => 4,
    }
}
fn report(case: Case) -> String {
    use std::fmt::Write;
    let state = Rc::new(RefCell::new(State::default()));
    let mut result = String::new();
    let mut held = Vec::new();
    {
        let mut capture = CaptureDelegate::new(
            Script(state.clone()),
            usize::try_from(case.hwnd).unwrap(),
            case.method,
        );
        let mut output = Some(Arc::try_new([99; 18]).unwrap());
        writeln!(
            result,
            "initial_capture={}",
            status(capture.capture(&mut output))
        )
        .unwrap();
        writeln!(result, "initial_inactive={}", status(capture.inactive())).unwrap();
        for phase in 0..2 {
            {
                let mut state = state.borrow_mut();
                state.flags = case.flags[phase];
                state.outcome = case.outcome;
                state.inactive = case.inactive;
            }
            let initialized = match capture.initialize() {
                Ok(Some(value)) => u32::from(value),
                Ok(None) => 3,
                Err(Error::Boundary(_)) => 2,
                Err(Error::InvalidatedSourceView) => unreachable!(),
            };
            writeln!(result, "init{phase}={initialized}").unwrap();
            writeln!(
                result,
                "loader{phase}={}",
                u8::from(capture.loader.is_some())
            )
            .unwrap();
            writeln!(result, "view{phase}={}", u8::from(capture.view.is_some())).unwrap();
            writeln!(
                result,
                "live{phase}={}",
                u8::from(capture.view.is_some() && capture.view_live)
            )
            .unwrap();
            writeln!(result, "module{phase}={}", u8::from(state.borrow().module)).unwrap();
            let captured = status(capture.capture(&mut output));
            let inactive = status(capture.inactive());
            writeln!(
                result,
                "capture{phase}={captured}\noutput{phase}={}\ninactive{phase}={inactive}",
                pixels(&output)
            )
            .unwrap();
            held.push(output.clone());
        }
    }
    assert!(state.borrow().alive.is_empty() && !state.borrow().module);
    for (index, image) in held.iter().enumerate() {
        writeln!(result, "held{index}={}", pixels(image)).unwrap();
    }
    writeln!(result, "events={}", state.borrow().events.join(",")).unwrap();
    writeln!(result, "owners_closed=1").unwrap();
    result
}

#[test]
#[ignore = "requires the explicit compiled fixed-source delegate model and external records"]
fn matches_original_capture_delegation_owners_failures_and_output_forwarding() {
    use std::{env, fs, path::PathBuf, process::Command};
    let executable = PathBuf::from(env::var_os("MFS_P1_WINDOWS_DELEGATE_MODEL").unwrap());
    let directory = PathBuf::from(env::var_os("MFS_P1_WINDOWS_DELEGATE_ARTIFACTS").unwrap());
    fs::create_dir(&directory).unwrap();
    let cases = matrix();
    assert_eq!(cases.len(), 274);
    let mut rows = String::new();
    for (index, case) in cases.iter().copied().enumerate() {
        let args = case.arguments();
        let original = Command::new(&executable).args(&args).output().unwrap();
        let expected = report(case);
        fs::write(
            directory.join(format!("case-{index}.source.txt")),
            &original.stdout,
        )
        .unwrap();
        fs::write(
            directory.join(format!("case-{index}.source.stderr")),
            &original.stderr,
        )
        .unwrap();
        fs::write(directory.join(format!("case-{index}.rust.txt")), &expected).unwrap();
        assert!(
            original.status.success(),
            "source case {index}: {:?}",
            original.status
        );
        assert_eq!(
            String::from_utf8(original.stdout)
                .unwrap()
                .replace("\r\n", "\n"),
            expected,
            "source/Rust case {index}"
        );
        rows.push_str(&format!("{index}\t{}\n", args.join("\t")));
    }
    fs::write(directory.join("cases.tsv"), rows).unwrap();
    println!(
        "original Rust Win32 delegate scripts=274 phases=548 source_bodies=1 real_mat=1 held_outputs=1 input_none=1 stale_source_calls_skipped=1 rust_stale_views_rejected=1 native_dll_loading=not_tested passed"
    );
}
