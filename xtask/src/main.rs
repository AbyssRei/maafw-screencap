// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

mod batch;
mod configuration;
mod custom_capture;
mod dependencies;
mod desktop_fullscreen;
mod frame_pool_alpha;
mod licenses;
mod linux_compat;
mod linux_config;
mod macos;
mod macos_capture_stages;
mod macos_display_geometry;
mod macos_display_patch;
mod macos_sample_cases;
mod macos_sample_matrix;
mod manager_metadata;
mod native;
mod native_tools;
mod pipewire;
mod pipewire_host;
mod pipewire_host_api;
mod pipewire_host_offer;
mod pipewire_trace;
mod quality;
mod records;
mod reference;
mod review;
mod screen_geometry;
mod windows_delegate;
mod wlr;
mod wlr_failures;
mod wlr_host_api;
mod wlr_protocol;
mod wlr_registry;

#[path = "../../build-support/p0-configuration.rs"]
mod build_configuration;

use std::{env, error::Error, ffi::OsStr, path::Path, process::Command};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let argv: Vec<_> = env::args().skip(1).collect();
    if argv.starts_with(&["reference".into(), "prepare-fork".into()]) {
        return reference::prepare_fork(&argv[2..]);
    }
    if argv.first().is_some_and(|arg| arg == "batch") {
        return batch::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "configure") {
        return configuration::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "configuration-check") {
        return configuration::run_checks(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-custom") {
        return custom_capture::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-delegate") {
        return windows_delegate::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-alpha") {
        return frame_pool_alpha::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-protocol") {
        return wlr_protocol::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "check") {
        return quality::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "native") {
        return native::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-parity") {
        return native::run_parity(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-context") {
        return native::run_context(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-model") {
        return native::run_model(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-trace") {
        return pipewire_trace::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-config") {
        return linux_config::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "upstream-metadata") {
        return manager_metadata::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "pipewire-prototype") {
        return pipewire::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "macos-prototype") {
        return macos::run(&argv[1..]);
    }
    if argv.first().is_some_and(|arg| arg == "wlr-prototype") {
        return wlr::run(&argv[1..]);
    }
    if argv.starts_with(&["reference".into(), "catalog".into()]) {
        return review::run(&argv[2..]);
    }
    if argv.starts_with(&["reference".into(), "verify-run".into()]) {
        return batch::verify_run(&argv[2..]);
    }
    if argv.starts_with(&["reference".into(), "licenses".into()]) {
        return licenses::run(&argv[2..]);
    }
    let mut args = argv.into_iter();
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("reference"), Some(action @ ("prepare" | "verify"))) => {
            let root = match args.next().as_deref() {
                Some("--root") => args.next().ok_or("--root requires a path")?.into(),
                None => env::var_os("MFS_REFERENCE_ROOT")
                    .map(Into::into)
                    .unwrap_or_else(|| {
                        repository()
                            .parent()
                            .unwrap()
                            .join("maafw-screencap-reference")
                    }),
                Some(_) => return Err("expected --root <external reference directory>".into()),
            };
            if args.next().is_some() {
                return Err("unexpected arguments".into());
            }
            if action == "prepare" {
                reference::prepare(&root)
            } else {
                reference::verify(&root)
            }
        }
        (None | Some("help" | "--help" | "-h"), None) => {
            println!(
                "cargo xtask reference prepare [--root <external directory>]\n\
                      cargo xtask reference prepare-fork --revision <id> --head <exact commit> [--root <external directory>]\n\
                      cargo xtask reference verify [--root <external directory>]\n\n\
                      cargo xtask reference verify-run <reference-relative run manifest>\n\n\
                      cargo xtask reference catalog <reference-relative input> [--root <external directory>]\n\n\
                      cargo xtask reference licenses <Rust batch manifest> [--root <external directory>]\n\n\
                      cargo xtask check [--message-file <commit message file>]\n\
                      cargo xtask batch [--desktop | --prepare-only]\n\n\
                      cargo xtask configure [--<family> AUTO|ON|OFF] [--offline-inputs <Rust batch manifest>] [--root <external directory>]\n\n\
                      cargo xtask configuration-check [--offline-inputs <Rust batch manifest>]\n\n\
                      cargo xtask native [--desktop] [--offline-inputs <run manifest>]\n\n\
                      cargo xtask upstream-parity gdi [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity print-window [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity screen-dc [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity frame-pool [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity desktop-dup [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity manager [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-parity custom [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-custom <external directory> [--check-only]\n\n\
                      cargo xtask upstream-delegate <win32|win32-native|win32-factory> <external directory> [--check-only]\n\
                      cargo xtask upstream-delegate win32-runtime <mask> <external case> [--check-only]\n\n\
                      cargo xtask upstream-alpha frame-pool <external directory> [--check-only]\n\n\
                      cargo xtask upstream-protocol <wlr|wlr-registry|wlr-failures> <external directory> [--check-only]\n\n\
                      cargo xtask upstream-context frame-pool [--offline-inputs <native run manifest>]\n\n\
                      cargo xtask upstream-context pipewire  # original source context on native Linux x64\n\n\
                      cargo xtask upstream-context wlr  # original source and recursive roundtrip on native Linux x64\n\n\
                      cargo xtask upstream-context macos  # original ScreenCaptureKit on native macOS arm64\n\n\
                      cargo xtask upstream-model pipewire [--offline-inputs <native run manifest>]  # portable control model on Windows, not Linux capture\n\n\
                      cargo xtask upstream-trace pipewire <source|rust> <external directory> [--check-only]\n\
                      cargo xtask upstream-trace screen-geometry <external directory> [--check-only]\n\
                      cargo xtask upstream-trace desktop-geometry <external window-geometry directory> [--check-only]\n\
                      cargo xtask upstream-trace desktop-fullscreen <external window-fullscreen directory> [--check-only]\n\
                      cargo xtask upstream-trace desktop-workarea <external window-workarea directory> [--check-only]\n\
                      cargo xtask upstream-trace pipewire-host <external matrix> [--check-only]\n\n\
                      cargo xtask upstream-trace wlr-host-api <external matrix> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-patch <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-host-premise <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-distinct-host <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-foundation-consumer <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-appkit-consumer <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-appkit-scene <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-display-observer-same-path <external directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-capture-stages <external observer directory> [--check-only]\n\n\
                      cargo xtask upstream-trace macos-sample-matrix <external directory> [--check-only]\n\n\
                      cargo xtask upstream-config linux <original executable> <new external directory>\n\n\
                      cargo xtask upstream-config linux-compat <original executable> <new external source directory>\n\
                      cargo xtask upstream-config linux-compat-records <external matrix> [--check-only]\n\n\
                      cargo xtask upstream-metadata <win32|macos> <external directory> [--check-only]\n\n\
                      cargo xtask pipewire-prototype  # native Linux x64 only\n\n\
                      cargo xtask macos-prototype     # native macOS arm64 only\n\n\
                      cargo xtask wlr-prototype       # native Linux x64 only\n\n\
                      prepare fetches the fixed upstream and cumulative PR patch; it does not\n\
                      apply the patch, build a reference executable, or assert P0 acceptance.\n\
                      verify checks record versions, identities, paths, Git blobs and SHA-256.\n\
                      git must be available. The reference directory must be outside the product."
            );
            Ok(())
        }
        _ => Err("unknown command; run cargo xtask help".into()),
    }
}

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn command(program: &str, args: &[&OsStr], directory: &Path) -> Result<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn git(directory: &Path, args: &[&str]) -> Result<Vec<u8>> {
    // Downloaded evidence can live deeper than the original runner workspace.
    // Scope long-path support to this process; never alter the user's Git config.
    let mut arguments: Vec<&OsStr> = vec!["-c".as_ref(), "core.longpaths=true".as_ref()];
    arguments.extend(args.iter().map(OsStr::new));
    command("git", &arguments, directory)
}
