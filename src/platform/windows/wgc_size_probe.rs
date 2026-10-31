// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Controlled Size results after a real query, never a replacement COM object.
use super::native::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use windows_core::Interface;

pub(crate) const INJECTED: i32 = 0x8000_4004_u32 as i32;
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Query {
    pub native_code: i32,
    pub width: i32,
    pub height: i32,
    pub returned_code: i32,
    pub returned_width: i32,
    pub returned_height: i32,
    pub thread: u32,
    pub item: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trace {
    pub mode: u32,
    pub calls: usize,
    pub queries: [Query; 8],
}
thread_local! {static ACTIVE: RefCell<Option<Trace>>=const {RefCell::new(None)};}
pub(crate) struct Guard(PhantomData<Rc<()>>);
impl Guard {
    pub fn begin(mode: u32) -> Self {
        assert!(mode <= 9);
        ACTIVE.with(|slot| {
            assert!(slot.borrow().is_none());
            *slot.borrow_mut() = Some(Trace {
                mode,
                ..Trace::default()
            });
        });
        Self(PhantomData)
    }
    pub fn trace(&self) -> Trace {
        ACTIVE.with(|slot| slot.borrow().unwrap())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = None);
    }
}
pub(crate) fn size(item: &GraphicsCaptureItem) -> windows_core::Result<SizeInt32> {
    // Complete the real call before touching the observer. Injection models a
    // caller-visible failure/size, not an actual OS Size failure or window race.
    let native = item.Size();
    ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(trace) = slot.as_mut() else {
            return native;
        };
        let index = trace.calls + 1;
        let mut query = Query {
            // SAFETY: Scalar identity query on this exact native caller.
            thread: unsafe { GetCurrentThreadId() },
            item: item.as_raw() as usize,
            ..Query::default()
        };
        let mut result = match native {
            Ok(size) => {
                query.width = size.Width;
                query.height = size.Height;
                Ok(size)
            }
            Err(error) => {
                query.native_code = error.code().0;
                Err(error)
            }
        };
        let fault = match trace.mode {
            1..=3 => trace.mode as usize,
            4 => 1,
            5 => 3,
            6 => 4,
            7 => 2,
            _ => 0,
        };
        if let Ok(ref mut size) = result {
            if index == fault {
                result = Err(windows_core::Error::from_hresult(windows_core::HRESULT(
                    INJECTED,
                )));
            } else if (trace.mode == 8 && index == 1) || (trace.mode == 9 && index == 2) {
                size.Width = 0;
            } else if (5..=7).contains(&trace.mode) && index == 1 {
                size.Width = size.Width.checked_add(1).expect("fixture width");
            }
        }
        match &result {
            Ok(size) => {
                query.returned_width = size.Width;
                query.returned_height = size.Height;
            }
            Err(error) => query.returned_code = error.code().0,
        }
        trace.queries[trace.calls] = query;
        trace.calls = index;
        result
    })
}
