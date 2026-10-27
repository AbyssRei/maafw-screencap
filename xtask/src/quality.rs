// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::{Result, git, repository};

pub fn run(args: &[String]) -> Result<()> {
    let message = match args {
        [] => String::from_utf8(git(repository(), &["log", "-1", "--format=%B"])?)?,
        [flag, path] if flag == "--message-file" => std::fs::read_to_string(path)?,
        _ => return Err("expected check [--message-file <path>]".into()),
    };
    commit_message(&message)?;
    let metadata: serde_json::Value = serde_json::from_slice(&crate::command(
        "cargo",
        &[
            "metadata".as_ref(),
            "--format-version=1".as_ref(),
            "--no-deps".as_ref(),
        ],
        repository(),
    )?)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("Cargo packages missing")?;
    let product = packages
        .iter()
        .find(|p| p["name"] == "maafw-screencap")
        .ok_or("capture package missing")?;
    let task = packages
        .iter()
        .find(|p| p["name"] == "xtask")
        .ok_or("xtask package missing")?;
    if product["version"] != task["version"] {
        return Err("product/xtask version mismatch".into());
    }
    for package in [product, task] {
        if package["license"] != "LGPL-3.0-or-later" {
            return Err("workspace package license mismatch".into());
        }
    }
    println!("Conventional Commit, workspace version and license metadata checks passed.");
    Ok(())
}

fn commit_message(message: &str) -> Result<()> {
    let subject = message.lines().next().ok_or("empty commit message")?;
    let (prefix, description) = subject
        .split_once(": ")
        .ok_or("commit subject must use <type>[scope][!]: <description>")?;
    if description.trim().is_empty() {
        return Err("commit description is empty".into());
    }
    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let kind = if let Some((kind, scope)) = prefix.split_once('(') {
        let scope = scope.strip_suffix(')').ok_or("unclosed commit scope")?;
        if scope.trim().is_empty() || scope.contains(['(', ')']) {
            return Err("invalid commit scope".into());
        }
        kind
    } else {
        prefix
    };
    if kind.is_empty() || !kind.bytes().all(|c| c.is_ascii_lowercase()) {
        return Err("commit type must contain lowercase ASCII letters".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_scopes_breaking_markers_and_custom_types() {
        for message in [
            "feat: capture",
            "fix(gdi)!: reject invalid buffers\n\nBREAKING CHANGE: old buffers are rejected",
            "ci: validate",
            "custom: purpose",
        ] {
            commit_message(message).unwrap();
        }
    }

    #[test]
    fn rejects_malformed_or_empty_commit_subjects() {
        for message in [
            "",
            "update code",
            "Feat: wrong case",
            "feat(): empty scope",
            "fix(gdi: missing close",
            "fix!: ",
            "fix: ",
            "fix(gdi))!: malformed",
        ] {
            assert!(commit_message(message).is_err(), "accepted {message:?}");
        }
    }
}
