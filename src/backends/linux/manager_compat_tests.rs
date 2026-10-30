// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
#[path = "../../../tests/upstream-parity/linux-manager/compat-cases.rs"]
mod corpus;

struct Host<'a> {
    case: &'a corpus::Case,
    events: Vec<&'static str>,
    live: usize,
}
impl Portal for Host<'_> {
    type Helper = u8;
    type Error = std::convert::Infallible;
    fn create(&mut self) -> Result<u8, Self::Error> {
        self.events.push("create");
        self.live += 1;
        Ok(1)
    }
    fn open(&mut self, h: u8) -> Result<u8, Self::Error> {
        assert_eq!(h, 1);
        self.events.push("open");
        Ok(self.case.open)
    }
    fn fd(&mut self, h: u8) -> Result<i32, Self::Error> {
        assert_eq!(h, 1);
        self.events.push("fd");
        Ok(self.case.fd)
    }
    fn node(&mut self, h: u8) -> Result<u32, Self::Error> {
        assert_eq!(h, 1);
        self.events.push("node");
        Ok(self.case.node)
    }
    fn destroy(&mut self, h: u8) -> Result<(), Self::Error> {
        assert_eq!(h, 1);
        self.events.push("destroy");
        assert_eq!(self.live, 1);
        self.live = 0;
        Ok(())
    }
}
#[test]
#[ignore = "requires explicit observations from complete original legacy factory bodies"]
fn matches_original_compatibility_arguments_metadata_and_portal_call_order() {
    let root =
        std::path::PathBuf::from(std::env::var_os("MFS_P1_LINUX_COMPAT_OBSERVATIONS").unwrap());
    let output =
        std::path::PathBuf::from(std::env::var_os("MFS_P1_LINUX_COMPAT_RUST_ARTIFACTS").unwrap());
    std::fs::create_dir(&output).unwrap();
    let mut created = 0;
    let mut residuals = 0;
    for case in corpus::cases() {
        let mut host = Host {
            case: &case,
            events: Vec::new(),
            live: 0,
        };
        let config = if case.kwin {
            Config::from_legacy_kwin(
                case.path.as_deref(),
                case.width,
                case.height,
                case.vk,
                &mut host,
            )
            .unwrap()
        } else {
            Config::from_legacy_wlr(case.path.as_deref(), case.vk).unwrap()
        };
        let text = std::fs::read_to_string(root.join(&case.name).join("stdout.txt")).unwrap();
        let fields: std::collections::BTreeMap<_, _> =
            text.lines().map(|l| l.split_once('=').unwrap()).collect();
        assert_eq!(config.is_some(), case.created());
        assert_eq!(fields["created"], if config.is_some() { "1" } else { "0" });
        assert_eq!(host.events.join(","), case.events());
        assert_eq!(host.events.join(","), fields["portal_events"]);
        assert_eq!(host.live, case.residual());
        assert_eq!(host.live, fields["portal_live"].parse::<usize>().unwrap());
        let prefix = output.join(&case.name);
        if let Some(config) = config {
            created += 1;
            let info = config.info().unwrap();
            let uuid = config.uuid().unwrap();
            assert_eq!(info, unhex(fields["info_hex"]));
            assert_eq!(uuid, unhex(fields["uuid_hex"]));
            std::fs::write(prefix.with_extension("info"), info).unwrap();
            std::fs::write(prefix.with_extension("uuid"), uuid).unwrap();
        }
        std::fs::write(prefix.with_extension("events"), host.events.join(",")).unwrap();
        residuals += host.live;
        // This host ledger is a model, with no native Portal owner. Original
        // helper reclaim is separately identified as fixture-only in its record.
    }
    println!(
        "Rust Linux compat cases=120 exact_uuid_metadata=1 portal_call_order=1 created={created} source_open_failure_residuals={residuals} native_authorization=0 passed"
    );
}
fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2));
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
