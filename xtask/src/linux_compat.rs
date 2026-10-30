// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use crate::{Result, records};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
#[path = "../../tests/upstream-parity/linux-manager/compat-cases.rs"]
mod corpus;
pub fn observe(args: &[String]) -> Result<()> {
    let [executable, path] = args else {
        return Err(
            "expected upstream-config linux-compat <original executable> <new directory>".into(),
        );
    };
    let executable = fs::canonicalize(executable)?;
    let identity = records::hash(&executable)?;
    if Path::new(path).exists() {
        return Err("compat observation directory must be new".into());
    }
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut created = 0;
    let mut residuals = 0;
    for case in corpus::cases() {
        let dir = root.join(&case.name);
        fs::create_dir(&dir)?;
        let path_arg = case
            .path
            .as_ref()
            .map_or_else(|| "<null>".to_owned(), |b| hex(b));
        let arguments = vec![
            "--compat".to_owned(),
            if case.kwin { "kwin" } else { "wlr" }.to_owned(),
            path_arg,
            case.width.to_string(),
            case.height.to_string(),
            case.vk.to_string(),
            case.open.to_string(),
            case.fd.to_string(),
            case.node.to_string(),
        ];
        records::create_json(&dir.join("arguments.json"), &arguments)?;
        let result = Command::new(&executable).args(&arguments).output()?;
        fs::write(dir.join("stdout.txt"), &result.stdout)?;
        fs::write(dir.join("stderr.txt"), &result.stderr)?;
        if !result.status.success() {
            return Err(format!("Original compat {} exited {}", case.name, result.status).into());
        }
        source(&case, std::str::from_utf8(&result.stdout)?)?;
        created += usize::from(case.created());
        residuals += case.residual();
        for file in ["arguments.json", "stdout.txt", "stderr.txt"] {
            artifacts.push(records::artifact(&root, &dir.join(file))?);
        }
    }
    if records::hash(&executable)? != identity {
        return Err("compat executable changed during observation".into());
    }
    records::create_json(
        &root.join("observations.json"),
        &serde_json::json!({"layout_version":1,"cases":120,"created":created,"residual_helpers":residuals,"executable_sha256":identity,"artifacts":artifacts,
        "scope":"Complete fixed original Wlr/KWin compat cpp and real original configuration/queried manager/virtual ABI/Mat. Host Portal calls are scripted and fake helper ownership is separately observed/reclaimed after original failure. No native authorization, input device, fd consumption, connect/capture or full P1 acceptance."}),
    )?;
    println!(
        "Original Linux compat cases=120 created={created} source_open_failure_residuals={residuals} host_call_order=1 no_native_connection=1 passed"
    );
    Ok(())
}
pub fn verify(args: &[String]) -> Result<()> {
    let (path, check) =
        match args {
            [p] => (p, false),
            [p, f] if f == "--check-only" => (p, true),
            _ => return Err(
                "expected upstream-config linux-compat-records <external matrix> [--check-only]"
                    .into(),
            ),
        };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut created = 0;
    let mut residuals = 0;
    for case in corpus::cases() {
        let path = root.join("source").join(&case.name).join("stdout.txt");
        let text = fs::read_to_string(&path)?;
        let fields = source(&case, &text)?;
        artifacts.push(records::artifact(&root, &path)?);
        let prefix = root.join("rust").join(&case.name);
        for (extension, expected) in [("events", case.events().as_bytes().to_vec())]
            .into_iter()
            .chain(if case.created() {
                vec![
                    ("info", unhex(fields["info_hex"])?),
                    ("uuid", unhex(fields["uuid_hex"])?),
                ]
            } else {
                Vec::new()
            })
        {
            let path = prefix.with_extension(extension);
            if fs::read(&path)? != expected {
                return Err(format!("Rust compat {} {extension} differs", case.name).into());
            }
            artifacts.push(records::artifact(&root, &path)?);
        }
        if !case.created()
            && (prefix.with_extension("info").exists() || prefix.with_extension("uuid").exists())
        {
            return Err("rejected compat constructor published a configuration".into());
        }
        created += usize::from(case.created());
        residuals += case.residual();
    }
    let report = serde_json::json!({"layout_version":1,"cases":120,"created":created,"residual_helpers":residuals,"artifacts":artifacts,
        "scope":"Actual original compat source/Rust configuration/metadata/UUID and scripted host call-order records. Original failed-Open helper residual is observed before explicit fixture reclaim, not product cleanup success. Input configuration values are retained. No actual Portal authorization/native fd ownership/input connection/capture or complete compat/P1 acceptance."});
    let path = root.join("compat-validation.json");
    if check && path.exists() {
        let saved: serde_json::Value = records::read(&path)?;
        if saved != report {
            return Err("saved compat validation differs from actual records".into());
        }
    } else if !check {
        records::create_json(&path, &report)?;
    }
    println!(
        "Linux compat cases=120 created={created} source_open_failure_residuals={residuals} actual_records_verified=1 native_capture_acceptance=0 passed"
    );
    Ok(())
}
fn source<'a>(case: &corpus::Case, text: &'a str) -> Result<BTreeMap<&'a str, &'a str>> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (k, v) = line.split_once('=').ok_or("invalid source compat field")?;
        if fields.insert(k, v).is_some() {
            return Err("duplicate source compat field".into());
        }
    }
    if fields.len() != if case.created() { 7 } else { 5 }
        || fields.get("created") != Some(&if case.created() { "1" } else { "0" })
        || fields.get("portal_events") != Some(&case.events())
        || fields
            .get("portal_live")
            .and_then(|s| s.parse::<usize>().ok())
            != Some(case.residual())
        || fields
            .get("fixture_reclaimed")
            .and_then(|s| s.parse::<usize>().ok())
            != Some(case.residual())
        || fields.get("no_native_connection") != Some(&"1")
    {
        return Err(format!(
            "Original compat {} state/call-order/residual differs",
            case.name
        )
        .into());
    }
    if case.created() {
        let uuid = unhex(fields.get("uuid_hex").ok_or("missing original UUID")?)?;
        let info = unhex(fields.get("info_hex").ok_or("missing original metadata")?)?;
        let (expected_info, expected_uuid) = expected_metadata(case);
        if info != expected_info || uuid != expected_uuid {
            return Err("Original compat metadata differs from explicit arguments/defaults".into());
        }
    }
    Ok(fields)
}
fn expected_metadata(case: &corpus::Case) -> (Vec<u8>, Vec<u8>) {
    // Independent canonical field order from original Config.h/get_info, not
    // the migrated parser/serializer. Keep raw C-string bytes, including FF.
    fn quote(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![b'"'];
        for b in bytes {
            match *b {
                b'"' => out.extend_from_slice(br#"\""#),
                b'\\' => out.extend_from_slice(br#"\\"#),
                8 => out.extend_from_slice(br#"\b"#),
                12 => out.extend_from_slice(br#"\f"#),
                b'\n' => out.extend_from_slice(br#"\n"#),
                b'\r' => out.extend_from_slice(br#"\r"#),
                b'\t' => out.extend_from_slice(br#"\t"#),
                0..=31 => out.extend_from_slice(format!("\\u{:04x}", b).as_bytes()),
                _ => out.push(*b),
            }
        }
        out.push(b'"');
        out
    }
    let raw = case.path.as_deref().unwrap_or_default();
    let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
    let raw = &raw[..end];
    let (socket, device, method, input, fd, node, width, height) = if case.kwin {
        (
            b"".as_slice(),
            raw,
            4,
            2,
            case.fd,
            case.node,
            case.width,
            case.height,
        )
    } else {
        (raw, b"/dev/uinput".as_slice(), 1, 1, -1, 0, 0, 0)
    };
    let mut info=format!("{{\"eis_socket_path\":\"\",\"input_method\":{input},\"pw_node_id\":{node},\"pw_socket_fd\":{fd},\"screencap_method\":{method},\"type\":\"linux\",\"uinput_path\":").into_bytes();
    info.extend(quote(device));
    info.extend(format!(",\"uinput_screen_height\":{height},\"uinput_screen_width\":{width},\"use_win32_vk_code\":{},\"wlr_socket_path\":",case.vk!=0).bytes());
    info.extend(quote(socket));
    info.push(b'}');
    let mut uuid = if case.kwin {
        format!("{fd}_{node}").into_bytes()
    } else {
        raw.to_vec()
    };
    uuid.push(b'_');
    uuid.extend_from_slice(if case.kwin { device } else { socket });
    (info, uuid)
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}
fn unhex(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid original compat hex".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| Ok(u8::from_str_radix(std::str::from_utf8(b)?, 16)?))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_silent_helper_cleanup_or_changed_host_call_order() {
        let case = corpus::cases()
            .into_iter()
            .find(|c| c.residual() == 1)
            .unwrap();
        let text = "created=0\nportal_events=create,open\nportal_live=1\nfixture_reclaimed=1\nno_native_connection=1\n";
        assert!(source(&case, text).is_ok());
        assert!(source(&case, &text.replace("portal_live=1", "portal_live=0")).is_err());
        assert!(source(&case, &text.replace("create,open", "create,open,destroy")).is_err());
        assert!(source(&case, &format!("{text}portal_live=1\n")).is_err());
    }
}
