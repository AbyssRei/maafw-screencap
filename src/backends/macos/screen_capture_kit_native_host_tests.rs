// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::backends::macos::manager::{InputFeatures, Manager, SCREEN_CAPTURE_KIT};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct HostState {
    phase: u64,
    attempts: u64,
    drops: u64,
    queries: u64,
    events: Vec<[u64; 4]>,
}
struct Input {
    id: u64,
    state: Rc<RefCell<HostState>>,
}
impl InputFeatures for Input {
    fn features(&self) -> u64 {
        let mut state = self.state.borrow_mut();
        let phase = state.phase;
        let query = state.queries;
        state.events.push([5, self.id, phase, query]);
        state.queries += 1;
        [5, u64::MAX, 4][usize::try_from(phase).unwrap()]
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        let phase = state.phase;
        state.events.push([6, self.id, phase, 0]);
        state.drops += 1;
    }
}

#[test]
#[ignore = "native macOS real original/Rust SCK with controlled host input owners and owned scene"]
fn captures_after_host_input_failure_and_retains_images_after_native_manager_shutdown() {
    let adapter = Adapter::open();
    adapter.warm();
    warm_capture(&adapter);
    let root = directory().join("native-host");
    std::fs::create_dir(&root).unwrap();
    let library = env::var_os("MFS_MAC_MANAGER_QUERY_LIBRARY").unwrap();
    let library_bytes = std::fs::read(&library).unwrap();
    let path = CString::new(library.as_bytes()).unwrap();
    // SAFETY: This exact prepared source module is pinned until process exit.
    // The original implementation gives no block-disposal/drain observer; a
    // synchronous return is insufficient evidence to unload its callback code.
    // This deliberate fixture lifetime is not a source module cleanup success.
    let module = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
    assert!(!module.is_null());
    type Run = unsafe extern "C" fn(u32, *const std::ffi::c_char) -> i32;
    // SAFETY: native-host.h is the private primitive ABI authority. The pinned
    // module owns the callable for this synchronous call and any late native code.
    let pointer = unsafe { libc::dlsym(module, c"mfs_mh_native_run".as_ptr()) };
    assert!(!pointer.is_null());
    let original: Run = unsafe { std::mem::transmute(pointer) };
    let mut owner = unsafe { std::mem::zeroed::<libc::Dl_info>() };
    assert_ne!(unsafe { libc::dladdr(pointer, &mut owner) }, 0);
    assert!(!owner.dli_fname.is_null());
    let loaded = unsafe { CStr::from_ptr(owner.dli_fname) };
    assert_eq!(
        std::fs::canonicalize(std::ffi::OsStr::from_bytes(loaded.to_bytes())).unwrap(),
        std::fs::canonicalize(&library).unwrap()
    );
    std::fs::write(
        root.join("libmfs_macos_manager_queries.dylib"),
        &library_bytes,
    )
    .unwrap();
    std::fs::write(
        root.join("module-status.txt"),
        "schema=1\nprocess_pinned=1\nnative_input_actions=0\n",
    )
    .unwrap();
    let native_path = CString::new(root.as_os_str().as_bytes()).unwrap();
    // SAFETY: Nonzero fixture target, stable NUL-terminated fresh directory;
    // native code saves its own independent CPU images and contains exceptions.
    assert_eq!(unsafe { original(scene(), native_path.as_ptr()) }, 0);
    assert_eq!(std::fs::read(&library).unwrap(), library_bytes);
    let samples = held();
    let mut images = Vec::new();
    images.try_reserve_exact(60).unwrap();
    let mut records = String::new();
    for case in 0..20 {
        let input_method = 1 + case % 2;
        let state = Rc::new(RefCell::new(HostState::default()));
        let mut values = [0_u64; 18];
        let info;
        {
            let mut manager = Manager::<Arc<Capture>, Input>::with_host_input(
                scene(),
                SCREEN_CAPTURE_KIT,
                input_method,
            );
            values[0] = u64::from(manager.connected());
            values[1] = manager.get_features();
            let mut uuid = String::new();
            assert!(manager.request_uuid(&mut uuid).unwrap());
            assert_eq!(uuid, scene().to_string());
            for phase in 0..3 {
                state.borrow_mut().phase = phase;
                let connected = manager.connect_native_with_host_input(|window, input| {
                    assert_eq!((window, input), (scene(), input_method));
                    let mut current = state.borrow_mut();
                    current.attempts += 1;
                    let id = current.attempts;
                    current.events.push([4, id, phase, 0]);
                    if phase == 1 {
                        return Err(Error::new("controlled native host input", 91));
                    }
                    drop(current);
                    Ok(Input {
                        id,
                        state: state.clone(),
                    })
                });
                let slot = [2, 7, 11][usize::try_from(phase).unwrap()];
                values[slot] = match connected {
                    Ok(value) => u64::from(value),
                    Err(error) => {
                        assert_eq!(error, Error::new("controlled native host input", 91));
                        2
                    }
                };
                values[slot + 1] = u64::from(manager.connected());
                values[slot + 2] = manager.get_features();
                let trace = trace();
                let mut output = Image::Empty;
                values[slot + 3] = u64::from(
                    objc2::rc::autoreleasepool(|_| {
                        manager.capture_native(
                            &mut output,
                            converter(&adapter, &trace, &samples, false, Duration::ZERO),
                            trace.clone(),
                        )
                    })
                    .unwrap(),
                );
                assert_eq!(values[slot + 3], 1);
                drained(&trace, true);
                assert_eq!(trace.source_failure.load(Ordering::Acquire), 0);
                trace.reclaim_source_semaphore_for_fixture();
                bgr(output.pixels());
                assert_eq!(
                    output.pixels(),
                    std::fs::read(root.join(format!("source-current-{case}-{phase}.bgr"))).unwrap()
                );
                std::fs::write(
                    root.join(format!("rust-current-{case}-{phase}.bgr")),
                    output.pixels(),
                )
                .unwrap();
                images.push(output);
                if phase == 0 {
                    assert!(manager.inactive());
                    values[6] = manager.get_features();
                }
            }
            info = manager.get_info().unwrap();
        }
        let state = state.borrow();
        values[15] = state.attempts;
        values[16] = state.drops;
        values[17] = state.queries;
        use std::fmt::Write;
        writeln!(
            &mut records,
            "{case}\t{input_method}\t{}\t{:?}\t{:?}\t{info}",
            scene(),
            values,
            state.events
        )
        .unwrap();
    }
    std::fs::write(root.join("rust.tsv"), records).unwrap();
    stop_scene();
    for (index, image) in images.iter().enumerate() {
        bgr(image.pixels());
        std::fs::write(
            root.join(format!("rust-held-{}-{}.bgr", index / 3, index % 3)),
            image.pixels(),
        )
        .unwrap();
    }
    drop(images);
    assert_eq!(adapter.owners(), 0);
    let samples = samples.lock().unwrap();
    assert_eq!(samples.len(), 60);
    for (index, sample) in samples.iter().enumerate() {
        raw(sample);
        std::fs::write(
            root.join(format!("rust-raw-{}-{}.bgra", index / 3, index % 3)),
            sample.pixels(),
        )
        .unwrap();
    }
    std::fs::write(root.join("summary.txt"), "schema=1\ncases=20\nsource_captures=60\nrust_captures=60\ninput_failure_captures=40\ninput_actions=0\nsource_module_process_pinned=1\nrust_fixture_reclaimed_refs=61\nscene_closed=1\n").unwrap();
    drop(samples);
    drop(adapter);
    assert_eq!(MODULE_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "macOS native host cases=20 captures=120 input_failure_captures=40 retained_bgr=120 input_actions=0 source_module_pinned=1 passed"
    );
}
