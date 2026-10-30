// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real native capture with an explicit host init script and real owned fds.
//! The controlled helper is NOT real desktop Portal authorization/device input.
use super::manager_capture::{Convert, check_close};
use super::*;
use crate::backends::linux::manager::{
    self,
    compat::Portal,
    host::{InputOwner, InputProvider},
    native as managed,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Default)]
struct Stats {
    helper_ended: u32,
    helper_original_closed: u32,
    helper_cookie_before: u64,
    helper_cookie_after: u64,
    helper_offer_fd: Option<i32>,
    created: u32,
    initialized: u32,
    inactive: u32,
    destroyed: u32,
    native_closed_before_input: u32,
    host_fds_closed: u32,
    errors: u32,
    capture_observation_error: i32,
    close_errors: [i32; 2],
}
struct Helper {
    socket: Option<UnixStream>,
    state: Rc<RefCell<Stats>>,
}
impl Drop for Helper {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if let Some(fd) = state.helper_offer_fd {
            state.helper_cookie_after = super::super::fd_fault::socket_cookie(fd);
        }
        state.helper_ended += 1;
    }
}
struct Offer<'a> {
    socket: &'a str,
    node: u32,
    helper: Option<Helper>,
    state: Rc<RefCell<Stats>>,
    calls: Vec<&'static str>,
}
impl Portal for Offer<'_> {
    type Helper = u8;
    type Error = std::io::Error;
    fn create(&mut self) -> Result<u8, Self::Error> {
        self.calls.push("create");
        assert!(self.helper.is_none());
        self.helper = Some(Helper {
            socket: None,
            state: self.state.clone(),
        });
        Ok(0)
    }
    fn open(&mut self, _: u8) -> Result<u8, Self::Error> {
        self.calls.push("open");
        let original = UnixStream::connect(self.socket)?;
        let before = super::super::fd_fault::socket_cookie(original.as_raw_fd());
        let offered = original.try_clone()?;
        assert_eq!(
            super::super::fd_fault::socket_cookie(offered.as_raw_fd()),
            before
        );
        self.helper.as_mut().unwrap().socket = Some(offered);
        let fd = original.into_raw_fd();
        // SAFETY: End this unique helper socket descriptor ONCE; the offered
        // duplicate independently retains the same real connection identity.
        if unsafe { libc::close(fd) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: Immediate observation before any diagnostic file can reuse it.
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
        let mut state = self.state.borrow_mut();
        state.helper_original_closed += 1;
        state.helper_cookie_before = before;
        Ok(1)
    }
    fn fd(&mut self, _: u8) -> Result<i32, Self::Error> {
        self.calls.push("fd");
        let fd = self
            .helper
            .as_mut()
            .unwrap()
            .socket
            .take()
            .unwrap()
            .into_raw_fd();
        self.state.borrow_mut().helper_offer_fd = Some(fd);
        Ok(fd)
    }
    fn node(&mut self, _: u8) -> Result<u32, Self::Error> {
        self.calls.push("node");
        Ok(self.node)
    }
    fn destroy(&mut self, _: u8) -> Result<(), Self::Error> {
        self.calls.push("destroy");
        drop(self.helper.take().unwrap());
        Ok(())
    }
}
struct Provider {
    outcome: u8,
    fd: i32,
    state: Rc<RefCell<Stats>>,
}
struct Input {
    outcome: u8,
    fd: i32,
    state: Rc<RefCell<Stats>>,
    owned: Option<[UnixStream; 2]>,
}
impl<C> InputProvider<C, managed::Error> for Provider {
    type Input = Input;
    fn create(
        &mut self,
        config: &manager::Config,
        client: Option<&C>,
    ) -> Result<Input, managed::Error> {
        assert_eq!(
            (config.method, config.input_method, config.pipewire_fd),
            (manager::PIPEWIRE, 2, self.fd)
        );
        assert_eq!((config.uinput_width, config.uinput_height), (17, 19));
        assert_eq!(config.uinput_path.as_ref(), b"/capture-only-host");
        assert!(config.use_win32_vk_code && client.is_none());
        let (first, second) = UnixStream::pair().map_err(|error| managed::Error::HostInput {
            operation: "create",
            code: i64::from(error.raw_os_error().unwrap_or(-1)),
        })?;
        self.state.borrow_mut().created += 1;
        Ok(Input {
            outcome: self.outcome,
            fd: self.fd,
            state: self.state.clone(),
            owned: Some([first, second]),
        })
    }
}
impl InputOwner for Input {
    type Error = managed::Error;
    fn init(&mut self) -> Result<bool, managed::Error> {
        self.state.borrow_mut().initialized += 1;
        match self.outcome {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(managed::Error::HostInput {
                operation: "init",
                code: 97,
            }),
        }
    }
    fn inactive(&mut self) {
        self.state.borrow_mut().inactive += 1;
    }
    fn features(&self) -> u64 {
        u64::MAX
    }
}
impl Drop for Input {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        // SAFETY: Observation of the exact offered fd, after native Backend
        // destruction and before opening any diagnostic fd. No close/retry.
        let status = unsafe { libc::fcntl(self.fd, libc::F_GETFD) };
        let code = if status < 0 {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
        } else {
            0
        };
        if status < 0 && code == libc::EBADF {
            state.native_closed_before_input += 1;
        } else {
            state.capture_observation_error = code;
            state.errors += 1;
        }
        for (index, stream) in self.owned.take().unwrap().into_iter().enumerate() {
            let fd = stream.into_raw_fd();
            // SAFETY: Detach each unique owned host socket and close ONCE.
            // Failure is retained as scalar evidence; never retry close.
            if unsafe { libc::close(fd) } == 0 {
                state.host_fds_closed += 1;
            } else {
                state.close_errors[index] =
                    std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                state.errors += 1;
            }
        }
        state.destroyed += 1;
    }
}

#[test]
#[ignore = "requires owned native PipeWire source/routing and real OpenCV conversion"]
fn retains_real_capture_after_host_init_false_or_error_and_closes_native_before_host_owner() {
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    assert_eq!(
        Path::new(&env::var("PIPEWIRE_RUNTIME_DIR").unwrap())
            .join(env::var("PIPEWIRE_REMOTE").unwrap()),
        Path::new(&socket)
    );
    let node: u32 = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse()
        .unwrap();
    let directory =
        std::path::PathBuf::from(env::var_os("MFS_P1_PIPEWIRE_HOST_ARTIFACTS").unwrap());
    fs::create_dir(&directory).unwrap();
    let adapter = Adapter::open();
    let mut cache = RawCache::default();
    let warm = [1_u8; 12];
    // SAFETY: Initialized two-row RGB control, isolated before native baseline.
    unsafe {
        cache.copy_native(
            warm.as_ptr(),
            Layout::checked(2, 2, Format::Rgb, 12, None).unwrap(),
        )
    }
    .unwrap();
    drop(
        adapter
            .convert(&cache.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    cache.clear();
    let baseline_fds = count("/proc/self/fd");
    let baseline = task_ids();
    let task_text = |ids: &[std::ffi::OsString]| {
        ids.iter()
            .map(|id| format!("{}\n", id.to_str().unwrap()))
            .collect::<String>()
    };
    fs::write(
        directory.join("baseline-task-ids.txt"),
        task_text(&baseline),
    )
    .unwrap();
    let mut converter = Convert {
        adapter: &adapter,
        raw: Vec::with_capacity(60),
    };
    let mut held = Vec::with_capacity(60);
    let mut identities = Vec::with_capacity(60);
    let mut records = String::new();
    let mut api_records = String::new();
    let mut outcomes = [0_u32; 3];
    for cycle in 0..20 {
        let outcome = u8::try_from(cycle % 3).unwrap();
        let state = Rc::new(RefCell::new(Stats::default()));
        let mut offer = Offer {
            socket: &socket,
            node,
            helper: None,
            state: state.clone(),
            calls: Vec::with_capacity(5),
        };
        let config =
            manager::Config::from_legacy_kwin(Some(b"/capture-only-host"), 17, 19, 255, &mut offer)
                .unwrap()
                .unwrap();
        assert_eq!(offer.calls, ["create", "open", "fd", "node", "destroy"]);
        fs::write(
            directory.join(format!("api-{cycle}.events")),
            offer.calls.join(","),
        )
        .unwrap();
        assert!(offer.helper.is_none());
        drop(offer);
        assert_eq!(state.borrow().helper_ended, 1);
        let fd = config.pipewire_fd;
        // SAFETY: Live unique host offer; read-only observation before transfer.
        let offer_open = unsafe { libc::fcntl(fd, libc::F_GETFD) } >= 0;
        assert!(offer_open);
        let metadata = config.info().unwrap();
        struct Balance(u32);
        impl Drop for Balance {
            fn drop(&mut self) {
                for _ in 0..self.0 {
                    // SAFETY: Fixture-only balancing after manager/native/host teardown.
                    unsafe { sys::pw_deinit() };
                }
            }
        }
        let mut balance = Balance(0);
        let mut manager = manager::Manager::<managed::HostedFactory<Provider>>::new(config);
        let mut output = None;
        assert!(
            !manager
                .capture_native(&mut output, &mut converter)
                .result
                .unwrap()
        );
        assert!(output.is_none() && !manager.connected());
        let mut uuid = Vec::new();
        manager.request_uuid(&mut uuid).unwrap();
        assert_eq!(uuid, format!("{fd}_{node}_/capture-only-host").as_bytes());
        fs::write(directory.join(format!("api-{cycle}.uuid")), &uuid).unwrap();
        fs::write(directory.join(format!("api-{cycle}.info")), &metadata).unwrap();
        assert_eq!(manager.get_features(), 0);
        // SAFETY: One freshly detached real socket, no independent owner or
        // reuse. Host script owns distinct socketpair fds and cannot close it.
        let connected: managed::HostConnection = unsafe {
            manager.connect_source_host_native(Provider {
                outcome,
                fd,
                state: state.clone(),
            })
        };
        if connected.close.is_some() {
            balance.0 = 1;
        }
        eprintln!("native host cycle={cycle} outcome={outcome} attempt={connected:?}");
        let (result, error_code) = match &connected.result {
            Ok(true) => (1, 0),
            Ok(false) => (0, 0),
            Err(managed::Error::HostInput {
                operation: "init",
                code,
            }) => (2, *code),
            Err(other) => panic!("unexpected host/native error: {other:?}"),
        };
        assert_eq!(
            (result, error_code),
            match outcome {
                0 => (1, 0),
                1 => (0, 0),
                _ => (2, 97),
            }
        );
        assert!(connected.failure.is_none());
        assert_eq!(manager.connected(), outcome == 0);
        outcomes[usize::from(outcome)] += 1;
        assert_eq!(manager.get_features(), 3);
        let close = connected.close.unwrap();
        for call in 0..3 {
            if call == 1 {
                assert!(manager.inactive());
            }
            assert!(
                manager
                    .capture_native(&mut output, &mut converter)
                    .result
                    .unwrap()
            );
            let image = output.take().unwrap();
            let raw = converter.raw.last().unwrap();
            let stem = format!("host-{cycle}-call-{call}");
            fs::write(directory.join(format!("{stem}.rgb")), raw.pixels()).unwrap();
            fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
            let serial = known_bgr(&image);
            assert_eq!(serial, marker(raw));
            identities.push(serial);
            held.push((image, serial));
        }
        assert_eq!(manager.get_info().unwrap(), metadata);
        let features_after = manager.get_features();
        assert_eq!(features_after, 3);
        let live = live_tasks();
        drop(manager);
        let closed = close.take().unwrap();
        eprintln!(
            "native host close cycle={cycle} native={closed:?} host={:?}",
            *state.borrow()
        );
        check_close(closed, 3);
        drop(balance);
        // SAFETY: Observe before resource diagnostics can allocate a new fd.
        let socket_closed = unsafe { libc::fcntl(fd, libc::F_GETFD) } < 0
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF);
        let final_fds = count("/proc/self/fd");
        assert_resources(&directory, cycle, baseline_fds, &baseline, &live);
        let snapshot = state.borrow();
        use std::fmt::Write;
        writeln!(&mut records,"{cycle}\t{outcome}\t{result}\t{error_code}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{baseline_fds}\t{final_fds}\t{}\t1\t{}\t{}\t{}\t{features_after}\t{}\t{}",
            snapshot.created,snapshot.initialized,snapshot.inactive,snapshot.destroyed,snapshot.native_closed_before_input,snapshot.host_fds_closed,snapshot.errors,u8::from(socket_closed),baseline.len(),snapshot.capture_observation_error,snapshot.close_errors[0],snapshot.close_errors[1],snapshot.helper_ended,u8::from(offer_open)).unwrap();
        fs::write(directory.join("rust-cases.tsv"), &records).unwrap();
        writeln!(
            &mut api_records,
            "{cycle}\t{fd}\t{node}\t0\t0\t{}\t{}\t{}\t5",
            snapshot.helper_original_closed,
            snapshot.helper_cookie_before,
            snapshot.helper_cookie_after
        )
        .unwrap();
        fs::write(directory.join("rust-api.tsv"), &api_records).unwrap();
        let closed_tasks = task_ids();
        fs::write(
            directory.join(format!("closed-{cycle}-task-ids.txt")),
            task_text(&closed_tasks),
        )
        .unwrap();
        assert_eq!(closed_tasks, baseline);
        assert_eq!(
            (
                snapshot.created,
                snapshot.initialized,
                snapshot.inactive,
                snapshot.destroyed
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(
            (
                snapshot.native_closed_before_input,
                snapshot.host_fds_closed,
                snapshot.errors
            ),
            (1, 2, 0)
        );
        assert!(socket_closed);
        assert_eq!(snapshot.capture_observation_error, 0);
        assert_eq!(snapshot.close_errors, [0, 0]);
        assert_eq!(snapshot.helper_original_closed, 1);
        assert!(
            snapshot.helper_cookie_before != 0
                && snapshot.helper_cookie_before == snapshot.helper_cookie_after
        );
        for (image, serial) in &held {
            assert_eq!(known_bgr(image), *serial);
        }
    }
    assert_eq!(outcomes, [7, 7, 6]);
    assert_eq!(converter.raw.len(), 60);
    for (index, ((image, serial), raw)) in held.iter().zip(&converter.raw).enumerate() {
        assert_eq!(known_bgr(image), *serial);
        assert_eq!(marker(raw), *serial);
        fs::write(
            directory.join(format!("held-{index}.rust.bgr")),
            image.pixels(),
        )
        .unwrap();
        fs::write(
            directory.join(format!("held-{index}.rust.rgb")),
            raw.pixels(),
        )
        .unwrap();
    }
    std::thread::spawn(move || {
        for (raw, serial) in converter.raw.into_iter().zip(identities) {
            assert_eq!(marker(&raw), serial);
        }
    })
    .join()
    .unwrap();
    drop(held);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust PipeWire host manager objects=20 init_true=7 init_false=7 init_error=6 captures=60 retained_raw=60 retained_bgr=60 helper_ended_before_connect=20 native_closed_before_host=20 host_fds_closed=40 input_actions=0 passed"
    );
}
