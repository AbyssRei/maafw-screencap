// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real retired display/foreign-backend lifetime, with the original prefix oracle.
use super::*;
use std::{
    fs::OpenOptions,
    io::Write,
    os::fd::{AsFd, AsRawFd},
};

fn fd_is_closed(fd: i32) -> bool {
    // SAFETY: F_GETFD observes this scalar descriptor; no ownership transfer or
    // variadic argument is needed. The tests do not open another fd before it.
    let result = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    result < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EBADF)
}
fn fd_is_live(fd: i32) -> bool {
    // SAFETY: Same immediate scalar F_GETFD observation, no ownership transfer.
    (unsafe { libc::fcntl(fd, libc::F_GETFD) }) >= 0
}
struct HeldRaw {
    samples: Vec<(String, Sample)>,
    busy: u64,
}
fn capture<'a>(
    source: &mut Capture,
    client: &Arc<Client>,
    adapter: &'a Adapter,
    directory: &Path,
    oracle: &Path,
    stem: &str,
    held: &mut HeldRaw,
) -> Image<'a> {
    let counts = source.context.counts.clone();
    let before = counts.destroyed.load(Ordering::Acquire);
    let image = source
        .capture_with(|sample| {
            // Actual conversion runs with the native frame still alive. Shared
            // control reentry must reject before touching globals/channel owners.
            assert_eq!(client.close().unwrap_err().code, libc::EBUSY);
            assert_eq!(
                client.connect(Path::new("unused-busy-path")).unwrap_err().code,
                libc::EBUSY
            );
            assert!(client.connected().unwrap());
            held.busy = held.busy.checked_add(2).unwrap();
            raw(&sample);
            std::fs::write(directory.join(format!("{stem}.raw")), sample.pixels()).unwrap();
            std::fs::write(
                directory.join(format!("{stem}.meta.txt")),
                format!(
                    "conversion={:?}\nwidth={}\nheight={}\nnative_stride={}\nready_time={:?}\nflags={:?}\n",
                    sample.conversion, sample.width, sample.height, sample.native_stride,
                    sample.ready_time, sample.flags
                ),
            ).unwrap();
            held.samples.push((stem.to_string(), sample.clone()));
            let image = adapter.convert(sample, 0)?;
            assert_eq!(counts.destroyed.load(Ordering::Acquire), before);
            Ok(image)
        })
        .unwrap();
    assert_eq!(
        counts.destroyed.load(Ordering::Acquire),
        before.checked_add(1).unwrap()
    );
    bgr(image.pixels());
    assert_eq!(
        image.pixels(),
        std::fs::read(oracle.join(format!("{stem}.bgr"))).unwrap()
    );
    std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
    image
}

#[test]
#[ignore = "requires the native owned compositor and original close-prefix oracle"]
fn closes_shared_client_after_ending_each_retained_native_snapshot() {
    let socket = PathBuf::from(env::var_os("MFS_P0_WLR_SOCKET").unwrap());
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("early-close");
    let oracle = PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap())
        .join("early-close-prefix");
    std::fs::create_dir(&directory).unwrap();
    let alias = socket.parent().unwrap().join(".mfs-r");
    assert_eq!(
        std::fs::symlink_metadata(&alias).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    std::os::unix::fs::symlink(&socket, &alias).unwrap();
    struct Alias(PathBuf);
    impl Drop for Alias {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).unwrap();
        }
    }
    let alias = Alias(alias);
    let adapter = Adapter::open();
    warm(&adapter);
    let mut records = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("early-close.tsv"))
        .unwrap();
    writeln!(records,"cycle\tmode\tconnected\tglobals\tmanager_refs\tshm_refs\toutput_refs\tnative_fd_retained\tclosed_rejected\tstale_rejected\trebound\told_channel_released\told_fd_closed\tfds_before\tfds_after\ttasks_equal\tcallbacks_balanced\tcaptures\theld\tbusy_rejections\tdata_created\tdata_destroyed\tdata_dropped").unwrap();
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let task_text = baseline_tasks
        .iter()
        .map(|id| format!("{}\n", id.to_str().unwrap()))
        .collect::<String>();
    std::fs::write(directory.join("early-close-tasks.txt"), &task_text).unwrap();
    let mut held_raw = HeldRaw {
        samples: Vec::new(),
        busy: 0,
    };
    let mut held_bgr = Vec::new();
    held_raw.samples.try_reserve_exact(60).unwrap();
    held_bgr.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        for (mode, name) in ["control", "cold", "buffered", "reconnect"]
            .into_iter()
            .enumerate()
        {
            let client = Client::new().unwrap().into_shared().unwrap();
            assert!(client.connect(&socket).unwrap());
            let counts = client.context.counts.clone();
            let context = Arc::downgrade(&client.context);
            let channel = client.current_channel().unwrap().unwrap();
            let retired = Arc::downgrade(&channel);
            // SAFETY: The local channel owner covers this immediate descriptor
            // observation. Only the integer escapes, never a Connection/backend.
            let old_fd = unsafe { channel.connection() }.as_fd().as_raw_fd();
            drop(channel);
            let (manager, shm, output) = {
                let state = client.context.state().unwrap();
                (
                    Arc::downgrade(state.globals.manager.as_ref().unwrap()),
                    Arc::downgrade(state.globals.shm.as_ref().unwrap()),
                    Arc::downgrade(state.globals.output.as_ref().unwrap()),
                )
            };
            let mut source = if mode == 0 {
                None
            } else {
                Some(Capture::from_client(client.clone()).unwrap())
            };
            held_raw.busy = 0;
            let previous = held_raw.samples.len();
            if mode >= 2 {
                let stem = format!("{cycle}-{name}-before");
                let image = capture(
                    source.as_mut().unwrap(),
                    &client,
                    &adapter,
                    &directory,
                    &oracle,
                    &stem,
                    &mut held_raw,
                );
                held_bgr.push((stem, image));
            }
            let mut rebound = 0_u64;
            if mode == 3 {
                assert!(client.connect(&alias.0).unwrap());
                rebound = 1;
                let current = client.current_channel().unwrap().unwrap();
                assert!(
                    !retired.ptr_eq(&Arc::downgrade(&current)),
                    "fresh connection identity"
                );
            } else {
                client.close().unwrap();
            }
            let connected = u64::from(client.connected().unwrap());
            let globals = {
                let s = client.context.state().unwrap();
                u64::from(s.globals.manager.is_some())
                    | (u64::from(s.globals.shm.is_some()) << 1)
                    | (u64::from(s.globals.output.is_some()) << 2)
            };
            assert_eq!(connected, u64::from(mode == 3));
            assert_eq!(globals, if mode == 3 { 7 } else { 0 });
            let expected_refs = usize::from(mode != 0);
            let refs = [
                manager.strong_count(),
                shm.strong_count(),
                output.strong_count(),
            ];
            assert_eq!(refs, [expected_refs; 3]);
            assert_eq!(retired.strong_count(), expected_refs);
            let live = u64::from(fd_is_live(old_fd));
            assert_eq!(live, u64::from(mode != 0));
            let mut closed_rejected = 0_u64;
            let mut stale_rejected = 0_u64;
            if mode != 0 {
                let before = counts.frames.load(Ordering::Acquire);
                let mut converted = false;
                let mut caller = [11_u8, 29, 47];
                let failure = source
                    .as_mut()
                    .unwrap()
                    .capture_with(|_| {
                        converted = true;
                        caller.fill(0);
                        Ok(())
                    })
                    .unwrap_err();
                assert!(!converted);
                assert_eq!(caller, [11, 29, 47]);
                assert_eq!(counts.frames.load(Ordering::Acquire), before);
                if mode == 3 {
                    assert_eq!(failure.operation, "Wlr stale connection snapshot");
                    stale_rejected = 1;
                } else {
                    assert_eq!(failure.operation, "closed Wlr source");
                    closed_rejected = 1;
                }
            }
            drop(source.take());
            assert!(retired.upgrade().is_none());
            assert!(fd_is_closed(old_fd));
            assert_eq!(
                [
                    manager.strong_count(),
                    shm.strong_count(),
                    output.strong_count()
                ],
                [0; 3]
            );
            if mode == 3 {
                let mut fresh = Capture::from_client(client.clone()).unwrap();
                let stem = format!("{cycle}-{name}-fresh");
                let image = capture(
                    &mut fresh,
                    &client,
                    &adapter,
                    &directory,
                    &oracle,
                    &stem,
                    &mut held_raw,
                );
                held_bgr.push((stem, image));
                drop(fresh);
            }
            client.close().unwrap();
            drop(client);
            assert!(context.upgrade().is_none());
            released_counts(&counts);
            assert_eq!(fds(), baseline_fds);
            assert_eq!(tasks(), baseline_tasks);
            std::fs::write(directory.join(format!("{cycle}-{name}.tasks")), &task_text).unwrap();
            let captures = held_raw.samples.len().checked_sub(previous).unwrap();
            let busy = held_raw.busy;
            assert_eq!(captures, [0, 0, 1, 2][mode]);
            assert_eq!(
                busy,
                u64::try_from(captures).unwrap().checked_mul(2).unwrap()
            );
            let created = counts.data_created.load(Ordering::Acquire);
            let destroyed = counts.destroyed.load(Ordering::Acquire);
            let dropped = counts.data_dropped.load(Ordering::Acquire);
            writeln!(records,"{cycle}\t{name}\t{connected}\t{globals}\t{}\t{}\t{}\t{live}\t{closed_rejected}\t{stale_rejected}\t{rebound}\t1\t1\t{baseline_fds}\t{baseline_fds}\t1\t1\t{captures}\t{captures}\t{busy}\t{created}\t{destroyed}\t{dropped}",refs[0],refs[1],refs[2]).unwrap();
            records.flush().unwrap();
        }
    }
    assert_eq!(held_raw.samples.len(), 60);
    assert_eq!(held_bgr.len(), 60);
    for (stem, sample) in &held_raw.samples {
        raw(sample);
        std::fs::write(directory.join(format!("{stem}.raw-held")), sample.pixels()).unwrap();
    }
    for (stem, image) in &held_bgr {
        bgr(image.pixels());
        std::fs::write(directory.join(format!("{stem}.held")), image.pixels()).unwrap();
    }
    drop(held_bgr);
    assert_eq!(adapter.owners(), 0);
    std::thread::spawn(move || {
        for (_, sample) in held_raw.samples {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr early_close cycles=20 cases=80 control_prefixes=80 retained_native_channels=60 closed_rejected=40 stale_rejected=20 captures=60 retained_raw=60 retained_bgr=60 busy_control_rejections=120 resources_balanced=1 passed"
    );
}
