// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real original virtual consumers; scoped source/Rust capture-only facades.
use super::*;
use crate::backends::manager_metadata::case_matrix;
use std::{
    fs,
    io::{BufWriter, Write},
};
type Query = unsafe extern "C" fn(*mut c_void, *mut QueryResult) -> i32;
type WindowCreate = unsafe extern "C" fn() -> usize;
type WindowCall = unsafe extern "C" fn(usize) -> i32;

struct Window {
    value: usize,
    destroy: WindowCall,
}
impl Window {
    fn close(&mut self) {
        let value = std::mem::replace(&mut self.value, 0);
        // SAFETY: Message window belongs to this original caller thread; its
        // module/bridge remains live until the test and all windows finish.
        assert_eq!(unsafe { (self.destroy)(value) }, 0);
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        if self.value != 0 {
            // SAFETY: Failure cleanup of the same owned thread-local window.
            let code = unsafe { (self.destroy)(self.value) };
            assert_eq!(code, 0, "query fixture window cleanup failed");
        }
    }
}
#[derive(Clone)]
struct Row {
    side: &'static str,
    kind: &'static str,
    case: usize,
    phase: &'static str,
    hwnd: usize,
    method: u64,
    valid: i32,
    result: QueryResult,
}
impl Row {
    fn line(&self) -> String {
        let value = &self.result;
        assert!(value.uuid_length <= value.uuid.len() && value.info_length <= value.info.len());
        let uuid = std::str::from_utf8(&value.uuid[..value.uuid_length]).unwrap();
        let info = std::str::from_utf8(&value.info[..value.info_length]).unwrap();
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{uuid}\t{info}\n",
            self.side,
            self.kind,
            self.case,
            self.phase,
            self.hwnd,
            self.method,
            self.valid,
            value.connected,
            value.uuid_success,
            value.features
        )
    }
}
struct Recorder {
    query: Query,
    rows: Vec<Row>,
    output: BufWriter<fs::File>,
}
impl Recorder {
    fn record(&mut self, unit: NonNull<c_void>, mut row: Row, connected: bool) {
        // SAFETY: The caller retains its loader/unit/module through this whole
        // synchronous virtual query. All native temporaries finish on return.
        let code = unsafe { (self.query)(unit.as_ptr(), &mut row.result) };
        assert_eq!(code, 0, "real interface query failed");
        self.output.write_all(row.line().as_bytes()).unwrap();
        self.output.flush().unwrap();
        let value = &row.result;
        assert_eq!(value.connected, i32::from(connected));
        assert_eq!(value.uuid_success, 1);
        assert_eq!(value.features, 0);
        assert_eq!(
            &value.uuid[..value.uuid_length],
            format!("{:016X}", row.hwnd).as_bytes()
        );
        let method = i64::from_ne_bytes(row.method.to_ne_bytes());
        let info = format!(
            "{{\"hwnd\":{},\"keyboard_method\":0,\"mouse_method\":0,\"screencap_method\":{method},\"type\":\"win32\"}}",
            row.hwnd
        );
        assert_eq!(&value.info[..value.info_length], info.as_bytes());
        assert!(
            value.uuid[value.uuid_length..]
                .iter()
                .all(|&byte| byte == 0)
        );
        assert!(
            value.info[value.info_length..]
                .iter()
                .all(|&byte| byte == 0)
        );
        self.rows.push(row);
    }
}
#[test]
#[ignore = "explicit real DLL/virtual queries and owned message-window lifetimes; no screen capture"]
fn consumes_queries_through_real_factories_and_preserves_values_after_unload()
-> Result<(), Box<dyn std::error::Error>> {
    let build = fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_DELEGATE_NATIVE_BUILD").ok_or("native query build required")?,
    ))?;
    let root = PathBuf::from(
        std::env::var_os("MFS_DELEGATE_QUERY_RECORDS")
            .ok_or("new external query records required")?,
    );
    assert!(root.is_absolute());
    fs::create_dir(&root)?;
    let ops = Ops::load(&build.join("mfs_delegate_bridge.dll"))?;
    ops.reset();
    let host = host::Guard::new(&ops);
    let previous = crate::platform::windows::manager::probe::current();
    struct Dpi(isize);
    impl Drop for Dpi {
        fn drop(&mut self) {
            assert!(crate::platform::windows::manager::probe::set(self.0));
        }
    }
    let _dpi = Dpi(previous);
    // SAFETY: All addresses belong to the checked bridge kept by ops.
    let query: Query = unsafe { ops.module.symbol(c"mfs_dn_query")? };
    let create: WindowCreate = unsafe { ops.module.symbol(c"mfs_dn_query_window_create")? };
    let destroy: WindowCall = unsafe { ops.module.symbol(c"mfs_dn_query_window_destroy")? };
    let valid: WindowCall = unsafe { ops.module.symbol(c"mfs_dn_query_window_valid")? };
    let mut recorder = Recorder {
        query,
        rows: Vec::new(),
        output: BufWriter::new(fs::File::create_new(root.join("queries.tsv"))?),
    };
    let mut lifetimes = String::new();
    for side in ["source", "rust"] {
        let directory = root.join(side);
        fs::create_dir(&directory)?;
        let module = directory.join("MaaWin32ControlUnit.dll");
        fs::copy(build.join(format!("mfs_delegate_{side}.dll")), &module)?;
        for (case, (hwnd, method)) in case_matrix("win32").unwrap().into_iter().enumerate() {
            let hwnd = usize::try_from(hwnd)?;
            assert!(!library::loaded(&module));
            let mut boundary = Native::new(&directory, &ops).unwrap();
            let mut loader = boundary.new_loader().unwrap();
            assert!(boundary.load(&mut loader).unwrap());
            let unit = boundary
                .create(&mut loader, hwnd, method, 0, 0)
                .unwrap()
                .unwrap();
            let mut row = Row {
                side,
                kind: "raw",
                case,
                phase: "cold",
                hwnd,
                method,
                valid: -1,
                result: QueryResult::default(),
            };
            recorder.record(unit, row.clone(), false);
            if hwnd == 0 && method == 0 {
                let view = boundary.view(&loader, unit).unwrap();
                assert!(boundary.connect(&mut loader, &view).unwrap());
                row.phase = "connected";
                recorder.record(unit, row.clone(), true);
                assert!(boundary.inactive(&mut loader, &view).unwrap());
                row.phase = "inactive";
                recorder.record(unit, row, true);
            }
            drop(loader);
            assert!(!library::loaded(&module));
            use std::fmt::Write;
            writeln!(lifetimes, "{side}\traw\t{case}\t1")?;
        }
        for case in 0..40 {
            // SAFETY: A new hidden message-only window belongs to this caller.
            let hwnd = unsafe { create() };
            assert_ne!(hwnd, 0);
            let mut window = Window {
                value: hwnd,
                destroy,
            };
            assert_eq!(unsafe { valid(hwnd) }, 1);
            let method = if case % 2 == 0 { 0 } else { 1 << 63 };
            let mut boundary = Native::new(&directory, &ops).unwrap();
            let mut loader = boundary.new_loader().unwrap();
            assert!(boundary.load(&mut loader).unwrap());
            let unit = boundary
                .create(&mut loader, hwnd, method, 0, 0)
                .unwrap()
                .unwrap();
            let view = boundary.view(&loader, unit).unwrap();
            let mut row = Row {
                side,
                kind: "window",
                case,
                phase: "cold",
                hwnd,
                method,
                valid: unsafe { valid(hwnd) },
                result: QueryResult::default(),
            };
            recorder.record(unit, row.clone(), false);
            assert_eq!(boundary.connect(&mut loader, &view).unwrap(), method == 0);
            row.phase = "connected";
            recorder.record(unit, row.clone(), method == 0);
            window.close();
            assert_eq!(unsafe { valid(hwnd) }, 0);
            row.phase = "destroyed";
            row.valid = unsafe { valid(hwnd) };
            recorder.record(unit, row.clone(), false);
            assert!(boundary.inactive(&mut loader, &view).unwrap());
            row.phase = "inactive";
            recorder.record(unit, row, false);
            drop(loader);
            assert!(!library::loaded(&module));
            use std::fmt::Write;
            writeln!(lifetimes, "{side}\twindow\t{case}\t1")?;
        }
    }
    fs::write(root.join("lifetimes.tsv"), lifetimes)?;
    recorder.output.flush()?;
    assert_eq!(recorder.rows.len(), 500);
    assert_eq!(host.counts(), [128, 128, 0, 0, 41]);
    assert_eq!(host.query_counts(), [250; 4]);
    let (events, owners) = ops.events();
    assert_eq!(owners, 0);
    assert_eq!(events.len(), 2836);
    let thread = crate::platform::windows::manager::probe::thread();
    let mut table = String::new();
    for event in events {
        assert_eq!(event.thread, thread);
        use std::fmt::Write;
        writeln!(
            table,
            "{}\t{}\t{}\t{}\t{}\t{}",
            event.kind, event.thread, event.a, event.b, event.c, event.d
        )?;
    }
    fs::write(root.join("events.tsv"), table)?;
    assert_eq!(DESTROY_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(library::CLOSE_ERROR.load(Ordering::Acquire), 0);
    drop(host);
    drop(ops);
    // Only fixed CPU snapshots cross this joined thread after all unit/module
    // and HWND owners end; no native string/object/allocator lease escapes.
    let held = std::thread::spawn(move || {
        recorder
            .rows
            .into_iter()
            .map(|row| row.line())
            .collect::<String>()
    })
    .join()
    .unwrap();
    fs::write(root.join("held.tsv"), &held)?;
    assert_eq!(fs::read_to_string(root.join("queries.tsv"))?, held);
    fs::write(
        root.join("summary.json"),
        format!(
            "{{\"layout_version\":1,\"thread\":{thread},\"units\":256,\"raw_units\":176,\"window_units\":80,\"snapshots\":500,\"virtual_queries\":2000,\"events\":2836,\"host_created\":128,\"host_destroyed\":128,\"host_queries\":[250,250,250,250],\"host_captures\":0,\"host_conversions\":0,\"host_inactive\":41,\"retained_snapshots\":500,\"actual_captures\":0}}\n"
        ),
    )?;
    println!(
        "real virtual manager queries units=256 snapshots=500 calls=2000 host_queries=1000 window_lifetimes=80 retained_after_unload=500 captures=0 input_none=1 passed"
    );
    Ok(())
}
