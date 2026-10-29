// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit acceptance host only, not a product execution channel.
use std::{
    mem::size_of,
    time::{Duration, Instant},
};
use windows_core::Interface;
#[cfg(feature = "prototype-windows-com")]
#[allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
pub(crate) mod host {
    include!(concat!(env!("OUT_DIR"), "/windows_com_prototype.rs"));
}

#[cfg(feature = "prototype-windows-com")]
pub(crate) struct HostInitialization(pub(crate) bool);
#[cfg(feature = "prototype-windows-com")]
impl Drop for HostInitialization {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: Own one successful caller-thread reference, including S_FALSE.
            unsafe {
                host::RoUninitialize();
            }
            self.0 = false;
        }
    }
}

// Harness-only current-thread queue. Capture still runs synchronously without
// pumping. Only shutdown dispatches messages, after every capture owner closes.
#[cfg(feature = "prototype-windows-com")]
pub(crate) struct HostDispatcher(Option<host::DispatcherQueueController>);
#[cfg(feature = "prototype-windows-com")]
impl HostDispatcher {
    pub(crate) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // SAFETY: A fresh test thread owns its successful MTA initialization;
        // current-thread queue creation does not request an apartment change.
        let controller = unsafe {
            host::CreateDispatcherQueueController(host::DispatcherQueueOptions {
                dwSize: u32::try_from(size_of::<host::DispatcherQueueOptions>())?,
                threadType: host::DQTYPE_THREAD_CURRENT,
                apartmentType: host::DQTAT_COM_NONE,
            })?
        };
        Ok(Self(Some(controller.cast()?)))
    }

    pub(crate) fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(controller) = self.0.as_ref() else {
            return Ok(());
        };
        // Any native shutdown failure aborts this test process; do not reissue
        // shutdown, unload an active queue, or turn an observation timeout into
        // permission to abandon the caller's owned native thread.
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            let operation = controller.ShutdownQueueAsync()?;
            let began = Instant::now();
            let mut over_budget = false;
            while operation.Status()? == windows_future::AsyncStatus::Started {
                let mut message = host::MSG::default();
                // SAFETY: Read and dispatch only this harness thread's queue.
                while unsafe {
                    host::PeekMessageW(&mut message, None, 0, 0, u32::try_from(host::PM_REMOVE)?)
                }
                .as_bool()
                {
                    if message.message == u32::try_from(host::WM_QUIT)? {
                        return Err("unexpected WM_QUIT during dispatcher shutdown".into());
                    }
                    unsafe {
                        let _ = host::TranslateMessage(&message);
                        host::DispatchMessageW(&message);
                    }
                }
                over_budget |= began.elapsed() > Duration::from_secs(5);
                std::thread::sleep(Duration::from_millis(1));
            }
            operation.GetResults()?;
            Ok(over_budget)
        })();
        match result {
            Ok(over_budget) => {
                self.0.take();
                println!("Rust dispatcher_shutdown=completed over_budget={over_budget}");
                if over_budget {
                    return Err("dispatcher shutdown exceeded observation budget".into());
                }
                Ok(())
            }
            Err(error) => {
                eprintln!("native dispatcher shutdown failed: {error}");
                std::process::abort();
            }
        }
    }
}
#[cfg(feature = "prototype-windows-com")]
impl Drop for HostDispatcher {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            // Queue completion has already been established for this branch;
            // retain a failed test result even on an earlier assertion unwind.
            eprintln!("dispatcher cleanup exceeded observation budget: {error}");
            std::process::abort();
        }
    }
}
