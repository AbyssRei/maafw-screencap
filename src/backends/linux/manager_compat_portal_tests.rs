// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real socket residuals at the controlled host boundary, before any manager.
use super::super::{Config, Error, Portal};
use crate::backends::task_observer::task_ids;
use std::{
    fs,
    io::Write,
    os::fd::{AsRawFd, IntoRawFd},
    os::unix::net::UnixStream,
    path::PathBuf,
};

fn cookie(fd: i32) -> u64 {
    let mut value = 0_u64;
    let mut size = libc::socklen_t::try_from(std::mem::size_of_val(&value)).unwrap();
    // SAFETY: Observe a live host-owned socket; writable scalar storage matches
    // SO_COOKIE. Neither this call nor the caller transfers or consumes the fd.
    assert_eq!(
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_COOKIE,
                std::ptr::from_mut(&mut value).cast(),
                &mut size,
            )
        },
        0
    );
    assert_eq!(
        usize::try_from(size).unwrap(),
        std::mem::size_of_val(&value)
    );
    assert_ne!(value, 0);
    value
}
fn fds() -> usize {
    fs::read_dir("/proc/self/fd").unwrap().count()
}
struct Helper {
    socket: Option<UnixStream>,
}
struct Host<'a> {
    socket: &'a str,
    stage: u32,
    helper: Option<Helper>,
    events: Vec<&'static str>,
    before: u64,
    original_closed: u32,
}
impl Portal for Host<'_> {
    type Helper = u8;
    type Error = u32;
    fn create(&mut self) -> Result<u8, u32> {
        self.events.push("create");
        assert!(self.helper.is_none());
        self.helper = Some(Helper { socket: None });
        Ok(0)
    }
    fn open(&mut self, _: u8) -> Result<u8, u32> {
        self.events.push("open");
        let original = UnixStream::connect(self.socket).unwrap();
        self.before = cookie(original.as_raw_fd());
        let offered = original.try_clone().unwrap();
        assert_eq!(cookie(offered.as_raw_fd()), self.before);
        self.helper.as_mut().unwrap().socket = Some(offered);
        let fd = original.into_raw_fd();
        // SAFETY: End this unique original host descriptor once, while the
        // independently owned duplicate retains the connection's identity.
        assert_eq!(unsafe { libc::close(fd) }, 0);
        assert_closed(fd);
        self.original_closed += 1;
        match self.stage {
            0 => Ok(0),
            1 => Err(91),
            _ => Ok(1),
        }
    }
    fn fd(&mut self, _: u8) -> Result<i32, u32> {
        self.events.push("fd");
        if self.stage == 2 {
            return Err(92);
        }
        // All cases fail before manager creation/native consumption. This host
        // ledger retains the offer for explicit post-error fixture reclaim.
        Ok(self
            .helper
            .as_ref()
            .unwrap()
            .socket
            .as_ref()
            .unwrap()
            .as_raw_fd())
    }
    fn node(&mut self, _: u8) -> Result<u32, u32> {
        self.events.push("node");
        if self.stage == 3 { Err(93) } else { Ok(0) }
    }
    fn destroy(&mut self, _: u8) -> Result<(), u32> {
        self.events.push("destroy");
        assert_eq!(self.stage, 4);
        Err(94) // Host failure before any helper owner is destroyed.
    }
}
fn assert_closed(fd: i32) {
    // SAFETY: Immediate scalar observation, before any operation may reuse the
    // just-closed number. This never closes or retries an unknown descriptor.
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
}
#[test]
#[ignore = "requires native Linux private socket and exact original KWin failure records"]
fn retains_host_socket_after_each_portal_failure_before_manager_construction() {
    let root = PathBuf::from(std::env::var_os("MFS_P1_KWIN_PORTAL_FAILURE_ROOT").unwrap());
    let directory = root.join("rust");
    fs::create_dir(&directory).unwrap();
    let socket = std::env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    let mut rows = fs::File::create_new(directory.join("cases.tsv")).unwrap();
    writeln!(rows,"index\tstage\terror\thelper_live\toriginal_closed\thelper_ended\tfd\tcookie_before\tcookie_after\tfd_calls\tnode_calls\tdestroy_calls\tcreated\tfds_before\tfds_residual\tfds_final\ttasks\tcaller\tfixture_closed\tfd_closed\thelper_final\tnative_constructed\tevents").unwrap();
    let baseline = task_ids();
    let save_tasks = |name: String, ids: &[std::ffi::OsString]| {
        let text = ids
            .iter()
            .map(|id| format!("{}\n", id.to_str().unwrap()))
            .collect::<String>();
        fs::write(directory.join(name), text).unwrap();
    };
    save_tasks("baseline-task-ids.txt".to_owned(), &baseline);
    let baseline_fds = fds();
    // SAFETY: Calling thread scalar query; no native allocation or ownership.
    let caller = unsafe { libc::gettid() };
    for index in 0..100 {
        let stage = index % 5;
        let mut host = Host {
            socket: &socket,
            stage,
            helper: None,
            events: Vec::new(),
            before: 0,
            original_closed: 0,
        };
        let code =
            match Config::from_legacy_kwin(Some(b"/capture-only-host"), 17, 19, 255, &mut host) {
                Ok(None) if stage == 0 => 0,
                Err(Error::Host(code)) if stage != 0 => code,
                other => panic!("unexpected Portal/config result: {other:?}"),
            };
        assert_eq!(code, if stage == 0 { 0 } else { 90 + stage });
        let order = ["create", "open", "fd", "node", "destroy"];
        let count = if stage < 2 {
            2
        } else {
            usize::try_from(stage + 1).unwrap()
        };
        assert_eq!(host.events, order[..count]);
        assert_eq!(host.original_closed, 1);
        let fd = host
            .helper
            .as_ref()
            .unwrap()
            .socket
            .as_ref()
            .unwrap()
            .as_raw_fd();
        let after = cookie(fd);
        assert_eq!(after, host.before);
        let residual_fds = fds();
        assert_eq!(residual_fds, baseline_fds + 1);
        assert_eq!(task_ids(), baseline);
        // Source residual is recorded before explicit fixture reclaim. The
        // migrated constructor neither closes the offer nor destroys helper.
        let reclaim = host
            .helper
            .as_mut()
            .unwrap()
            .socket
            .take()
            .unwrap()
            .into_raw_fd();
        assert_eq!(reclaim, fd);
        // SAFETY: Same proven uniquely owned socket, no native consumer exists.
        assert_eq!(unsafe { libc::close(reclaim) }, 0);
        assert_closed(reclaim);
        drop(host.helper.take().unwrap());
        let final_fds = fds();
        let final_tasks = task_ids();
        save_tasks(format!("closed-{index}-task-ids.txt"), &final_tasks);
        writeln!(rows,"{index}\t{stage}\t{code}\t1\t1\t0\t{fd}\t{}\t{after}\t{}\t{}\t{}\t0\t{baseline_fds}\t{residual_fds}\t{final_fds}\t{}\t{caller}\t1\t1\t0\t0\t{}",host.before,u32::from(stage>=2),u32::from(stage>=3),u32::from(stage==4),baseline.len(),host.events.join(",")).unwrap();
        rows.flush().unwrap();
        assert_eq!(final_fds, baseline_fds);
        assert_eq!(final_tasks, baseline);
    }
    println!(
        "Rust KWin Portal failures cases=100 stages=5 residual_sockets=100 fixture_reclaims=100 manager_created=0 native_capture=0 full_P1_acceptance=0 passed"
    );
}
