//! JSONTestSuite `test_parsing` cases (NFR4.3): every `y_` case is accepted,
//! every `n_` case rejected, and no case (including `i_`) panics.

use std::fs;
use std::path::Path;

fn cases(prefix: &str) -> Vec<(String, Vec<u8>)> {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/JSONTestSuite/test_parsing");
    let mut cases: Vec<(String, Vec<u8>)> = fs::read_dir(&dir)
        .expect("vendored JSONTestSuite")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix))
        })
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read(&path).expect("read case"))
        })
        .collect();
    cases.sort();
    cases
}

/// Upstream marks these `y_` (accept), but NFR4.3 requires duplicate object
/// keys to be rejected. They are asserted to fail with `DuplicateKey`, and
/// only with that error.
const DUPLICATE_KEY_CASES: &[&str] = &[
    "y_object_duplicated_key.json",
    "y_object_duplicated_key_and_value.json",
];

#[test]
fn every_y_case_is_accepted_and_round_trips() {
    let cases = cases("y_");
    assert_eq!(cases.len(), 95);
    for (name, bytes) in cases {
        if DUPLICATE_KEY_CASES.contains(&name.as_str()) {
            let err = rail_json::parse(&bytes).expect_err(&name);
            assert_eq!(err.kind, rail_json::ErrorKind::DuplicateKey, "{name}");
            continue;
        }
        let value = rail_json::parse(&bytes).unwrap_or_else(|e| panic!("{name} rejected: {e}"));
        let written = rail_json::to_string(&value);
        let again = rail_json::parse(written.as_bytes())
            .unwrap_or_else(|e| panic!("{name} rewritten: {e}"));
        assert_eq!(value, again, "{name}");
    }
}

#[test]
fn every_n_case_is_rejected() {
    let cases = cases("n_");
    assert_eq!(cases.len(), 188);
    for (name, bytes) in cases {
        assert!(rail_json::parse(&bytes).is_err(), "{name} was accepted");
    }
}

#[test]
fn implementation_defined_cases_finish_without_panicking() {
    let cases = cases("i_");
    assert_eq!(cases.len(), 35);
    for (_, bytes) in cases {
        let _ = rail_json::parse(&bytes);
    }
}
