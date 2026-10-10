//! Data model: the index, composition, layout and definition IDs
//! (BR1.1 to BR1.4, BR3.3).

mod common;

use common::{TempDir, entry_toml, index_toml};
use corpus_check::index::{
    Comparison, InputMode, check_composition, check_layout, check_rail_ids, parse_index,
};
use corpus_check::model::{Class, Composition};

fn small_composition(per_class: usize) -> Composition {
    let mut approved = Vec::new();
    for class in Class::ALL {
        for i in 1..=per_class {
            approved.push((class, format!("{}-p{i}", class.name())));
        }
    }
    Composition {
        per_class,
        approved,
    }
}

#[test]
fn the_approved_list_is_forty_programs_ten_per_class() {
    let comp = Composition::approved();
    assert_eq!(comp.approved.len(), 40);
    assert_eq!(comp.per_class, 10);
    for class in Class::ALL {
        assert_eq!(comp.of_class(class).len(), 10, "{class}");
    }
    assert_eq!(comp.of_class(Class::PureTransform)[0], "binary-trees");
    assert_eq!(comp.of_class(Class::DataParallel)[9], "parallel-blur");
    let mut names: Vec<_> = comp.approved.iter().map(|(_, n)| n.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 40, "program names are unique");
    assert!(names.iter().all(|n| n.len() <= 32), "at most 32 bytes");
}

#[test]
fn parses_every_index_entry_field() {
    let text = format!(
        "schema_version = 1\n\n{}",
        entry_toml("p1", "pure-transform")
    );
    let index = parse_index("index.toml", &text).expect("valid");
    assert_eq!(index.schema_version, 1);
    let e = &index.entries[0];
    assert_eq!(e.program_id, "p1");
    assert_eq!(e.class, Class::PureTransform);
    assert_eq!(e.task, "A task.");
    assert_eq!(e.path, "corpus/pure-transform/p1/");
    assert_eq!(e.args, vec!["{input:data.txt}".to_string()]);
    assert_eq!(e.input_mode, InputMode::Files);
    assert_eq!(e.stdin_from, None);
    assert!(!e.threads_flag);
    assert_eq!(e.generator_outputs, vec!["data.txt".to_string()]);
    assert_eq!(e.size_range, (10, 1000));
    assert_eq!(e.reference_size, 100);
    assert_eq!(e.fixed_inputs, vec!["basic".to_string(), "bad".to_string()]);
    assert_eq!(e.comparison, Comparison::StdoutExact);
    assert!(!e.floating_point);
}

#[test]
fn a_missing_run_data_field_names_the_program_and_the_field() {
    for field in [
        "program_id",
        "class",
        "task",
        "path",
        "args",
        "input_mode",
        "threads_flag",
        "generator_outputs",
        "size_range",
        "reference_size",
        "fixed_inputs",
        "comparison",
        "floating_point",
    ] {
        let entry = entry_toml("p1", "pure-transform");
        let stripped: String = entry
            .lines()
            .filter(|l| !l.starts_with(&format!("{field} =")))
            .map(|l| format!("{l}\n"))
            .collect();
        let findings =
            parse_index("index.toml", &format!("schema_version = 1\n{stripped}")).expect_err(field);
        let text = findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains(field), "{field}: {text}");
        if field != "program_id" {
            assert!(text.contains("p1"), "{field}: {text}");
        }
    }
}

#[test]
fn run_data_must_be_consistent() {
    let base = entry_toml("p1", "pure-transform");
    let cases: Vec<(String, &str)> = vec![
        // stdin mode needs stdin_from
        (
            base.replace("input_mode = \"files\"", "input_mode = \"stdin\""),
            "stdin_from",
        ),
        // threads_flag true exactly for data-parallel
        (
            base.replace("threads_flag = false", "threads_flag = true"),
            "threads_flag",
        ),
        // {threads} placeholder without a threads flag
        (base.replace("{input:data.txt}", "{threads}"), "args"),
        // {input:x} must name a generator output
        (
            base.replace("{input:data.txt}", "{input:other.txt}"),
            "args",
        ),
        // fixed inputs: one to eight
        (base.replace("[\"basic\", \"bad\"]", "[]"), "fixed_inputs"),
        (
            base.replace(
                "[\"basic\", \"bad\"]",
                "[\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\",\"i\"]",
            ),
            "fixed_inputs",
        ),
        // path must be corpus/<class>/<program>/
        (
            base.replace("corpus/pure-transform/p1/", "corpus/elsewhere/p1/"),
            "path",
        ),
        // enums
        (base.replace("\"stdout-exact\"", "\"fuzzy\""), "comparison"),
        (base.replace("\"files\"", "\"pipe\""), "input_mode"),
        (
            base.replace("class = \"pure-transform\"", "class = \"weird\""),
            "class",
        ),
        // size range and reference size
        (base.replace("\"10 1000\"", "\"1000 10\""), "size_range"),
        (base.replace("\"10 1000\"", "\"ten\""), "size_range"),
        (
            base.replace("reference_size = 100", "reference_size = 5000"),
            "reference_size",
        ),
        // program id spelling
        (
            base.replace("program_id = \"p1\"", "program_id = \"Bad_Name\""),
            "program_id",
        ),
        // io-services must compare trees
        (
            entry_toml("p1", "io-services").replace("\"stdout-and-tree\"", "\"stdout-exact\""),
            "comparison",
        ),
    ];
    for (entry, field) in cases {
        let findings =
            parse_index("index.toml", &format!("schema_version = 1\n{entry}")).expect_err(field);
        assert!(
            findings.iter().any(|f| f.field.as_deref() == Some(field)),
            "{field}: {findings:?}"
        );
    }
}

#[test]
fn only_schema_version_one_and_unique_programs_are_accepted() {
    let e = entry_toml("p1", "pure-transform");
    let findings = parse_index("index.toml", &format!("schema_version = 2\n{e}")).unwrap_err();
    assert!(
        findings
            .iter()
            .any(|f| f.field.as_deref() == Some("schema_version"))
    );
    let findings = parse_index("index.toml", &format!("schema_version = 1\n{e}{e}")).unwrap_err();
    assert!(
        findings.iter().any(|f| f.message.contains("twice")),
        "{findings:?}"
    );
    let findings = parse_index("index.toml", "schema_version = 1\n[oops\n").unwrap_err();
    assert!(findings[0].message.contains("line 2"), "{findings:?}");
}

#[test]
fn composition_requires_the_exact_count_for_each_delivered_class() {
    let comp = small_composition(2);
    let all = index_toml(
        2,
        &[
            "pure-transform",
            "parsing-text",
            "io-services",
            "data-parallel",
        ],
    );
    let index = parse_index("index.toml", &all).expect("valid");
    assert!(check_composition("index.toml", &index, &comp, &Class::ALL).is_empty());

    // one program short in a class
    let short = index_toml(2, &["pure-transform"]) + &entry_toml("parsing-text-p1", "parsing-text");
    let index = parse_index("index.toml", &short).expect("valid");
    let findings = check_composition("index.toml", &index, &comp, &Class::ALL[..2]);
    let text: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert!(
        text.iter()
            .any(|t| t.contains("parsing-text") && t.contains('1') && t.contains('2')),
        "{text:?}"
    );
    // missing approved program is named
    assert!(
        text.iter().any(|t| t.contains("parsing-text-p2")),
        "{text:?}"
    );
}

#[test]
fn composition_rejects_unapproved_programs() {
    let comp = small_composition(2);
    let text = index_toml(2, &["pure-transform"]).replace("pure-transform-p2", "sneaky-extra");
    let index = parse_index("index.toml", &text).expect("valid");
    // An unapproved program is refused even in a class still being written.
    let findings = check_composition("index.toml", &index, &comp, &[]);
    assert!(
        findings
            .iter()
            .any(|f| f.to_string().contains("sneaky-extra")),
        "{findings:?}"
    );
    // The missing approved program is named once the class must be complete.
    let findings = check_composition("index.toml", &index, &comp, &[Class::PureTransform]);
    let text: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert!(
        text.iter().any(|t| t.contains("pure-transform-p2")),
        "{text:?}"
    );
}

#[test]
fn a_class_still_being_written_passes_unless_it_must_be_complete() {
    let comp = small_composition(2);
    let index = parse_index("index.toml", &index_toml(1, &["pure-transform"])).unwrap();
    assert!(check_composition("index.toml", &index, &comp, &[]).is_empty());
    let index = parse_index("index.toml", &index_toml(2, &["pure-transform"])).unwrap();
    assert!(check_composition("index.toml", &index, &comp, &[Class::PureTransform]).is_empty());
    let findings = check_composition("index.toml", &index, &comp, &Class::ALL);
    for class in ["parsing-text", "io-services", "data-parallel"] {
        assert!(
            findings.iter().any(|f| f.message.contains(class)),
            "{class}: {findings:?}"
        );
    }
}

fn make_program(corpus: &TempDir, class: &str, program: &str, skip: Option<&str>) {
    let base = format!("{class}/{program}");
    for dir in [
        "rail",
        "rust/src",
        "go",
        "ocaml/bin",
        "inputs/gen",
        "inputs/fixed",
        "expected",
    ] {
        corpus.mkdir(&format!("{base}/{dir}"));
    }
    for file in [
        "prompt.md",
        "fairness-review.md",
        "rail/main.rlc",
        "rust/Cargo.toml",
        "rust/src/main.rs",
        "go/go.mod",
        "go/main.go",
        "ocaml/dune-project",
        "ocaml/bin/main.ml",
    ] {
        corpus.write(&format!("{base}/{file}"), "x");
    }
    if let Some(skip) = skip {
        let p = corpus.path().join(&base).join(skip);
        if p.is_dir() {
            std::fs::remove_dir_all(p).unwrap();
        } else {
            std::fs::remove_file(p).unwrap();
        }
    }
}

#[test]
fn layout_passes_for_a_complete_program() {
    let corpus = TempDir::new("layout-ok");
    make_program(&corpus, "pure-transform", "pure-transform-p1", None);
    let index = parse_index("index.toml", &index_toml(1, &["pure-transform"])).unwrap();
    assert!(check_layout(corpus.path(), &index).is_empty());
}

#[test]
fn layout_names_each_missing_part() {
    for missing in [
        "prompt.md",
        "fairness-review.md",
        "rail",
        "rust",
        "go",
        "ocaml",
        "inputs/gen",
        "inputs/fixed",
        "expected",
        "rail/main.rlc",
        "rust/Cargo.toml",
        "go/go.mod",
        "ocaml/bin/main.ml",
    ] {
        let corpus = TempDir::new("layout-missing");
        make_program(
            &corpus,
            "pure-transform",
            "pure-transform-p1",
            Some(missing),
        );
        let index = parse_index("index.toml", &index_toml(1, &["pure-transform"])).unwrap();
        let findings = check_layout(corpus.path(), &index);
        assert!(
            findings
                .iter()
                .any(|f| f.program.as_deref() == Some("pure-transform-p1")
                    && f.message.contains(missing)),
            "{missing}: {findings:?}"
        );
    }
}

#[test]
fn definition_ids_must_be_well_formed_and_unique() {
    let good = "(mod m)\n(fn #sk0a1b answer (x) (* x 7))\n(fn #sk0m4n main (_) (answer 6))\n";
    assert!(check_rail_ids("main.rlc", "p1", good).is_empty());

    for (bad, why) in [
        ("(fn #SK0A1B f () 1)\n", "uppercase"),
        ("(fn #sk0a1 f () 1)\n", "five characters"),
        ("(fn #sk0a1bc f () 1)\n", "seven characters"),
        ("(fn #sk0u1b f () 1)\n", "letter u is not Crockford base32"),
        ("(fn #sk0i1b f () 1)\n", "letter i is not Crockford base32"),
        ("(fn #sk0l1b f () 1)\n", "letter l is not Crockford base32"),
        ("(fn #sk0o1b f () 1)\n", "letter o is not Crockford base32"),
    ] {
        let findings = check_rail_ids("main.rlc", "p1", bad);
        assert!(!findings.is_empty(), "{why}");
        assert_eq!(findings[0].program.as_deref(), Some("p1"));
    }

    let dup = "(fn #sk0a1b a () 1)\n(fn #sk0a1b b () 2)\n";
    let findings = check_rail_ids("main.rlc", "p1", dup);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].message.contains("#sk0a1b"), "{findings:?}");
}

#[test]
fn entries_must_be_ordered_by_class_then_program_id() {
    let a = entry_toml("alpha", "pure-transform");
    let b = entry_toml("beta", "pure-transform");
    let c = entry_toml("gamma", "parsing-text");
    let ok = format!("schema_version = 1\n{a}{b}{c}");
    assert!(parse_index("index.toml", &ok).is_ok());
    for bad in [
        format!("schema_version = 1\n{b}{a}{c}"),
        format!("schema_version = 1\n{c}{a}{b}"),
    ] {
        let findings = parse_index("index.toml", &bad).unwrap_err();
        assert!(
            findings.iter().any(|f| f.message.contains("order")),
            "{findings:?}"
        );
    }
}
