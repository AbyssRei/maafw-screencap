// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Weak observations of actual submitted native objects, separate from Blocks.
use super::{BlockLeaseObservation, Event, LateObservation};
use crate::Result;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Snapshot {
    phase: String,
    request: u64,
    registered_filter: u64,
    registered_config: u64,
    filter: u64,
    config: u64,
    begin_ns: u64,
    end_ns: u64,
    thread: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Phase {
    phase: String,
    registered: u32,
    begin_ns: u64,
    end_ns: u64,
    thread: u64,
    rows: Vec<Snapshot>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Analysis {
    registrations: Vec<Snapshot>,
    late: Vec<Phase>,
    scope: &'static str,
}
fn fields<'a>(line: &'a str, expected: &[&str]) -> Result<BTreeMap<&'a str, &'a str>> {
    let mut values = BTreeMap::new();
    for field in line.split(' ') {
        let (key, value) = field.split_once('=').ok_or("invalid input weak field")?;
        if values.insert(key, value).is_some() {
            return Err("duplicate input weak field".into());
        }
    }
    if values.keys().copied().collect::<Vec<_>>() != expected {
        return Err("native input weak field set differs".into());
    }
    Ok(values)
}
fn integer(values: &BTreeMap<&str, &str>, key: &str) -> Result<u64> {
    let raw = values[key];
    let value: u64 = raw.parse()?;
    if value.to_string() != raw {
        return Err("noncanonical input weak integer".into());
    }
    Ok(value)
}
fn interval(values: &BTreeMap<&str, &str>) -> Result<(u64, u64, u64)> {
    let (begin, end, thread) = (
        integer(values, "begin_ns")?,
        integer(values, "end_ns")?,
        integer(values, "thread")?,
    );
    if integer(values, "schema")? != 1
        || integer(values, "valid")? != 1
        || integer(values, "thread_error")? != 0
        || begin == 0
        || end < begin
        || thread == 0
    {
        return Err("invalid native input weak interval or query".into());
    }
    Ok((begin, end, thread))
}
pub(super) fn analyze(
    text: &str,
    events: &[Event],
    late: &[LateObservation],
    blocks: &[BlockLeaseObservation],
) -> Result<Option<Analysis>> {
    let mut registrations = BTreeMap::new();
    let mut phases: Vec<Phase> = Vec::new();
    for line in text.lines() {
        if let Some(line) = line.strip_prefix("capture_input_weak_summary ") {
            let values = fields(
                line,
                &[
                    "begin_ns",
                    "capacity",
                    "end_ns",
                    "phase",
                    "registered",
                    "schema",
                    "thread",
                    "thread_error",
                    "valid",
                ],
            )?;
            let (begin_ns, end_ns, thread) = interval(&values)?;
            let registered = u32::try_from(integer(&values, "registered")?)?;
            if integer(&values, "capacity")? != 32
                || registered > 32
                || phases.len() >= 2
                || values["phase"] != ["begin", "end"][phases.len()]
            {
                return Err("native input weak phase/count differs".into());
            }
            phases.push(Phase {
                phase: values["phase"].into(),
                registered,
                begin_ns,
                end_ns,
                thread,
                rows: Vec::new(),
            });
        } else if let Some(line) = line.strip_prefix("capture_input_weak ") {
            let values = fields(
                line,
                &[
                    "begin_ns",
                    "config",
                    "end_ns",
                    "filter",
                    "phase",
                    "registered_config",
                    "registered_filter",
                    "request",
                    "schema",
                    "thread",
                    "thread_error",
                    "valid",
                ],
            )?;
            let (begin_ns, end_ns, thread) = interval(&values)?;
            let row = Snapshot {
                phase: values["phase"].into(),
                request: integer(&values, "request")?,
                registered_filter: integer(&values, "registered_filter")?,
                registered_config: integer(&values, "registered_config")?,
                filter: integer(&values, "filter")?,
                config: integer(&values, "config")?,
                begin_ns,
                end_ns,
                thread,
            };
            if row.request == 0
                || row.registered_filter == 0
                || row.registered_config == 0
                || row.registered_filter == row.registered_config
                || row.registered_filter % 8 != 0
                || row.registered_config % 8 != 0
                || (row.filter != 0 && row.filter != row.registered_filter)
                || (row.config != 0 && row.config != row.registered_config)
            {
                return Err("native input weak object identity differs".into());
            }
            if row.phase == "registered" {
                if row.filter == 0
                    || row.config == 0
                    || registrations.insert(row.request, row).is_some()
                {
                    return Err("native input weak registration absent or repeated".into());
                }
            } else {
                let phase = phases.last_mut().ok_or("input weak row before phase")?;
                if row.phase != phase.phase || phase.rows.iter().any(|r| r.request == row.request) {
                    return Err("native input weak row phase/duplicate differs".into());
                }
                phase.rows.push(row);
            }
        }
    }
    if registrations.is_empty() && phases.is_empty() {
        return Ok(None);
    }
    let samples: BTreeMap<_, _> = events
        .iter()
        .filter(|e| e.stage == "sample_enter")
        .map(|e| (e.request, e))
        .collect();
    if samples.len() != registrations.len() || registrations.len() > 32 {
        return Err("native input weak registration/sample set differs".into());
    }
    for (&request, row) in &registrations {
        let event = samples
            .get(&request)
            .ok_or("weak registration has no submitted sample")?;
        if !event.object_present || row.thread != event.thread || row.begin_ns < event.monotonic_ns
        {
            return Err("native input weak registration channel differs".into());
        }
        if events.iter().any(|e| {
            e.request == request && e.stage == "sample_callback" && row.end_ns > e.monotonic_ns
        }) {
            return Err("native input weak registration followed its native callback".into());
        }
    }
    if phases.len() != late.len() {
        return Err("native input weak late phase coverage differs".into());
    }
    if let [begin, end] = phases.as_slice() {
        let origin = blocks.first().map_or(late[0].monotonic_ns, |block| {
            block.monotonic_ns.max(late[0].monotonic_ns)
        });
        let unresolved = late[1].pending != 0
            || late[1].semaphore_pending != 0
            || blocks.last().is_some_and(|block| block.live != 0);
        let elapsed = end
            .begin_ns
            .checked_sub(origin)
            .ok_or("native input weak end query preceded its observation window")?;
        if end.begin_ns < begin.end_ns || (unresolved && elapsed < late[1].budget_ns) {
            return Err("native input weak end query overlapped or ended before its budget".into());
        }
    }
    let mut previous: BTreeMap<_, _> = registrations
        .iter()
        .map(|(&id, r)| (id, (r.filter, r.config)))
        .collect();
    for (phase, callback) in phases.iter().zip(late) {
        if phase.phase != callback.phase
            || phase.thread != callback.thread
            || phase.end_ns > callback.monotonic_ns
            || phase.rows.len() != usize::try_from(phase.registered)?
        {
            return Err("native input weak late interval/count differs".into());
        }
        for (&id, registration) in &registrations {
            let present = phase.rows.iter().any(|r| r.request == id);
            if (registration.end_ns <= phase.begin_ns && !present)
                || (registration.begin_ns >= phase.end_ns && present)
                || (registration.end_ns > phase.begin_ns && registration.begin_ns < phase.end_ns)
            {
                return Err("native input weak registry membership differs".into());
            }
        }
        for row in &phase.rows {
            let original = registrations
                .get(&row.request)
                .ok_or("unregistered late weak object")?;
            let prior = previous
                .get(&row.request)
                .ok_or("missing prior weak observation")?;
            if row.registered_filter != original.registered_filter
                || row.registered_config != original.registered_config
                || row.thread != phase.thread
                || row.begin_ns < phase.begin_ns
                || row.end_ns > phase.end_ns
                || (prior.0 == 0 && row.filter != 0)
                || (prior.1 == 0 && row.config != 0)
            {
                return Err("native input weak late identity/lifetime differs".into());
            }
            previous.insert(row.request, (row.filter, row.config));
        }
    }
    Ok(Some(Analysis {
        registrations: registrations.into_values().collect(),
        late: phases,
        scope: "Actual submitted filter/config objects are held only by zeroing weak Foundation tables. Each allObjects read temporarily retains its results within the recorded private autorelease-pool interval; no native object escapes into a report. Alive/zero outcomes are observations, never a requirement to dispose an uninvoked source request. These intervals are separate from callback/semaphore/block-token snapshots and do not prove native request cancellation, native block disposal, source/Rust parity or full P1.",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Event {
        Event {
            stage: "sample_enter".into(),
            request: 2,
            monotonic_ns: 10,
            thread: 9,
            object_present: true,
            error_present: false,
            error_code: 0,
            displays: 0,
            windows: 0,
            display: 0,
            width: 96,
            height: 64,
        }
    }
    fn callbacks() -> Vec<LateObservation> {
        vec![
            LateObservation {
                phase: "begin".into(),
                monotonic_ns: 100,
                thread: 7,
                submitted: 2,
                forwarded: 1,
                pending: 1,
                semaphore_pending: 1,
                budget_ns: 15_000_000_000,
            },
            LateObservation {
                phase: "end".into(),
                monotonic_ns: 15_000_000_200,
                thread: 7,
                submitted: 2,
                forwarded: 1,
                pending: 1,
                semaphore_pending: 1,
                budget_ns: 15_000_000_000,
            },
        ]
    }
    fn row(phase: &str, begin: u64, end: u64, filter: u64, config: u64, thread: u64) -> String {
        format!(
            "capture_input_weak schema=1 phase={phase} request=2 registered_filter=4096 registered_config=8192 filter={filter} config={config} begin_ns={begin} end_ns={end} thread={thread} thread_error=0 valid=1\n"
        )
    }
    fn fixture() -> String {
        row("registered", 20, 30, 4096, 8192, 9)
            + "capture_input_weak_summary schema=1 phase=begin registered=1 valid=1 capacity=32 begin_ns=40 end_ns=80 thread=7 thread_error=0\n"
            + &row("begin", 50, 60, 4096, 8192, 7)
            + "capture_input_weak_summary schema=1 phase=end registered=1 valid=1 capacity=32 begin_ns=15000000100 end_ns=15000000180 thread=7 thread_error=0\n"
            + &row("end", 15_000_000_120, 15_000_000_140, 0, 0, 7)
    }
    #[test]
    fn accepts_observed_release_or_retention_without_inventing_disposal() {
        let text = fixture();
        let first = analyze(&text, &[source()], &callbacks(), &[])
            .unwrap()
            .unwrap();
        assert_eq!(first.late[1].rows[0].filter, 0);
        let retained = text.replace("filter=0 config=0", "filter=4096 config=8192");
        assert_ne!(
            analyze(&retained, &[source()], &callbacks(), &[])
                .unwrap()
                .unwrap(),
            first
        );
        assert!(
            analyze("historical data", &[source()], &[], &[])
                .unwrap()
                .is_none()
        );
        assert!(
            analyze(
                &row("registered", 20, 30, 4096, 8192, 9),
                &[source()],
                &[],
                &[]
            )
            .unwrap()
            .is_some()
        );
    }
    #[test]
    fn refuses_substitution_missing_registration_or_invalid_read_intervals() {
        let good = fixture();
        for bad in [
            good.replace("registered_filter=4096", "registered_filter=4097"),
            good.replace("filter=4096 config=8192", "filter=8192 config=4096"),
            good.replacen("thread=9", "thread=7", 1),
            good.replace("thread_error=0", "thread_error=1"),
            good.replace("begin_ns=20", "begin_ns=9"),
            good.replace("end_ns=60", "end_ns=49"),
            good.replace("begin_ns=50", "begin_ns=39"),
            good.replace("registered=1", "registered=0"),
            good.replace("capacity=32", "capacity=33"),
            good.replace("valid=1", "valid=0"),
            good.replace("request=2", "request=3"),
            good.replace("request=2", "request=02"),
            good.replace("schema=1", "schema=1 unknown=0"),
            good.lines().skip(1).collect::<Vec<_>>().join("\n"),
            good.clone() + &row("registered", 20, 30, 4096, 8192, 9),
            good.replace(
                "filter=4096 config=8192 begin_ns=50",
                "filter=0 config=0 begin_ns=50",
            )
            .replace(
                "filter=0 config=0 begin_ns=15000000120",
                "filter=4096 config=8192 begin_ns=15000000120",
            ),
        ] {
            assert!(
                analyze(&bad, &[source()], &callbacks(), &[]).is_err(),
                "accepted {bad}"
            );
        }
        assert!(analyze(&good, &[], &callbacks(), &[]).is_err());
        assert!(analyze(&good, &[source()], &callbacks()[..1], &[]).is_err());
    }
    #[test]
    fn refuses_reordered_or_early_end_queries_and_registration_after_callback() {
        let good = fixture();
        let overlapping = good
            .replace(
                "begin_ns=15000000100 end_ns=15000000180",
                "begin_ns=50 end_ns=80",
            )
            .replace(
                "begin_ns=15000000120 end_ns=15000000140",
                "begin_ns=60 end_ns=70",
            );
        let early = good
            .replace(
                "begin_ns=15000000100 end_ns=15000000180",
                "begin_ns=110 end_ns=180",
            )
            .replace(
                "begin_ns=15000000120 end_ns=15000000140",
                "begin_ns=120 end_ns=140",
            );
        for text in [&overlapping, &early] {
            assert!(analyze(text, &[source()], &callbacks(), &[]).is_err());
        }
        let mut completed = callbacks();
        completed[1].pending = 0;
        completed[1].semaphore_pending = 0;
        assert!(analyze(&early, &[source()], &completed, &[]).is_ok());
        let blocks = [
            BlockLeaseObservation {
                phase: "begin".into(),
                monotonic_ns: 101,
                thread: 7,
                acquired: 6,
                released: 5,
                live: 1,
                budget_ns: 15_000_000_000,
            },
            BlockLeaseObservation {
                phase: "end".into(),
                monotonic_ns: 15_000_000_201,
                thread: 7,
                acquired: 6,
                released: 5,
                live: 1,
                budget_ns: 15_000_000_000,
            },
        ];
        assert!(analyze(&early, &[source()], &completed, &blocks).is_err());
        let full_budget = good.replace(
            "begin_ns=15000000100 end_ns=15000000180",
            "begin_ns=15000000101 end_ns=15000000180",
        );
        assert!(analyze(&full_budget, &[source()], &completed, &blocks).is_ok());
        let mut callback = source();
        callback.stage = "sample_callback".into();
        callback.monotonic_ns = 25;
        assert!(analyze(&good, &[source(), callback], &callbacks(), &[]).is_err());
    }
}
