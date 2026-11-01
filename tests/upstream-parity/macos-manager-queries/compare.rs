// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Real C++ virtual queries through original factories, on the SAME manager.
use super::*;
use crate::backends::manager_metadata::case_matrix;
#[cfg(unix)]
use std::ffi::CStr;
use std::{
    ffi::c_void,
    fs,
    mem::{align_of, offset_of, size_of},
    path::{Path, PathBuf},
    ptr::NonNull,
};

#[repr(C)]
struct Value {
    scalar: u64,
    length: usize,
    bytes: [u8; 256],
}
impl Default for Value {
    fn default() -> Self {
        Self {
            scalar: 0,
            length: 0,
            bytes: [0; 256],
        }
    }
}
#[repr(C)]
struct Host {
    context: *mut c_void,
    call: unsafe extern "C" fn(*mut c_void, u32, *mut Value) -> i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
struct Snapshot {
    features: u64,
    connected: u32,
    uuid_success: u32,
    uuid_length: usize,
    info_length: usize,
    uuid: [u8; 11],
    info: [u8; 256],
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            features: 0,
            connected: 0,
            uuid_success: 0,
            uuid_length: 0,
            info_length: 0,
            uuid: [0; 11],
            info: [0; 256],
        }
    }
}
#[cfg(target_os = "windows")]
use crate::platform::windows::test_library::Module;
#[cfg(unix)]
struct Module(NonNull<c_void>);
#[cfg(unix)]
impl Module {
    fn open(path: &Path) -> Self {
        use std::os::unix::ffi::OsStrExt;
        assert!(path.is_absolute());
        let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: Initialized terminated absolute path, explicit test adapter.
        Self(
            NonNull::new(unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) })
                .expect("load explicit query adapter"),
        )
    }
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        // SAFETY: Live module and valid terminated symbol name.
        let value = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        assert!(!value.is_null());
        assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        // SAFETY: Caller proves exact private header type and module lifetime.
        unsafe { std::mem::transmute_copy(&value) }
    }
}
#[cfg(unix)]
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: All native owners and function-pointer consumers ended first.
        assert_eq!(unsafe { libc::dlclose(self.0.as_ptr()) }, 0);
    }
}
type Create = unsafe extern "C" fn(u32, u64, *const Host, *mut *mut c_void) -> i32;
type Scalar = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type Query = unsafe extern "C" fn(*mut c_void, *mut Snapshot) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void) -> i32;
struct Ops {
    _module: Module,
    create: Create,
    connect: Scalar,
    inactive: Scalar,
    query: Query,
    destroy: Destroy,
    environment: u32,
}
impl Ops {
    fn load(path: &Path) -> Self {
        let module = Module::open(path);
        // SAFETY: All signatures match bridge.h. Ops keeps the module live.
        unsafe {
            let layout: unsafe extern "C" fn(u32) -> usize = module.symbol(c"mfs_mq_layout");
            let expected = [
                1,
                size_of::<Value>(),
                align_of::<Value>(),
                offset_of!(Value, scalar),
                offset_of!(Value, length),
                offset_of!(Value, bytes),
                size_of::<Host>(),
                align_of::<Host>(),
                offset_of!(Host, context),
                offset_of!(Host, call),
                size_of::<Snapshot>(),
                align_of::<Snapshot>(),
                offset_of!(Snapshot, features),
                offset_of!(Snapshot, connected),
                offset_of!(Snapshot, uuid_success),
                offset_of!(Snapshot, uuid_length),
                offset_of!(Snapshot, info_length),
                offset_of!(Snapshot, uuid),
                offset_of!(Snapshot, info),
                8,
                8,
                8,
                NONE as usize,
                SCREEN_CAPTURE_KIT as usize,
                0,
            ];
            for (index, value) in expected.into_iter().enumerate() {
                assert_eq!(
                    layout(index.try_into().unwrap()),
                    value,
                    "layout item {index}"
                );
            }
            assert_eq!(layout(expected.len().try_into().unwrap()), usize::MAX);
            let environment: unsafe extern "C" fn() -> u32 = module.symbol(c"mfs_mq_environment");
            let environment = environment();
            #[cfg(not(target_os = "macos"))]
            assert_eq!(environment, 0, "portable controlled-unavailable model only");
            #[cfg(target_os = "macos")]
            assert_eq!(
                environment,
                if objc2::available!(macos = 14.0) {
                    1
                } else {
                    2
                }
            );
            Self {
                create: module.symbol(c"mfs_mq_create"),
                connect: module.symbol(c"mfs_mq_connect"),
                inactive: module.symbol(c"mfs_mq_inactive"),
                query: module.symbol(c"mfs_mq_query"),
                destroy: module.symbol(c"mfs_mq_destroy"),
                environment,
                _module: module,
            }
        }
    }
}
struct Unit<'a> {
    pointer: NonNull<c_void>,
    ops: &'a Ops,
}
impl Drop for Unit<'_> {
    fn drop(&mut self) {
        // SAFETY: Exactly one factory owner; context and module are still live.
        assert_eq!(unsafe { (self.ops.destroy)(self.pointer.as_ptr()) }, 0);
    }
}
#[cfg(target_os = "macos")]
type QueryManager = NativeManager;
#[cfg(not(target_os = "macos"))]
type QueryManager = Manager<()>;
struct Context {
    manager: QueryManager,
    calls: [u64; 6],
    fail: Option<u32>,
}
unsafe extern "C" fn dispatch(context: *mut c_void, operation: u32, output: *mut Value) -> i32 {
    if context.is_null() || output.is_null() {
        return 1;
    }
    // SAFETY: C++ calls synchronously on this owner thread, with a unique
    // Context borrow and initialized exclusive output. It never retains output
    // or reenters this callback. Rust panic is contained before the ABI returns.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let context = unsafe { &mut *context.cast::<Context>() };
        let output = unsafe { &mut *output };
        *output = Value::default();
        let index = usize::try_from(operation).unwrap();
        let Some(count) = context.calls.get_mut(index) else {
            return 1;
        };
        *count += 1;
        if context.fail == Some(operation) {
            return 2;
        }
        let mut text = String::new();
        output.scalar = match operation {
            0 => {
                #[cfg(target_os = "macos")]
                let connected = match context.manager.connect_native() {
                    Ok(value) => value,
                    Err(_) => return 2,
                };
                #[cfg(not(target_os = "macos"))]
                let connected = context
                    .manager
                    .connect_with(false, |_| -> Result<(), ()> {
                        panic!("controlled unavailable SCK must not construct")
                    })
                    .unwrap();
                u64::from(connected)
            }
            1 => u64::from(context.manager.connected()),
            2 => match context.manager.request_uuid(&mut text) {
                Ok(value) => u64::from(value),
                Err(_) => return 2,
            },
            3 => context.manager.get_features(),
            4 => {
                text = match context.manager.get_info() {
                    Ok(value) => value,
                    Err(_) => return 2,
                };
                0
            }
            5 => u64::from(context.manager.inactive()),
            _ => return 1,
        };
        if text.len() > output.bytes.len() {
            return 2;
        }
        output.length = text.len();
        output.bytes[..text.len()].copy_from_slice(text.as_bytes());
        0
    }));
    result.unwrap_or(2)
}
#[derive(Clone)]
struct Row {
    side: &'static str,
    case: usize,
    phase: &'static str,
    id: u32,
    method: u64,
    value: Snapshot,
}
impl Row {
    fn line(&self) -> String {
        let value = &self.value;
        let uuid = std::str::from_utf8(&value.uuid[..value.uuid_length]).unwrap();
        let info = std::str::from_utf8(&value.info[..value.info_length]).unwrap();
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{uuid}\t{info}\n",
            self.side,
            self.case,
            self.phase,
            self.id,
            self.method,
            value.connected,
            value.uuid_success,
            value.features
        )
    }
    fn validate(&self, connected: bool) {
        let value = &self.value;
        assert_eq!(value.connected, u32::from(connected));
        assert_eq!(value.uuid_success, 1);
        assert_eq!(value.features, 0);
        let uuid = self.id.to_string();
        let method = i64::from_ne_bytes(self.method.to_ne_bytes());
        let info = format!(
            "{{\"input_method\":0,\"screencap_method\":{method},\"type\":\"macos\",\"window_id\":{}}}",
            self.id
        );
        assert_eq!(value.uuid_length, uuid.len());
        assert_eq!(value.info_length, info.len());
        assert_eq!(&value.uuid[..uuid.len()], uuid.as_bytes());
        assert_eq!(&value.info[..info.len()], info.as_bytes());
        assert!(value.uuid[uuid.len()..].iter().all(|&b| b == 0));
        assert!(value.info[info.len()..].iter().all(|&b| b == 0));
    }
}
#[test]
#[ignore = "explicit original factories/virtual query adapter; portable model or formal native macOS, zero screenshots"]
fn consumes_original_virtual_queries_and_retains_cpu_snapshots_after_unload() {
    use std::io::Write;
    let library = fs::canonicalize(PathBuf::from(
        std::env::var_os("MFS_MAC_MANAGER_QUERY_LIBRARY").expect("explicit query adapter"),
    ))
    .unwrap();
    let root = PathBuf::from(
        std::env::var_os("MFS_MAC_MANAGER_QUERY_RECORDS").expect("new external query records"),
    );
    assert!(root.is_absolute());
    let product = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let ancestor = root
        .ancestors()
        .find(|path| path.exists())
        .unwrap()
        .canonicalize()
        .unwrap();
    assert!(!ancestor.starts_with(product));
    fs::create_dir(&root).unwrap();
    let ops = Ops::load(&library);
    let environment = ops.environment;
    let mut rows = Vec::new();
    let mut totals = [0_u64; 6];
    let mut records = fs::File::create(root.join("queries.tsv")).unwrap();
    let matrix = case_matrix("macos").unwrap();
    for side in ["source", "rust"] {
        for (case, &(id, method)) in matrix.iter().enumerate() {
            let id = u32::try_from(id).unwrap();
            let mut context = Context {
                manager: QueryManager::new(id, method),
                calls: [0; 6],
                fail: None,
            };
            let host = Host {
                context: (&mut context as *mut Context).cast(),
                call: dispatch,
            };
            let mut pointer = std::ptr::null_mut();
            // SAFETY: Fixed inputs; callback context is stable until Unit drop.
            assert_eq!(
                unsafe {
                    (ops.create)(
                        id,
                        method,
                        if side == "rust" {
                            &host
                        } else {
                            std::ptr::null()
                        },
                        &mut pointer,
                    )
                },
                0
            );
            let unit = Unit {
                pointer: NonNull::new(pointer).unwrap(),
                ops: &ops,
            };
            let connected = method != SCREEN_CAPTURE_KIT || environment == 1;
            for phase in ["cold", "connected", "reconnected", "inactive"] {
                if phase == "connected" || phase == "reconnected" {
                    let mut value = 99;
                    // SAFETY: Live original factory owner, exclusive scalar.
                    assert_eq!(unsafe { (ops.connect)(pointer, &mut value) }, 0);
                    assert_eq!(value, u32::from(connected));
                } else if phase == "inactive" {
                    let mut value = 99;
                    // SAFETY: Same live original interface, no capture work.
                    assert_eq!(unsafe { (ops.inactive)(pointer, &mut value) }, 0);
                    assert_eq!(value, 1);
                }
                let mut row = Row {
                    side,
                    case,
                    phase,
                    id,
                    method,
                    value: Snapshot::default(),
                };
                // SAFETY: Native temporaries close before fixed CPU output returns.
                assert_eq!(unsafe { (ops.query)(pointer, &mut row.value) }, 0);
                records.write_all(row.line().as_bytes()).unwrap();
                records.flush().unwrap();
                row.validate(phase != "cold" && connected);
                rows.push(row);
            }
            drop(unit);
            let expected = if side == "rust" {
                [2, 4, 4, 4, 4, 1]
            } else {
                [0; 6]
            };
            assert_eq!(context.calls, expected);
            for (total, count) in totals.iter_mut().zip(context.calls) {
                *total += count;
            }
        }
    }
    // Actual ABI rejection controls: failure outputs initialized, null-safe
    // original Destroy, and an injected Rust query failure contained by C++.
    let mut controls = fs::File::create(root.join("controls.tsv")).unwrap();
    // SAFETY: Null arguments follow the bridge's explicit rejection contract;
    // other contexts, owners and initialized outputs stay live per call.
    unsafe {
        let mut pointer = std::ptr::dangling_mut::<c_void>();
        let code = (ops.create)(0, 0, std::ptr::null(), std::ptr::null_mut());
        writeln!(controls, "null_create_output\t{code}").unwrap();
        assert_eq!(code, 1);
        let bad = Host {
            context: std::ptr::null_mut(),
            call: dispatch,
        };
        let code = (ops.create)(0, 0, &bad, &mut pointer);
        writeln!(
            controls,
            "null_host_context\t{code}\t{}",
            u8::from(pointer.is_null())
        )
        .unwrap();
        assert_eq!(code, 1);
        assert!(pointer.is_null());
        let mut value = 99;
        let code = (ops.connect)(std::ptr::null_mut(), &mut value);
        writeln!(controls, "null_connect_owner\t{code}\t{value}").unwrap();
        assert_eq!(code, 1);
        assert_eq!(value, 0);
        let mut snapshot = Snapshot {
            features: 99,
            ..Default::default()
        };
        let code = (ops.query)(std::ptr::null_mut(), &mut snapshot);
        writeln!(
            controls,
            "null_query_owner\t{code}\t{}",
            u8::from(snapshot == Snapshot::default())
        )
        .unwrap();
        assert_eq!(code, 1);
        assert_eq!(snapshot, Snapshot::default());
        let code = (ops.destroy)(std::ptr::null_mut());
        writeln!(controls, "null_original_destroy\t{code}").unwrap();
        assert_eq!(code, 0);
        let mut context = Context {
            manager: QueryManager::new(7, NONE),
            calls: [0; 6],
            fail: Some(1),
        };
        let host = Host {
            context: (&mut context as *mut Context).cast(),
            call: dispatch,
        };
        assert_eq!((ops.create)(7, NONE, &host, &mut pointer), 0);
        let unit = Unit {
            pointer: NonNull::new(pointer).unwrap(),
            ops: &ops,
        };
        let code = (ops.query)(pointer, &mut snapshot);
        writeln!(
            controls,
            "callback_failure\t{code}\t{}",
            u8::from(snapshot == Snapshot::default())
        )
        .unwrap();
        assert_eq!(code, 2);
        assert_eq!(snapshot, Snapshot::default());
        drop(unit);
        assert_eq!(context.calls, [0, 1, 0, 0, 0, 0]);
    }
    controls.flush().unwrap();
    drop(controls);
    assert_eq!(rows.len(), 528);
    for case in 0..matrix.len() {
        for phase in 0..4 {
            assert_eq!(
                rows[case * 4 + phase].value,
                rows[264 + case * 4 + phase].value
            );
        }
    }
    drop(records);
    drop(ops); // No native owners/function pointers in retained rows.
    let held = std::thread::spawn(move || {
        let mut text = String::new();
        for row in rows {
            let connected =
                row.phase != "cold" && (row.method != SCREEN_CAPTURE_KIT || environment == 1);
            row.validate(connected);
            text.push_str(&row.line());
        }
        text
    })
    .join()
    .unwrap();
    fs::write(root.join("held.tsv"), &held).unwrap();
    assert_eq!(fs::read(root.join("queries.tsv")).unwrap(), held.as_bytes());
    fs::copy(&library, root.join(library.file_name().unwrap())).unwrap();
    let summary = format!(
        "version=1\nenvironment={environment}\ncases=66\nfactory_units=133\nmatrix_units=132\nsnapshots=528\nvirtual_queries=2113\nmatrix_queries=2112\nretained=528\ncaptures=0\ninput_none=1\njoined_cpu_reader=1\ncallback_counts={},{},{},{},{},{}\nfailed_callback_count=1\nrejection_controls=6\n",
        totals[0], totals[1], totals[2], totals[3], totals[4], totals[5]
    );
    fs::write(root.join("summary.txt"), summary).unwrap();
    println!(
        "macOS manager virtual queries environment={environment} cases=66 factory_units=133 matrix_units=132 snapshots=528 queries=2113 retained=528 callbacks=1255 rejection_controls=6 captures=0 input_none=1 passed"
    );
}
