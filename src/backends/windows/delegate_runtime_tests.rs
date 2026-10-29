// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// One explicitly selected mask per fresh test process; no failed-case retry.
use super::*;
use crate::platform::windows::window_api::{Point, SystemWindow, WindowApi};
use crate::test_support::windows_capture_host::{HostDispatcher, HostInitialization, host as com};
use std::fmt::Write;

fn geometry(window: *mut c_void, selected: u64) -> Result<[i32; 4], Box<dyn std::error::Error>> {
    let output = if selected == 4 {
        let monitor = SystemWindow
            .monitor_rects(SystemWindow.monitor(window))
            .map_err(|error| format!("monitor geometry {error}"))?
            .monitor;
        let origin = SystemWindow
            .origin(window, Point::default())
            .map_err(|error| format!("client origin {error}"))?;
        let difference = |a: i32, b: i32| a.checked_sub(b).ok_or("runtime geometry overflow");
        [
            difference(monitor.right, monitor.left)?,
            difference(monitor.bottom, monitor.top)?,
            difference(origin.x, monitor.left)?,
            difference(origin.y, monitor.top)?,
        ]
    } else {
        [96, 64, 0, 0]
    };
    if output[0] <= 0
        || output[1] <= 0
        || output[2] < 0
        || output[3] < 0
        || i64::from(output[2]) + 96 > i64::from(output[0])
        || i64::from(output[3]) + 64 > i64::from(output[1])
    {
        return Err("runtime target not contained in unrotated output".into());
    }
    Ok(output)
}
fn pixels(image: &Image, scope: [i32; 4]) -> bool {
    if (image.span.width, image.span.height) != (scope[0], scope[1]) {
        return false;
    }
    let bytes = image.bytes();
    for y in 0..64 {
        for x in 0..96 {
            let expected = if x < 32 {
                [0x1f, 0x53, 0xa7]
            } else if y < 32 {
                [0xb9, 0x4d, 0x27]
            } else {
                [0x3b, 0xe2, 0x81]
            };
            let offset = ((y + usize::try_from(scope[3]).unwrap())
                * usize::try_from(scope[0]).unwrap()
                + x
                + usize::try_from(scope[2]).unwrap())
                * 3;
            if bytes[offset..offset + 3] != expected {
                return false;
            }
        }
    }
    true
}
#[test]
#[ignore = "explicit isolated real source/Rust native factory capture; requires renderer and current desktop"]
fn captures_selected_method_through_original_and_rust_factories()
-> Result<(), Box<dyn std::error::Error>> {
    let method: u64 = std::env::var("MFS_DELEGATE_CAPTURE_METHOD")?.parse()?;
    assert!([1, 2, 4, 8, 16, 32, 63].contains(&method));
    let build = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD").ok_or("native build required")?,
    ))?;
    let renderer = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_GDI_PARITY_LIBRARY").ok_or("renderer module required")?,
    ))?;
    let root = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_RUNTIME_RECORDS")
            .ok_or("new external runtime records required")?,
    );
    assert!(root.is_absolute());
    std::fs::create_dir(&root)?;
    let root = std::fs::canonicalize(root)?;
    let source = root.join("source");
    let rust = root.join("rust");
    std::fs::create_dir(&source)?;
    std::fs::create_dir(&rust)?;
    std::fs::copy(
        build.join("mfs_delegate_source.dll"),
        source.join("MaaWin32ControlUnit.dll"),
    )?;
    std::fs::copy(
        build.join("mfs_delegate_rust.dll"),
        rust.join("MaaWin32ControlUnit.dll"),
    )?;
    let output = std::process::Command::new(build.join("mfs_delegate_runtime_source.exe"))
        .current_dir(&root)
        .args([
            source.as_os_str(),
            renderer.as_os_str(),
            source.as_os_str(),
            std::ffi::OsStr::new(&method.to_string()),
            std::ffi::OsStr::new("20"),
        ])
        .output()?;
    std::fs::write(root.join("source.stdout"), &output.stdout)?;
    std::fs::write(root.join("source.stderr"), &output.stderr)?;
    let source_passed = output.status.success();
    println!("runtime method={method} source_status={:?}", output.status);
    // Continue the independent Rust attempt even if the original failed. Its
    // success cannot erase that source failure or known-color assertion.
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))?;
    ops.reset();
    ops.mode(5);
    let host = host::Guard::new(&ops);
    // SAFETY: This exact test runs on a fresh libtest thread in its own process.
    // Own and balance successful MTA initialization only; never force a STA.
    unsafe { com::RoInitialize(com::RO_INIT_MULTITHREADED) }.ok()?;
    let _initialization = HostInitialization(true);
    let mut dispatcher = HostDispatcher::new()?;
    let restore = crate::platform::windows::manager::probe::current();
    struct Dpi(isize);
    impl Drop for Dpi {
        fn drop(&mut self) {
            assert!(crate::platform::windows::manager::probe::set(self.0));
        }
    }
    let _dpi = Dpi(restore);
    crate::platform::windows::manager::set_per_monitor_v2()
        .map_err(|error| format!("fixture DPI {error}"))?;
    let adapter = Adapter::load()?;
    let owners_before = adapter.image_owners()?;
    let objects_before = adapter.objects();
    let mut fixture = adapter.render_fixture(96, 64)?;
    let ui = fixture.thread_info()?;
    if method & 12 != 0 {
        fixture.start_desktop_updates()?;
    }
    let mut held = Vec::new();
    let mut rows = String::new();
    let mut mismatch = false;
    for cycle in 0..20 {
        println!("runtime method={method} Rust cycle={cycle} initialize");
        let mut capture = CaptureDelegate::new(
            Native::new(&rust, &ops).unwrap(),
            fixture.hwnd() as usize,
            method,
        );
        let initialized = capture
            .initialize()
            .map_err(|error| format!("Rust runtime init {error:?}"))?;
        if initialized != Some(true) {
            return Err(format!(
                "Rust runtime initialization false: {initialized:?}; source_passed={source_passed}"
            )
            .into());
        }
        let selected = host.selected();
        assert!(selected != 0 && (selected & method) == selected);
        let scope = geometry(fixture.hwnd(), selected)?;
        for shot in 0..3 {
            let mut image = ops.image();
            if !capture
                .capture(&mut image)
                .map_err(|error| format!("Rust runtime capture {error:?}"))?
            {
                return Err("Rust runtime capture false".into());
            }
            std::fs::write(rust.join(format!("rust-{cycle}-{shot}.bgr")), image.bytes())?;
            mismatch |= !pixels(&image, scope);
            held.push((image, scope));
        }
        assert_eq!(capture.inactive(), Ok(true));
        drop(capture);
        let unloaded = !library::loaded(&rust.join("MaaWin32ControlUnit.dll"));
        assert!(unloaded);
        assert_eq!(adapter.image_owners()?, owners_before);
        writeln!(
            rows,
            "{cycle}\t{selected}\t{}\t{}\t{}\t{}\t{}",
            u8::from(unloaded),
            scope[0],
            scope[1],
            scope[2],
            scope[3]
        )?;
        std::fs::write(rust.join("rust-cycles.tsv"), &rows)?;
    }
    fixture.close()?;
    drop(fixture);
    let objects_after = adapter.objects();
    for (index, (image, scope)) in held.iter().enumerate() {
        mismatch |= !pixels(image, *scope);
        std::fs::write(rust.join(format!("rust-held-{index}.bgr")), image.bytes())?;
    }
    let (events, owners) = ops.events();
    assert_eq!(owners, 60);
    let mut table = String::new();
    for event in &events {
        writeln!(
            table,
            "{}\t{}\t{}\t{}\t{}\t{}",
            event.kind, event.thread, event.a, event.b, event.c, event.d
        )?;
    }
    std::fs::write(rust.join("rust-events.tsv"), table)?;
    let counts = host.counts();
    assert_eq!(
        (counts[0], counts[1], counts[3], counts[4]),
        (20, 20, 60, 20)
    );
    drop(held);
    assert_eq!(ops.events().1, 0);
    dispatcher.close()?;
    assert_eq!(DESTROY_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    std::fs::write(
        rust.join("rust-summary.txt"),
        format!(
            "method={method}\ncycles=20\nheld=60\nwindow_closed=1\ndispatcher_closed=1\nknown_pixels={}\ncontext_created={}\ncontext_destroyed={}\nconversions={}\nui_thread={}\ncaller_thread={}\ngdi_before={objects_before}\ngdi_after={objects_after}\n",
            u8::from(!mismatch),
            counts[0],
            counts[1],
            counts[2],
            ui[0],
            ui[1]
        ),
    )?;
    if !source_passed || mismatch {
        return Err(format!(
            "runtime method={method} failed: source_passed={source_passed}, Rust_known_pixels={}",
            !mismatch
        )
        .into());
    }
    println!(
        "real native runtime method={method}; source_lifecycles=20 Rust_lifecycles=20; source_retained=60 Rust_retained=60; actual factory/virtual capture and window/module shutdown=1 passed"
    );
    Ok(())
}
