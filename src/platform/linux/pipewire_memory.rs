// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Owned read-only MemFd mapping for the fixed source's no-data-pointer path.
//! Native buffer/FD ownership remains with its caller. DMA-BUF is not handled.
use super::{NativeError, spa};
use rustix::{
    fd::BorrowedFd,
    io::Errno,
    mm::{MapFlags, ProtFlags},
};
use std::{
    ffi::c_void,
    marker::PhantomData,
    ptr,
    rc::Rc,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

pub(crate) struct MemFdMap<'a> {
    pointer: *mut c_void,
    length: usize,
    cleanup_error: &'a AtomicI32,
    unmap_calls: &'a AtomicU64,
    // A mapped native lease is channel-local, unlike independent CPU frames.
    _lease: PhantomData<(&'a spa::sys::spa_data, Rc<()>)>,
}
fn error(operation: &'static str, code: Errno) -> NativeError {
    NativeError {
        operation,
        code: code.raw_os_error(),
    }
}
impl<'a> MemFdMap<'a> {
    /// # Safety
    /// `data` and its optional chunk are initialized native lease records. A
    /// nonnegative representable FD stays open and is not truncated/written by
    /// the producer until reads and destruction finish on the native channel.
    /// `required` is the checked target-row readable span; no reference to the
    /// complete mapping or non-target padding may be formed from this owner.
    pub unsafe fn new(
        data: &'a spa::sys::spa_data,
        required: usize,
        cleanup_error: &'a AtomicI32,
        unmap_calls: &'a AtomicU64,
    ) -> Result<Self, NativeError> {
        if data.type_ != spa::sys::SPA_DATA_MemFd || !data.data.is_null() {
            return Err(error("owned MemFd mapping kind/pointer", Errno::INVAL));
        }
        if data.mapoffset != 0 {
            // U maps at zero. Do not silently introduce P3 offset semantics or
            // expose another region of the native allocation as target pixels.
            return Err(error("owned MemFd nonzero mapoffset", Errno::INVAL));
        }
        let descriptor = i32::try_from(data.fd)
            .map_err(|_| error("owned MemFd descriptor width", Errno::OVERFLOW))?;
        if descriptor < 0 {
            return Err(error("owned MemFd descriptor", Errno::BADF));
        }
        let capacity = if data.maxsize > 0 {
            data.maxsize
        } else {
            // SAFETY: The caller retains the initialized optional native chunk.
            unsafe { data.chunk.as_ref() }.map_or(0, |chunk| chunk.size)
        };
        let length = usize::try_from(capacity)
            .map_err(|_| error("owned MemFd mapping length", Errno::OVERFLOW))?;
        if required == 0 || required > length || isize::try_from(length).is_err() {
            return Err(error("owned MemFd readable span", Errno::INVAL));
        }
        // SAFETY: The lease retains the live descriptor until guard destruction;
        // BorrowedFd never transfers ownership or closes the native buffer FD.
        let fd = unsafe { BorrowedFd::borrow_raw(descriptor) };
        let stat = rustix::fs::fstat(fd).map_err(|code| error("owned MemFd fstat", code))?;
        let file_length = u64::try_from(stat.st_size)
            .map_err(|_| error("owned MemFd negative file size", Errno::INVAL))?;
        if u64::try_from(required).map_err(|_| error("owned MemFd span width", Errno::OVERFLOW))?
            > file_length
        {
            return Err(error("owned MemFd short file", Errno::INVAL));
        }
        // Preserve source PROT_READ, MAP_PRIVATE, offset zero and full declared
        // map length. Only target rows inside the verified file span are read;
        // trailing mapped padding need not be part of that readable span.
        // SAFETY: Null asks the OS for a fresh mapping, with no Rust aliases.
        // The lease guarantees stable file size/content during CPU access.
        let pointer = unsafe {
            rustix::mm::mmap(
                ptr::null_mut(),
                length,
                ProtFlags::READ,
                MapFlags::PRIVATE,
                fd,
                0,
            )
        }
        .map_err(|code| error("owned MemFd mmap", code))?;
        Ok(Self {
            pointer,
            length,
            cleanup_error,
            unmap_calls,
            _lease: PhantomData,
        })
    }
    pub fn pointer(&self) -> *const u8 {
        self.pointer.cast()
    }
}
impl Drop for MemFdMap<'_> {
    fn drop(&mut self) {
        // SAFETY: This non-cloneable owner holds the complete fresh OS mapping.
        // Caller pixel borrows end before drop; no mapping pointer is published.
        // A valid OS mapping at address zero is also released (copy rejects null).
        let result = unsafe { rustix::mm::munmap(self.pointer, self.length) };
        let code = result.err().map_or(0, Errno::raw_os_error);
        if code != 0 {
            let _ =
                self.cleanup_error
                    .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
        }
        if self
            .unmap_calls
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .is_err()
        {
            let _ = self.cleanup_error.compare_exchange(
                0,
                Errno::OVERFLOW.raw_os_error(),
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}
