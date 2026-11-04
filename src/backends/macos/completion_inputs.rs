// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! P1's MRC create references are released by invocation, not block disposal.
use std::sync::Mutex;

pub(super) struct CompletionInputs<T>(Mutex<Option<T>>);

impl<T> CompletionInputs<T> {
    pub(super) fn new(value: T) -> Self {
        Self(Mutex::new(Some(value)))
    }

    pub(super) fn release(&self) {
        let value = self
            .0
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        drop(value);
    }
}

impl<T> Drop for CompletionInputs<T> {
    fn drop(&mut self) {
        if let Some(value) = self
            .0
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner())
            .take()
        {
            // Fixed F releases these +1s ONLY inside the sample handler. A
            // discarded uninvoked block releases its copied references but
            // leaves the create references outstanding. Preserve that leak in
            // P1; product cleanup changes belong after baseline acceptance.
            std::mem::forget(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn completed_create_owners_release_before_the_block_copies() {
        let original = Arc::new(());
        let weak = Arc::downgrade(&original);
        let copied = original.clone();
        let inputs = CompletionInputs::new(original);
        inputs.release();
        inputs.release();
        assert_eq!(weak.strong_count(), 1);
        drop(inputs);
        assert_eq!(weak.strong_count(), 1);
        drop(copied);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn uninvoked_disposal_retains_the_source_create_owner() {
        let original = Arc::new(());
        let weak = Arc::downgrade(&original);
        let pointer = Arc::as_ptr(&original);
        let copied = original.clone();
        let inputs = CompletionInputs::new(original);
        drop(inputs);
        drop(copied);
        assert_eq!(weak.strong_count(), 1);
        // SAFETY: The known uninvoked owner above deliberately leaked exactly
        // one Arc +1. All block copies are gone; this test alone reclaims it.
        drop(unsafe { Arc::from_raw(pointer) });
        assert!(weak.upgrade().is_none());
    }

    #[cfg(all(target_os = "macos", feature = "macos-screen-capture-kit"))]
    #[test]
    #[ignore = "explicit native Foundation/Blocks ownership control, no capture"]
    fn native_block_disposal_preserves_uninvoked_create_references() {
        use block2::RcBlock;
        use objc2::{
            rc::{Retained, Weak},
            runtime::NSObject,
        };
        for invoke in [true, false] {
            let filter = NSObject::new();
            let config = NSObject::new();
            let filter_weak = Weak::from_retained(&filter);
            let config_weak = Weak::from_retained(&config);
            let filter_pointer = Retained::as_ptr(&filter).cast_mut();
            let config_pointer = Retained::as_ptr(&config).cast_mut();
            let copies = (filter.clone(), config.clone());
            let initial = CompletionInputs::new((filter, config));
            let block = RcBlock::new(move || {
                std::hint::black_box(&copies);
                initial.release();
            });
            let alias = block.clone();
            drop(block);
            if invoke {
                alias.call(());
            }
            assert!(filter_weak.load().is_some() && config_weak.load().is_some());
            drop(alias);
            assert_eq!(filter_weak.load().is_some(), !invoke);
            assert_eq!(config_weak.load().is_some(), !invoke);
            if !invoke {
                // SAFETY: Last native block alias has been synchronously
                // released without invocation. Each initial Retained leaked
                // exactly one +1 above; only this fixture now reclaims them.
                drop(unsafe { Retained::from_raw(filter_pointer) }.unwrap());
                drop(unsafe { Retained::from_raw(config_pointer) }.unwrap());
                assert!(filter_weak.load().is_none() && config_weak.load().is_none());
            }
        }
        println!(
            "native Rust macOS completion_inputs cases=2 invoked=1 uninvoked=1 heap_alias=2 native_weak_checks=10 fixture_reclaimed_refs=2 capture=0 full_P1_acceptance=0 passed"
        );
    }
}
