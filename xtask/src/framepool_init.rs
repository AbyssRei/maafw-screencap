// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Hidden/destroyed native initialization failures and real device notifications.
use crate::{Result, records};
use std::{collections::BTreeSet, fs, path::Path};

const CALLS: &str = "cycle\tcall\tvalid\tvisible\tbefore_slots\tafter_slots\tno_image\tprevious_final\tcaller\twindow_thread\tbegin_qpc\tend_qpc";
const LIFETIMES: &str =
    "index\tattached\tfinal\tattach_thread\tfinal_thread\tattach_qpc\tfinal_qpc";

fn table(text: &str, header: &str, count: usize, columns: usize) -> Result<Vec<Vec<i128>>> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err("Initialization record header differs".into());
    }
    let rows = lines
        .map(|line| {
            let mut values = Vec::new();
            for field in line.split('\t') {
                let value: i128 = field.parse()?;
                if value.to_string() != field {
                    return Err("Noncanonical initialization scalar".into());
                }
                values.push(value);
            }
            if values.len() != columns {
                return Err("Initialization record column count differs".into());
            }
            Ok(values)
        })
        .collect::<Result<Vec<_>>>()?;
    if rows.len() != count {
        return Err("Incomplete or extra initialization records".into());
    }
    Ok(rows)
}
fn calls(text: &str, rust: bool) -> Result<Vec<Vec<i128>>> {
    let header = if rust {
        format!("{CALLS}\tnative_hresult")
    } else {
        CALLS.to_owned()
    };
    let rows = table(text, &header, 80, if rust { 13 } else { 12 })?;
    let mut caller = None;
    let mut end = 0;
    for (index, row) in rows.iter().enumerate() {
        let (cycle, call) = (index / 4, index % 4);
        if row[..12].iter().any(|n| u64::try_from(*n).is_err())
            || row[..8]
                != [
                    cycle as i128,
                    call as i128,
                    i128::from(call < 2),
                    0,
                    if call == 0 || call == 3 { 0 } else { 7 },
                    if call < 2 { 7 } else { 0 },
                    1,
                    i128::from(call == 1 || call == 2),
                ]
            || row[8] <= 0
            || u32::try_from(row[8]).is_err()
            || *caller.get_or_insert(row[8]) != row[8]
            || row[9] != if call < 2 { row[8] } else { 0 }
            || row[10] <= end
            || row[11] < row[10]
            || (rust && row[12] != if call < 2 { -2147024809 } else { -2005270527 })
        {
            return Err("Initialization outcome, native state, caller or clock differs".into());
        }
        end = row[11];
    }
    Ok(rows)
}
fn lifetimes(text: &str, calls: &[Vec<i128>]) -> Result<Vec<Vec<i128>>> {
    let rows = table(text, LIFETIMES, 40, 7)?;
    for (index, row) in rows.iter().enumerate() {
        let previous = &calls[(index / 2) * 4 + index % 2];
        let next = &calls[(index / 2) * 4 + index % 2 + 1];
        if row.iter().any(|n| u64::try_from(*n).is_err())
            || row[..3] != [index as i128, 1, 1]
            || row[3] != previous[8]
            || row[4] <= 0
            || u32::try_from(row[4]).is_err()
            || row[5] < previous[11]
            || row[5] > next[10]
            || row[6] < next[10]
            || row[6] > next[11]
        {
            return Err(
                "Old native device survived replacement or callback identity/clock differs".into(),
            );
        }
    }
    Ok(rows)
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => {
            return Err(
                "expected upstream-trace framepool-init <external matrix> [--check-only]".into(),
            );
        }
    };
    if !Path::new(path).is_dir() {
        return Err("Initialization observations must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut expected = BTreeSet::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        expected.insert(name.to_owned());
        let p = root.join(name);
        inputs.push(records::artifact(&root, &p)?);
        Ok(fs::read(p)?)
    };
    let mut observations = Vec::new();
    for side in ["source", "rust"] {
        if read(&format!("{side}/abi.tsv"))? != b"1\t8\t8\t8\t4\t4\t4\n" {
            return Err("Native initialization fixture ABI differs".into());
        }
        let c = calls(
            std::str::from_utf8(&read(&format!("{side}/calls.tsv"))?)?,
            side == "rust",
        )?;
        let l = lifetimes(
            std::str::from_utf8(&read(&format!("{side}/lifetimes.tsv"))?)?,
            &c,
        )?;
        observations.push(serde_json::json!({"side":side,"calls":c,"lifetimes":l}));
    }
    let log = String::from_utf8(read("source-log.txt")?)?;
    let item = "CreateForWindow GraphicsCaptureItem failed [ret=-2147024809]";
    let device = "D3D11CreateDeviceAndSwapChain failed [ret=-2005270527]";
    if log.matches(item).count()!=40 || log.matches(device).count()!=40
        || log.matches("CreateForWindow GraphicsCaptureItem failed").count()!=40
        || log.matches("D3D11CreateDeviceAndSwapChain failed").count()!=40
        || log.lines().filter(|line| line.starts_with("original FramePool initialization")).count()!=1
        || log.lines().filter(|line| *line=="original FramePool initialization cycles=20 calls=80 hidden_rejections=40 destroyed_rejections=40 final_device_notifications=40 native_captures=0 full_P1_acceptance=0 passed").count()!=1
    {return Err("Actual original initialization HRESULT log or completion differs".into());}
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 initialization entry")?;
        let kind = entry.file_type()?;
        if ["source", "rust"].contains(&name.as_str()) && kind.is_dir() {
            for child in fs::read_dir(entry.path())? {
                let child = child?;
                if !child.file_type()?.is_file() {
                    return Err("Unexpected initialization evidence link/directory".into());
                }
                actual.insert(format!(
                    "{name}/{}",
                    child
                        .file_name()
                        .into_string()
                        .map_err(|_| "Non-UTF8 initialization file")?
                ));
            }
        } else if name == "source-log.txt" && kind.is_file() {
            actual.insert(name);
        } else if name != "init-validation.json" || !kind.is_file() {
            return Err("Unexpected initialization evidence entry".into());
        }
    }
    if actual != expected || inputs.len() != 7 {
        return Err("Initialization file set differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cycles_per_side":20,"calls":160,"real_native_rejections":160,
        "native_device_final_notifications":80,"native_captures":0,"registered_files":7,"observations":observations,"inputs":inputs,
        "native_session_callback_quiescence_proved":false,"full_P1_accepted":false,
        "scope":"Unchanged F class/Rust hidden-window CreateForWindow E_INVALIDARG and then actual destroyed-HWND D3D DXGI_ERROR_INVALID_CALL. Device/context/swapchain stay after item failure and are released before replacement returns; actual SetPrivateDataInterface IUnknown final notifications bind old device lifetimes without retaining devices. Native callbacks use pinned process-lifetime atomic records, not caller borrows. No frame, conversion, general GPU accounting, device-loss/aborting-OOM or complete initialization/failure/P1 acceptance; old pixel failures remain unchanged."});
    let output = root.join("init-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved initialization report differs".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "FramePool initialization calls=160 native_rejections=160 final_device_notifications=80 native_captures=0 files=7 full_P1_acceptance=0 passed"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (String, String) {
        let mut c = format!("{CALLS}\tnative_hresult\n");
        let mut l = format!("{LIFETIMES}\n");
        for index in 0..80 {
            let cycle = index / 4;
            let call = index % 4;
            let begin = 1000 + index * 100;
            let end = begin + 50;
            c.push_str(&format!(
                "{cycle}\t{call}\t{}\t0\t{}\t{}\t1\t{}\t7\t{}\t{begin}\t{end}\t{}\n",
                u8::from(call < 2),
                if call == 0 || call == 3 { 0 } else { 7 },
                if call < 2 { 7 } else { 0 },
                u8::from(call == 1 || call == 2),
                if call < 2 { 7 } else { 0 },
                if call < 2 {
                    -2147024809_i64
                } else {
                    -2005270527_i64
                }
            ));
            if call < 2 {
                l.push_str(&format!(
                    "{}\t1\t1\t7\t9\t{}\t{}\n",
                    2 * cycle + call,
                    end + 10,
                    begin + 110
                ));
            }
        }
        (c, l)
    }
    #[test]
    fn rejects_wrong_failure_states_calls_codes_and_clocks() {
        let (c, l) = fixture();
        let rows = calls(&c, true).unwrap();
        lifetimes(&l, &rows).unwrap();
        for changed in [
            c.replacen("\t0\t7\t1\t0\t7", "\t0\t0\t1\t0\t7", 1),
            c.replacen("-2147024809", "0", 1),
            c.replacen("1000\t1050", "0\t1050", 1),
            c.replacen("1\t1\t0\t7\t7", "1\t0\t0\t7\t7", 1),
            c.replacen("0\t0\t1", "00\t0\t1", 1),
        ] {
            assert_ne!(changed, c);
            assert!(calls(&changed, true).is_err());
        }
        assert!(calls(&format!("{c}{}", c.lines().last().unwrap()), true).is_err());
    }
    #[test]
    fn rejects_missing_or_late_device_final_notifications() {
        let (c, l) = fixture();
        let rows = calls(&c, true).unwrap();
        for changed in [
            l.replacen("0\t1\t1", "0\t1\t0", 1),
            l.replacen("\t7\t9\t1060\t1110", "\t8\t9\t1060\t1110", 1),
            l.replacen("1060\t1110", "1060\t2000", 1),
            l.replacen("1060\t1110", "1000\t1110", 1),
        ] {
            assert_ne!(changed, l);
            assert!(lifetimes(&changed, &rows).is_err());
        }
    }
}
