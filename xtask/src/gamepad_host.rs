// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Whole original Gamepad outer host/control records, not desktop capture.
use crate::{Result, records};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
#[path = "../../tests/upstream-parity/windows-delegate/gamepad-cases.rs"]
mod cases;

fn fields(text: &str) -> Result<BTreeMap<&str, &str>> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let (k, v) = line.split_once('=').ok_or("Invalid Gamepad record line")?;
        if out.insert(k, v).is_some() {
            return Err("Duplicate Gamepad record field".into());
        }
    }
    if out.len() != 17 {
        return Err("Incomplete Gamepad record".into());
    }
    Ok(out)
}
fn expect(fields: &BTreeMap<&str, &str>, key: &str, value: impl ToString) -> Result<()> {
    if fields.get(key).copied() != Some(value.to_string().as_str()) {
        return Err(format!("Gamepad field {key} differs").into());
    }
    Ok(())
}
fn id(text: &str) -> Result<u8> {
    let v: u8 = text.parse()?;
    if v == 0 || v.to_string() != text {
        return Err("Invalid Gamepad owner identity".into());
    }
    Ok(v)
}
#[derive(Default)]
struct Capture {
    module: bool,
    live: BTreeSet<u8>,
    next: u8,
    view: Option<u8>,
}
impl Capture {
    fn event(&mut self, event: &str, case: cases::Case) -> Result<()> {
        match event {
            "module.load" if !self.module && self.live.is_empty() => {}
            "symbol.create" if !self.module => self.module = true,
            "symbol.destroy" if self.module => {}
            "module.unload" if self.module && self.live.is_empty() => self.module = false,
            _ if event.starts_with("unit.create:") && self.module => {
                if event != format!("unit.create:{}:{}:0:0", case.hwnd, case.method) {
                    return Err("Gamepad capture selectors/input None differ".into());
                }
            }
            _ if event.starts_with("unit.connect:") && self.module => {
                let owner = id(&event[13..])?;
                self.next = self
                    .next
                    .checked_add(1)
                    .ok_or("Gamepad identity overflow")?;
                if owner != self.next || !self.live.insert(owner) {
                    return Err("Repeated Gamepad capture owner".into());
                }
                self.view = Some(owner);
            }
            _ if event.starts_with("unit.drop:") && self.module => {
                if !self.live.remove(&id(&event[10..])?) {
                    return Err("Gamepad capture owner destroyed twice".into());
                }
            }
            _ if event.starts_with("unit.capture:") && self.module => {
                if !self.live.contains(&id(&event[13..])?) {
                    return Err("Gamepad used a dead capture owner".into());
                }
            }
            _ if event.starts_with("unit.inactive:") && self.module => {
                if !self.live.contains(&id(&event[14..])?) {
                    return Err("Gamepad inactive used a dead owner".into());
                }
            }
            _ => return Err(format!("Invalid Gamepad capture event {event}").into()),
        }
        Ok(())
    }
}
fn validate(case: cases::Case, text: &str) -> Result<()> {
    let f = fields(text)?;
    for (key, value) in [
        ("initial_connected", 0),
        ("initial_capture", 0),
        ("initial_inactive", 1),
        ("owners_closed", 1),
    ] {
        expect(&f, key, value)?;
    }
    let events = f["events"]
        .split(',')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let starts = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.starts_with("input.create:").then_some(i))
        .collect::<Vec<_>>();
    if starts.len() != 2 || starts[0] != 0 {
        return Err("Gamepad must attempt both outer host constructions".into());
    }
    let resets = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.starts_with("input.reset:").then_some(i))
        .collect::<Vec<_>>();
    if resets.len() > 1 {
        return Err("Gamepad reset is not one destructor operation".into());
    }
    let end = resets.first().copied().unwrap_or(events.len());
    let mut input = None;
    let mut next_input = 0u8;
    let mut capture = Capture::default();
    let mut output = "63".repeat(18);
    for phase in 0..2 {
        let host = case.host[phase];
        let flag = case.flags[phase];
        let selected = case.hwnd != 0 && case.method != 0;
        let native_init = selected && [0, 4].contains(&host);
        let connected = match host {
            1 => 0,
            2 | 3 => 2,
            _ if native_init && [32, 64, 128].contains(&flag) => 2,
            _ => 1,
        };
        expect(&f, &format!("connect{phase}"), connected)?;
        expect(
            &f,
            &format!("connected{phase}"),
            u8::from(connected == 1 && host != 4),
        )?;
        let stop = if phase == 0 { starts[1] } else { end };
        let segment = events
            .get(starts[phase]..stop)
            .ok_or("Gamepad reset precedes host calls")?;
        let mut prefix = vec![format!("input.create:{}", case.gamepad_type)];
        if host != 3 {
            next_input = next_input.checked_add(1).ok_or("Input identity overflow")?;
            prefix.push(format!("input.new:{next_input}"));
            if let Some(old) = input {
                prefix.push(format!("input.drop:{old}"));
            }
            input = Some(next_input);
            prefix.push(format!("input.connect:{next_input}"));
        }
        if !segment.starts_with(&prefix.iter().map(String::as_str).collect::<Vec<_>>()) {
            return Err("Gamepad host create/replace/connect order differs".into());
        }
        let body = &segment[prefix.len()..];
        let input_queries = body
            .iter()
            .filter(|e| e.starts_with("input."))
            .copied()
            .collect::<Vec<_>>();
        let expected_query = if connected == 1 {
            vec![format!(
                "input.connected:{}",
                input.ok_or("Connected without input")?
            )]
        } else {
            Vec::new()
        };
        if input_queries
            != expected_query
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        {
            return Err("Gamepad connected short circuit differs".into());
        }
        let count = |prefix: &str| body.iter().filter(|e| e.starts_with(prefix)).count();
        if count("module.load") != usize::from(native_init)
            || count("symbol.create") != usize::from(native_init && ![1, 32].contains(&flag))
            || count("symbol.destroy") != usize::from(native_init && ![1, 32].contains(&flag))
            || count("unit.create:")
                != usize::from(native_init && [0, 8, 16, 64, 128].contains(&flag))
            || count("unit.connect:") != usize::from(native_init && [0, 16, 128].contains(&flag))
        {
            return Err(
                "Gamepad optional capture init did not follow host success exactly once".into(),
            );
        }
        // Every initialization must finish before the outer connected query and
        // capture/inactive calls. Host false/error must retain all old owners.
        let mut runtime = false;
        let mut query = false;
        let mut calls = Vec::new();
        let mut inactive = Vec::new();
        for e in body {
            if e.starts_with("input.connected:") {
                if runtime {
                    return Err("Gamepad queried input after capture".into());
                }
                query = true;
                continue;
            }
            let is_runtime = e.starts_with("unit.capture:") || e.starts_with("unit.inactive:");
            if is_runtime {
                runtime = true;
                if let Some(owner) = e.strip_prefix("unit.capture:") {
                    calls.push(id(owner)?);
                } else {
                    inactive.push(id(&e[14..])?);
                }
            } else if runtime || query || !native_init {
                return Err(
                    "Gamepad host failure/reordering changed retained capture resources".into(),
                );
            }
            capture.event(e, case)?;
            if e.starts_with("unit.connect:") && flag == 16 {
                capture.view = None;
            }
        }
        let live = capture.view.filter(|v| capture.live.contains(v));
        let stale = capture.view.is_some() && live.is_none();
        if calls != live.into_iter().collect::<Vec<_>>() || inactive != calls {
            return Err("Gamepad did not forward through the retained live capture owner".into());
        }
        let capture_result = if stale {
            4
        } else if live.is_none() {
            0
        } else if [3, 5].contains(&case.outcome) {
            2
        } else {
            u8::from([1, 2].contains(&case.outcome))
        };
        let inactive_result = if stale {
            4
        } else if live.is_none() {
            1
        } else {
            case.inactive as u8
        };
        expect(&f, &format!("capture{phase}"), capture_result)?;
        expect(&f, &format!("inactive{phase}"), inactive_result)?;
        if let Some(owner) = live {
            match case.outcome {
                1 | 4 | 5 => output = format!("07{owner:02x}0b").repeat(6),
                2 => output = "empty".into(),
                _ => {}
            }
        }
        expect(&f, &format!("output{phase}"), &output)?;
        expect(&f, &format!("held{phase}"), &output)?;
    }
    let mut tail = Vec::new();
    if let Some(input) = input {
        tail.push(format!("input.reset:{input}"));
    }
    for owner in &capture.live {
        tail.push(format!("unit.drop:{owner}"));
    }
    if capture.module {
        tail.push("module.unload".into());
    }
    if let Some(input) = input {
        tail.push(format!("input.drop:{input}"));
    }
    if events[end..] != tail.iter().map(String::as_str).collect::<Vec<_>>() {
        return Err("Gamepad reset/view/loader/input destruction order differs".into());
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (path, check) = match args {
        [p] => (p, false),
        [p, flag] if flag == "--check-only" => (p, true),
        _ => return Err(
            "expected upstream-delegate gamepad-host <existing external records> [--check-only]"
                .into(),
        ),
    };
    if !Path::new(path).is_dir() {
        return Err("Gamepad records must already exist".into());
    }
    let root = records::external_root(Path::new(path))?;
    let mut inputs = Vec::new();
    let mut expected = BTreeSet::new();
    let matrix = cases::matrix();
    if matrix.len() != 72 {
        return Err("Gamepad host corpus changed; recharacterize scope".into());
    }
    if matrix
        .iter()
        .copied()
        .map(cases::Case::arguments)
        .collect::<BTreeSet<_>>()
        .len()
        != matrix.len()
    {
        return Err("Duplicate Gamepad argument scenario".into());
    }
    for (index, case) in matrix.iter().copied().enumerate() {
        let mut texts = Vec::new();
        for extension in ["source", "rust", "stderr"] {
            let name = format!("{index}.{extension}");
            let file = root.join(&name);
            expected.insert(name);
            inputs.push(records::artifact(&root, &file)?);
            let text = fs::read_to_string(file)?.replace("\r\n", "\n");
            if extension == "stderr" {
                if !text.is_empty() {
                    return Err("Original Gamepad emitted an unexpected error".into());
                }
            } else {
                validate(case, &text)?;
                texts.push(text);
            }
        }
        if texts[0] != texts[1] {
            return Err("Gamepad original/Rust control or ownership records differ".into());
        }
    }
    let mut actual = BTreeSet::new();
    for e in fs::read_dir(&root)? {
        let e = e?;
        if !e.file_type()?.is_file() {
            return Err("Unexpected Gamepad record directory/link".into());
        }
        let n = e
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 Gamepad file")?;
        if n != "gamepad-host-validation.json" {
            actual.insert(n);
        }
    }
    if actual != expected {
        return Err("Gamepad record inventory differs".into());
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    let report = serde_json::json!({"layout_version":1,"cases":72,"connect_calls_per_side":144,"registered_files":216,"input_actions":0,"native_captures":0,"full_P1_accepted":false,"inputs":inputs,
        "scope":"Whole unchanged F Gamepad manager/header/API versus private typed-host outer owner over the same CaptureDelegate. Real original public handles, C++ Mat/shared ownership and LibraryHolder/boost::function execute; ViGEm/input and DLL discovery are explicit fixtures. Validates host create/connect ordering, retained old delegate, warning-only capture init false, connected short circuit and reset-before-capture-before-input teardown. Undefined original stale views are observed without dispatch; Rust refuses them. No desktop/native DLL capture, ViGEm device/actions, arbitrary allocation/reset exceptions, public SDK or complete P1 acceptance."});
    let output = root.join("gamepad-host-validation.json");
    if check && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("Saved Gamepad host report differs".into());
        }
    } else if !check {
        records::create_json(&output, &report)?;
    }
    println!(
        "Gamepad host records cases=72 connect_calls_per_side=144 files=216 reset_before_capture_before_input=1 input_actions=0 native_capture=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}
