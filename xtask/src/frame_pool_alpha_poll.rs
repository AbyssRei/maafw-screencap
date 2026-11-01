// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Forwarded native polling/cache boundaries, never original frame freshness.
use super::*;

const HEADER: &str =
    "arm\tphase\tdrain_calls\tdrain_frames\tpoll_calls\tpoll_frames\tcache_clones\terrors";

fn table(root: &Path, observations: &[Vec<u64>], rust: bool) -> Result<Vec<Vec<u64>>> {
    let text = fs::read_to_string(root.join("poll-branches.tsv"))?;
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("Alpha polling observer header differs".into());
    }
    let values = lines.map(numbers).collect::<Result<Vec<_>>>()?;
    if values.len() != 10 || observations.len() != 10 {
        return Err("Alpha polling observation requires all ten calls".into());
    }
    for (index, row) in values.iter().enumerate() {
        let output = &observations[index];
        if row.len() != 8
            || row.contains(&u64::MAX)
            || row[0] != (index / 5) as u64
            || row[1] != (index % 5) as u64
            || row[3].checked_add(1) != Some(row[2])
            || row[4] == 0
            || row[5] > 1
            || row[6] > 1
            || row[5] + row[6] != 1
            || row[7] != 0
            || row[0] == 0 && row[6] != 0
            || row[6] == 1 && output[11] < 2_000_000
            || rust && (row[6] != output[12] || row[5] != output[17])
        {
            return Err("Alpha forwarded drain/poll/cache evidence differs".into());
        }
    }
    Ok(values)
}

pub(super) fn check(source: &Path, rust: &Path) -> Result<serde_json::Value> {
    let observations = |root: &Path| -> Result<Vec<Vec<u64>>> {
        fs::read_to_string(root.join("observations.tsv"))?
            .lines()
            .skip(1)
            .map(numbers)
            .collect()
    };
    // The caller has already validated both complete image/state tables.
    let original = table(source, &observations(source)?, false)?;
    let migrated = table(rust, &observations(rust)?, true)?;
    Ok(serde_json::json!({"source":original,"rust":migrated,
        "scope":"Separate original-source interposition forwards exactly the original two TryGetNextFrame expressions and timeout cache clone. Rust records existing successful-call evidence. Counts observe native frame presence, drained frames and the actual cache branch; they do not observe frame timestamps or prove a historical failure cause. No additional native query, frame retention, wait change or capture retry."}))
}
