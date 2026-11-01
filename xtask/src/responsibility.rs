// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Read-only P1 responsibility mapping. Definitions and scoped evidence links
//! cannot encode whole-case acceptance or replace native result validators.
use super::{Artifact, Catalog, Evidence, Result, nonempty, records, unique, verify_lines};
use crate::{git, reference, repository};
use serde::Deserialize;
use std::{collections::BTreeSet, env, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    layout_version: u32,
    catalog: Artifact,
    baseline: Artifact,
    sources: Vec<SourceMapping>,
    cases: Vec<CaseMapping>,
    remaining: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceMapping {
    path: String,
    artifact: Artifact,
    git_blob: String,
    // A changed F source must not silently inherit the ancestor description.
    baseline_delta: Option<String>,
    scope: Scope,
    implementation: Vec<Anchor>,
    verification: Vec<Anchor>,
    remaining: Vec<String>,
}
#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Partial,
    Missing,
    Boundary,
    Excluded,
    Build,
    License,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Anchor {
    artifact: Artifact,
    line: usize,
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseMapping {
    id: String,
    implementation: Vec<Anchor>,
    verification: Vec<Anchor>,
    evidence: Vec<Artifact>,
    remaining: Vec<String>,
}

fn notes(values: &[String]) -> Result<()> {
    if values.is_empty() {
        return Err("responsibility mapping requires explicit remaining scope".into());
    }
    for value in values {
        nonempty(value)?;
    }
    Ok(())
}
fn graph(audit: &Audit, catalog: &Catalog, required: &BTreeSet<String>) -> Result<()> {
    records::version(audit.layout_version)?;
    let expected: BTreeSet<_> = catalog
        .sources
        .iter()
        .map(|s| s.path.as_str())
        .chain(required.iter().map(String::as_str))
        .collect();
    let actual = unique(audit.sources.iter().map(|s| s.path.as_str()))?;
    if actual != expected {
        return Err(
            "mapping must cover the union of the complete catalog and selected F scope".into(),
        );
    }
    let expected = catalog.cases.iter().map(|c| c.id.as_str()).collect();
    if unique(audit.cases.iter().map(|c| c.id.as_str()))? != expected {
        return Err("mapping must cover every catalog case exactly once".into());
    }
    for source in &audit.sources {
        notes(&source.remaining)?;
        if let Some(original) = catalog.sources.iter().find(|s| s.path == source.path) {
            use super::Responsibility;
            let valid = match original.responsibility {
                Responsibility::Backend
                | Responsibility::DirectHelper
                | Responsibility::Manager
                | Responsibility::NativeResource => {
                    matches!(source.scope, Scope::Partial | Scope::Missing)
                }
                Responsibility::Excluded => source.scope == Scope::Excluded,
                Responsibility::Build => source.scope == Scope::Build,
                Responsibility::License => source.scope == Scope::License,
                Responsibility::HostAuthorization => source.scope == Scope::Boundary,
                Responsibility::PublicContract | Responsibility::SharedProtocol => matches!(
                    source.scope,
                    Scope::Partial | Scope::Boundary | Scope::Missing
                ),
            };
            if !valid {
                return Err("mapping cannot waive the catalog's capture responsibility by changing its scope".into());
            }
        }
        if source.scope == Scope::Partial
            && (source.implementation.is_empty() || source.verification.is_empty())
        {
            return Err(
                "partially migrated source requires implementation and verification anchors".into(),
            );
        }
        if source.scope == Scope::Missing && !source.implementation.is_empty() {
            return Err("missing implementation cannot carry a migrated-code claim".into());
        }
        if let Some(delta) = &source.baseline_delta {
            nonempty(delta)?;
        }
    }
    for case in &audit.cases {
        // Cases remain pending. A linked native report proves only its actual
        // scope; this structural command never upgrades an execution verdict.
        notes(&case.remaining)?;
        if case.verification.is_empty() {
            return Err("case mapping requires a concrete verification boundary".into());
        }
    }
    notes(&audit.remaining)
}
fn anchors(root: &Path, values: &[Anchor]) -> Result<()> {
    for anchor in values {
        let path = records::verify(root, &anchor.artifact)?;
        verify_lines(
            &fs::read(path)?,
            &[Evidence {
                line: anchor.line,
                text: anchor.text.clone(),
            }],
        )?;
    }
    Ok(())
}
pub fn run(args: &[String]) -> Result<()> {
    let (input, root) = match args {
        [input] => (input, env::var_os("MFS_REFERENCE_ROOT").map(Into::into).unwrap_or_else(|| repository().parent().unwrap().join("maafw-screencap-reference"))),
        [input, flag, root] if flag == "--root" => (input, root.into()),
        _ => return Err("expected reference responsibilities <reference-relative mapping> [--root <external directory>]".into()),
    };
    let root = records::external_root(&root)?;
    let audit: Audit = records::read(&records::resolve(&root, input)?)?;
    let catalog: Catalog = records::read(&records::verify(&root, &audit.catalog)?)?;
    super::verify(&root, &audit.catalog)?;
    if !audit.baseline.path.starts_with("baselines/f/") {
        return Err("current responsibility mapping requires an indexed F baseline".into());
    }
    let context = reference::review_context(&root, &audit.baseline)?;
    let baseline: serde_json::Value = records::read(&records::verify(&root, &audit.baseline)?)?;
    let head = String::from_utf8(git(&context.upstream, &["rev-parse", "HEAD"])?)?;
    let tree = String::from_utf8(git(&context.upstream, &["rev-parse", "HEAD^{tree}"])?)?;
    if head.trim() != context.commit || baseline["tree"].as_str() != Some(tree.trim()) {
        return Err("F responsibility source checkout identity differs".into());
    }
    let archive: Artifact = serde_json::from_value(baseline["source_archive"].clone())?;
    records::verify(&root, &archive)?;
    graph(&audit, &catalog, &context.required_sources)?;
    for source in &audit.sources {
        let bytes = fs::read(records::verify(&root, &source.artifact)?)?;
        let object = format!("{}:{}", context.commit, source.path);
        let blob = String::from_utf8(git(&context.upstream, &["rev-parse", &object])?)?;
        if blob.trim() != source.git_blob || bytes != git(&context.upstream, &["show", &object])? {
            return Err(format!("responsibility source differs from F: {}", source.path).into());
        }
        let changed = catalog
            .sources
            .iter()
            .find(|s| s.path == source.path)
            .is_none_or(|s| s.sha256 != source.artifact.sha256);
        if changed != source.baseline_delta.is_some() {
            return Err(
                "changed/additional F responsibility needs an explicit ancestor delta".into(),
            );
        }
        anchors(&root, &source.implementation)?;
        anchors(&root, &source.verification)?;
    }
    for case in &audit.cases {
        anchors(&root, &case.implementation)?;
        anchors(&root, &case.verification)?;
        for artifact in &case.evidence {
            records::verify(&root, artifact)?;
        }
    }
    println!(
        "P1 responsibility mapping sources={} catalog_sources={} routes={} pending_cases={} source_and_links_verified=1 native_acceptance=0 phase_complete=0",
        audit.sources.len(),
        catalog.sources.len(),
        catalog.methods.len(),
        audit.cases.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_selected_subset_missing_case_changed_scope_and_acceptance_claims() {
        let catalog: Catalog = serde_json::from_value(super::super::tests::fixture()).unwrap();
        let anchor = json!({"artifact":{"path":"runs/source.rs","sha256":"0".repeat(64)},"line":1,"text":"capture"});
        let mapping = json!({"path":"capture.cpp","artifact":{"path":"source.cpp","sha256":"0".repeat(64)},"git_blob":"1".repeat(40),"baseline_delta":null,"scope":"partial","implementation":[anchor.clone()],"verification":[anchor.clone()],"remaining":["native matrix incomplete"]});
        let cases:Vec<_>=catalog.cases.iter().map(|c|json!({"id":c.id,"implementation":[],"verification":[anchor.clone()],"evidence":[],"remaining":["unverified native scope"]})).collect();
        let value = json!({"layout_version":1,"catalog":{"path":"catalog.json","sha256":"0".repeat(64)},"baseline":{"path":"baseline.json","sha256":"0".repeat(64)},"sources":[mapping],"cases":cases,"remaining":["P1 incomplete"]});
        let check = |v| graph(&serde_json::from_value(v)?, &catalog, &BTreeSet::new());
        assert!(check(value.clone()).is_ok());
        let audit: Audit = serde_json::from_value(value.clone()).unwrap();
        assert!(graph(&audit, &catalog, &BTreeSet::from(["extra-F.cpp".into()])).is_err());
        let mut subset = value.clone();
        subset["sources"] = json!([]);
        assert!(check(subset).is_err());
        let mut missing = value.clone();
        missing["cases"].as_array_mut().unwrap().pop();
        assert!(check(missing).is_err());
        let mut empty = value.clone();
        empty["cases"][0]["remaining"] = json!([]);
        assert!(check(empty).is_err());
        let mut accepted = value.clone();
        accepted["cases"][0]["status"] = "passed".into();
        assert!(check(accepted).is_err());
        let mut false_scope = value;
        false_scope["sources"][0]["scope"] = "missing".into();
        assert!(check(false_scope).is_err());
    }
}
