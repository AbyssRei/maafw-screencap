// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Readonly observations on the actual original outer/inner native owners.
use crate::Result;
use std::{fs, path::Path};

pub(super) fn verify(path: &Path, winners: &[u64]) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let rows = text
        .lines()
        .map(|line| {
            line.split('\t')
                .map(|value| {
                    let number = value.parse::<u64>()?;
                    if number.to_string() != value {
                        return Err("Noncanonical whole-chain scalar".into());
                    }
                    Ok(number)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    if !text.ends_with('\n') || rows.len() != 160 || rows.iter().any(|r| r.len() != 20) {
        return Err("Incomplete original whole-chain owner matrix".into());
    }
    let thread = rows[0][2];
    u32::try_from(thread)?;
    if thread == 0 || winners.len() != 20 {
        return Err("Missing whole-chain thread or complete cycles".into());
    }
    let mut class_size = None;
    for (cycle, &winner) in winners.iter().enumerate() {
        let group = &rows[cycle * 8..cycle * 8 + 8];
        let active = &group[1];
        for (stage, row) in group.iter().enumerate() {
            if row[..3] != [u64::try_from(cycle)?, u64::try_from(stage)?, thread] {
                return Err("Whole-chain operation order/thread differs".into());
            }
            for &pointer in &row[3..6] {
                usize::try_from(pointer)?;
                if pointer % 8 != 0 {
                    return Err("Invalid whole-chain owner/module address".into());
                }
            }
            if stage == 7 {
                if row[3..] != [0; 17] {
                    return Err("Original chain/module survived outer destruction".into());
                }
                continue;
            }
            if row[3] == 0 || row[3] != group[0][3] || row[19] != 1 {
                return Err("Actual original Gamepad class/owner changed".into());
            }
            if stage == 0 {
                if row[4..19] != [0; 15] {
                    return Err("Cold original outer created an inner owner/module".into());
                }
                continue;
            }
            if row[4] == 0
                || row[5] == 0
                || row[4..19] != active[4..19]
                || row[6..11] != [0x4d465357, 1, 0, 0, 1]
                || row[11] < 8
                || row[12] != 8
                || row[13] != thread
                || row[14..17] != [0x4d465353, 1, winner]
                || row[17] == 0
                || row[17] % 8 != 0
                || row[18] != thread
            {
                return Err(
                    "Original Win32 class/backend/owner retention or exclusions differ".into(),
                );
            }
            usize::try_from(row[17])?;
            if let Some(old) = class_size.replace(row[11])
                && old != row[11]
            {
                return Err("Original inner class size changed".into());
            }
        }
    }
    Ok(())
}
