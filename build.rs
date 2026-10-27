// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

fn main() {
    println!("cargo::rerun-if-env-changed=TARGET");
    println!("cargo::rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo::rerun-if-changed=build-support/windows-prototype.txt");
    #[cfg(feature = "prototype-windows")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("windows_prototype.rs");
        let input = std::fs::read_to_string("build-support/windows-prototype.txt").unwrap();
        let mut args = vec![
            "--out",
            out.to_str().unwrap(),
            "--flat",
            "--sys",
            "--filter",
        ];
        args.extend(
            input
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#')),
        );
        windows_bindgen::bindgen(args);
    }
}
