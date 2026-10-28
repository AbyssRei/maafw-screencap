// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::platform::windows::window_api::*;
use crate::test_support::native_module::Module;
use std::{
    error::Error,
    ffi::c_void,
    mem::{align_of, offset_of, size_of},
    panic::{AssertUnwindSafe, catch_unwind},
    time::Duration,
};

#[repr(C)]
struct Api {
    context: *mut c_void,
    invoke: unsafe extern "C" fn(*mut c_void, u32, Window, *mut i64) -> i64,
}
#[derive(Default, Debug, PartialEq)]
#[repr(C)]
pub struct Output {
    pub first: i64,
    pub second: i64,
    pub scale: f64,
}
struct Context<'a> {
    api: &'a dyn WindowApi,
    panicked: bool,
}
pub struct Adapter {
    module: Module,
}
impl Adapter {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let path =
            std::env::var_os("MFS_GDI_PARITY_LIBRARY").ok_or("MFS_GDI_PARITY_LIBRARY required")?;
        let module = Module::load(&path)?;
        type Layout = unsafe extern "C" fn(*mut u64, usize) -> i32;
        // SAFETY: Exact header signature, module retained for the entire call.
        let layout: Layout = unsafe { module.symbol(c"mfs_window_script_layout")? };
        let mut observed = [0; 10];
        // SAFETY: Exact writable fixed-size layout array.
        if unsafe { layout(observed.as_mut_ptr(), observed.len()) } != 0 {
            return Err("window layout query failed".into());
        }
        let expected = [
            size_of::<Api>(),
            align_of::<Api>(),
            offset_of!(Api, context),
            offset_of!(Api, invoke),
            size_of::<Output>(),
            align_of::<Output>(),
            offset_of!(Output, first),
            offset_of!(Output, second),
            offset_of!(Output, scale),
            8,
        ]
        .map(|n| u64::try_from(n).unwrap());
        if observed != expected {
            return Err("window C/Rust layout mismatch".into());
        }
        Ok(Self { module })
    }
    pub fn run(
        &self,
        api: &dyn WindowApi,
        mode: u32,
        window: Window,
    ) -> Result<Output, Box<dyn Error>> {
        type Run = unsafe extern "C" fn(*const Api, u32, Window, *mut Output) -> i32;
        // SAFETY: Exact signature from window-script.h; synchronous only.
        let run: Run = unsafe { self.module.symbol(c"mfs_window_script")? };
        let mut context = Context {
            api,
            panicked: false,
        };
        let table = Api {
            context: (&mut context as *mut Context<'_>).cast(),
            invoke,
        };
        let mut output = Output::default();
        // SAFETY: Borrowed table/context/output all outlive the call, which
        // retains no callback, pointer, object or asynchronous work.
        let status = unsafe { run(&table, mode, window, &mut output) };
        if status != 0 || context.panicked {
            return Err(format!(
                "window reference status={status}, callback panic={}",
                context.panicked
            )
            .into());
        }
        Ok(output)
    }
}
unsafe extern "C" fn invoke(context: *mut c_void, op: u32, window: Window, data: *mut i64) -> i64 {
    // SAFETY: Native wrapper borrows this live Context and exactly eight aligned
    // initialized writable slots synchronously, without aliasing another call.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
    let a = unsafe { &mut *data.cast::<[i64; 8]>() };
    let result = catch_unwind(AssertUnwindSafe(|| {
        let api = context.api;
        let write_rect = |a: &mut [i64; 8], rect: Rect| {
            a[..4].copy_from_slice(&[rect.left, rect.top, rect.right, rect.bottom].map(i64::from));
        };
        match op {
            1 => i64::from(api.valid(window)),
            2 => i64::try_from(api.foreground().addr()).unwrap(),
            3 => {
                api.foreground_request(window);
                1
            }
            4 => {
                api.order(
                    window,
                    match a[0] {
                        0 => Order::Top,
                        1 => Order::Topmost,
                        2 => Order::NotTopmost,
                        _ => panic!("unexpected order"),
                    },
                    a[1] != 0,
                );
                1
            }
            5 => i64::from(api.ticks()),
            6 => {
                api.sleep(Duration::from_millis(u64::try_from(a[0]).unwrap()));
                1
            }
            7 => i64::from(api.zoomed(window)),
            8 => i64::from(api.iconic(window)),
            9 => i64::try_from(api.monitor(window).addr()).unwrap(),
            10 => match api.monitor_rects(window) {
                Ok(r) => {
                    write_rect(a, r.work);
                    a[4..].copy_from_slice(
                        &[
                            r.monitor.left,
                            r.monitor.top,
                            r.monitor.right,
                            r.monitor.bottom,
                        ]
                        .map(i64::from),
                    );
                    1
                }
                Err(_) => 0,
            },
            11 | 12 => match if op == 11 {
                api.window_rect(window)
            } else {
                api.client_rect(window)
            } {
                Ok(r) => {
                    write_rect(a, r);
                    1
                }
                Err(_) => 0,
            },
            13 => match api.origin(
                window,
                Point {
                    x: i32::try_from(a[0]).unwrap(),
                    y: i32::try_from(a[1]).unwrap(),
                },
            ) {
                Ok(p) => {
                    a[0] = i64::from(p.x);
                    a[1] = i64::from(p.y);
                    1
                }
                Err(_) => 0,
            },
            14 => i64::from(
                api.position(
                    window,
                    Position {
                        x: i32::try_from(a[0]).unwrap(),
                        y: i32::try_from(a[1]).unwrap(),
                        width: i32::try_from(a[2]).unwrap(),
                        height: i32::try_from(a[3]).unwrap(),
                    },
                )
                .is_ok(),
            ),
            15 => i64::from(api.dpi(window)),
            16 => i64::try_from(api.style(window)).unwrap(),
            _ => panic!("unexpected window operation {op}"),
        }
    }));
    match result {
        Ok(value) => value,
        Err(_) => {
            context.panicked = true;
            0
        }
    }
}
