// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use std::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy)]
struct Reject;

// SAFETY: This allocator never creates a block, so it cannot invalidate or
// mismatch an allocation. It deterministically exercises recoverable errors.
unsafe impl Allocator for Reject {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Err(AllocError)
    }

    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
        unreachable!("Reject never returns an allocation")
    }
}

struct CountDrop<'a>(&'a AtomicUsize);
impl Drop for CountDrop<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn fallible_box_and_arc_preserve_standard_ownership() {
    let boxed = Box::try_new(42_u32).unwrap();
    assert_eq!(*boxed, 42);
    let shared = Arc::try_new(*boxed).unwrap();
    let weak = Arc::downgrade(&shared);
    let address = Arc::as_ptr(&shared) as usize;
    std::thread::spawn(move || {
        assert_eq!(Arc::as_ptr(&shared) as usize, address);
        assert_eq!(*shared, 42);
    })
    .join()
    .unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn drops_constructor_inputs_once_when_box_allocation_fails() {
    let drops = AtomicUsize::new(0);
    let result = Box::try_new_in(CountDrop(&drops), Reject);
    assert!(result.is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn drops_constructor_inputs_once_when_arc_allocation_fails() {
    let drops = AtomicUsize::new(0);
    let result = Arc::try_new_in(CountDrop(&drops), Reject);
    assert!(result.is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_impossible_buffer_capacity_without_losing_existing_data() {
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(3).unwrap();
    buffer.extend_from_slice(&[1_u8, 2, 3]);
    assert!(buffer.try_reserve_exact(usize::MAX).is_err());
    assert_eq!(buffer, [1, 2, 3]);
}
