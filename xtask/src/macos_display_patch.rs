// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Actual first-display capture: independently known owned patch, full-image
//! conversion/retention, real native geometry and callback ownership witnesses.
use crate::{
    Result,
    macos_display_geometry::{self, Geometry},
    records,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scene {
    window_id: u32,
    width: u32,
    height: u32,
    colour: String,
    owner_pid: u32,
    display_patch: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalStatus {
    original_oracle_passed: bool,
    expected_source_files: u32,
    recorded_source_files: u32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    Queried,
    HostPremise,
    DistinctHost,
    FoundationConsumer,
    AppkitConsumer,
    AppkitScene,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneExecutable {
    layout_version: u32,
    consumer_path: String,
    scene_path: String,
    consumer_sha256: String,
    scene_sha256: String,
    consumer_unix_mode: u32,
    scene_unix_mode: u32,
    scene_pid: u32,
}
impl SceneExecutable {
    fn validate(&self, owner_pid: u32, consumer: &[u8], scene: &[u8]) -> Result<()> {
        self.validate_paths(owner_pid, consumer, scene, true)
    }
    fn validate_paths(
        &self,
        owner_pid: u32,
        consumer: &[u8],
        scene: &[u8],
        distinct: bool,
    ) -> Result<()> {
        let native_path = |path: &str| {
            path.starts_with('/')
                && !path.contains(['\\', '\0', '\n', '\r'])
                && path
                    .split('/')
                    .skip(1)
                    .all(|part| !matches!(part, "" | "." | ".."))
        };
        if self.layout_version != 1
            || self.scene_pid == 0
            || self.scene_pid != owner_pid
            || !native_path(&self.consumer_path)
            || !native_path(&self.scene_path)
            || (self.consumer_path != self.scene_path) != distinct
            || self.consumer_path.rsplit_once('/').map(|p| p.0)
                != self.scene_path.rsplit_once('/').map(|p| p.0)
            || self.consumer_sha256.len() != 64
            || !self
                .consumer_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.consumer_sha256 != self.scene_sha256
            || self.consumer_sha256 != binary_hash(consumer)?
            || self.scene_sha256 != binary_hash(scene)?
            || self.consumer_unix_mode != self.scene_unix_mode
            || self.consumer_unix_mode > 0o7777
            || self.consumer_unix_mode & 0o100 == 0
        {
            return Err("scene host requires the declared path relation, identical binary bytes/mode and actual owned PID".into());
        }
        Ok(())
    }
}
fn binary_hash(bytes: &[u8]) -> Result<String> {
    use std::fmt::Write;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}")?;
    }
    Ok(encoded)
}
pub fn run(args: &[String], probe: Probe) -> Result<()> {
    let host_premise = probe == Probe::HostPremise;
    let (path,check_only)=match args {
        [path]=>(path,false),[path,flag] if flag=="--check-only"=>(path,true),
        _=>return Err("expected upstream-trace macos-display-<patch|host-premise|distinct-host|foundation-consumer|appkit-consumer|appkit-scene> <actual external directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    let scene: Scene = serde_json::from_slice(&read("scene/ready.json")?)?;
    if scene.width != 47
        || scene.height != 31
        || scene.colour != "green"
        || scene.display_patch != 1
    {
        return Err("wrong owned display scene premise".into());
    }
    if matches!(
        probe,
        Probe::DistinctHost
            | Probe::FoundationConsumer
            | Probe::AppkitConsumer
            | Probe::AppkitScene
    ) {
        let identity: SceneExecutable =
            serde_json::from_slice(&read("scene-executable-identity.json")?)?;
        let consumer = read("scene-executables/consumer")?;
        let host = read("scene-executables/scene")?;
        if probe == Probe::DistinctHost {
            identity.validate(scene.owner_pid, &consumer, &host)?;
        } else {
            identity.validate_paths(scene.owner_pid, &consumer, &host, false)?;
        }
    }
    if probe == Probe::FoundationConsumer {
        foundation_state(&read("foundation-consumer-state.txt")?)?;
    }
    let appkit_loop = if probe == Probe::AppkitConsumer {
        Some(appkit_state(&read("appkit-consumer-state.txt")?)?)
    } else {
        None
    };
    let scene_loop = if probe == Probe::AppkitScene {
        Some(appkit_scene_state(&read("scene/appkit-scene-state.txt")?)?)
    } else {
        None
    };
    let original: OriginalStatus = serde_json::from_slice(&read("source-oracle-status.json")?)?;
    original_status(original, host_premise)?;
    let geometry_bytes = read("scene/display-geometry.tsv")?;
    let g = macos_display_geometry::parse(std::str::from_utf8(&geometry_bytes)?)?;
    if g.window != scene.window_id || g.pid != scene.owner_pid {
        return Err("native first-display geometry substituted the owned window/process".into());
    }
    let source_geometry = if host_premise {
        host_geometry(&g, &read("source-geometry-provenance.txt")?)?;
        "source-expected-geometry.tsv"
    } else {
        "source-display-geometry.tsv"
    };
    for name in [source_geometry, "rust-display-geometry.tsv"] {
        if read(name)? != geometry_bytes {
            return Err("source/Rust native first-display identities differ".into());
        }
    }
    if read("scene/stopped.txt")? != b"window_closed=1\n" {
        return Err("display scene did not acknowledge native window release".into());
    }
    traces(std::str::from_utf8(&read("rust-display-trace.tsv")?)?, &g)?;
    for call in 0..3 {
        returned(std::str::from_utf8(&read(&format!(
            "display-call-{call}.rust-status.txt"
        ))?)?)?;
        for side in ["source", "rust"] {
            let bgr = read(&format!("display-call-{call}.{side}.bgr"))?;
            g.pixels(&bgr, 3)?;
            if read(&format!("display-held-{call}.{side}.bgr"))? != bgr {
                return Err("full-display BGR owner changed after capture closure".into());
            }
            if side == "rust" {
                let raw = read(&format!("display-call-{call}.rust.raw"))?;
                g.pixels(&raw, 4)?;
                if read(&format!("display-held-{call}.rust.raw"))? != raw {
                    return Err(
                        "full-display raw CPU owner changed after scene/native shutdown".into(),
                    );
                }
                let (bgra, raw_tail) = raw.as_chunks::<4>();
                let (bgr, bgr_tail) = bgr.as_chunks::<3>();
                if !raw_tail.is_empty() || !bgr_tail.is_empty() || bgra.len() != bgr.len() {
                    return Err("full-image native channel spans differ".into());
                }
                for (bgra, bgr) in bgra.iter().zip(bgr) {
                    if bgra[..3] != *bgr {
                        return Err("full-image BGRA-to-BGR conversion differs".into());
                    }
                }
            }
        }
    }
    let mut report = serde_json::json!({"layout_version":1,"source_captures":3,"rust_captures":3,"payloads":18,
        "original_first_display_id":g.display,"owned_window_id":g.window,"owned_pid":g.pid,"width":g.width,"height":g.height,
        "known_owned_patch_region":g.region,"artifacts":artifacts,
        "scope":"Actual unchanged original first-enumerated-display capture and same migrated Rust path, original display width/height configuration preserved. Separate AppKit owner places a 47x31 opaque green window on that actual display using matching NSScreen identity. Native SCDisplay/SCWindow point frames, real pid/window IDs and unchanged source observations fix the interior pixel region. All six source/Rust captures assert the known green patch; complete Rust BGRA-to-BGR conversion and all eighteen immediate/held files are checked. Rust callback/lock/native-input owners end before explicit source semaphore fixture reclamation and acknowledged scene shutdown; CPU owners are read on a joined thread afterward. Other full-desktop pixels are not prescribed or compared across different capture times. Not zero/multiple-display, mixed-scale/Retina, complete format/resource/late-callback coverage or full macOS/P1 acceptance."});
    if host_premise {
        report
            .as_object_mut()
            .unwrap()
            .remove("original_first_display_id");
        report["expected_first_display_id"] = g.display.into();
        report["diagnostic_only"] = true.into();
        report["paired_acceptance"] = false.into();
        report["consumer_geometry_queries"] = 0.into();
        report["live_consumer_display_identity_observed"] = false.into();
        report["scope"] = "Separate fresh original consumer process without additional native geometry queries, followed by a separate Rust process. Both capture through their unchanged default first-display implementations. Host-owned single-display geometry is an expected premise only: the consumer's actual enumerated display identity was not independently observed. All six captures assert the owned green patch, complete Rust BGRA-to-BGR and eighteen immediate/held payloads; callback/lock/input lifetimes and scene shutdown are checked. This diagnostic does not accept or replace the separately recorded queried variant, prove initialization-history causality, or establish complete display/backend/P1 parity. Shared system-service state and ordering between fixture processes are not controlled.".into();
    }
    if probe == Probe::DistinctHost {
        report["distinct_scene_executable_path"] = true.into();
        report["diagnostic_only"] = true.into();
        report["paired_acceptance"] = false.into();
        report["scope"] = "Uninstrumented original display-patch consumer plus independent Rust capture of an owned scene hosted by a byte-identical executable copied to another pathname. Native records require identical SHA256 and executable modes and the actual scene PID. Consumer geometry queries and the original capture/default/five-second wait remain unchanged. All six captures, eighteen immediate/held payloads, known green ROI, complete Rust conversion, native ownership and scene shutdown pass the same strict checks. The copied host and original consumer binary inputs are archived for independent byte verification. This is an executable-path diagnostic, not an acceptance or replacement of the separate same-path failures, proof of root cause, or full P1 acceptance; scheduling and system-service state remain uncontrolled.".into();
    }
    if probe == Probe::FoundationConsumer {
        report["source_nsapplication_initialized"] = false.into();
        report["same_scene_executable_path"] = true.into();
        report["diagnostic_only"] = true.into();
        report["paired_acceptance"] = false.into();
        report["scope"] = "Same-path, byte-identical source consumer and AppKit scene executable; only the source consumer fixture skips NSApplication initialization and pumps its Foundation default run loop. The source capture class, native geometry queries, default first display and original five-second wait are unchanged and uninstrumented. Records require NSApp nil before/after, successful original captures, exact scene PID/path/binary/mode, all six known-patch captures, eighteen payloads, full Rust conversion and owner checks. Rust retains its existing separate AppKit test host. This is a consumer-host control, not acceptance or replacement of same-path AppKit failures, proof of a system mechanism, or complete P1 acceptance.".into();
    }
    if let Some([polls, events, updates]) = appkit_loop {
        report["source_appkit_event_loop"] =
            serde_json::json!({"polls":polls,"events":events,"updates":updates});
        report["same_scene_executable_path"] = true.into();
        report["diagnostic_only"] = true.into();
        report["paired_acceptance"] = false.into();
        report["scope"] = "Same-path, byte-identical original consumer and scene executable. Only the consumer fixture enables main-thread AppKit updateWindows, nextEventMatchingMask with a bounded deadline, and sendEvent for dequeued events; its previous Foundation-loop cases remain separate. The source class, geometry queries, default first display and original five-second wait remain unchanged and uninstrumented. Post-capture state requires initialized NSApplication, main thread, actual polls and matching updates; an empty event queue is recorded without claiming event dispatch. All six known-patch captures, eighteen payloads, full Rust conversion, binary/mode/PID and owner checks remain mandatory. This event-processing control is not NSApplication.run, proof of root cause, replacement of old failures or complete P1 acceptance.".into();
    }
    if let Some([polls, events, updates]) = scene_loop {
        report["scene_appkit_event_loop"] =
            serde_json::json!({"polls":polls,"events":events,"updates":updates});
        report["scene_window_closed"] = true.into();
        report["same_scene_executable_path"] = true.into();
        report["diagnostic_only"] = true.into();
        report["paired_acceptance"] = false.into();
        report["scope"] = "Same-path, byte-identical original consumer and scene executable. Only the owned scene producer enables main-thread AppKit updateWindows, bounded nextEventMatchingMask and sendEvent for dequeued events. The uninstrumented consumer uses its original display-patch mode and Foundation loop; source class, geometry queries, default first display, original five-second wait and Rust host remain unchanged. Producer state is saved after native window close and before stop acknowledgment, requiring an initialized main-thread NSApplication, nonzero polls and matching updates; zero dequeued events never implies event dispatch. Producer closure does not certify source capture. Original source success, all six known-patch captures, eighteen payloads, full Rust conversion, same binary/path/mode/PID and owner checks remain mandatory. This producer-event control is not NSApplication.run, root-cause proof, replacement of old failures or complete P1 acceptance.".into();
    }
    let output = root.join(match probe {
        Probe::HostPremise => "display-host-premise-validation.json",
        Probe::DistinctHost => "display-distinct-host-validation.json",
        Probe::Queried => "display-patch-validation.json",
        Probe::FoundationConsumer => "display-foundation-consumer-validation.json",
        Probe::AppkitConsumer => "display-appkit-consumer-validation.json",
        Probe::AppkitScene => "display-appkit-scene-validation.json",
    });
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("saved macOS display patch report differs from actual records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    if host_premise {
        println!(
            "macOS display_host_premise source_captures=3 rust_captures=3 payloads=18 consumer_geometry_queries=0 live_consumer_display_identity_observed=0 known_owned_patch=6 diagnostic_only=1 paired_acceptance=0 full_P1_acceptance=0 passed"
        );
    } else if probe == Probe::AppkitScene {
        println!(
            "macOS display_appkit_scene source_captures=3 rust_captures=3 payloads=18 scene_main_thread_appkit_polling=1 scene_window_closed=1 same_binary_and_path_verified=1 known_owned_patch=6 diagnostic_only=1 paired_acceptance=0 full_P1_acceptance=0 passed"
        );
    } else if probe == Probe::AppkitConsumer {
        println!(
            "macOS display_appkit_consumer source_captures=3 rust_captures=3 payloads=18 main_thread_appkit_polling=1 same_binary_and_path_verified=1 known_owned_patch=6 diagnostic_only=1 paired_acceptance=0 full_P1_acceptance=0 passed"
        );
    } else if probe == Probe::FoundationConsumer {
        println!(
            "macOS display_foundation_consumer source_captures=3 rust_captures=3 payloads=18 source_nsapplication_initialized=0 same_binary_and_path_verified=1 known_owned_patch=6 diagnostic_only=1 paired_acceptance=0 full_P1_acceptance=0 passed"
        );
    } else if probe == Probe::DistinctHost {
        println!(
            "macOS display_distinct_host source_captures=3 rust_captures=3 payloads=18 same_binary_sha256_verified=1 distinct_scene_executable_path=1 known_owned_patch=6 diagnostic_only=1 paired_acceptance=0 full_P1_acceptance=0 passed"
        );
    } else {
        println!(
            "macOS display_patch source_captures=3 rust_captures=3 payloads=18 actual_native_geometry=1 known_owned_patch=6 full_image_rust_conversion=3 retained_cpu_verified=1 full_desktop_known_pixels=0 full_P1_acceptance=0 passed"
        );
    }
    Ok(())
}
fn foundation_state(bytes: &[u8]) -> Result<()> {
    if bytes != b"nsapplication_initialized=0\nnsapp_nil_before=1\nnsapp_nil_after=1\nsource_capture_passed=1\n" {
        return Err("Foundation consumer initialized NSApplication, changed premise or failed its original capture".into());
    }
    Ok(())
}

/// Instrumented same-path control only. Its images never supply an unchanged
/// source oracle or a source/Rust paired-acceptance record.
pub(crate) fn run_observer_same_path(args: &[String]) -> Result<()> {
    let (path, check_only) = match args {
        [path] => (path, false),
        [path, flag] if flag == "--check-only" => (path, true),
        _ => return Err("expected upstream-trace macos-display-observer-same-path <external directory> [--check-only]".into()),
    };
    let root = records::external_root(Path::new(path))?;
    let mut artifacts = Vec::new();
    let mut read = |name: &str| -> Result<Vec<u8>> {
        let path = root.join(name);
        artifacts.push(records::artifact(&root, &path)?);
        Ok(fs::read(path)?)
    };
    if read("diagnostic-only.txt")?
        != b"observer_same_path=1\noriginal_oracle=0\nrust_comparison=0\n"
    {
        return Err("observer control must remain separate from original/Rust parity".into());
    }
    let scene: Scene = serde_json::from_slice(&read("scene/ready.json")?)?;
    if scene.width != 47
        || scene.height != 31
        || scene.colour != "green"
        || scene.display_patch != 1
    {
        return Err("observer same-path owned scene premise differs".into());
    }
    let identity: SceneExecutable =
        serde_json::from_slice(&read("scene-executable-identity.json")?)?;
    identity.validate_paths(
        scene.owner_pid,
        &read("scene-executables/consumer")?,
        &read("scene-executables/scene")?,
        false,
    )?;
    let geometry_bytes = read("scene/display-geometry.tsv")?;
    let g = macos_display_geometry::parse(std::str::from_utf8(&geometry_bytes)?)?;
    if g.window != scene.window_id
        || g.pid != scene.owner_pid
        || read("consumer/source-display-geometry.tsv")? != geometry_bytes
        || read("scene/stopped.txt")? != b"window_closed=1\n"
    {
        return Err(
            "observer scene/consumer geometry or acknowledged window closure differs".into(),
        );
    }
    let scope = "dispatch_observer event=scope slot=16 value=1";
    let scene_errors = read("scene-stderr.log")?;
    let scene_output = read("scene-stdout.log")?;
    if !std::str::from_utf8(&scene_errors)?.contains(scope)
        || std::str::from_utf8(&scene_errors)?.contains("dispatch_observer event=create")
        || !std::str::from_utf8(&scene_output)?
            .contains("owned macOS scene stop=1 main_thread=1 passed")
    {
        return Err(
            "observer scene did not use the diagnostic host or performed source capture".into(),
        );
    }
    let consumer_bytes = read("consumer-log.txt")?;
    let consumer_log = std::str::from_utf8(&consumer_bytes)?;
    let programs = consumer_log
        .lines()
        .filter_map(|line| line.strip_prefix("program="))
        .collect::<Vec<_>>();
    let arguments = consumer_log
        .lines()
        .filter_map(|line| line.strip_prefix("arguments="))
        .collect::<Vec<_>>();
    let exits = consumer_log
        .lines()
        .filter_map(|line| line.strip_prefix("exit="))
        .collect::<Vec<_>>();
    if programs != [identity.consumer_path.as_str()] || arguments.len() != 1 || exits != ["0"] {
        return Err(
            "observer consumer invocation/status differs from archived executable identity".into(),
        );
    }
    let arguments: Vec<String> = serde_json::from_str(arguments[0])?;
    if arguments.len() != 3
        || arguments[0] != "display-patch"
        || arguments[2] != scene.window_id.to_string()
        || !arguments[1].starts_with('/')
        || !arguments[1].ends_with("/consumer")
    {
        return Err("observer consumer changed its original display mode or owned target".into());
    }
    if !consumer_log.contains("original macOS display_patch first_enumerated=1 captures=3 retained=3 native_geometry=1 known_owned_patch=3 full_display_pixels_unasserted=1 passed") {
        return Err("instrumented observer consumer failed or captured an incomplete patch series".into());
    }
    let waits = observer_waits(consumer_log)?;
    for call in 0..3 {
        let pixels = read(&format!("consumer/display-call-{call}.source.bgr"))?;
        g.pixels(&pixels, 3)?;
        if read(&format!("consumer/display-held-{call}.source.bgr"))? != pixels {
            return Err("observer CPU image changed after capture owner closure".into());
        }
    }
    let report = serde_json::json!({"layout_version":1,"diagnostic_only":true,"original_oracle":false,
        "paired_acceptance":false,"rust_comparison":false,"same_executable_path":true,
        "observed_captures":3,"payloads":6,"native_successful_waits":3,"owned_window":g.window,"owned_pid":g.pid,
        "wait_timestamps_create_enter_return_signal_enter_return":waits,"artifacts":artifacts,
        "scope":"Both owned scene and consumer use the same instrumented executable pathname and identical archived bytes/mode. Producer uses the existing Foundation scene loop; consumer captures the original first display with native semaphore wrappers that log real create/wait/signal calls and preserve original values/default five-second wait. All three known patch images, held bytes, geometry/window/PID, actual successful native waits, caller/signal thread continuity and acknowledged scene closure are checked. Instrumentation changes binary identity and timing. This removes the previous different scene-path confound only for this observer control; it does not prove root cause, accept any uninstrumented failure, reclaim residual source references, prove late-owner drain or establish source/Rust/full P1 acceptance."});
    let output = root.join("observer-same-path-validation.json");
    if check_only && output.exists() {
        if records::read::<serde_json::Value>(&output)? != report {
            return Err("observer same-path report differs from raw native records".into());
        }
    } else if !check_only {
        records::create_json(&output, &report)?;
    }
    println!(
        "macOS display_observer_same_path observed_captures=3 payloads=6 native_successful_waits=3 same_binary_path_mode_pid=1 diagnostic_only=1 original_oracle=0 rust_comparison=0 paired_acceptance=0 full_P1_acceptance=0 passed"
    );
    Ok(())
}

fn observer_waits(text: &str) -> Result<[[u64; 5]; 3]> {
    let mut times = [[None; 5]; 3];
    let mut threads = [[0; 5]; 3];
    let mut scopes = 0;
    for line in text
        .lines()
        .filter_map(|line| line.strip_prefix("dispatch_observer "))
    {
        let parts = line.splitn(7, ' ').collect::<Vec<_>>();
        if parts.len() != 7 {
            return Err("incomplete native observer event".into());
        }
        let mut fields = Vec::new();
        for (key, part) in [
            "event",
            "slot",
            "value",
            "monotonic_ns",
            "thread",
            "thread_error",
            "message",
        ]
        .into_iter()
        .zip(parts)
        {
            fields.push(
                part.strip_prefix(&format!("{key}="))
                    .ok_or("invalid native observer event fields")?,
            );
        }
        let number = |index: usize| -> Result<u64> {
            let value: u64 = fields[index].parse()?;
            if value.to_string() != fields[index] {
                return Err("noncanonical native observer integer".into());
            }
            Ok(value)
        };
        let slot = usize::try_from(number(1)?)?;
        let value = number(2)?;
        let time = number(3)?;
        let thread = number(4)?;
        if time == 0 || thread == 0 || number(5)? != 0 {
            return Err("native observer lacks clock/thread evidence".into());
        }
        if fields[0] == "scope" {
            if slot != 16 || value != 1 || fields[6] != "diagnostic_only_not_parity_oracle" {
                return Err("observer scope relabelled as parity".into());
            }
            scopes += 1;
            continue;
        }
        let kind = match fields[0] {"create"=>0,"wait_enter"=>1,"wait_return"=>2,"signal_enter"=>3,"signal_return"=>4,_=>return Err("observer source error, late observation or unknown event cannot certify this successful series".into())};
        if slot >= 3
            || (kind != 4 && value != 0)
            || fields[6]
                != if kind == 0 {
                    "native_create_returned"
                } else {
                    ""
                }
            || times[slot][kind].replace(time).is_some()
        {
            return Err(
                "observer wait failed, native create was null or event was duplicated".into(),
            );
        }
        threads[slot][kind] = thread;
    }
    if scopes != 1 {
        return Err("missing or duplicated diagnostic observer scope".into());
    }
    let mut output = [[0; 5]; 3];
    for slot in 0..3 {
        for kind in 0..5 {
            output[slot][kind] =
                times[slot][kind].ok_or("incomplete native observer semaphore lifetime")?;
        }
        let [create, enter, returned, signal, signalled] = output[slot];
        if create > enter
            || enter > returned
            || create > signal
            || signal > signalled
            || threads[slot][0..3] != [threads[slot][0]; 3]
            || threads[slot][3] != threads[slot][4]
        {
            return Err("observer native wait/signal clock or thread continuity differs".into());
        }
    }
    Ok(output)
}
fn appkit_state(bytes: &[u8]) -> Result<[u64; 3]> {
    appkit_loop_state(bytes, "source_capture_passed")
}
fn appkit_scene_state(bytes: &[u8]) -> Result<[u64; 3]> {
    appkit_loop_state(bytes, "window_closed")
}
fn appkit_loop_state(bytes: &[u8], completed_field: &str) -> Result<[u64; 3]> {
    let text = std::str::from_utf8(bytes)?;
    if !text.ends_with('\n') || text.contains('\r') {
        return Err("AppKit state must have complete LF records".into());
    }
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("invalid AppKit state")?;
        if fields.insert(key, value).is_some() {
            return Err("duplicate AppKit state".into());
        }
    }
    let mut counts = [0; 3];
    for (index, key) in ["polls", "events", "updates"].into_iter().enumerate() {
        let value = fields.remove(key).ok_or("missing AppKit loop count")?;
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("invalid AppKit loop count".into());
        }
        counts[index] = value.parse::<u64>()?;
    }
    if fields
        != BTreeMap::from([
            ("nsapplication_initialized", "1"),
            ("nsapp_present_before", "1"),
            ("nsapp_present_after", "1"),
            ("main_thread", "1"),
            (completed_field, "1"),
        ])
        || counts[0] == 0
        || counts[1] > counts[0]
        || counts[2] != counts[0]
    {
        return Err(
            "AppKit control failed its owner completion, host premise or event processing".into(),
        );
    }
    Ok(counts)
}
fn host_geometry(g: &Geometry, provenance: &[u8]) -> Result<()> {
    if g.count != 1 || provenance != b"origin=host_expected_snapshot\nconsumer_geometry_queries=0\nlive_consumer_display_identity_observed=0\nsingle_display_premise=1\n" {
        return Err("host expectation requires one display and explicit unobserved consumer identity".into());
    }
    Ok(())
}
fn original_status(value: OriginalStatus, host_premise: bool) -> Result<()> {
    let expected = if host_premise { 8 } else { 7 };
    if !value.original_oracle_passed
        || value.expected_source_files != expected
        || value.recorded_source_files != expected
    {
        return Err("original display oracle failed/incomplete; independent Rust observations cannot accept this pair".into());
    }
    Ok(())
}
fn returned(text: &str) -> Result<()> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("invalid native display return snapshot")?;
        if fields.insert(key, value).is_some() {
            return Err("duplicate native display return field".into());
        }
    }
    let expected = BTreeMap::from([
        ("result", "1"),
        ("timed_out", "0"),
        ("content_calls", "1"),
        ("sample_calls", "1"),
        ("completed", "1"),
        ("source_failure", "0"),
        ("native_error", "0"),
        ("fault", "0"),
        ("wait_result", "0"),
        ("error_debug", "None"),
    ]);
    let elapsed = fields
        .remove("elapsed_ns")
        .ok_or("missing native display elapsed time")?;
    if elapsed.is_empty()
        || !elapsed.bytes().all(|b| b.is_ascii_digit())
        || elapsed.parse::<u64>()? == 0
        || fields != expected
    {
        return Err(
            "native display returned None/error/timeout or has incomplete callback evidence".into(),
        );
    }
    Ok(())
}
fn traces(text: &str, g: &Geometry) -> Result<()> {
    let rows = text.lines().collect::<Vec<_>>();
    if rows.len() != 3 {
        return Err("incomplete native display callback records".into());
    }
    for (call, row) in rows.iter().enumerate() {
        let v = row
            .split('\t')
            .map(|value| -> Result<u64> {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("invalid display callback decimal".into());
                }
                Ok(value.parse()?)
            })
            .collect::<Result<Vec<_>>>()?;
        if v.len() != 17
            || v[0] != u64::try_from(call)?
            || v[1..9] != [1; 8]
            || v[9] != u64::try_from(g.span(4)?)?
            || v[10..14] != [1, 1, 0, 0]
            || v[14] < u64::try_from(g.width.checked_mul(4).ok_or("display stride overflow")?)?
            || v[15] != u64::try_from(g.width)?
            || v[16] != u64::try_from(g.height)?
        {
            return Err(
                "native display callback/lock/retained source reference evidence differs".into(),
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_failed_missing_duplicate_and_late_observer_wait_records() {
        let mut good = String::from(
            "dispatch_observer event=scope slot=16 value=1 monotonic_ns=1 thread=1 thread_error=0 message=diagnostic_only_not_parity_oracle\n",
        );
        for slot in 0..3 {
            for (event, time, thread, message, value) in [
                ("create", 10, 7, "native_create_returned", 0),
                ("wait_enter", 20, 7, "", 0),
                ("signal_enter", 30, 9, "", 0),
                ("signal_return", 40, 9, "", 1),
                ("wait_return", 50, 7, "", 0),
            ] {
                good.push_str(&format!("dispatch_observer event={event} slot={slot} value={value} monotonic_ns={time} thread={thread} thread_error=0 message={message}\n"));
            }
        }
        assert_eq!(observer_waits(&good).unwrap(), [[10, 20, 50, 30, 40]; 3]);
        for bad in [
            good.replace(
                "event=wait_return slot=0 value=0",
                "event=wait_return slot=0 value=1",
            ),
            good.replace("event=wait_return slot=0", "event=wait_return slot=3"),
            good.replace("thread_error=0", "thread_error=1"),
            good.replace("thread=7", "thread=0"),
            good.replace(
                "message=native_create_returned",
                "message=native_create_null",
            ),
            good.replace("monotonic_ns=50", "monotonic_ns=19"),
            good.replace("event=signal_return", "event=late_observation_end"),
            good.replace("event=wait_enter", "event=wait_return"),
            good.lines().skip(1).collect::<Vec<_>>().join("\n"),
            good.lines().take(15).collect::<Vec<_>>().join("\n"),
            good.clone() + good.lines().next().unwrap() + "\n",
        ] {
            assert!(observer_waits(&bad).is_err(), "accepted {bad}");
        }
    }
    #[test]
    fn rejects_unclosed_scene_missing_polling_and_consumer_completion_substitution() {
        let good = "nsapplication_initialized=1\nnsapp_present_before=1\nnsapp_present_after=1\nmain_thread=1\npolls=7\nevents=0\nupdates=7\nwindow_closed=1\n";
        assert_eq!(appkit_scene_state(good.as_bytes()).unwrap(), [7, 0, 7]);
        assert_eq!(
            appkit_scene_state(good.replace("events=0", "events=3").as_bytes()).unwrap(),
            [7, 3, 7]
        );
        // Producer closure and consumer capture have different owners. Neither
        // record can substitute for the other even with identical loop counts.
        assert!(appkit_state(good.as_bytes()).is_err());
        assert!(
            appkit_scene_state(
                good.replace("window_closed=1", "source_capture_passed=1")
                    .as_bytes()
            )
            .is_err()
        );
        for bad in [
            good.replace("window_closed=1", "window_closed=0"),
            good.replace("window_closed=1\n", ""),
            good.replace("main_thread=1", "main_thread=0"),
            good.replace("nsapp_present_before=1", "nsapp_present_before=0"),
            good.replace("nsapp_present_after=1", "nsapp_present_after=0"),
            good.replace("nsapplication_initialized=1", "nsapplication_initialized=0"),
            good.replace("polls=7", "polls=0"),
            good.replace("polls=7\n", ""),
            good.replace("events=0", "events=8"),
            good.replace("updates=7", "updates=6"),
            good.replace("polls=7", "polls=18446744073709551616"),
            good.to_owned() + "window_closed=1\n",
            good.to_owned() + "source_capture_passed=1\n",
        ] {
            assert!(
                appkit_scene_state(bad.as_bytes()).is_err(),
                "accepted {bad}"
            );
        }
    }
    #[test]
    fn rejects_failed_capture_missing_polling_and_inconsistent_appkit_host_records() {
        let good = "nsapplication_initialized=1\nnsapp_present_before=1\nnsapp_present_after=1\nmain_thread=1\npolls=7\nevents=0\nupdates=7\nsource_capture_passed=1\n";
        assert_eq!(appkit_state(good.as_bytes()).unwrap(), [7, 0, 7]);
        assert_eq!(
            appkit_state(good.replace("events=0", "events=3").as_bytes()).unwrap(),
            [7, 3, 7]
        );
        for bad in [
            good.replace("source_capture_passed=1", "source_capture_passed=0"),
            good.replace("main_thread=1", "main_thread=0"),
            good.replace("nsapp_present_after=1", "nsapp_present_after=0"),
            good.replace("nsapp_present_before=1", "nsapp_present_before=0"),
            good.replace("nsapplication_initialized=1", "nsapplication_initialized=0"),
            good.replace("polls=7", "polls=0"),
            good.replace("events=0", "events=8"),
            good.replace("updates=7", "updates=6"),
            good.replace("polls=7", "polls=07"),
            good.replace("polls=7", "polls=18446744073709551616"),
            good.replace("polls=7", "polls=+7"),
            good.replace("polls=7\n", ""),
            good.to_owned() + "polls=7\n",
            good.to_owned() + "unknown=1\n",
            good.trim_end().to_owned(),
        ] {
            assert!(appkit_state(bad.as_bytes()).is_err(), "accepted {bad}");
        }
    }
    #[test]
    fn rejects_substituted_scene_executables_paths_modes_and_owners() {
        let bytes = b"owned executable input";
        let good = SceneExecutable {
            layout_version: 1,
            consumer_path: "/native/build/consumer".into(),
            scene_path: "/native/build/scene".into(),
            consumer_sha256: binary_hash(bytes).unwrap(),
            scene_sha256: binary_hash(bytes).unwrap(),
            consumer_unix_mode: 0o755,
            scene_unix_mode: 0o755,
            scene_pid: 7,
        };
        good.validate(7, bytes, bytes).unwrap();
        let same = SceneExecutable {
            scene_path: good.consumer_path.clone(),
            ..good.clone()
        };
        same.validate_paths(7, bytes, bytes, false).unwrap();
        assert!(same.validate(7, bytes, bytes).is_err());
        assert!(good.validate_paths(7, bytes, bytes, false).is_err());
        let state = b"nsapplication_initialized=0\nnsapp_nil_before=1\nnsapp_nil_after=1\nsource_capture_passed=1\n";
        foundation_state(state).unwrap();
        for invalid in [
            String::from_utf8_lossy(state)
                .replace("source_capture_passed=1", "source_capture_passed=0"),
            String::from_utf8_lossy(state).replace("nsapp_nil_after=1", "nsapp_nil_after=0"),
            String::from_utf8_lossy(state)
                .replace("nsapplication_initialized=0", "nsapplication_initialized=1"),
        ] {
            assert!(foundation_state(invalid.as_bytes()).is_err());
        }
        assert!(good.validate(7, bytes, b"changed executable").is_err());
        for bad in [
            SceneExecutable {
                scene_path: good.consumer_path.clone(),
                ..good.clone()
            },
            SceneExecutable {
                scene_path: "native/build/scene".into(),
                ..good.clone()
            },
            SceneExecutable {
                scene_path: "/native/build/../scene".into(),
                ..good.clone()
            },
            SceneExecutable {
                scene_unix_mode: 0o644,
                ..good.clone()
            },
            SceneExecutable {
                consumer_unix_mode: 0o644,
                scene_unix_mode: 0o644,
                ..good.clone()
            },
            SceneExecutable {
                scene_pid: 8,
                ..good.clone()
            },
            SceneExecutable {
                consumer_sha256: "0".repeat(64),
                scene_sha256: "0".repeat(64),
                ..good
            },
        ] {
            assert!(bad.validate(7, bytes, bytes).is_err());
        }
    }
    #[test]
    fn rejects_missing_callbacks_unreleased_inputs_wrong_spans_and_cleanup_relabeling() {
        let g = macos_display_geometry::parse(
            "1\t17\t640\t480\t0\t0\t640\t480\t91\t7\t100\t100\t47\t31\t1\t2\n",
        )
        .unwrap();
        let rows = (0..3)
            .map(|i| format!("{i}\t1\t1\t1\t1\t1\t1\t1\t1\t1228800\t1\t1\t0\t0\t2560\t640\t480\n"))
            .collect::<String>();
        traces(&rows, &g).unwrap();
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: false,
                    expected_source_files: 7,
                    recorded_source_files: 1
                },
                false
            )
            .is_err()
        );
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: false,
                    expected_source_files: 7,
                    recorded_source_files: 7
                },
                false
            )
            .is_err()
        );
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: true,
                    expected_source_files: 7,
                    recorded_source_files: 1
                },
                false
            )
            .is_err()
        );
        original_status(
            OriginalStatus {
                original_oracle_passed: true,
                expected_source_files: 7,
                recorded_source_files: 7,
            },
            false,
        )
        .unwrap();
        let returned_ok = "result=1\nelapsed_ns=7\ntimed_out=0\ncontent_calls=1\nsample_calls=1\ncompleted=1\nsource_failure=0\nnative_error=0\nfault=0\nwait_result=0\nerror_debug=None\n";
        returned(returned_ok).unwrap();
        assert!(returned(&returned_ok.replace("result=1", "result=0")).is_err());
        assert!(returned(&returned_ok.replace("timed_out=0", "timed_out=1")).is_err());
        assert!(returned(&(returned_ok.to_owned() + "result=1\n")).is_err());
        for bad in [
            rows.lines().take(2).collect::<Vec<_>>().join("\n"),
            rows.replacen("\t1\t1\t0\t0\t2560", "\t1\t0\t0\t0\t2560", 1),
            rows.replacen("1228800", "1228799", 1),
            rows.replacen("\t1\t1\t1\t1", "\t1\t0\t1\t1", 1),
        ] {
            assert!(traces(&bad, &g).is_err());
        }
    }
    #[test]
    fn rejects_host_expectations_relabelled_as_observed_or_multiple_displays() {
        let mut g = macos_display_geometry::parse(
            "1\t17\t640\t480\t0\t0\t640\t480\t91\t7\t100\t100\t47\t31\t1\t2\n",
        )
        .unwrap();
        let provenance = b"origin=host_expected_snapshot\nconsumer_geometry_queries=0\nlive_consumer_display_identity_observed=0\nsingle_display_premise=1\n";
        host_geometry(&g, provenance).unwrap();
        assert!(
            host_geometry(
                &g,
                &String::from_utf8_lossy(provenance)
                    .replace("identity_observed=0", "identity_observed=1")
                    .into_bytes()
            )
            .is_err()
        );
        g.count = 2;
        assert!(host_geometry(&g, provenance).is_err());
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: true,
                    expected_source_files: 8,
                    recorded_source_files: 8,
                },
                false
            )
            .is_err()
        );
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: true,
                    expected_source_files: 7,
                    recorded_source_files: 7,
                },
                true
            )
            .is_err()
        );
        assert!(
            original_status(
                OriginalStatus {
                    original_oracle_passed: false,
                    expected_source_files: 8,
                    recorded_source_files: 8,
                },
                true
            )
            .is_err()
        );
    }
}
