// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
struct Never;
impl Converter for Never {
    type Image = u8;
    type Error = ();
    #[cfg(feature = "linux-wlr")]
    fn wlr(&mut self, _: wlr::Sample) -> Result<u8, ()> {
        panic!("no Wlr adapter call")
    }
    #[cfg(feature = "linux-pipewire")]
    fn pipewire(&mut self, _: Option<RawFrame>, _: pipewire::Conversion) -> Result<Option<u8>, ()> {
        panic!("no PipeWire adapter call")
    }
    #[cfg(feature = "linux-pipewire")]
    fn pipewire_reuse(&mut self, _: Option<RawFrame>) -> Option<u8> {
        panic!("no PipeWire reuse call")
    }
    #[cfg(feature = "linux-pipewire")]
    fn empty(&mut self) -> Result<u8, ()> {
        panic!("no empty conversion call")
    }
}
#[test]
fn none_and_unimplemented_native_methods_do_not_enter_capture_adapters() {
    for method in [NONE, super::super::EXT_IMAGE, u64::MAX] {
        let mut manager = super::super::NativeManager::new(Config {
            method,
            wlr_socket: Vec::new(),
            pipewire_fd: -1,
            pipewire_node: 0,
        });
        let connected = manager.connect_native().unwrap();
        assert_eq!(connected.connected, method == NONE);
        assert!(connected.failure.is_none());
        assert!(matches!(connected.evidence, Evidence::None));
        let mut output = 99;
        let captured = manager.capture_native(&mut output, &mut Never);
        assert!(!captured.result.unwrap());
        assert_eq!(output, 99);
        assert!(matches!(captured.evidence, Evidence::None));
        assert!(captured.replayed.is_none());
        assert!(manager.inactive_native());
        let close = connected.close;
        drop(manager);
        if method == NONE {
            assert!(matches!(close.unwrap().take(), Some(Evidence::None)));
        } else {
            assert!(close.is_none());
        }
    }
}
#[cfg(feature = "linux-pipewire")]
#[test]
fn safe_native_connect_rejects_an_unproven_source_fd_before_construction() {
    let mut manager = super::super::NativeManager::new(Config {
        method: PIPEWIRE,
        wlr_socket: Vec::new(),
        pipewire_fd: 0,
        pipewire_node: 0,
    });
    assert!(matches!(
        manager.connect_native(),
        Err(Error::UnprovenFdOffer)
    ));
    assert!(!manager.connected());
    assert!(manager.capture.is_none());
    assert!(manager.input.is_none());
}
