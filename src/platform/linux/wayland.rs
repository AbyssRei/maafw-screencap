// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Owned native display plus the one missing queue getter needed to preserve
//! the source's recursive roundtrip. No public SDK or general async channel.
use std::{
    ffi::c_int, marker::PhantomData, os::unix::ffi::OsStrExt, path::Path, ptr::NonNull, rc::Rc,
};
use wayland_client::{Connection, Proxy, backend::Backend};
use wayland_sys::{client::*, ffi_dispatch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Error {
    pub operation: &'static str,
    pub code: i32,
}
impl Error {
    pub const fn new(operation: &'static str, code: i32) -> Self {
        Self { operation, code }
    }
    pub const fn allocation() -> Self {
        Self::new(
            "fallible Wlr allocation",
            rustix::io::Errno::NOMEM.raw_os_error(),
        )
    }
    fn last(operation: &'static str) -> Self {
        Self::new(
            operation,
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
        )
    }
}

struct Display(NonNull<wl_display>);
impl Drop for Display {
    fn drop(&mut self) {
        // SAFETY: This is the unique native connection owner, after every
        // temporary backend/Connection clone and the inner queue have gone.
        unsafe {
            ffi_dispatch!(
                wayland_client_handle(),
                wl_display_disconnect,
                self.0.as_ptr()
            );
        }
    }
}
pub(crate) struct Channel {
    // Declaration order also retains the display through backend destruction.
    connection: Option<Connection>,
    _display: Display,
    _channel: PhantomData<Rc<()>>,
}
impl Channel {
    pub fn connect(path: &Path) -> Result<Self, Error> {
        let bytes = path.as_os_str().as_bytes();
        if bytes.contains(&0) {
            return Err(Error::new(
                "Wayland socket contains NUL",
                rustix::io::Errno::INVAL.raw_os_error(),
            ));
        }
        let length = bytes.len().checked_add(1).ok_or(Error::new(
            "Wayland socket size",
            rustix::io::Errno::OVERFLOW.raw_os_error(),
        ))?;
        let mut name = Vec::new();
        name.try_reserve_exact(length)
            .map_err(|_| Error::allocation())?;
        name.extend_from_slice(bytes);
        name.push(0);
        // SAFETY: Checked NUL-terminated native path remains initialized here.
        // Use the original native connect entry, including its environment/FD
        // behavior, rather than replacing source selection with a Rust socket.
        let pointer = unsafe {
            ffi_dispatch!(
                wayland_client_handle(),
                wl_display_connect,
                name.as_ptr().cast()
            )
        };
        let display =
            Display(NonNull::new(pointer).ok_or_else(|| Error::last("wl_display_connect"))?);
        // SAFETY: This foreign-display backend does not disconnect the display.
        // Channel destroys all clones/objects and its queue before Display drop.
        let backend = unsafe { Backend::from_foreign_display(display.0.as_ptr()) };
        Ok(Self {
            connection: Some(Connection::from_backend(backend)),
            _display: display,
            _channel: PhantomData,
        })
    }
    /// # Safety
    /// No Connection/backend clone or object created from it may outlive this
    /// Channel. Access and all native dispatch remain on its sole owner channel.
    /// This restricted private borrow must never become an outward SDK handle.
    pub unsafe fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("live private channel")
    }
}
impl Drop for Channel {
    fn drop(&mut self) {
        drop(self.connection.take());
    }
}

unsafe extern "C" {
    // Exact private wayland-reentry.h opaque-pointer declaration. Native tests
    // consume the real returned queue through the maintained roundtrip binding.
    fn mfs_wayland_proxy_get_queue(proxy: *const wl_proxy) -> *mut wl_event_queue;
}
/// # Safety
/// Only call within a synchronous ObjectData::event on this live backend, while
/// wayland-backend's native dispatcher has installed its BACKEND TLS context.
/// There is no prepared read, exclusive Rust state borrow or state lock across
/// this call. The proxy/connection remain live on that same owner channel until
/// it returns. Recursive events obey those same invariants.
pub(crate) unsafe fn callback_roundtrip<P: Proxy>(
    backend: &Backend,
    proxy: &P,
) -> Result<c_int, Error> {
    let pointer = proxy.id().as_ptr();
    let display = backend.display_ptr();
    if pointer.is_null() || display.is_null() {
        return Err(Error::new(
            "recursive Wayland live object",
            rustix::io::Errno::INVAL.raw_os_error(),
        ));
    }
    let owner = proxy.backend().upgrade().ok_or(Error::new(
        "recursive Wayland proxy owner",
        rustix::io::Errno::INVAL.raw_os_error(),
    ))?;
    if owner.display_ptr() != display {
        return Err(Error::new(
            "recursive Wayland connection mismatch",
            rustix::io::Errno::INVAL.raw_os_error(),
        ));
    }
    // SAFETY: Retained live proxy, header-checked native getter; no ownership
    // transfer. Calling the real queue avoids reacquiring Rust's dispatch mutex.
    let queue = unsafe { mfs_wayland_proxy_get_queue(pointer) };
    if queue.is_null() {
        return Err(Error::new(
            "recursive Wayland queue",
            rustix::io::Errno::INVAL.raw_os_error(),
        ));
    }
    // SAFETY: This exact native queue belongs to the retained backend/proxy;
    // active outer ObjectData dispatch provides TLS for nested Rust callbacks.
    let result = unsafe {
        ffi_dispatch!(
            wayland_client_handle(),
            wl_display_roundtrip_queue,
            display,
            queue
        )
    };
    if result < 0 {
        Err(Error::last("wl_display_roundtrip_queue"))
    } else {
        Ok(result)
    }
}
