//! Acceptance harness tests: the volume ratchet (BR2.1-BR2.5, NFR9.15).
//! Each refusal has a base/new pair; changes that cannot weaken a suite
//! pass with a notice.

mod common;

use rail_testkit::harness::ratchet::{RatchetReport, check_ratchet, compare_settings};
use rail_testkit::harness::settings::Settings;

use common::valid_text;

/// Replaces `from` with `to` inside one suite's entry.
fn edit(text: &str, id: &str, from: &str, to: &str) -> String {
    let start = text
        .find(&format!("[[suites]]\nid = \"{id}\""))
        .expect("suite entry");
    let end = text[start + 1..]
        .find("[[suites]]")
        .map_or(text.len(), |n| start + 1 + n);
    let body = &text[start..end];
    assert!(body.contains(from), "{id} lacks {from:?}");
    format!(
        "{}{}{}",
        &text[..start],
        body.replacen(from, to, 1),
        &text[end..]
    )
}

/// The valid fixture with S0 using a recorded seed, so seed rules apply.
fn seeded() -> String {
    edit(
        &valid_text(),
        "S0",
        "uses_seed = false,",
        "uses_seed = true, pr_seed = 5,",
    )
}

fn ratchet(base: &str, new: &str) -> RatchetReport {
    check_ratchet(Some(base), "base.toml", new, "new.toml").expect("both files are valid")
}

fn refused(base: &str, new: &str, words: &[&str]) {
    let report = ratchet(base, new);
    assert!(!report.passed(), "expected a refusal: {report:?}");
    let all = report.failures.join("\n");
    for word in words {
        assert!(all.contains(word), "{word:?} not in {all}");
    }
}

fn passes_with_notice(base: &str, new: &str, words: &[&str]) {
    let report = ratchet(base, new);
    assert!(report.passed(), "expected a pass: {report:?}");
    let all = report.notices.join("\n");
    for word in words {
        assert!(all.contains(word), "{word:?} not in notices {all}");
    }
}

#[test]
fn unchanged_settings_pass_without_notices() {
    let report = ratchet(&valid_text(), &valid_text());
    assert!(report.passed() && report.notices.is_empty(), "{report:?}");
}

#[test]
fn br2_1_a_lowered_volume_is_refused() {
    let base = valid_text();
    let count = |to: &str| edit(&base, "T1", "count = 1000000", to);
    refused(
        &base,
        &count("count = 999999"),
        &["T1", "volumes.full[0]", "count 1000000", "count 999999"],
    );
    refused(
        &base,
        &edit(
            &base,
            "T2",
            "{ kind = \"exhaustive\", over = \"every §2.7 rule (variants per rule set by U3)\" }",
            "{ kind = \"count\", count = 5 }",
        ),
        &["T2", "volumes.full[0]", "exhaustive", "count 5"],
    );
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "pr = [{ kind = \"count\", count = 1 }]",
            "pr = [{ kind = \"not_run\", reason = \"slow\" }]",
        ),
        &["S0", "volumes.pr[0]", "not_run"],
    );
}

#[test]
fn br2_4_a_removed_entry_or_changed_measure_or_set_is_refused() {
    let base = valid_text();
    refused(
        &base,
        &edit(&base, "T4", ", { kind = \"count\", count = 100000 }", ""),
        &["T4", "volumes.full[1]", "removed"],
    );
    refused(
        &base,
        &edit(
            &base,
            "T1",
            "unit_of_measure = \"generated modules\"",
            "unit_of_measure = \"modules\"",
        ),
        &["T1", "unit_of_measure", "generated modules"],
    );
    refused(
        &base,
        &edit(
            &base,
            "T2",
            "over = \"every §2.7 rule",
            "over = \"most §2.7 rules",
        ),
        &["T2", "volumes.full[0]", "over"],
    );
}

#[test]
fn br2_4_a_removed_or_replaced_input_or_platform_is_refused() {
    let base = valid_text();
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "  { kind = \"fixed\", path = \"crates/rail/tests/golden\" },\n",
            "",
        ),
        &["S0", "inputs", "crates/rail/tests/golden"],
    );
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "path = \"fixtures\"",
            "path = \"fixtures/skeleton\"",
        ),
        &["S0", "inputs", "\"fixtures\""],
    );
    refused(
        &base,
        &edit(
            &base,
            "T2",
            "platforms = [\"macos\", \"linux\"]",
            "platforms = [\"macos\"]",
        ),
        &["T2", "platforms", "linux"],
    );
}

#[test]
fn br2_4_build_deadline_cross_platform_driver_and_seed_use_never_narrow() {
    let base = seeded();
    refused(
        &base,
        &edit(&base, "S0", "build = \"debug\"", "build = \"release\""),
        &["S0", "build", "debug", "release"],
    );
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "process_deadline_seconds = 60",
            "process_deadline_seconds = 30",
        ),
        &["S0", "process_deadline_seconds", "60", "30"],
    );
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "drivers = [\"cli\", \"rap\"]",
            "drivers = [\"cli\"]",
        ),
        &["S0", "drivers", "rap"],
    );
    refused(
        &base,
        &edit(
            &base,
            "S0",
            "uses_seed = true, pr_seed = 5,",
            "uses_seed = false,",
        ),
        &["S0", "uses_seed"],
    );
    let plain = valid_text();
    refused(
        &plain,
        &edit(
            &plain,
            "T1",
            "cross_platform = true",
            "cross_platform = false",
        ),
        &["T1", "cross_platform"],
    );
}

#[test]
fn br2_2_a_suite_can_never_go_back_to_pending_or_disappear() {
    let base = valid_text();
    let pending = edit(
        &edit(
            &base,
            "S0",
            "status = \"implemented\"",
            "status = \"pending\"",
        ),
        "S0",
        "pr = [{ kind = \"count\", count = 1 }]\nnightly = [{ kind = \"count\", count = 1 }]\n",
        "",
    );
    refused(&base, &pending, &["S0", "implemented", "pending"]);

    let base = Settings::parse(&base, "base.toml").unwrap();
    let mut new = base.clone();
    new.suites.remove(5);
    let report = compare_settings(&base, &new);
    assert!(!report.passed());
    assert!(report.failures.join("\n").contains("T5"), "{report:?}");
}

#[test]
fn br2_5_differing_schema_versions_fail_and_a_missing_base_starts_the_ratchet() {
    let base = valid_text().replacen("schema_version = 1", "schema_version = 2", 1);
    let report = check_ratchet(Some(&base), "base.toml", &valid_text(), "new.toml").unwrap();
    assert!(!report.passed());
    assert!(report.failures.join("\n").contains("schema"), "{report:?}");

    let report = check_ratchet(None, "base.toml", &valid_text(), "new.toml").unwrap();
    assert!(report.passed());
    assert!(
        report.notices.join("\n").contains("no base settings"),
        "{report:?}"
    );
}

#[test]
fn an_invalid_new_file_is_a_settings_error() {
    let new = valid_text().replacen("pr_minutes = 10", "pr_minutes = 0", 1);
    assert!(check_ratchet(Some(&valid_text()), "base.toml", &new, "new.toml").is_err());
}

#[test]
fn br2_3_raises_additions_seed_budget_group_and_pending_build_changes_pass_with_notices() {
    let base = valid_text();
    passes_with_notice(
        &base,
        &edit(&base, "T1", "count = 1000000", "count = 2000000"),
        &["T1", "raised"],
    );
    passes_with_notice(
        &base,
        &edit(
            &base,
            "T1",
            "full = [{ kind = \"count\", count = 1000000 }]",
            "full = [{ kind = \"count\", count = 1000000 }, { kind = \"exhaustive\", over = \"the corpus\" }]",
        ),
        &["T1", "volumes.full[1]", "added"],
    );
    passes_with_notice(
        &base,
        &edit(
            &base,
            "S0",
            "  { kind = \"fixed\", path = \"crates/rail/tests/golden\" },\n",
            "  { kind = \"fixed\", path = \"crates/rail/tests/golden\" },\n  { kind = \"fixed\", path = \"scripts\" },\n",
        ),
        &["S0", "inputs", "scripts"],
    );
    passes_with_notice(
        &base,
        &edit(
            &base,
            "T2",
            "platforms = [\"macos\", \"linux\"]",
            "platforms = [\"linux\", \"macos\"]",
        ),
        &[],
    );
    passes_with_notice(
        &base,
        &base.replacen("pr_minutes = 10", "pr_minutes = 12", 1),
        &["budgets", "pr_minutes"],
    );
    passes_with_notice(
        &base,
        &edit(
            &base,
            "S0",
            "nightly_group = \"smoke\"",
            "nightly_group = \"fast\"",
        ),
        &["S0", "nightly_group"],
    );
    passes_with_notice(
        &base,
        &edit(&base, "T2", "build = \"debug\"", "build = \"release\""),
        &["T2", "build"],
    );
    let seeded = seeded();
    passes_with_notice(
        &seeded,
        &edit(&seeded, "S0", "pr_seed = 5", "pr_seed = 6"),
        &["S0", "pr_seed", "5", "6"],
    );
}
