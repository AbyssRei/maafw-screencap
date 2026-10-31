// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Same live original/Rust unit reentered synchronously from each callback.
//! No concurrent destruction, input action or public SDK reentry policy.
use super::*;

enum Value<'a> {
    Scalar,
    Uuid(StringOutput<'a>),
    Image(Image<'a>),
    Info(Info<'a>),
}
struct Observation<'a> {
    operation: u32,
    status: i32,
    value: u64,
    output: Value<'a>,
}
impl<'a> Observation<'a> {
    fn query(lib: &'a Lib, unit: *mut c_void, operation: u32) -> Self {
        let mut output = match operation {
            4 => Value::Uuid(StringOutput::new(lib, SEED)),
            5 => Value::Image(lib.seed()),
            6 => Value::Info(Objects(lib).empty_object().unwrap()),
            _ => Value::Scalar,
        };
        let destination = match &mut output {
            Value::Scalar => std::ptr::null_mut(),
            Value::Uuid(value) => value.pointer.as_ptr(),
            Value::Image(value) => value.pointer.as_ptr(),
            Value::Info(value) => value.pointer.as_ptr(),
        };
        let mut value = 0;
        // SAFETY: A retained actual unit and uniquely owned matching native
        // output. Byte views end before mutation; no mutable Rust state borrow
        // spans this call or its synchronous nested callback.
        let status = unsafe { (lib.unit.query)(unit, operation, destination, &mut value) };
        Self {
            operation,
            status,
            value,
            output,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        match &self.output {
            Value::Scalar => Vec::new(),
            Value::Uuid(value) => value.bytes(),
            Value::Image(value) => value.pixels().to_vec(),
            Value::Info(value) => value.bytes().unwrap(),
        }
    }
    fn check(&self, cycle: usize, depth: u32, fault: bool) {
        assert_eq!(
            self.status,
            if fault { 3 } else { 0 },
            "cycle={cycle} depth={depth} op={}",
            self.operation
        );
        let expected = if fault {
            0
        } else {
            scalar(cycle, depth, self.operation)
        };
        assert_eq!(self.value, expected);
        assert_eq!(
            self.bytes(),
            expected_bytes(cycle, depth, self.operation, fault)
        );
        if let Value::Image(image) = &self.output {
            let view = image.view();
            assert_eq!(
                (
                    view.width,
                    view.height,
                    view.channels,
                    view.stride,
                    view.length,
                    view.references
                ),
                (7, 5, 3, 21, 105, 1)
            );
        }
    }
    fn save(&self, directory: &Path, name: &str) {
        if self.operation >= 4 {
            std::fs::write(directory.join(format!("{name}.bin")), self.bytes()).unwrap();
        }
        if let Value::Image(image) = &self.output {
            let view = image.view();
            std::fs::write(
                directory.join(format!("{name}.layout")),
                format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    view.width,
                    view.height,
                    view.channels,
                    view.stride,
                    view.kind,
                    view.length,
                    view.references
                ),
            )
            .unwrap();
        }
    }
}
fn code(cycle: usize, depth: u32) -> u8 {
    [0, 1, 2, 255][(cycle + usize::try_from(depth).unwrap()) % 4]
}
fn feature(cycle: usize, depth: u32) -> u64 {
    u64::MAX - u64::try_from(cycle).unwrap() * 17 - u64::from(depth)
}
fn scalar(cycle: usize, depth: u32, operation: u32) -> u64 {
    match operation {
        0..=2 => {
            u64::from(boolean_value(code(cycle, depth), usize::try_from(operation).unwrap()) != 0)
        }
        3 => feature(cycle, depth),
        4..=6 => 1,
        _ => unreachable!(),
    }
}
fn expected_bytes(cycle: usize, depth: u32, operation: u32, fault: bool) -> Vec<u8> {
    match (operation, fault) {
        (4, false) => format!("uuid-{cycle}-{depth}\0")
            .bytes()
            .chain([0xff])
            .collect(),
        (5, false) => (0..105)
            .map(|i| {
                u8::try_from((i * 17 + cycle * 23 + usize::try_from(depth).unwrap() * 41 + 7) % 256)
                    .unwrap()
            })
            .collect(),
        (6, false) => {
            format!("{{\"cycle\":{cycle},\"depth\":{depth},\"type\":\"custom\"}}").into_bytes()
        }
        (4, true) => SEED.to_vec(),
        (5, true) => [9, 17, 23].repeat(35),
        (6, true) => b"{}".to_vec(),
        _ => Vec::new(),
    }
}
struct Depth<'a>(&'a Cell<u32>);
impl Drop for Depth<'_> {
    fn drop(&mut self) {
        self.0.set(0);
    }
}
struct Reentry<'a> {
    controller: Controller<'a>,
    context: Cell<*mut c_void>,
    unit: Cell<*mut c_void>,
    cycle: usize,
    inner: Cell<u32>,
    fault: Cell<bool>,
    depth: Cell<u32>,
    events: RefCell<Vec<(u32, u32)>>,
    nested: RefCell<Option<Observation<'a>>>,
}
impl Reentry<'_> {
    fn install(&self) {
        let lib = self.controller.lib;
        let argument = (self as *const Self).cast_mut().cast();
        // SAFETY: Stable Reentry address, same live borrowed table/context on
        // one owner thread. Installing callbacks does not invoke them. Nested
        // fault replacement is restored before the outer callback resumes.
        unsafe {
            assert_eq!(
                (lib.configure)(
                    self.context.get(),
                    Some(on_connect),
                    Some(on_connected),
                    Some(on_inactive),
                    Some(on_image),
                    argument
                ),
                0
            );
            assert_eq!(
                (lib.identity_configure)(
                    self.context.get(),
                    Some(on_uuid),
                    Some(on_features),
                    argument
                ),
                0
            );
            assert_eq!(
                (lib.metadata.configure)(self.context.get(), Some(on_info), argument),
                0
            );
        }
    }
    fn callback<T>(
        &self,
        operation: u32,
        failure: T,
        action: impl FnOnce(&Controller<'_>) -> T,
    ) -> T {
        catch_unwind(AssertUnwindSafe(|| {
            assert_eq!(std::thread::current().id(), self.controller.thread);
            let depth = self.depth.get();
            assert!(depth <= 1);
            self.events.borrow_mut().push((depth, operation));
            if depth == 0 {
                self.depth.set(1);
                let _depth = Depth(&self.depth);
                if self.fault.get() {
                    // SAFETY: Install a real C++ throw only after this outer
                    // callback has begun. Even equal inner/outer operations
                    // still exercise an actual nested virtual invocation.
                    assert_eq!(
                        unsafe {
                            (self.controller.lib.unit.fault)(self.context.get(), self.inner.get())
                        },
                        0
                    );
                }
                let nested =
                    Observation::query(self.controller.lib, self.unit.get(), self.inner.get());
                if self.fault.get() {
                    self.install();
                }
                nested.check(self.cycle, 1, self.fault.get());
                assert!(self.nested.borrow_mut().replace(nested).is_none());
            }
            self.controller.code.set(255);
            self.controller.boolean.set(code(self.cycle, depth));
            *self.controller.uuid.borrow_mut() = expected_bytes(self.cycle, depth, 4, false);
            *self.controller.pixels.borrow_mut() = expected_bytes(self.cycle, depth, 5, false);
            *self.controller.info.borrow_mut() = format!(
                "{{\"cycle\":{},\"depth\":{depth},\"type\":\"foreign\"}}",
                self.cycle
            )
            .into_bytes();
            action(&self.controller)
        }))
        .unwrap_or_else(|_| {
            self.controller.panic.set(true);
            failure
        })
    }
}
unsafe fn state<'a>(argument: *mut c_void) -> &'a Reentry<'a> {
    // SAFETY: Exact retained Reentry object on its original owner channel.
    unsafe { &*argument.cast::<Reentry<'a>>() }
}
macro_rules! boolean_callback {
    ($name:ident,$operation:expr) => {
        unsafe extern "C" fn $name(argument: *mut c_void) -> u8 {
            // SAFETY: Exact stable registered state, retained through shutdown.
            unsafe { state(argument) }.callback($operation, 0, |controller| {
                boolean(controller, $operation as usize)
            })
        }
    };
}
boolean_callback!(on_connect, 0);
boolean_callback!(on_connected, 1);
boolean_callback!(on_inactive, 2);
unsafe extern "C" fn on_features(argument: *mut c_void) -> u64 {
    // SAFETY: Exact registered state and synchronous retained unit.
    let state = unsafe { state(argument) };
    let depth = state.depth.get();
    state.callback(3, 0, |controller| {
        controller.enter(3);
        feature(state.cycle, depth)
    })
}
unsafe extern "C" fn on_uuid(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Actual live temporary string, same exact controller subobject.
    unsafe { state(argument) }.callback(4, 0, |controller| unsafe {
        uuid(
            (controller as *const Controller<'_>).cast_mut().cast(),
            buffer,
        )
    })
}
unsafe extern "C" fn on_image(argument: *mut c_void, buffer: *mut OpaqueBuffer) -> u8 {
    // SAFETY: Actual live temporary image, same exact controller subobject.
    unsafe { state(argument) }.callback(5, 0, |controller| unsafe {
        shot(
            (controller as *const Controller<'_>).cast_mut().cast(),
            buffer,
        )
    })
}
unsafe extern "C" fn on_info(argument: *mut c_void, buffer: *mut OpaqueString) -> u8 {
    // SAFETY: Actual live temporary string, same exact controller subobject.
    unsafe { state(argument) }.callback(6, 0, |controller| unsafe {
        info(
            (controller as *const Controller<'_>).cast_mut().cast(),
            buffer,
        )
    })
}

fn scope_controls(lib: &Lib, directory: &Path) {
    let native = Native::new(lib, true);
    let objects = Objects(lib);
    let mut rows = String::new();
    // SAFETY: Real native caller values outside every acceptance buffer scope.
    assert_eq!(unsafe { (lib.unit.publication_probe)() }, 3);
    rows.push_str("outside-publication\t3\n");
    let mut outer_value = native
        .with_info_buffer(|outer| {
            let outer_bytes = br#"{"scope":"outer"}"#;
            // SAFETY: Complete readable fixture bytes and actual live outer buffer.
            status(unsafe {
                (lib.string_set)(outer.0.as_ptr(), outer_bytes.as_ptr(), outer_bytes.len())
            })?;
            let mut anchor = objects.empty_object()?;
            let mut outer_value = objects.parse_object(outer)?.unwrap();
            assert_eq!(objects.set_custom_type(&mut outer_value), Err(1));
            rows.push_str("outer-type-before-release\t1\n");
            let mut inner_value = native.with_info_buffer(|inner| {
                let inner_bytes = br#"{"scope":"inner"}"#;
                // SAFETY: Distinct complete bytes and live nested buffer.
                status(unsafe {
                    (lib.string_set)(inner.0.as_ptr(), inner_bytes.as_ptr(), inner_bytes.len())
                })?;
                assert!(matches!(objects.parse_object(outer), Err(1)));
                rows.push_str("foreign-buffer-parse\t1\n");
                assert_eq!(objects.set_custom_type(&mut anchor), Err(1));
                rows.push_str("outer-type-during-inner\t1\n");
                let mut inner_value = objects.parse_object(inner)?.unwrap();
                assert_eq!(objects.set_custom_type(&mut inner_value), Err(1));
                rows.push_str("inner-type-before-release\t1\n");
                Ok(inner_value)
            })?;
            objects.set_custom_type(&mut inner_value)?;
            rows.push_str("inner-type-after-release\t0\n");
            assert_eq!(
                inner_value.bytes()?,
                br#"{"scope":"inner","type":"custom"}"#
            );
            std::fs::write(
                directory.join("control-inner.json.bin"),
                inner_value.bytes()?,
            )
            .unwrap();
            Ok(outer_value)
        })
        .unwrap();
    objects.set_custom_type(&mut outer_value).unwrap();
    rows.push_str("outer-type-after-release\t0\n");
    assert_eq!(
        outer_value.bytes().unwrap(),
        br#"{"scope":"outer","type":"custom"}"#
    );
    std::fs::write(
        directory.join("control-outer.json.bin"),
        outer_value.bytes().unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("scope-controls.tsv"), rows).unwrap();
}

#[test]
#[ignore = "requires complete original Custom API, real vtables and callback exception fixtures"]
fn preserves_same_unit_nested_queries_faults_and_independent_callers() {
    let directory =
        PathBuf::from(env::var_os("MFS_CUSTOM_CAPTURE_ARTIFACTS").unwrap()).join("reentry");
    std::fs::create_dir(&directory).unwrap();
    let lib = Lib::open();
    let mut rows = std::fs::File::create_new(directory.join("queries.tsv")).unwrap();
    let mut events = std::fs::File::create_new(directory.join("callbacks.tsv")).unwrap();
    let mut lifecycles = std::fs::File::create_new(directory.join("lifecycles.tsv")).unwrap();
    let mut held = Vec::new();
    for cycle in 0..20 {
        let state = Reentry {
            controller: Controller {
                lib: &lib,
                thread: std::thread::current().id(),
                panic: Cell::new(false),
                calls: Cell::new([0; 7]),
                code: Cell::new(255),
                boolean: Cell::new(255),
                populate: Cell::new(true),
                features: 0,
                uuid: RefCell::new(Vec::new()),
                pixels: RefCell::new(Vec::new()),
                info: RefCell::new(Vec::new()),
            },
            context: Cell::new(std::ptr::null_mut()),
            unit: Cell::new(std::ptr::null_mut()),
            cycle,
            inner: Cell::new(0),
            fault: Cell::new(false),
            depth: Cell::new(0),
            events: RefCell::new(Vec::new()),
            nested: RefCell::new(None),
        };
        let native = Native::new(&lib, true);
        state.context.set(native.context.as_ptr());
        state.install();
        let host = Box::try_new(Host {
            manager: Manager::create(Some(&native)).unwrap(),
            native: &native,
            dropped: Cell::new(0),
            operations: Cell::new(0),
            panic: Cell::new(false),
            thread: std::thread::current().id(),
        })
        .unwrap();
        let mut source = std::ptr::null_mut();
        let mut rust = std::ptr::null_mut();
        // SAFETY: Stable borrowed callback table/state and Host; actual original
        // creation APIs transfer uniquely owned units without invoking capture.
        unsafe {
            assert_eq!((lib.unit.source)(native.context.as_ptr(), &mut source), 0);
            assert_eq!(
                (lib.unit.rust)(
                    Some(dispatch),
                    Some(release),
                    (&*host as *const Host<'_, '_>).cast_mut().cast(),
                    &mut rust
                ),
                0
            );
        }
        let source = Unit {
            pointer: NonNull::new(source).unwrap(),
            lib: &lib,
            rust: false,
        };
        let rust = Unit {
            pointer: NonNull::new(rust).unwrap(),
            lib: &lib,
            rust: true,
        };
        for outer in 0..7 {
            for inner in 0..7 {
                for fault in [false, true] {
                    for (side, unit) in [("source", &source), ("rust", &rust)] {
                        state.inner.set(inner);
                        state.fault.set(fault);
                        state.unit.set(unit.pointer.as_ptr());
                        state.events.borrow_mut().clear();
                        let outer_value = Observation::query(&lib, unit.pointer.as_ptr(), outer);
                        assert!(
                            !state.controller.panic.get(),
                            "callback cycle={cycle} outer={outer} inner={inner} fault={fault} side={side}"
                        );
                        assert!(!host.panic.get());
                        assert_eq!(state.depth.get(), 0);
                        assert_eq!(native.unit_destination.get(), (0, std::ptr::null_mut()));
                        outer_value.check(cycle, 0, false);
                        let inner_value = state.nested.borrow_mut().take().unwrap();
                        // Re-read the actual nested output after the outer
                        // caller has published, so misrouted destinations or
                        // temporary-owner invalidation cannot pass byte checks.
                        inner_value.check(cycle, 1, fault);
                        let expected_events = if fault {
                            vec![(0, outer)]
                        } else {
                            vec![(0, outer), (1, inner)]
                        };
                        assert_eq!(*state.events.borrow(), expected_events);
                        let flag = u8::from(fault);
                        for (depth, operation) in expected_events {
                            writeln!(
                                events,
                                "{cycle}\t{outer}\t{inner}\t{flag}\t{side}\t{depth}\t{operation}"
                            )
                            .unwrap();
                        }
                        for (depth, observation) in [(1, inner_value), (0, outer_value)] {
                            writeln!(
                                rows,
                                "{cycle}\t{outer}\t{inner}\t{flag}\t{side}\t{depth}\t{}\t{}\t{}",
                                observation.operation, observation.status, observation.value
                            )
                            .unwrap();
                            let name = format!("{cycle}-{outer}-{inner}-{flag}-{side}-{depth}");
                            observation.save(&directory, &name);
                            if !fault && outer == inner && outer >= 4 {
                                held.push((cycle, depth, name, observation));
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(state.controller.calls.get(), [42; 7]);
        assert_eq!(host.operations.get(), 196);
        drop(source);
        drop(rust);
        assert_eq!(host.dropped.get(), 1);
        state.unit.set(std::ptr::null_mut());
        writeln!(
            lifecycles,
            "{cycle}\t196\t{:?}\t1",
            state.controller.calls.get()
        )
        .unwrap();
        drop(host);
        drop(native);
    }
    assert_eq!(held.len(), 240);
    let mut cpu = Vec::new();
    for (cycle, depth, name, observation) in held {
        observation.check(cycle, depth, false);
        let bytes = observation.bytes();
        std::fs::write(directory.join(format!("{name}.held")), &bytes).unwrap();
        cpu.push((
            bytes,
            expected_bytes(cycle, depth, observation.operation, false),
        ));
    }
    scope_controls(&lib, &directory);
    let mut counts = Vec::new();
    for index in 0..7 {
        // SAFETY: Scalar native counters after actual public virtual teardown.
        counts.push(unsafe { (lib.unit.count)(index) });
    }
    counts.extend([lib.count(0), lib.count(1), lib.count(6)]);
    // SAFETY: Scalar metadata ownership counter in the retained module.
    counts.push(unsafe { (lib.metadata.count)(0) });
    counts.extend([lib.count(2), lib.count(3), lib.count(4), lib.count(5)]);
    for index in 1..5 {
        // SAFETY: Scalar metadata operation counts, all native owners gone.
        counts.push(unsafe { (lib.metadata.count)(index) });
    }
    assert_eq!(
        counts,
        [
            20, 20, 20, 20, 20, 420, 420, 0, 0, 0, 0, 560, 560, 1122, 1122, 1681, 422, 422, 422
        ]
    );
    std::fs::write(
        directory.join("stats.tsv"),
        counts
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{i}\t{v}\n"))
            .collect::<String>(),
    )
    .unwrap();
    // SAFETY: Actual compiled header type, not a frozen OpenCV bit packing.
    std::fs::write(
        directory.join("pixel-kind.txt"),
        format!("{}\n", unsafe { (lib.abi)(12) }),
    )
    .unwrap();
    drop(lib);
    std::thread::spawn(move || {
        for (actual, expected) in cpu {
            assert_eq!(actual, expected);
        }
    })
    .join()
    .unwrap();
    eprintln!(
        "Custom reentry lifecycles=20 operation_pairs=49 fault_variants=2 actual_queries=7840 nested_cpp_exceptions=1960 held_native_values=240 scope_controls=7 input_actions=0 passed"
    );
}
