// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Callback-stage observations from the separate native diagnostic. Analysis
//! of a failed call never accepts that call or its late result.
use crate::{Result, records};
use serde::Serialize;
use std::{collections::BTreeMap, fs, path::Path};
#[path = "macos_input_weak.rs"]
mod input_weak;

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    late_observations: Vec<LateObservation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    block_lease_observations: Vec<BlockLeaseObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_weak_observations: Option<input_weak::Analysis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct LateObservation {
    phase: String,
    monotonic_ns: u64,
    thread: u64,
    submitted: u32,
    forwarded: u32,
    pending: u32,
    semaphore_pending: u32,
    budget_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct BlockLeaseObservation {
    phase: String,
    monotonic_ns: u64,
    thread: u64,
    acquired: u32,
    released: u32,
    live: u32,
    budget_ns: u64,
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let (path, flags) = args.split_first().ok_or("missing native stage directory")?;
    let (mut check_only, mut require_late, mut require_blocks, mut require_inputs, mut distinct) =
        (false, false, false, false, false);
    for flag in flags {
        let selected = match flag.as_str() {
            "--check-only" => &mut check_only,
            "--require-late-snapshots" => &mut require_late,
            "--require-block-leases" => &mut require_blocks,
            "--require-input-weak" => &mut require_inputs,
            "--distinct-scene-path" => &mut distinct,
            _ => return Err("unknown native stage analysis flag".into()),
        };
        if std::mem::replace(selected, true) {
            return Err("duplicate native stage analysis flag".into());
        }
    }
    require_inputs |= distinct;
    require_blocks |= require_inputs;
    require_late |= require_blocks;
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<String> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read_to_string(path)?)
    };
    if read("diagnostic-only.txt")? != crate::macos_display_patch::observer_scope(distinct) {
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
    if require_late && (exit != 0 || distinct) && consumer.late_observations.len() != 2 {
        return Err("current failed diagnostic lacks required late callback snapshots".into());
    }
    if require_blocks && (exit != 0 || distinct) && consumer.block_lease_observations.len() != 2 {
        return Err(
            "current failed diagnostic lacks required block capture lease snapshots".into(),
        );
    }
    if require_inputs && consumer.input_weak_observations.is_none() {
        return Err("current diagnostic lacks actual native input weak observations".into());
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
    let mut report = serde_json::json!({"layout_version":1,"diagnostic_only":true,"source_capture_accepted":false,
        "paired_acceptance":false,"consumer_exit":exit,"native_wait_returns":waits,"consumer":consumer,"scene":scene,"artifacts":artifacts,
        "scope":"Only actual content-query/sample submission, callback entry and original-handler-return observations from the instrumented executable. Missing callbacks mean unobserved within this recorded process, not proven cancellation, permanent noncompletion or native owner drain. Width/height are first SCDisplay properties for content callbacks or original configured extents for sample calls, never inferred physical pixel dimensions. Callback results/errors and late observations cannot accept a failed original call. This analysis does not independently certify binary identity, target pixels or full P1; those have separate records and strict gates."});
    if distinct {
        report["same_executable_path"] = false.into();
        report["requires_post_return_snapshots"] = true.into();
    }
    if !report["consumer"]["late_observations"].is_null()
        || !report["scene"]["late_observations"].is_null()
    {
        report["late_observation_scope"] = "Timestamped submission/handler-return snapshots include fixture metadata requests before any original semaphore. Outstanding callbacks or an observed unsignalled original wait require the full recorded fifteen-second post-failure observation. Submission/handler-return counts are checked against the preceding native stage lines; the semaphore count is a separate call-site scalar observation. A zero pending count observes handler return only, not native block disposal, unregister, cancellation or capture acceptance. Older logs without these snapshots retain their original analysis and provide no such observation-window evidence.".into();
    }
    if require_late {
        report["requires_late_snapshots_on_failure"] = true.into();
    }
    if !report["consumer"]["block_lease_observations"].is_null()
        || !report["scene"]["block_lease_observations"].is_null()
    {
        report["block_lease_observation_scope"] = "Separate atomic counts observe construction/copy/destruction of the diagnostic wrapper's captured C++ token, including local and stack copies. They are not native block reference counts, callback-return counts, cancellation or whole-session drain. Each snapshot is taken separately from callback/semaphore counts; matching phases do not imply simultaneous observations. Original native selectors, caller wait and failure status are unchanged.".into();
    }
    if require_blocks {
        report["requires_block_lease_snapshots_on_failure"] = true.into();
    }
    if require_inputs {
        report["requires_native_input_weak_observations"] = true.into();
    }
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
    if require_late {
        println!(
            "macOS callback_observation required_on_failure=1 source_capture_acceptance=0 native_owner_drain=0 recorded"
        );
    }
    if require_blocks {
        println!(
            "macOS block_lease_observation required_on_failure=1 callback_return_is_disposal=0 native_owner_drain=0 recorded"
        );
    }
    if require_inputs {
        println!(
            "macOS input_weak_observation required=1 native_object_snapshot_owners_escape=0 native_request_disposal=0 recorded"
        );
    }
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
    let late_observations = late_observations(text)?;
    let block_lease_observations = block_lease_observations(text, &late_observations)?;
    let input_weak_observations =
        input_weak::analyze(text, &events, &late_observations, &block_lease_observations)?;
    let mut summary = Summary {
        events,
        content_submissions: 0,
        sample_submissions: 0,
        unobserved_content_callbacks: Vec::new(),
        unobserved_sample_callbacks: Vec::new(),
        unobserved_original_handler_returns: Vec::new(),
        late_observations,
        block_lease_observations,
        input_weak_observations,
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

fn late_observations(text: &str) -> Result<Vec<LateObservation>> {
    let mut submitted = 0_u64;
    let mut forwarded = 0_u64;
    let mut observations = Vec::new();
    for line in text.lines() {
        if let Some(line) = line.strip_prefix("capture_observer ") {
            // analyze() has already validated every stage/identity. These two
            // transitions share the native stderr lock with the snapshots.
            let stage = line
                .split(' ')
                .find_map(|part| part.strip_prefix("stage="))
                .ok_or("native stage has no operation")?;
            if matches!(stage, "content_enter" | "sample_enter") {
                submitted = submitted
                    .checked_add(1)
                    .ok_or("submission count overflow")?;
            } else if matches!(stage, "content_forwarded" | "sample_forwarded") {
                forwarded = forwarded.checked_add(1).ok_or("handler count overflow")?;
            }
            continue;
        }
        let Some(line) = line.strip_prefix("capture_pending ") else {
            continue;
        };
        let mut fields = BTreeMap::new();
        for part in line.split(' ') {
            let (key, value) = part
                .split_once('=')
                .ok_or("invalid callback snapshot field")?;
            if fields.insert(key, value).is_some() {
                return Err("duplicate callback snapshot field".into());
            }
        }
        if fields.keys().copied().collect::<Vec<_>>()
            != [
                "budget_ns",
                "forwarded",
                "monotonic_ns",
                "pending",
                "phase",
                "schema",
                "semaphore_pending",
                "submitted",
                "thread",
                "thread_error",
                "valid",
            ]
        {
            return Err("callback snapshot fields differ".into());
        }
        let integer = |name: &str| -> Result<u64> {
            let value: u64 = fields[name].parse()?;
            if value.to_string() != fields[name] {
                return Err("noncanonical callback snapshot integer".into());
            }
            Ok(value)
        };
        let value = LateObservation {
            phase: fields["phase"].into(),
            monotonic_ns: integer("monotonic_ns")?,
            thread: integer("thread")?,
            submitted: u32::try_from(integer("submitted")?)?,
            forwarded: u32::try_from(integer("forwarded")?)?,
            pending: u32::try_from(integer("pending")?)?,
            semaphore_pending: u32::try_from(integer("semaphore_pending")?)?,
            budget_ns: integer("budget_ns")?,
        };
        if integer("schema")? != 1
            || integer("valid")? != 1
            || integer("thread_error")? != 0
            || value.monotonic_ns == 0
            || value.thread == 0
            || value.budget_ns != 15_000_000_000
            || value.semaphore_pending > 16
            || u64::from(value.submitted) != submitted
            || u64::from(value.forwarded) != forwarded
            || value.submitted.checked_sub(value.forwarded) != Some(value.pending)
            || observations.len() >= 2
            || value.phase != ["begin", "end"][observations.len()]
        {
            return Err("callback snapshot scope, counts or observation phase differs".into());
        }
        observations.push(value);
    }
    if !observations.is_empty() {
        if observations.len() != 2 {
            return Err("incomplete callback observation window".into());
        }
        let begin = &observations[0];
        let end = &observations[1];
        let elapsed = end
            .monotonic_ns
            .checked_sub(begin.monotonic_ns)
            .ok_or("callback observation clock reversed")?;
        if begin.thread != end.thread
            || ((end.pending != 0 || end.semaphore_pending != 0) && elapsed < end.budget_ns)
        {
            return Err("unresolved callback observation ended early or changed caller".into());
        }
    }
    Ok(observations)
}

fn block_lease_observations(
    text: &str,
    late: &[LateObservation],
) -> Result<Vec<BlockLeaseObservation>> {
    let mut observations = Vec::new();
    let mut pending_phase = None;
    for line in text.lines() {
        if let Some(line) = line.strip_prefix("capture_pending ") {
            // late_observations() already checked this line and its field set.
            pending_phase = line
                .split(' ')
                .find_map(|field| field.strip_prefix("phase="));
            continue;
        }
        let Some(line) = line.strip_prefix("capture_block_leases ") else {
            continue;
        };
        let mut fields = BTreeMap::new();
        for part in line.split(' ') {
            let (key, value) = part.split_once('=').ok_or("invalid block lease field")?;
            if fields.insert(key, value).is_some() {
                return Err("duplicate block lease field".into());
            }
        }
        if fields.keys().copied().collect::<Vec<_>>()
            != [
                "acquired",
                "budget_ns",
                "live",
                "monotonic_ns",
                "phase",
                "released",
                "schema",
                "thread",
                "thread_error",
                "valid",
            ]
        {
            return Err("block capture lease fields differ".into());
        }
        let integer = |name: &str| -> Result<u64> {
            let value: u64 = fields[name].parse()?;
            if value.to_string() != fields[name] {
                return Err("noncanonical block capture lease integer".into());
            }
            Ok(value)
        };
        let value = BlockLeaseObservation {
            phase: fields["phase"].into(),
            monotonic_ns: integer("monotonic_ns")?,
            thread: integer("thread")?,
            acquired: u32::try_from(integer("acquired")?)?,
            released: u32::try_from(integer("released")?)?,
            live: u32::try_from(integer("live")?)?,
            budget_ns: integer("budget_ns")?,
        };
        let phase = observations.len();
        if integer("schema")? != 1
            || integer("valid")? != 1
            || integer("thread_error")? != 0
            || value.thread == 0
            || value.monotonic_ns == 0
            || value.budget_ns != 15_000_000_000
            || value.acquired.checked_sub(value.released) != Some(value.live)
            || phase >= 2
            || late.len() != 2
            || value.phase != ["begin", "end"][phase]
            || pending_phase != Some(value.phase.as_str())
            || value.thread != late[phase].thread
            || value.monotonic_ns < late[phase].monotonic_ns
        {
            return Err("block capture lease scope, counts or phase differs".into());
        }
        observations.push(value);
    }
    if !observations.is_empty() {
        if observations.len() != 2 {
            return Err("incomplete block capture lease observation".into());
        }
        let begin = &observations[0];
        let end = &observations[1];
        let elapsed = end
            .monotonic_ns
            .checked_sub(begin.monotonic_ns)
            .ok_or("block lease observation clock reversed")?;
        if begin.thread != end.thread
            || begin.monotonic_ns > late[1].monotonic_ns
            || end.acquired < begin.acquired
            || end.released < begin.released
            || end.live != 0 && elapsed < end.budget_ns
        {
            return Err("block capture leases ended early or changed identity/counts".into());
        }
    }
    Ok(observations)
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

    fn pending(phase: &str, time: u64, forwarded: u32, semaphore: u32) -> String {
        format!(
            "capture_pending schema=1 phase={phase} monotonic_ns={time} thread=7 thread_error=0 submitted=1 forwarded={forwarded} pending={} valid=1 semaphore_pending={semaphore} budget_ns=15000000000\n",
            1 - forwarded
        )
    }

    #[test]
    fn requires_full_late_metadata_window_without_any_original_semaphore() {
        let input = event("content_enter", 1, 1)
            + &pending("begin", 2, 0, 0)
            + &pending("end", 15_000_000_002, 0, 0);
        let result = analyze(&input).unwrap();
        assert_eq!(result.unobserved_content_callbacks, [1]);
        assert_eq!(result.late_observations.len(), 2);
        assert!(analyze(&input.replace("15000000002", "15000000001")).is_err());
        assert!(analyze(&input.replace("15000000002", "3")).is_err());
        let legacy = analyze(&event("content_enter", 1, 1)).unwrap();
        assert!(legacy.late_observations.is_empty());
        assert!(
            serde_json::to_value(legacy)
                .unwrap()
                .get("late_observations")
                .is_none()
        );
    }

    #[test]
    fn permits_early_handler_return_and_retains_unsignalled_wait_observation() {
        let input = event("content_enter", 1, 1)
            + &pending("begin", 2, 0, 0)
            + &event("content_callback", 1, 3)
            + &event("content_forwarded", 1, 4)
            + &pending("end", 5, 1, 0);
        assert!(
            analyze(&input)
                .unwrap()
                .unobserved_content_callbacks
                .is_empty()
        );
        assert!(analyze(&input.replace("semaphore_pending=0", "semaphore_pending=1")).is_err());
        analyze(
            &input
                .replace("semaphore_pending=0", "semaphore_pending=1")
                .replace(
                    "phase=end monotonic_ns=5",
                    "phase=end monotonic_ns=15000000002",
                ),
        )
        .unwrap();
    }

    #[test]
    fn rejects_inconsistent_incomplete_or_non_native_callback_snapshots() {
        let input = event("content_enter", 1, 1)
            + &pending("begin", 2, 0, 0)
            + &pending("end", 15_000_000_002, 0, 0);
        for bad in [
            input.replace("schema=1", "schema=2"),
            input.replace("submitted=1", "submitted=2"),
            input.replace("submitted=1", "submitted=4294967296"),
            input.replace("forwarded=0", "forwarded=1"),
            input.replace("pending=1 valid=1", "pending=0 valid=1"),
            input.replace("valid=1", "valid=0"),
            input.replace("semaphore_pending=0", "semaphore_pending=17"),
            input.replace("budget_ns=15000000000", "budget_ns=14999999999"),
            input.replace("thread_error=0 submitted", "thread_error=1 submitted"),
            input.replace(
                "phase=end monotonic_ns=15000000002 thread=7",
                "phase=end monotonic_ns=15000000002 thread=8",
            ),
            input.replace("phase=end", "phase=begin"),
            input.clone() + &pending("end", 16_000_000_000, 0, 0),
            input.replace(&pending("end", 15_000_000_002, 0, 0), ""),
            input.replace("schema=1 phase", "schema=1 schema=1 phase"),
            input.replace("valid=1", "valid=01"),
            input.replace("valid=1", "valid=1 extra=0"),
        ] {
            assert!(analyze(&bad).is_err(), "accepted {bad}");
        }
    }

    fn leases(phase: &str, time: u64, acquired: u32, released: u32) -> String {
        format!(
            "capture_block_leases schema=1 phase={phase} monotonic_ns={time} thread=7 thread_error=0 acquired={acquired} released={released} live={} valid=1 budget_ns=15000000000\n",
            acquired - released
        )
    }

    fn returned_with_live_capture() -> String {
        event("content_enter", 1, 1)
            + &event("content_callback", 1, 2)
            + &event("content_forwarded", 1, 3)
            + &pending("begin", 10, 1, 0)
            + &leases("begin", 11, 3, 2)
            + &pending("end", 15_000_000_010, 1, 0)
            + &leases("end", 15_000_000_011, 3, 2)
    }

    #[test]
    fn observes_a_live_captured_token_after_the_original_handler_returned() {
        let input = returned_with_live_capture();
        let summary = analyze(&input).unwrap();
        assert!(summary.unobserved_original_handler_returns.is_empty());
        assert_eq!(summary.late_observations[1].pending, 0);
        assert_eq!(summary.block_lease_observations[1].live, 1);
        assert!(
            analyze(
                &input
                    .replace("15000000010", "20")
                    .replace("15000000011", "21")
            )
            .is_err()
        );
        let disposed = input
            .replace("15000000010", "20")
            .replace("15000000011", "21")
            .replace(&leases("end", 21, 3, 2), &leases("end", 21, 3, 3));
        assert_eq!(
            analyze(&disposed).unwrap().block_lease_observations[1].live,
            0
        );
    }

    #[test]
    fn retains_an_unobserved_handler_when_its_captured_token_has_been_destroyed() {
        let input = event("content_enter", 1, 1)
            + &pending("begin", 10, 0, 0)
            + &leases("begin", 11, 3, 2)
            + &pending("end", 15_000_000_010, 0, 0)
            + &leases("end", 15_000_000_011, 3, 3);
        let summary = analyze(&input).unwrap();
        assert_eq!(summary.unobserved_content_callbacks, [1]);
        assert_eq!(summary.block_lease_observations[1].live, 0);
        let legacy = analyze(&event("content_enter", 1, 1)).unwrap();
        assert!(
            serde_json::to_value(legacy)
                .unwrap()
                .get("block_lease_observations")
                .is_none()
        );
    }

    #[test]
    fn rejects_invalid_widths_counts_identity_and_incomplete_block_lease_windows() {
        let input = returned_with_live_capture();
        for bad in [
            input.replace(
                "capture_block_leases schema=1",
                "capture_block_leases schema=2",
            ),
            input.replace(
                "acquired=3 released=2 live=1",
                "acquired=4294967296 released=4294967295 live=1",
            ),
            input.replace(
                "acquired=3 released=2 live=1",
                "acquired=3 released=4 live=1",
            ),
            input.replace(
                "acquired=3 released=2 live=1",
                "acquired=3 released=2 live=0",
            ),
            input.replace("acquired=3", "acquired=03"),
            input.replace("live=1 valid=1", "live=1 valid=0"),
            input.replace("thread_error=0 acquired", "thread_error=1 acquired"),
            input.replace(
                "thread=7 thread_error=0 acquired",
                "thread=8 thread_error=0 acquired",
            ),
            input.replace(
                "capture_block_leases schema=1 phase=end",
                "capture_block_leases schema=1 phase=begin",
            ),
            input.replace(&leases("begin", 11, 3, 2), &leases("begin", 9, 3, 2)),
            input.replace(
                &leases("end", 15_000_000_011, 3, 2),
                &leases("end", 15_000_000_011, 2, 1),
            ),
            input.replace(&leases("end", 15_000_000_011, 3, 2), ""),
            input.clone() + &leases("end", 16_000_000_000, 3, 2),
            input.replace("acquired=3", "acquired=3 extra=0"),
            input.replace("live=1 valid", "live=1 live=1 valid"),
            input.replace(&pending("begin", 10, 1, 0), ""),
        ] {
            assert!(analyze(&bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn rejects_lease_snapshots_moved_across_their_native_observation_phases() {
        let input = returned_with_live_capture();
        let begin = leases("begin", 11, 3, 2);
        let end = leases("end", 15_000_000_011, 3, 2);
        let moved = input.replace(&begin, "").replace(&end, "") + &begin + &end;
        assert!(analyze(&moved).is_err());
        let impossible = input
            .replace(&begin, &leases("begin", 20_000_000_011, 3, 2))
            .replace(&end, &leases("end", 20_000_000_012, 3, 3));
        assert!(analyze(&impossible).is_err());
    }
}
