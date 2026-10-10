//! Library policy (BR5.1, BR5.2) and committed lockfiles (BR5.5).

use crate::finding::Finding;
use crate::index::IndexEntry;
use crate::model::Class;
use crate::pins::Pins;
use crate::toml::{self, Value};
use std::fs;
use std::path::Path;

/// OCaml libraries that ship with the compiler count as the standard library.
const OCAML_DISTRIBUTION: [&str; 4] = ["unix", "threads", "threads.posix", "str"];

/// Checks the Rust, Go and OCaml manifests of one program, and its
/// generator's, against the allowed libraries.
pub fn check_manifests(program_dir: &Path, entry: &IndexEntry, pins: &Pins) -> Vec<Finding> {
    let mut findings = Vec::new();
    let policy = Policy::new(entry, pins);
    for package in ["rust", "inputs/gen"] {
        check_rust_package(program_dir, package, &policy, &mut findings);
    }
    check_go(program_dir, &policy, &mut findings);
    check_ocaml(program_dir, &policy, &mut findings);
    findings
}

/// What this program may depend on.
struct Policy<'a> {
    program: &'a str,
    class: Class,
    rust_approved: Vec<&'a str>,
    ocaml_approved: Vec<&'a str>,
}

impl<'a> Policy<'a> {
    fn new(entry: &'a IndexEntry, pins: &'a Pins) -> Policy<'a> {
        let approved = |language: &str| -> Vec<&'a str> {
            pins.libraries
                .iter()
                .filter(|l| l.language == language)
                .map(|l| l.name.as_str())
                .collect()
        };
        Policy {
            program: &entry.program_id,
            class: entry.class,
            rust_approved: approved("rust"),
            ocaml_approved: approved("ocaml"),
        }
    }

    fn finding(&self, file: &str, message: String) -> Finding {
        Finding::new(file, message).program(self.program)
    }

    /// Why `name` may not be used, or `None` when it may.
    fn refusal(&self, name: &str, approved: &[&str]) -> Option<String> {
        if !approved.contains(&name) {
            let known = matches!(name, "rayon" | "domainslib");
            return Some(if known {
                format!("`{name}` is not an approved library in toolchains.toml")
            } else {
                format!("dependency `{name}` is not allowed (standard library only)")
            });
        }
        if self.class != Class::DataParallel {
            return Some(format!(
                "`{name}` is allowed only in data-parallel programs"
            ));
        }
        None
    }
}

fn check_rust_package(dir: &Path, package: &str, policy: &Policy, findings: &mut Vec<Finding>) {
    let manifest = format!("{package}/Cargo.toml");
    let lock = format!("{package}/Cargo.lock");
    let Ok(text) = fs::read_to_string(dir.join(&manifest)) else {
        // The shared layout check reports a missing rust/ package; a missing
        // generator package is reported here.
        if package == "inputs/gen" {
            findings
                .push(policy.finding(&manifest, "the generator package is missing".to_string()));
        }
        return;
    };
    match toml::parse(&text) {
        Err(e) => findings.push(policy.finding(&manifest, format!("cannot read it: {e}"))),
        Ok(doc) => {
            let mut deps = Vec::new();
            collect_dependency_names(&doc, &mut deps);
            for name in deps {
                if let Some(why) = policy.refusal(&name, &policy.rust_approved) {
                    findings.push(policy.finding(&manifest, why));
                }
            }
        }
    }
    if !dir.join(&lock).is_file() {
        findings.push(policy.finding(
            &lock,
            "every Rust package commits its lockfile (missing lockfile)".to_string(),
        ));
    }
}

/// Every key of any `dependencies`, `dev-dependencies` or
/// `build-dependencies` table, at any depth.
fn collect_dependency_names(value: &Value, out: &mut Vec<String>) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (key, child) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            if let Some(deps) = child.as_table() {
                out.extend(deps.keys().cloned());
            }
        } else {
            collect_dependency_names(child, out);
        }
    }
}

fn check_go(dir: &Path, policy: &Policy, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(dir.join("go/go.mod")) else {
        return;
    };
    let mut in_block = false;
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        let module = if in_block {
            if line == ")" {
                in_block = false;
                None
            } else {
                line.split_whitespace().next()
            }
        } else if let Some(rest) = line.strip_prefix("require") {
            let rest = rest.trim();
            if rest == "(" {
                in_block = true;
                None
            } else {
                rest.split_whitespace().next()
            }
        } else {
            None
        };
        if let Some(module) = module.filter(|m| !m.is_empty()) {
            findings.push(policy.finding(
                "go/go.mod",
                format!("dependency `{module}` is not allowed (Go uses the standard library only)"),
            ));
        }
    }
}

/// A parsed S-expression, enough for dune files.
#[derive(Debug)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn parse_sexps(text: &str) -> Vec<Sexp> {
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ';' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                stack.push(Vec::new());
                i += 1;
            }
            ')' => {
                if stack.len() > 1 {
                    let list = stack.pop().expect("non-empty");
                    stack.last_mut().expect("root").push(Sexp::List(list));
                }
                i += 1;
            }
            '"' => {
                let start = i + 1;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let atom: String = chars[start..i.min(chars.len())].iter().collect();
                stack.last_mut().expect("root").push(Sexp::Atom(atom));
                i += 1;
            }
            c if c.is_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && !"();".contains(chars[i]) {
                    i += 1;
                }
                let atom: String = chars[start..i].iter().collect();
                stack.last_mut().expect("root").push(Sexp::Atom(atom));
            }
        }
    }
    stack.swap_remove(0)
}

/// The children of every list whose first atom is `head`, at any depth.
fn forms<'a>(sexps: &'a [Sexp], head: &str, out: &mut Vec<&'a [Sexp]>) {
    for sexp in sexps {
        if let Sexp::List(items) = sexp {
            if matches!(items.first(), Some(Sexp::Atom(a)) if a == head) {
                out.push(&items[1..]);
            }
            forms(items, head, out);
        }
    }
}

fn check_ocaml(dir: &Path, policy: &Policy, findings: &mut Vec<Finding>) {
    let mut allowed: Vec<&str> = OCAML_DISTRIBUTION.to_vec();
    allowed.extend(&policy.ocaml_approved);
    let judge = |file: &str, name: &str, findings: &mut Vec<Finding>| {
        if OCAML_DISTRIBUTION.contains(&name) {
            return;
        }
        if let Some(why) = policy.refusal(name, &allowed) {
            findings.push(policy.finding(file, why));
        }
    };

    if let Ok(text) = fs::read_to_string(dir.join("ocaml/dune-project")) {
        let sexps = parse_sexps(&text);
        let mut found = Vec::new();
        forms(&sexps, "depends", &mut found);
        for children in found {
            for child in children {
                let name = match child {
                    Sexp::Atom(a) => a.as_str(),
                    Sexp::List(items) => match items.first() {
                        Some(Sexp::Atom(a)) => a.as_str(),
                        _ => continue,
                    },
                };
                if name.starts_with(':') || matches!(name, "ocaml" | "dune") {
                    continue;
                }
                judge("ocaml/dune-project", name, findings);
            }
        }
    }
    for path in dune_files(&dir.join("ocaml")) {
        let shown = format!(
            "ocaml/{}",
            path.strip_prefix(dir.join("ocaml"))
                .unwrap_or(&path)
                .to_string_lossy()
        );
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let sexps = parse_sexps(&text);
        let mut found = Vec::new();
        forms(&sexps, "libraries", &mut found);
        for children in found {
            for child in children {
                if let Sexp::Atom(name) = child {
                    judge(&shown, name, findings);
                }
            }
        }
    }
}

/// Every file named `dune` under `dir`, sorted.
fn dune_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| n == "dune") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}
