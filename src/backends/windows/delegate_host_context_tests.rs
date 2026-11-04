// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Same executable/bridge, fresh process per side and host context. No pump or
// added COM/DPI repair inside original/Rust capture operations.
use super::*;
use crate::test_support::windows_capture_host::{HostDispatcher, HostInitialization, host as com};
use std::{fs, io::Write};

const CASE: &str = "backends::windows::delegate_native::tests::host_context::compares_real_factory_calls_and_caller_com_dpi_effects";
const WHOLE_CASE: &str = "backends::windows::delegate_native::tests::host_context::compares_whole_original_win32_class_and_rust_capture_under_host_contexts";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[path = "delegate_whole_context_tests.rs"]
mod whole;

fn snapshot() -> [i64; 5] {
    let mut apartment = com::APTTYPE::default();
    let mut qualifier = com::APTTYPEQUALIFIER::default();
    // SAFETY: Read-only caller observations with initialized writable outputs.
    let code = unsafe { com::CoGetApartmentType(&mut apartment, &mut qualifier) };
    let context = unsafe { com::GetThreadDpiAwarenessContext() };
    let dpi = (1..=5)
        .find(|&kind| {
            // SAFETY: Documented native pseudo-handles; comparison does not
            // dereference them or change this caller's DPI awareness.
            unsafe {
                com::AreDpiAwarenessContextsEqual(
                    context,
                    com::DPI_AWARENESS_CONTEXT(std::ptr::without_provenance_mut::<c_void>(
                        (-kind as isize) as usize,
                    )),
                )
            }
            .as_bool()
        })
        .expect("unrecognized native DPI context");
    [
        i64::from(code.0),
        if code.is_ok() {
            i64::from(apartment)
        } else {
            -1
        },
        if code.is_ok() {
            i64::from(qualifier)
        } else {
            -1
        },
        dpi,
        i64::from(crate::platform::windows::manager::probe::thread()),
    ]
}

fn record(file: &mut fs::File, stage: u32) -> Result<()> {
    writeln!(
        file,
        "{stage}\t{}",
        snapshot().map(|v| v.to_string()).join("\t")
    )?;
    file.flush()?;
    Ok(())
}

fn outcome(value: std::result::Result<bool, Error>) -> Result<[i32; 2]> {
    match value {
        Ok(value) => Ok([0, i32::from(value)]),
        Err(Error::Bridge(code)) => Ok([code, 0]),
        Err(error) => Err(format!("host-context boundary {error:?}").into()),
    }
}

fn child(root: &Path, side: &str, mode: &str, dpi: i64, method: u64, complete: bool) -> Result<()> {
    assert!(matches!(side, "source" | "rust"));
    let directory = root.join(side);
    fs::create_dir(&directory)?;
    let mut states = fs::File::create_new(directory.join("states.tsv"))?;
    let mut calls = fs::File::create_new(directory.join("calls.tsv"))?;
    record(&mut states, 0)?;
    assert_eq!(snapshot()[0], i64::from(com::CO_E_NOTINITIALIZED.0));
    let restore = crate::platform::windows::manager::probe::current();
    struct Restore(isize);
    impl Drop for Restore {
        fn drop(&mut self) {
            assert!(crate::platform::windows::manager::probe::set(self.0));
        }
    }
    let restore = Restore(restore);
    assert!(crate::platform::windows::manager::probe::set(
        isize::try_from(-dpi)?
    ));
    let mut initialized = HostInitialization(false);
    if mode != "uninitialized" {
        let kind = if mode == "sta" {
            com::RO_INIT_SINGLETHREADED
        } else {
            com::RO_INIT_MULTITHREADED
        };
        // SAFETY: Fresh caller thread. Balance only this successful explicit
        // reference; never normalize an apartment changed by native capture.
        unsafe { com::RoInitialize(kind) }.ok()?;
        initialized.0 = true;
    }
    let mut dispatcher = if mode == "mta-dispatcher" {
        Some(HostDispatcher::new()?)
    } else {
        None
    };
    record(&mut states, 1)?;
    let build = fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD").ok_or("native build required")?,
    ))?;
    let provider = directory.join("MaaWin32ControlUnit.dll");
    let binary = if complete && side == "source" {
        "mfs_delegate_whole_source.dll".to_owned()
    } else {
        format!("mfs_delegate_{side}.dll")
    };
    fs::copy(build.join(binary), &provider)?;
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))?;
    ops.reset();
    ops.mode(5);
    let host = (side == "rust").then(|| host::Guard::new(&ops));
    let adapter = Adapter::load()?;
    let owners_before = adapter.image_owners()?;
    let mut fixture = adapter.render_fixture(96, 64)?;
    let ui = fixture.thread_info()?;
    assert!(ui[0] != 0 && ui[0] == ui[2] && ui[0] != ui[1]);
    // UI-owner geometry commands do not change the caller's DPI context. The
    // entire target remains 96x64, with six native nonclient pixels outside it.
    let before = fixture.geometry([0; 6])?;
    let x = before[4].checked_add(64).ok_or("fixture x overflow")?;
    let y = before[5].checked_add(64).ok_or("fixture y overflow")?;
    fixture.geometry([4, 0x9400_0000, x, y, 96, 64])?;
    let setup = fixture.geometry([2, 0, 0, 0, 0, 0])?;
    assert_eq!(
        &setup[8..18],
        &[x - 6, y - 6, x + 102, y + 70, 0, 0, 96, 64, x, y]
    );
    fs::write(
        directory.join("setup.tsv"),
        format!("{}\n", setup.map(|v| v.to_string()).join("\t")),
    )?;
    if method & 12 != 0 {
        fixture.start_desktop_updates()?;
    }
    record(&mut states, 2)?;
    let mut boundary = Native::new(&directory, &ops).unwrap();
    let mut loader = boundary.new_loader().unwrap();
    assert!(boundary.load(&mut loader).unwrap());
    record(&mut states, 3)?;
    let unit = boundary
        .create(&mut loader, fixture.hwnd() as usize, method, 0, 0)
        .unwrap()
        .unwrap();
    record(&mut states, 4)?;
    let mut inspector = if complete && side == "source" {
        Some(whole::Inspector::new(&provider, &directory, unit, method)?)
    } else {
        None
    };
    let view = boundary.view(&loader, unit).unwrap();
    let connected = outcome(boundary.connect(&mut loader, &view))?;
    record(&mut states, 5)?;
    writeln!(calls, "0\t{}\t{}", connected[0], connected[1])?;
    calls.flush()?;
    let selected = if complete && side == "source" {
        // Complete original class has no winner-observation hook. A single
        // known method bit binds its configuration; it is explicitly labelled
        // as such, never presented as an observed mixed-method winner.
        if method == 64 { 0 } else { method }
    } else if let Some(host) = &host {
        host.selected()
    } else {
        ops.events()
            .0
            .iter()
            .rev()
            .find(|e| e.kind == 7)
            .map_or(0, |e| e.a)
    };
    let mut image = ops.image();
    let captured = outcome(boundary.capture(&mut loader, &view, &mut image))?;
    record(&mut states, 6)?;
    writeln!(calls, "1\t{}\t{}", captured[0], captured[1])?;
    calls.flush()?;
    let scope = runtime::geometry(fixture.hwnd(), selected)?;
    let known = !image.span.owner.is_null() && runtime::pixels(&image, scope);
    fs::write(directory.join("current.bgr"), image.bytes())?;
    fs::write(
        directory.join("image.tsv"),
        format!(
            "{selected}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            image.span.width,
            image.span.height,
            image.span.stride,
            image.span.length,
            scope[0],
            scope[1],
            scope[2],
            scope[3],
            u8::from(known)
        ),
    )?;
    let inactive = outcome(boundary.inactive(&mut loader, &view))?;
    record(&mut states, 7)?;
    writeln!(calls, "2\t{}\t{}", inactive[0], inactive[1])?;
    calls.flush()?;
    if let Some(mut inspector) = inspector.take() {
        inspector.finish(unit)?;
    }
    drop(view);
    drop(loader);
    record(&mut states, 8)?;
    assert!(!library::loaded(&provider));
    assert_eq!(DESTROY_ERROR.load(std::sync::atomic::Ordering::Acquire), 0);
    fixture.close()?;
    record(&mut states, 9)?;
    fs::write(directory.join("held.bgr"), image.bytes())?;
    drop(image);
    assert_eq!(ops.events().1, 0);
    drop(host);
    assert_eq!(adapter.image_owners()?, owners_before);
    drop(adapter);
    drop(ops);
    record(&mut states, 10)?;
    if let Some(dispatcher) = &mut dispatcher {
        dispatcher.close()?;
    }
    drop(dispatcher);
    record(&mut states, 11)?;
    drop(initialized);
    record(&mut states, 12)?;
    drop(restore);
    record(&mut states, 13)?;
    // Original false/exception outcomes are observations, not successful
    // capture claims. The independent reader checks pairing and known pixels.
    fs::write(
        directory.join("completed.txt"),
        "factory_destroyed=1\nmodule_unloaded=1\nfixture_joined=1\nimages_released=1\nhost_unregistered=1\ndispatcher_closed=1\nexplicit_com_balanced=1\n",
    )?;
    Ok(())
}

#[test]
#[ignore = "explicit fresh-process original/Rust Win32 factory COM/DPI/capture comparison"]
fn compares_real_factory_calls_and_caller_com_dpi_effects() -> Result<()> {
    run(false)
}

#[test]
#[ignore = "explicit complete original Win32 class/API with Input=None versus Rust capture, fresh host processes"]
fn compares_whole_original_win32_class_and_rust_capture_under_host_contexts() -> Result<()> {
    run(true)
}

fn run(complete: bool) -> Result<()> {
    let root = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_CONTEXT_RECORDS")
            .ok_or("external context records required")?,
    );
    let mode = std::env::var("MFS_DELEGATE_CONTEXT_MODE")?;
    let dpi: i64 = std::env::var("MFS_DELEGATE_CONTEXT_DPI")?.parse()?;
    let method: u64 = std::env::var("MFS_DELEGATE_CAPTURE_METHOD")?.parse()?;
    assert!(root.is_absolute());
    assert!(matches!(
        mode.as_str(),
        "uninitialized" | "mta" | "mta-dispatcher" | "sta"
    ));
    assert!([1, 2, 4].contains(&dpi));
    assert!([0, 1, 2, 4, 8, 16, 32, 63, 64].contains(&method));
    assert!(!complete || method != 63);
    if let Ok(side) = std::env::var("MFS_DELEGATE_CONTEXT_SIDE") {
        child(&root, &side, &mode, dpi, method, complete)?;
        println!("Win32 host child side={side} mode={mode} dpi={dpi} method={method} completed");
        return Ok(());
    }
    fs::create_dir(&root)?;
    let mut passed = true;
    for side in ["source", "rust"] {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                if complete { WHOLE_CASE } else { CASE },
                "--exact",
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("MFS_DELEGATE_CONTEXT_SIDE", side)
            .output()?;
        fs::write(root.join(format!("{side}.stdout")), &output.stdout)?;
        fs::write(root.join(format!("{side}.stderr")), &output.stderr)?;
        let text = std::str::from_utf8(&output.stdout)?;
        let valid = output.status.success()
            && text.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")
            && text.contains(&format!(
                "Win32 host child side={side} mode={mode} dpi={dpi} method={method} completed"
            ));
        fs::write(
            root.join(format!("{side}.exit")),
            format!(
                "{}\n",
                output.status.code().ok_or("child exit code missing")?
            ),
        )?;
        passed &= valid;
        println!(
            "Win32 host side={side} mode={mode} dpi={dpi} method={method} process_passed={valid}"
        );
    }
    if !passed {
        return Err("host-context child failed; both original results retained".into());
    }
    if complete {
        println!(
            "Win32 whole host processes mode={mode} dpi={dpi} method={method} sides=2 original_class_api=1 input_none=1 completed; independent records required"
        );
    } else {
        println!(
            "Win32 host processes mode={mode} dpi={dpi} method={method} sides=2 completed; independent records required"
        );
    }
    Ok(())
}
