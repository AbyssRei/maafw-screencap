// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fixed-source DMA-BUF READ/SHARED mapping and one-shot CPU access brackets.
//! U records sync failures but keeps the source's read/END selection. This is
//! private migration code, not the later SDK coherency/retry contract.
use super::{NativeError, spa};
use rustix::{
    fd::BorrowedFd,
    io::Errno,
    ioctl::{Setter, opcode},
    mm::{MapFlags, ProtFlags},
};
use std::{
    ffi::c_void,
    marker::PhantomData,
    ptr,
    rc::Rc,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

#[repr(C)]
struct CpuSync {
    flags: u64,
}
const READ: u64 = 1;
const START: u64 = 0;
const END: u64 = 4;
const SYNC: rustix::ioctl::Opcode = opcode::write::<CpuSync>(b'b', 0);

#[derive(Default)]
pub(crate) struct DmaSyncEvidence {
    borrowed: AtomicU64,
    starts: AtomicU64,
    ends: AtomicU64,
    start_error: AtomicI32,
    end_error: AtomicI32,
}
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DmaSyncSnapshot {
    pub borrowed: u64,
    pub starts: u64,
    pub ends: u64,
    pub start_error: i32,
    pub end_error: i32,
}
fn first(error: &AtomicI32, code: i32) {
    if code != 0 {
        let _ = error.compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
}
fn increment(counter: &AtomicU64, error: &AtomicI32) {
    if counter
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .is_err()
    {
        first(error, Errno::OVERFLOW.raw_os_error());
    }
}
impl DmaSyncEvidence {
    pub fn snapshot(&self) -> DmaSyncSnapshot {
        DmaSyncSnapshot {
            borrowed: self.borrowed.load(Ordering::Acquire),
            starts: self.starts.load(Ordering::Acquire),
            ends: self.ends.load(Ordering::Acquire),
            start_error: self.start_error.load(Ordering::Acquire),
            end_error: self.end_error.load(Ordering::Acquire),
        }
    }
    pub fn borrowed_pointer(&self) {
        // Source returns an existing pointer before its mapping/sync branches.
        increment(&self.borrowed, &self.start_error);
    }
    fn access(&self, fd: BorrowedFd<'_>, start: bool) -> bool {
        let (calls, error) = if start {
            (&self.starts, &self.start_error)
        } else {
            (&self.ends, &self.end_error)
        };
        increment(calls, error);
        // SAFETY: _IOW('b',0,struct dma_buf_sync) reads this complete initialized
        // C-layout u64 flag field. The live native lease retains its FD, and the
        // operation consumes neither FD nor pointer. Native ABI is tested.
        let request = unsafe {
            Setter::<SYNC, CpuSync>::new(CpuSync {
                flags: READ | if start { START } else { END },
            })
        };
        // SAFETY: Same verified ioctl/input ABI and retained FD. U performs one
        // attempt; P3's EINTR/EAGAIN retry and failure rejection are separate.
        match unsafe { rustix::ioctl::ioctl(fd, request) } {
            Ok(()) => true,
            Err(code) => {
                first(error, code.raw_os_error());
                false
            }
        }
    }
}
pub(crate) struct DmaBufMap<'a> {
    pointer: *mut c_void,
    length: usize,
    fd: BorrowedFd<'a>,
    started: bool,
    sync: &'a DmaSyncEvidence,
    unmap_error: &'a AtomicI32,
    unmap_calls: &'a AtomicU64,
    _lease: PhantomData<(&'a spa::sys::spa_data, Rc<()>)>,
}
impl<'a> DmaBufMap<'a> {
    /// # Safety
    /// Initialized native data and a representable nonnegative FD stay live on
    /// this channel. Provider writes have completed and no producer modifies,
    /// truncates or reuses storage until all reads, sync and unmap complete.
    /// `required` and `length` are checked target span and source stride*height.
    /// U deliberately retains the mapped pointer after START error; its error
    /// evidence must not be described as SDK-valid CPU coherency. A CPU sync
    /// operation alone is not proof that an asynchronous producer has finished.
    pub unsafe fn new(
        data: &'a spa::sys::spa_data,
        required: usize,
        length: usize,
        sync: &'a DmaSyncEvidence,
        unmap_error: &'a AtomicI32,
        unmap_calls: &'a AtomicU64,
    ) -> Result<Self, NativeError> {
        let error = |operation, code: Errno| NativeError {
            operation,
            code: code.raw_os_error(),
        };
        if data.type_ != spa::sys::SPA_DATA_DmaBuf || !data.data.is_null() {
            return Err(error("owned DMA-BUF kind/pointer", Errno::INVAL));
        }
        if data.mapoffset != 0 {
            return Err(error("owned DMA-BUF nonzero mapoffset", Errno::INVAL));
        }
        let descriptor = i32::try_from(data.fd)
            .map_err(|_| error("owned DMA-BUF descriptor width", Errno::OVERFLOW))?;
        if descriptor < 0 {
            return Err(error("owned DMA-BUF descriptor", Errno::BADF));
        }
        if required == 0 || required > length || isize::try_from(length).is_err() {
            return Err(error("owned DMA-BUF readable span", Errno::INVAL));
        }
        // SAFETY: Caller retains this live native descriptor for the complete
        // lease; this owner borrows it and never closes or transfers it.
        let fd = unsafe { BorrowedFd::borrow_raw(descriptor) };
        let stat = rustix::fs::fstat(fd).map_err(|code| error("owned DMA-BUF fstat", code))?;
        let size = u64::try_from(stat.st_size)
            .map_err(|_| error("owned DMA-BUF negative file size", Errno::INVAL))?;
        if u64::try_from(required)
            .map_err(|_| error("owned DMA-BUF span width", Errno::OVERFLOW))?
            > size
        {
            return Err(error("owned DMA-BUF short file", Errno::INVAL));
        }
        // SAFETY: Fresh read-only shared mapping with source offset zero and
        // checked stride*height. Native lease excludes writes/truncation. Only
        // target rows in the separately verified readable span may be borrowed.
        let pointer = unsafe {
            rustix::mm::mmap(
                ptr::null_mut(),
                length,
                ProtFlags::READ,
                MapFlags::SHARED,
                fd,
                0,
            )
        }
        .map_err(|code| error("owned DMA-BUF mmap", code))?;
        let started = sync.access(fd, true);
        Ok(Self {
            pointer,
            length,
            fd,
            started,
            sync,
            unmap_error,
            unmap_calls,
            _lease: PhantomData,
        })
    }
    pub fn pointer(&self) -> *const u8 {
        self.pointer.cast()
    }
    #[cfg(test)]
    pub fn abi() -> [u64; 7] {
        [
            std::mem::size_of::<CpuSync>() as u64,
            std::mem::align_of::<CpuSync>() as u64,
            std::mem::offset_of!(CpuSync, flags) as u64,
            SYNC as u64,
            READ,
            START,
            END,
        ]
    }
}
impl Drop for DmaBufMap<'_> {
    fn drop(&mut self) {
        if self.started {
            self.sync.access(self.fd, false);
        }
        // SAFETY: This unique owner holds the complete fresh mapping; CPU row
        // borrows ended before drop. END precedes unmap even when END fails.
        let result = unsafe { rustix::mm::munmap(self.pointer, self.length) };
        first(
            self.unmap_error,
            result.err().map_or(0, Errno::raw_os_error),
        );
        increment(self.unmap_calls, self.unmap_error);
    }
}
