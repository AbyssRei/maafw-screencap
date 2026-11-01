// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual INVALID_CALL while another native caller still owns an acquired frame.
use super::super::*;
use super::native_map::save;
use crate::{
    backends::windows::parity_adapter::{Adapter, Converter, Image},
    platform::windows::desktop_dup::map_probe,
};
use std::{error::Error as StdError, fs::File, io::Write};

struct ObservedConverter<'a>(Converter<'a>);
impl BgraConverter for ObservedConverter<'_> {
    type Image = Image;
    type Error = i32;
    fn bgra_to_bgr(&mut self, view: BgraView<'_>) -> Result<Image, i32> {
        if map_probe::conversion() {
            return Err(3);
        }
        self.0.bgra_to_bgr(view)
    }
}
struct HeldFrame {
    duplication: IDXGIOutputDuplication,
    resource: Option<IDXGIResource>,
    owned: bool,
    _caller: PhantomData<Rc<()>>,
}
impl HeldFrame {
    fn open(duplication: IDXGIOutputDuplication) -> Result<Self, Box<dyn StdError>> {
        let mut owner = Self {
            duplication,
            resource: None,
            owned: false,
            _caller: PhantomData,
        };
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut raw = std::ptr::null_mut();
        // SAFETY: Live independently retained native duplication, same caller,
        // no outstanding acquisition yet, and exact initialized output slots.
        let code = unsafe {
            (owner.duplication.vtable().AcquireNextFrame)(
                owner.duplication.as_raw(),
                2000,
                &mut info,
                &mut raw,
            )
        };
        owner.owned = code.is_ok();
        if code.is_ok() && !raw.is_null() {
            // SAFETY: The successful native call transfers this resource ref.
            owner.resource = Some(unsafe { IDXGIResource::from_raw(raw) });
        }
        if code != HRESULT(0) || owner.resource.is_none() {
            return Err(format!("native held-frame precondition {code:?}").into());
        }
        Ok(owner)
    }
    fn close(&mut self) -> Option<HRESULT> {
        let code = if self.owned {
            self.owned = false;
            // SAFETY: Exactly one successful acquisition owned by this test
            // guard on this caller, distinct from the failed capture's attempt.
            Some(unsafe { (self.duplication.vtable().ReleaseFrame)(self.duplication.as_raw()) })
        } else {
            None
        };
        self.resource.take();
        code
    }
}
impl Drop for HeldFrame {
    fn drop(&mut self) {
        self.close();
    }
}
#[test]
#[ignore = "explicit original/Rust real held-acquisition rejection, caller ownership and single recovery records"]
fn preserves_caller_acquisition_when_native_rejects_capture_and_recovers_after_release()
-> Result<(), Box<dyn StdError>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("MFS_DESKTOP_ACQUIRE_ROOT").ok_or("missing held-acquisition root")?,
    );
    assert!(root.is_absolute());
    assert_eq!(
        std::fs::read_to_string(root.join("source/acquire.tsv"))?
            .lines()
            .count(),
        41,
        "complete original held-acquisition matrix required"
    );
    let directory = root.join("rust");
    std::fs::create_dir(&directory)?;
    let create = |name: &str| {
        File::options()
            .write(true)
            .create_new(true)
            .open(directory.join(name))
    };
    let mut rows = create("acquire.tsv")?;
    let mut held_rows = create("held.tsv")?;
    let mut states = create("state.tsv")?;
    writeln!(
        rows,
        "cycle\tphase\toutcome\torder\tacquires\tcopies\tmaps\tunmaps\treleases\tconversions\tacquired\treleased\ttimeout\tpreserved\tmarker\tpitch\tthread\tcontext\tresource\tduplication"
    )?;
    writeln!(
        held_rows,
        "cycle\tprepared\trejected\tmanual_release\tafter_release\tthread\tresource\tduplication"
    )?;
    writeln!(
        states,
        "cycle\twidth\theight\tstaging\tduplication\tthread\tcopied_recovery"
    )?;
    let adapter = Adapter::load()?;
    assert_eq!(adapter.image_owners()?, 0);
    let mut saved = Vec::new();
    saved.try_reserve_exact(40)?;
    for cycle in 0_u32..20 {
        let mut fixture = adapter.render_fixture(96, 64)?;
        fixture.start_inside_desktop_updates()?;
        let thread = fixture.thread_info()?[1];
        let geometry = fixture.geometry([0; 6])?;
        assert_eq!(&geometry[14..16], &[96, 64]);
        let mut capture = DesktopDupCapture::new(fixture.hwnd());
        let mut converter = ObservedConverter(adapter.converter(0));
        let warm = capture
            .capture(&mut converter)
            .image
            .map_err(|e| format!("held-acquisition warm {e:?}"))?;
        assert!(!capture.just_initialized && capture.readable.is_some());
        let coordinates = capture
            .output_coordinates()
            .map_err(|e| format!("held-acquisition output {e:?}"))?;
        let x = usize::try_from(geometry[16].checked_sub(i64::from(coordinates[0])).unwrap())?;
        let y = usize::try_from(geometry[17].checked_sub(i64::from(coordinates[1])).unwrap())?;
        save(&directory, &format!("{cycle}-warm"), &warm, x, y)?;
        saved.push((format!("{cycle}-warm-held"), warm, x, y));
        let duplication = capture.duplication.as_ref().unwrap().clone();
        let staging = capture.readable.as_ref().unwrap().as_raw() as usize;
        let dup_address = duplication.as_raw() as usize;
        let (width, height) = (capture.description.Width, capture.description.Height);
        let mut held = HeldFrame::open(duplication)?;
        let resource = held.resource.as_ref().unwrap().as_raw() as usize;
        assert_ne!(resource, staging);
        let guard = map_probe::Guard::begin(0);
        let rejected = capture.capture(&mut converter);
        let trace = guard.trace();
        drop(guard);
        assert!(
            matches!(rejected.image, Err(Error::Capture(CaptureError::Native("AcquireNextFrame", code))) if code == DXGI_ERROR_INVALID_CALL)
        );
        assert_eq!(
            (
                trace.order,
                trace.acquires,
                trace.copies,
                trace.maps,
                trace.unmaps,
                trace.releases,
                trace.conversions
            ),
            (1, 1, 0, 0, 0, 0, 0)
        );
        assert_eq!(trace.acquired, DXGI_ERROR_INVALID_CALL.0);
        assert_eq!(trace.timeout, 2000);
        assert!(!trace.violation && !trace.mapped && !trace.held);
        assert_eq!(
            (trace.context, trace.resource, trace.duplication),
            (0, 0, dup_address)
        );
        assert_eq!(u64::from(trace.thread), thread);
        assert_eq!(
            (rejected.evidence.sample_count, rejected.evidence.warmups),
            (1, 0)
        );
        let sample = &rejected.evidence.samples[0];
        assert_eq!(sample.acquire, Some(DXGI_ERROR_INVALID_CALL));
        assert_eq!(sample.release, None);
        assert!(sample.frame.is_none());
        assert_eq!(
            (sample.pitch, sample.copied_bytes, sample.failure),
            (0, 0, Some("AcquireNextFrame"))
        );
        assert!(!capture.just_initialized);
        assert_eq!(
            capture.duplication.as_ref().unwrap().as_raw() as usize,
            dup_address
        );
        assert_eq!(
            capture.readable.as_ref().unwrap().as_raw() as usize,
            staging
        );
        assert_eq!(
            (capture.description.Width, capture.description.Height),
            (width, height)
        );
        let released = held.close().ok_or("caller held-frame close was missing")?;
        // SAFETY: Deliberate extra release through the live interface after the
        // guard's single successful release; the native API must now reject it.
        let after = unsafe { (held.duplication.vtable().ReleaseFrame)(held.duplication.as_raw()) };
        writeln!(
            held_rows,
            "{cycle}\t0\t{}\t{}\t{}\t{}\t{resource}\t{dup_address}",
            trace.acquired, released.0, after.0, trace.thread
        )?;
        held_rows.flush()?;
        assert_eq!(released, HRESULT(0));
        assert_eq!(after, DXGI_ERROR_INVALID_CALL);
        drop(held);
        writeln!(
            rows,
            "{cycle}\trejected\t0\t1\t1\t0\t0\t0\t0\t0\t{}\t0\t2000\t1\t-1\t0\t{}\t0\t0\t{dup_address}",
            trace.acquired, trace.thread
        )?;
        rows.flush()?;
        let guard = map_probe::Guard::begin(0);
        let attempt = capture.capture(&mut converter);
        let recovery = guard.trace();
        drop(guard);
        assert!(!recovery.violation && !recovery.mapped && !recovery.held);
        assert_eq!(
            (
                recovery.order,
                recovery.acquires,
                recovery.copies,
                recovery.maps,
                recovery.unmaps,
                recovery.releases,
                recovery.conversions
            ),
            (123456, 1, 1, 1, 1, 1, 1)
        );
        assert_eq!(
            (
                recovery.acquired,
                recovery.mapped_result,
                recovery.released,
                recovery.subresource,
                recovery.timeout
            ),
            (0, 0, 0, 0, 2000)
        );
        assert_ne!(recovery.context, 0);
        assert_eq!(
            (recovery.resource, recovery.duplication),
            (staging, dup_address)
        );
        assert_eq!(u64::from(recovery.thread), thread);
        assert_eq!(
            (attempt.evidence.sample_count, attempt.evidence.warmups),
            (1, 0)
        );
        let copied = usize::try_from(width)?
            .checked_mul(usize::try_from(height)?)
            .unwrap()
            .checked_mul(4)
            .unwrap();
        assert_eq!(attempt.evidence.samples[0].copied_bytes, copied);
        assert_eq!(attempt.evidence.samples[0].release, Some(HRESULT(0)));
        assert!(!capture.just_initialized);
        assert_eq!(
            capture.readable.as_ref().unwrap().as_raw() as usize,
            staging
        );
        assert_eq!(
            capture.duplication.as_ref().unwrap().as_raw() as usize,
            dup_address
        );
        let image = attempt
            .image
            .map_err(|e| format!("held-acquisition single recovery {e:?}"))?;
        let marker = save(&directory, &format!("{cycle}-recovery"), &image, x, y)?;
        saved.push((format!("{cycle}-recovery-held"), image, x, y));
        writeln!(
            rows,
            "{cycle}\trecovery\t1\t123456\t1\t1\t1\t1\t1\t1\t{}\t{}\t2000\t1\t{marker}\t{}\t{}\t{}\t{}\t{}",
            recovery.acquired,
            recovery.released,
            recovery.pitch,
            recovery.thread,
            recovery.context,
            recovery.resource,
            recovery.duplication
        )?;
        rows.flush()?;
        writeln!(
            states,
            "{cycle}\t{width}\t{height}\t{staging}\t{dup_address}\t{}\t{copied}",
            recovery.thread
        )?;
        states.flush()?;
        capture.uninit();
        drop(capture);
        drop(fixture);
    }
    assert_eq!(saved.len(), 40);
    assert_eq!(adapter.image_owners()?, 40);
    for (name, image, x, y) in &saved {
        save(&directory, name, image, *x, *y)?;
    }
    drop(saved);
    assert_eq!(adapter.image_owners()?, 0);
    println!(
        "native Rust DesktopDup held_acquisition cycles=20 public_calls=60 observed_calls=40 native_rejections=20 caller_acquires=20 caller_releases=20 recovery=20 retained=40 owners_released=1 full_P1_acceptance=0 passed"
    );
    Ok(())
}
