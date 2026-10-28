// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use crate::platform::windows::geometry;
use std::{cell::RefCell, ptr::null_mut};
#[cfg(feature = "prototype-windows")]
use {crate::platform::windows::foreground, std::sync::atomic::AtomicU32};
#[path = "geometry_fork_tests.rs"]
mod corrected_fork;
#[cfg(feature = "prototype-windows")]
#[path = "../../../tests/upstream-parity/window_adapter.rs"]
mod original;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Event(u32, usize, [i64; 4]);
#[derive(Clone)]
struct GeometryChange {
    trigger: u32,
    occurrence: u32,
    outer: Rect,
    client: Rect,
    origin: Point,
    style: isize,
}
#[derive(Clone)]
struct State {
    valid: bool,
    zoomed: bool,
    iconic: bool,
    has_monitor: bool,
    foreground: Window,
    accepts: bool,
    ticks: u32,
    fail: u32,
    fail_occurrence: u32,
    change: Option<GeometryChange>,
    monitor: Rect,
    work: Rect,
    outer: Rect,
    client: Rect,
    origin: Point,
    dpi: u32,
    style: isize,
}
impl Default for State {
    fn default() -> Self {
        Self {
            valid: true,
            zoomed: false,
            iconic: false,
            has_monitor: true,
            foreground: null_mut(),
            accepts: false,
            ticks: 0,
            fail: 0,
            fail_occurrence: 1,
            change: None,
            monitor: Rect {
                left: -9999,
                top: -9999,
                right: 9999,
                bottom: 9999,
            },
            work: Rect {
                left: -100,
                top: -50,
                right: 900,
                bottom: 650,
            },
            outer: Rect {
                left: 0,
                top: 0,
                right: 110,
                bottom: 120,
            },
            client: Rect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 90,
            },
            origin: Point { x: 5, y: 25 },
            dpi: 144,
            style: 0,
        }
    }
}
struct Script {
    state: RefCell<State>,
    events: RefCell<Vec<Event>>,
    queries: RefCell<[u32; 17]>,
}
impl Script {
    fn new(state: State) -> Self {
        Self {
            state: RefCell::new(state),
            events: RefCell::new(Vec::new()),
            queries: RefCell::new([0; 17]),
        }
    }
    fn event(&self, op: u32, window: Window, args: [i64; 4]) {
        self.events
            .borrow_mut()
            .push(Event(op, window.addr(), args));
    }
    fn boolean(&self, op: u32, window: Window, value: bool) -> bool {
        self.event(op, window, [i64::from(value), 0, 0, 0]);
        value
    }
    fn rect(&self, op: u32, window: Window, value: Rect) -> Result<Rect, u32> {
        self.event(
            op,
            window,
            [value.left, value.top, value.right, value.bottom].map(i64::from),
        );
        let occurrence = self.after_query(op);
        if self.state.borrow().fail == op && occurrence >= self.state.borrow().fail_occurrence {
            Err(42)
        } else {
            Ok(value)
        }
    }
    fn after_query(&self, op: u32) -> u32 {
        let occurrence = {
            let mut queries = self.queries.borrow_mut();
            queries[usize::try_from(op).unwrap()] += 1;
            queries[usize::try_from(op).unwrap()]
        };
        let change = self.state.borrow().change.clone();
        if let Some(change) =
            change.filter(|change| change.trigger == op && change.occurrence == occurrence)
        {
            let mut state = self.state.borrow_mut();
            state.outer = change.outer;
            state.client = change.client;
            state.origin = change.origin;
            state.style = change.style;
        }
        occurrence
    }
}
impl WindowApi for Script {
    fn valid(&self, w: Window) -> bool {
        self.boolean(1, w, self.state.borrow().valid)
    }
    fn foreground(&self) -> Window {
        let w = self.state.borrow().foreground;
        self.event(2, w, [0; 4]);
        w
    }
    fn foreground_request(&self, w: Window) {
        self.event(3, w, [0; 4]);
        if self.state.borrow().accepts {
            self.state.borrow_mut().foreground = w;
        }
    }
    fn order(&self, w: Window, o: Order, show: bool) {
        self.event(
            4,
            w,
            [
                match o {
                    Order::Top => 0,
                    Order::Topmost => 1,
                    Order::NotTopmost => 2,
                },
                i64::from(show),
                0,
                0,
            ],
        );
    }
    fn ticks(&self) -> u32 {
        let t = self.state.borrow().ticks;
        self.event(5, null_mut(), [i64::from(t), 0, 0, 0]);
        t
    }
    fn sleep(&self, d: Duration) {
        self.event(
            6,
            null_mut(),
            [i64::try_from(d.as_millis()).unwrap(), 0, 0, 0],
        );
    }
    fn zoomed(&self, w: Window) -> bool {
        self.boolean(7, w, self.state.borrow().zoomed)
    }
    fn iconic(&self, w: Window) -> bool {
        self.boolean(8, w, self.state.borrow().iconic)
    }
    fn monitor(&self, w: Window) -> Monitor {
        let m = if self.state.borrow().has_monitor {
            3
        } else {
            0
        };
        self.event(9, w, [m, 0, 0, 0]);
        std::ptr::without_provenance_mut(usize::try_from(m).unwrap())
    }
    fn monitor_rects(&self, m: Monitor) -> Result<MonitorRects, u32> {
        self.rect(10, m, self.state.borrow().work)
            .map(|work| MonitorRects {
                work,
                monitor: self.state.borrow().monitor,
            })
    }
    fn window_rect(&self, w: Window) -> Result<Rect, u32> {
        let outer = self.state.borrow().outer;
        self.rect(11, w, outer)
    }
    fn client_rect(&self, w: Window) -> Result<Rect, u32> {
        let client = self.state.borrow().client;
        self.rect(12, w, client)
    }
    fn origin(&self, w: Window, p: Point) -> Result<Point, u32> {
        self.event(13, w, [i64::from(p.x), i64::from(p.y), 0, 0]);
        let origin = self.state.borrow().origin;
        let occurrence = self.after_query(13);
        if self.state.borrow().fail == 13 && occurrence >= self.state.borrow().fail_occurrence {
            Err(42)
        } else {
            Ok(origin)
        }
    }
    fn position(&self, w: Window, p: Position) -> Result<(), u32> {
        self.event(14, w, [p.x, p.y, p.width, p.height].map(i64::from));
        if self.state.borrow().fail == 14 {
            Err(42)
        } else {
            Ok(())
        }
    }
    fn dpi(&self, w: Window) -> u32 {
        let d = self.state.borrow().dpi;
        self.event(15, w, [i64::from(d), 0, 0, 0]);
        d
    }
    fn style(&self, w: Window) -> isize {
        let s = self.state.borrow().style;
        self.event(16, w, [i64::try_from(s).unwrap(), 0, 0, 0]);
        self.after_query(16);
        s
    }
}
fn window(id: usize) -> Window {
    std::ptr::without_provenance_mut(id)
}

#[test]
fn rejects_checked_geometry_overflow_before_any_position_request() {
    let s = Script::new(State {
        work: Rect {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 700,
        },
        ..State::default()
    });
    assert_eq!(
        geometry::ensure_on_screen(&s, window(1)),
        Err(geometry::Error::Overflow)
    );
    assert!(!s.events.borrow().iter().any(|e| e.0 == 14));
    s.state.borrow_mut().client = Rect {
        left: i32::MIN,
        top: 0,
        right: i32::MAX,
        bottom: 1,
    };
    assert_eq!(
        geometry::window_size(&s, window(1)),
        Err(geometry::Error::Overflow)
    );
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit compiled original window helpers; requires ScreenDC parity adapter"]
fn preserves_original_global_cooldown_wrap_sentinel_and_foreground_call_order()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = original::Adapter::load()?;
    let rust = Script::new(State::default());
    let source = Script::new(State::default());
    let last = AtomicU32::new(0);
    // One continuous original-static sequence. Cross-window calls share it.
    let steps = [
        (0, 1, 0, true, false),
        (0, 2, 0, true, false),
        (100, 1, 0, true, false),
        (5099, 2, 0, true, false),
        (5100, 2, 0, true, true),
        (5101, 1, 1, true, false),
        (5102, 1, 0, false, false),
        (5103, 0, 0, true, false),
        (u32::MAX - 10, 1, 0, true, false),
        (4988, 2, 0, true, false),
        (4989, 2, 0, true, true),
    ];
    for (ticks, w, f, valid, accepts) in steps {
        for s in [&rust, &source] {
            *s.state.borrow_mut() = State {
                ticks,
                foreground: window(f),
                valid,
                accepts,
                ..State::default()
            };
            s.events.borrow_mut().clear();
        }
        let result = foreground::with_cooldown(&rust, window(w), &last);
        let expected = adapter.run(&source, 0, window(w))?;
        assert_eq!(
            i64::from(result),
            expected.first,
            "tick={ticks}, window={w}"
        );
        assert_eq!(
            *rust.events.borrow(),
            *source.events.borrow(),
            "tick={ticks}, window={w}"
        );
    }
    for w in [0, 1] {
        rust.events.borrow_mut().clear();
        source.events.borrow_mut().clear();
        foreground::ensure_topmost(&rust, window(w));
        adapter.run(&source, 1, window(w))?;
        assert_eq!(*rust.events.borrow(), *source.events.borrow());
    }
    println!("original ForegroundUtils: 11 continuous cooldown steps and 2 direct topmost cases");
    Ok(())
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit compiled original window helpers; requires ScreenDC parity adapter"]
fn preserves_source_work_area_geometry_and_query_short_circuits()
-> Result<(), Box<dyn std::error::Error>> {
    let adapter = original::Adapter::load()?;
    let base = State::default();
    let mut cases = vec![(0, base.clone()), (1, base.clone())];
    cases.extend(
        [
            State {
                valid: false,
                ..base.clone()
            },
            State {
                zoomed: true,
                ..base.clone()
            },
            State {
                iconic: true,
                ..base.clone()
            },
            State {
                has_monitor: false,
                ..base.clone()
            },
        ]
        .map(|s| (1, s)),
    );
    for fail in 10..=14 {
        cases.push((
            1,
            State {
                fail,
                origin: Point { x: 999, y: 999 },
                ..base.clone()
            },
        ));
    }
    for origin in [
        Point { x: 899, y: 649 },
        Point { x: -300, y: -200 },
        Point { x: -90, y: -40 },
    ] {
        cases.push((
            1,
            State {
                origin,
                ..base.clone()
            },
        ));
    }
    cases.push((
        1,
        State {
            client: Rect {
                left: 0,
                top: 0,
                right: 2000,
                bottom: 1000,
            },
            style: 0x8000_0000,
            ..base.clone()
        },
    ));
    // Work-area corrections have no tolerance: preserve small overflows.
    cases.push((
        1,
        State {
            origin: Point { x: 801, y: 561 },
            ..base.clone()
        },
    ));
    for (index, (w, state)) in cases.into_iter().enumerate() {
        let rust = Script::new(state.clone());
        let source = Script::new(state);
        let got = geometry::ensure_on_screen(&rust, window(w));
        let expected = adapter.run(&source, 2, window(w))?;
        assert_eq!(i64::from(got.is_ok()), expected.first, "case={index}");
        assert_eq!(
            *rust.events.borrow(),
            *source.events.borrow(),
            "case={index}"
        );
    }
    for state in [
        base.clone(),
        State {
            fail: 12,
            ..base.clone()
        },
        State {
            client: Rect {
                left: 7,
                top: 9,
                right: 4,
                bottom: 3,
            },
            ..base.clone()
        },
    ] {
        let rust = Script::new(state.clone());
        let source = Script::new(state);
        let got = geometry::window_size(&rust, window(1)).unwrap();
        let expected = adapter.run(&source, 3, window(1))?;
        assert_eq!(
            (i64::from(got.0), i64::from(got.1)),
            (expected.first, expected.second)
        );
        assert_eq!(*rust.events.borrow(), *source.events.borrow());
    }
    for style in [0, 0x8000_0000, 0x1_0000_0000, -0x8000_0000] {
        let rust = Script::new(State {
            style,
            ..base.clone()
        });
        let source = Script::new(State {
            style,
            ..base.clone()
        });
        assert_eq!(
            i64::from(geometry::is_fullscreen(&rust, window(1))),
            adapter.run(&source, 5, window(1))?.first
        );
        assert_eq!(*rust.events.borrow(), *source.events.borrow());
    }
    for dpi in [0, 96, 144, 192] {
        let rust = Script::new(State {
            dpi,
            ..base.clone()
        });
        let source = Script::new(State {
            dpi,
            ..base.clone()
        });
        assert_eq!(
            geometry::window_scale(&rust, window(1)),
            adapter.run(&source, 4, window(1))?.scale
        );
        assert_eq!(*rust.events.borrow(), *source.events.borrow());
    }
    println!(
        "original HwndUtils: 16 geometry, 3 size, 4 style and 4 scale cases; overflow checked separately"
    );
    Ok(())
}
