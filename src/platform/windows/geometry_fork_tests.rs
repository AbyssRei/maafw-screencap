// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Revised PR behavior at the actual helper boundary, separate from capture.
use super::*;

struct Case {
    name: String,
    state: State,
    position: Option<Position>,
    succeeds: bool,
}
fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
    Rect {
        left,
        top,
        right,
        bottom,
    }
}
fn position(x: i32, y: i32, width: i32, height: i32) -> Option<Position> {
    Some(Position {
        x,
        y,
        width,
        height,
    })
}
fn popup(area: Rect) -> State {
    State {
        monitor: rect(0, 0, 1920, 1080),
        work: rect(0, 0, 1920, 1040),
        outer: area,
        client: rect(0, 0, area.right - area.left, area.bottom - area.top),
        origin: Point {
            x: area.left,
            y: area.top,
        },
        style: 0x8000_0000,
        ..State::default()
    }
}
fn cases() -> Vec<Case> {
    let full = popup(rect(0, 0, 1920, 1080));
    let mut cases = Vec::new();
    let mut push = |name: String, state: State, position: Option<Position>, succeeds: bool| {
        cases.push(Case {
            name,
            state,
            position,
            succeeds,
        });
    };
    push("exact_popup".into(), full.clone(), None, true);
    let framed = State {
        outer: rect(-8, -8, 1928, 1088),
        ..full.clone()
    };
    push(
        "exact_client_with_outer_borders".into(),
        framed.clone(),
        None,
        true,
    );
    for edge in 0..4 {
        for delta in [-9, -8, -7, -1, 1, 7, 8, 9] {
            let mut edges = [0, 0, 1920, 1080];
            edges[edge] += delta;
            // These expected values describe the work-area result, including
            // smaller clients; they are independent of the exemption code.
            let (x, width) = match edge {
                0 if delta > 0 => (delta, 1920 - delta),
                2 if delta < 0 => (0, 1920 + delta),
                _ => (0, 1920),
            };
            push(
                format!("edge_{edge}_delta_{delta}"),
                popup(rect(edges[0], edges[1], edges[2], edges[3])),
                position(x, 0, width, 1040),
                true,
            );
        }
    }
    push(
        "review_negative_one_translation".into(),
        popup(rect(-1, 0, 1919, 1080)),
        position(0, 0, 1920, 1040),
        true,
    );
    push(
        "oversized_popup".into(),
        popup(rect(-100, -100, 2020, 1180)),
        position(0, 0, 1920, 1040),
        true,
    );
    push(
        "ordinary_client_in_taskbar".into(),
        State {
            style: 0,
            ..popup(rect(100, 1020, 500, 1080))
        },
        position(100, 980, 400, 60),
        true,
    );
    push(
        "ordinary_framed_monitor_client".into(),
        State { style: 0, ..framed },
        position(-8, -8, 1936, 1056),
        true,
    );
    push(
        "smaller_popup_in_work_area".into(),
        popup(rect(10, 10, 640, 480)),
        None,
        true,
    );
    push(
        "exact_negative_monitor".into(),
        State {
            monitor: rect(-1920, -1080, 0, 0),
            work: rect(-1920, -1080, 0, -40),
            ..popup(rect(-1920, -1080, 0, 0))
        },
        None,
        true,
    );
    for (name, work, desired) in [
        ("left", rect(40, 0, 1920, 1080), position(40, 0, 1880, 1080)),
        ("top", rect(0, 40, 1920, 1080), position(0, 40, 1920, 1040)),
        ("right", rect(0, 0, 1880, 1080), position(0, 0, 1880, 1080)),
        ("bottom", rect(0, 0, 1920, 1040), position(0, 0, 1920, 1040)),
        ("hidden", rect(0, 0, 1920, 1080), None),
    ] {
        push(
            format!("popup_taskbar_{name}"),
            State {
                work,
                ..full.clone()
            },
            None,
            true,
        );
        push(
            format!("ordinary_taskbar_{name}"),
            State {
                work,
                style: 0,
                ..full.clone()
            },
            desired,
            true,
        );
    }
    for fail in [12, 13] {
        push(
            format!("exact_query_failure_{fail}"),
            State {
                fail,
                ..full.clone()
            },
            None,
            false,
        );
    }
    push(
        "position_failure".into(),
        State {
            style: 0,
            fail: 14,
            ..full.clone()
        },
        position(0, 0, 1920, 1040),
        false,
    );
    push(
        "maximized_exemption".into(),
        State {
            zoomed: true,
            ..full.clone()
        },
        None,
        true,
    );
    push(
        "minimized_exemption".into(),
        State {
            iconic: true,
            ..full
        },
        None,
        true,
    );
    let full = popup(rect(0, 0, 1920, 1080));
    for (name, change) in [
        (
            "resize_between_client_and_origin",
            GeometryChange {
                trigger: 12,
                occurrence: 1,
                outer: rect(0, 0, 2400, 1200),
                client: rect(0, 0, 2400, 1200),
                origin: Point { x: 0, y: 0 },
                style: 0x8000_0000,
            },
        ),
        (
            "move_after_candidate_style",
            GeometryChange {
                trigger: 16,
                occurrence: 1,
                outer: rect(-1, 0, 1919, 1080),
                client: rect(0, 0, 1920, 1080),
                origin: Point { x: -1, y: 0 },
                style: 0x8000_0000,
            },
        ),
        (
            "style_changed_before_confirmation",
            GeometryChange {
                trigger: 16,
                occurrence: 1,
                outer: rect(0, 0, 1920, 1080),
                client: rect(0, 0, 1920, 1080),
                origin: Point { x: 0, y: 0 },
                style: 0,
            },
        ),
    ] {
        push(
            name.into(),
            State {
                change: Some(change),
                ..full.clone()
            },
            position(0, 0, 1920, 1040),
            true,
        );
    }
    for fail in [11, 12, 13] {
        push(
            format!("confirmation_query_failure_{fail}"),
            State {
                fail,
                fail_occurrence: 2,
                ..full.clone()
            },
            None,
            false,
        );
    }
    assert_eq!(cases.len(), 61);
    cases
}
fn positions(script: &Script) -> Vec<Position> {
    script
        .events
        .borrow()
        .iter()
        .filter(|event| event.0 == 14)
        .map(|event| {
            let [x, y, width, height] = event.2.map(|value| i32::try_from(value).unwrap());
            Position {
                x,
                y,
                width,
                height,
            }
        })
        .collect()
}
fn assert_result(case: &Case, script: &Script, succeeded: bool) {
    assert_eq!(succeeded, case.succeeds, "{}", case.name);
    assert_eq!(
        positions(script),
        case.position.into_iter().collect::<Vec<_>>(),
        "{}",
        case.name
    );
    if (case.state.fail_occurrence == 1 && (case.state.fail == 12 || case.state.fail == 13))
        || case.state.zoomed
        || case.state.iconic
    {
        assert!(
            !script.events.borrow().iter().any(|event| event.0 == 16),
            "{}",
            case.name
        );
    }
}
#[test]
fn exempts_only_exact_popup_clients_and_keeps_ordinary_work_area_correction() {
    for case in cases() {
        let script = Script::new(case.state.clone());
        assert_result(
            &case,
            &script,
            geometry::ensure_on_screen(&script, window(1)).is_ok(),
        );
    }
    let script = Script::new(popup(rect(0, 0, 1920, 1080)));
    geometry::ensure_on_screen(&script, window(1)).unwrap();
    for op in [11, 12, 13, 16] {
        assert_eq!(
            script
                .events
                .borrow()
                .iter()
                .filter(|event| event.0 == op)
                .count(),
            2,
            "one confirmation only, op={op}"
        );
    }
}

#[cfg(feature = "prototype-windows")]
#[test]
#[ignore = "explicit corrected-fork C++ helper adapter, no desktop capture"]
fn matches_corrected_fork_client_edges_query_failures_and_position_flags()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let adapter = original::Adapter::load()?;
    let directory = std::path::PathBuf::from(
        std::env::var_os("MFS_F_GEOMETRY_ARTIFACTS")
            .ok_or("explicit geometry artifact directory required")?,
    );
    let mut inputs = std::fs::File::create_new(directory.join("cases.tsv"))?;
    let mut events = std::fs::File::create_new(directory.join("events.tsv"))?;
    let mut changes = std::fs::File::create_new(directory.join("changes.tsv"))?;
    for (index, case) in cases().into_iter().enumerate() {
        write!(changes, "{index}\t{}", case.state.fail_occurrence)?;
        if let Some(change) = &case.state.change {
            write!(changes, "\t{}\t{}", change.trigger, change.occurrence)?;
            for value in [
                change.outer.left,
                change.outer.top,
                change.outer.right,
                change.outer.bottom,
                change.client.left,
                change.client.top,
                change.client.right,
                change.client.bottom,
                change.origin.x,
                change.origin.y,
            ] {
                write!(changes, "\t{value}")?;
            }
            writeln!(changes, "\t{}", change.style)?;
        } else {
            writeln!(changes, "\t0\t0")?;
        }
        let rust = Script::new(case.state.clone());
        let source = Script::new(case.state.clone());
        let got = geometry::ensure_on_screen(&rust, window(1));
        let expected = adapter.run(&source, 2, window(1))?;
        assert_result(&case, &rust, got.is_ok());
        assert_result(&case, &source, expected.first != 0);
        assert_eq!(
            *rust.events.borrow(),
            *source.events.borrow(),
            "{}",
            case.name
        );
        write!(
            inputs,
            "{index}\t{}\t{}\t{}\t{}\t{}",
            case.name,
            case.state.style,
            case.state.fail,
            u8::from(case.state.zoomed),
            u8::from(case.state.iconic)
        )?;
        for rect in [
            case.state.monitor,
            case.state.work,
            case.state.outer,
            case.state.client,
        ] {
            for scalar in [rect.left, rect.top, rect.right, rect.bottom] {
                write!(inputs, "\t{scalar}")?;
            }
        }
        writeln!(
            inputs,
            "\t{}\t{}\t{}\t{}\t{}",
            case.state.origin.x,
            case.state.origin.y,
            case.state.dpi,
            u8::from(got.is_ok()),
            expected.first
        )?;
        for (side, script) in [("rust", &rust), ("source", &source)] {
            for event in script.events.borrow().iter() {
                writeln!(
                    events,
                    "{index}\t{side}\t{}\t{}\t{}\t{}\t{}\t{}",
                    event.0, event.1, event.2[0], event.2[1], event.2[2], event.2[3]
                )?;
            }
        }
    }
    println!(
        "corrected fork geometry cases=61 source_and_rust=122 exact_client_exemption=1 bounded_confirmation=1 work_area_preserved=1 offsets_have_no_slack=1 native_capture=0 passed"
    );
    Ok(())
}
