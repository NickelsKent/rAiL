//! Acceptance harness: cargo-facing code. The builds of `rail` the suites
//! run against (BR3.6, BR5.3), and the `cargo metadata` walk that keeps the
//! harness's crates out of the `rail` binary (BR5.1, NFR7.7).

// The crate root already forbids unsafe code; this module repeats it because
// `scripts/check-forbid-unsafe.sh` checks every file named `build.rs`.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::harness::settings::Build;

use rail_json::Value;

/// The dependency graph from `cargo metadata`.
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    /// Package id -> package name.
    names: BTreeMap<String, String>,
    /// Package id -> (dependency package id, has a normal edge, has a dev edge).
    edges: BTreeMap<String, Vec<(String, bool, bool)>>,
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("cargo metadata: missing field {key:?}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("cargo metadata: field {key:?} is not a string"))
}

fn items<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], String> {
    field(value, key)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("cargo metadata: field {key:?} is not an array"))
}

/// Reads `cargo metadata --format-version 1` output.
pub fn parse_metadata(json: &[u8]) -> Result<Metadata, String> {
    let root = rail_json::parse(json).map_err(|e| format!("cargo metadata: {e}"))?;
    let mut meta = Metadata::default();
    for package in items(&root, "packages")? {
        meta.names.insert(
            text(package, "id")?.to_string(),
            text(package, "name")?.to_string(),
        );
    }
    for node in items(field(&root, "resolve")?, "nodes")? {
        let mut deps = Vec::new();
        for dep in items(node, "deps")? {
            let mut normal = false;
            let mut dev = false;
            for kind in items(dep, "dep_kinds")? {
                match field(kind, "kind")? {
                    Value::Null => normal = true,
                    k if k.as_str() == Some("dev") => dev = true,
                    _ => {}
                }
            }
            deps.push((text(dep, "pkg")?.to_string(), normal, dev));
        }
        meta.edges.insert(text(node, "id")?.to_string(), deps);
    }
    Ok(meta)
}

impl Metadata {
    fn ids_named(&self, name: &str) -> Vec<&String> {
        self.names
            .iter()
            .filter(|(_, n)| n.as_str() == name)
            .map(|(id, _)| id)
            .collect()
    }

    fn name(&self, id: &str) -> String {
        self.names
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }
}

/// The package names on a path from `from` to `to` that follows only
/// normal (non-dev, non-build) edges, if there is one.
pub fn normal_dependency_path(meta: &Metadata, from: &str, to: &str) -> Option<Vec<String>> {
    let mut parent: BTreeMap<&String, Option<&String>> = BTreeMap::new();
    let mut queue: VecDeque<&String> = VecDeque::new();
    for id in meta.ids_named(from) {
        parent.insert(id, None);
        queue.push_back(id);
    }
    while let Some(id) = queue.pop_front() {
        if meta.names.get(id).is_some_and(|n| n == to) {
            let mut path = vec![meta.name(id)];
            let mut at = parent.get(id).copied().flatten();
            while let Some(p) = at {
                path.push(meta.name(p));
                at = parent.get(p).copied().flatten();
            }
            path.reverse();
            return Some(path);
        }
        for (dep, normal, _) in meta.edges.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            if *normal && !parent.contains_key(dep) {
                parent.insert(dep, Some(id));
                queue.push_back(dep);
            }
        }
    }
    None
}

/// Whether `of` has `dep` as a direct dev-dependency.
pub fn has_dev_dependency(meta: &Metadata, of: &str, dep: &str) -> bool {
    let targets: BTreeSet<&String> = meta.ids_named(dep).into_iter().collect();
    meta.ids_named(of).into_iter().any(|id| {
        meta.edges
            .get(id)
            .is_some_and(|deps| deps.iter().any(|(d, _, dev)| *dev && targets.contains(d)))
    })
}

/// Runs `cargo metadata --format-version 1 --locked` for the workspace at
/// `root` and returns its output.
pub fn cargo_metadata(root: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .map_err(|e| format!("could not start cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

/// A build of `rail` failed: tool error, exit code 2 (BR5.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildError {
    /// The build that failed.
    pub build: Build,
    /// What went wrong.
    pub message: String,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not build rail ({}): {}", self.build, self.message)
    }
}

impl std::error::Error for BuildError {}

/// The pinned nightly toolchain (`fuzz/rust-toolchain.toml`), used only for
/// the sanitizer build.
pub fn pinned_nightly(root: &Path) -> Result<String, String> {
    let path = root.join("fuzz/rust-toolchain.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let table: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    table
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{}: no toolchain.channel", path.display()))
}

/// Builds `rail` for `build` from the workspace at `root` with `--locked`,
/// in the harness's own environment, and returns the binary's path
/// (BR3.6, BR5.3). Debug and release use the pinned stable toolchain; the
/// sanitizer build uses the pinned nightly with `rustflags` as `RUSTFLAGS`
/// and its own target directory.
pub fn build_rail(
    root: &Path,
    build: Build,
    rustflags: Option<&str>,
) -> Result<PathBuf, BuildError> {
    let fail = |message: String| BuildError { build, message };
    let mut cmd = Command::new("cargo");
    let target = match build {
        Build::Debug | Build::Release => root.join("target"),
        Build::Sanitizer => {
            let nightly = pinned_nightly(root).map_err(fail)?;
            cmd.arg(format!("+{nightly}"));
            cmd.env("RUSTFLAGS", rustflags.unwrap_or_default());
            root.join("target/sanitizer")
        }
    };
    cmd.args([
        "build",
        "--locked",
        "--quiet",
        "-p",
        "rail",
        "--manifest-path",
    ])
    .arg(root.join("Cargo.toml"))
    .arg("--target-dir")
    .arg(&target);
    if build == Build::Release {
        cmd.arg("--release");
    }
    let output = cmd
        .stdin(Stdio::null())
        .output()
        .map_err(|e| fail(format!("could not start cargo: {e}")))?;
    if !output.status.success() {
        return Err(fail(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let profile = if build == Build::Release {
        "release"
    } else {
        "debug"
    };
    let binary = target.join(profile).join("rail");
    if binary.is_file() {
        Ok(binary)
    } else {
        Err(fail(format!("{} was not produced", binary.display())))
    }
}
