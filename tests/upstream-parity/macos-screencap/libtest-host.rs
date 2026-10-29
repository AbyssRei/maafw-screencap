// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit test process host; no capture code or SDK initialization policy.
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSRunLoop};
use std::process::ExitCode;

pub(crate) fn run(tests: &[&test::TestDescAndFn]) {
    if std::env::var("MFS_P1_MACOS_MAIN_THREAD_HOST").as_deref() != Ok("1") {
        finish(test::test_main_env_args(tests));
        return;
    }
    // Compiler-generated owning-module #[test] functions are static. Copy only
    // their descriptors/function pointers; the normal libtest runner still
    // selects/runs/reports them. Reject an unexpected dynamic/benchmark entry.
    let mut owned = Vec::new();
    owned.try_reserve_exact(tests.len()).unwrap();
    for item in tests {
        let test::TestFn::StaticTestFn(function) = item.testfn else {
            panic!("native macOS host requires static owning-module tests");
        };
        owned.push(test::TestDescAndFn {
            desc: item.desc.clone(),
            testfn: test::TestFn::StaticTestFn(function),
        });
    }
    let arguments: Vec<_> = std::env::args().collect();
    let result = objc2::rc::autoreleasepool(|_| {
        let marker = MainThreadMarker::new().expect("native host process main thread");
        let app = NSApplication::sharedApplication(marker);
        assert!(app.setActivationPolicy(NSApplicationActivationPolicy::Accessory));
        app.finishLaunching();
        let worker = std::thread::spawn(move || {
            let references: Vec<_> = owned.iter().collect();
            test::test_main(&arguments, &references)
        });
        while !worker.is_finished() {
            // SAFETY: Native process main thread; framework's live default run
            // loop/mode; bounded fixture iteration, without capture state locks.
            unsafe {
                let mode = NSDefaultRunLoopMode;
                NSRunLoop::currentRunLoop()
                    .runMode_beforeDate(mode, &NSDate::dateWithTimeIntervalSinceNow(0.01));
            }
        }
        let result = worker.join().expect("native libtest worker joined");
        drop(app);
        eprintln!(
            "native Rust macOS host main_thread=1 appkit_initialized=1 pumped=1 harness_worker_joined=1"
        );
        result
    });
    finish(result);
}
fn finish(result: ExitCode) {
    // Standard libtest returns SUCCESS or its ERROR_EXIT_CODE (101); permission
    // preflight's explicit process exit 77 remains independently classified.
    if result != ExitCode::SUCCESS {
        std::process::exit(test::ERROR_EXIT_CODE.into());
    }
}
