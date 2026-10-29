// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
// Shared input matrix only; original compiled get_info supplies the oracle.
pub fn cases(kind: &str) -> Option<Vec<(u64, u64)>> {
    let identities: &[u64] = match kind {
        "win32" => &[
            0,
            1,
            0xffff,
            0x8000_0000,
            0xffff_ffff,
            0x7fff_ffff_ffff_ffff,
            0x8000_0000_0000_0000,
            u64::MAX,
        ],
        "macos" => &[0, 1, 0xffff, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff],
        _ => return None,
    };
    const METHODS: [u64; 11] = [
        0,
        1,
        2,
        3,
        4,
        7,
        1 << 31,
        1 << 32,
        0x7fff_ffff_ffff_ffff,
        1 << 63,
        u64::MAX,
    ];
    Some(
        identities
            .iter()
            .flat_map(|&id| METHODS.map(|method| (id, method)))
            .collect(),
    )
}
