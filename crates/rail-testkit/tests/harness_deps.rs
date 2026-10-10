//! Acceptance harness tests: dependency isolation (BR5.1, NFR7.7). The
//! harness's crates (`toml`, `serde`) must never reach the `rail` binary,
//! and `rail-testkit` must never depend on the compiler it tests.

use rail_testkit::harness::build::{
    cargo_metadata, has_dev_dependency, normal_dependency_path, parse_metadata,
};
use rail_testkit::repo_root;

/// A tiny `cargo metadata` document: `app -> lib` (normal), `app -> kit`
/// (dev), `kit -> lib` (normal).
fn synthetic(app_to_kit_kind: &str) -> Vec<u8> {
    let node = |id: &str, deps: &str| format!(r#"{{"id":"{id}","deps":[{deps}]}}"#);
    let dep = |name: &str, kind: &str| {
        format!(
            r#"{{"name":"{name}","pkg":"{name} 0.1.0","dep_kinds":[{{"kind":{kind},"target":null}}]}}"#
        )
    };
    format!(
        r#"{{"packages":[{{"id":"app 0.1.0","name":"app"}},{{"id":"lib 0.1.0","name":"lib"}},{{"id":"kit 0.1.0","name":"kit"}}],"resolve":{{"nodes":[{},{},{}],"root":null}}}}"#,
        node(
            "app 0.1.0",
            &format!("{},{}", dep("lib", "null"), dep("kit", app_to_kit_kind))
        ),
        node("lib 0.1.0", ""),
        node("kit 0.1.0", &dep("lib", "null")),
    )
    .into_bytes()
}

#[test]
fn a_dev_edge_is_not_walked_but_is_found() {
    let meta = parse_metadata(&synthetic(r#""dev""#)).unwrap();
    assert_eq!(normal_dependency_path(&meta, "app", "kit"), None);
    assert!(has_dev_dependency(&meta, "app", "kit"));
    assert_eq!(
        normal_dependency_path(&meta, "app", "lib"),
        Some(vec!["app".to_string(), "lib".to_string()])
    );
}

#[test]
fn a_normal_edge_to_the_testkit_is_found() {
    let meta = parse_metadata(&synthetic("null")).unwrap();
    assert_eq!(
        normal_dependency_path(&meta, "app", "kit"),
        Some(vec!["app".to_string(), "kit".to_string()])
    );
    assert!(!has_dev_dependency(&meta, "app", "kit"));
}

#[test]
fn malformed_metadata_is_an_error_not_a_panic() {
    assert!(parse_metadata(b"{").is_err());
    assert!(parse_metadata(br#"{"packages":[]}"#).is_err());
}

#[test]
fn nfr7_7_rail_never_reaches_the_testkit_through_normal_dependencies() {
    let json = cargo_metadata(&repo_root()).unwrap();
    let meta = parse_metadata(&json).unwrap();
    assert_eq!(normal_dependency_path(&meta, "rail", "rail-testkit"), None);
    assert_eq!(normal_dependency_path(&meta, "rail", "toml"), None);
    // The walk is not vacuous: rail does use the testkit for its tests, and
    // does reach its own crates.
    assert!(has_dev_dependency(&meta, "rail", "rail-testkit"));
    assert!(normal_dependency_path(&meta, "rail", "rail-json").is_some());
}

#[test]
fn br5_1_the_testkit_has_no_compiler_crate_as_a_normal_dependency() {
    let text = std::fs::read_to_string(repo_root().join("crates/rail-testkit/Cargo.toml")).unwrap();
    let manifest: toml::Table = toml::from_str(&text).unwrap();
    let deps = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .cloned()
        .unwrap_or_default();
    let compiler = [
        "rail",
        "rail-syntax",
        "rail-check",
        "rail-lower",
        "rail-codegen",
        "rail-build",
        "rail-runtime",
        "rail-rap",
        "rail-tools",
        "rail-diag",
    ];
    let found: Vec<&str> = compiler
        .iter()
        .copied()
        .filter(|c| deps.contains_key(*c))
        .collect();
    assert!(
        found.is_empty(),
        "compiler crates in rail-testkit [dependencies]: {found:?}"
    );
    assert!(
        deps.contains_key("rail-json"),
        "rail-json is the allowed data-format crate"
    );
}
