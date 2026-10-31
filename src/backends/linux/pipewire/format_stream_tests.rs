// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Real native negotiation/raw rows plus the original post-unlock conversion.
use super::*;

struct Audit<'a> {
    adapter: &'a Adapter,
    directory: &'a Path,
    format: Format,
    raw: [Vec<u8>; 2],
    bgr: [Vec<u8>; 2],
    conversions: usize,
    reuses: usize,
}
impl Audit<'_> {
    fn save(&self, extension: &str, bytes: &[u8]) {
        let index = self.conversions + self.reuses;
        fs::write(
            self.directory.join(format!(
                "cycle-{}-call-{}.{extension}",
                index / 3,
                index % 3
            )),
            bytes,
        )
        .unwrap();
    }
    fn phase(&self, raw: &RawFrame) -> usize {
        assert_eq!(
            (raw.width, raw.height, raw.stride, raw.format),
            (28, 24, 28 * self.format.channels().unwrap(), self.format)
        );
        self.raw
            .iter()
            .position(|v| v == raw.pixels())
            .expect("actual native raw rows differ from both fixture phases")
    }
}
impl<'a> Converter<RawFrame> for Audit<'a> {
    type Image = (RawFrame, Bgr<'a>, usize);
    type Error = AdapterError;
    fn convert(
        &mut self,
        raw: Option<RawFrame>,
        mode: Conversion,
    ) -> Result<Option<Self::Image>, Self::Error> {
        assert_eq!(self.format.conversion(), Some(Some(mode)));
        let raw = raw.ok_or(AdapterError::InvalidLayout)?;
        self.save("raw", raw.pixels());
        let phase = self.phase(&raw);
        // Consumer has released its native lock before this exact call. No
        // added callback-progress wait changes this stream's capture cadence.
        let image = self.adapter.convert(&raw, mode)?;
        self.save("bgr", image.pixels());
        assert_eq!(image.pixels(), self.bgr[phase]);
        self.conversions += 1;
        Ok(Some((raw, Bgr::Converted(image), phase)))
    }
    fn reuse(&mut self, raw: Option<RawFrame>) -> Option<Self::Image> {
        assert_eq!(self.format, Format::Bgr);
        let raw = raw?;
        self.save("raw", raw.pixels());
        let phase = self.phase(&raw);
        let image = Bgr::Reused(raw.clone());
        self.save("bgr", image.pixels());
        assert_eq!(
            raw.pixels().as_ptr(),
            image.pixels().as_ptr(),
            "BGR shares the same CPU allocation"
        );
        assert_eq!(image.pixels(), self.bgr[phase]);
        self.reuses += 1;
        Some((raw, image, phase))
    }
}

#[test]
#[ignore = "explicit native owned seven-format producer and original/OpenCV oracle"]
fn captures_negotiated_format_rows_and_retains_raw_and_bgr_after_shutdown() {
    let name = env::var("MFS_P1_PIPEWIRE_STREAM_FORMAT").unwrap();
    let format = match name.as_str() {
        "RGB" => Format::Rgb,
        "RGBA" => Format::Rgba,
        "RGBx" => Format::Rgbx,
        "BGR" => Format::Bgr,
        "BGRA" => Format::Bgra,
        "BGRx" => Format::Bgrx,
        "YUY2" => Format::Yuy2,
        _ => panic!("unknown explicit format"),
    };
    let root = env::var("MFS_P1_PIPEWIRE_FORMAT_STREAM_ARTIFACTS").unwrap();
    let root = Path::new(&root);
    let directory = root.join("rust");
    fs::create_dir_all(&directory).unwrap();
    let adapter = Adapter::open();
    assert_eq!(adapter.owner_count(), 0);
    let channels = format.channels().unwrap();
    let raw = std::array::from_fn(|phase| {
        let padded = fs::read(root.join(format!("source/oracle/phase-{phase}.raw"))).unwrap();
        assert_eq!(padded.len(), (28 * channels + 12) * 24);
        padded
            .chunks_exact(28 * channels + 12)
            .flat_map(|row| row[..28 * channels].iter().copied())
            .collect()
    });
    let bgr = std::array::from_fn(|phase| {
        fs::read(root.join(format!("source/oracle/phase-{phase}.bgr"))).unwrap()
    });
    let mut audit = Audit {
        adapter: &adapter,
        directory: &directory,
        format,
        raw,
        bgr,
        conversions: 0,
        reuses: 0,
    };
    let node = env::var("MFS_P0_PIPEWIRE_NODE_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert_eq!(
        Path::new(&env::var("PIPEWIRE_RUNTIME_DIR").unwrap())
            .join(env::var("PIPEWIRE_REMOTE").unwrap()),
        Path::new(&env::var("MFS_P0_PIPEWIRE_SOCKET").unwrap())
    );
    let mut held = Vec::new();
    held.try_reserve_exact(60).unwrap();
    let mut seen = [0; 2];
    let mut rows = String::from("cycle\tcall\tphase\n");
    // Warm native conversion outside fd/task accounting for this format.
    if let Some(mode) = format.conversion().unwrap() {
        let mut cache = RawCache::default();
        let bytes = &audit.raw[0];
        let layout =
            Layout::checked(28, 24, format, u32::try_from(bytes.len()).unwrap(), None).unwrap();
        // SAFETY: Complete initialized tight fixture rows cover the checked
        // layout; RawCache allocates independent storage with no source alias.
        unsafe { cache.copy_native(bytes.as_ptr(), layout) }.unwrap();
        drop(adapter.convert(&cache.snapshot().unwrap(), mode).unwrap());
    }
    let fds = count("/proc/self/fd");
    let baseline = task_ids();
    for cycle in 0..20 {
        let native = Native::new().unwrap();
        let balance = BalanceInit;
        let mut native = native;
        native.init_daemon(node).unwrap();
        let mut consumer = Consumer::default();
        for call in 0..3 {
            if call == 1 {
                consumer.inactive(&mut native);
            }
            let result = consumer.capture(&mut native, &mut audit);
            assert!(!result.replayed);
            let (raw, image, phase) = result.image.unwrap().unwrap();
            rows.push_str(&format!("{cycle}\t{call}\t{phase}\n"));
            fs::write(directory.join("calls.tsv"), &rows).unwrap();
            seen[phase] += 1;
            held.push((raw, image, phase));
        }
        let live = live_tasks();
        native.close();
        consumer.reset_after_close();
        assert!(!native.connected());
        let e = native.evidence();
        fs::write(
            directory.join(format!("evidence-{cycle}.txt")),
            format!("{e:#?}\n"),
        )
        .unwrap();
        assert_eq!(
            (
                e.fault,
                e.queue_error,
                e.core_disconnect_error,
                e.mapping_error,
                e.unmap_error
            ),
            (0, 0, 0, 0, 0),
            "{e:?}"
        );
        assert_eq!(e.dequeued, e.returned, "{e:?}");
        assert_eq!(
            (e.copies, e.allocated),
            (3, 3),
            "retained raw CPU owners: {e:?}"
        );
        assert_eq!(e.owned_maps, e.unmap_calls, "{e:?}");
        assert_eq!(
            e.dma,
            DmaSyncSnapshot::default(),
            "ordinary mapped stream: {e:?}"
        );
        drop(native);
        drop(balance);
        assert_resources(&directory, cycle, fds, &baseline, &live);
        for (raw, image, phase) in &held {
            assert_eq!(audit.phase(raw), *phase);
            assert_eq!(image.pixels(), audit.bgr[*phase]);
        }
    }
    assert!(
        seen.iter().all(|n| *n > 0),
        "both producer phases must reach the consumer: {seen:?}"
    );
    assert_eq!(
        (audit.conversions, audit.reuses),
        if format == Format::Bgr {
            (0, 60)
        } else {
            (60, 0)
        }
    );
    for (i, (raw, image, phase)) in held.iter().enumerate() {
        for (extension, bytes) in [("raw", raw.pixels()), ("bgr", image.pixels())] {
            fs::write(
                directory.join(format!("cycle-{}-call-{}-held.{extension}", i / 3, i % 3)),
                bytes,
            )
            .unwrap();
        }
        assert_eq!(audit.phase(raw), *phase);
    }
    drop(held);
    drop(audit);
    assert_eq!(adapter.owner_count(), 0);
    drop(adapter);
    assert_eq!(LOADER_CLEANUP_ERROR.load(Ordering::Acquire), 0);
    eprintln!(
        "native Rust PipeWire format={name} cycles=20 captures=60 retained_raw=60 retained_bgr=60 phases=2 passed"
    );
}
