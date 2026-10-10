//! Acceptance harness tests: reading and validating the settings file
//! (BR1.1-BR1.7, BR3.6, BR6.1, NFR9.17).

mod common;

use rail_testkit::harness::settings::{
    Build, Level, Platform, Settings, SettingsError, Status, SuiteId, Volume,
};
use rail_testkit::repo_root;
use rail_testkit::suites::Registry;

use common::{FakeSuite, data, valid_text};

fn parse(text: &str) -> Result<Settings, SettingsError> {
    Settings::parse(text, "suites.toml")
}

/// The text of one suite's entry in the valid fixture.
fn entry(text: &str, id: &str) -> (usize, usize) {
    let start = text
        .find(&format!("[[suites]]\nid = \"{id}\""))
        .expect("suite entry");
    let end = text[start + 1..]
        .find("[[suites]]")
        .map_or(text.len(), |n| start + 1 + n);
    (start, end)
}

fn edit_suite(id: &str, from: &str, to: &str) -> Result<Settings, SettingsError> {
    let text = valid_text();
    let (start, end) = entry(&text, id);
    let body = &text[start..end];
    assert!(body.contains(from), "{id} lacks {from:?}");
    let edited = format!(
        "{}{}{}",
        &text[..start],
        body.replacen(from, to, 1),
        &text[end..]
    );
    parse(&edited)
}

fn err(result: Result<Settings, SettingsError>) -> SettingsError {
    match result {
        Ok(_) => panic!("expected a settings error"),
        Err(e) => e,
    }
}

fn expect_error(result: Result<Settings, SettingsError>, key: &str, words: &str) {
    let e = err(result);
    let text = e.to_string();
    assert!(e.key.contains(key), "key {:?} lacks {key:?}: {text}", e.key);
    assert!(text.contains(words), "{text:?} lacks {words:?}");
    assert!(text.starts_with("suites.toml"), "{text}");
}

#[test]
fn the_valid_file_with_all_fifteen_suites_loads() {
    let settings = parse(&valid_text()).expect("valid settings");
    assert_eq!(settings.schema_version, 1);
    let ids: Vec<String> = settings.suites.iter().map(|s| s.id().to_string()).collect();
    let expected: Vec<String> = SuiteId::ALL.iter().map(|s| s.to_string()).collect();
    assert_eq!(ids, expected);
    let s0 = settings.suite(SuiteId::S0);
    assert_eq!(s0.status, Status::Implemented);
    assert_eq!(s0.platforms, vec![Platform::Macos, Platform::Linux]);
    assert_eq!(
        s0.volumes.for_level(Level::Pr),
        Some(&[Volume::Count { count: 1 }][..])
    );
    assert_eq!(s0.nightly_group.as_deref(), Some("smoke"));
    let t13 = settings.suite(SuiteId::parse("T13").unwrap());
    assert_eq!(t13.build, Build::Release);
    assert_eq!(t13.status, Status::Pending);
    assert_eq!(t13.volumes.for_level(Level::Pr), None);
    assert_eq!(settings.budgets.pr_minutes, 10);
    assert_eq!(settings.budgets.nightly_group_minutes, 300);
}

#[test]
fn br1_1_an_unknown_key_is_refused_with_file_line_and_key() {
    let text = std::fs::read_to_string(data("unknown-key.toml")).unwrap();
    let e = err(Settings::parse(&text, "unknown-key.toml"));
    assert_eq!(e.line, Some(20), "{e}");
    assert!(e.to_string().contains("colour"), "{e}");
    assert!(e.to_string().starts_with("unknown-key.toml:20:"), "{e}");
}

#[test]
fn br1_1_a_wrong_schema_version_or_non_toml_is_refused() {
    let text = std::fs::read_to_string(data("bad-schema.toml")).unwrap();
    let e = err(Settings::parse(&text, "bad-schema.toml"));
    assert_eq!(e.key, "schema_version", "{e}");
    let text = std::fs::read_to_string(data("not-toml.toml")).unwrap();
    let e = err(Settings::parse(&text, "not-toml.toml"));
    assert_eq!(e.line, Some(2), "{e}");
}

#[test]
fn br1_2_a_missing_duplicated_or_misordered_suite_is_refused() {
    let text = valid_text();
    let (start, end) = entry(&text, "T5");
    let missing = format!("{}{}", &text[..start], &text[end..]);
    expect_error(parse(&missing), "suites", "T5");

    let t5 = &text[start..end];
    let duplicated = format!("{}{t5}{}", &text[..end], &text[end..]);
    expect_error(parse(&duplicated), "suites", "T5");

    let (s4, e4) = entry(&text, "T4");
    let swapped = format!(
        "{}{}{}{}",
        &text[..s4],
        &text[start..end],
        &text[s4..e4],
        &text[end..]
    );
    expect_error(parse(&swapped), "suites", "T5");
}

#[test]
fn br1_3_a_missing_or_not_run_full_volume_is_refused() {
    expect_error(
        edit_suite(
            "T5",
            "full = [{ kind = \"count\", count = 10000 }]",
            "full = []",
        ),
        "volumes.full",
        "T5",
    );
    expect_error(
        edit_suite(
            "T5",
            "full = [{ kind = \"count\", count = 10000 }]",
            "full = [{ kind = \"not_run\", reason = \"later\" }]",
        ),
        "volumes.full",
        "not_run",
    );
}

#[test]
fn br1_4_pending_suites_have_only_full_volumes_and_implemented_suites_all_three() {
    expect_error(
        edit_suite(
            "T5",
            "unit_of_measure = \"single-definition patches\"",
            "unit_of_measure = \"single-definition patches\"\npr = [{ kind = \"count\", count = 1 }]",
        ),
        "volumes.pr",
        "pending",
    );
    expect_error(
        edit_suite("T5", "uses_seed = true,", "uses_seed = true, pr_seed = 7,"),
        "pr_seed",
        "pending",
    );
    expect_error(
        edit_suite("S0", "pr = [{ kind = \"count\", count = 1 }]\n", ""),
        "volumes.pr",
        "implemented",
    );
    expect_error(
        edit_suite("S0", "uses_seed = false", "uses_seed = true"),
        "pr_seed",
        "S0",
    );
    let text = valid_text();
    let (start, end) = entry(&text, "S0");
    let body = &text[start..end];
    let inputs_start = body.find("inputs = [").unwrap();
    let inputs_end = body[inputs_start..].find("]\n").unwrap() + inputs_start + 2;
    let edited = format!(
        "{}{}inputs = []\n{}{}",
        &text[..start],
        &body[..inputs_start],
        &body[inputs_end..],
        &text[end..]
    );
    expect_error(parse(&edited), "inputs", "S0");
}

#[test]
fn br1_5_volumes_never_shrink_from_level_to_level() {
    expect_error(
        edit_suite(
            "S0",
            "pr = [{ kind = \"count\", count = 1 }]",
            "pr = [{ kind = \"count\", count = 2 }]",
        ),
        "volumes",
        "pr",
    );
    expect_error(
        edit_suite(
            "S0",
            "nightly = [{ kind = \"count\", count = 1 }]",
            "nightly = [{ kind = \"exhaustive\", over = \"all\" }]",
        ),
        "volumes",
        "nightly",
    );
    // not_run < count: a not_run pull-request volume is fine.
    let ok = edit_suite(
        "S0",
        "pr = [{ kind = \"count\", count = 1 }]",
        "pr = [{ kind = \"not_run\", reason = \"too slow for a pull request\" }]",
    );
    assert!(ok.is_ok(), "{:?}", ok.err());
}

#[test]
fn br1_6_input_paths_stay_inside_the_repository() {
    expect_error(
        edit_suite("S0", "path = \"fixtures\"", "path = \"/etc\""),
        "inputs",
        "absolute",
    );
    expect_error(
        edit_suite("S0", "path = \"fixtures\"", "path = \"fixtures/../..\""),
        "inputs",
        "..",
    );
}

#[test]
fn br1_6_missing_paths_and_unknown_generators_are_refused_against_the_repository() {
    let registry = Registry::production();
    let missing = edit_suite("S0", "path = \"fixtures\"", "path = \"fixtures/nowhere\"").unwrap();
    let e = missing
        .check_environment(&repo_root(), &registry)
        .unwrap_err();
    assert!(e.to_string().contains("fixtures/nowhere"), "{e}");

    let generator = edit_suite(
        "S0",
        "{ kind = \"fixed\", path = \"fixtures\" }",
        "{ kind = \"generator\", generator = \"no-such-generator\" }",
    )
    .unwrap();
    let e = generator
        .check_environment(&repo_root(), &registry)
        .unwrap_err();
    assert!(e.to_string().contains("no-such-generator"), "{e}");
}

#[test]
fn br1_7_only_macos_and_linux_are_platforms() {
    let e = err(edit_suite(
        "T2",
        "platforms = [\"macos\", \"linux\"]",
        "platforms = [\"macos\", \"windows\"]",
    ));
    assert!(e.to_string().contains("windows"), "{e}");
    expect_error(
        edit_suite("T2", "platforms = [\"macos\", \"linux\"]", "platforms = []"),
        "platforms",
        "T2",
    );
}

#[test]
fn br3_6_t13_and_t14_must_use_the_release_build() {
    for id in ["T13", "T14"] {
        expect_error(
            edit_suite(id, "build = \"release\"", "build = \"debug\""),
            "build",
            id,
        );
    }
}

#[test]
fn br3_6_an_implemented_sanitizer_suite_needs_its_rustflags() {
    let text = valid_text();
    let (start, end) = entry(&text, "S0");
    let s0 = text[start..end].replace("build = \"debug\"", "build = \"sanitizer\"");
    let edited = format!("{}{s0}{}", &text[..start], &text[end..]);
    expect_error(parse(&edited), "sanitizer_rustflags", "S0");
    let with_flags = edited.replacen(
        "build = \"sanitizer\"",
        "build = \"sanitizer\"\nsanitizer_rustflags = \"-Zsanitizer=address\"",
        1,
    );
    assert!(parse(&with_flags).is_ok());
}

#[test]
fn nfr9_17_an_implemented_suite_needs_a_nightly_group() {
    expect_error(
        edit_suite("S0", "nightly_group = \"smoke\"\n", ""),
        "nightly_group",
        "S0",
    );
}

#[test]
fn br6_1_settings_and_registered_implementations_must_agree() {
    let settings = parse(&valid_text()).unwrap();
    // S0 is implemented in the settings but nothing is registered.
    let e = settings
        .check_environment(&repo_root(), &Registry::new(Vec::new()))
        .unwrap_err();
    assert!(e.to_string().contains("S0"), "{e}");
    // T1 is registered but the settings say pending.
    let registry = Registry::new(vec![
        Box::new(FakeSuite::passing(SuiteId::S0)),
        Box::new(FakeSuite::passing(SuiteId::parse("T1").unwrap())),
    ]);
    let e = settings
        .check_environment(&repo_root(), &registry)
        .unwrap_err();
    assert!(e.to_string().contains("T1"), "{e}");
    assert!(e.to_string().contains("pending"), "{e}");
    // The production registry agrees with the valid file.
    settings
        .check_environment(&repo_root(), &Registry::production())
        .unwrap();
}
