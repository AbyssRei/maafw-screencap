// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[cfg(feature = "linux-pipewire")]
#[test]
fn safe_hosted_connect_refuses_an_unproven_descriptor_before_host_creation() {
    use super::super::host::{InputOwner, InputProvider};
    struct NeverHost;
    struct NeverInput;
    impl InputOwner for NeverInput {
        type Error = Error;
        fn init(&mut self) -> Result<bool, Error> {
            panic!("unproven fd reached host init")
        }
        fn inactive(&mut self) {
            panic!("unproven fd created host owner")
        }
        fn features(&self) -> u64 {
            panic!("unproven fd created host owner")
        }
    }
    impl<C> InputProvider<C, Error> for NeverHost {
        type Input = NeverInput;
        fn create(&mut self, _: &Config, _: Option<&C>) -> Result<NeverInput, Error> {
            panic!("unproven fd reached host creation")
        }
    }
    let mut manager = super::super::Manager::<HostedFactory<NeverHost>>::new(Config {
        method: PIPEWIRE,
        input_method: 2,
        pipewire_fd: 0,
        ..Config::default()
    });
    let connection: HostConnection = manager.connect_host_native(NeverHost);
    assert!(matches!(connection.result, Err(Error::UnprovenFdOffer)));
    assert!(connection.failure.is_none() && connection.close.is_none());
    assert!(matches!(connection.evidence, Evidence::None));
    assert!(!manager.connected() && manager.capture.is_none() && manager.input.is_none());
}
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
fn missing_host_input_provider_is_explicit_and_retains_the_created_none_capture() {
    for input_method in [2, 4] {
        let mut manager = super::super::NativeManager::new(Config {
            input_method,
            ..Config::default()
        });
        assert!(
            matches!(manager.connect_native(), Err(Error::HostInputRequired(value)) if value == input_method)
        );
        assert!(!manager.connected());
        assert!(manager.capture.is_some());
        assert!(manager.input.is_none());
        assert_eq!(manager.get_features(), 0);
        let mut output = 99;
        assert!(
            !manager
                .capture_native(&mut output, &mut Never)
                .result
                .unwrap()
        );
        assert_eq!(output, 99);
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
            ..Config::default()
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
        ..Config::default()
    });
    assert!(matches!(
        manager.connect_native(),
        Err(Error::UnprovenFdOffer)
    ));
    assert!(!manager.connected());
    assert!(manager.capture.is_none());
    assert!(manager.input.is_none());
}
