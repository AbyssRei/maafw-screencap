// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::protocol::cases;
use super::*;
use std::{fs, io::Write, os::unix::fs::MetadataExt};

struct AddressSpaceLimit {
    before: libc::rlimit,
    vm: u64,
    selected: u64,
    active: bool,
}
impl AddressSpaceLimit {
    fn new() -> Self {
        let pages: u64 = fs::read_to_string("/proc/self/statm")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        // SAFETY: Scalar query, no pointers or ownership transfer.
        let page = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap();
        assert!(page > 0);
        let vm = pages.checked_mul(page).unwrap();
        let selected = vm.checked_add(33_554_432).unwrap();
        let mut before = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: Writable initialized native layout, checked immediately.
        assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut before) }, 0);
        assert!(before.rlim_cur == libc::RLIM_INFINITY || selected <= before.rlim_cur);
        let limit = libc::rlimit {
            rlim_cur: selected,
            rlim_max: before.rlim_max,
        };
        // SAFETY: Test process only; lowers soft AS, never changes hard limit.
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) }, 0);
        Self {
            before,
            vm,
            selected,
            active: true,
        }
    }
    fn restore(&mut self) -> u64 {
        // SAFETY: Exact saved layout, within the unchanged hard limit.
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_AS, &self.before) }, 0);
        self.active = false;
        let mut current = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: Writable initialized native layout and scalar process query.
        assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut current) }, 0);
        assert_eq!(
            (current.rlim_cur, current.rlim_max),
            (self.before.rlim_cur, self.before.rlim_max)
        );
        current.rlim_cur
    }
}
impl Drop for AddressSpaceLimit {
    fn drop(&mut self) {
        // SAFETY: Unwind cleanup restores the exact original limit once. An
        // un-restorable process must not continue the remaining native tests.
        if self.active && unsafe { libc::setrlimit(libc::RLIMIT_AS, &self.before) } != 0 {
            std::process::abort();
        }
    }
}
fn allocation_files() -> Vec<(u64, u64, u64)> {
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
            let metadata = fs::metadata(&path).unwrap();
            Some((
                path.file_name().unwrap().to_str().unwrap().parse().unwrap(),
                metadata.ino(),
                metadata.len(),
            ))
        })
        .collect()
}

#[test]
#[ignore = "requires explicit native Wlr256MiB offer, source mmap observations and isolated process limits"]
fn retains_the_failed_mmap_fd_until_buffer_replacement() {
    let socket = PathBuf::from(env::var_os("MFS_P1_WLR_ALLOCATION_SOCKET").unwrap());
    let directory = PathBuf::from(env::var_os("MFS_P1_WLR_ALLOCATION_ARTIFACTS").unwrap());
    let source = PathBuf::from(env::var_os("MFS_P1_WLR_ALLOCATION_SOURCE").unwrap());
    let adapter = Adapter::open();
    warm(&adapter);
    let mut rows = fs::File::create(directory.join("rust-allocation.tsv")).unwrap();
    let baseline = fds();
    let baseline_tasks = tasks();
    let mut held = Vec::new();
    for cycle in 0..20 {
        let (
            connected,
            failed,
            recovered,
            vm,
            selected,
            old_limit,
            restored,
            failed_fd,
            failed_inode,
            recovered_inode,
        );
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
            let original_owner = output.pointer;
            connected = fds();
            assert!(allocation_files().is_empty());
            let calls = Cell::new(0);
            let mut limit = AddressSpaceLimit::new();
            vm = limit.vm;
            selected = limit.selected;
            old_limit = limit.before.rlim_cur;
            let result = capture.capture_with(|_| {
                calls.set(calls.get() + 1);
                Ok(())
            });
            restored = limit.restore();
            let error = result.unwrap_err();
            let evidence = capture.evidence();
            failed = fds();
            let files = allocation_files();
            // Save actual failed observations before assertions. Missing FD
            // state in the old implementation remains an inspectable failure.
            fs::write(directory.join(format!("attempt-{cycle}.rust.tsv")), format!(
                "{vm}\t{selected}\t{old_limit}\t{restored}\t{}\t{connected}\t{failed}\t{}\t{}\n",
                evidence.buffer_error, files.len(), files.first().map_or(u64::MAX, |f| f.0)
            )).unwrap();
            assert_eq!(
                (error.operation, error.code),
                ("native Wlr frame failed", -6)
            );
            assert_eq!(
                (calls.get(), evidence.fault, evidence.buffer_error),
                (0, 0, libc::ENOMEM)
            );
            assert_eq!(output.pointer, original_owner);
            assert_eq!(output.pixels_for(7, 5), [201, 73, 17].repeat(35));
            fs::write(
                directory.join(format!("failed-{cycle}.rust.bgr")),
                output.pixels_for(7, 5),
            )
            .unwrap();
            assert_eq!(
                failed,
                connected + 1,
                "failed native mmap must retain the source memfd owner"
            );
            let [(fd, inode, size)] = files.as_slice() else {
                panic!("exact one failed memfd required")
            };
            assert_eq!(*size, 268_435_456);
            (failed_fd, failed_inode) = (*fd, *inode);
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
            let files = allocation_files();
            let [(_, inode, size)] = files.as_slice() else {
                panic!("exact one recovery memfd required")
            };
            assert_eq!(*size, 140);
            assert_ne!(*inode, failed_inode);
            assert_eq!(recovered, connected + 1);
            recovered_inode = *inode;
            output = image;
            held.push(output);
        }
        let final_fds = fds();
        assert_eq!((final_fds, tasks()), (baseline, baseline_tasks.clone()));
        assert!(allocation_files().is_empty());
        writeln!(rows,"{cycle}\t{baseline}\t{connected}\t{failed}\t{recovered}\t{final_fds}\t{vm}\t{selected}\t{old_limit}\t{restored}\t268435456\t12\t{failed_fd}\t{failed_inode}\t{recovered_inode}\t0\t1\t1").unwrap();
        rows.flush().unwrap();
    }
    for (index, image) in held.iter().enumerate() {
        let pixels = image.pixels_for(7, 5);
        assert_eq!(pixels, cases::target(0));
        assert_eq!(
            pixels,
            fs::read(source.join(format!("held-{index}.source.bgr"))).unwrap()
        );
        fs::write(directory.join(format!("held-{index}.rust.bgr")), pixels).unwrap();
    }
    println!(
        "Rust Wlr allocation cycles=20 failed_mmap=20 fd_retained=20 recovery=20 held=20 limits_restored=20 passed"
    );
}
