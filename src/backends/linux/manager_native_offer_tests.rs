// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native pre-init host-construction failure; no daemon, authorization or capture.
use super::super::{
    Manager,
    host::{InputOwner, InputProvider},
};
use super::*;
use crate::backends::task_observer::task_ids;
use crate::platform::linux::pipewire::sys;
use std::{
    cell::Cell,
    fmt::Write,
    fs,
    os::fd::{AsRawFd, IntoRawFd},
    os::unix::net::UnixStream,
    rc::Rc,
};

fn cookie(fd: i32) -> Option<u64> {
    let mut value = 0_u64;
    let mut length = libc::socklen_t::try_from(std::mem::size_of_val(&value)).unwrap();
    // SAFETY: Scalar observation only; initialized, correctly sized output.
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_COOKIE,
            std::ptr::from_mut(&mut value).cast(),
            &mut length,
        )
    };
    (result == 0 && usize::try_from(length).unwrap() == std::mem::size_of_val(&value))
        .then_some(value)
}
struct Residual {
    fd: i32,
    cookie: u64,
}
impl Residual {
    fn reclaim(&mut self) {
        assert_eq!(cookie(self.fd), Some(self.cookie));
        let fd = std::mem::replace(&mut self.fd, -1);
        // SAFETY: Native teardown ended without any init/consumption. The same
        // unique socket identity was just proved; fixture reclaim closes ONCE.
        assert_eq!(unsafe { libc::close(fd) }, 0);
    }
}
impl Drop for Residual {
    fn drop(&mut self) {
        if self.fd >= 0 && cookie(self.fd) == Some(self.cookie) {
            let fd = std::mem::replace(&mut self.fd, -1);
            // SAFETY: Failure-only fixture reclaim after all native owners.
            // Never close a reused/unproved number or retry a failed close.
            let _ = unsafe { libc::close(fd) };
        }
    }
}
struct Host(Rc<Cell<u32>>);
struct Input;
impl InputOwner for Input {
    type Error = Error;
    fn init(&mut self) -> Result<bool, Error> {
        panic!("failed constructor created input")
    }
    fn inactive(&mut self) {
        panic!("failed constructor created input")
    }
    fn features(&self) -> u64 {
        panic!("failed constructor created input")
    }
}
impl<C> InputProvider<C, Error> for Host {
    type Input = Input;
    fn create(&mut self, config: &Config, client: Option<&C>) -> Result<Input, Error> {
        assert_eq!((config.method, config.input_method), (PIPEWIRE, 2));
        assert_eq!((config.uinput_width, config.uinput_height), (17, 19));
        assert_eq!(config.uinput_path.as_ref(), b"/capture-only-host");
        assert!(client.is_none() && config.pipewire_fd >= 0);
        self.0.set(self.0.get() + 1);
        Err(Error::HostInput {
            operation: "create",
            code: 98,
        })
    }
}
struct NoConversion;
impl Converter for NoConversion {
    type Image = u8;
    type Error = ();
    #[cfg(feature = "linux-wlr")]
    fn wlr(&mut self, _: wlr::Sample) -> Result<u8, ()> {
        panic!("unexpected Wlr conversion")
    }
    fn pipewire(&mut self, _: Option<RawFrame>, _: pipewire::Conversion) -> Result<Option<u8>, ()> {
        panic!("uninitialized capture converted")
    }
    fn pipewire_reuse(&mut self, _: Option<RawFrame>) -> Option<u8> {
        panic!("uninitialized capture reused")
    }
    fn empty(&mut self) -> Result<u8, ()> {
        panic!("uninitialized capture changed output")
    }
}
fn idle(evidence: &Evidence) {
    let Evidence::PipeWire(e) = evidence else {
        panic!("missing native owner evidence")
    };
    assert_eq!(
        (
            e.fault,
            e.queue_error,
            e.core_disconnect_error,
            e.mapping_error,
            e.unmap_error
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(
        (
            e.owned_maps,
            e.unmap_calls,
            e.dequeued,
            e.returned,
            e.copies,
            e.allocated
        ),
        (0, 0, 0, 0, 0, 0)
    );
    assert_eq!(
        (
            e.dma.borrowed,
            e.dma.starts,
            e.dma.ends,
            e.dma.start_error,
            e.dma.end_error
        ),
        (0, 0, 0, 0, 0)
    );
}
fn save_tasks(path: &std::path::Path, ids: &[std::ffi::OsString]) {
    let text = ids
        .iter()
        .map(|id| format!("{}\n", id.to_str().unwrap()))
        .collect::<String>();
    fs::write(path, text).unwrap();
}
fn fds() -> usize {
    fs::read_dir("/proc/self/fd").unwrap().count()
}

#[test]
#[ignore = "explicit isolated native host offer ownership fixture; not capture or Portal authorization"]
fn retains_the_unconsumed_offer_after_host_construction_error_until_fixture_reclaim() {
    let directory =
        std::path::PathBuf::from(std::env::var_os("MFS_P1_PIPEWIRE_OFFER_ARTIFACTS").unwrap());
    fs::create_dir(&directory).unwrap();
    drop(Native::new().unwrap());
    // SAFETY: Fixture-only balance AFTER its warm native owner has ended.
    unsafe { sys::pw_deinit() };
    let baseline_fds = fds();
    let baseline = task_ids();
    save_tasks(&directory.join("baseline-task-ids.txt"), &baseline);
    let mut rows = String::new();
    for cycle in 0..20 {
        let (helper, peer) = UnixStream::pair().unwrap();
        let original_cookie = cookie(helper.as_raw_fd()).unwrap();
        assert_ne!(original_cookie, 0);
        // SAFETY: Duplicate this owned live socket, not a guessed descriptor.
        let fd = unsafe { libc::dup(helper.as_raw_fd()) };
        assert!(fd >= 0 && fd != helper.as_raw_fd());
        let mut offer = Residual {
            fd,
            cookie: original_cookie,
        };
        let helper_fd = helper.into_raw_fd();
        // SAFETY: Sole helper owner is detached, then explicitly closed ONCE.
        assert_eq!(unsafe { libc::close(helper_fd) }, 0);
        // SAFETY: Observe before any new descriptor can reuse the number.
        assert_eq!(unsafe { libc::fcntl(helper_fd, libc::F_GETFD) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
        let before = cookie(fd).unwrap();
        assert_eq!(before, original_cookie);
        struct Balance(Cell<bool>);
        impl Drop for Balance {
            fn drop(&mut self) {
                // SAFETY: Declared before manager, so native destruction ends
                // first, including unwinding. This is not product deinit.
                if self.0.get() {
                    unsafe { sys::pw_deinit() };
                }
            }
        }
        let balance = Balance(Cell::new(false));
        let creates = Rc::new(Cell::new(0));
        let mut manager = Manager::<HostedFactory<Host>>::new(Config {
            method: PIPEWIRE,
            input_method: 2,
            pipewire_fd: fd,
            pipewire_node: 0,
            uinput_width: 17,
            uinput_height: 19,
            uinput_path: std::borrow::Cow::Borrowed(b"/capture-only-host"),
            ..Config::default()
        });
        // SAFETY: Fresh unique detached offer; provider cannot close/reuse it.
        // Its controlled construction error PRECEDES native initialization.
        let connection = unsafe { manager.connect_source_host_native(Host(creates.clone())) };
        balance.0.set(connection.close.is_some());
        assert!(matches!(
            connection.result,
            Err(Error::HostInput {
                operation: "create",
                code: 98
            })
        ));
        assert!(
            connection.failure.is_none() && manager.capture.is_some() && manager.input.is_none()
        );
        assert!(!manager.connected());
        idle(&connection.evidence);
        let after_error = cookie(fd).unwrap();
        let mut output = 99;
        let attempt = manager.capture_native(&mut output, &mut NoConversion);
        let refused = matches!(
            attempt.result,
            Err(CaptureFailure::PipeWire(pipewire::Error::Disconnected))
        );
        idle(&attempt.evidence);
        let preserved = output == 99;
        assert!(manager.inactive());
        let features = manager.get_features();
        drop(manager);
        let close = connection.close.unwrap().take().unwrap();
        eprintln!("host offer cycle={cycle} primary=create98 closed={close:?}");
        idle(&close);
        drop(balance);
        let after_drop = cookie(fd).unwrap();
        // Residual input is observed BEFORE fixture reclaim; no cleanup success
        // is attributed to the product for retaining this unconsumed descriptor.
        offer.reclaim();
        drop(peer);
        let final_fds = fds();
        let final_tasks = task_ids();
        save_tasks(
            &directory.join(format!("closed-{cycle}-task-ids.txt")),
            &final_tasks,
        );
        writeln!(&mut rows,"{cycle}\t98\t{}\t0\t0\t{features}\t1\t{before}\t{after_error}\t{after_drop}\t{}\t{}\t1\t1\t{baseline_fds}\t{final_fds}\t{}\t{}\t1",
            creates.get(),u8::from(refused),u8::from(preserved),baseline.len(),final_tasks.len()).unwrap();
        fs::write(directory.join("rust-cases.tsv"), &rows).unwrap();
        assert_eq!(
            (before, after_error, after_drop),
            (original_cookie, original_cookie, original_cookie)
        );
        assert_eq!(
            (creates.get(), features, refused, preserved),
            (1, 0, true, true)
        );
        assert_eq!(final_fds, baseline_fds);
        assert_eq!(final_tasks, baseline);
    }
    eprintln!(
        "native Rust PipeWire host offer cycles=20 create_errors=20 native_init=0 retained_identity=20 unconsumed_after_destroy=20 fixture_reclaims=20 capture=0 passed"
    );
}
