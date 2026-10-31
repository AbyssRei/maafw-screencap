// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Controlled NULL-data real MemFd stream. Native consumer behavior is intact;
//! only the isolated preload hides/restores actual dequeued data pointers.
use super::*;
use std::ffi::c_void;

type Arm = unsafe extern "C" fn() -> i32;
type Read = unsafe extern "C" fn(u32) -> u64;
pub(super) struct Probe {
    arm: Arm,
    read: Read,
    disarm: Arm,
}
fn symbol(name: &CStr) -> *mut c_void {
    // SAFETY: Static NUL-terminated name; the explicit preload remains loaded
    // throughout this isolated test process, including native callback drain.
    let pointer = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    assert!(!pointer.is_null(), "missing MemFd control {name:?}");
    // SAFETY: dladdr initializes this plain C output on successful lookup.
    let mut info: libc::Dl_info = unsafe { mem::zeroed() };
    // SAFETY: Valid loaded symbol and complete writable output record.
    assert_ne!(unsafe { libc::dladdr(pointer, &mut info) }, 0);
    assert!(!info.dli_fname.is_null());
    // SAFETY: dladdr returned this initialized NUL-terminated owner filename.
    let owner = unsafe { CStr::from_ptr(info.dli_fname) }.to_str().unwrap();
    assert_eq!(
        fs::canonicalize(owner).unwrap(),
        fs::canonicalize(env::var("MFS_P1_PIPEWIRE_MEMFD_PROBE_LIBRARY").unwrap()).unwrap()
    );
    pointer
}
impl Probe {
    pub fn new() -> Self {
        // SAFETY: Exact fixed-width signatures in memfd-probe.h, checked symbol
        // owners and process-long preload lifetime. No callback is registered.
        unsafe {
            Self {
                arm: mem::transmute::<*mut c_void, Arm>(symbol(c"mfs_pw_memfd_probe_arm")),
                read: mem::transmute::<*mut c_void, Read>(symbol(c"mfs_pw_memfd_probe_read")),
                disarm: mem::transmute::<*mut c_void, Arm>(symbol(c"mfs_pw_memfd_probe_disarm")),
            }
        }
    }
    pub fn arm(&self) {
        // SAFETY: No native stream/callback exists on this serial test channel.
        assert_eq!(unsafe { (self.arm)() }, 0);
    }
    pub fn finish(&self, cycle: usize, directory: &Path, evidence: &Evidence) {
        // SAFETY: Native close has stopped/drained/destroyed the stream. These
        // mutex-protected scalar reads retain no pointer or callback storage.
        let counts = unsafe { std::array::from_fn::<_, 4, _>(|i| (self.read)(i as u32)) };
        fs::write(
            directory.join(format!("probe-{cycle}.tsv")),
            format!(
                "cycle\tdequeued\trestored\tlive\terrors\tcopies\town_maps\tunmaps\n{cycle}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                counts[0], counts[1], counts[2], counts[3],
                evidence.copies, evidence.owned_maps, evidence.unmap_calls
            ),
        )
        .unwrap();
        // SAFETY: Same completed native lifetime; disarm releases no resource.
        assert_eq!(unsafe { (self.disarm)() }, 0);
        assert!(counts[0] >= 3, "{counts:?}");
        assert_eq!(counts, [evidence.dequeued, evidence.returned, 0, 0]);
        assert_eq!(evidence.owned_maps, 3, "{evidence:?}");
        assert_eq!(evidence.dma, DmaSyncSnapshot::default());
    }
}

#[test]
#[ignore = "explicit isolated native Linux MemFd dequeue intervention and original source oracle"]
fn maps_null_data_in_real_memfd_callbacks_and_restores_before_return() {
    run_raw_probe(false, false, true);
}
