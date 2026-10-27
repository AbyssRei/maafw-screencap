// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::build_configuration::{self as model, Configuration, FAMILIES, Mode};
use crate::{
    Result, batch,
    records::{self, Artifact, Index},
    repository,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbeState {
    Off,
    Foreign,
    Available,
    Missing,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    family: String,
    state: ProbeState,
    evidence: Vec<String>,
    explanation: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    layout_version: u32,
    dependency_batch: Artifact,
    target: String,
    requests: [String; 4],
    #[serde(default)]
    environment: BTreeMap<String, Option<String>>,
    probes: Vec<Probe>,
    configuration: Option<Artifact>,
    #[serde(default)]
    consumer_lock: Option<Artifact>,
}

fn resolve(
    target: &str,
    modes: [Mode; 4],
    mut native: impl FnMut(usize, &mut Vec<String>) -> std::result::Result<(), String>,
) -> (Option<Configuration>, Vec<Probe>) {
    let mut configuration = Configuration {
        target: target.into(),
        modes,
        enabled: [false; 4],
        native_evidence: None,
    };
    let mut probes = Vec::new();
    let mut failed = false;
    for (n, family) in FAMILIES.iter().enumerate() {
        let mut evidence = Vec::new();
        let (state, explanation) = if modes[n] == Mode::Off {
            (
                ProbeState::Off,
                "Explicit OFF; no native discovery executed".into(),
            )
        } else if !model::applicable(target, n) {
            (
                ProbeState::Foreign,
                "Not applicable to this native target; no foreign discovery executed".into(),
            )
        } else {
            match native(n, &mut evidence) {
                Ok(()) => (
                    ProbeState::Available,
                    "Native dependency discovery passed; runtime capture remains a separate gate"
                        .into(),
                ),
                Err(error) => (ProbeState::Missing, error),
            }
        };
        match model::choose(modes[n], state == ProbeState::Available) {
            Ok(enabled) => configuration.enabled[n] = enabled,
            Err(_) => failed = true,
        }
        probes.push(Probe {
            family: (*family).into(),
            state,
            evidence,
            explanation,
        });
    }
    configuration.native_evidence = Some(std::array::from_fn(|n| {
        if configuration.enabled[n] {
            probes[n].evidence.join("\n")
        } else {
            String::new()
        }
    }));
    ((!failed).then_some(configuration), probes)
}

fn input<'a>(run: &'a batch::Run, suffix: &str) -> Result<&'a Artifact> {
    let items: Vec<_> = run
        .inputs
        .iter()
        .filter(|a| a.path.ends_with(suffix))
        .collect();
    if items.len() != 1 {
        return Err(format!("configuration requires one archived {suffix}").into());
    }
    Ok(items[0])
}

fn dependency_target(root: &Path, parent: &Artifact) -> Result<String> {
    let run: batch::Run = records::read(&records::verify(root, parent)?)?;
    for name in [
        "nightly-update",
        "dependency-update",
        "latest-major-check",
        "offline-inputs",
        "source-stability",
    ] {
        if !run
            .checks
            .iter()
            .any(|c| c.name == name && c.status == "passed")
        {
            return Err(
                format!("configuration requires passed dependency preparation: {name}").into(),
            );
        }
    }
    Ok(fs::read_to_string(records::verify(
        root,
        input(&run, "/inputs/dependency-target.txt")?,
    )?)?
    .trim()
    .into())
}

pub(crate) fn verify(root: &Path, artifact: &Artifact) -> Result<()> {
    let record: Record = records::read(&records::verify(root, artifact)?)?;
    records::version(record.layout_version)?;
    if record.target != dependency_target(root, &record.dependency_batch)?
        || record.probes.len() != 4
    {
        return Err("configuration target/coverage differs from its dependency batch".into());
    }
    let modes = record
        .requests
        .iter()
        .map(|m| Mode::parse(m))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if let Some(lock) = &record.consumer_lock {
        let dependency: batch::Run =
            records::read(&records::verify(root, &record.dependency_batch)?)?;
        if fs::read(records::verify(root, lock)?)?
            != fs::read(records::verify(
                root,
                input(&dependency, "/inputs/Cargo.lock")?,
            )?)?
        {
            return Err(
                "configuration consumer lock differs from its prepared dependency batch".into(),
            );
        }
    }
    let mut enabled = [false; 4];
    let mut failed = false;
    for (n, probe) in record.probes.iter().enumerate() {
        if probe.family != FAMILIES[n]
            || probe.explanation.trim().is_empty()
            || ((modes[n] == Mode::Off) != (probe.state == ProbeState::Off))
            || (modes[n] != Mode::Off
                && !model::applicable(&record.target, n)
                && probe.state != ProbeState::Foreign)
            || (model::applicable(&record.target, n) && probe.state == ProbeState::Foreign)
            || (probe.state == ProbeState::Available && probe.evidence.is_empty())
            || (matches!(probe.state, ProbeState::Off | ProbeState::Foreign)
                && !probe.evidence.is_empty())
        {
            return Err("configuration probe contradicts its request, target or evidence".into());
        }
        match model::choose(modes[n], probe.state == ProbeState::Available) {
            Ok(value) => enabled[n] = value,
            Err(_) => failed = true,
        }
    }
    match &record.configuration {
        Some(config) if !failed => {
            let config =
                Configuration::parse(&fs::read_to_string(records::verify(root, config)?)?)?;
            if config.target != record.target || config.modes.as_slice() != modes.as_slice() {
                return Err("prepared file differs from native discovery decisions".into());
            }
            config.verify_features(&enabled, enabled[0])?;
            if config.native_evidence.is_some() {
                for (n, probe) in record.probes.iter().enumerate() {
                    if enabled[n] {
                        config.verify_native(n, &probe.evidence)?;
                    }
                }
            }
        }
        None if failed => {}
        _ => return Err("explicit ON failure must not publish a usable configuration".into()),
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let mut modes = [Mode::Auto; 4];
    let mut seen = [false; 4];
    let mut offline = None;
    let mut consumer_lock = repository().join("Cargo.lock");
    let mut root = env::var_os("MFS_REFERENCE_ROOT")
        .map(Into::into)
        .unwrap_or_else(|| {
            repository()
                .parent()
                .unwrap()
                .join("maafw-screencap-reference")
        });
    let mut arguments = args.iter();
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or("configuration option requires a value")?;
        match flag.as_str() {
            "--root" => root = value.into(),
            "--consumer-lock" => consumer_lock = value.into(),
            "--offline-inputs" if offline.is_none() => offline = Some(value.clone()),
            _ => {
                let family = FAMILIES
                    .iter()
                    .position(|f| flag == &format!("--{f}"))
                    .ok_or("unknown configuration option")?;
                if seen[family] {
                    return Err("duplicate backend request".into());
                }
                seen[family] = true;
                modes[family] = Mode::parse(value)?;
            }
        }
    }
    let root = records::external_root(&root)?;
    if offline.is_none() {
        // This mode updates/archives dependencies without building the capture
        // package, so configuration remains usable while capture is incomplete.
        // Pass the explicit root to the preparer without changing process env.
        batch::prepare_at(&root)?;
    }
    let index: Index = records::read(&root.join("manifest.json"))?;
    records::version(index.layout_version)?;
    let parent = if let Some(path) = offline {
        index
            .runs
            .iter()
            .find(|a| a.path == path)
            .ok_or("offline dependency batch must be indexed")?
    } else {
        index
            .runs
            .last()
            .ok_or("dependency preparation produced no record")?
    };
    batch::verify(&root, parent)?;
    let dependency: batch::Run = records::read(&records::verify(&root, parent)?)?;
    let target = dependency_target(&root, parent)?;
    if fs::read(records::verify(
        &root,
        input(&dependency, "/inputs/Cargo.lock")?,
    )?)? != fs::read(&consumer_lock)?
        || fs::read(records::verify(
            &root,
            input(&dependency, "/inputs/toolchain.txt")?,
        )?)? != crate::command("rustc", &["-vV".as_ref()], repository())?
    {
        return Err(
            "configuration requires the exact prepared Cargo.lock and installed toolchain".into(),
        );
    }
    let id = format!(
        "p0-config-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let directory = root.join("runs").join(&id);
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    fs::create_dir(&inputs)?;
    let mut run=batch::Run {layout_version:records::LAYOUT_VERSION,run_id:id,phase:"P0".into(),phase_status:"incomplete".into(),checks:Vec::new(),inputs:vec![parent.clone()],pending:vec!["P0 prototype build selection only; no migrated backend, SDK, runtime permissions, capture, full generator/toolchain closure or phase acceptance is asserted.".into()]};
    let before = batch::snapshot_source(&root, &inputs, &mut run)?;
    let (selected, probes) = resolve(&target, modes, |family, evidence| {
        model::probe(&target, family, evidence)
    });
    let mut record = Record {
        layout_version: records::LAYOUT_VERSION,
        dependency_batch: parent.clone(),
        target,
        requests: modes.map(|m| m.name().into()),
        environment: model::PROBE_ENV
            .into_iter()
            .map(|name| (name.into(), env::var(name).ok()))
            .collect(),
        probes,
        configuration: None,
        consumer_lock: None,
    };
    let saved_lock = inputs.join("consumer-Cargo.lock");
    fs::copy(consumer_lock, &saved_lock)?;
    record.consumer_lock = Some(records::artifact(&root, &saved_lock)?);
    run.inputs.push(record.consumer_lock.clone().unwrap());
    if let Some(selected) = &selected {
        let config = inputs.join("build.conf");
        fs::write(&config, selected.encode())?;
        Configuration::parse(&fs::read_to_string(&config)?)?;
        record.configuration = Some(records::artifact(&root, &config)?);
        run.inputs.push(record.configuration.clone().unwrap());
    }
    let report = inputs.join("configuration.json");
    records::create_json(&report, &record)?;
    let report = records::artifact(&root, &report)?;
    verify(&root, &report)?;
    run.inputs.push(report);
    let log = directory.join("configuration.log");
    let stable = before == batch::source_fingerprint(repository())?
        && fs::read(inputs.join("product-git-head.txt"))?
            == crate::git(repository(), &["rev-parse", "HEAD"])?;
    let passed = selected.is_some() && stable;
    fs::write(
        &log,
        format!(
            "Selection={} source_stable={stable}\n{}\n",
            if selected.is_some() {
                "ready"
            } else {
                "failed: explicit ON dependency unavailable"
            },
            record
                .probes
                .iter()
                .map(|p| format!(
                    "{}: {:?}; {}\n{}",
                    p.family,
                    p.state,
                    p.explanation,
                    p.evidence.join("\n")
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )?;
    run.checks.push(batch::Check {
        name: "prototype-configuration".into(),
        status: if passed { "passed" } else { "failed" }.into(),
        log: records::artifact(&root, &log)?,
    });
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    batch::verify(&root, &records::artifact(&root, &manifest)?)?;
    batch::register(&root, &manifest)?;
    if !passed {
        return Err(format!(
            "configuration failed; evidence retained at {}",
            manifest.display()
        )
        .into());
    }
    println!(
        "MFS_BUILD_CONFIGURATION={}",
        root.join(&record.configuration.unwrap().path).display()
    );
    println!("CARGO_FEATURES={}", selected.unwrap().features().join(","));
    Ok(())
}

pub fn run_checks(args: &[String]) -> Result<()> {
    let offline = match args {
        [] => None,
        [flag, path] if flag == "--offline-inputs" => Some(path),
        _ => {
            return Err(
                "expected configuration-check [--offline-inputs <Rust dependency batch>]".into(),
            );
        }
    };
    let path = env::var_os("MFS_REFERENCE_ROOT")
        .map(Into::into)
        .unwrap_or_else(|| {
            repository()
                .parent()
                .unwrap()
                .join("maafw-screencap-reference")
        });
    let root = records::external_root(&path)?;
    if offline.is_none() {
        batch::prepare_at(&root)?;
    }
    let index: Index = records::read(&root.join("manifest.json"))?;
    let parent = if let Some(input) = offline {
        index
            .runs
            .iter()
            .find(|a| &a.path == input)
            .ok_or("configuration checks require an indexed dependency batch")?
    } else {
        index.runs.last().ok_or("dependency preparation missing")?
    };
    batch::verify(&root, parent)?;
    dependency_target(&root, parent)?;
    let id = format!(
        "p0-configuration-check-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let directory = root.join("runs").join(&id);
    fs::create_dir(&directory)?;
    let inputs = directory.join("inputs");
    fs::create_dir(&inputs)?;
    let mut snapshot = batch::Run {
        layout_version: records::LAYOUT_VERSION,
        run_id: id,
        phase: "P0".into(),
        phase_status: "incomplete".into(),
        checks: Vec::new(),
        pending: Vec::new(),
        inputs: Vec::new(),
    };
    let before = batch::snapshot_source(&root, &inputs, &mut snapshot)?;
    let deps = repository().parent().unwrap().join("maafw-screencap-deps");
    let status = Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(inputs.join("product-source/scripts/check-p0-configuration.ps1"))
        .arg("-ReferenceRoot")
        .arg(&root)
        .arg("-DepsRoot")
        .arg(&deps)
        .arg("-RunRoot")
        .arg(&directory)
        .arg("-ProductSource")
        .arg(inputs.join("product-source"))
        .arg("-ConfigureTool")
        .arg(env::current_exe()?)
        .arg("-RustBatch")
        .arg(&parent.path)
        .current_dir(repository())
        .env("CARGO_HOME", deps.join("cargo-home"))
        .env("CARGO_TARGET_DIR", deps.join("target"))
        .status()?;
    let result = directory.join("native-result.json");
    let mut run: batch::Run = records::read(&result)?;
    run.inputs.extend(snapshot.inputs);
    run.inputs.push(parent.clone());
    run.inputs.push(records::artifact(&root, &result)?);
    let stable = before == batch::source_fingerprint(repository())?
        && fs::read(inputs.join("product-git-head.txt"))?
            == crate::git(repository(), &["rev-parse", "HEAD"])?;
    let log = directory.join("source-stability.log");
    fs::write(
        &log,
        format!("Source stable across configuration/consumer checks: {stable}\n"),
    )?;
    run.checks.push(batch::Check {
        name: "source-stability".into(),
        status: if stable { "passed" } else { "failed" }.into(),
        log: records::artifact(&root, &log)?,
    });
    let manifest = directory.join("manifest.json");
    records::create_json(&manifest, &run)?;
    batch::verify(&root, &records::artifact(&root, &manifest)?)?;
    batch::register(&root, &manifest)?;
    println!(
        "Configuration check evidence archived at {}",
        manifest.display()
    );
    if !status.success() || run.checks.iter().any(|c| c.status != "passed") {
        return Err("configuration checks failed; every attempted result is retained".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_probes_off_or_foreign_backends_and_rejects_foreign_on() {
        let mut called = Vec::new();
        let (config, probes) = resolve(
            "x86_64-unknown-linux-gnu",
            [Mode::Auto, Mode::Off, Mode::Auto, Mode::Off],
            |n, _| {
                called.push(n);
                Err("missing client library".into())
            },
        );
        assert_eq!(called, vec![2]);
        assert_eq!(config.unwrap().enabled, [false; 4]);
        assert_eq!(probes[0].state, ProbeState::Foreign);
        assert_eq!(probes[1].state, ProbeState::Off);
        let (config, _) = resolve(
            "x86_64-unknown-linux-gnu",
            [Mode::On, Mode::Off, Mode::Off, Mode::Off],
            |_, _| panic!("foreign probe"),
        );
        assert!(config.is_none());
    }
}
