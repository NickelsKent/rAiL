//! `corpus/toolchains.toml` (BR5.3, BR5.5).

use crate::finding::Finding;
use crate::toml::{self, Value};

/// An approved third-party library (entities.md ApprovedLibrary).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub language: String,
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub vendored_path: String,
    pub reason: String,
    pub allowed_class: String,
}

/// The pinned toolchains and approved libraries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pins {
    pub rust: String,
    pub go: String,
    pub ocaml: String,
    pub dune: String,
    pub opam: String,
    pub bubblewrap_min: String,
    pub libraries: Vec<Library>,
}

/// The versions found installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub rust: String,
    pub go: String,
    pub ocaml: String,
    pub dune: String,
}

/// A tool whose version output is parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Rustc,
    Go,
    Ocaml,
    Dune,
    Bwrap,
}

const LIBRARY_FIELDS: [&str; 7] = [
    "language",
    "name",
    "version",
    "checksum",
    "vendored_path",
    "reason",
    "allowed_class",
];

/// Parses `toolchains.toml`.
pub fn parse_pins(file: &str, text: &str) -> Result<Pins, Vec<Finding>> {
    let doc = toml::parse(text).map_err(|e| vec![Finding::new(file, e.to_string())])?;
    let mut findings = Vec::new();
    if doc.get("schema_version").and_then(Value::as_int) != Some(1) {
        findings.push(Finding::new(file, "schema_version must be 1").field("schema_version"));
    }
    let mut pin = |section: &str, key: &str| -> String {
        match doc
            .get(section)
            .and_then(|t| t.get(key))
            .and_then(Value::as_str)
        {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => {
                findings.push(
                    Finding::new(file, format!("[{section}] needs a non-empty `{key}`")).field(key),
                );
                String::new()
            }
        }
    };
    let rust = pin("toolchains", "rust");
    let go = pin("toolchains", "go");
    let ocaml = pin("toolchains", "ocaml");
    let dune = pin("toolchains", "dune");
    let opam = pin("toolchains", "opam");
    let bubblewrap_min = pin("sandbox", "bubblewrap_min");

    let mut libraries = Vec::new();
    for lib in doc
        .get("libraries")
        .and_then(Value::as_array)
        .unwrap_or(&[])
    {
        let mut fields = Vec::new();
        for key in LIBRARY_FIELDS {
            match lib.get(key).and_then(Value::as_str) {
                Some(v) if !v.is_empty() => fields.push(v.to_string()),
                _ => {
                    findings.push(
                        Finding::new(file, format!("[[libraries]] entry needs `{key}`")).field(key),
                    );
                    fields.push(String::new());
                }
            }
        }
        let [
            language,
            name,
            version,
            checksum,
            vendored_path,
            reason,
            allowed_class,
        ]: [String; 7] = fields.try_into().expect("seven fields");
        let expected_language = match name.as_str() {
            "rayon" => Some("rust"),
            "domainslib" => Some("ocaml"),
            _ => None,
        };
        if !name.is_empty() && expected_language != Some(language.as_str()) {
            findings.push(
                Finding::new(
                    file,
                    "only rayon (rust) and domainslib (ocaml) are approved libraries",
                )
                .field("name"),
            );
        }
        if !allowed_class.is_empty() && allowed_class != "data-parallel" {
            findings.push(
                Finding::new(file, "libraries are approved for data-parallel only")
                    .field("allowed_class"),
            );
        }
        libraries.push(Library {
            language,
            name,
            version,
            checksum,
            vendored_path,
            reason,
            allowed_class,
        });
    }

    if findings.is_empty() {
        Ok(Pins {
            rust,
            go,
            ocaml,
            dune,
            opam,
            bubblewrap_min,
            libraries,
        })
    } else {
        Err(findings)
    }
}

/// The version number in a tool's version output.
pub fn parse_version_output(tool: Tool, output: &str) -> Option<String> {
    let first = output.lines().next()?.trim();
    let version = match tool {
        Tool::Rustc => first.strip_prefix("rustc ")?.split_whitespace().next()?,
        Tool::Go => first
            .strip_prefix("go version go")?
            .split_whitespace()
            .next()?,
        Tool::Bwrap => first
            .strip_prefix("bubblewrap ")?
            .split_whitespace()
            .next()?,
        Tool::Ocaml | Tool::Dune => first.split_whitespace().next()?,
    };
    version
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
        .then(|| version.to_string())
}

/// BR5.3: every installed toolchain equals its pin, and the Rust pin equals
/// the workspace's `rust-toolchain.toml` (`workspace_toolchain` is its text).
pub fn verify_installed(
    pins: &Pins,
    installed: &Installed,
    workspace_toolchain: &str,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (name, pin, got) in [
        ("rust", &pins.rust, &installed.rust),
        ("go", &pins.go, &installed.go),
        ("ocaml", &pins.ocaml, &installed.ocaml),
        ("dune", &pins.dune, &installed.dune),
    ] {
        // An empty version means the language was not selected.
        if !got.is_empty() && pin != got {
            findings.push(
                Finding::new(
                    "toolchains.toml",
                    format!("installed {name} is {got} but the pin is {pin}"),
                )
                .field(name),
            );
        }
    }
    let channel = toml::parse(workspace_toolchain)
        .ok()
        .and_then(|d| {
            d.get("toolchain")
                .and_then(|t| t.get("channel"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    if channel != pins.rust {
        findings.push(
            Finding::new(
                "toolchains.toml",
                format!(
                    "the rust pin is {} but rust-toolchain.toml says {channel}",
                    pins.rust
                ),
            )
            .field("rust"),
        );
    }
    findings
}

/// Reads the installed versions of the toolchains the selected languages
/// need (`rustc` always). A language that is not selected leaves its field
/// empty, which `verify_installed` does not check. The tools run in `dir`,
/// so a `rust-toolchain.toml` there or above selects the toolchain, as it
/// does for the builds.
pub fn detect_installed(
    langs: &[crate::model::Lang],
    dir: &std::path::Path,
) -> Result<Installed, String> {
    use crate::model::Lang;
    let version = |program: &str, args: &[&str], tool: Tool| -> Result<String, String> {
        let out = std::process::Command::new(program)
            .args(args)
            .current_dir(dir)
            .output()
            .map_err(|e| format!("cannot run `{program}`: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout);
        parse_version_output(tool, &text).ok_or_else(|| {
            format!(
                "cannot read the version from `{program} {}`",
                args.join(" ")
            )
        })
    };
    let wants = |lang: Lang| langs.contains(&lang);
    Ok(Installed {
        rust: version("rustc", &["--version"], Tool::Rustc)?,
        go: if wants(Lang::Go) {
            version("go", &["version"], Tool::Go)?
        } else {
            String::new()
        },
        ocaml: if wants(Lang::Ocaml) {
            version("ocaml", &["-vnum"], Tool::Ocaml)?
        } else {
            String::new()
        },
        dune: if wants(Lang::Ocaml) {
            version("dune", &["--version"], Tool::Dune)?
        } else {
            String::new()
        },
    })
}
