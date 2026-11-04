// Copyright (C) 2026 AbyssRei (Rust migration)
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Fixed macOS manager screenshot path and host-owned input lifetime boundary.
//! No input actions, native input implementation or public SDK API.
use std::{collections::TryReserveError, fmt::Write};

// Fixed MaaDef.h values, private U selectors rather than finalized SDK ABI.
pub(crate) const NONE: u64 = 0;
pub(crate) const SCREEN_CAPTURE_KIT: u64 = 1;
const GLOBAL_EVENT: u64 = 1;
const POST_TO_PID: u64 = 2;

pub(crate) trait InputFeatures {
    fn features(&self) -> u64;
}
pub(crate) enum NoInput {}
impl InputFeatures for NoInput {
    fn features(&self) -> u64 {
        match *self {}
    }
}

pub(crate) struct Manager<B, I = NoInput> {
    window_id: u32,
    method: u64,
    input_method: u64,
    connected: bool,
    // Rust fields drop in declaration order. The source's reverse C++ member
    // order releases screencap_ before input_; keep that owner order here.
    selected: Option<B>,
    input: Option<I>,
}
impl<B> Manager<B, NoInput> {
    pub const fn new(window_id: u32, method: u64) -> Self {
        Self::with_host_input(window_id, method, NONE)
    }
    pub fn connect_with<E>(
        &mut self,
        available: bool,
        create: impl FnOnce(u32) -> Result<B, E>,
    ) -> Result<bool, E> {
        self.connect_with_host_input(available, create, |_, _| {
            unreachable!("Input=None cannot construct a host input")
        })
    }
}
impl<B, I: InputFeatures> Manager<B, I> {
    pub const fn with_host_input(window_id: u32, method: u64, input_method: u64) -> Self {
        Self {
            window_id,
            method,
            input_method,
            connected: false,
            selected: None,
            input: None,
        }
    }
    pub fn connect_with_host_input<E>(
        &mut self,
        available: bool,
        create: impl FnOnce(u32) -> Result<B, E>,
        create_input: impl FnOnce(u32, u64) -> Result<I, E>,
    ) -> Result<bool, E> {
        self.connected = false;
        if self.method == SCREEN_CAPTURE_KIT {
            if !available {
                return Ok(false);
            }
            // Construct BEFORE replacing the old owner. Failed allocation or
            // unavailable OS leaves that owner live, while connected is false.
            self.selected = Some(create(self.window_id)?);
        }
        if matches!(self.input_method, GLOBAL_EVENT | POST_TO_PID) {
            // Input construction follows capture replacement. An input error
            // retains the new capture and any old input with connected=false.
            self.input = Some(create_input(self.window_id, self.input_method)?);
        }
        // Both None/unknown switches only log; existing owners remain live.
        self.connected = true;
        Ok(true)
    }
    pub const fn connected(&self) -> bool {
        self.connected
    }
    pub fn get_features(&self) -> u64 {
        // Query the retained host owner, including after failed reconnect.
        // These two original feature bits are not finalized SDK capabilities.
        self.input.as_ref().map_or(0, I::features) & 3
    }
    pub fn get_info(&self) -> Result<String, TryReserveError> {
        let mut value = String::new();
        value.try_reserve_exact(192)?;
        // Preserve the source's uint64 -> int64 metadata conversion as a bit
        // interpretation; selectors remain u64 for actual method selection.
        let method = i64::from_ne_bytes(self.method.to_ne_bytes());
        let input = i64::from_ne_bytes(self.input_method.to_ne_bytes());
        write!(&mut value,"{{\"input_method\":{input},\"screencap_method\":{method},\"type\":\"macos\",\"window_id\":{}}}",self.window_id).expect("String formatting cannot fail");
        Ok(value)
    }
    pub fn request_uuid(&self, output: &mut String) -> Result<bool, TryReserveError> {
        let mut value = String::new();
        value.try_reserve_exact(10)?; // u32 decimal is at most ten bytes.
        write!(&mut value, "{}", self.window_id).expect("String formatting cannot fail");
        *output = value;
        Ok(true)
    }
    pub fn capture_with<Image, E>(
        &self,
        output: &mut Image,
        capture: impl FnOnce(&B) -> Result<Option<Image>, E>,
    ) -> Result<bool, E> {
        let Some(unit) = &self.selected else {
            return Ok(false);
        };
        // No connected-state gate at this forwarding entry in the source.
        let Some(image) = capture(unit)? else {
            return Ok(false);
        };
        *output = image;
        Ok(true)
    }
    pub const fn inactive(&self) -> bool {
        true
    }
}

#[cfg(all(target_os = "macos", feature = "macos-manager"))]
#[path = "manager_native.rs"]
mod native;
#[cfg(all(target_os = "macos", feature = "macos-manager"))]
pub(crate) type NativeManager = Manager<std::sync::Arc<super::screen_capture_kit::Capture>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    #[test]
    #[ignore = "requires explicitly built original get_info model with current meojson"]
    fn metadata_matches_original_signed_selectors_and_window_identity() {
        crate::backends::manager_metadata::compare("macos", |id, method| {
            Manager::<()>::new(u32::try_from(id).unwrap(), method)
                .get_info()
                .unwrap()
        });
    }

    #[test]
    fn none_and_unknown_methods_connect_without_a_capture_or_output_change() {
        for method in [NONE, 2, u64::MAX] {
            let mut manager = Manager::<()>::new(7, method);
            let mut output = vec![99];
            assert!(!manager.connected());
            assert_eq!(manager.get_features(), 0);
            assert!(
                !manager
                    .capture_with(&mut output, |_| -> Result<_, ()> { panic!("no source") })
                    .unwrap()
            );
            assert!(
                manager
                    .connect_with(false, |_| -> Result<_, ()> { panic!("no factory") })
                    .unwrap()
            );
            assert!(manager.connected());
            assert_eq!(manager.get_features(), 0);
            assert!(
                !manager
                    .capture_with(&mut output, |_| -> Result<_, ()> { panic!("no source") })
                    .unwrap()
            );
            assert_eq!(output, [99]);
        }
    }
    #[test]
    fn nullopt_and_exception_keep_output_but_engaged_empty_replaces_it() {
        let mut manager = Manager::new(7, SCREEN_CAPTURE_KIT);
        manager.connect_with(true, Ok::<_, ()>).unwrap();
        let mut output = vec![99];
        assert!(
            !manager
                .capture_with(&mut output, |_| Ok::<_, ()>(None))
                .unwrap()
        );
        assert_eq!(output, [99]);
        assert_eq!(
            manager.capture_with(&mut output, |_| Err::<Option<Vec<u8>>, _>(9)),
            Err(9)
        );
        assert_eq!(output, [99]);
        assert!(
            manager
                .capture_with(&mut output, |_| Ok::<_, ()>(Some(Vec::new())))
                .unwrap()
        );
        assert!(output.is_empty());
    }
    #[test]
    fn failed_reconnect_keeps_old_owner_and_allows_source_forwarding() {
        struct Unit(Rc<Cell<usize>>);
        impl Drop for Unit {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let dropped = Rc::new(Cell::new(0));
        let mut manager = Manager::new(7, SCREEN_CAPTURE_KIT);
        manager
            .connect_with(true, |_| Ok::<_, ()>(Unit(dropped.clone())))
            .unwrap();
        assert_eq!(manager.connect_with(true, |_| Err::<Unit, _>(9)), Err(9));
        assert!(!manager.connected());
        assert_eq!(manager.get_features(), 0);
        assert_eq!(dropped.get(), 0);
        assert!(
            !manager
                .connect_with(false, |_| -> Result<Unit, ()> { panic!("unavailable") })
                .unwrap()
        );
        let mut output = 99;
        assert!(
            manager
                .capture_with(&mut output, |_| Ok::<_, ()>(Some(7)))
                .unwrap()
        );
        assert_eq!(output, 7);
        assert!(manager.inactive());
        assert_eq!(manager.get_features(), 0);
        assert_eq!(dropped.get(), 0);
        manager
            .connect_with(true, |_| {
                assert_eq!(
                    dropped.get(),
                    0,
                    "new owner must construct before old owner release"
                );
                Ok::<_, ()>(Unit(dropped.clone()))
            })
            .unwrap();
        assert_eq!(dropped.get(), 1);
        drop(manager);
        assert_eq!(dropped.get(), 2);
    }
    #[test]
    fn uuid_is_decimal_window_identity_even_without_connection() {
        for (id, expected) in [(0, "0"), (7, "7"), (u32::MAX, "4294967295")] {
            let manager = Manager::<()>::new(id, NONE);
            let mut output = String::from("unchanged");
            assert!(manager.request_uuid(&mut output).unwrap());
            assert_eq!(output, expected);
        }
    }
}

#[cfg(all(
    test,
    any(
        all(target_os = "windows", feature = "prototype-windows"),
        target_os = "linux",
        all(target_os = "macos", feature = "macos-manager")
    )
))]
#[path = "../../../tests/upstream-parity/macos-manager-queries/compare.rs"]
mod query_tests;

#[cfg(all(
    test,
    any(
        all(target_os = "windows", feature = "prototype-windows"),
        target_os = "linux",
        all(target_os = "macos", feature = "macos-manager")
    )
))]
#[path = "../../../tests/upstream-parity/macos-manager-queries/host.rs"]
mod host_tests;
