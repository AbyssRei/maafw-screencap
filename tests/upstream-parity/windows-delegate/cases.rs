// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Explicit source-model cases, shared by execution and record validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Case {
    pub hwnd: u64,
    pub method: u64,
    pub flags: [u32; 2],
    pub outcome: u32,
    pub inactive: u32,
}
impl Case {
    pub fn arguments(self) -> [String; 6] {
        [
            self.hwnd.to_string(),
            self.method.to_string(),
            self.flags[0].to_string(),
            self.flags[1].to_string(),
            self.outcome.to_string(),
            self.inactive.to_string(),
        ]
    }
}
pub fn matrix() -> Vec<Case> {
    let flags = [0, 1, 2, 4, 8, 16, 32, 64, 128];
    let mut cases = Vec::new();
    for reconnect in flags {
        for outcome in 0..6 {
            for inactive in 0..3 {
                cases.push(Case {
                    hwnd: 0x1234,
                    method: (1 << 63) | 1,
                    flags: [0, reconnect],
                    outcome,
                    inactive,
                });
            }
        }
    }
    for initial in flags[1..].iter().copied() {
        for outcome in [0, 1, 4, 5] {
            for inactive in 0..3 {
                cases.push(Case {
                    hwnd: 0x1234,
                    method: 1,
                    flags: [initial, 0],
                    outcome,
                    inactive,
                });
            }
        }
    }
    for (hwnd, method) in [(0, 0), (0, 1), (0, u64::MAX), (u64::MAX, 0)] {
        cases.push(Case {
            hwnd,
            method,
            flags: [0, 0],
            outcome: 1,
            inactive: 1,
        });
    }
    for hwnd in [1, u64::MAX] {
        for method in [1, 1 << 63, u64::MAX] {
            for outcome in [1, 2] {
                cases.push(Case {
                    hwnd,
                    method,
                    flags: [0, 0],
                    outcome,
                    inactive: 1,
                });
            }
        }
    }
    cases
}
