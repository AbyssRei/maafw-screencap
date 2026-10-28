// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
// Derived from fixed MaaFramework PseudoMinimizeHelper.{h,cpp}; see notices.
use super::native::*;
use std::{
    alloc::AllocError,
    marker::PhantomData,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// Non-owning OS identifier, never a pointer into Rust storage. Only the Win32
/// queries/style/ShowWindow operations in Api use it across threads, as in U.
/// The host owns the window; destruction is observed by IsWindow, not prevented.
#[derive(Clone, Copy)]
pub(crate) struct Window(HWND);
// SAFETY: Copying this opaque identifier transfers no native allocation or Rust
// reference. Api never dereferences it, accesses a DC, or destroys the window.
// The helper owner itself remains !Send/!Sync; its worker uses only these calls.
unsafe impl Send for Window {}
// SAFETY: The same non-owning identifier may be queried by caller and monitor.
// Win32 owns the target state; no Rust memory is aliased through this value.
unsafe impl Sync for Window {}

pub(crate) trait Api: Send + Sync + 'static {
    fn valid(&self, window: Window) -> bool;
    fn iconic(&self, window: Window) -> bool;
    fn foreground(&self, window: Window) -> bool;
    fn style(&self, window: Window) -> Result<isize, u32>;
    fn set_style(&self, window: Window, value: isize) -> Result<(), u32>;
    fn alpha(&self, window: Window) -> Result<u8, u32>;
    fn set_alpha(&self, window: Window, value: u8) -> Result<(), u32>;
    fn show(&self, window: Window, minimize: bool) -> bool;
    fn sleep(&self, duration: Duration);
}

pub(crate) struct System;
impl Api for System {
    fn valid(&self, window: Window) -> bool {
        // SAFETY: Win32 validates the opaque non-owning HWND.
        unsafe { IsWindow(window.0) != 0 }
    }
    fn iconic(&self, window: Window) -> bool {
        // SAFETY: Query only, without borrowing target memory.
        unsafe { IsIconic(window.0) != 0 }
    }
    fn foreground(&self, window: Window) -> bool {
        // SAFETY: Query only; returned HWND is compared, never dereferenced.
        unsafe { GetForegroundWindow() == window.0 }
    }
    fn style(&self, window: Window) -> Result<isize, u32> {
        // SAFETY: Fixed style index, opaque HWND. Read last error immediately,
        // before locking/recording evidence; zero is also a valid style value.
        unsafe {
            SetLastError(0);
            let value = GetWindowLongPtrW(window.0, GWL_EXSTYLE);
            let error = GetLastError();
            if value == 0 && error != 0 {
                Err(error)
            } else {
                Ok(value)
            }
        }
    }
    fn set_style(&self, window: Window, value: isize) -> Result<(), u32> {
        // SAFETY: Only extended styles are changed. Preserve the source's
        // zero/last-error distinction, including a previous style of zero.
        unsafe {
            SetLastError(0);
            let previous = SetWindowLongPtrW(window.0, GWL_EXSTYLE, value);
            let error = GetLastError();
            if previous == 0 && error != 0 {
                Err(error)
            } else {
                Ok(())
            }
        }
    }
    fn alpha(&self, window: Window) -> Result<u8, u32> {
        let (mut color, mut alpha, mut flags) = (0, 255, 0);
        // SAFETY: Initialized stack outputs cover exactly the native arguments.
        unsafe {
            if GetLayeredWindowAttributes(window.0, &mut color, &mut alpha, &mut flags) == 0 {
                Err(GetLastError())
            } else {
                Ok(alpha)
            }
        }
    }
    fn set_alpha(&self, window: Window, value: u8) -> Result<(), u32> {
        // SAFETY: Preserve U's color key 0 and LWA_ALPHA, even when the original
        // layered window had different flags/color-key semantics.
        unsafe {
            if SetLayeredWindowAttributes(window.0, 0, value, LWA_ALPHA as u32) == 0 {
                Err(GetLastError())
            } else {
                Ok(())
            }
        }
    }
    fn show(&self, window: Window, minimize: bool) -> bool {
        // SAFETY: Synchronous original ShowWindow call on the opaque target;
        // its return means previous visibility, not operation success.
        unsafe {
            ShowWindow(
                window.0,
                if minimize {
                    SW_MINIMIZE
                } else {
                    SW_SHOWNOACTIVATE
                },
            ) != 0
        }
    }
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    SaveStyle,
    SaveAlpha,
    ReadStyle,
    ApplyStyle,
    ApplyAlpha,
    RevertStyle,
    RevertAlpha,
}
impl Operation {
    fn index(self) -> usize {
        match self {
            Self::SaveStyle => 0,
            Self::SaveAlpha => 1,
            Self::ReadStyle => 2,
            Self::ApplyStyle => 3,
            Self::ApplyAlpha => 4,
            Self::RevertStyle => 5,
            Self::RevertAlpha => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub operation: Operation,
    pub code: u32,
}

/// Allocation-independent summary, not an unbounded call log. Native failures
/// retain their first/last identity and count globally and per operation, so
/// earlier errors cannot hide either restoration failure. A zero ShowWindow return is kept
/// separately because U logs it as failure although it means prior visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub first: Option<Failure>,
    pub last: Option<Failure>,
    pub failures: u64,
    pub first_by_operation: [Option<Failure>; 7],
    pub last_by_operation: [Option<Failure>; 7],
    pub counts_by_operation: [u64; 7],
    pub minimize_returned_zero: bool,
    pub worker_panicked: bool,
}

#[derive(Clone, Copy)]
struct Saved {
    style: isize,
    alpha: u8,
    layered: bool,
}

struct Shared<A> {
    window: Window,
    api: A,
    running: AtomicBool,
    pseudo: AtomicBool,
    saved: Mutex<Saved>,
    evidence: Mutex<Evidence>,
}

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // No guard spans a native call or sleep. Recover the fixed owner/evidence
    // state on poison so Drop can still attempt the source's restoration.
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

impl<A: Api> Shared<A> {
    fn begin(&self) -> bool {
        if self.running.load(Ordering::SeqCst) {
            return false;
        }
        self.save();
        self.running.store(true, Ordering::SeqCst);
        true
    }

    fn request_stop(&self) -> bool {
        self.running.swap(false, Ordering::SeqCst)
    }

    fn failure(&self, operation: Operation, code: u32) {
        let failure = Failure { operation, code };
        let mut evidence = locked(&self.evidence);
        evidence.first.get_or_insert(failure);
        evidence.last = Some(failure);
        evidence.failures = evidence.failures.saturating_add(1);
        let index = operation.index();
        evidence.first_by_operation[index].get_or_insert(failure);
        evidence.last_by_operation[index] = Some(failure);
        evidence.counts_by_operation[index] = evidence.counts_by_operation[index].saturating_add(1);
    }

    fn save(&self) {
        let style = self.api.style(self.window).unwrap_or_else(|code| {
            self.failure(Operation::SaveStyle, code);
            0
        });
        let layered = style & WS_EX_LAYERED as isize != 0;
        let alpha = if layered {
            self.api.alpha(self.window).unwrap_or_else(|code| {
                self.failure(Operation::SaveAlpha, code);
                255
            })
        } else {
            255
        };
        *locked(&self.saved) = Saved {
            style,
            alpha,
            layered,
        };
    }

    fn ensure_not_minimized(&self) {
        if self.api.valid(self.window) && self.api.iconic(self.window) {
            self.apply();
        }
    }

    fn apply(&self) {
        let style = match self.api.style(self.window) {
            Ok(style) => style,
            Err(code) => {
                self.failure(Operation::ReadStyle, code);
                return;
            }
        };
        if let Err(code) = self.api.set_style(
            self.window,
            style | WS_EX_LAYERED as isize | WS_EX_TRANSPARENT as isize,
        ) {
            self.failure(Operation::ApplyStyle, code);
            return;
        }
        if let Err(code) = self.api.set_alpha(self.window, 0) {
            self.failure(Operation::ApplyAlpha, code);
            return;
        }
        // U does not roll back partially changed styles on either failure above,
        // and ignores the show return before setting its pseudo flag.
        self.api.show(self.window, false);
        self.pseudo.store(true, Ordering::SeqCst);
    }

    fn revert(&self) {
        let saved = *locked(&self.saved);
        if let Err(code) = self.api.set_style(self.window, saved.style) {
            self.failure(Operation::RevertStyle, code);
        }
        if saved.layered
            && let Err(code) = self.api.set_alpha(self.window, saved.alpha)
        {
            self.failure(Operation::RevertAlpha, code);
        }
        self.pseudo.store(false, Ordering::SeqCst);
    }

    fn monitor(&self) {
        while self.running.load(Ordering::SeqCst) {
            if !self.api.valid(self.window) {
                self.pseudo.store(false, Ordering::SeqCst);
                break;
            }
            if self.pseudo.load(Ordering::SeqCst) && self.api.foreground(self.window) {
                self.revert();
            }
            self.api.sleep(Duration::from_millis(100));
        }
        // Like U, target loss stops the loop without clearing running. A second
        // start remains a no-op until the owner explicitly calls stop.
    }

    fn restore_on_stop(&self) {
        if self.pseudo.load(Ordering::SeqCst) {
            self.revert();
            if !self.api.show(self.window, true) {
                locked(&self.evidence).minimize_returned_zero = true;
            }
        }
    }
}

/// One serial caller owns start/ensure/inactive/stop/destruction. Only the owned
/// monitor queries/restores the same HWND concurrently, matching U. There are no
/// registered callbacks/native leases. Stop clears running, joins to completion
/// (including U's current 100ms sleep/native call), then restores/minimizes.
/// No timeout, cancellation or immediate wakeup is introduced.
pub(crate) struct Helper<A: Api = System> {
    shared: Arc<Shared<A>>,
    monitor: Option<JoinHandle<()>>,
    _caller: PhantomData<Rc<()>>,
}

impl Helper<System> {
    pub fn new(hwnd: HWND) -> Result<Self, AllocError> {
        Self::with_api(hwnd, System)
    }
}

impl<A: Api> Helper<A> {
    fn with_api(hwnd: HWND, api: A) -> Result<Self, AllocError> {
        Ok(Self {
            shared: Arc::try_new(Shared {
                window: Window(hwnd),
                api,
                running: AtomicBool::new(false),
                pseudo: AtomicBool::new(false),
                saved: Mutex::new(Saved {
                    style: 0,
                    alpha: 255,
                    layered: false,
                }),
                evidence: Mutex::new(Evidence::default()),
            })?,
            monitor: None,
            _caller: PhantomData,
        })
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        if !self.shared.begin() {
            return Ok(());
        }
        let shared = Arc::clone(&self.shared);
        // Standard thread ownership; explicit OS spawn failure is returned. This
        // cannot promise recovery from process-aborting allocations in std/OS.
        self.monitor = Some(thread::Builder::new().spawn(move || shared.monitor())?);
        Ok(())
    }

    pub fn ensure_not_minimized(&self) {
        self.shared.ensure_not_minimized();
    }

    pub fn is_pseudo_minimized(&self) -> bool {
        self.shared.pseudo.load(Ordering::SeqCst)
    }

    pub fn revert_pseudo_minimize(&self) {
        self.shared.revert();
    }

    pub fn stop(&mut self) {
        if !self.shared.request_stop() {
            return;
        }
        if let Some(monitor) = self.monitor.take()
            && monitor.join().is_err()
        {
            locked(&self.shared.evidence).worker_panicked = true;
        }
        // Always after completed join, even when a Rust test/worker panicked.
        self.shared.restore_on_stop();
    }

    pub fn evidence(&self) -> Evidence {
        *locked(&self.shared.evidence)
    }
}

impl<A: Api> Drop for Helper<A> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "pseudo_minimize_tests.rs"]
mod tests;
