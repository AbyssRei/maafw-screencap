// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual Wlr compatibility constructor + same native factory, with a controlled
//! input owner retaining the same client. No input action implementation.
use super::*;
use crate::backends::linux::manager::{
    self,
    host::{InputOwner, InputProvider},
    native::{Error as HostError, Evidence as ManagerEvidence, HostedFactory},
};
use std::{cell::RefCell, os::fd::IntoRawFd, os::unix::net::UnixStream, rc::Rc};

#[derive(Default)]
struct State {
    outcome: u64,
    created: u64,
    initialized: u64,
    inactive: u64,
    destroyed: u64,
    create_refs: usize,
    destroy_refs: usize,
    connected: bool,
    closed: u64,
    errors: u64,
    client: Weak<Client>,
    counts: Option<Arc<Counts>>,
}
struct Host(Rc<RefCell<State>>);
struct Owner {
    client: Arc<Client>,
    state: Rc<RefCell<State>>,
    sockets: Option<(UnixStream, UnixStream)>,
}
impl InputProvider<Arc<Client>, HostError> for Host {
    type Input = Owner;
    fn create(
        &mut self,
        config: &manager::Config,
        client: Option<&Arc<Client>>,
    ) -> Result<Owner, HostError> {
        assert_eq!((config.method, config.input_method), (1, 1));
        assert!(config.use_win32_vk_code);
        let client = client.unwrap().clone();
        assert!(client.connected().unwrap());
        let sockets = UnixStream::pair().unwrap();
        let mut state = self.0.borrow_mut();
        state.created += 1;
        state.create_refs = Arc::strong_count(&client);
        state.client = Arc::downgrade(&client);
        // Only a CPU scalar ledger survives native owners. This test lives in
        // the owning module; no extra product visibility or native ref added.
        state.counts = Some(client.context.counts.clone());
        drop(state);
        Ok(Owner {
            client,
            state: self.0.clone(),
            sockets: Some(sockets),
        })
    }
}
impl InputOwner for Owner {
    type Error = HostError;
    fn init(&mut self) -> Result<bool, HostError> {
        let mut state = self.state.borrow_mut();
        state.initialized += 1;
        if state.outcome == 2 {
            Err(HostError::HostInput {
                operation: "Wlr host init",
                code: 97,
            })
        } else {
            Ok(state.outcome == 0)
        }
    }
    fn inactive(&mut self) {
        self.state.borrow_mut().inactive += 1;
    }
    fn features(&self) -> u64 {
        u64::MAX
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.destroy_refs = Arc::strong_count(&self.client);
        state.connected = self.client.connected().unwrap_or(false);
        if state.destroy_refs != 1 || !state.connected {
            state.errors += 1;
        }
        if let Some((first, second)) = self.sockets.take() {
            for socket in [first, second] {
                // SAFETY: Detach each uniquely owned fixture fd immediately
                // before its sole close; no native callback consumes these.
                if unsafe { libc::close(socket.into_raw_fd()) } == 0 {
                    state.closed += 1;
                } else {
                    state.errors += 1;
                }
            }
        }
        state.destroyed += 1;
        // Manager/capture refs have ended. The last client ref ends naturally
        // AFTER this body, keeping native snapshots live until this point.
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
#[ignore = "requires whole original Wlr compat ABI oracle and real owned compositor"]
fn captures_after_host_init_outcomes_and_releases_shared_client_after_input() {
    use std::io::Write;
    let root = PathBuf::from(env::var_os("MFS_P1_WLR_HOST_MATRIX").unwrap());
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
        let state = Rc::new(RefCell::new(State {
            outcome: cycle % 3,
            ..State::default()
        }));
        let config = manager::Config::from_legacy_wlr(Some(socket.as_bytes()), 255)
            .unwrap()
            .unwrap();
        let mut unit = manager::Manager::<HostedFactory<Host>>::new(config);
        let metadata = unit.get_info().unwrap();
        let mut uuid = Vec::new();
        assert!(unit.request_uuid(&mut uuid).unwrap());
        let expected_uuid = [socket.as_bytes(), b"_", socket.as_bytes()].concat();
        assert_eq!(uuid, expected_uuid);
        std::fs::write(directory.join(format!("api-{cycle}.uuid")), &uuid).unwrap();
        std::fs::write(directory.join(format!("api-{cycle}.info")), &metadata).unwrap();
        assert_eq!(
            metadata,
            std::fs::read(source.join(format!("api-{cycle}.info"))).unwrap()
        );
        assert!(!unit.connected());
        assert_eq!(unit.get_features(), 0);
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
        let (result, code) = match connection.result {
            Ok(value) => (u64::from(value), 0),
            Err(HostError::HostInput {
                operation: "Wlr host init",
                code: 97,
            }) => (2, 97),
            other => panic!("unexpected Wlr host connection: {other:?}"),
        };
        assert_eq!(
            (result, code),
            [(1, 0), (0, 0), (2, 97)][usize::try_from(cycle % 3).unwrap()]
        );
        assert_eq!(unit.connected(), cycle % 3 == 0);
        assert_eq!(unit.get_features(), 3);
        let close = connection.close.unwrap();
        assert!(close.take().is_none());
        for call in 0..3_u64 {
            if call == 1 {
                assert!(unit.inactive());
            }
            let attempt = unit.capture_native(&mut output, &mut converter);
            assert!(attempt.result.unwrap());
            let ManagerEvidence::Wlr(evidence) = attempt.evidence else {
                panic!("missing Wlr capture")
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
        let Some(ManagerEvidence::Wlr(early)) = close.take() else {
            panic!("missing capture close")
        };
        // This snapshot was made while input still held the client. It MUST
        // not be mistaken for proof that every native callback owner retired.
        assert!(early.data_created > early.data_dropped);
        assert_eq!((early.frames, early.copied), (3, 3));
        assert_eq!((early.cleanup, early.fault, early.buffer_error), (0, 0, 0));
        let state = state.borrow();
        let counts = state.counts.as_ref().unwrap();
        let final_fds = fds();
        let final_tasks = tasks();
        save_tasks(
            &directory.join(format!("closed-{cycle}-task-ids.txt")),
            &final_tasks,
        );
        writeln!(cases,"{cycle}\t{}\t{result}\t{code}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{baseline_fds}\t{final_fds}\t{}\t{}\t{features}\t0\t0\t3\t1\t1",state.outcome,state.created,state.initialized,state.inactive,state.destroyed,state.create_refs,state.destroy_refs,u8::from(state.connected),state.closed,state.errors,u8::from(state.client.upgrade().is_none()),baseline_tasks.len(),u8::from(final_tasks==baseline_tasks)).unwrap();
        writeln!(
            ledgers,
            "{cycle}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            early.data_created,
            early.data_dropped,
            counts.data_created.load(Ordering::Acquire),
            counts.destroyed.load(Ordering::Acquire),
            counts.data_dropped.load(Ordering::Acquire),
            counts.frames.load(Ordering::Acquire),
            counts.copied.load(Ordering::Acquire),
            counts.cleanup.load(Ordering::Acquire),
            counts.source_fd_zero_retained.load(Ordering::Acquire),
            early.destroyed
        )
        .unwrap();
        cases.flush().unwrap();
        ledgers.flush().unwrap();
        assert_eq!(
            (
                state.created,
                state.initialized,
                state.inactive,
                state.destroyed,
                state.closed,
                state.errors
            ),
            (1, 1, 1, 1, 2, 0)
        );
        assert_eq!(
            (state.create_refs, state.destroy_refs, state.connected),
            (3, 1, true)
        );
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
        "native Rust Wlr ABI objects=20 host_true_false_error=7,7,6 captures=60 retained_raw=60 retained_bgr=60 legacy_constructors=20 shared_last_client=20 final_callback_owners_released=20 input_actions=0 passed"
    );
}
