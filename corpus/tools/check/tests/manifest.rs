//! Library policy and lockfiles (BR5.1, BR5.2, BR5.5).

mod common;

use common::{TempDir, entry_toml};
use corpus_check::index::{IndexEntry, parse_index};
use corpus_check::manifest::check_manifests;
use corpus_check::pins::{Pins, parse_pins};

const PINS: &str = r#"schema_version = 1
[toolchains]
rust = "1.98.1"
go = "1.27.2"
ocaml = "5.5.1"
dune = "3.24.2"
opam = "2.6.1"
[sandbox]
bubblewrap_min = "0.6.0"
[[libraries]]
language = "rust"
name = "rayon"
version = "1.2.3"
checksum = "sha256:ab"
vendored_path = "corpus/vendor/rust"
reason = "approved"
allowed_class = "data-parallel"
[[libraries]]
language = "ocaml"
name = "domainslib"
version = "0.5.1"
checksum = "sha256:cd"
vendored_path = "corpus/vendor/ocaml"
reason = "approved"
allowed_class = "data-parallel"
"#;

fn pins() -> Pins {
    parse_pins("toolchains.toml", PINS).unwrap()
}

fn entry(class: &str) -> IndexEntry {
    let mut text = format!("schema_version = 1\n{}", entry_toml("p1", class));
    if class == "data-parallel" {
        text = text.replace(
            "args = [\"{input:data.txt}\"]",
            "args = [\"--threads\", \"{threads}\"]",
        );
    }
    parse_index("index.toml", &text).unwrap().entries.remove(0)
}

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"p1\"\nversion = \"0.1.0\"\n";
const CARGO: &str = "[package]\nname = \"p1\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A clean program: manifests with no dependencies and committed lockfiles.
fn clean(dir: &TempDir) {
    dir.write("rust/Cargo.toml", CARGO);
    dir.write("rust/Cargo.lock", LOCK);
    dir.write("inputs/gen/Cargo.toml", CARGO);
    dir.write("inputs/gen/Cargo.lock", LOCK);
    dir.write("go/go.mod", "module p1\n\ngo 1.27\n");
    dir.write(
        "ocaml/dune-project",
        "(lang dune 3.24)\n(package (name p1) (depends (ocaml (>= 5.5)) dune))\n",
    );
    dir.write(
        "ocaml/bin/dune",
        "(executable (name main) (libraries unix threads.posix str))\n",
    );
}

fn text(findings: &[corpus_check::finding::Finding]) -> String {
    findings
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_clean_program_passes() {
    let dir = TempDir::new("manifest-clean");
    clean(&dir);
    let findings = check_manifests(dir.path(), &entry("pure-transform"), &pins());
    assert!(findings.is_empty(), "{}", text(&findings));
}

#[test]
fn a_rust_dependency_is_refused_in_every_dependency_table() {
    for table in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "target.cfg-unix.dependencies",
    ] {
        let dir = TempDir::new("manifest-rust");
        clean(&dir);
        dir.write(
            "rust/Cargo.toml",
            format!("{CARGO}\n[{table}]\nserde = \"1\"\n"),
        );
        let findings = check_manifests(dir.path(), &entry("pure-transform"), &pins());
        let t = text(&findings);
        assert!(t.contains("serde") && t.contains("p1"), "{table}: {t}");
    }
    // inline-table and table forms
    for body in [
        "[dependencies]\nserde = { version = \"1\" }\n",
        "[dependencies.serde]\nversion = \"1\"\n",
    ] {
        let dir = TempDir::new("manifest-rust2");
        clean(&dir);
        dir.write("rust/Cargo.toml", format!("{CARGO}\n{body}"));
        assert!(
            text(&check_manifests(
                dir.path(),
                &entry("pure-transform"),
                &pins()
            ))
            .contains("serde")
        );
    }
}

#[test]
fn the_generator_manifest_is_checked_too() {
    let dir = TempDir::new("manifest-gen");
    clean(&dir);
    dir.write(
        "inputs/gen/Cargo.toml",
        format!("{CARGO}\n[dependencies]\nrand = \"0.8\"\n"),
    );
    let t = text(&check_manifests(
        dir.path(),
        &entry("pure-transform"),
        &pins(),
    ));
    assert!(t.contains("rand") && t.contains("inputs/gen"), "{t}");
}

#[test]
fn rayon_is_allowed_only_in_data_parallel_programs() {
    let dir = TempDir::new("manifest-rayon");
    clean(&dir);
    dir.write(
        "rust/Cargo.toml",
        format!("{CARGO}\n[dependencies]\nrayon = \"=1.2.3\"\n"),
    );
    let ok = check_manifests(dir.path(), &entry("data-parallel"), &pins());
    assert!(ok.is_empty(), "{}", text(&ok));
    let t = text(&check_manifests(
        dir.path(),
        &entry("pure-transform"),
        &pins(),
    ));
    assert!(t.contains("rayon") && t.contains("data-parallel"), "{t}");
}

#[test]
fn rayon_needs_an_approved_pin_even_in_data_parallel() {
    let dir = TempDir::new("manifest-nopin");
    clean(&dir);
    dir.write(
        "rust/Cargo.toml",
        format!("{CARGO}\n[dependencies]\nrayon = \"=1.2.3\"\n"),
    );
    let no_libs = parse_pins("t.toml", PINS.split("[[libraries]]").next().unwrap()).unwrap();
    let t = text(&check_manifests(
        dir.path(),
        &entry("data-parallel"),
        &no_libs,
    ));
    assert!(t.contains("rayon"), "{t}");
}

#[test]
fn a_go_requirement_is_refused() {
    for body in [
        "module p1\n\ngo 1.27\n\nrequire github.com/x/y v1.0.0\n",
        "module p1\n\ngo 1.27\n\nrequire (\n\tgithub.com/x/y v1.0.0\n\tgolang.org/x/sync v0.1.0\n)\n",
    ] {
        let dir = TempDir::new("manifest-go");
        clean(&dir);
        dir.write("go/go.mod", body);
        let t = text(&check_manifests(
            dir.path(),
            &entry("data-parallel"),
            &pins(),
        ));
        assert!(t.contains("github.com/x/y") && t.contains("go.mod"), "{t}");
    }
}

#[test]
fn ocaml_libraries_outside_the_distribution_are_refused() {
    let dir = TempDir::new("manifest-ocaml");
    clean(&dir);
    dir.write(
        "ocaml/bin/dune",
        "(executable (name main) (libraries unix cohttp))\n",
    );
    let t = text(&check_manifests(
        dir.path(),
        &entry("pure-transform"),
        &pins(),
    ));
    assert!(t.contains("cohttp"), "{t}");

    clean(&dir);
    dir.write(
        "ocaml/dune-project",
        "(lang dune 3.24)\n(package (name p1) (depends ocaml dune (yojson (>= 2.0))))\n",
    );
    let t = text(&check_manifests(
        dir.path(),
        &entry("pure-transform"),
        &pins(),
    ));
    assert!(t.contains("yojson"), "{t}");
}

#[test]
fn domainslib_is_allowed_only_in_data_parallel_programs() {
    let dir = TempDir::new("manifest-domainslib");
    clean(&dir);
    dir.write(
        "ocaml/bin/dune",
        "(executable (name main) (libraries domainslib))\n",
    );
    let ok = check_manifests(dir.path(), &entry("data-parallel"), &pins());
    assert!(ok.is_empty(), "{}", text(&ok));
    let t = text(&check_manifests(
        dir.path(),
        &entry("parsing-text"),
        &pins(),
    ));
    assert!(t.contains("domainslib"), "{t}");
}

#[test]
fn every_rust_package_commits_its_lockfile() {
    for missing in ["rust/Cargo.lock", "inputs/gen/Cargo.lock"] {
        let dir = TempDir::new("manifest-lock");
        clean(&dir);
        std::fs::remove_file(dir.path().join(missing)).unwrap();
        let t = text(&check_manifests(
            dir.path(),
            &entry("pure-transform"),
            &pins(),
        ));
        assert!(
            t.contains(missing) && t.contains("lockfile"),
            "{missing}: {t}"
        );
    }
}
