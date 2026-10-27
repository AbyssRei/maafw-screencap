// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

// Shared build preparation contract. No capture code, SDK types or dependencies.
use std::{env, fs, path::PathBuf, process::Command};

pub const FAMILIES: [&str; 4] = ["windows", "pipewire", "wlr", "screen-capture-kit"];
pub const FEATURES: [&str; 4] = [
    "prototype-windows-com",
    "prototype-linux-pipewire",
    "prototype-linux-wlr",
    "prototype-macos",
];
pub const PROBE_ENV: [&str; 13] = [
    "PATH",
    "PKG_CONFIG_PATH",
    "PKG_CONFIG_LIBDIR",
    "PKG_CONFIG_SYSROOT_DIR",
    "PKG_CONFIG",
    "LIBCLANG_PATH",
    "LLVM_CONFIG_PATH",
    "DEVELOPER_DIR",
    "SDKROOT",
    "ProgramFiles(x86)",
    "WindowsSdkDir",
    "LIB",
    "INCLUDE",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Auto,
    On,
    Off,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "AUTO" => Ok(Self::Auto),
            "ON" => Ok(Self::On),
            "OFF" => Ok(Self::Off),
            _ => Err("expected AUTO, ON or OFF".into()),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::On => "ON",
            Self::Off => "OFF",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Configuration {
    pub target: String,
    pub modes: [Mode; 4],
    pub enabled: [bool; 4],
    // None retains the interpretation of archived format-1 prototype records.
    pub native_evidence: Option<[String; 4]>,
}

pub fn applicable(target: &str, family: usize) -> bool {
    matches!(
        (target, family),
        ("x86_64-pc-windows-msvc", 0)
            | ("x86_64-unknown-linux-gnu", 1 | 2)
            | ("aarch64-apple-darwin", 3)
    )
}

// Only the preparer chooses capabilities. build.rs never changes this decision.
#[allow(dead_code)] // Used by xtask; build.rs only verifies the prepared choice.
pub fn choose(mode: Mode, available: bool) -> Result<bool, String> {
    match (mode, available) {
        (Mode::On, false) => Err("explicit ON requires an available native dependency".into()),
        (Mode::Off, _) | (Mode::Auto, false) => Ok(false),
        (_, true) => Ok(true),
    }
}

impl Configuration {
    pub fn encode(&self) -> String {
        let mut text = format!(
            "format={}\ntarget={}\n",
            if self.native_evidence.is_some() { 2 } else { 1 },
            self.target
        );
        for (n, family) in FAMILIES.iter().enumerate() {
            text.push_str(&format!(
                "{family}={},{}\n",
                self.modes[n].name(),
                if self.enabled[n] {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }
        if let Some(evidence) = &self.native_evidence {
            for (n, family) in FAMILIES.iter().enumerate() {
                let hex = evidence[n]
                    .as_bytes()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                text.push_str(&format!("native.{family}={hex}\n"));
            }
        }
        text
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let lines: Vec<_> = text.lines().collect();
        let version = match lines.first().copied() {
            Some("format=1") if lines.len() == 6 => 1,
            Some("format=2") if lines.len() == 10 => 2,
            _ => 0,
        };
        if version == 0 {
            return Err("invalid P0 configuration format/version".into());
        }
        let target = lines[1]
            .strip_prefix("target=")
            .ok_or("missing native target")?;
        if ![
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
        ]
        .contains(&target)
        {
            return Err("P0 configuration requires a supported native runner target".into());
        }
        let mut result = Self {
            target: target.into(),
            modes: [Mode::Off; 4],
            enabled: [false; 4],
            native_evidence: None,
        };
        for (n, family) in FAMILIES.iter().enumerate() {
            let value = lines[n + 2]
                .strip_prefix(&format!("{family}="))
                .ok_or("missing/duplicate/out-of-order backend field")?;
            let (mode, enabled) = value.split_once(',').ok_or("missing backend selection")?;
            result.modes[n] = Mode::parse(mode)?;
            result.enabled[n] = match enabled {
                "enabled" => true,
                "disabled" => false,
                _ => return Err("invalid backend selection".into()),
            };
            if (result.modes[n] == Mode::Off && result.enabled[n])
                || (result.modes[n] == Mode::On && !result.enabled[n])
                || (result.enabled[n] && !applicable(target, n))
            {
                return Err("backend selection contradicts its request or native target".into());
            }
        }
        if version == 2 {
            let mut evidence = std::array::from_fn(|_| String::new());
            for (n, family) in FAMILIES.iter().enumerate() {
                let hex = lines[n + 6]
                    .strip_prefix(&format!("native.{family}="))
                    .ok_or("missing/duplicate native evidence field")?;
                if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("invalid native evidence encoding".into());
                }
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()?;
                evidence[n] = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                if result.enabled[n] == evidence[n].is_empty() {
                    return Err(
                        "native evidence must describe precisely the enabled backends".into(),
                    );
                }
            }
            result.native_evidence = Some(evidence);
        }
        Ok(result)
    }
    pub fn features(&self) -> Vec<&'static str> {
        FEATURES
            .iter()
            .enumerate()
            .filter_map(|(n, feature)| self.enabled[n].then_some(*feature))
            .collect()
    }
    pub fn verify_features(&self, actual: &[bool; 4], windows_base: bool) -> Result<(), String> {
        if actual != &self.enabled || windows_base != self.enabled[0] {
            return Err("Cargo features differ from the explicit prepared configuration; build.rs cannot enable/disable dependencies".into());
        }
        Ok(())
    }
    pub fn verify_native(&self, family: usize, actual: &[String]) -> Result<(), String> {
        let expected = self.native_evidence.as_ref().ok_or(
            "legacy P0 configuration lacks exact native evidence; prepare a new configuration",
        )?;
        if expected[family] != actual.join("\n") {
            return Err(
                "native dependency evidence changed since preparation; prepare a new configuration"
                    .into(),
            );
        }
        Ok(())
    }
}

fn output(program: &str, args: &[&str], evidence: &mut Vec<String>) -> Result<String, String> {
    let result = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("cannot start {program}: {e}"))?;
    let stdout = String::from_utf8_lossy(&result.stdout).trim().to_owned();
    evidence.push(format!(
        "{program} {args:?}; exit={}; stdout={stdout}; stderr={}",
        result.status,
        String::from_utf8_lossy(&result.stderr).trim()
    ));
    if !result.status.success() {
        return Err(format!("native probe failed: {program} {args:?}"));
    }
    Ok(stdout)
}

fn require_file(path: PathBuf, evidence: &mut Vec<String>) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("required native file missing: {}", path.display()));
    }
    evidence.push(format!("native_file={}", path.display()));
    Ok(())
}

pub fn probe(target: &str, family: usize, evidence: &mut Vec<String>) -> Result<(), String> {
    if !applicable(target, family) {
        return Err("backend is not applicable to the selected native target".into());
    }
    match family {
        0 => {
            let program_files = PathBuf::from(
                env::var_os("ProgramFiles(x86)").ok_or("ProgramFiles(x86) unavailable")?,
            );
            let vswhere = program_files.join("Microsoft Visual Studio/Installer/vswhere.exe");
            let installation = output(
                vswhere.to_str().ok_or("non-UTF-8 vswhere path")?,
                &[
                    "-version",
                    "[18.0,19.0)",
                    "-products",
                    "*",
                    "-latest",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-property",
                    "installationPath",
                ],
                evidence,
            )?;
            if installation.is_empty() {
                return Err("VS2026 MSVC x64 installation missing".into());
            }
            let root = PathBuf::from(installation);
            let version_path = root.join("VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt");
            require_file(version_path.clone(), evidence)?;
            let version = fs::read_to_string(version_path).map_err(|e| e.to_string())?;
            evidence.push(format!("msvc_toolset={}", version.trim()));
            require_file(
                root.join("VC/Tools/MSVC")
                    .join(version.trim())
                    .join("bin/Hostx64/x64/cl.exe"),
                evidence,
            )?;
            // Explicitly require an installed SDK; actual compiler/linker choice
            // is still independently recorded by the native acceptance build.
            let kits = env::var_os("WindowsSdkDir")
                .map(PathBuf::from)
                .unwrap_or_else(|| program_files.join("Windows Kits/10"));
            let mut versions: Vec<_> = fs::read_dir(kits.join("Lib"))
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .filter(|e| e.path().join("um/x64/kernel32.lib").is_file())
                .collect();
            versions.sort_by_key(|e| e.file_name());
            let sdk = versions.last().ok_or("Windows SDK x64 libraries missing")?;
            for name in [
                "kernel32.lib",
                "gdi32.lib",
                "user32.lib",
                "d3d11.lib",
                "dxgi.lib",
            ] {
                require_file(sdk.path().join("um/x64").join(name), evidence)?;
            }
        }
        1 | 2 => {
            let program = env::var("PKG_CONFIG").unwrap_or_else(|_| "pkg-config".into());
            let packages: &[&str] = if family == 1 {
                &["libpipewire-0.3", "libspa-0.2"]
            } else {
                &["wayland-client"]
            };
            output(&program, &["--version"], evidence)?;
            for package in packages {
                output(&program, &["--modversion", package], evidence)?;
                output(&program, &["--cflags", "--libs", package], evidence)?;
                let directory = output(&program, &["--variable=pcfiledir", package], evidence)?;
                require_file(
                    PathBuf::from(directory).join(format!("{package}.pc")),
                    evidence,
                )?;
            }
            if family == 1 {
                output(
                    &program,
                    &["--atleast-version=1.2.0", "libpipewire-0.3"],
                    evidence,
                )?;
                output("clang", &["--version"], evidence)?;
                let mut directories = Vec::new();
                if let Some(path) = env::var_os("LIBCLANG_PATH") {
                    directories.push(PathBuf::from(path));
                } else {
                    let llvm =
                        env::var("LLVM_CONFIG_PATH").unwrap_or_else(|_| "llvm-config".into());
                    if let Ok(path) = output(&llvm, &["--libdir"], evidence) {
                        directories.push(PathBuf::from(path));
                    }
                    if let Ok(path) = output("clang", &["--print-resource-dir"], evidence)
                        && let Some(path) = PathBuf::from(path).ancestors().nth(2)
                    {
                        directories.push(path.to_owned());
                    }
                    directories.extend(
                        ["/usr/lib", "/usr/lib64", "/usr/lib/x86_64-linux-gnu"].map(PathBuf::from),
                    );
                    if let Ok(entries) = fs::read_dir("/usr/lib") {
                        let mut installed: Vec<_> = entries
                            .filter_map(Result::ok)
                            .filter(|e| e.file_name().to_string_lossy().starts_with("llvm-"))
                            .map(|e| e.path().join("lib"))
                            .collect();
                        installed.sort();
                        directories.extend(installed);
                    }
                }
                let library = directories.into_iter().find_map(|path| {
                    if path.is_file(){return Some(path);}
                    let mut candidates:Vec<_>=fs::read_dir(path).ok()?.filter_map(Result::ok).map(|e|e.path()).collect();
                    candidates.sort();
                    candidates.into_iter().find(|p| {
                        let name=p.file_name().unwrap().to_string_lossy();
                        p.is_file() && (name.starts_with("libclang.so") || (name.starts_with("libclang-") && !name.starts_with("libclang-cpp") && name.contains(".so")))
                    })
                }).ok_or("libclang shared library missing; set LIBCLANG_PATH for a nonstandard installation")?;
                require_file(library, evidence)?;
            }
        }
        3 => {
            let sdk = output("xcrun", &["--sdk", "macosx", "--show-sdk-path"], evidence)?;
            output(
                "xcrun",
                &["--sdk", "macosx", "--show-sdk-version"],
                evidence,
            )?;
            output(
                "xcrun",
                &["--sdk", "macosx", "clang", "--version"],
                evidence,
            )?;
            for name in ["ScreenCaptureKit", "AppKit", "CoreMedia", "CoreVideo"] {
                require_file(
                    PathBuf::from(&sdk).join(format!(
                        "System/Library/Frameworks/{name}.framework/{name}.tbd"
                    )),
                    evidence,
                )?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_auto_disables_but_explicit_on_fails_and_off_stays_disabled() {
        assert_eq!(choose(Mode::Auto, false), Ok(false));
        assert!(choose(Mode::On, false).is_err());
        assert_eq!(choose(Mode::Off, true), Ok(false));
        assert_eq!(choose(Mode::On, true), Ok(true));
    }
    #[test]
    fn rejects_foreign_capabilities_contradictory_requests_and_extra_fields() {
        let valid = Configuration {
            target: "x86_64-unknown-linux-gnu".into(),
            modes: [Mode::Off, Mode::Auto, Mode::On, Mode::Off],
            enabled: [false, false, true, false],
            native_evidence: None,
        };
        let text = valid.encode();
        assert_eq!(Configuration::parse(&text).unwrap(), valid);
        for changed in [
            text.replace("windows=OFF,disabled", "windows=ON,enabled"),
            text.replace("wlr=ON,enabled", "wlr=ON,disabled"),
            text.replace("pipewire=AUTO,disabled", "pipewire=OFF,enabled"),
            text.clone() + "unknown=1\n",
            text.replace("format=1", "format=2"),
        ] {
            assert!(Configuration::parse(&changed).is_err());
        }
        assert!(
            valid
                .verify_features(&[false, true, true, false], false)
                .is_err()
        );
        assert!(valid.verify_features(&valid.enabled, true).is_err());
        assert!(valid.verify_features(&valid.enabled, false).is_ok());
    }
    #[test]
    fn rejects_changed_native_evidence_and_invalid_encoded_inputs() {
        let evidence = "native_file=/sdk/library.tbd\nversion=1.2.3".to_owned();
        let config = Configuration {
            target: "aarch64-apple-darwin".into(),
            modes: [Mode::Off, Mode::Off, Mode::Off, Mode::On],
            enabled: [false, false, false, true],
            native_evidence: Some([
                String::new(),
                String::new(),
                String::new(),
                evidence.clone(),
            ]),
        };
        let encoded = config.encode();
        let decoded = Configuration::parse(&encoded).unwrap();
        assert!(decoded.verify_native(3, &[evidence]).is_ok());
        assert!(
            decoded
                .verify_native(3, &["native_file=/different/sdk.tbd".into()])
                .is_err()
        );
        assert!(
            Configuration::parse(&encoded.replace("native.wlr=\n", "native.wlr=zz\n")).is_err()
        );
        assert!(
            Configuration::parse(&encoded.replace("native.wlr=\n", "native.wlr=00\n")).is_err()
        );
    }
}
