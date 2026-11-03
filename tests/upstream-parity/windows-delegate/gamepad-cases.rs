// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Directed outer host/capture ownership cases; no input actions or desktop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Case {
    pub hwnd: u64,
    pub gamepad_type: u64,
    pub method: u64,
    pub host: [u32; 2],
    pub flags: [u32; 2],
    pub outcome: u32,
    pub inactive: u32,
}
impl Case {
    pub fn arguments(self) -> [String; 9] {
        [
            self.hwnd.to_string(),
            self.gamepad_type.to_string(),
            self.method.to_string(),
            self.host[0].to_string(),
            self.host[1].to_string(),
            self.flags[0].to_string(),
            self.flags[1].to_string(),
            self.outcome.to_string(),
            self.inactive.to_string(),
        ]
    }
}
pub fn matrix() -> Vec<Case> {
    let base = Case {
        hwnd: 0x1234,
        gamepad_type: 0,
        method: (1 << 63) | 1,
        host: [0, 0],
        flags: [0, 0],
        outcome: 1,
        inactive: 1,
    };
    let mut cases = Vec::new();
    // Cold and warm host true/false/error/create error/live disconnection.
    for first in 0..5 {
        for second in 0..5 {
            cases.push(Case {
                host: [first, second],
                ..base
            });
        }
    }
    // Capture false/exception is not the same as a failed host connection.
    for flag in [1, 2, 4, 8, 16, 32, 64, 128] {
        cases.push(Case {
            flags: [flag, 0],
            ..base
        });
        cases.push(Case {
            flags: [0, flag],
            ..base
        });
    }
    // The existing owner still forwards changed/false/error output after each
    // host failure; inactive also preserves its own failure/exception result.
    for host in [1, 2, 3] {
        for outcome in [0, 2, 4, 5] {
            for inactive in [0, 2] {
                cases.push(Case {
                    host: [0, host],
                    outcome,
                    inactive,
                    ..base
                });
            }
        }
    }
    for (hwnd, method) in [(0, 0), (0, 1), (0, u64::MAX), (u64::MAX, 0)] {
        cases.push(Case {
            hwnd,
            method,
            ..base
        });
    }
    for gamepad_type in [1, 1 << 63, u64::MAX] {
        cases.push(Case {
            hwnd: u64::MAX,
            gamepad_type,
            method: u64::MAX,
            ..base
        });
    }
    cases
}
