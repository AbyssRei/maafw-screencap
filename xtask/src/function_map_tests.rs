// Copyright (C) 2026 AbyssRei
// SPDX-License-Identifier: LGPL-3.0-or-later
use super::*;
use serde_json::{Value, json};

// Schema/graph controls only. These placeholders name no real source or run;
// the read-only command's Git/artifact checks use the external native corpus.
fn fixture() -> (Value, Value) {
    let hash = "0".repeat(64);
    let owner = format!("0:10:{hash}");
    let closure = format!("3:4:{hash}");
    let input = format!("11:20:{hash}");
    let artifact = json!({"path":"fixture","sha256":hash});
    let rust = json!({"artifact":artifact,"product_path":"src/lib.rs","start":0,"end":1,"line":1,"sha256":hash,"name":"capture"});
    let evidence = json!({"reference_root":"runs/context","manifest":artifact,"check":"capture","status":"failed","log":artifact});
    let functions = [
        (&owner, "capture"),
        (&closure, "capture_closure"),
        (&input, "excluded_application_or_input"),
    ];
    let mappings: Vec<_> = functions.iter().map(|(id, role)| {
        let excluded = *role == "excluded_application_or_input";
        json!({"source_path":"source.cpp","source_id":id,"role":role,"rationale":"scoped fixture only",
            "rust":if excluded {vec![]} else {vec![rust.clone()]},
            "evidence":if excluded {vec![]} else {vec![evidence.clone()]},"semantic_equivalence_accepted":false})
    }).collect();
    let map = json!({"layout_version":1,"family":"fixture","baseline":artifact,"census":artifact,"product_head":"1".repeat(40),
        "functions":mappings,"runtime":[],"limitations":["No actual source/behavior acceptance"],"native_acceptance":false,"full_P1_accepted":false});
    let source_functions = json!([
        {"id":owner,"kind":"function","name":"capture","start":0,"end":10,"line":1,"sha256":hash,"parent":null,"role":"capture"},
        {"id":closure,"kind":"closure","name":"closure","start":3,"end":4,"line":1,"sha256":hash,"parent":owner,"role":"capture_closure"},
        {"id":input,"kind":"function","name":"click","start":11,"end":20,"line":2,"sha256":hash,"parent":null,"role":"excluded_application_or_input"}
    ]);
    let census = json!({"layout_version":1,"family":"fixture","fixed_commit":"2".repeat(40),"original_bytes_preserved":true,
        "compiler_preprocessed":false,"function_exhaustiveness_accepted":false,"scope":"syntax fixture only",
        "sources":[{"path":"source.cpp","artifact":artifact,"git_blob":"3".repeat(40),"namespace_markers":[],"parse_has_error":false,"functions":source_functions}],
        "runtime_groups":[{"reference_root":"runs/context","manifest":artifact,"check_prefix":"capture"}]});
    (map, census)
}

fn check(map: Value, census: Value) -> Result<usize> {
    graph(
        &serde_json::from_value(map)?,
        &serde_json::from_value(census)?,
    )
}

#[test]
fn rejects_missing_reclassified_or_falsely_accepted_function_correspondence() {
    let (map, census) = fixture();
    assert_eq!(check(map.clone(), census.clone()).unwrap(), 1);
    let mut missing = map.clone();
    missing["functions"].as_array_mut().unwrap().pop();
    assert!(check(missing, census.clone()).is_err());
    let mut duplicate = map.clone();
    duplicate["functions"]
        .as_array_mut()
        .unwrap()
        .push(map["functions"][0].clone());
    assert!(check(duplicate, census.clone()).is_err());
    let mut excluded = map.clone();
    excluded["functions"][0]["role"] = "excluded_application_or_input".into();
    assert!(check(excluded, census.clone()).is_err());
    let mut accepted = map.clone();
    accepted["functions"][0]["semantic_equivalence_accepted"] = true.into();
    assert!(check(accepted, census.clone()).is_err());
    let mut parent = census.clone();
    parent["sources"][0]["functions"][1]["parent"] =
        parent["sources"][0]["functions"][2]["id"].clone();
    assert!(check(map.clone(), parent).is_err());
    let mut orphan = census.clone();
    orphan["sources"][0]["functions"][1]["parent"] = Value::Null;
    assert!(check(map.clone(), orphan).is_err());
    let mut exhaustive = census.clone();
    exhaustive["function_exhaustiveness_accepted"] = true.into();
    assert!(check(map.clone(), exhaustive).is_err());
    let mut native = map;
    native["native_acceptance"] = true.into();
    assert!(check(native, census).is_err());
}

#[test]
fn accepts_declaration_only_files_beside_reviewed_bodies_but_rejects_empty_work() {
    let (map, mut census) = fixture();
    let mut header = census["sources"][0].clone();
    header["path"] = "source.h".into();
    header["functions"] = json!([]);
    census["sources"]
        .as_array_mut()
        .unwrap()
        .push(header.clone());
    assert_eq!(check(map.clone(), census.clone()).unwrap(), 1);
    let mut empty = map;
    empty["functions"] = json!([]);
    census["sources"] = json!([header]);
    assert!(check(empty, census).is_err());
}

#[test]
fn rejects_wrong_source_line_hash_and_out_of_bounds_spans() {
    let data = b"alpha\nbeta\n";
    let beta = "f44e64e75f3948e9f73f8dfa94721c4ce8cbb4f265c4790c702b2d41cfbf2753";
    assert!(span(data, 6, 10, 2, beta).is_ok());
    assert!(span(data, 6, 10, 1, beta).is_err());
    assert!(span(data, 7, 10, 2, beta).is_err());
    assert!(span(data, 6, 6, 2, beta).is_err());
    assert!(span(data, 10, 6, 2, beta).is_err());
    assert!(span(data, 6, u64::MAX, 2, beta).is_err());
    for path in [
        "../source.rs",
        "source\\file.rs",
        "source:stream",
        "/source.rs",
    ] {
        assert!(git_path(path).is_err());
    }
}
