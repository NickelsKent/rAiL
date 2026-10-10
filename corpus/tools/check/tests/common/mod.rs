//! Helpers shared by the checker's tests. Each test file uses a subset.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh temporary directory, removed (with permissions restored) on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "corpus-check-test-{}-{label}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        // Resolve symlinks (macOS /var -> /private/var) so sandbox profiles
        // and path comparisons see the canonical path.
        let path = path.canonicalize().expect("canonical temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `contents` at `rel`, creating parent directories.
    pub fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, contents).unwrap();
        p
    }

    pub fn mkdir(&self, rel: &str) -> PathBuf {
        let p = self.path.join(rel);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        restore_permissions(&self.path);
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
fn restore_permissions(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(rd) = fs::read_dir(dir) else {
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                restore_permissions(&e.path());
            }
        }
        return;
    };
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            restore_permissions(&p);
        } else {
            let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
        }
    }
}

/// One valid `[[entries]]` table for `program` in `class`.
pub fn entry_toml(program: &str, class: &str) -> String {
    format!(
        r#"[[entries]]
program_id = "{program}"
class = "{class}"
task = "A task."
path = "corpus/{class}/{program}/"
args = ["{{input:data.txt}}"]
input_mode = "files"
threads_flag = {threads}
generator_outputs = ["data.txt"]
size_range = "10 1000"
reference_size = 100
fixed_inputs = ["basic", "bad"]
comparison = "{comparison}"
floating_point = false
"#,
        threads = class == "data-parallel",
        comparison = if class == "io-services" {
            "stdout-and-tree"
        } else {
            "stdout-exact"
        },
    )
}

/// An index with `per_class` programs named `p1..` in each class.
pub fn index_toml(per_class: usize, classes: &[&str]) -> String {
    let mut s = String::from("schema_version = 1\n\n");
    for class in classes {
        for i in 1..=per_class {
            let mut e = entry_toml(&format!("{class}-p{i}"), class);
            if *class == "data-parallel" {
                e = e.replace(
                    "args = [\"{input:data.txt}\"]",
                    "args = [\"--threads\", \"{threads}\", \"{input:data.txt}\"]",
                );
            }
            s.push_str(&e);
            s.push('\n');
        }
    }
    s
}

/// Compiles `source` with rustc into `dir/name` and returns the executable.
/// Stand-in programs are single files, so the tests stay fast and need no
/// cargo packages.
pub fn build_rust(dir: &Path, name: &str, source: &str) -> PathBuf {
    let src = dir.join(format!("{name}.rs"));
    fs::write(&src, source).unwrap();
    let exe = dir.join(name);
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let out = std::process::Command::new(rustc)
        .args(["--edition", "2024", "-C", "opt-level=0", "-o"])
        .arg(&exe)
        .arg(&src)
        .output()
        .expect("rustc starts");
    assert!(
        out.status.success(),
        "stand-in {name} failed to compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    exe
}

/// A stand-in input generator. `mode` is one of `ok`, `nondet` (different
/// bytes every run), `extra` (a different file set every run) or `fail`.
pub fn generator_source(mode: &str) -> String {
    format!(
        r#"
use std::collections::hash_map::RandomState;
use std::hash::{{BuildHasher, Hasher}};
const MODE: &str = "{mode}";
fn random() -> u64 {{ RandomState::new().build_hasher().finish() }}
fn main() {{
    let a: Vec<String> = std::env::args().collect();
    let (mut seed, mut size, mut out) = (0u64, 0u64, String::new());
    let mut i = 1;
    while i < a.len() {{
        match a[i].as_str() {{
            "--seed" => seed = a[i + 1].parse().unwrap(),
            "--size" => size = a[i + 1].parse().unwrap(),
            "--out" => out = a[i + 1].clone(),
            _ => std::process::exit(2),
        }}
        i += 2;
    }}
    if MODE == "fail" {{ eprintln!("generator failed"); std::process::exit(3); }}
    std::fs::create_dir_all(&out).unwrap();
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut s = String::new();
    for _ in 0..size {{
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s.push_str(&format!("{{}}\n", x >> 33));
    }}
    if MODE == "nondet" {{ s.push_str(&format!("{{}}\n", random())); }}
    std::fs::write(format!("{{out}}/data.txt"), s).unwrap();
    if MODE == "extra" {{ std::fs::write(format!("{{out}}/extra-{{}}.txt", random()), "x").unwrap(); }}
}}
"#
    )
}

// ---- stand-in programs and run helpers ----

use corpus_check::check::{Context, Job};
use corpus_check::expected::Mode;
use corpus_check::index::{Comparison, IndexEntry, InputMode};
use corpus_check::model::{Class, Lang};
use corpus_check::sandbox::{Platform, Sandbox, default_search_path};
use std::time::Duration;

/// The corpus-check binary, used as the sandbox probe.
pub const PROBE: &str = env!("CARGO_BIN_EXE_corpus-check");

/// The macOS sandbox profile shipped with the checker.
pub fn macos_profile() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sandbox/macos.sb")
}

/// The sandbox for this platform, located but not self-tested.
pub fn live_sandbox() -> Sandbox {
    Sandbox::locate(
        Platform::current(),
        &default_search_path(),
        &macos_profile(),
    )
    .expect("the platform's sandbox tool is installed")
}

/// A stand-in program whose behaviour is chosen by its first argument.
pub const STANDIN: &str = r#"
use std::io::{Read, Write};
fn denied(e: std::io::Error) -> ! {
    println!("denied");
    eprintln!("{e}");
    std::process::exit(7)
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str).unwrap_or("ok") {
        "ok" => println!("hello"),
        "wrong" => println!("hellp"),
        "short" => print!("hell"),
        "echo-threads" => println!("total={}", 1000 + a[3].parse::<u64>().unwrap()),
        "stable-threads" => println!("total=1000"),
        "write-abs" => match std::fs::write(&a[2], "x") {
            Ok(()) => println!("wrote"),
            Err(e) => denied(e),
        },
        "read-dir" => match std::fs::read_dir(&a[2]) {
            Ok(_) => println!("listed"),
            Err(e) => denied(e),
        },
        "connect" => match std::net::TcpStream::connect(&a[2]) {
            Ok(_) => println!("connected"),
            Err(e) => denied(e),
        },
        "escape-rel" => {
            let _ = std::fs::write("../canary/escaped.txt", "x");
            println!("hello");
        }
        "stray" => {
            std::fs::write("stray.txt", "x").unwrap();
            println!("hello");
        }
        "hang" => std::thread::sleep(std::time::Duration::from_secs(8)),
        "fork-hang" => {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("hang")
                .spawn()
                .unwrap();
            println!("child={}", child.id());
            std::io::stdout().flush().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(8));
        }
        "flood" => {
            // 200 MiB, far past the checker's output limit, but finite.
            let block = [b'x'; 65536];
            let mut out = std::io::stdout();
            for _ in 0..3200 {
                if out.write_all(&block).is_err() {
                    break;
                }
            }
        }
        "perm" => match std::fs::read_to_string("locked.txt") {
            Ok(s) => print!("{s}"),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("error: permission: locked.txt");
                std::process::exit(1)
            }
            Err(e) => denied(e),
        },
        "cat-input" => print!("{}", std::fs::read_to_string(&a[2]).unwrap()),
        "stdin-upper" => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).unwrap();
            print!("{}", s.to_uppercase());
        }
        _ => std::process::exit(9),
    }
}
"#;

/// Builds the stand-in into `dir/build/standin`; returns (exe, build dir).
pub fn build_standin(dir: &Path) -> (PathBuf, PathBuf) {
    let build = dir.join("build");
    fs::create_dir_all(&build).unwrap();
    // The build directory is resolved so sandbox profiles see real paths.
    let build = build.canonicalize().unwrap();
    let exe = build_rust(&build, "standin", STANDIN);
    (exe, build)
}

/// An index entry for run tests.
pub fn make_entry(program: &str, class: Class, args: &[&str], fixed: &[&str]) -> IndexEntry {
    IndexEntry {
        program_id: program.to_string(),
        class,
        task: "A task.".to_string(),
        path: format!("corpus/{class}/{program}/"),
        args: args.iter().map(ToString::to_string).collect(),
        input_mode: InputMode::Files,
        stdin_from: None,
        threads_flag: class == Class::DataParallel,
        generator_outputs: vec!["data.txt".to_string()],
        size_range: (1, 100),
        reference_size: 10,
        fixed_inputs: fixed.iter().map(ToString::to_string).collect(),
        comparison: if class == Class::IoServices {
            Comparison::StdoutAndTree
        } else {
            Comparison::StdoutExact
        },
        floating_point: false,
    }
}

/// A program directory holding fixed inputs: (input name, [(file, text)]).
pub fn make_program_dir(
    root: &TempDir,
    program: &str,
    inputs: &[(&str, &[(&str, &str)])],
) -> PathBuf {
    let base = format!("programs/{program}");
    root.mkdir(&format!("{base}/inputs/fixed"));
    root.mkdir(&format!("{base}/expected"));
    for (input, files) in inputs {
        root.mkdir(&format!("{base}/inputs/fixed/{input}"));
        for (file, text) in *files {
            root.write(&format!("{base}/inputs/fixed/{input}/{file}"), text);
        }
    }
    root.path().join(base)
}

pub fn context(sandbox: Sandbox, scratch: &Path, mode: Mode) -> Context {
    Context {
        sandbox,
        mode,
        deadline: Duration::from_secs(60),
        budget: corpus_check::budget::Budget::new(None),
        is_root: false,
        cores: 4,
        scratch: scratch.to_path_buf(),
        helper: None,
    }
}

pub fn job(
    entry: &IndexEntry,
    lang: Lang,
    program_dir: &Path,
    exe: &Path,
    build_dir: &Path,
) -> Job {
    Job {
        entry: entry.clone(),
        lang,
        program_dir: program_dir.to_path_buf(),
        exe: exe.to_path_buf(),
        build_dir: build_dir.to_path_buf(),
    }
}

// ---- synthetic corpora for the command-line tests ----

use corpus_check::pins::detect_installed;

pub const RUST_HELLO: &str = r#"fn main() {
    let path = std::env::args().nth(1).expect("an input path");
    let text = std::fs::read_to_string(path).expect("readable input");
    if text.contains("slow") {
        std::thread::sleep(std::time::Duration::from_secs(30));
    }
    println!("hello {}", text.trim());
}
"#;

const GO_HELLO: &str = r#"package main

import (
	"fmt"
	"os"
	"strings"
)

func main() {
	data, err := os.ReadFile(os.Args[1])
	if err != nil {
		os.Exit(3)
	}
	fmt.Println("hello", strings.TrimSpace(string(data)))
}
"#;

const OCAML_HELLO: &str = r#"let () =
  let s = In_channel.with_open_bin Sys.argv.(1) In_channel.input_all in
  print_endline ("hello " ^ String.trim s)
"#;

/// A deterministic generator package source.
pub const GEN_OK: &str = r#"fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut seed = 0u64;
    let mut size = 0u64;
    let mut out = String::new();
    let mut i = 1;
    while i + 1 < a.len() {
        match a[i].as_str() {
            "--seed" => seed = a[i + 1].parse().unwrap(),
            "--size" => size = a[i + 1].parse().unwrap(),
            "--out" => out = a[i + 1].clone(),
            _ => std::process::exit(2),
        }
        i += 2;
    }
    std::fs::create_dir_all(&out).unwrap();
    let mut x = seed;
    let mut s = String::new();
    for _ in 0..size {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
        s.push_str(&format!("{}\n", x >> 40));
    }
    std::fs::write(format!("{out}/data.txt"), s).unwrap();
}
"#;

/// A generator whose output changes from run to run.
pub const GEN_RANDOM: &str = r#"use std::hash::{BuildHasher, Hasher};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = a.iter().position(|x| x == "--out").map(|i| a[i + 1].clone()).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let r = std::collections::hash_map::RandomState::new().build_hasher().finish();
    std::fs::write(format!("{out}/data.txt"), format!("{r}\n")).unwrap();
}
"#;

fn cargo_package(dir: &TempDir, base: &str, name: &str, main: &str) {
    dir.write(
        &format!("{base}/Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    );
    dir.write(
        &format!("{base}/Cargo.lock"),
        format!("version = 4\n\n[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
    );
    dir.write(&format!("{base}/src/main.rs"), main);
}

/// A synthetic corpus: `root/corpus/...` with the given programs
/// (name, class), each with rust, go and ocaml versions that print
/// `hello <input>`, a generator, two fixed inputs `one` and `two`, and a
/// `root/rust-toolchain.toml` equal to the repository's.
pub struct Synth {
    pub root: TempDir,
}

impl Synth {
    pub fn new(programs: &[(&str, &str)]) -> Synth {
        let root = TempDir::new("synth");
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let toolchain = fs::read_to_string(repo.join("rust-toolchain.toml")).unwrap();
        root.write("rust-toolchain.toml", toolchain);
        let mut index = String::from("schema_version = 1\n\n");
        for (name, class) in programs {
            let entry =
                entry_toml(name, class).replace("[\"basic\", \"bad\"]", "[\"one\", \"two\"]");
            index.push_str(&entry);
            index.push('\n');
        }
        root.write("corpus/index.toml", index);
        let synth = Synth { root };
        for (name, class) in programs {
            synth.write_program(name, class);
        }
        synth.write_pins(&[Lang::Rust, Lang::Go, Lang::Ocaml]);
        synth
    }

    pub fn corpus(&self) -> PathBuf {
        self.root.path().join("corpus")
    }

    pub fn program(&self, class: &str, name: &str) -> PathBuf {
        self.corpus().join(class).join(name)
    }

    fn write_program(&self, name: &str, class: &str) {
        let r = &self.root;
        let b = format!("corpus/{class}/{name}");
        r.write(&format!("{b}/prompt.md"), "# prompt\n");
        r.write(&format!("{b}/fairness-review.md"), "# review\n");
        r.write(
            &format!("{b}/rail/main.rlc"),
            format!("(mod {name})\n(fn #sk0a1b main (_) 0)\n"),
        );
        cargo_package(r, &format!("{b}/rust"), name, RUST_HELLO);
        cargo_package(
            r,
            &format!("{b}/inputs/gen"),
            &format!("{name}-gen"),
            GEN_OK,
        );
        r.write(
            &format!("{b}/go/go.mod"),
            format!("module {name}\n\ngo 1.27\n"),
        );
        r.write(&format!("{b}/go/main.go"), GO_HELLO);
        r.write(&format!("{b}/ocaml/dune-project"), "(lang dune 3.0)\n");
        r.write(&format!("{b}/ocaml/bin/dune"), "(executable (name main))\n");
        r.write(&format!("{b}/ocaml/bin/main.ml"), OCAML_HELLO);
        r.write(&format!("{b}/inputs/fixed/one/data.txt"), "one\n");
        r.write(&format!("{b}/inputs/fixed/two/data.txt"), "two\n");
        r.mkdir(&format!("{b}/expected"));
    }

    /// Writes `toolchains.toml` with the versions installed here. A language
    /// whose toolchain is missing, or not listed in `langs`, gets a pin no
    /// machine has (its check is not requested in those tests).
    pub fn write_pins(&self, langs: &[Lang]) {
        let root = self.root.path();
        let rust = detect_installed(&[], root)
            .expect("rustc is installed")
            .rust;
        let go = langs
            .contains(&Lang::Go)
            .then(|| detect_installed(&[Lang::Go], root).ok())
            .flatten();
        let ocaml = langs
            .contains(&Lang::Ocaml)
            .then(|| detect_installed(&[Lang::Ocaml], root).ok())
            .flatten();
        let none = "0.0.0".to_string();
        self.root.write(
            "corpus/toolchains.toml",
            format!(
                "schema_version = 1\n[toolchains]\nrust = \"{rust}\"\ngo = \"{}\"\nocaml = \"{}\"\n\
                 dune = \"{}\"\nopam = \"0\"\n[sandbox]\nbubblewrap_min = \"0.6.0\"\n",
                go.map_or(none.clone(), |i| i.go),
                ocaml.clone().map_or(none.clone(), |i| i.ocaml),
                ocaml.map_or(none, |i| i.dune),
            ),
        );
    }

    /// Writes the expected outputs a correct program gives.
    pub fn write_expected(&self, class: &str, name: &str) {
        let dir = self.program(class, name);
        fs::write(dir.join("expected/one.out"), "hello one\n").unwrap();
        fs::write(dir.join("expected/two.out"), "hello two\n").unwrap();
    }
}

/// Whether Go, dune and OCaml are installed; without them the three-language
/// tests say so and stop, unless CORPUS_CHECK_REQUIRE_TOOLCHAINS is set.
pub fn have_all_toolchains() -> bool {
    let found = [
        ("go", "version"),
        ("dune", "--version"),
        ("ocaml", "-version"),
    ]
    .iter()
    .all(|(tool, arg)| {
        std::process::Command::new(tool)
            .arg(arg)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    });
    if !found {
        assert!(
            std::env::var_os("CORPUS_CHECK_REQUIRE_TOOLCHAINS").is_none(),
            "go, dune and ocaml are required but not all are on PATH"
        );
        eprintln!("SKIPPED: go, dune or ocaml is not on PATH");
    }
    found
}

/// Runs the corpus-check binary; returns (exit code, stdout, stderr).
pub fn run_cli(synth: &Synth, args: &[&str], env: &[(&str, &str)]) -> (i32, String, String) {
    let mut cmd = std::process::Command::new(PROBE);
    cmd.current_dir(synth.root.path())
        .arg("--corpus-dir")
        .arg(synth.corpus())
        .arg("--sandbox-profile")
        .arg(macos_profile())
        .arg("--work-dir")
        .arg(synth.root.path().join("work"))
        .args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("corpus-check starts");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}
