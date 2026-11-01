// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Included by owning private unit tests; no internals are exported for a model.
#[path = "cases.rs"]
mod cases;
#[cfg(any(
    all(target_os = "windows", feature = "prototype-windows"),
    target_os = "linux",
    all(target_os = "macos", feature = "macos-manager")
))]
pub(crate) use cases::cases as case_matrix;
pub fn compare(kind: &str, info: impl FnMut(u64, u64) -> String) {
    compare_rows(kind, kind, false, info);
}
#[cfg(target_os = "windows")]
pub fn compare_identity(mut query: impl FnMut(u64, u64) -> (bool, String, u64)) {
    compare_rows("win32", "win32-identity", true, |id, method| {
        let (success, uuid, features) = query(id, method);
        format!("{}\t{uuid}\t{features}", u8::from(success))
    });
}
fn compare_rows(
    kind: &str,
    directory_name: &str,
    identity: bool,
    mut info: impl FnMut(u64, u64) -> String,
) {
    use std::{fmt::Write, fs, path::PathBuf, process::Command};
    let model =
        std::env::var_os("MFS_P1_MANAGER_METADATA_MODEL").expect("explicit metadata source model");
    let base = PathBuf::from(
        std::env::var_os("MFS_P1_MANAGER_METADATA_ARTIFACTS").expect("external metadata artifacts"),
    );
    assert!(base.is_absolute());
    let product = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    if let Some(existing) = base.ancestors().find(|p| p.exists()) {
        assert!(!existing.canonicalize().unwrap().starts_with(&product));
    }
    fs::create_dir_all(&base).unwrap();
    let directory = base.join(directory_name);
    fs::create_dir(&directory).unwrap();
    let mut cases = String::new();
    let mut expected = Vec::new();
    for (id, method) in cases::cases(kind).expect("supported metadata kind") {
        writeln!(&mut cases, "{kind}\t{id}\t{method}").unwrap();
        expected.push(info(id, method));
    }
    let input = directory.join("cases.tsv");
    fs::write(&input, cases).unwrap();
    let mut command = Command::new(model);
    command.arg(&input);
    if identity {
        command.arg("--win32-identity");
    }
    let output = command.output().unwrap();
    fs::write(directory.join("source-stdout.txt"), &output.stdout).unwrap();
    fs::write(directory.join("source-stderr.txt"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "source metadata model failed: {:?}",
        output.status
    );
    let source = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = source.lines().collect();
    assert_eq!(lines.len(), expected.len());
    let mut rust = String::new();
    for (index, (line, value)) in lines.iter().zip(&expected).enumerate() {
        let (row, original) = line.split_once('\t').unwrap();
        assert_eq!(row.parse::<usize>().unwrap(), index);
        assert_eq!(original, value, "{kind} row {index}");
        writeln!(&mut rust, "{index}\t{value}").unwrap();
    }
    fs::write(directory.join("rust-stdout.txt"), rust).unwrap();
    if identity {
        println!(
            "manager identity kind=win32 cases={} exact_original_uuid_features=1 input_none=1 no_native_connection=1 passed",
            expected.len()
        );
    } else {
        println!(
            "manager metadata kind={kind} cases={} exact_original_json=1 input_none=1 no_native_connection=1 passed",
            expected.len()
        );
    }
}
