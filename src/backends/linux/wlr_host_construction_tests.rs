// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real Wlr capture remains usable after a controlled host create error.
//! This never constructs an input owner/device or closes a live shared client.
use super::*;
use crate::backends::linux::manager::{
    self,
    host::{InputOwner, InputProvider},
    native::{Error as HostError, Evidence as ManagerEvidence, HostedFactory},
};
use std::{cell::RefCell, convert::Infallible, io::Write};

#[derive(Default)]
struct Construction {
    attempts: u64,
    refs: usize,
    connected: bool,
    client: Weak<Client>,
    counts: Option<Arc<Counts>>,
}
struct Host(Rc<RefCell<Construction>>);
// No input can be returned by this named error boundary. These operations
// cannot run; the live native capture still uses the ordinary manager factory.
impl InputOwner for Infallible {
    type Error = HostError;
    fn init(&mut self) -> Result<bool, HostError> {
        match *self {}
    }
    fn inactive(&mut self) {
        match *self {}
    }
    fn features(&self) -> u64 {
        match *self {}
    }
}
impl InputProvider<Arc<Client>, HostError> for Host {
    type Input = Infallible;
    fn create(
        &mut self,
        config: &manager::Config,
        client: Option<&Arc<Client>>,
    ) -> Result<Infallible, HostError> {
        assert_eq!((config.method, config.input_method), (1, 1));
        assert!(config.use_win32_vk_code);
        let client = client.unwrap().clone();
        let mut state = self.0.borrow_mut();
        state.attempts += 1;
        state.refs = Arc::strong_count(&client);
        state.connected = client.connected().unwrap();
        state.client = Arc::downgrade(&client);
        // Retain only CPU counters after all native owners disappear.
        state.counts = Some(client.context.counts.clone());
        Err(HostError::HostInput {
            operation: "Wlr host create",
            code: 98,
        })
    }
}
fn save_tasks(path: &Path, ids: &[std::ffi::OsString]) {
    let text = ids
        .iter()
        .map(|id| format!("{}\n", id.to_str().unwrap()))
        .collect::<String>();
    std::fs::write(path, text).unwrap();
}

#[test]
#[ignore = "requires whole original Wlr ABI constructor-error oracle and owned compositor"]
fn retains_the_native_capture_after_host_construction_error() {
    let root = PathBuf::from(env::var_os("MFS_P1_WLR_HOST_CONSTRUCTION_MATRIX").unwrap());
    let source = root.join("source");
    let directory = root.join("rust");
    std::fs::create_dir(&directory).unwrap();
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    std::fs::write(directory.join("socket-path.txt"), socket.as_bytes()).unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let mut cases = std::fs::File::create_new(directory.join("rust-cases.tsv")).unwrap();
    let mut ledgers = std::fs::File::create_new(directory.join("rust-ledgers.tsv")).unwrap();
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    save_tasks(&directory.join("baseline-task-ids.txt"), &baseline_tasks);
    let mut converter = ManagerConvert {
        adapter: &adapter,
        held: Vec::new(),
    };
    converter.held.try_reserve_exact(60).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    for cycle in 0..20_u64 {
        let state = Rc::new(RefCell::new(Construction::default()));
        let config = manager::Config::from_legacy_wlr(Some(socket.as_bytes()), 255)
            .unwrap()
            .unwrap();
        let mut unit = manager::Manager::<HostedFactory<Host>>::new(config);
        let metadata = unit.get_info().unwrap();
        let mut uuid = Vec::new();
        assert!(unit.request_uuid(&mut uuid).unwrap());
        assert_eq!(uuid, [socket.as_bytes(), b"_", socket.as_bytes()].concat());
        std::fs::write(directory.join(format!("api-{cycle}.uuid")), &uuid).unwrap();
        std::fs::write(directory.join(format!("api-{cycle}.info")), &metadata).unwrap();
        assert_eq!(
            metadata,
            std::fs::read(source.join(format!("api-{cycle}.info"))).unwrap()
        );
        let mut output = None;
        assert!(
            !unit
                .capture_native(&mut output, &mut converter)
                .result
                .unwrap()
        );
        assert!(output.is_none());
        let connection = unit.connect_host_native(Host(state.clone()));
        assert!(connection.failure.is_none());
        assert!(matches!(
            connection.result,
            Err(HostError::HostInput {
                operation: "Wlr host create",
                code: 98
            })
        ));
        let after_refs = state.borrow().client.strong_count();
        assert_eq!(after_refs, 2, "manager and retained capture own the client");
        assert!(!unit.connected());
        assert_eq!(unit.get_features(), 0);
        let close = connection.close.unwrap();
        assert!(close.take().is_none());
        for call in 0..3_u64 {
            if call == 1 {
                assert!(unit.inactive());
            }
            let attempt = unit.capture_native(&mut output, &mut converter);
            assert!(attempt.result.unwrap());
            let ManagerEvidence::Wlr(evidence) = attempt.evidence else {
                panic!("missing retained Wlr capture");
            };
            assert_eq!((evidence.frames, evidence.copied), (call + 1, call + 1));
            assert_eq!(
                (evidence.cleanup, evidence.fault, evidence.buffer_error),
                (0, 0, 0)
            );
            let image = output.take().unwrap();
            bgr(image.pixels());
            let stem = format!("host-{cycle}-call-{call}");
            assert_eq!(
                image.pixels(),
                std::fs::read(source.join(format!("{stem}.bgr"))).unwrap()
            );
            std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
            let sample = converter.held.last().unwrap();
            std::fs::write(directory.join(format!("{stem}.raw")), sample.pixels()).unwrap();
            std::fs::write(directory.join(format!("{stem}.meta.txt")), format!("conversion={:?}\nwidth={}\nheight={}\nnative_stride={}\nready_time={:?}\nflags={:?}\n", sample.conversion,sample.width,sample.height,sample.native_stride,sample.ready_time,sample.flags)).unwrap();
            held.push(image);
        }
        assert_eq!(unit.get_info().unwrap(), metadata);
        let features = unit.get_features();
        drop(unit);
        let Some(ManagerEvidence::Wlr(closed)) = close.take() else {
            panic!("missing retained capture close evidence");
        };
        assert_eq!((closed.frames, closed.copied), (3, 3));
        assert_eq!(
            (closed.cleanup, closed.fault, closed.buffer_error),
            (0, 0, 0)
        );
        assert!(closed.data_created > 0);
        assert_eq!(closed.data_created, closed.destroyed);
        assert_eq!(closed.data_created, closed.data_dropped);
        let state = state.borrow();
        let counts = state.counts.as_ref().unwrap();
        let final_fds = fds();
        let final_tasks = tasks();
        save_tasks(
            &directory.join(format!("closed-{cycle}-task-ids.txt")),
            &final_tasks,
        );
        // Zeros describe the uninhabited input owner and the unobserved native
        // input proxy slots. Source alone records its actual partial snapshots.
        writeln!(cases, "{cycle}\t98\t{}\t0\t0\t0\t0\t{}\t{after_refs}\t{}\t3\t{features}\t0\t{}\t{baseline_fds}\t{final_fds}\t{}\t{}\t0\t0", state.attempts,state.refs,u8::from(state.connected),u8::from(state.client.upgrade().is_none()),baseline_tasks.len(),u8::from(final_tasks==baseline_tasks)).unwrap();
        writeln!(
            ledgers,
            "{cycle}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            closed.data_created,
            closed.destroyed,
            closed.data_dropped,
            closed.frames,
            closed.copied,
            closed.cleanup,
            closed.fault,
            closed.buffer_error,
            closed.source_fd_zero_retained
        )
        .unwrap();
        cases.flush().unwrap();
        ledgers.flush().unwrap();
        assert_eq!((state.attempts, state.refs, state.connected), (1, 3, true));
        assert!(state.client.upgrade().is_none());
        released_counts(counts);
        assert_eq!(final_fds, baseline_fds);
        assert_eq!(final_tasks, baseline_tasks);
        for sample in &converter.held {
            raw(sample);
        }
        for image in &held {
            bgr(image.pixels());
        }
    }
    for (index, (image, sample)) in held.iter().zip(&converter.held).enumerate() {
        bgr(image.pixels());
        raw(sample);
        std::fs::write(directory.join(format!("held-{index}.bgr")), image.pixels()).unwrap();
        std::fs::write(directory.join(format!("held-{index}.raw")), sample.pixels()).unwrap();
    }
    drop(converter);
    drop(held);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr host_create_failure objects=20 constructor_errors=20 captures=60 retained_raw=60 retained_bgr=60 final_callback_owners_released=20 resources_balanced=1 input_actions=0 passed"
    );
}
