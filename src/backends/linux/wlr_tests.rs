// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use std::{
    env,
    ffi::{CStr, CString, c_void},
    mem::{align_of, offset_of, size_of},
    os::unix::ffi::OsStrExt,
    ptr::NonNull,
};

#[path = "wlr_host_tests.rs"]
mod host;
#[path = "wlr_host_construction_tests.rs"]
mod host_construction;

#[repr(C)]
#[derive(Default)]
struct View {
    pixels: *const u8,
    length: usize,
    stride: usize,
    width: i32,
    height: i32,
}
type Convert =
    unsafe extern "C" fn(*const u8, usize, i32, i32, usize, i32, i32, *mut *mut c_void) -> i32;
type NativeView = unsafe extern "C" fn(*const c_void, *mut View) -> i32;
type Release = unsafe extern "C" fn(*mut c_void);
type Owners = unsafe extern "C" fn() -> u64;
type Abi = unsafe extern "C" fn(u32) -> usize;
static LOADER_ERROR: AtomicI32 = AtomicI32::new(0);
#[path = "wlr_failure_tests.rs"]
mod failures;
#[path = "wlr_name_tests.rs"]
mod names;
#[path = "wlr_protocol_tests.rs"]
mod protocol;
#[path = "wlr_registry_tests.rs"]
mod registry;
struct Module(NonNull<c_void>);
impl Module {
    fn open(path: &Path) -> Self {
        assert!(path.is_absolute());
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: Explicit built absolute module path, immediate/local symbols.
        Self(
            NonNull::new(unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) })
                .expect("load Wlr acceptance adapter"),
        )
    }
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        // SAFETY: Live module, static C symbol; caller supplies the exact header
        // type, checked below against native ABI and actual consumption.
        let pointer = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        assert!(!pointer.is_null());
        // SAFETY: Same retained symbol and verified callable pointer type.
        unsafe { std::mem::transmute_copy(&pointer) }
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Images borrow Adapter, so no owner or call outlives this module.
        let code = unsafe { libc::dlclose(self.0.as_ptr()) };
        if code != 0 {
            LOADER_ERROR.store(code, Ordering::Release);
        }
    }
}
struct Adapter {
    _module: Module,
    convert: Convert,
    view: NativeView,
    release: Release,
    owners: Owners,
}
impl Adapter {
    fn open() -> Self {
        let path = env::var_os("MFS_P1_WLR_OPENCV_ADAPTER").unwrap();
        let module = Module::open(Path::new(&path));
        // SAFETY: Private conversion-adapter.h is authoritative; this exact
        // self-owned C++ source is built by Wlr without PipeWire dependencies.
        let (convert, view, release, owners, abi) = unsafe {
            (
                module.symbol::<Convert>(c"mfs_pw_convert"),
                module.symbol::<NativeView>(c"mfs_pw_bgr_view"),
                module.symbol::<Release>(c"mfs_pw_bgr_release"),
                module.symbol::<Owners>(c"mfs_pw_bgr_owners"),
                module.symbol::<Abi>(c"mfs_pw_adapter_layout"),
            )
        };
        for (index, value) in [
            1,
            size_of::<View>(),
            align_of::<View>(),
            offset_of!(View, pixels),
            offset_of!(View, length),
            offset_of!(View, stride),
            offset_of!(View, width),
            offset_of!(View, height),
        ]
        .into_iter()
        .enumerate()
        {
            // SAFETY: Scalar native header query, live module and no pointers.
            assert_eq!(unsafe { abi(u32::try_from(index).unwrap()) }, value);
        }
        Self {
            _module: module,
            convert,
            view,
            release,
            owners,
        }
    }
    fn owners(&self) -> u64 {
        // SAFETY: Live module; native ledger is atomic.
        unsafe { (self.owners)() }
    }
    fn convert(&self, sample: Sample, fault: i32) -> Result<Image<'_>, Error> {
        let (channels, mode) = match sample.conversion {
            Conversion::Bgra => (4, 1),
            Conversion::Rgba => (4, 2),
            Conversion::Rgb => (3, 3),
            Conversion::BgrCopy if fault == 0 => (3, 5),
            Conversion::BgrCopy => return Err(Error::new("test BGR fault unsupported", 22)),
        };
        assert!((0..=1).contains(&fault));
        // A valid two-channel Mat with the BGRA conversion code triggers a
        // real cv::Exception inside the unchanged adapter. Its header has no
        // synthetic fault argument; retain that exact eight-argument ABI.
        let native_channels = if fault == 1 { 2 } else { channels };
        let mut output = std::ptr::null_mut();
        // SAFETY: Sample owns exact initialized packed source-Mat bytes through
        // this synchronous conversion; returned Mat has independent storage.
        let code = unsafe {
            (self.convert)(
                sample.pixels().as_ptr(),
                sample.pixels().len(),
                i32::try_from(sample.width).unwrap(),
                i32::try_from(sample.height).unwrap(),
                usize::try_from(sample.width).unwrap() * usize::try_from(channels).unwrap(),
                native_channels,
                mode,
                &mut output,
            )
        };
        if code != 0 {
            assert!(output.is_null());
            return Err(Error::new("native Wlr conversion", code));
        }
        Ok(Image {
            pointer: NonNull::new(output).unwrap(),
            adapter: self,
            width: i32::try_from(sample.width).unwrap(),
            height: i32::try_from(sample.height).unwrap(),
        })
    }
}
struct Image<'a> {
    pointer: NonNull<c_void>,
    adapter: &'a Adapter,
    width: i32,
    height: i32,
}
impl Image<'_> {
    fn pixels(&self) -> &[u8] {
        self.pixels_for(64, 48)
    }
    fn pixels_for(&self, width: i32, height: i32) -> &[u8] {
        assert_eq!((self.width, self.height), (width, height));
        let mut view = View::default();
        // SAFETY: Owning live Mat and exact native C header record.
        assert_eq!(
            unsafe { (self.adapter.view)(self.pointer.as_ptr(), &mut view) },
            0
        );
        let stride = usize::try_from(width).unwrap().checked_mul(3).unwrap();
        let length = stride
            .checked_mul(usize::try_from(height).unwrap())
            .unwrap();
        assert_eq!(
            (view.width, view.height, view.stride, view.length),
            (width, height, stride, length)
        );
        assert!(!view.pixels.is_null());
        // SAFETY: Adapter borrows real Mat/module owner, immutable and
        // complete checked native span, not the reused Wayland mapping.
        unsafe { std::slice::from_raw_parts(view.pixels, view.length) }
    }
}
impl Drop for Image<'_> {
    fn drop(&mut self) {
        // SAFETY: Sole opaque owner and retained module; release once.
        unsafe { (self.adapter.release)(self.pointer.as_ptr()) };
    }
}
fn tasks() -> Vec<std::ffi::OsString> {
    let mut tasks: Vec<_> = std::fs::read_dir("/proc/self/task")
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    tasks.sort();
    tasks
}
fn fds() -> usize {
    std::fs::read_dir("/proc/self/fd").unwrap().count()
}
fn bgr(bytes: &[u8]) {
    assert_eq!(bytes.len(), 9216);
    for y in 0..48 {
        for x in 0..64 {
            let expected = if y >= 24 {
                [255, 0, 0]
            } else if x < 32 {
                [0, 0, 255]
            } else {
                [0, 255, 0]
            };
            assert_eq!(
                &bytes[(y * 64 + x) * 3..(y * 64 + x + 1) * 3],
                expected,
                "pixel {x},{y}"
            );
        }
    }
}
fn raw(sample: &Sample) {
    assert_eq!((sample.width, sample.height), (64, 48));
    let (channels, order) = match sample.conversion {
        Conversion::Bgra => (4, [2, 1, 0]),
        Conversion::Rgba => (4, [0, 1, 2]),
        Conversion::Rgb => (3, [0, 1, 2]),
        Conversion::BgrCopy => (3, [2, 1, 0]),
    };
    assert_eq!(sample.native_stride, 64 * u32::try_from(channels).unwrap());
    assert_eq!(sample.pixels().len(), 64 * 48 * channels);
    for y in 0..48 {
        for x in 0..64 {
            let expected = if y >= 24 {
                [0, 0, 255]
            } else if x < 32 {
                [255, 0, 0]
            } else {
                [0, 255, 0]
            };
            for c in 0..3 {
                assert_eq!(
                    sample.pixels()[(y * 64 + x) * channels + order[c]],
                    expected[c]
                );
            }
        }
    }
}
fn warm(adapter: &Adapter) {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(64 * 48 * 4).unwrap();
    bytes.resize(64 * 48 * 4, 255);
    let sample = Sample {
        width: 64,
        height: 48,
        native_stride: 256,
        conversion: Conversion::Bgra,
        ready_time: None,
        flags: None,
        bytes: Arc::try_new(bytes).unwrap(),
    };
    drop(adapter.convert(sample, 0).unwrap());
    assert_eq!(adapter.owners(), 0);
}

struct ManagerConvert<'a> {
    adapter: &'a Adapter,
    held: Vec<Sample>,
}
impl<'a> crate::backends::linux::manager::native::Converter for ManagerConvert<'a> {
    type Image = Option<Image<'a>>;
    type Error = Error;
    fn wlr(&mut self, sample: Sample) -> Result<Self::Image, Error> {
        raw(&sample);
        self.held.push(sample.clone());
        self.adapter.convert(sample, 0).map(Some)
    }
    #[cfg(feature = "linux-pipewire")]
    fn pipewire(
        &mut self,
        _: Option<crate::backends::linux::pipewire::RawFrame>,
        _: crate::backends::linux::pipewire::Conversion,
    ) -> Result<Option<Self::Image>, Error> {
        unreachable!("Wlr manager cannot select PipeWire")
    }
    #[cfg(feature = "linux-pipewire")]
    fn pipewire_reuse(
        &mut self,
        _: Option<crate::backends::linux::pipewire::RawFrame>,
    ) -> Option<Self::Image> {
        unreachable!("Wlr manager cannot reuse PipeWire")
    }
    #[cfg(feature = "linux-pipewire")]
    fn empty(&mut self) -> Result<Self::Image, Error> {
        unreachable!("Wlr manager has no empty source sample")
    }
}

#[test]
#[ignore = "requires original native Wlr manager oracle and owned compositor"]
fn forwards_native_manager_reconnects_and_retains_images_after_shutdown() {
    use crate::backends::linux::manager::{
        self,
        native::{CaptureFailure, Evidence as ManagerEvidence},
    };
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("manager");
    let source = PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap()).join("manager");
    std::fs::create_dir(&directory).unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut converter = ManagerConvert {
        adapter: &adapter,
        held: Vec::new(),
    };
    converter.held.try_reserve_exact(60).unwrap();
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        let mut manager = manager::NativeManager::new(manager::Config {
            method: manager::WLR,
            wlr_socket: socket.as_os_str().as_bytes().to_vec(),
            pipewire_fd: -1,
            pipewire_node: 0,
            ..manager::Config::default()
        });
        let connected = manager.connect_native().unwrap();
        assert!(connected.connected && connected.failure.is_none());
        let ManagerEvidence::Wlr(initial) = connected.evidence else {
            panic!("missing initial Wlr evidence")
        };
        let first_close = connected.close.unwrap();
        assert!(first_close.take().is_none());
        let mut final_close = None;
        assert_eq!(initial.frames, 0, "connect must not capture");
        let mut last_outer = initial.outer;
        let mut output = None;
        for call in 0..3 {
            if call == 1 {
                assert!(manager.inactive_native());
            }
            if call == 2 {
                let connected = manager.connect_native().unwrap();
                assert!(connected.connected && connected.failure.is_none());
                let ManagerEvidence::Wlr(e) = connected.evidence else {
                    panic!("missing reconnect evidence")
                };
                assert_eq!(
                    (e.frames, e.outer),
                    (2, last_outer),
                    "reconnect must reuse client without capture/roundtrip"
                );
                let Some(ManagerEvidence::Wlr(closed)) = first_close.take() else {
                    panic!("old capture not closed")
                };
                assert_eq!((closed.frames, closed.cleanup, closed.fault), (2, 0, 0));
                assert!(
                    closed.data_created > closed.data_dropped,
                    "client globals must remain live through reconnect"
                );
                final_close = connected.close;
            }
            let result = manager.capture_native(&mut output, &mut converter);
            assert!(result.result.unwrap());
            let ManagerEvidence::Wlr(e) = result.evidence else {
                panic!("missing capture evidence")
            };
            assert_eq!((e.frames, e.copied), (call + 1, call + 1));
            assert_eq!((e.cleanup, e.fault, e.buffer_error), (0, 0, 0));
            last_outer = e.outer;
            let image = output.take().unwrap();
            bgr(image.pixels());
            let stem = format!("manager-{cycle}-{call}");
            std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
            assert_eq!(
                image.pixels(),
                std::fs::read(source.join(format!("{stem}.bgr"))).unwrap()
            );
            let sample = converter.held.last().unwrap();
            std::fs::write(directory.join(format!("{stem}.raw")), sample.pixels()).unwrap();
            std::fs::write(directory.join(format!("{stem}.meta.txt")),format!("conversion={:?}\nwidth={}\nheight={}\nnative_stride={}\nready_time={:?}\nflags={:?}\n",sample.conversion,sample.width,sample.height,sample.native_stride,sample.ready_time,sample.flags)).unwrap();
            held.push(image);
        }
        drop(manager);
        let Some(ManagerEvidence::Wlr(closed)) = final_close.unwrap().take() else {
            panic!("missing completed manager close")
        };
        assert_eq!((closed.frames, closed.copied), (3, 3));
        assert_eq!(
            (closed.cleanup, closed.fault, closed.buffer_error),
            (0, 0, 0)
        );
        assert_eq!(closed.data_created, closed.destroyed);
        assert_eq!(closed.data_created, closed.data_dropped);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        for image in &held {
            bgr(image.pixels());
        }
        eprintln!(
            "native Wlr manager cycle={cycle} retained={} fds={} baseline_fds={baseline_fds} reconnect_roundtrip=0",
            held.len(),
            fds()
        );
    }
    let missing = Path::new(&socket)
        .parent()
        .unwrap()
        .join("mfs-rust-manager-missing");
    assert!(!missing.exists());
    let mut manager = manager::NativeManager::new(manager::Config {
        method: manager::WLR,
        wlr_socket: missing.as_os_str().as_bytes().to_vec(),
        pipewire_fd: -1,
        pipewire_node: 0,
        ..manager::Config::default()
    });
    let connected = manager.connect_native().unwrap();
    assert!(!connected.connected && connected.failure.is_some());
    let mut output = Some(held.pop().unwrap());
    let pointer = output.as_ref().unwrap().pixels().as_ptr();
    let before_owners = adapter.owners();
    assert!(
        !manager
            .capture_native(&mut output, &mut converter)
            .result
            .unwrap()
    );
    assert_eq!(adapter.owners(), before_owners);
    let connected = manager.connect_native().unwrap();
    assert!(connected.connected && connected.failure.is_none());
    assert!(matches!(
        manager.capture_native(&mut output, &mut converter).result,
        Err(CaptureFailure::Wayland(_))
    ));
    assert_eq!(output.as_ref().unwrap().pixels().as_ptr(), pointer);
    bgr(output.as_ref().unwrap().pixels());
    assert_eq!(adapter.owners(), before_owners);
    assert_eq!(
        converter.held.len(),
        60,
        "failed client must not enter conversion"
    );
    drop(manager);
    assert_eq!(fds(), baseline_fds);
    assert_eq!(tasks(), baseline_tasks);
    held.push(output.take().unwrap());
    // Even an empty Option<Image> has drop glue tied to the adapter borrow.
    // End this output slot before explicitly unloading the owning module.
    drop(output);
    std::thread::spawn(move || {
        for sample in converter.held {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    drop(held);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr manager cycles=20 captures=60 connects=40 retained_raw=60 retained_bgr=60 same_client_reconnect=20 failed_client_retained=1 failed_output_preserved=1 resources_balanced=1 passed"
    );
}

fn context_capture<'a>(
    capture: &mut Capture,
    adapter: &'a Adapter,
    directory: &Path,
    oracle: &Path,
    stem: &str,
    held_raw: &mut Vec<Sample>,
) -> Image<'a> {
    let counts = capture.context.counts.clone();
    let before = counts.destroyed.load(Ordering::Acquire);
    let image = capture
        .capture_with(|sample| {
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
            held_raw.push(sample.clone());
            let image = adapter.convert(sample, 0)?;
            assert_eq!(counts.destroyed.load(Ordering::Acquire), before);
            Ok(image)
        })
        .unwrap();
    assert_eq!(counts.destroyed.load(Ordering::Acquire), before + 1);
    bgr(image.pixels());
    assert_eq!(
        image.pixels(),
        std::fs::read(oracle.join(format!("{stem}.bgr"))).unwrap()
    );
    std::fs::write(directory.join(format!("{stem}.bgr")), image.pixels()).unwrap();
    image
}

fn released_counts(counts: &Counts) {
    assert_eq!(counts.cleanup.load(Ordering::Acquire), 0);
    assert_eq!(counts.source_fd_zero_retained.load(Ordering::Acquire), 0);
    assert_eq!(
        counts.data_created.load(Ordering::Acquire),
        counts.destroyed.load(Ordering::Acquire)
    );
    assert_eq!(
        counts.data_created.load(Ordering::Acquire),
        counts.data_dropped.load(Ordering::Acquire)
    );
}

#[test]
#[ignore = "requires native owned compositor and original shared-client oracle"]
fn retains_shared_client_after_caller_and_first_capture_release() {
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("shared-client");
    let oracle =
        PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap()).join("shared-client");
    std::fs::create_dir(&directory).unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    held_raw.try_reserve_exact(60).unwrap();
    let mut held_bgr = Vec::new();
    held_bgr.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        let mut client = Client::new().unwrap();
        assert!(client.connect(Path::new(&socket)).unwrap());
        let client = client.into_shared().unwrap();
        let owner = Arc::downgrade(&client);
        let counts = client.context.counts.clone();
        let client_context = Arc::downgrade(&client.context);
        let mut first = Capture::from_client(client.clone()).unwrap();
        let first_context = Arc::downgrade(&first.context);
        let mut second = Capture::from_client(client.clone()).unwrap();
        assert!(Arc::ptr_eq(
            first.client.as_ref().unwrap(),
            second.client.as_ref().unwrap()
        ));
        // Constructor snapshots point to the actual same client globals, while
        // the mutable frame/buffer contexts are distinct.
        assert!(!Arc::ptr_eq(&first.context, &second.context));
        {
            let a = first.context.state().unwrap();
            let b = second.context.state().unwrap();
            assert!(Arc::ptr_eq(
                a.output.as_ref().unwrap(),
                b.output.as_ref().unwrap()
            ));
            assert!(Arc::ptr_eq(
                a.shm.as_ref().unwrap(),
                b.shm.as_ref().unwrap()
            ));
            assert!(Arc::ptr_eq(
                a.manager.as_ref().unwrap(),
                b.manager.as_ref().unwrap()
            ));
        }
        drop(client);
        assert_eq!(owner.strong_count(), 2);
        for call in 0..2 {
            let source = if call == 0 { &mut first } else { &mut second };
            held_bgr.push(context_capture(
                source,
                &adapter,
                &directory,
                &oracle,
                &format!("shared-{cycle}-{call}"),
                &mut held_raw,
            ));
            if call == 0 {
                assert!(second.context.state().unwrap().buffer.is_none());
            }
        }
        {
            let a = first.context.state().unwrap();
            let b = second.context.state().unwrap();
            assert_ne!(
                a.buffer.as_ref().unwrap().file.as_raw_fd(),
                b.buffer.as_ref().unwrap().file.as_raw_fd()
            );
        }
        drop(first);
        assert!(first_context.upgrade().is_none());
        assert_eq!(owner.strong_count(), 1);
        held_bgr.push(context_capture(
            &mut second,
            &adapter,
            &directory,
            &oracle,
            &format!("shared-{cycle}-2"),
            &mut held_raw,
        ));
        // Only an idle, Ready-completed map is overwritten. All previously
        // returned raw and BGR owners must remain independent of either buffer.
        second
            .context
            .state()
            .unwrap()
            .buffer
            .as_mut()
            .unwrap()
            .map
            .fill(0);
        assert_eq!(counts.frames.load(Ordering::Acquire), 3);
        assert_eq!(second.evidence().fault, 0);
        assert_eq!(second.evidence().buffer_error, 0);
        drop(second);
        assert!(owner.upgrade().is_none());
        assert!(client_context.upgrade().is_none());
        released_counts(&counts);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        for sample in &held_raw {
            raw(sample);
        }
        for image in &held_bgr {
            bgr(image.pixels());
        }
        eprintln!(
            "native Wlr shared cycle={cycle} held={} fds={} baseline_fds={baseline_fds} client_released=1 callbacks_released=1",
            held_bgr.len(),
            fds()
        );
    }
    std::thread::spawn(move || {
        for sample in held_raw {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    drop(held_bgr);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr shared_client cycles=20 captures=60 caller_released=20 first_source_released=20 last_client_released=20 retained_raw=60 retained_bgr=60 independent_buffers=20 resources_balanced=1 passed"
    );
}

#[test]
#[ignore = "requires native owned compositor and original safe-reconnect oracle"]
fn reconnects_same_and_changed_paths_after_old_capture_owners_end() {
    let socket = PathBuf::from(env::var_os("MFS_P0_WLR_SOCKET").unwrap());
    let alias = socket.parent().unwrap().join("mfs-rust-alias");
    let missing = socket.parent().unwrap().join("mfs-rust-missing");
    assert!(!alias.exists() && !missing.exists());
    std::os::unix::fs::symlink(&socket, &alias).unwrap();
    struct Alias(PathBuf);
    impl Drop for Alias {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).unwrap();
        }
    }
    let alias = Alias(alias);
    let directory =
        PathBuf::from(env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap()).join("reconnect");
    let oracle =
        PathBuf::from(env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap()).join("reconnect");
    std::fs::create_dir(&directory).unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    held_raw.try_reserve_exact(20).unwrap();
    let mut held_bgr = Vec::new();
    held_bgr.try_reserve_exact(20).unwrap();
    for cycle in 0..20 {
        let mut client = Client::new().unwrap();
        let counts = client.context.counts.clone();
        assert!(client.connect(&socket).unwrap());
        let output_id = client
            .context
            .state()
            .unwrap()
            .globals
            .output
            .as_ref()
            .unwrap()
            .proxy
            .id()
            .protocol_id();
        let before = counts.outer.load(Ordering::Acquire);
        let data_before = counts.data_created.load(Ordering::Acquire);
        assert!(client.connect(&socket).unwrap());
        assert_eq!(counts.outer.load(Ordering::Acquire), before);
        assert_eq!(counts.data_created.load(Ordering::Acquire), data_before);
        assert_eq!(
            client
                .context
                .state()
                .unwrap()
                .globals
                .output
                .as_ref()
                .unwrap()
                .proxy
                .id()
                .protocol_id(),
            output_id
        );
        client.close();
        client.close();
        assert!(!client.connected().unwrap());
        released_counts(&counts);
        assert!(client.connect(&socket).unwrap());
        assert!(counts.outer.load(Ordering::Acquire) > before);
        let before_change = counts.outer.load(Ordering::Acquire);
        assert!(client.connect(&alias.0).unwrap());
        assert!(counts.outer.load(Ordering::Acquire) > before_change);
        let client = client.into_shared().unwrap();
        let mut source = Capture::from_client(client.clone()).unwrap();
        held_bgr.push(context_capture(
            &mut source,
            &adapter,
            &directory,
            &oracle,
            &format!("reconnected-{cycle}"),
            &mut held_raw,
        ));
        drop(source);
        // Unique native owner is recovered only after all capture/proxy
        // snapshots have ended. No disconnect under a foreign backend clone.
        let mut client =
            Arc::try_unwrap(client).unwrap_or_else(|_| panic!("live old client owner"));
        assert!(client.connect(&missing).is_err());
        assert!(!client.connected().unwrap());
        assert!(client.channel.is_none());
        assert!(client.context.state().unwrap().globals.output.is_none());
        assert_eq!(client.socket_path, missing);
        released_counts(&counts);
        let before_missing = counts.outer.load(Ordering::Acquire);
        assert!(client.connect(&missing).is_err());
        assert_eq!(counts.outer.load(Ordering::Acquire), before_missing);
        assert!(client.connect(&socket).unwrap());
        client.close();
        released_counts(&counts);
        drop(client);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        for sample in &held_raw {
            raw(sample);
        }
        for image in &held_bgr {
            bgr(image.pixels());
        }
        eprintln!(
            "native Wlr reconnect cycle={cycle} fds={} baseline_fds={baseline_fds} roundtrips={}",
            fds(),
            counts.outer.load(Ordering::Acquire)
        );
    }
    std::thread::spawn(move || {
        for sample in held_raw {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    drop(held_bgr);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr reconnect cycles=20 same_path_noop=20 close_reopen=20 changed_path=20 missing_path=20 same_missing_retry=20 subsequent_connect=20 captures=20 retained_raw=20 retained_bgr=20 resources_balanced=1 passed"
    );
}

#[test]
fn rejects_overflowing_native_layouts_and_unsupported_source_conversions() {
    for layout in [
        Layout {
            format: 0,
            width: 0,
            height: 48,
            stride: 256,
        },
        Layout {
            format: 0,
            width: 64,
            height: u32::MAX,
            stride: 256,
        },
        Layout {
            format: 0,
            width: 64,
            height: 48,
            stride: u32::MAX,
        },
    ] {
        assert!(layout.size().is_err());
    }
    assert!(
        Layout {
            format: u32::MAX,
            width: 64,
            height: 48,
            stride: 256
        }
        .conversion()
        .is_err()
    );
}

#[test]
#[ignore = "requires native owned source-client compositor, OpenCV and original Wlr oracle"]
fn captures_source_scene_with_recursive_roundtrip_and_retained_outputs() {
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let directory = env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap();
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    let oracle = env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap();
    let oracle = Path::new(&oracle);
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline_fds = fds();
    let baseline_tasks = tasks();
    let mut held_raw = Vec::new();
    held_raw.try_reserve_exact(60).unwrap();
    let mut held_bgr = Vec::new();
    held_bgr.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        let mut capture = Capture::connect(Path::new(&socket)).unwrap();
        let counts = capture.context.counts.clone();
        for call in 0..3 {
            if call == 1 {
                let before = capture.evidence();
                capture.inactive();
                assert_eq!(capture.evidence().outer, before.outer);
            }
            let before = counts.destroyed.load(Ordering::Acquire);
            let image = capture
                .capture_with(|sample| {
                    raw(&sample);
                std::fs::write(directory.join(format!("cycle-{cycle}-call-{call}.meta.txt")),
                    format!("conversion={:?}\nwidth={}\nheight={}\nnative_stride={}\nready_time={:?}\nflags={:?}\n",sample.conversion,sample.width,sample.height,sample.native_stride,sample.ready_time,sample.flags)).unwrap();
                    std::fs::write(
                        directory.join(format!("cycle-{cycle}-call-{call}.raw")),
                        sample.pixels(),
                    )
                    .unwrap();
                    assert_eq!(
                        counts.destroyed.load(Ordering::Acquire),
                        before,
                        "frame destroyed before conversion"
                    );
                    held_raw.push(sample.clone());
                    let image = adapter.convert(sample, 0)?;
                    assert_eq!(
                        counts.destroyed.load(Ordering::Acquire),
                        before,
                        "frame destroyed during native conversion"
                    );
                    Ok(image)
                })
                .unwrap();
            assert_eq!(
                counts.destroyed.load(Ordering::Acquire),
                before + 1,
                "frame owner not released after conversion"
            );
            std::fs::write(
                directory.join(format!("cycle-{cycle}-call-{call}.bgr")),
                image.pixels(),
            )
            .unwrap();
            bgr(image.pixels());
            assert_eq!(
                image.pixels(),
                std::fs::read(oracle.join(format!("cycle-{cycle}-call-{call}.bgr"))).unwrap()
            );
            held_bgr.push(image);
            // SAFETY PREMISE: Ready completed this Copy; no next Copy has been
            // requested. Only owned idle SHM is overwritten, not a live sample.
            capture
                .context
                .state()
                .unwrap()
                .buffer
                .as_mut()
                .unwrap()
                .map
                .fill(0);
            for sample in &held_raw {
                raw(sample);
            }
            for image in &held_bgr {
                bgr(image.pixels());
            }
        }
        capture.close();
        capture.close();
        let evidence = capture.evidence();
        assert_eq!((evidence.frames, evidence.copied), (3, 3));
        assert!(evidence.recursive >= 1);
        assert_eq!(evidence.fault, 0, "{evidence:?}");
        assert_eq!(evidence.buffer_error, 0, "{evidence:?}");
        assert_eq!(evidence.cleanup, 0, "{evidence:?}");
        assert_eq!(evidence.source_fd_zero_retained, 0, "{evidence:?}");
        assert_eq!(evidence.data_created, evidence.destroyed, "{evidence:?}");
        assert_eq!(evidence.data_created, evidence.data_dropped, "{evidence:?}");
        assert!(capture.capture_with(Ok::<_, Error>).is_err());
        drop(capture);
        assert_eq!(fds(), baseline_fds);
        assert_eq!(tasks(), baseline_tasks);
        eprintln!("native Rust Wlr cycle={cycle} evidence={evidence:?}");
    }
    std::thread::spawn(move || {
        for sample in held_raw {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    for image in &held_bgr {
        bgr(image.pixels());
    }
    drop(held_bgr);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust Wlr cycles=20 captures=60 retained_raw=60 retained_bgr=60 frame_conversion_lifetimes=60 overwritten_maps=60 resources_balanced=1 passed"
    );
}

#[test]
#[ignore = "requires native owned Wlr compositor and real OpenCV exception fixture"]
fn releases_frame_after_native_conversion_exception_and_captures_again() {
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline = fds();
    let before_tasks = tasks();
    let mut capture = Capture::connect(Path::new(&socket)).unwrap();
    let counts = capture.context.counts.clone();
    let before = counts.destroyed.load(Ordering::Acquire);
    let result = capture.capture_with(|sample| {
        assert_eq!(counts.destroyed.load(Ordering::Acquire), before);
        adapter.convert(sample, 1)
    });
    // Consume the Result here, releasing its possible image/module borrow
    // before the adapter is explicitly unloaded at the end of this test.
    assert_eq!(result.err(), Some(Error::new("native Wlr conversion", 2)));
    assert_eq!(counts.destroyed.load(Ordering::Acquire), before + 1);
    let image = capture
        .capture_with(|sample| adapter.convert(sample, 0))
        .unwrap();
    bgr(image.pixels());
    capture.close();
    let e = capture.evidence();
    assert_eq!((e.frames, e.cleanup, e.fault, e.buffer_error), (2, 0, 0, 0));
    assert_eq!(e.source_fd_zero_retained, 0);
    assert_eq!(e.data_created, e.destroyed);
    assert_eq!(e.data_created, e.data_dropped);
    drop(capture);
    bgr(image.pixels());
    drop(image);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(fds(), baseline);
    assert_eq!(tasks(), before_tasks);
    eprintln!(
        "native Wlr conversion_exception=1 frame_released=1 subsequent_capture=1 native_owners_closed=1 passed"
    );
}

#[test]
#[ignore = "isolated stdin/FD-zero native Wlr residual probe; explicit fixture reclamation"]
fn retains_original_fd_zero_residual_and_cpu_outputs_until_fixture_reclaim() {
    let socket = env::var_os("MFS_P0_WLR_SOCKET").unwrap();
    let artifacts = env::var_os("MFS_P1_WLR_RUST_ARTIFACTS").unwrap();
    let directory = Path::new(&artifacts).join("fd-zero");
    std::fs::create_dir_all(&directory).unwrap();
    let original = env::var_os("MFS_P1_WLR_SOURCE_ARTIFACTS").unwrap();
    let adapter = Adapter::open();
    warm(&adapter);
    let baseline = fds();
    let baseline_tasks = tasks();
    let mut raw_owners = Vec::new();
    let mut bgr_owners = Vec::new();
    raw_owners.try_reserve_exact(20).unwrap();
    bgr_owners.try_reserve_exact(20).unwrap();
    for cycle in 0..20 {
        let mut capture = Capture::connect(Path::new(&socket)).unwrap();
        // Own the connection and initialize the adapter BEFORE freeing zero.
        // This exact isolated libtest process has one active native test.
        // SAFETY: Observe and duplicate this process's live stdin descriptor.
        let flags = unsafe { libc::fcntl(0, libc::F_GETFD) };
        assert!(flags >= 0);
        let saved = unsafe { libc::fcntl(0, libc::F_DUPFD_CLOEXEC, 3) };
        assert!(saved >= 3);
        assert_eq!(unsafe { libc::close(0) }, 0);
        let context = capture.context.clone();
        let mut pointer = std::ptr::null_mut::<u8>();
        let mut length = 0;
        let image = capture
            .capture_with(|sample| {
                raw(&sample);
                {
                    let state = context.state().unwrap();
                    let buffer = state.buffer.as_ref().unwrap();
                    assert_eq!(buffer.file.as_raw_fd(), 0);
                    pointer = buffer.map.as_ptr().cast_mut();
                    length = buffer.map.len();
                }
                raw_owners.push(sample.clone());
                adapter.convert(sample, 0)
            })
            .unwrap();
        capture.close();
        let evidence = capture.evidence();
        assert_eq!(evidence.source_fd_zero_retained, 1);
        assert_eq!(
            (evidence.frames, evidence.cleanup, evidence.fault),
            (1, 0, 0)
        );
        assert_eq!(evidence.buffer_error, 0);
        assert_eq!(evidence.data_created, evidence.destroyed);
        assert_eq!(evidence.data_created, evidence.data_dropped);
        drop(capture);
        drop(context);
        assert!(!pointer.is_null());
        assert_eq!(length, 64 * 48 * 4);
        assert!(unsafe { libc::fcntl(0, libc::F_GETFD) } >= 0);
        // SAFETY: The sole source-compatible mapping owner was intentionally
        // retained by Buffer::drop, after native Ready/conversion and complete
        // channel/proxy teardown. mincore observes its mapped span first.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        let page = usize::try_from(page).unwrap();
        assert_ne!(page, 0);
        let mut resident = vec![0_u8; length.div_ceil(page)];
        assert_eq!(
            unsafe { libc::mincore(pointer.cast(), length, resident.as_mut_ptr()) },
            0
        );
        // No further Copy or Rust/native pixel borrow remains; mutate only the
        // fixture-owned residual and prove that both CPU outputs are isolated.
        unsafe { std::ptr::write_bytes(pointer, 0, length) };
        raw(raw_owners.last().unwrap());
        bgr(image.pixels());
        // SAFETY: Explicit fixture recovery of the two observed residuals,
        // exactly once, with no surviving Rust MmapMut/File owner or borrow.
        assert_eq!(unsafe { libc::munmap(pointer.cast(), length) }, 0);
        assert_eq!(unsafe { libc::close(0) }, 0);
        assert_eq!(unsafe { libc::dup2(saved, 0) }, 0);
        assert_eq!(unsafe { libc::fcntl(0, libc::F_SETFD, flags) }, 0);
        assert_eq!(unsafe { libc::close(saved) }, 0);
        let expected =
            std::fs::read(Path::new(&original).join(format!("cycle-{cycle}-call-0.bgr"))).unwrap();
        assert_eq!(image.pixels(), expected);
        std::fs::write(
            directory.join(format!("cycle-{cycle}.raw")),
            raw_owners.last().unwrap().pixels(),
        )
        .unwrap();
        std::fs::write(directory.join(format!("cycle-{cycle}.bgr")), image.pixels()).unwrap();
        bgr_owners.push(image);
        assert_eq!(fds(), baseline, "after explicit fixture residual reclaim");
        assert_eq!(tasks(), baseline_tasks);
        eprintln!(
            "native Wlr fd_zero cycle={cycle} source_residuals=1 fixture_reclaimed=1 evidence={evidence:?}"
        );
    }
    std::thread::spawn(move || {
        for sample in raw_owners {
            raw(&sample);
        }
    })
    .join()
    .unwrap();
    for image in &bgr_owners {
        bgr(image.pixels());
    }
    drop(bgr_owners);
    assert_eq!(adapter.owners(), 0);
    drop(adapter);
    assert_eq!(LOADER_ERROR.load(Ordering::Acquire), 0);
    assert_eq!(fds(), baseline);
    // Native task identities were checked in every cycle before the separate
    // joined CPU reader. A joined reader's transient exiting /proc task is not
    // an active backend resource and is not used to repeat a capture/assertion.
    eprintln!(
        "native Rust Wlr fd_zero_cycles=20 captures=20 retained_raw=20 retained_bgr=20 source_descriptor_mapping_retained=20 fixture_reclaimed=20 product_cleanup_success=0 passed"
    );
}
