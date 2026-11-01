// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Isolated-process cleanup-error control; no capture, HWND or input operation.
use super::*;
use std::cell::Cell;

struct ReportError<'a> {
    ops: &'a Ops,
    code: i32,
    consumed: Cell<u32>,
}
// SAFETY: All original ABI operations delegate to the checked real bridge.
// Destruction first consumes the actual source object synchronously, then
// reports an error. Thus no unit, callback or borrowed host survives, even
// though the loader must conservatively treat consumption as unproved.
unsafe impl UnitOps for ReportError<'_> {
    type Image = Image;
    unsafe fn create(
        &self,
        function: usize,
        hwnd: usize,
        method: u64,
        mouse: u64,
        keyboard: u64,
    ) -> Result<Option<NonNull<c_void>>, i32> {
        // SAFETY: Forward the caller's exact live function/scalars to Ops.
        unsafe { self.ops.create(function, hwnd, method, mouse, keyboard) }
    }
    unsafe fn destroy(&self, function: usize, unit: NonNull<c_void>) -> i32 {
        // SAFETY: Matching live source object and factory, consumed once.
        let result = unsafe { self.ops.destroy(function, unit) };
        if result != 0 {
            return result;
        }
        self.consumed.set(self.consumed.get() + 1);
        self.code
    }
    unsafe fn connect(&self, unit: NonNull<c_void>) -> Result<bool, i32> {
        // SAFETY: Same live unit and bridge contract as the caller.
        unsafe { self.ops.connect(unit) }
    }
    unsafe fn capture(&self, unit: NonNull<c_void>, output: &mut Image) -> Result<bool, i32> {
        // SAFETY: Same live unit and exclusive output as the caller.
        unsafe { self.ops.capture(unit, output) }
    }
    unsafe fn inactive(&self, unit: NonNull<c_void>) -> Result<bool, i32> {
        // SAFETY: Same live unit and bridge contract as the caller.
        unsafe { self.ops.inactive(unit) }
    }
}

#[test]
#[ignore = "requires explicit real delegate DLLs; run alone because cleanup errors retain module references until process exit"]
fn retains_module_and_first_error_after_consumed_unit_reports_cleanup_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let build = std::fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD").ok_or("explicit native build required")?,
    ))?;
    let records = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_DESTROY_RECORDS").ok_or("new cleanup records required")?,
    );
    assert!(records.is_absolute());
    std::fs::create_dir(&records)?;
    let records = std::fs::canonicalize(records)?;
    let provider = records.join("MaaWin32ControlUnit.dll");
    std::fs::copy(build.join("mfs_delegate_source.dll"), &provider)?;
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))?;
    ops.reset();
    assert_eq!(DESTROY_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    assert!(!library::loaded(&provider));
    let thread = crate::platform::windows::manager::probe::thread();
    let mut rows = String::new();
    for (case, code, expected_error, expected_loaded) in [
        (0, 0, 0, false),
        (1, 7101, 7101, true),
        (2, 7102, 7101, true),
    ] {
        let wrapped = ReportError {
            ops: &ops,
            code,
            consumed: Cell::new(0),
        };
        {
            let mut boundary = Native::new(&records, &wrapped).unwrap();
            let mut loader = boundary.new_loader().unwrap();
            assert!(boundary.load(&mut loader).unwrap());
            // Original constructor accepts these selectors without any connect,
            // HWND query, native backend construction or capture operation.
            assert!(boundary.create(&mut loader, 0, 0, 0, 0).unwrap().is_some());
            assert!(library::loaded(&provider));
        }
        assert_eq!(wrapped.consumed.get(), 1);
        let error = DESTROY_ERROR.load(Ordering::Acquire);
        let loaded = library::loaded(&provider);
        assert_eq!(error, expected_error);
        assert_eq!(loaded, expected_loaded);
        use std::fmt::Write;
        writeln!(rows, "{case}\t{code}\t1\t{error}\t{}", u8::from(loaded))?;
    }
    let (events, owners) = ops.events();
    assert_eq!(owners, 0);
    assert_eq!(events.len(), 6);
    let mut table = String::new();
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.thread, thread);
        assert_eq!(event.kind, if index % 2 == 0 { 1 } else { 5 });
        assert_eq!((event.a, event.b, event.c, event.d), (0, 0, 0, 0));
        use std::fmt::Write;
        writeln!(table, "{}\t{}\t0\t0\t0\t0", event.kind, event.thread)?;
    }
    // End every borrowed bridge/table owner. Only deliberate module references
    // remain; object consumption above is independently witnessed by C++ events.
    drop(ops);
    assert!(library::loaded(&provider));
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    std::fs::write(records.join("cases.tsv"), rows)?;
    std::fs::write(records.join("events.tsv"), table)?;
    std::fs::write(
        records.join("scope.txt"),
        "consumed_objects=3\nreported_cleanup_errors=2\nfirst_error=7101\nprovider_retained_after_bridge_owner=1\nactual_captures=0\nunknown_consumption_tested=0\n",
    )?;
    println!(
        "real delegate cleanup cases=3 consumed_units=3 reported_errors=2 first_error=7101 module_retained_after_bridge_owner=1 actual_captures=0 unknown_consumption_tested=0 passed"
    );
    Ok(())
}
