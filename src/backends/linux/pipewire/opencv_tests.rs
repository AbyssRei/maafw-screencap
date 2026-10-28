// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit test-only adapter. The private C header is the ABI authority; native
//! layout queries and real consumption verify these Rust declarations. No SDK
//! header, core OpenCV dependency or Session/Frame API is introduced.
use super::*;
use std::{
    ffi::{CString, c_void},
    mem::{align_of, offset_of, size_of},
    os::unix::ffi::OsStrExt,
    ptr::NonNull,
};

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct View {
    pixels: *const u8,
    length: usize,
    stride: usize,
    width: i32,
    height: i32,
}
type ConvertFn =
    unsafe extern "C" fn(*const u8, usize, i32, i32, usize, i32, i32, *mut *mut c_void) -> i32;
type ViewFn = unsafe extern "C" fn(*const c_void, *mut View) -> i32;
type ReleaseFn = unsafe extern "C" fn(*mut c_void);
type OwnersFn = unsafe extern "C" fn() -> u64;
type LayoutFn = unsafe extern "C" fn(u32) -> usize;
static LOADER_CLEANUP_ERROR: AtomicI32 = AtomicI32::new(0);

struct Module(NonNull<c_void>);
impl Module {
    fn open(path: &Path) -> Self {
        assert!(path.is_absolute(), "explicit absolute private adapter path");
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: NUL-terminated absolute path of the explicitly built adapter;
        // local visibility, immediate relocation, no global loader changes.
        let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        Self(NonNull::new(handle).expect("load explicit private OpenCV adapter"))
    }
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> T {
        assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        // SAFETY: Live loader owner and static symbol name. Caller supplies the
        // exact conversion-adapter.h signature; no function outlives this module.
        let pointer = unsafe { libc::dlsym(self.0.as_ptr(), name.as_ptr()) };
        assert!(!pointer.is_null(), "missing adapter symbol {name:?}");
        // SAFETY: Header signature plus native ABI/layout checks and pointer
        // representation check above. Dynamic POSIX symbols are callable here.
        unsafe { std::mem::transmute_copy(&pointer) }
    }
}
impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: Every returned Image borrows Adapter, which owns this module.
        // All images and synchronous foreign calls end before this final close.
        let code = unsafe { libc::dlclose(self.0.as_ptr()) };
        if code != 0 {
            LOADER_CLEANUP_ERROR.store(code, Ordering::Release);
        }
    }
}
struct Adapter {
    _module: Module,
    convert: ConvertFn,
    view: ViewFn,
    release: ReleaseFn,
    owners: OwnersFn,
}
#[derive(Debug, PartialEq, Eq)]
enum AdapterError {
    InvalidLayout,
    Native(i32),
    NoCallbackProgress,
}
impl Adapter {
    fn open() -> Self {
        let path =
            env::var_os("MFS_P1_PIPEWIRE_OPENCV_ADAPTER").expect("explicit native OpenCV adapter");
        let module = Module::open(Path::new(&path));
        // SAFETY: Names/signatures are from the maintained private C header.
        // The module remains owned by the complete Adapter after construction.
        let (convert, view, release, owners, layout) = unsafe {
            (
                module.symbol::<ConvertFn>(c"mfs_pw_convert"),
                module.symbol::<ViewFn>(c"mfs_pw_bgr_view"),
                module.symbol::<ReleaseFn>(c"mfs_pw_bgr_release"),
                module.symbol::<OwnersFn>(c"mfs_pw_bgr_owners"),
                module.symbol::<LayoutFn>(c"mfs_pw_adapter_layout"),
            )
        };
        for (index, expected) in [
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
            // SAFETY: Exact header function; no foreign pointer input.
            assert_eq!(
                unsafe { layout(u32::try_from(index).unwrap()) },
                expected,
                "private ABI field index={index}"
            );
        }
        Self {
            _module: module,
            convert,
            view,
            release,
            owners,
        }
    }
    fn owner_count(&self) -> u64 {
        // SAFETY: Owning module is live; native ledger uses an atomic scalar.
        unsafe { (self.owners)() }
    }
    fn convert(&self, raw: &RawFrame, mode: Conversion) -> Result<Image<'_>, AdapterError> {
        let channels = raw.format.channels().ok_or(AdapterError::InvalidLayout)?;
        let width = i32::try_from(raw.width).map_err(|_| AdapterError::InvalidLayout)?;
        let height = i32::try_from(raw.height).map_err(|_| AdapterError::InvalidLayout)?;
        let channels = i32::try_from(channels).map_err(|_| AdapterError::InvalidLayout)?;
        let mode = match mode {
            Conversion::Bgra => 1,
            Conversion::Rgba => 2,
            Conversion::Rgb => 3,
            Conversion::Yuy2 => 4,
        };
        let mut owner = ptr::null_mut();
        // SAFETY: Immutable independent CPU owner covers all initialized target
        // rows for this call. Native code validates length/stride, reads via a
        // temporary Mat header and returns a distinct owning BGR allocation.
        // No C++ exception or object layout crosses this private C boundary.
        let code = unsafe {
            (self.convert)(
                raw.pixels().as_ptr(),
                raw.pixels().len(),
                width,
                height,
                raw.stride,
                channels,
                mode,
                &mut owner,
            )
        };
        let Some(owner) = NonNull::new(owner) else {
            return Err(if code == 0 {
                AdapterError::InvalidLayout
            } else {
                AdapterError::Native(code)
            });
        };
        // Establish cleanup even if native success/layout validation later fails.
        let mut image = Image {
            adapter: self,
            owner,
            view: View::default(),
        };
        if code != 0 {
            return Err(AdapterError::Native(code));
        }
        // SAFETY: The exact opaque owner was created by this module. Output is
        // initialized, independently writable storage for this synchronous view.
        let code = unsafe { (self.view)(owner.as_ptr(), &mut image.view) };
        if code != 0 {
            return Err(AdapterError::Native(code));
        }
        let view = image.view;
        let row = usize::try_from(view.width)
            .ok()
            .and_then(|w| w.checked_mul(3))
            .ok_or(AdapterError::InvalidLayout)?;
        let length = usize::try_from(view.height)
            .ok()
            .and_then(|h| h.checked_mul(row))
            .ok_or(AdapterError::InvalidLayout)?;
        if view.width != width
            || view.height != height
            || view.pixels.is_null()
            || row != view.stride
            || length != view.length
            || isize::try_from(length).is_err()
        {
            return Err(AdapterError::InvalidLayout);
        }
        Ok(image)
    }
}
// Deliberately no Send/Sync declaration for opaque Mat/loader owners. Their
// mobility requires its own validation; native RawFrame has separate evidence.
struct Image<'a> {
    adapter: &'a Adapter,
    owner: NonNull<c_void>,
    view: View,
}
impl Image<'_> {
    fn pixels(&self) -> &[u8] {
        // SAFETY: Constructor verified exact initialized tight BGR span. Native
        // owner is immutable and its module/lifetime is retained by this borrow.
        unsafe { std::slice::from_raw_parts(self.view.pixels, self.view.length) }
    }
}
impl Drop for Image<'_> {
    fn drop(&mut self) {
        // SAFETY: Owns exactly one successful foreign transfer, not cloneable;
        // all pixel borrows end and this module is retained until release returns.
        unsafe {
            (self.adapter.release)(self.owner.as_ptr());
        }
    }
}
enum Bgr<'a> {
    Converted(Image<'a>),
    Reused(RawFrame),
}
impl Bgr<'_> {
    fn pixels(&self) -> &[u8] {
        match self {
            Self::Converted(image) => image.pixels(),
            Self::Reused(raw) => raw.pixels(),
        }
    }
}
struct OpenCv<'a> {
    adapter: &'a Adapter,
    progress: Arc<AtomicU64>,
    witnessed: bool,
}
impl<'a> Converter<RawFrame> for OpenCv<'a> {
    type Image = Bgr<'a>;
    type Error = AdapterError;
    fn convert(
        &mut self,
        raw: Option<RawFrame>,
        mode: Conversion,
    ) -> Result<Option<Self::Image>, Self::Error> {
        if !self.witnessed {
            // Separate diagnostic prelude: require a real process callback to
            // advance at conversion entry, proving the native lock was released.
            // This added wait is not source latency/performance parity evidence.
            let before = self.progress.load(Ordering::Acquire);
            let deadline = Instant::now()
                .checked_add(Duration::from_secs(2))
                .ok_or(AdapterError::NoCallbackProgress)?;
            while self.progress.load(Ordering::Acquire) == before {
                if Instant::now() >= deadline {
                    return Err(AdapterError::NoCallbackProgress);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            self.witnessed = true;
        }
        let raw = raw.ok_or(AdapterError::InvalidLayout)?;
        self.adapter
            .convert(&raw, mode)
            .map(|image| Some(Bgr::Converted(image)))
    }
    fn reuse(&mut self, raw: Option<RawFrame>) -> Option<Self::Image> {
        raw.map(Bgr::Reused)
    }
}
fn known_bgr(image: &Bgr<'_>) -> u32 {
    let pixels = image.pixels();
    assert_eq!(pixels.len(), 28 * 24 * 3);
    let serial = (u32::from(pixels[2]) << 16) | (u32::from(pixels[1]) << 8) | u32::from(pixels[0]);
    assert_ne!(serial, 0);
    let rgb = [
        [191, 191, 191],
        [191, 191, 0],
        [0, 191, 191],
        [0, 191, 0],
        [191, 0, 191],
        [191, 0, 0],
        [0, 0, 191],
    ];
    for y in 0..24 {
        for x in 0..28 {
            if x == 0 && y == 0 {
                continue;
            }
            for (channel, base) in rgb[x / 4].iter().copied().enumerate() {
                let delta =
                    (y * 7 + x * 3 + channel * 13 + usize::try_from(serial & 63).unwrap()) & 63;
                assert_eq!(
                    pixels[y * 84 + x * 3 + 2 - channel],
                    base ^ u8::try_from(delta).unwrap(),
                    "native OpenCV pixel x={x} y={y} c={channel} serial={serial}"
                );
            }
        }
    }
    serial
}

#[test]
#[ignore = "explicit private native OpenCV ABI, real exception and owner-transfer checks"]
fn validates_opencv_abi_failures_and_owner_transfer() {
    let adapter = Adapter::open();
    assert_eq!(adapter.owner_count(), 0);
    let bytes = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    for (length, width, height, stride, channels, mode, expected) in [
        (12, 2, 2, 5, 3, 3, 1),          // short stride
        (11, 2, 2, 6, 3, 3, 1),          // short initialized span
        (12, 2, 2, usize::MAX, 3, 3, 1), // checked stride/span overflow
        (12, 0, 2, 6, 3, 3, 1),
        (12, 2, 2, 6, 3, 0, 1), // unsupported private mode
        (12, 2, 2, 6, 3, 4, 2), // real OpenCV YUY2 conversion rejects RGB channel count
    ] {
        let mut output = NonNull::<u8>::dangling().as_ptr().cast();
        // SAFETY: All source bytes are initialized and live. Invalid layout and
        // mode cases must reject before reading; the genuine exception case has
        // a valid RGB Mat. Output is independently writable pointer storage.
        let code = unsafe {
            (adapter.convert)(
                bytes.as_ptr(),
                length,
                width,
                height,
                stride,
                channels,
                mode,
                &mut output,
            )
        };
        assert_eq!(code, expected);
        assert!(output.is_null(), "failure output must be cleared");
        assert_eq!(
            adapter.owner_count(),
            0,
            "failure must not transfer/leak a Mat owner"
        );
    }
    let mut view = View {
        pixels: NonNull::<u8>::dangling().as_ptr(),
        length: 1,
        stride: 1,
        width: 1,
        height: 1,
    };
    // SAFETY: Null opaque owner is explicitly accepted as a failed view; the
    // output is a real initialized struct of the verified native layout.
    assert_eq!(unsafe { (adapter.view)(ptr::null(), &mut view) }, 1);
    assert_eq!(view, View::default());
    // SAFETY: Null release is defined as no-op by the private C header.
    unsafe {
        (adapter.release)(ptr::null_mut());
    }
    let mut cache = RawCache::default();
    let layout = Layout::checked(2, 2, Format::Rgb, 12, None).unwrap();
    // SAFETY: Complete initialized input and nonaliasing newly owned storage.
    unsafe { cache.copy_native(bytes.as_ptr(), layout) }.unwrap();
    let image = adapter
        .convert(&cache.snapshot().unwrap(), Conversion::Rgb)
        .unwrap();
    assert_eq!(adapter.owner_count(), 1);
    assert_eq!(image.pixels(), [3, 2, 1, 6, 5, 4, 9, 8, 7, 12, 11, 10]);
    cache.clear();
    assert_eq!(image.pixels(), [3, 2, 1, 6, 5, 4, 9, 8, 7, 12, 11, 10]);
    drop(image);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!("native OpenCV private ABI failures=6 exception=1 independent_bgr_owner=1 passed");
}

#[test]
#[ignore = "explicit original native SPA format parser and actual OpenCV all-format oracle"]
fn matches_original_format_pods_and_padded_conversion_bytes() {
    let directory =
        env::var("MFS_P1_PIPEWIRE_FORMAT_ORACLE").expect("explicit native format oracle");
    let directory = Path::new(&directory);
    let records = fs::read_to_string(directory.join("cases.tsv")).unwrap();
    let adapter = Adapter::open();
    let formats = [
        Format::Bgra,
        Format::Bgrx,
        Format::Rgba,
        Format::Rgbx,
        Format::Rgb,
        Format::Bgr,
        Format::Yuy2,
    ];
    let mut lines = records.lines();
    assert_eq!(
        lines.next().unwrap(),
        "index\tspa_format\tchannels\twidth\theight\tstride\tchoice_none\tlinear_modifier\tnative_parse_result"
    );
    let (mut cases, mut conversions, mut reuses) = (0, 0, 0);
    for line in lines {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 9);
        let values: Vec<_> = fields[..8]
            .iter()
            .map(|field| field.parse::<u32>().unwrap())
            .collect();
        let [
            index,
            numeric,
            channels,
            width,
            height,
            stride,
            choice,
            modifier,
        ] = <[u32; 8]>::try_from(values).unwrap();
        assert_eq!(usize::try_from(index).unwrap(), cases);
        let format = formats[cases / 4];
        assert_eq!((width, height), (8, 3));
        assert_eq!(usize::try_from(choice).unwrap(), (cases / 2) % 2);
        assert_eq!(usize::try_from(modifier).unwrap(), cases % 2);
        assert_eq!(pod::to_format(numeric), format);
        assert_eq!(format.channels(), Some(usize::try_from(channels).unwrap()));
        assert_eq!(
            stride,
            width
                .checked_mul(channels)
                .unwrap()
                .checked_add(11)
                .unwrap()
        );
        let name = format!("case-{index}");
        let encoded = fs::read(directory.join(format!("{name}.pod"))).unwrap();
        assert!((8..=4096).contains(&encoded.len()));
        let declared = u32::from_ne_bytes(encoded[..4].try_into().unwrap());
        assert_eq!(
            usize::try_from(declared).unwrap().checked_add(8).unwrap(),
            encoded.len()
        );
        // Aligned, fully initialized native POD storage. Only complete valid
        // native builder outputs are offered to native parsers; malformed-input
        // validation and callback negotiation are separate unfinished matrices.
        let mut storage = [0_u64; 512];
        // SAFETY: The initialized u64 array is 4096 bytes and permits arbitrary
        // byte writes. No u64 value or native pointer is used during this copy.
        let bytes =
            unsafe { std::slice::from_raw_parts_mut(storage.as_mut_ptr().cast::<u8>(), 4096) };
        bytes[..encoded.len()].copy_from_slice(&encoded);
        // SAFETY: Exact complete native fixture with verified header length in
        // correctly aligned initialized storage, borrowed only for this call.
        let parsed = unsafe { pod::parse(storage.as_ptr().cast()) };
        assert_eq!(parsed, Some((width, height, format)), "case={index}");
        let mut raw = fs::read(directory.join(format!("{name}.raw"))).unwrap();
        assert_eq!(
            raw.len(),
            usize::try_from(stride.checked_mul(height).unwrap()).unwrap()
        );
        let layout = Layout::checked(
            width,
            height,
            format,
            u32::try_from(raw.len()).unwrap(),
            Some(Chunk {
                stride: i32::try_from(stride).unwrap(),
                offset: 0,
                size: u32::try_from(raw.len()).unwrap(),
                flags: 0,
            }),
        )
        .unwrap();
        let mut cache = RawCache::default();
        // SAFETY: This initialized fixture contains the entire checked padded
        // span. The new CPU cache cannot alias its immutable input allocation.
        let copied = unsafe { cache.copy_native(raw.as_ptr(), layout) }.unwrap();
        assert_eq!(
            copied.copied_bytes,
            usize::try_from(
                width
                    .checked_mul(height)
                    .unwrap()
                    .checked_mul(channels)
                    .unwrap()
            )
            .unwrap()
        );
        raw.fill(0); // Independent target copy must survive input overwrite.
        let frame = cache.snapshot().unwrap();
        let output = match format.conversion().unwrap() {
            Some(mode) => {
                conversions += 1;
                Bgr::Converted(adapter.convert(&frame, mode).unwrap())
            }
            None => {
                reuses += 1;
                Bgr::Reused(frame)
            }
        };
        cache.clear();
        assert_eq!(
            output.pixels(),
            fs::read(directory.join(format!("{name}.bgr"))).unwrap(),
            "source format/conversion bytes differ case={index} format={format:?}"
        );
        drop(output);
        assert_eq!(adapter.owner_count(), 0);
        eprintln!(
            "native format case={index} format={format:?} choice_none={choice} linear_modifier={modifier} native_parse_result={} source_bytes_equal=1",
            fields[8].parse::<i32>().unwrap()
        );
        cases += 1;
    }
    assert_eq!((cases, conversions, reuses), (28, 24, 4));
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native PipeWire format oracle cases=28 formats=7 conversions=24 bgr_reuses=4 exact_bytes=1 passed"
    );
}

#[test]
#[ignore = "explicit native Linux + actual OpenCV conversion boundary and retained BGR owners"]
fn captures_bgr_after_native_unlock_and_retains_opencv_owners_after_shutdown() {
    let adapter = Adapter::open();
    assert_eq!(adapter.owner_count(), 0);
    // Warm conversion runtime before native fd/thread resource accounting.
    let bytes = [0_u8; 28 * 24 * 3];
    let layout = Layout::checked(
        28,
        24,
        Format::Rgb,
        u32::try_from(bytes.len()).unwrap(),
        None,
    )
    .unwrap();
    let mut warm = RawCache::default();
    // SAFETY: This initialized test array has the checked complete target span
    // and cannot alias the newly allocated independent CPU cache.
    unsafe { warm.copy_native(bytes.as_ptr(), layout) }.unwrap();
    drop(
        adapter
            .convert(&warm.snapshot().unwrap(), Conversion::Rgb)
            .unwrap(),
    );
    let socket = env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap();
    assert_eq!(
        Path::new(&env::var("PIPEWIRE_RUNTIME_DIR").unwrap())
            .join(env::var("PIPEWIRE_REMOTE").unwrap()),
        Path::new(&socket)
    );
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    let directory = env::var("MFS_P1_PIPEWIRE_BGR_ARTIFACTS").expect("explicit BGR artifacts");
    fs::create_dir_all(&directory).unwrap();
    let fds = count("/proc/self/fd");
    let baseline = task_ids();
    task_snapshot(Path::new(&directory), "baseline", &baseline);
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    for cycle in 0..20 {
        let native = Native::new().unwrap();
        let balance_init = BalanceInit;
        let mut native = native;
        native.init_daemon(node).unwrap();
        let mut consumer = Consumer::default();
        let mut converter = OpenCv {
            adapter: &adapter,
            progress: Arc::clone(&native.callbacks.dequeued),
            witnessed: false,
        };
        for call in 0..3 {
            if call == 1 {
                consumer.inactive(&mut native);
            }
            let result = consumer.capture(&mut native, &mut converter);
            assert!(!result.replayed);
            let image = result.image.unwrap().unwrap();
            fs::write(
                Path::new(&directory).join(format!("cycle-{cycle}-call-{call}.bgr")),
                image.pixels(),
            )
            .unwrap();
            let serial = known_bgr(&image);
            held.push((image, serial));
        }
        assert!(
            converter.witnessed,
            "actual native callback progress at conversion entry"
        );
        let live = live_tasks();
        if cycle == 0 {
            task_snapshot(Path::new(&directory), "first-live", &task_ids());
        }
        native.close();
        consumer.reset_after_close();
        let evidence = native.evidence();
        assert_eq!(
            (
                evidence.fault,
                evidence.queue_error,
                evidence.core_disconnect_error
            ),
            (0, 0, 0),
            "{evidence:?}"
        );
        assert_eq!(evidence.dequeued, evidence.returned, "{evidence:?}");
        assert_eq!(
            (evidence.copies, evidence.allocated),
            (3, 1),
            "conversion releases raw snapshots, permitting unique cache reuse"
        );
        eprintln!("native OpenCV closed cycle={cycle} callback_progress=1 evidence={evidence:?}");
        drop(native);
        drop(balance_init);
        assert_resources(Path::new(&directory), cycle, fds, &baseline, &live);
        assert_eq!(adapter.owner_count(), u64::try_from(held.len()).unwrap());
        for (image, serial) in &held {
            assert_eq!(known_bgr(image), *serial);
        }
    }
    assert_eq!(adapter.owner_count(), 60);
    drop(held);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust PipeWire OpenCV cycles=20 captures=60 retained=60 conversion_unlock_witnesses=20 passed"
    );
}
