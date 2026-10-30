// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Callback-stage observations from the separate native diagnostic. Analysis
//! of a failed call never accepts that call or its late result.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Event {
    stage: String,
    request: u64,
    monotonic_ns: u64,
    thread: u64,
    object_present: bool,
    error_present: bool,
    error_code: i64,
    displays: u64,
    windows: u64,
    display: u64,
    width: u64,
    height: u64,
}
#[derive(Serialize)]
struct Summary {
    events: Vec<Event>,
    content_submissions: usize,
    sample_submissions: usize,
    unobserved_content_callbacks: Vec<u64>,
    unobserved_sample_callbacks: Vec<u64>,
    unobserved_original_handler_returns: Vec<u64>,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let (path,check_only)=match args {
        [path]=>(path,false),[path,flag] if flag=="--check-only"=>(path,true),
        _=>return Err("expected upstream-trace macos-capture-stages <external same-path observer directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<String> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read_to_string(path)?)
    };
    if read("diagnostic-only.txt")?
        != "observer_same_path=1\noriginal_oracle=0\nrust_comparison=0\n"
    {
        return Err("native stage analysis requires the separate diagnostic scope".into());
    }
    let consumer_log = read("consumer-log.txt")?;
    let consumer = analyze(&consumer_log)?;
    let scene = analyze(&read("scene-stderr.log")?)?;
    let exits = consumer_log
        .lines()
        .filter_map(|line| line.strip_prefix("exit="))
        .collect::<Vec<_>>();
    if exits.len() != 1 {
        return Err("missing or duplicated diagnostic process exit".into());
    }
    let exit: i32 = exits[0].parse()?;
    if exit.to_string() != exits[0] {
        return Err("noncanonical diagnostic exit".into());
    }
    let mut waits = Vec::new();
    for line in consumer_log
        .lines()
        .filter_map(|line| line.strip_prefix("dispatch_observer event=wait_return "))
    {
        let fields = line.split(' ').collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err("incomplete actual native wait return".into());
        }
        let slot: u32 = fields[0]
            .strip_prefix("slot=")
            .ok_or("missing native wait slot")?
            .parse()?;
        let result: i64 = fields[1]
            .strip_prefix("value=")
            .ok_or("missing native wait value")?
            .parse()?;
        waits.push((slot, result));
    }
    let report = serde_json::json!({"layout_version":1,"diagnostic_only":true,"source_capture_accepted":false,
        "paired_acceptance":false,"consumer_exit":exit,"native_wait_returns":waits,"consumer":consumer,"scene":scene,"artifacts":artifacts,
        "scope":"Only actual content-query/sample submission, callback entry and original-handler-return observations from the instrumented executable. Missing callbacks mean unobserved within this recorded process, not proven cancellation, permanent noncompletion or native owner drain. Width/height are first SCDisplay properties for content callbacks or original configured extents for sample calls, never inferred physical pixel dimensions. Callback results/errors and late observations cannot accept a failed original call. This analysis does not independently certify binary identity, target pixels or full P1; those have separate records and strict gates."});
    let output = root.join("capture-stages-analysis.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved native stage analysis differs from raw records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS capture_stages consumer_exit={exit} content_submissions={} sample_submissions={} unobserved_content_callbacks={} unobserved_sample_callbacks={} diagnostic_only=1 source_capture_acceptance=0 paired_acceptance=0 recorded",
        report["consumer"]["content_submissions"],
        report["consumer"]["sample_submissions"],
        report["consumer"]["unobserved_content_callbacks"],
        report["consumer"]["unobserved_sample_callbacks"]
    );
    Ok(())
}

fn analyze(text: &str) -> Result<Summary> {
    let mut events = Vec::new();
    let mut requests: BTreeMap<u64, Vec<Event>> = BTreeMap::new();
    for line in text
        .lines()
        .filter_map(|line| line.strip_prefix("capture_observer "))
    {
        let mut fields = BTreeMap::new();
        for part in line.split(' ') {
            let (key, value) = part
                .split_once('=')
                .ok_or("invalid native callback-stage field")?;
            if fields.insert(key, value).is_some() {
                return Err("duplicate native callback-stage field".into());
            }
        }
        if fields.keys().copied().collect::<Vec<_>>()
            != [
                "display",
                "displays",
                "error",
                "error_code",
                "height",
                "monotonic_ns",
                "object",
                "request",
                "stage",
                "thread",
                "thread_error",
                "width",
                "windows",
            ]
        {
            return Err("native callback-stage fields differ".into());
        }
        let integer = |name: &str| -> Result<u64> {
            let value: u64 = fields[name].parse()?;
            if value.to_string() != fields[name] {
                return Err("noncanonical native callback-stage integer".into());
            }
            Ok(value)
        };
        let boolean = |name: &str| -> Result<bool> {
            match integer(name)? {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err("invalid native callback-stage boolean".into()),
            }
        };
        let code: i64 = fields["error_code"].parse()?;
        let event = Event {
            stage: fields["stage"].into(),
            request: integer("request")?,
            monotonic_ns: integer("monotonic_ns")?,
            thread: integer("thread")?,
            object_present: boolean("object")?,
            error_present: boolean("error")?,
            error_code: code,
            displays: integer("displays")?,
            windows: integer("windows")?,
            display: integer("display")?,
            width: integer("width")?,
            height: integer("height")?,
        };
        if integer("thread_error")? != 0
            || event.thread == 0
            || event.monotonic_ns == 0
            || event.request == 0
            || code.to_string() != fields["error_code"]
            || (!event.error_present && code != 0)
        {
            return Err("native callback-stage clock/thread/error evidence invalid".into());
        }
        let (kind, phase) = event
            .stage
            .rsplit_once('_')
            .ok_or("invalid native callback stage")?;
        if !["content", "sample"].contains(&kind) {
            return Err("unknown native callback-stage operation".into());
        }
        let phase = match phase {
            "enter" => 0,
            "callback" => 1,
            "forwarded" => 2,
            _ => return Err("unknown native callback-stage phase".into()),
        };
        let history = requests.entry(event.request).or_default();
        if history.len() != phase
            || history.first().is_some_and(|first| {
                !first.stage.starts_with(kind) || first.monotonic_ns > event.monotonic_ns
            })
        {
            return Err("duplicate, reordered or substituted native callback request".into());
        }
        if phase == 2 {
            let mut previous = history[1].clone();
            previous.stage = event.stage.clone();
            previous.monotonic_ns = event.monotonic_ns;
            if previous != event || history[1].monotonic_ns > event.monotonic_ns {
                return Err("original handler return changed callback evidence/thread".into());
            }
        }
        if kind == "sample"
            && (event.displays != 0
                || event.windows != 0
                || event.display != 0
                || history.first().is_some_and(|first| {
                    (first.width, first.height) != (event.width, event.height)
                }))
        {
            return Err("sample callback changed submitted extent evidence".into());
        }
        if phase == 0
            && (event.error_present
                || event.error_code != 0
                || event.displays != 0
                || event.windows != 0
                || event.display != 0
                || (kind == "content"
                    && (event.object_present || event.width != 0 || event.height != 0)))
        {
            return Err("native submission was relabelled as a callback result".into());
        }
        history.push(event.clone());
        events.push(event);
    }
    if events.is_empty() {
        return Err("native callback-stage observations absent; older logs do not establish these boundaries".into());
    }
    let mut summary = Summary {
        events,
        content_submissions: 0,
        sample_submissions: 0,
        unobserved_content_callbacks: Vec::new(),
        unobserved_sample_callbacks: Vec::new(),
        unobserved_original_handler_returns: Vec::new(),
    };
    for (request, history) in requests {
        let content = history[0].stage == "content_enter";
        if content {
            summary.content_submissions += 1;
        } else {
            summary.sample_submissions += 1;
        }
        if history.len() == 1 {
            if content {
                summary.unobserved_content_callbacks.push(request);
            } else {
                summary.unobserved_sample_callbacks.push(request);
            }
        } else if history.len() == 2 {
            summary.unobserved_original_handler_returns.push(request);
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(stage: &str, request: u64, time: u64) -> String {
        format!(
            "capture_observer stage={stage} request={request} monotonic_ns={time} thread=7 thread_error=0 object=0 error=0 error_code=0 displays=0 windows=0 display=0 width=0 height=0\n"
        )
    }
    #[test]
    fn records_nested_cross_thread_callbacks_and_preserves_native_errors() {
        let content = |stage: &str, time| {
            event(stage, 1, time)
                .replace("thread=7", "thread=9")
                .replace("object=0", "object=1")
                .replace("displays=0", "displays=1")
                .replace("windows=0", "windows=2")
                .replace("display=0", "display=17")
                .replace("width=0", "width=1920")
                .replace("height=0", "height=1080")
        };
        let sample = |stage: &str, time| {
            event(stage, 2, time)
                .replace("thread=7", "thread=11")
                .replace("object=0", "object=1")
                .replace("width=0", "width=1920")
                .replace("height=0", "height=1080")
        };
        let input = event("content_enter", 1, 10)
            + &content("content_callback", 20)
            + &sample("sample_enter", 21).replace("thread=11", "thread=9")
            + &content("content_forwarded", 22)
            + &sample("sample_callback", 30)
            + &sample("sample_forwarded", 31);
        let summary = analyze(&input).unwrap();
        assert_eq!(
            (summary.content_submissions, summary.sample_submissions),
            (1, 1)
        );
        assert!(
            summary.unobserved_content_callbacks.is_empty()
                && summary.unobserved_sample_callbacks.is_empty()
                && summary.unobserved_original_handler_returns.is_empty()
        );
        let failed=input.replace("stage=sample_callback request=2 monotonic_ns=30 thread=11 thread_error=0 object=1 error=0 error_code=0",
            "stage=sample_callback request=2 monotonic_ns=30 thread=11 thread_error=0 object=0 error=1 error_code=-3801")
            .replace("stage=sample_forwarded request=2 monotonic_ns=31 thread=11 thread_error=0 object=1 error=0 error_code=0",
            "stage=sample_forwarded request=2 monotonic_ns=31 thread=11 thread_error=0 object=0 error=1 error_code=-3801");
        let summary = analyze(&failed).unwrap();
        assert!(summary.events[4].error_present);
        assert_eq!(summary.events[4].error_code, -3801);
    }
    #[test]
    fn retains_unobserved_callbacks_and_handler_returns_without_accepting_failure() {
        let input = event("content_enter", 1, 10)
            + &event("content_callback", 1, 20)
            + &event("content_forwarded", 1, 30)
            + &event("sample_enter", 2, 40);
        let result = analyze(&input).unwrap();
        assert_eq!(result.unobserved_sample_callbacks, [2]);
        assert!(result.unobserved_content_callbacks.is_empty());
        let result = analyze(&(input + &event("sample_callback", 2, 50))).unwrap();
        assert_eq!(result.unobserved_original_handler_returns, [2]);
        assert!(analyze("dispatch_observer event=wait_return slot=0 value=49\n").is_err());
    }
    #[test]
    fn rejects_duplicate_reordered_substituted_or_invalid_native_stage_evidence() {
        let good = event("content_enter", 1, 10)
            + &event("content_callback", 1, 20)
            + &event("content_forwarded", 1, 30);
        analyze(&good).unwrap();
        for bad in [
            good.clone() + &event("content_forwarded", 1, 40),
            good.replace("content_enter", "sample_enter"),
            good.replace("request=1 monotonic_ns=20", "request=2 monotonic_ns=20"),
            good.replace("thread_error=0", "thread_error=1"),
            good.replace("monotonic_ns=30", "monotonic_ns=19"),
            good.replace("error_code=0", "error_code=7"),
            good.replace("object=0", "object=2"),
            good.replace("width=0", "width=01"),
            good.replace("windows=0", "windows=0 unknown=1"),
        ] {
            assert!(analyze(&bad).is_err(), "accepted {bad}");
        }
    }
}
