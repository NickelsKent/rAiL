//! Diagnostic shape (C3), deterministic order and the fixed rule registry
//! (BR4.1, BR4.3, BR4.4).

use rail_diag::{
    CheckResult, Diagnostic, FX001, Location, Registry, RegistryError, RuleInfo, SKL001, Span,
    TY001,
};

fn diag(rule: rail_diag::RuleId, def: &str, path: &str, span: (usize, usize)) -> Diagnostic {
    Diagnostic::error(
        rule,
        Location {
            module: "skeleton.broken".to_string(),
            def: def.to_string(),
            path: path.to_string(),
            span: Span::new(span.0, span.1),
        },
        "expected i64, found bool",
    )
}

#[test]
fn diagnostic_json_has_the_c3_shape() {
    let d = diag(TY001, "#sk0a1b", "4", (82, 90));
    assert_eq!(
        rail_json::to_string(&d.to_json()),
        r##"{"rule":"TY001","level":"E","loc":{"module":"skeleton.broken","def":"#sk0a1b","path":"4","span":[82,90]},"message":"expected i64, found bool","confidence":1.0}"##
    );
}

#[test]
fn diagnostic_renders_one_human_line() {
    let d = diag(TY001, "#sk0a1b", "4", (82, 90));
    assert_eq!(
        d.render(),
        "error TY001 skeleton.broken #sk0a1b/4 [82,90]: expected i64, found bool"
    );
}

#[test]
fn check_result_sorts_by_module_def_path_span_rule() {
    let result = CheckResult::new(
        "skeleton.broken",
        vec![
            diag(TY001, "#sk0m4n", "4", (1, 2)),
            diag(TY001, "#sk0a1b", "4.1", (5, 6)),
            diag(SKL001, "#sk0a1b", "4", (9, 10)),
            diag(FX001, "#sk0a1b", "4", (3, 4)),
            diag(TY001, "#sk0a1b", "4", (3, 4)),
        ],
    );
    let order: Vec<(String, String, (usize, usize), &str)> = result
        .diagnostics
        .iter()
        .map(|d| {
            (
                d.loc.def.clone(),
                d.loc.path.clone(),
                (d.loc.span.start, d.loc.span.end),
                d.rule.as_str(),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("#sk0a1b".to_string(), "4".to_string(), (3, 4), "FX001"),
            ("#sk0a1b".to_string(), "4".to_string(), (3, 4), "TY001"),
            ("#sk0a1b".to_string(), "4".to_string(), (9, 10), "SKL001"),
            ("#sk0a1b".to_string(), "4.1".to_string(), (5, 6), "TY001"),
            ("#sk0m4n".to_string(), "4".to_string(), (1, 2), "TY001"),
        ]
    );
    assert!(result.blocking);
}

#[test]
fn check_result_blocks_exactly_when_a_diagnostic_exists() {
    let clean = CheckResult::new("skeleton.answer", Vec::new());
    assert!(!clean.blocking);
    assert_eq!(
        rail_json::to_string(&clean.to_json()),
        r#"{"module":"skeleton.answer","diagnostics":[],"blocking":false}"#
    );
    let blocked = CheckResult::new("m", vec![diag(SKL001, "#000000", "", (0, 1))]);
    assert!(blocked.blocking);
}

#[test]
fn registry_holds_exactly_the_three_skeleton_rules() {
    let registry = Registry::skeleton();
    assert_eq!(registry.ids(), ["FX001", "SKL001", "TY001"]);
    for id in [TY001, FX001, SKL001] {
        assert!(registry.get(id.as_str()).is_some());
    }
    assert!(registry.get("FMT001").is_none());
}

#[test]
fn registering_skl001_twice_fails() {
    let mut registry = Registry::skeleton();
    let err = registry
        .register(RuleInfo {
            id: SKL001,
            summary: "reused",
        })
        .unwrap_err();
    assert_eq!(err, RegistryError::AlreadyRegistered("SKL001"));
    assert_eq!(registry.ids().len(), 3);
}

#[test]
fn rule_ids_follow_the_c3_pattern() {
    assert!(RuleInfo::valid_id("TY001"));
    assert!(RuleInfo::valid_id("SKL001"));
    assert!(!RuleInfo::valid_id("ty001"));
    assert!(!RuleInfo::valid_id("TOOLONG001"));
    assert!(!RuleInfo::valid_id("TY01"));
    let mut registry = Registry::skeleton();
    assert_eq!(
        registry.register(RuleInfo {
            id: rail_diag::RuleId::new("bad"),
            summary: "x"
        }),
        Err(RegistryError::InvalidId("bad"))
    );
}
