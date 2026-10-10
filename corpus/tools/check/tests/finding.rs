//! Every finding names its file, program and field (Step 5 tidy-up).

use corpus_check::finding::Finding;

#[test]
fn a_finding_prints_file_program_field_and_message() {
    let f = Finding::new("index.toml", "is missing")
        .program("binary-trees")
        .field("task");
    assert_eq!(
        f.to_string(),
        "index.toml: program binary-trees: field task: is missing"
    );
    assert_eq!(Finding::new("a", "b").to_string(), "a: b");
}
