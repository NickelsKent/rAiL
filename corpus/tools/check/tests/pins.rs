//! Data model: the toolchain pins (BR5.3).

use corpus_check::pins::{Installed, Tool, parse_pins, parse_version_output, verify_installed};

const PINS: &str = r#"schema_version = 1

[toolchains]
rust = "1.98.1"
go = "1.27.2"
ocaml = "5.5.1"
dune = "3.24.2"
opam = "2.6.1"

[sandbox]
bubblewrap_min = "0.6.0"
"#;

const WORKSPACE: &str = "[toolchain]\nchannel = \"1.98.1\"\ncomponents = [\"rustfmt\"]\n";

fn installed() -> Installed {
    Installed {
        rust: "1.98.1".into(),
        go: "1.27.2".into(),
        ocaml: "5.5.1".into(),
        dune: "3.24.2".into(),
    }
}

#[test]
fn parses_the_pins() {
    let pins = parse_pins("toolchains.toml", PINS).expect("valid");
    assert_eq!(pins.rust, "1.98.1");
    assert_eq!(pins.go, "1.27.2");
    assert_eq!(pins.ocaml, "5.5.1");
    assert_eq!(pins.dune, "3.24.2");
    assert_eq!(pins.opam, "2.6.1");
    assert_eq!(pins.bubblewrap_min, "0.6.0");
    assert!(pins.libraries.is_empty());
}

#[test]
fn parses_approved_libraries() {
    let text = format!(
        "{PINS}\n[[libraries]]\nlanguage = \"rust\"\nname = \"rayon\"\nversion = \"1.2.3\"\n\
         checksum = \"sha256:ab\"\nvendored_path = \"corpus/vendor/rust\"\nreason = \"approved\"\n\
         allowed_class = \"data-parallel\"\n"
    );
    let pins = parse_pins("toolchains.toml", &text).expect("valid");
    assert_eq!(pins.libraries.len(), 1);
    assert_eq!(pins.libraries[0].name, "rayon");
    assert_eq!(pins.libraries[0].allowed_class, "data-parallel");
}

#[test]
fn a_missing_pin_is_named() {
    let findings =
        parse_pins("toolchains.toml", &PINS.replace("go = \"1.27.2\"\n", "")).unwrap_err();
    assert!(
        findings.iter().any(|f| f.field.as_deref() == Some("go")),
        "{findings:?}"
    );
}

#[test]
fn matching_versions_are_accepted() {
    let pins = parse_pins("toolchains.toml", PINS).unwrap();
    assert!(verify_installed(&pins, &installed(), WORKSPACE).is_empty());
}

#[test]
fn a_version_mismatch_names_the_toolchain_and_both_versions() {
    let pins = parse_pins("toolchains.toml", PINS).unwrap();
    for (tool, mutate) in [
        (
            "go",
            (|i: &mut Installed| i.go = "1.26.0".into()) as fn(&mut Installed),
        ),
        ("rust", |i| i.rust = "1.97.0".into()),
        ("ocaml", |i| i.ocaml = "5.4.0".into()),
        ("dune", |i| i.dune = "3.20.0".into()),
    ] {
        let mut inst = installed();
        mutate(&mut inst);
        let findings = verify_installed(&pins, &inst, WORKSPACE);
        assert_eq!(findings.len(), 1, "{tool}: {findings:?}");
        let text = findings[0].to_string();
        assert!(text.contains(tool), "{text}");
        let (pin, got) = match tool {
            "go" => ("1.27.2", "1.26.0"),
            "rust" => ("1.98.1", "1.97.0"),
            "ocaml" => ("5.5.1", "5.4.0"),
            _ => ("3.24.2", "3.20.0"),
        };
        assert!(text.contains(pin) && text.contains(got), "{text}");
    }
}

#[test]
fn the_rust_pin_must_equal_the_workspace_toolchain() {
    let pins = parse_pins("toolchains.toml", PINS).unwrap();
    let findings = verify_installed(&pins, &installed(), "[toolchain]\nchannel = \"1.99.0\"\n");
    assert_eq!(findings.len(), 1, "{findings:?}");
    let text = findings[0].to_string();
    assert!(
        text.contains("rust-toolchain.toml") && text.contains("1.98.1") && text.contains("1.99.0")
    );
}

#[test]
fn reads_the_version_from_each_tool_output() {
    assert_eq!(
        parse_version_output(Tool::Rustc, "rustc 1.98.1 (797e8a9bc 2026-08-05)\n").as_deref(),
        Some("1.98.1")
    );
    assert_eq!(
        parse_version_output(Tool::Go, "go version go1.27.2 darwin/arm64\n").as_deref(),
        Some("1.27.2")
    );
    assert_eq!(
        parse_version_output(Tool::Ocaml, "5.5.1\n").as_deref(),
        Some("5.5.1")
    );
    assert_eq!(
        parse_version_output(Tool::Dune, "3.24.2\n").as_deref(),
        Some("3.24.2")
    );
    assert_eq!(
        parse_version_output(Tool::Bwrap, "bubblewrap 0.9.0\n").as_deref(),
        Some("0.9.0")
    );
    assert_eq!(parse_version_output(Tool::Go, "garbage"), None);
}
