// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

#[path = "build-support/p0-configuration.rs"]
mod configuration;

fn main() {
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_LINUX_WLR");
    println!("cargo::rerun-if-changed=build-support/wayland-reentry.c");
    println!("cargo::rerun-if-changed=build-support/wayland-reentry.h");
    println!("cargo::rerun-if-changed=build-support/p0-configuration.rs");
    println!("cargo::rerun-if-env-changed=MFS_BUILD_CONFIGURATION");
    println!("cargo::rerun-if-env-changed=HOST");
    for name in configuration::PROBE_ENV {
        println!("cargo::rerun-if-env-changed={name}");
    }
    if let Some(path) = std::env::var_os("MFS_BUILD_CONFIGURATION") {
        println!(
            "cargo::rerun-if-changed={}",
            std::path::Path::new(&path).display()
        );
        let text = std::fs::read_to_string(&path).expect("read explicit P0 build configuration");
        let selected =
            configuration::Configuration::parse(&text).expect("valid P0 build configuration");
        let target = std::env::var("TARGET").expect("Cargo target");
        let host = std::env::var("HOST").expect("Cargo host");
        assert_eq!(
            selected.target, target,
            "configuration target differs from Cargo TARGET"
        );
        assert_eq!(
            host, target,
            "P0 prepared configuration currently requires native builds"
        );
        let os = std::env::var("CARGO_CFG_TARGET_OS").expect("Cargo target OS");
        let expected_os = match target.as_str() {
            "x86_64-pc-windows-msvc" => "windows",
            "x86_64-unknown-linux-gnu" => "linux",
            "aarch64-apple-darwin" => "macos",
            _ => unreachable!(),
        };
        assert_eq!(
            os, expected_os,
            "configuration target OS differs from Cargo"
        );
        let features = configuration::FEATURES.map(|name| {
            let variable = format!(
                "CARGO_FEATURE_{}",
                name.replace('-', "_").to_ascii_uppercase()
            );
            println!("cargo::rerun-if-env-changed={variable}");
            std::env::var_os(variable).is_some()
        });
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_PROTOTYPE_WINDOWS");
        selected
            .verify_features(
                &features,
                std::env::var_os("CARGO_FEATURE_PROTOTYPE_WINDOWS").is_some(),
            )
            .expect("prepared Cargo feature selection");
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_LINUX_PIPEWIRE");
        assert!(
            os != "linux"
                || std::env::var_os("CARGO_FEATURE_LINUX_PIPEWIRE").is_none()
                || selected.enabled[1],
            "PipeWire consumer cannot be enabled when the prepared PipeWire family is OFF"
        );
        assert!(
            os != "linux"
                || std::env::var_os("CARGO_FEATURE_LINUX_WLR").is_none()
                || selected.enabled[2],
            "Wlr capture cannot be enabled when the prepared Wlr family is OFF"
        );
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_WINDOWS_GDI");
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_WINDOWS_PRINT_WINDOW");
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_WINDOWS_SCREEN_DC");
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_WINDOWS_FRAME_POOL");
        println!("cargo::rerun-if-env-changed=CARGO_FEATURE_WINDOWS_DESKTOP_DUP");
        assert!(
            (std::env::var_os("CARGO_FEATURE_WINDOWS_GDI").is_none()
                && std::env::var_os("CARGO_FEATURE_WINDOWS_PRINT_WINDOW").is_none()
                && std::env::var_os("CARGO_FEATURE_WINDOWS_SCREEN_DC").is_none()
                && std::env::var_os("CARGO_FEATURE_WINDOWS_FRAME_POOL").is_none()
                && std::env::var_os("CARGO_FEATURE_WINDOWS_DESKTOP_DUP").is_none())
                || selected.enabled[0],
            "Windows capture backends cannot be enabled when the prepared Windows family is OFF"
        );
        for (family, enabled) in selected.enabled.iter().enumerate() {
            if *enabled {
                let mut evidence = Vec::new();
                configuration::probe(&target, family, &mut evidence)
                    .expect("selected native dependencies remain available");
                selected
                    .verify_native(family, &evidence)
                    .expect("native dependencies match the explicit prepared input");
                for item in evidence {
                    if let Some(path) = item.strip_prefix("native_file=") {
                        println!("cargo::rerun-if-changed={path}");
                    }
                }
            }
        }
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(out.join("p0-build-configuration.txt"), selected.encode())
            .expect("write verified configuration into OUT_DIR");
        // Enumerate selected feature names here too so the saved build input is
        // inspectable without depending on xtask's external report layout.
        std::fs::write(
            out.join("p0-build-features.txt"),
            selected.features().join(","),
        )
        .expect("write verified features into OUT_DIR");
    }
    println!("cargo::rerun-if-env-changed=TARGET");
    #[cfg(feature = "linux-wlr")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        let native = pkg_config::Config::new()
            .probe("wayland-client")
            .expect("selected Wlr needs native libwayland-client headers and library");
        let mut header_roots = native.include_paths.clone();
        // pkg-config may omit a default system include directory from cflags.
        // Track the actual native declarations as well as our narrow adapter.
        header_roots.push(
            pkg_config::get_variable("wayland-client", "includedir")
                .expect("selected Wayland include directory")
                .into(),
        );
        for header in [
            "wayland-client-core.h",
            "wayland-util.h",
            "wayland-version.h",
        ] {
            let path = header_roots
                .iter()
                .map(|root| root.join(header))
                .find(|path| path.is_file())
                .expect("selected Wayland native header");
            println!("cargo::rerun-if-changed={}", path.display());
        }
        let mut bridge = cc::Build::new();
        bridge.file("build-support/wayland-reentry.c").std("c11");
        bridge.flag_if_supported("-Werror=implicit-function-declaration");
        for include in native.include_paths {
            bridge.include(include);
        }
        bridge.compile("mfs_wayland_reentry");
    }
    println!("cargo::rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo::rerun-if-changed=build-support/windows-prototype.txt");
    println!("cargo::rerun-if-changed=build-support/windows-com-prototype.txt");
    println!("cargo::rerun-if-changed=build-support/windows-gdi.txt");
    println!("cargo::rerun-if-changed=build-support/windows-print-window.txt");
    println!("cargo::rerun-if-changed=build-support/windows-gdi-common.txt");
    println!("cargo::rerun-if-changed=build-support/windows-window-coordination.txt");
    println!("cargo::rerun-if-changed=build-support/windows-screen-window.txt");
    println!("cargo::rerun-if-changed=build-support/windows-frame-pool.txt");
    println!("cargo::rerun-if-changed=build-support/windows-desktop-dup.txt");
    println!("cargo::rerun-if-changed=build-support/windows-manager.txt");
    #[cfg(feature = "windows-desktop-dup")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        assert_eq!(
            std::env::var("TARGET").as_deref(),
            Ok("x86_64-pc-windows-msvc")
        );
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("windows_desktop_dup.rs");
        let input = std::fs::read_to_string("build-support/windows-desktop-dup.txt").unwrap();
        windows_bindgen::builder()
            .output(out)
            .flat()
            .filters(
                input
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#')),
            )
            .write();
    }
    #[cfg(feature = "windows-frame-pool")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        assert_eq!(
            std::env::var("TARGET").as_deref(),
            Ok("x86_64-pc-windows-msvc")
        );
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("windows_frame_pool.rs");
        let input = std::fs::read_to_string("build-support/windows-frame-pool.txt").unwrap();
        windows_bindgen::builder()
            .output(out)
            .flat()
            .filters(
                input
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#')),
            )
            .write();
    }
    #[cfg(any(
        feature = "windows-print-window",
        feature = "windows-screen-dc",
        feature = "windows-desktop-dup-window",
        feature = "windows-frame-pool"
    ))]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("windows_window_coordination.rs");
        let mut input = String::new();
        for (enabled, path) in [
            (
                cfg!(feature = "windows-manager"),
                "build-support/windows-manager.txt",
            ),
            (
                cfg!(any(
                    feature = "windows-print-window",
                    feature = "windows-frame-pool"
                )),
                "build-support/windows-window-coordination.txt",
            ),
            (
                cfg!(any(
                    feature = "windows-screen-dc",
                    feature = "windows-desktop-dup-window"
                )),
                "build-support/windows-screen-window.txt",
            ),
        ] {
            if enabled {
                input.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
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
    #[cfg(any(
        feature = "windows-gdi",
        feature = "windows-print-window",
        feature = "windows-screen-dc"
    ))]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        assert_eq!(
            std::env::var("TARGET").as_deref(),
            Ok("x86_64-pc-windows-msvc"),
            "the initial Windows capture migration requires x86_64-pc-windows-msvc"
        );
        let out =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("windows_gdi.rs");
        let mut input = std::fs::read_to_string("build-support/windows-gdi-common.txt").unwrap();
        for (enabled, path) in [
            (
                cfg!(any(feature = "windows-gdi", feature = "windows-screen-dc")),
                "build-support/windows-gdi.txt",
            ),
            (
                cfg!(feature = "windows-print-window"),
                "build-support/windows-print-window.txt",
            ),
        ] {
            if enabled {
                input.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
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
    #[cfg(feature = "prototype-windows-com")]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("windows_com_prototype.rs");
        let input = std::fs::read_to_string("build-support/windows-com-prototype.txt").unwrap();
        windows_bindgen::builder()
            .output(out)
            .flat()
            .filters(
                input
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#')),
            )
            .write();
    }
}
