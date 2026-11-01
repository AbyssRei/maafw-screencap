// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private scene-host and capture-fixture helpers, outside the SDK.
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(crate) fn fingerprint(path: &Path) -> Result<(u64, u64)> {
    let mut file = File::open(path)?;
    let mut length = 0_u64;
    let mut hash = 0xcbf29ce484222325_u64;
    let mut buffer = [0_u8; 32768];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        length = length
            .checked_add(u64::try_from(count)?)
            .ok_or("executable length overflow")?;
        for value in &buffer[..count] {
            hash = (hash ^ u64::from(*value)).wrapping_mul(0x100000001b3);
        }
    }
    Ok((length, hash))
}
pub(crate) fn fields(text: &str) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("invalid same-binary field")?;
        if key.is_empty()
            || value.is_empty()
            || value.contains(['\t', '\r', '\n'])
            || result.insert(key.into(), value.into()).is_some()
        {
            return Err("duplicate/empty same-binary field".into());
        }
    }
    Ok(result)
}
pub(crate) fn scalar(value: &str) -> Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err("noncanonical same-binary integer".into());
    }
    Ok(parsed)
}
pub(crate) fn identity(
    path: &Path,
    pid: u32,
    mode: u32,
    role: &str,
    module: Option<&Path>,
) -> Result<String> {
    if !matches!(role, "producer" | "consumer") || mode > 1 {
        return Err("invalid same-binary role/mode".into());
    }
    let canonical = std::fs::canonicalize(path)?;
    let text = canonical
        .to_str()
        .ok_or("fixture executable path is not UTF-8")?;
    if text.contains(['\t', '\r', '\n']) {
        return Err("invalid fixture path field".into());
    }
    let (length, hash) = fingerprint(&canonical)?;
    let mut output = format!(
        "schema=1\nrole={role}\nmode={mode}\npid={pid}\nprogram_path={text}\nprogram_length={length}\nprogram_fnv64={hash}\n"
    );
    if let Some(module) = module {
        let canonical = std::fs::canonicalize(module)?;
        let text = canonical
            .to_str()
            .ok_or("fixture module path is not UTF-8")?;
        if text.contains(['\t', '\r', '\n']) {
            return Err("invalid module path field".into());
        }
        output +=
            &format!("module_path={text}\nmain_thread=1\nmodule_retained_until_process_exit=1\n");
    }
    Ok(output)
}
