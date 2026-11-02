// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Owned test-process kernel limits, never capture-library resource policy.
use super::protocol::cases;
use super::*;
use std::{fs, io::Write, os::unix::fs::MetadataExt, ptr};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    Memfd,
    Truncate,
}
impl Fault {
    fn key(self) -> &'static str {
        match self {
            Self::Memfd => "MEMFD",
            Self::Truncate => "FTRUNCATE",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Memfd => "memfd",
            Self::Truncate => "ftruncate",
        }
    }
    fn error(self) -> i32 {
        match self {
            Self::Memfd => libc::EMFILE,
            Self::Truncate => libc::EFBIG,
        }
    }
    fn limit(self) -> libc::rlimit {
        // SAFETY: Zeroed POD output and a valid resource selector.
        let mut output = unsafe { std::mem::zeroed() };
        let kind = match self {
            Self::Memfd => libc::RLIMIT_NOFILE,
            Self::Truncate => libc::RLIMIT_FSIZE,
        };
        // SAFETY: Writable native structure, checked immediately.
        assert_eq!(unsafe { libc::getrlimit(kind, &mut output) }, 0);
        output
    }
    fn set(self, value: &libc::rlimit) -> bool {
        let kind = match self {
            Self::Memfd => libc::RLIMIT_NOFILE,
            Self::Truncate => libc::RLIMIT_FSIZE,
        };
        // SAFETY: Test process only; the saved hard limit never changes.
        unsafe { libc::setrlimit(kind, value) == 0 }
    }
}
fn mask() -> libc::sigset_t {
    // SAFETY: Zeroed POD writable output; null set queries this thread only.
    let mut output = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, ptr::null(), &mut output) },
        0
    );
    output
}
fn action() -> libc::sigaction {
    // SAFETY: Zeroed POD writable output; no disposition is installed.
    let mut output = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::sigaction(libc::SIGXFSZ, ptr::null(), &mut output) },
        0
    );
    output
}
fn member(set: &libc::sigset_t, signal: i32) -> i32 {
    // SAFETY: Initialized set and valid Linux signal number.
    unsafe { libc::sigismember(set, signal) }
}
fn same_mask(a: &libc::sigset_t, b: &libc::sigset_t) -> bool {
    (1..=libc::SIGRTMAX()).all(|signal| member(a, signal) == member(b, signal))
}
fn pending() -> i32 {
    // SAFETY: Zeroed POD writable output, no signal is consumed.
    let mut set = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { libc::sigpending(&mut set) }, 0);
    member(&set, libc::SIGXFSZ)
}
struct Limit {
    kind: Fault,
    before: libc::rlimit,
    original_mask: libc::sigset_t,
    expected_mask: libc::sigset_t,
    selected: libc::sigset_t,
    action: libc::sigaction,
    active: bool,
    masked: bool,
    may_consume_new_signal: bool,
}
struct Observation {
    after: libc::rlimit,
    mask_before: i32,
    new_pending: i32,
    consumed: i32,
}
impl Limit {
    fn new(kind: Fault) -> Self {
        let before = kind.limit();
        assert!(before.rlim_cur > 0);
        assert!(kind != Fault::Truncate || before.rlim_cur >= 268_435_456);
        let original_mask = mask();
        // SAFETY: Native sigset_t has no ownership or Rust references.
        let mut selected = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::sigemptyset(&mut selected) }, 0);
        assert_eq!(unsafe { libc::sigaddset(&mut selected, libc::SIGXFSZ) }, 0);
        let mut guard = Self {
            kind,
            before,
            original_mask,
            expected_mask: original_mask,
            selected,
            action: action(),
            active: false,
            masked: false,
            may_consume_new_signal: false,
        };
        if kind == Fault::Truncate {
            // SAFETY: Only this test thread is changed; no handler is installed.
            assert_eq!(
                unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &guard.selected, ptr::null_mut()) },
                0
            );
            guard.masked = true;
            assert_eq!(pending(), 0, "never consume a preexisting SIGXFSZ");
            guard.may_consume_new_signal = true;
            assert_eq!(
                unsafe { libc::sigaddset(&mut guard.expected_mask, libc::SIGXFSZ) },
                0
            );
        }
        let zero = libc::rlimit {
            rlim_cur: 0,
            rlim_max: guard.before.rlim_max,
        };
        assert!(kind.set(&zero));
        guard.active = true;
        guard
    }
    fn restore(&mut self) -> Observation {
        assert!(self.kind.set(&self.before));
        self.active = false;
        let after = self.kind.limit();
        assert_eq!(
            (after.rlim_cur, after.rlim_max),
            (self.before.rlim_cur, self.before.rlim_max)
        );
        assert!(same_mask(&mask(), &self.expected_mask));
        let mut new_pending = 0;
        let mut consumed = 0;
        if self.masked {
            new_pending = pending();
            assert_eq!(new_pending, 1);
            let zero = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            // SAFETY: The isolated test thread blocked XFSZ before the owned
            // failing call and observed no previous pending XFSZ. Zero timeout
            // consumes that newly pending signal, never waits or installs code.
            assert_eq!(
                unsafe { libc::sigtimedwait(&self.selected, ptr::null_mut(), &zero) },
                libc::SIGXFSZ
            );
            consumed = 1;
            assert_eq!(pending(), 0);
            assert_eq!(
                unsafe {
                    libc::pthread_sigmask(libc::SIG_SETMASK, &self.original_mask, ptr::null_mut())
                },
                0
            );
            self.masked = false;
        }
        assert!(same_mask(&mask(), &self.original_mask));
        let current = action();
        assert_eq!(
            (current.sa_sigaction, current.sa_flags),
            (self.action.sa_sigaction, self.action.sa_flags)
        );
        assert!(same_mask(&current.sa_mask, &self.action.sa_mask));
        Observation {
            after,
            mask_before: member(&self.original_mask, libc::SIGXFSZ),
            new_pending,
            consumed,
        }
    }
}
impl Drop for Limit {
    fn drop(&mut self) {
        // Unwind fallback is restricted to this owned native test process. A
        // failed restoration aborts it; it cannot contaminate another test.
        if self.active && !self.kind.set(&self.before) {
            std::process::abort();
        }
        if self.masked {
            // SAFETY: Initialized output and this test's already-blocked set.
            let mut set = unsafe { std::mem::zeroed() };
            let zero = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if unsafe { libc::sigpending(&mut set) } != 0 {
                std::process::abort();
            }
            if member(&set, libc::SIGXFSZ) == 1
                && (!self.may_consume_new_signal
                    || unsafe { libc::sigtimedwait(&self.selected, ptr::null_mut(), &zero) }
                        != libc::SIGXFSZ)
            {
                // A preexisting pending signal is never consumed. This
                // unusable owned fixture terminates instead of continuing.
                std::process::abort();
            }
            if unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.original_mask, ptr::null_mut())
            } != 0
            {
                std::process::abort();
            }
        }
    }
}
fn files() -> Vec<(u64, u64)> {
    fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            let target = fs::read_link(&path).ok()?;
            if !target
                .as_os_str()
                .as_bytes()
                .starts_with(b"/memfd:maafw-wl-buffer-")
            {
                return None;
            }
            let metadata = fs::metadata(path).unwrap();
            Some((metadata.ino(), metadata.len()))
        })
        .collect()
}
fn run(kind: Fault) {
    let key = kind.key();
    let socket = PathBuf::from(env::var_os(format!("MFS_P1_WLR_{key}_SOCKET")).unwrap());
    let directory = PathBuf::from(env::var_os(format!("MFS_P1_WLR_{key}_ARTIFACTS")).unwrap());
    let source = PathBuf::from(env::var_os(format!("MFS_P1_WLR_{key}_SOURCE")).unwrap());
    let adapter = Adapter::open();
    warm(&adapter);
    let mut rows = fs::File::create(directory.join("rust-resource.tsv")).unwrap();
    let baseline = fds();
    let baseline_tasks = tasks();
    let mut held = Vec::new();
    for cycle in 0..20 {
        let (connected, failed, recovered, inode, before, observed);
        {
            let client = Client::new().unwrap();
            assert!(client.connect(&socket).unwrap());
            let mut capture = Capture::from_client(client.into_shared().unwrap()).unwrap();
            let sentinel = Sample {
                width: 7,
                height: 5,
                native_stride: 28,
                conversion: Conversion::Bgra,
                ready_time: None,
                flags: None,
                bytes: Arc::try_new([201, 73, 17, 255].repeat(35)).unwrap(),
            };
            let mut output = adapter.convert(sentinel, 0).unwrap();
            let owner = output.pointer;
            connected = fds();
            assert!(files().is_empty());
            let calls = Cell::new(0);
            let mut limit = Limit::new(kind);
            before = limit.before;
            assert!(
                kind != Fault::Memfd
                    || before.rlim_cur > libc::rlim_t::try_from(connected).unwrap()
            );
            let result = capture.capture_with(|_| {
                calls.set(calls.get() + 1);
                Ok(())
            });
            observed = limit.restore();
            let evidence = capture.evidence();
            failed = fds();
            let constructed = capture.context.state().unwrap().buffer.is_some();
            // Keep actual failed observations before the owner/pixel assertions.
            fs::write(
                directory.join(format!("attempt-{cycle}.rust.tsv")),
                format!(
                    "{}\t{}\t{}\t{}\t{}\t1\t{}\t{}\t1\n",
                    evidence.buffer_error,
                    files().len(),
                    u8::from(constructed),
                    calls.get(),
                    observed.mask_before,
                    observed.new_pending,
                    observed.consumed
                ),
            )
            .unwrap();
            let error = result.unwrap_err();
            assert_eq!(
                (error.operation, error.code),
                ("native Wlr frame failed", -6)
            );
            assert_eq!(
                (calls.get(), evidence.fault, evidence.buffer_error),
                (0, 0, kind.error())
            );
            assert!(constructed && files().is_empty());
            assert_eq!(failed, connected);
            assert_eq!(output.pointer, owner);
            assert_eq!(output.pixels_for(7, 5), [201, 73, 17].repeat(35));
            fs::write(
                directory.join(format!("failed-{cycle}.rust.bgr")),
                output.pixels_for(7, 5),
            )
            .unwrap();
            let (raw, image) = capture
                .capture_with(|sample| {
                    let raw = sample.bytes.clone();
                    adapter.convert(sample, 0).map(|image| (raw, image))
                })
                .unwrap();
            assert_eq!(raw.as_slice(), cases::raw(0));
            assert_eq!(image.pixels_for(7, 5), cases::target(0));
            fs::write(
                directory.join(format!("case-{cycle}.rust.raw")),
                raw.as_slice(),
            )
            .unwrap();
            recovered = fds();
            let current = files();
            assert_eq!(current.len(), 1);
            assert_eq!(current[0].1, 140);
            assert_eq!(recovered, connected + 1);
            inode = current[0].0;
            output = image;
            held.push(output);
        }
        let final_fds = fds();
        assert_eq!((final_fds, tasks()), (baseline, baseline_tasks.clone()));
        assert!(files().is_empty());
        writeln!(rows,"{cycle}\t{baseline}\t{connected}\t{failed}\t{recovered}\t{final_fds}\t{}\t{}\t{}\t{}\t{}\t0\t0\t140\t{inode}\t1\t0\t{}\t1\t{}\t{}\t1",
            kind.error(), before.rlim_cur, before.rlim_max, observed.after.rlim_cur, observed.after.rlim_max,
            observed.mask_before, observed.new_pending, observed.consumed).unwrap();
        rows.flush().unwrap();
    }
    for (index, image) in held.iter().enumerate() {
        let pixels = image.pixels_for(7, 5);
        assert_eq!(pixels, cases::target(0));
        fs::write(directory.join(format!("held-{index}.rust.bgr")), pixels).unwrap();
        assert_eq!(
            pixels,
            fs::read(source.join(format!("held-{index}.source.bgr"))).unwrap()
        );
    }
    println!(
        "Rust Wlr resource kind={} cycles=20 kernel_failures=20 failed_fds_retained=0 recovery=20 held=20 limits_signals_restored=20 passed",
        kind.name()
    );
}
#[test]
#[ignore = "requires native allocation IPC and isolated RLIMIT_NOFILE zero with original source oracle"]
fn recovers_after_real_memfd_exhaustion() {
    run(Fault::Memfd);
}
#[test]
#[ignore = "requires native allocation IPC and isolated RLIMIT_FSIZE/SIGXFSZ with original source oracle"]
fn recovers_after_real_ftruncate_file_limit() {
    run(Fault::Truncate);
}
