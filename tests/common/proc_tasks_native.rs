// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Isolated causal procfs control. No capture backend, host authorization or
//! capture retry; only two owned gated standard threads and diagnostic FDs.
use super::{
    native::{close_file, probe_absence, read_opened, read_task},
    *,
};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

struct Worker {
    id: OsString,
    release: Option<mpsc::SyncSender<()>>,
    handle: Option<JoinHandle<()>>,
}
impl Worker {
    fn new() -> Self {
        let (ready, receive) = mpsc::sync_channel(0);
        let (release, gate) = mpsc::sync_channel(0);
        let handle = thread::Builder::new()
            .name("MaaScreencap".into())
            .spawn(move || {
                let path = fs::read_link("/proc/thread-self").unwrap();
                ready.send(path.file_name().unwrap().to_owned()).unwrap();
                // Dropping the controller also releases this worker during unwind.
                let _ = gate.recv();
            })
            .unwrap();
        let mut worker = Self {
            id: OsString::new(),
            release: Some(release),
            handle: Some(handle),
        };
        worker.id = receive.recv().unwrap();
        worker
    }
    fn finish(&mut self) -> thread::Result<()> {
        drop(self.release.take());
        self.handle.take().map_or(Ok(()), JoinHandle::join)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn fds() -> usize {
    fs::read_dir("/proc/self/fd").unwrap().count()
}
fn limit() -> libc::rlimit {
    let mut value = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: Exact libc declaration and writable initialized local storage.
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut value) },
        0
    );
    value
}
struct Limited {
    before: libc::rlimit,
    changed: bool,
    files: Vec<File>,
}
impl Limited {
    fn exhaust() -> Self {
        let mut value = Self {
            before: limit(),
            changed: false,
            files: Vec::with_capacity(128),
        };
        let selected = libc::rlimit {
            rlim_cur: value.before.rlim_cur.min(64),
            rlim_max: value.before.rlim_max,
        };
        assert!(selected.rlim_cur >= 32 && u64::try_from(fds() + 8).unwrap() < selected.rlim_cur);
        // SAFETY: Isolated exact-test process; preserve hard limit and retain
        // the original full structure for one restoration before exit/unwind.
        assert_eq!(
            unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &selected) },
            0
        );
        value.changed = true;
        loop {
            match File::open("/dev/null") {
                Ok(file) => {
                    value.files.push(file);
                    assert!(value.files.len() < 128);
                }
                Err(error) => {
                    assert_eq!(error.raw_os_error(), Some(libc::EMFILE));
                    break;
                }
            }
        }
        value
    }
    fn restore(&mut self) -> (usize, Option<i32>) {
        let mut errors = 0;
        for file in self.files.drain(..) {
            if close_file(file).is_some() {
                errors += 1;
            }
        }
        let error = if self.changed {
            self.changed = false;
            // SAFETY: Restore this process's unchanged saved hard/soft limits.
            if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &self.before) } == 0 {
                None
            } else {
                Some(std::io::Error::last_os_error().raw_os_error().unwrap_or(-1))
            }
        } else {
            None
        };
        (errors, error)
    }
}
impl Drop for Limited {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
fn select_retired_state(id: &OsStr) {
    // Causal fixture only: select a fully retired known joined thread, as in
    // the independent C++ control. The capture gate's 100ms bound is unchanged.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let probe = probe_absence(id);
        if proved_absent(&probe) {
            return;
        }
        assert!(
            probe.stat_opened && probe.close_error.is_none(),
            "unexpected native retirement selection error: {probe:?}"
        );
        assert!(
            Instant::now() < deadline,
            "joined causal fixture did not retire"
        );
        thread::yield_now();
    }
}

#[test]
#[ignore = "explicit native Linux causal ESRCH/live-path/EMFILE observer control"]
fn validates_joined_read_esrch_without_accepting_live_or_unobservable_paths() {
    let directory = PathBuf::from(std::env::var_os("MFS_P1_RUST_TASK_OBSERVER_ARTIFACTS").unwrap());
    fs::create_dir(&directory).unwrap();
    let observations = directory.join("observations.txt");
    let mut evidence = String::from("fixture_started=1\nnative_capture=0\n");
    fs::write(&observations, &evidence).unwrap();
    let baseline_fds = fds();
    let baseline_tasks = task_ids();
    let (failure, absent, live_probe, limited_probe, before);
    {
        let mut retired = Worker::new();
        let mut live = Worker::new();
        before = read_task(&retired.id).unwrap().unwrap();
        let live_before = read_task(&live.id).unwrap().unwrap();
        assert!(known_live_source(&before) && known_live_source(&live_before));
        let mut held = File::open(
            PathBuf::from("/proc/self/task")
                .join(&retired.id)
                .join("stat"),
        )
        .unwrap();
        retired.finish().unwrap();
        select_retired_state(&retired.id);
        // The strict reader still returns the REAL read failure from a file
        // opened while the gated task was alive. It never silently returns None.
        failure = read_opened(&retired.id, &mut held).unwrap_err();
        evidence.push_str(&format!("known_before={before:?}\nopened_while_live=1\njoined_before_read=1\nstrict_read_failure={failure:?}\n"));
        fs::write(&observations, &evidence).unwrap();
        assert!(exact_retired_read(&before, &failure));
        absent = probe_absence(&retired.id);
        evidence.push_str(&format!("fresh_absence={absent:?}\n"));
        fs::write(&observations, &evidence).unwrap();
        assert!(accepted_retired_read(Some(&before), &failure, &absent));
        let synthetic = Fault {
            id: live_before.id.clone(),
            ..failure.clone()
        };
        live_probe = probe_absence(&live_before.id);
        evidence.push_str(&format!("live_path={live_probe:?}\n"));
        fs::write(&observations, &evidence).unwrap();
        assert!(live_probe.stat_opened && live_probe.close_error.is_none());
        assert!(!accepted_retired_read(
            Some(&live_before),
            &synthetic,
            &live_probe
        ));
        assert!(!only_retiring_tasks(
            std::slice::from_ref(&live_before.id),
            &[],
            std::slice::from_ref(&live_before)
        ));
        assert!(!only_retiring_tasks(
            std::slice::from_ref(&before.id),
            &[],
            &[]
        ));
        assert!(!only_retiring_tasks(
            &[],
            &baseline_tasks,
            std::slice::from_ref(&before)
        ));
        {
            let mut exhausted = Limited::exhaust();
            limited_probe = probe_absence(&retired.id);
            eprintln!("task_observer_real_fd_exhaustion={limited_probe:?}");
            assert_eq!(limited_probe.stat_error, Some(libc::EMFILE));
            assert!(!accepted_retired_read(
                Some(&before),
                &failure,
                &limited_probe
            ));
            let expected = (exhausted.before.rlim_cur, exhausted.before.rlim_max);
            assert_eq!(exhausted.restore(), (0, None));
            let restored = limit();
            assert_eq!((restored.rlim_cur, restored.rlim_max), expected);
            evidence.push_str(&format!(
                "real_EMFILE={limited_probe:?}\nfd_limit_restored=1\n"
            ));
            fs::write(&observations, &evidence).unwrap();
        }
        assert_eq!(close_file(held), None);
        live.finish().unwrap();
        select_retired_state(&live.id);
    }
    assert_eq!(fds(), baseline_fds);
    assert_eq!(task_ids(), baseline_tasks);
    evidence.push_str(&format!(
        "baseline_fds={baseline_fds}\nfinal_fds={}\nexact_task_ids_restored=1\nfixture_passed=1\n",
        fds()
    ));
    fs::write(&observations, &evidence).unwrap();
    println!(
        "native Rust task observer exact_read_ESRCH=1 strict_read_rejected=1 known_joined_source=1 fresh_stat_and_directory_absent=1 real_live_path_refused=1 real_EMFILE_refused=1 fd_limit_restored=1 exact_fd_task_owners_closed=1 native_capture=0 passed"
    );
}
