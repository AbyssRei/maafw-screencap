// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{Result, command, repository};
use semver::{Version, VersionReq};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

// User-approved policy for the selected bindings: the binding itself remains
// latest stable; its exclusive transitive graph may follow upstream constraints.
const CONSTRAINED_BINDINGS: &[&str] = &[
    "pipewire",
    "wayland-client",
    "wayland-backend",
    "wayland-protocols-wlr",
    "wayland-protocols-misc",
    "wayland-sys",
];

fn closure(
    roots: impl IntoIterator<Item = String>,
    nodes: &HashMap<&str, &Value>,
) -> HashSet<String> {
    let mut pending: Vec<_> = roots.into_iter().collect();
    let mut result = HashSet::new();
    while let Some(id) = pending.pop() {
        if result.insert(id.clone())
            && let Some(node) = nodes.get(id.as_str())
        {
            pending.extend(
                node["deps"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|edge| edge["pkg"].as_str().map(str::to_owned)),
            );
        }
    }
    result
}

fn exceptions(metadata: &Value) -> Result<HashSet<String>> {
    let packages: HashMap<_, _> = metadata["packages"]
        .as_array()
        .ok_or("packages missing")?
        .iter()
        .map(|p| Ok((p["id"].as_str().ok_or("package ID missing")?, p)))
        .collect::<Result<_>>()?;
    let nodes: HashMap<_, _> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("resolved graph missing")?
        .iter()
        .map(|n| Ok((n["id"].as_str().ok_or("node ID missing")?, n)))
        .collect::<Result<_>>()?;
    let mut direct = HashSet::new();
    for member in metadata["workspace_members"]
        .as_array()
        .ok_or("workspace members missing")?
    {
        let node = nodes
            .get(member.as_str().ok_or("member ID missing")?)
            .ok_or("workspace resolution missing")?;
        direct.extend(
            node["deps"]
                .as_array()
                .ok_or("node dependencies missing")?
                .iter()
                .filter_map(|edge| edge["pkg"].as_str().map(str::to_owned)),
        );
    }
    let mut binding_roots = Vec::new();
    let mut other_roots = Vec::new();
    for id in &direct {
        let package = packages.get(id.as_str()).ok_or("direct package missing")?;
        if CONSTRAINED_BINDINGS.contains(&package["name"].as_str().ok_or("package name missing")?) {
            binding_roots.push(id.clone());
        } else {
            other_roots.push(id.clone());
        }
    }
    let binding = closure(binding_roots, &nodes);
    let other = closure(other_roots, &nodes);
    Ok(binding
        .into_iter()
        .filter(|id| !direct.contains(id) && !other.contains(id))
        .collect())
}

fn required_version(
    releases: &Value,
    requirements: &[String],
    constrained: bool,
) -> Result<Version> {
    if !constrained {
        return Ok(releases["crate"]["max_stable_version"]
            .as_str()
            .ok_or("latest stable missing")?
            .parse()?);
    }
    if requirements.is_empty() {
        return Err("constrained dependency has no verified parent requirements".into());
    }
    let requirements: Vec<_> = requirements
        .iter()
        .map(|r| VersionReq::parse(r))
        .collect::<std::result::Result<_, _>>()?;
    releases["versions"]
        .as_array()
        .ok_or("release versions missing")?
        .iter()
        .filter(|r| r["yanked"] == false)
        .filter_map(|r| r["num"].as_str().and_then(|v| v.parse::<Version>().ok()))
        .filter(|v| v.pre.is_empty() && requirements.iter().all(|r| r.matches(v)))
        .max()
        .ok_or_else(|| "no stable release satisfies the verified parent constraints".into())
}

pub fn latest(metadata: &[u8]) -> Result<Value> {
    let metadata: Value = serde_json::from_slice(metadata)?;
    let allowed = exceptions(&metadata)?;
    let packages = metadata["packages"].as_array().ok_or("packages missing")?;
    let by_id: HashMap<_, _> = packages
        .iter()
        .filter_map(|p| p["id"].as_str().map(|id| (id, p)))
        .collect();
    let mut parents: HashMap<&str, Vec<Value>> = HashMap::new();
    for node in metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("resolved graph missing")?
    {
        let parent = by_id
            .get(node["id"].as_str().ok_or("node ID missing")?)
            .ok_or("parent package missing")?;
        for edge in node["deps"].as_array().ok_or("dependencies missing")? {
            let id = edge["pkg"].as_str().ok_or("edge package missing")?;
            let dependency = by_id.get(id).ok_or("dependency package missing")?;
            for declaration in parent["dependencies"]
                .as_array()
                .ok_or("manifest dependencies missing")?
            {
                let alias = declaration["rename"]
                    .as_str()
                    .or(declaration["name"].as_str())
                    .ok_or("dependency alias missing")?
                    .replace('-', "_");
                if declaration["name"] == dependency["name"]
                    && Some(alias.as_str()) == edge["name"].as_str()
                    && edge["dep_kinds"]
                        .as_array()
                        .ok_or("dependency kinds missing")?
                        .iter()
                        .any(|kind| {
                            kind["kind"] == declaration["kind"]
                                && kind["target"] == declaration["target"]
                        })
                {
                    parents.entry(id).or_default().push(json!({"parent":parent["name"],"parent_version":parent["version"],"requirement":declaration["req"]}));
                }
            }
        }
    }
    let mut responses = HashMap::new();
    let mut records = Vec::new();
    for package in packages {
        if !package["source"]
            .as_str()
            .is_some_and(|s| s.starts_with("registry+"))
        {
            continue;
        }
        let name = package["name"].as_str().ok_or("package name missing")?;
        let id = package["id"].as_str().ok_or("package ID missing")?;
        let url = format!("https://crates.io/api/v1/crates/{name}");
        if !responses.contains_key(name) {
            let response = command(
                "curl",
                &[
                    "--fail".as_ref(),
                    "--silent".as_ref(),
                    "--show-error".as_ref(),
                    "--location".as_ref(),
                    "--connect-timeout".as_ref(),
                    "10".as_ref(),
                    "--max-time".as_ref(),
                    "60".as_ref(),
                    "--proto".as_ref(),
                    "=https".as_ref(),
                    "--user-agent".as_ref(),
                    "maafw-screencap-p0".as_ref(),
                    url.as_ref(),
                ],
                repository(),
            )
            .map_err(|e| format!("latest-release query {url}: {e}"))?;
            responses.insert(name, serde_json::from_slice::<Value>(&response)?);
        }
        let response = &responses[name];
        let upstream = parents.get(id).cloned().unwrap_or_default();
        let requirements: Vec<_> = upstream
            .iter()
            .filter_map(|p| p["requirement"].as_str().map(str::to_owned))
            .collect();
        let constrained = allowed.contains(id);
        let expected = required_version(response, &requirements, constrained)?.to_string();
        let latest = response["crate"]["max_stable_version"]
            .as_str()
            .ok_or("latest stable missing")?;
        records.push(json!({"name":name,"resolved":package["version"],"latest_stable":latest,"required_version":expected,"satisfies_policy":package["version"]==expected,"policy":if constrained {"binding_upstream_constraints"} else {"latest_stable"},"upgrade_gap":package["version"]!=latest,"upstream_constraints":upstream,"source":url,"license":package["license"]}));
    }
    Ok(Value::Array(records))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn releases() -> Value {
        json!({"crate":{"max_stable_version":"2.0.0"},"versions":[{"num":"1.2.0","yanked":false},{"num":"1.3.0","yanked":false},{"num":"1.4.0","yanked":true},{"num":"1.5.0-rc.1","yanked":false},{"num":"2.0.0","yanked":false}]})
    }
    #[test]
    fn requires_latest_compatible_release_even_when_global_major_is_constrained() {
        assert_eq!(
            required_version(&releases(), &["^1.2".into()], true).unwrap(),
            Version::new(1, 3, 0)
        );
    }
    #[test]
    fn does_not_waive_global_latest_outside_the_binding_exception() {
        assert_eq!(
            required_version(&releases(), &["^1.2".into()], false).unwrap(),
            Version::new(2, 0, 0)
        );
        assert!(required_version(&releases(), &[], true).is_err());
    }
    #[test]
    fn never_exempts_direct_or_shared_nonbinding_dependencies() {
        let metadata = json!({"packages":[{"id":"root","name":"root"},{"id":"pw","name":"pipewire"},{"id":"tool","name":"tool"},{"id":"shared","name":"shared"},{"id":"exclusive","name":"exclusive"}],"workspace_members":["root"],"resolve":{"nodes":[{"id":"root","deps":[{"pkg":"pw"},{"pkg":"tool"}]},{"id":"pw","deps":[{"pkg":"shared"},{"pkg":"exclusive"}]},{"id":"tool","deps":[{"pkg":"shared"}]},{"id":"shared","deps":[]},{"id":"exclusive","deps":[]}]}});
        assert_eq!(
            exceptions(&metadata).unwrap(),
            HashSet::from(["exclusive".to_owned()])
        );
    }
}
