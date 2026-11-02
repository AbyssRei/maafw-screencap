// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! The complete private source-class owner, not an SDK Session or worker.
//! It moves only between completed, exclusive calls. Raw Driver/Resources and
//! native mapping guards retain their own stricter !Send/!Sync boundaries.
use super::{
    super::{Attempt, Consumer, Converter, Driver},
    Evidence, Native, NativeError, RawFrame,
};

pub(in crate::backends::linux) struct Capture {
    // No accessor, Driver impl, Deref or escape of this owner/its loop lock.
    driver: Native,
    consumer: Consumer,
}

// SAFETY: This is the source header's serialized cross-thread capture boundary,
// not a Send assertion for the raw driver. Every exposed mutating operation
// takes &mut self and completes its actual native loop-lock guards before it
// returns. Guard Drop also releases on Rust unwinding; conversion is invoked
// after unlock and is neither stored nor allowed to borrow native state.
//
// Callbacks and hooks live in stable Box allocations, not inside this movable
// value. Callbacks access State on the non-RT loop channel; consumer State
// access holds its native lock, with no borrow across timed_wait. Moves change
// no callback/data/hook addresses. At rest only independent CPU Arc<Vec> frames
// remain cached; no native map/buffer lease or opaque converter owner is kept.
//
// Stop/join is outside the loop lock and native hook/object destruction ends
// before callback storage is freed. Raw mappings never move with this owner.
// No method exposes the driver, a held guard, callbacks or resources, including
// on init failure, converter error or unwind. Native's raw lock method alone
// remains !Send. No Sync impl, concurrent operations or loop-thread reentry is
// authorized by this boundary. The unsafe supplied-FD contract remains intact.
unsafe impl Send for Capture {}

impl Capture {
    pub fn new() -> Result<Self, std::alloc::AllocError> {
        Ok(Self {
            driver: Native::new()?,
            consumer: Consumer::default(),
        })
    }

    /// # Safety
    /// Same unique live socket offer and unresolved-failure ownership contract
    /// as Native::with_fd. No independent close/reuse may race an init attempt.
    pub unsafe fn with_fd(fd: i32) -> Result<Self, std::alloc::AllocError> {
        Ok(Self {
            // SAFETY: Forward the exact private caller proof, without another
            // descriptor owner, duplicate or new transfer/recovery policy.
            driver: unsafe { Native::with_fd(fd) }?,
            consumer: Consumer::default(),
        })
    }

    fn at_rest(&self) {
        assert!(
            !self.driver.locked,
            "capture boundary retained a native lock"
        );
    }

    pub fn pending_source_fd(&self) -> Option<i32> {
        self.at_rest();
        self.driver.pending_source_fd()
    }

    pub fn connected(&self) -> bool {
        self.at_rest();
        self.driver.connected()
    }

    pub fn init_daemon(&mut self, node: u32) -> Result<(), NativeError> {
        self.at_rest();
        if !self.driver.connected() {
            // Source init closes stale state, including its timeout counter,
            // before setup. Already-connected init must keep that counter.
            self.consumer.reset_after_close();
        }
        let result = self.driver.init_daemon(node);
        self.at_rest();
        result
    }

    /// # Safety
    /// Preserve the original offer proof through this attempt, including after
    /// a previous native failure; a consumed old descriptor is not a new offer.
    pub unsafe fn init_source(&mut self, node: u32) -> Result<(), NativeError> {
        self.at_rest();
        if !self.driver.connected() {
            self.consumer.reset_after_close();
        }
        // SAFETY: Same caller proof as Native::init_source. The lower owner is
        // not exposed and complete setup/cleanup releases its loop guards.
        let result = unsafe { self.driver.init_source(node) };
        self.at_rest();
        result
    }

    pub fn inactive(&mut self) {
        self.at_rest();
        self.consumer.inactive(&mut self.driver);
        self.at_rest();
    }

    pub fn capture<C: Converter<RawFrame>>(
        &mut self,
        converter: &mut C,
    ) -> Attempt<C::Image, C::Error> {
        self.at_rest();
        let result = self.consumer.capture(&mut self.driver, converter);
        self.at_rest();
        result
    }

    pub fn close(&mut self) {
        self.at_rest();
        self.driver.close();
        self.consumer.reset_after_close();
        self.at_rest();
    }

    pub fn evidence(&self) -> Evidence {
        self.at_rest();
        self.driver.evidence()
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        // Complete native close first; Native's subsequent field Drop is an
        // idempotent empty close. Preserve the source's raw global-init policy.
        self.close();
    }
}
