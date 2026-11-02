// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read-only consistency checks for an externally reviewed function census.
//! Neither a syntax census nor linked check statuses establish semantic parity.
use crate::{Result, git, records, reference, repository};
use records::Artifact;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Map {
    layout_version: u32,
    family: String,
    baseline: Artifact,
    census: Artifact,
    product_head: String,
    functions: Vec<Mapping>,
    runtime: Vec<Evidence>,
    limitations: Vec<String>,
    native_acceptance: bool,
    #[serde(rename = "full_P1_accepted")]
    full_p1_accepted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Census {
    layout_version: u32,
    family: String,
    fixed_commit: String,
    original_bytes_preserved: bool,
    compiler_preprocessed: bool,
    function_exhaustiveness_accepted: bool,
    scope: String,
    sources: Vec<Source>,
    runtime_groups: Vec<RuntimeGroup>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeGroup {
    reference_root: String,
    manifest: Artifact,
    check_prefix: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    artifact: Artifact,
    git_blob: String,
    namespace_markers: Vec<Marker>,
    parse_has_error: bool,
    functions: Vec<Function>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    start_byte: u64,
    end_byte: u64,
    token: String,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Function,
    Closure,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Role {
    Capture,
    CaptureClosure,
    CaptureMetadata,
    CaptureLifetime,
    MixedInputNone,
    InputNoneBoundary,
    ExcludedApplicationOrInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Function {
    id: String,
    kind: Kind,
    name: String,
    start: u64,
    end: u64,
    line: u64,
    sha256: String,
    parent: Option<String>,
    role: Role,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    source_path: String,
    source_id: String,
    role: Role,
    rationale: String,
    rust: Vec<RustSite>,
    evidence: Vec<Evidence>,
    semantic_equivalence_accepted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RustSite {
    artifact: Artifact,
    product_path: String,
    start: u64,
    end: u64,
    line: u64,
    sha256: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    reference_root: String,
    manifest: Artifact,
    check: String,
    status: String,
    log: Artifact,
}

fn nonempty(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err("function mapping requires nonempty descriptions/identities".into());
    }
    Ok(())
}

fn commit(value: &str) -> Result<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("function mapping requires exact lowercase Git commit identities".into());
    }
    Ok(())
}

fn git_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(value)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err("function census requires portable Git-relative source paths".into());
    }
    Ok(())
}

fn range(bytes: &[u8], start: u64, end: u64) -> Result<&[u8]> {
    if start >= end {
        return Err("function span must be nonempty".into());
    }
    bytes
        .get(usize::try_from(start)?..usize::try_from(end)?)
        .ok_or_else(|| "function span exceeds its original source".into())
}

fn span(bytes: &[u8], start: u64, end: u64, line: u64, hash: &str) -> Result<()> {
    let selected = range(bytes, start, end)?;
    use std::fmt::Write;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(selected) {
        write!(&mut encoded, "{byte:02x}")?;
    }
    if encoded != hash {
        return Err("function source span digest differs".into());
    }
    let prefix = bytes
        .get(..usize::try_from(start)?)
        .ok_or("function start exceeds source")?;
    let actual_line = prefix
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        .checked_add(1)
        .ok_or("function source line overflow")?;
    if u64::try_from(actual_line)? != line {
        return Err("function source line differs".into());
    }
    Ok(())
}

fn graph(map: &Map, census: &Census) -> Result<usize> {
    records::version(map.layout_version)?;
    records::version(census.layout_version)?;
    nonempty(&map.family)?;
    nonempty(&census.scope)?;
    commit(&map.product_head)?;
    commit(&census.fixed_commit)?;
    if map.family != census.family
        || map.native_acceptance
        || map.full_p1_accepted
        || !census.original_bytes_preserved
        || census.compiler_preprocessed
        || census.function_exhaustiveness_accepted
        || census.sources.is_empty()
        || census.runtime_groups.is_empty()
        || map.limitations.is_empty()
    {
        return Err(
            "function census scope/identity must not claim compiler or native acceptance".into(),
        );
    }
    for note in &map.limitations {
        nonempty(note)?;
    }
    let mut expected = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for source in &census.sources {
        if !paths.insert(&source.path) || source.parse_has_error || source.functions.is_empty() {
            return Err("function census has duplicate/unreviewed/empty source scope".into());
        }
        for function in &source.functions {
            nonempty(&function.name)?;
            let id = format!("{}:{}:{}", function.start, function.end, function.sha256);
            if id != function.id || !expected.insert((&source.path, &function.id)) {
                return Err(
                    "function census identity is duplicate or differs from its span".into(),
                );
            }
            if function.role == Role::CaptureClosure && function.kind != Kind::Closure {
                return Err("capture closure role does not refer to a closure".into());
            }
            if function.kind == Kind::Closure && function.parent.is_none() {
                return Err("closure census requires its enclosing source function".into());
            }
            if let Some(parent) = &function.parent {
                let owner = source
                    .functions
                    .iter()
                    .find(|candidate| &candidate.id == parent)
                    .ok_or("function parent is absent from this source census")?;
                if owner.id == function.id
                    || owner.start > function.start
                    || owner.end < function.end
                    || owner.start == function.start && owner.end == function.end
                {
                    return Err("function parent does not enclose its child".into());
                }
            }
        }
    }
    let mut actual = BTreeSet::new();
    let mut excluded = 0;
    for item in &map.functions {
        nonempty(&item.rationale)?;
        let source = census
            .sources
            .iter()
            .find(|source| source.path == item.source_path)
            .ok_or("mapped source is absent from census")?;
        let function = source
            .functions
            .iter()
            .find(|function| function.id == item.source_id)
            .ok_or("mapped function is absent from census")?;
        if function.role != item.role
            || item.semantic_equivalence_accepted
            || !actual.insert((&item.source_path, &item.source_id))
        {
            return Err("function mapping role/identity/acceptance differs".into());
        }
        if item.role == Role::ExcludedApplicationOrInput {
            excluded += 1;
            if !item.rust.is_empty() || !item.evidence.is_empty() {
                return Err(
                    "excluded input/application code cannot claim migrated evidence".into(),
                );
            }
        } else if item.rust.is_empty() || item.evidence.is_empty() {
            return Err(
                "capture correspondence requires Rust sites and scoped check evidence".into(),
            );
        }
    }
    if expected != actual {
        return Err("mapping must cover the declared function census exactly".into());
    }
    Ok(excluded)
}

fn evidence(root: &Path, item: &Evidence) -> Result<()> {
    nonempty(&item.check)?;
    let context = records::resolve(root, &item.reference_root)?;
    if !item
        .manifest
        .path
        .starts_with(&format!("{}/", item.reference_root))
    {
        return Err("function evidence manifest is outside its declared native context".into());
    }
    let manifest: serde_json::Value = records::read(&records::verify(root, &item.manifest)?)?;
    let checks = manifest["checks"]
        .as_array()
        .ok_or("function evidence lacks native checks")?;
    let mut names = BTreeSet::new();
    let mut selected = None;
    for check in checks {
        let name = check["name"].as_str().ok_or("native check lacks name")?;
        if !names.insert(name) {
            return Err("native evidence contains duplicate check names".into());
        }
        if name == item.check {
            selected = Some(check);
        }
    }
    let selected = selected.ok_or("function evidence check does not exist")?;
    if selected["status"].as_str() != Some(item.status.as_str())
        || selected["log"]["path"].as_str() != Some(item.log.path.as_str())
        || selected["log"]["sha256"].as_str() != Some(item.log.sha256.as_str())
    {
        return Err("function evidence rewrites the actual native status/log identity".into());
    }
    records::verify(&context, &item.log)?;
    Ok(())
}

fn runtime_scope(root: &Path, map: &Map, census: &Census) -> Result<()> {
    let mut expected = BTreeSet::new();
    for group in &census.runtime_groups {
        nonempty(&group.check_prefix)?;
        records::resolve(root, &group.reference_root)?;
        if !group
            .manifest
            .path
            .starts_with(&format!("{}/", group.reference_root))
        {
            return Err("runtime group manifest is outside its native context".into());
        }
        let manifest: serde_json::Value = records::read(&records::verify(root, &group.manifest)?)?;
        let checks = manifest["checks"]
            .as_array()
            .ok_or("runtime group lacks checks")?;
        let mut matched = 0_usize;
        for check in checks {
            let name = check["name"]
                .as_str()
                .ok_or("runtime group check lacks name")?;
            if name.starts_with(&group.check_prefix) {
                if !expected.insert((group.manifest.path.clone(), name.to_owned())) {
                    return Err("runtime check groups overlap or duplicate a check".into());
                }
                matched += 1;
            }
        }
        if matched == 0 {
            return Err("runtime group prefix selects no actual checks".into());
        }
    }
    let actual: BTreeSet<_> = map
        .runtime
        .iter()
        .map(|check| (check.manifest.path.clone(), check.check.clone()))
        .collect();
    if actual.len() != map.runtime.len() || actual != expected {
        return Err("runtime mapping must retain every check in the declared native groups, including failures".into());
    }
    for check in &map.runtime {
        if !census.runtime_groups.iter().any(|group| {
            group.manifest.path == check.manifest.path
                && group.manifest.sha256 == check.manifest.sha256
                && group.reference_root == check.reference_root
                && check.check.starts_with(&group.check_prefix)
        }) {
            return Err("runtime check does not use its declared manifest/context identity".into());
        }
    }
    Ok(())
}

pub(crate) fn run(args: &[String]) -> Result<()> {
    let (input, root) = match args {
        [input] => (
            input,
            env::var_os("MFS_REFERENCE_ROOT").map(Into::into).unwrap_or_else(|| {
                repository().parent().unwrap().join("maafw-screencap-reference")
            }),
        ),
        [input, flag, root] if flag == "--root" => (input, root.into()),
        _ => return Err("expected reference function-map <reference-relative mapping> [--root <external directory>]".into()),
    };
    // This reader must not create an empty reference directory for a typo.
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err("function mapping reference root must already be a directory".into());
    }
    let root = records::external_root(&root)?;
    let map: Map = records::read(&records::resolve(&root, input)?)?;
    let census: Census = records::read(&records::verify(&root, &map.census)?)?;
    let excluded = graph(&map, &census)?;
    if !map.baseline.path.starts_with("baselines/f/") {
        return Err("function correspondence requires an indexed fixed fork baseline".into());
    }
    let context = reference::review_context(&root, &map.baseline)?;
    if context.commit != census.fixed_commit {
        return Err("function census fixed source differs from its baseline".into());
    }
    runtime_scope(&root, &map, &census)?;
    for source in &census.sources {
        git_path(&source.path)?;
        let bytes = fs::read(records::verify(&root, &source.artifact)?)?;
        let object = format!("{}:{}", census.fixed_commit, source.path);
        if bytes != git(&context.upstream, &["show", &object])?
            || String::from_utf8(git(&context.upstream, &["rev-parse", &object])?)?.trim()
                != source.git_blob
        {
            return Err("function census source bytes/blob differ from fixed Git source".into());
        }
        let mut balance = 0_usize;
        let mut previous_end = 0_u64;
        for marker in &source.namespace_markers {
            let start = usize::try_from(marker.start_byte)?;
            let end = usize::try_from(marker.end_byte)?;
            if marker.start_byte < previous_end
                || start != 0 && bytes.get(start - 1) != Some(&b'\n')
                || end != bytes.len() && bytes.get(end) != Some(&b'\n')
            {
                return Err("namespace markers must be ordered, separate original lines".into());
            }
            if std::str::from_utf8(range(&bytes, marker.start_byte, marker.end_byte)?)?.trim()
                != marker.token
            {
                return Err("namespace marker differs from original source bytes".into());
            }
            match marker.token.as_str() {
                "MAA_CTRL_UNIT_NS_BEGIN" => {
                    balance = balance.checked_add(1).ok_or("namespace count overflow")?
                }
                "MAA_CTRL_UNIT_NS_END" => {
                    balance = balance
                        .checked_sub(1)
                        .ok_or("unbalanced namespace marker")?
                }
                _ => return Err("unrecognized namespace marker normalization".into()),
            }
            previous_end = marker.end_byte;
        }
        if balance != 0 {
            return Err("unbalanced namespace marker normalization".into());
        }
        for function in &source.functions {
            span(
                &bytes,
                function.start,
                function.end,
                function.line,
                &function.sha256,
            )?;
        }
    }
    for item in &map.functions {
        for site in &item.rust {
            git_path(&site.product_path)?;
            if !site.product_path.starts_with("src/") || !site.product_path.ends_with(".rs") {
                return Err("function implementation site must name a Rust library source".into());
            }
            nonempty(&site.name)?;
            let bytes = fs::read(records::verify(&root, &site.artifact)?)?;
            let object = format!("{}:{}", map.product_head, site.product_path);
            if bytes != git(repository(), &["show", &object])? {
                return Err(
                    "function implementation source differs from product Git snapshot".into(),
                );
            }
            span(&bytes, site.start, site.end, site.line, &site.sha256)?;
        }
        for check in &item.evidence {
            evidence(&root, check)?;
        }
    }
    let mut statuses = std::collections::BTreeMap::new();
    let mut unique = BTreeSet::new();
    for check in &map.runtime {
        if !unique.insert((&check.manifest.path, &check.check)) {
            return Err("duplicate runtime check in function mapping".into());
        }
        evidence(&root, check)?;
        *statuses.entry(&check.status).or_insert(0_usize) += 1;
    }
    println!(
        "Function correspondence family={} sources={} indexed_functions={} excluded={} runtime_statuses={statuses:?} declared_census_matched=1 source_and_links_verified=1 exhaustive_semantics=0 native_acceptance=0 full_P1_acceptance=0",
        map.family,
        census.sources.len(),
        map.functions.len(),
        excluded,
    );
    Ok(())
}

#[cfg(test)]
#[path = "function_map_tests.rs"]
mod tests;
