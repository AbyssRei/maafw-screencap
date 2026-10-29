// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Native resource ordering for the fixed PipeWire screenshot implementation.
//! Daemon and explicitly unsafe supplied-FD setup share source ordering. General
//! FD failure ownership and CPU mapping/DMA synchronization remain unfinished.
pub(crate) use pipewire::{spa, sys};
use std::{ffi::c_void, marker::PhantomData, ptr, rc::Rc};

#[path = "pipewire_memory.rs"]
mod memory;
pub(crate) use memory::MemFdMap;

#[path = "pipewire_dma.rs"]
mod dma;
pub(crate) use dma::{DmaBufMap, DmaSyncEvidence, DmaSyncSnapshot};

#[derive(Debug)]
pub(crate) struct NativeError {
    pub operation: &'static str,
    pub code: i32,
}
impl NativeError {
    fn last(operation: &'static str) -> Self {
        Self {
            operation,
            code: std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        }
    }
}

pub(crate) struct Resources {
    pub loop_: *mut sys::pw_thread_loop,
    pub context: *mut sys::pw_context,
    pub core: *mut sys::pw_core,
    pub stream: *mut sys::pw_stream,
    pub core_disconnect_error: i32,
    core_hook: Box<spa::sys::spa_hook>,
    stream_hook: Box<spa::sys::spa_hook>,
    core_registered: bool,
    stream_registered: bool,
    // No mobility claim follows from serialized consumer access.
    _channel: PhantomData<Rc<()>>,
}
impl Resources {
    pub fn new() -> Result<Self, std::alloc::AllocError> {
        // SAFETY: Empty native hook records have null links/callbacks and zero
        // private data. Native registration initializes them before removal.
        let core_hook = Box::try_new(unsafe { std::mem::zeroed() })?;
        // SAFETY: Same hook invariant; independent stable allocation.
        let stream_hook = Box::try_new(unsafe { std::mem::zeroed() })?;
        Ok(Self {
            loop_: ptr::null_mut(),
            context: ptr::null_mut(),
            core: ptr::null_mut(),
            stream: ptr::null_mut(),
            core_disconnect_error: 0,
            core_hook,
            stream_hook,
            core_registered: false,
            stream_registered: false,
            _channel: PhantomData,
        })
    }

    /// # Safety
    /// `data` must retain a stable callback owner until close finishes. Events
    /// must remain valid and contain callbacks matching that owner's discipline.
    /// Raw global pw_init must already be held independently of these objects.
    /// All calls occur serially on the owning consumer channel.
    /// If `input_fd` is nonnegative, it must be a valid uniquely offered socket.
    /// No independent owner may close it during or after a successful native
    /// transfer. A null core does not establish consumption on every protocol;
    /// the caller must resolve residual ownership after complete native teardown.
    pub unsafe fn create_source(
        &mut self,
        input_fd: &mut i32,
        data: *mut c_void,
        core_events: &'static sys::pw_core_events,
        stream_events: &'static sys::pw_stream_events,
    ) -> Result<(), NativeError> {
        // SAFETY: Caller holds global initialization and no loop is running yet.
        unsafe {
            self.loop_ = sys::pw_thread_loop_new(c"MaaScreencap".as_ptr(), ptr::null());
            if self.loop_.is_null() {
                return Err(NativeError::last("pw_thread_loop_new"));
            }
            self.context =
                sys::pw_context_new(sys::pw_thread_loop_get_loop(self.loop_), ptr::null_mut(), 0);
            if self.context.is_null() {
                return Err(NativeError::last("pw_context_new"));
            }
            self.core = if *input_fd >= 0 {
                sys::pw_context_connect_fd(self.context, *input_fd, ptr::null_mut(), 0)
            } else {
                sys::pw_context_connect(self.context, ptr::null_mut(), 0)
            };
            if self.core.is_null() {
                return Err(NativeError::last(if *input_fd >= 0 {
                    "pw_context_connect_fd"
                } else {
                    "pw_context_connect"
                }));
            }
            // Exact U transition: clear the stored number ONLY after a non-null
            // core. Source close does not restore it, and later init uses daemon
            // mode. No general null-result FD cleanup is inferred here.
            *input_fd = -1;
            let interface = self.core.cast::<spa::sys::spa_interface>();
            let methods = (*interface).cb.funcs.cast::<sys::pw_core_methods>();
            let add = (*methods).add_listener.ok_or(NativeError {
                operation: "pw_core_add_listener",
                code: -1,
            })?;
            let result = add(
                (*interface).cb.data,
                &mut *self.core_hook,
                core_events,
                data,
            );
            if result < 0 {
                return Err(NativeError {
                    operation: "pw_core_add_listener",
                    code: result,
                });
            }
            self.core_registered = true;
            let props = sys::pw_properties_new(
                c"media.type".as_ptr(),
                c"Video".as_ptr(),
                c"media.category".as_ptr(),
                c"Capture".as_ptr(),
                c"media.role".as_ptr(),
                c"Screen".as_ptr(),
                ptr::null::<std::ffi::c_char>(),
            );
            if props.is_null() {
                return Err(NativeError::last("pw_properties_new"));
            }
            // Native stream_new consumes props on both success and failure.
            self.stream = sys::pw_stream_new(self.core, c"MaaFramework Screencap".as_ptr(), props);
            if self.stream.is_null() {
                return Err(NativeError::last("pw_stream_new"));
            }
            sys::pw_stream_add_listener(self.stream, &mut *self.stream_hook, stream_events, data);
            self.stream_registered = true;
        }
        Ok(())
    }

    pub fn close(&mut self) {
        // SAFETY: No consumer operation races close. Stop joins the non-RT loop
        // without holding its lock, before hooks, callback data or objects can
        // be destroyed. Stable callback storage outlives all synchronous native
        // destruction notifications. Each native owner is released once.
        unsafe {
            if !self.loop_.is_null() {
                sys::pw_thread_loop_stop(self.loop_);
            }
            if self.stream_registered {
                spa::sys::spa_hook_remove(&mut *self.stream_hook);
                self.stream_registered = false;
            }
            if !self.stream.is_null() {
                sys::pw_stream_destroy(self.stream);
                self.stream = ptr::null_mut();
            }
            if self.core_registered {
                spa::sys::spa_hook_remove(&mut *self.core_hook);
                self.core_registered = false;
            }
            if !self.core.is_null() {
                let code = sys::pw_core_disconnect(self.core);
                if code < 0 && self.core_disconnect_error == 0 {
                    self.core_disconnect_error = code;
                }
                self.core = ptr::null_mut();
            }
            if !self.context.is_null() {
                sys::pw_context_destroy(self.context);
                self.context = ptr::null_mut();
            }
            if !self.loop_.is_null() {
                sys::pw_thread_loop_destroy(self.loop_);
                self.loop_ = ptr::null_mut();
            }
        }
    }
}
impl Drop for Resources {
    fn drop(&mut self) {
        self.close();
    }
}
