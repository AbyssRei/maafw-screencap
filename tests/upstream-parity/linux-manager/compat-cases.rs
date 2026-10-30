// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Inputs for complete original legacy API bodies. No native Portal or input.
#[derive(Clone, Debug)]
pub struct Case {
    pub name: String,
    pub kwin: bool,
    pub path: Option<Vec<u8>>,
    pub width: i32,
    pub height: i32,
    pub vk: u8,
    pub open: u8,
    pub fd: i32,
    pub node: u32,
}
impl Case {
    pub fn valid_arguments(&self) -> bool {
        self.path.as_ref().is_some_and(|p| {
            !self.kwin
                || (!matches!(p.first(), None | Some(0)) && self.width > 0 && self.height > 0)
        })
    }
    pub fn created(&self) -> bool {
        self.valid_arguments() && (!self.kwin || self.open != 0)
    }
    pub fn events(&self) -> &'static str {
        if !self.kwin || !self.valid_arguments() {
            ""
        } else if self.open == 0 {
            "create,open"
        } else {
            "create,open,fd,node,destroy"
        }
    }
    pub fn residual(&self) -> usize {
        usize::from(self.kwin && self.valid_arguments() && self.open == 0)
    }
}
pub fn cases() -> Vec<Case> {
    let paths = [
        None,
        Some(Vec::new()),
        Some(b"owned-path".to_vec()),
        Some(b"\0unused".to_vec()),
        Some(b"owned-path\0suffix".to_vec()),
        Some(vec![b'p', 255, b'q']),
    ];
    let mut output = Vec::new();
    for (p, path) in paths.iter().enumerate() {
        for vk in [0, 1, 2, 255] {
            output.push(Case {
                name: format!("wlr-{p}-{vk}"),
                kwin: false,
                path: path.clone(),
                width: 0,
                height: 0,
                vk,
                open: 0,
                fd: -1,
                node: 0,
            });
        }
        for (d, (width, height)) in [(1, 1), (0, 1), (1, 0), (-1, 1), (i32::MAX, i32::MAX)]
            .into_iter()
            .enumerate()
        {
            for open in [0, 1, 255] {
                output.push(Case {
                    name: format!("kwin-{p}-{d}-{open}"),
                    kwin: true,
                    path: path.clone(),
                    width,
                    height,
                    vk: [0, 1, 2, 255][(p + d) % 4],
                    open,
                    fd: if d % 2 == 0 { -1 } else { 17 },
                    node: if p % 2 == 0 { 0 } else { u32::MAX },
                });
            }
        }
    }
    for (n, (fd, node)) in [
        (-1, 0),
        (-1, u32::MAX),
        (0, 0),
        (0, u32::MAX),
        (i32::MAX, 0),
        (i32::MAX, u32::MAX),
    ]
    .into_iter()
    .enumerate()
    {
        output.push(Case {
            name: format!("kwin-native-values-{n}"),
            kwin: true,
            path: Some(b"not-opened-input-device".to_vec()),
            width: 1,
            height: 1,
            vk: 255,
            open: 1,
            fd,
            node,
        });
    }
    output
}
