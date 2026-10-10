//! `corpus/index.toml` (BR1.1 to BR1.4, BR3.3).

use crate::finding::Finding;
use crate::model::{Class, Composition};
use crate::toml::{self, Value};
use std::collections::BTreeSet;
use std::path::Path;

/// How a program receives its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Stdin,
    Files,
    FixtureDir,
}

impl InputMode {
    fn parse(text: &str) -> Option<InputMode> {
        match text {
            "stdin" => Some(InputMode::Stdin),
            "files" => Some(InputMode::Files),
            "fixture-dir" => Some(InputMode::FixtureDir),
            _ => None,
        }
    }
}

/// How a program's result is compared with `expected/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparison {
    StdoutExact,
    StdoutAndTree,
}

impl Comparison {
    fn parse(text: &str) -> Option<Comparison> {
        match text {
            "stdout-exact" => Some(Comparison::StdoutExact),
            "stdout-and-tree" => Some(Comparison::StdoutAndTree),
            _ => None,
        }
    }
}

/// One program's row in the index (entities.md IndexEntry).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    pub program_id: String,
    pub class: Class,
    pub task: String,
    pub path: String,
    pub args: Vec<String>,
    pub input_mode: InputMode,
    pub stdin_from: Option<String>,
    pub threads_flag: bool,
    pub generator_outputs: Vec<String>,
    pub size_range: (u64, u64),
    pub reference_size: u64,
    pub fixed_inputs: Vec<String>,
    pub comparison: Comparison,
    pub floating_point: bool,
}

/// The parsed index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub schema_version: i64,
    pub entries: Vec<IndexEntry>,
}

/// The most fixed inputs one program may have (BR7.3).
pub const MAX_FIXED_INPUTS: usize = 8;

/// Parses `text` (read from `file`) and validates every entry's run data.
pub fn parse_index(file: &str, text: &str) -> Result<Index, Vec<Finding>> {
    let doc = toml::parse(text).map_err(|e| vec![Finding::new(file, e.to_string())])?;
    let mut findings = Vec::new();
    let schema_version = doc.get("schema_version").and_then(Value::as_int);
    if schema_version != Some(1) {
        findings.push(Finding::new(file, "schema_version must be 1").field("schema_version"));
    }
    let mut entries = Vec::new();
    let tables = doc.get("entries").and_then(Value::as_array).unwrap_or(&[]);
    for (i, table) in tables.iter().enumerate() {
        match parse_entry(file, i, table) {
            Ok(entry) => entries.push(entry),
            Err(mut found) => findings.append(&mut found),
        }
    }
    if findings.is_empty() {
        check_unique_and_ordered(file, &entries, &mut findings);
    }
    if findings.is_empty() {
        Ok(Index {
            schema_version: 1,
            entries,
        })
    } else {
        Err(findings)
    }
}

fn check_unique_and_ordered(file: &str, entries: &[IndexEntry], findings: &mut Vec<Finding>) {
    let mut seen = BTreeSet::new();
    for e in entries {
        if !seen.insert(e.program_id.clone()) {
            findings.push(
                Finding::new(file, "program listed twice")
                    .program(&e.program_id)
                    .field("program_id"),
            );
        }
    }
    for pair in entries.windows(2) {
        let key = |e: &IndexEntry| (e.class, e.program_id.clone());
        if key(&pair[0]) > key(&pair[1]) {
            findings.push(
                Finding::new(
                    file,
                    format!(
                        "entries must be in order by class, then program_id; \
                         `{}` must come before `{}`",
                        pair[1].program_id, pair[0].program_id
                    ),
                )
                .program(&pair[1].program_id),
            );
        }
    }
}

/// Reads one entry table, collecting every problem with it.
fn parse_entry(file: &str, position: usize, table: &Value) -> Result<IndexEntry, Vec<Finding>> {
    let label = table
        .get("program_id")
        .and_then(Value::as_str)
        .map_or_else(|| format!("entries[{position}]"), str::to_string);
    let mut findings = Vec::new();
    let mut bad = |field: &str, message: &str| {
        findings.push(Finding::new(file, message).program(&label).field(field));
    };

    let text_field = |name: &str, bad: &mut dyn FnMut(&str, &str)| -> Option<String> {
        match table.get(name) {
            Some(Value::Str(s)) if !s.is_empty() => Some(s.clone()),
            Some(_) => {
                bad(name, "must be a non-empty string");
                None
            }
            None => {
                bad(name, "is missing");
                None
            }
        }
    };
    let bool_field = |name: &str, bad: &mut dyn FnMut(&str, &str)| -> Option<bool> {
        match table.get(name) {
            Some(Value::Bool(b)) => Some(*b),
            Some(_) => {
                bad(name, "must be true or false");
                None
            }
            None => {
                bad(name, "is missing");
                None
            }
        }
    };
    let list_field = |name: &str, bad: &mut dyn FnMut(&str, &str)| -> Option<Vec<String>> {
        match table.get(name) {
            Some(Value::Array(items)) => {
                let strings: Option<Vec<String>> = items
                    .iter()
                    .map(|v| v.as_str().map(str::to_string))
                    .collect();
                if strings.is_none() {
                    bad(name, "must be a list of strings");
                }
                strings
            }
            Some(_) => {
                bad(name, "must be a list of strings");
                None
            }
            None => {
                bad(name, "is missing");
                None
            }
        }
    };

    let program_id = text_field("program_id", &mut bad);
    if let Some(id) = &program_id {
        let kebab = id.len() <= 32
            && !id.starts_with('-')
            && !id.ends_with('-')
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !kebab {
            bad(
                "program_id",
                "must be lowercase kebab-case, at most 32 bytes",
            );
        }
    }
    let class_text = text_field("class", &mut bad);
    let class = class_text.as_deref().and_then(|c| {
        let parsed = Class::parse(c);
        if parsed.is_none() {
            bad("class", "is not one of the four classes");
        }
        parsed
    });
    let task = text_field("task", &mut bad);
    let path = text_field("path", &mut bad);
    if let (Some(path), Some(class), Some(id)) = (&path, class, &program_id) {
        let want = format!("corpus/{class}/{id}/");
        if *path != want {
            bad("path", &format!("must be `{want}`"));
        }
    }
    let args = list_field("args", &mut bad);
    let mode_text = text_field("input_mode", &mut bad);
    let input_mode = mode_text.as_deref().and_then(|m| {
        let parsed = InputMode::parse(m);
        if parsed.is_none() {
            bad("input_mode", "must be stdin, files or fixture-dir");
        }
        parsed
    });
    let stdin_from = match table.get("stdin_from") {
        Some(Value::Str(s)) => Some(s.clone()),
        Some(_) => {
            bad("stdin_from", "must be a string");
            None
        }
        None => None,
    };
    let threads_flag = bool_field("threads_flag", &mut bad);
    let generator_outputs = list_field("generator_outputs", &mut bad);
    if generator_outputs.as_ref().is_some_and(Vec::is_empty) {
        bad("generator_outputs", "must name at least one output");
    }
    let size_text = text_field("size_range", &mut bad);
    let size_range = size_text.as_deref().and_then(|s| {
        let parts: Vec<&str> = s.split(' ').collect();
        let nums: Option<Vec<u64>> = parts.iter().map(|p| p.parse().ok()).collect();
        match nums.as_deref() {
            Some([lo, hi]) if lo >= &1 && lo <= hi => Some((*lo, *hi)),
            _ => {
                bad(
                    "size_range",
                    "must be two integers `<smallest> <largest>` with 1 <= smallest <= largest",
                );
                None
            }
        }
    });
    let reference_size = match table.get("reference_size") {
        Some(Value::Int(n)) if *n >= 1 => Some(*n as u64),
        Some(_) => {
            bad("reference_size", "must be a positive integer");
            None
        }
        None => {
            bad("reference_size", "is missing");
            None
        }
    };
    if let (Some(r), Some((lo, hi))) = (reference_size, size_range)
        && (r < lo || r > hi)
    {
        bad("reference_size", "must lie within size_range");
    }
    let fixed_inputs = list_field("fixed_inputs", &mut bad);
    if let Some(fixed) = &fixed_inputs {
        if fixed.is_empty() || fixed.len() > MAX_FIXED_INPUTS {
            bad(
                "fixed_inputs",
                &format!("must list 1 to {MAX_FIXED_INPUTS} fixed inputs"),
            );
        }
        let unique: BTreeSet<_> = fixed.iter().collect();
        if unique.len() != fixed.len() {
            bad("fixed_inputs", "names must be unique");
        }
    }
    let cmp_text = text_field("comparison", &mut bad);
    let comparison = cmp_text.as_deref().and_then(|c| {
        let parsed = Comparison::parse(c);
        if parsed.is_none() {
            bad("comparison", "must be stdout-exact or stdout-and-tree");
        }
        parsed
    });
    let floating_point = bool_field("floating_point", &mut bad);

    if let (Some(class), Some(comparison)) = (class, comparison) {
        let want = if class == Class::IoServices {
            Comparison::StdoutAndTree
        } else {
            Comparison::StdoutExact
        };
        if comparison != want {
            bad(
                "comparison",
                "must be stdout-and-tree for io-services and stdout-exact otherwise",
            );
        }
    }
    if let (Some(class), Some(flag)) = (class, threads_flag)
        && flag != (class == Class::DataParallel)
    {
        bad(
            "threads_flag",
            "must be true exactly for data-parallel programs",
        );
    }
    if input_mode == Some(InputMode::Stdin) {
        match (&stdin_from, &generator_outputs) {
            (None, _) => bad("stdin_from", "is required when input_mode is stdin"),
            (Some(name), Some(outputs)) if !outputs.contains(name) => {
                bad("stdin_from", "must name one of generator_outputs");
            }
            _ => {}
        }
    }
    if let (Some(args), Some(outputs)) = (&args, &generator_outputs) {
        for problem in check_placeholders(args, outputs, threads_flag, class) {
            bad("args", &problem);
        }
    }

    match (
        findings.is_empty(),
        program_id,
        class,
        task,
        path,
        args,
        input_mode,
        threads_flag,
        generator_outputs,
        size_range,
        reference_size,
        fixed_inputs,
        comparison,
        floating_point,
    ) {
        (
            true,
            Some(program_id),
            Some(class),
            Some(task),
            Some(path),
            Some(args),
            Some(input_mode),
            Some(threads_flag),
            Some(generator_outputs),
            Some(size_range),
            Some(reference_size),
            Some(fixed_inputs),
            Some(comparison),
            Some(floating_point),
        ) => Ok(IndexEntry {
            program_id,
            class,
            task,
            path,
            args,
            input_mode,
            stdin_from,
            threads_flag,
            generator_outputs,
            size_range,
            reference_size,
            fixed_inputs,
            comparison,
            floating_point,
        }),
        _ => Err(findings),
    }
}

/// Problems with the placeholders in an argument template.
fn check_placeholders(
    args: &[String],
    outputs: &[String],
    threads_flag: Option<bool>,
    class: Option<Class>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut uses_threads = false;
    for arg in args {
        let mut rest = arg.as_str();
        while let Some(start) = rest.find('{') {
            let Some(len) = rest[start..].find('}') else {
                problems.push(format!("unterminated placeholder in `{arg}`"));
                break;
            };
            let name = &rest[start + 1..start + len];
            match name {
                "threads" => {
                    uses_threads = true;
                    if threads_flag == Some(false) {
                        problems.push("{threads} needs threads_flag = true".to_string());
                    }
                }
                "helper" => {
                    if class.is_some() && class != Some(Class::IoServices) {
                        problems.push("{helper} is only for io-services".to_string());
                    }
                }
                _ => match name.strip_prefix("input:") {
                    Some(input) if outputs.iter().any(|o| o == input) => {}
                    Some(input) => {
                        problems.push(format!("{{input:{input}}} is not one of generator_outputs"))
                    }
                    None => problems.push(format!("unknown placeholder {{{name}}}")),
                },
            }
            rest = &rest[start + len + 1..];
        }
    }
    if threads_flag == Some(true) && !uses_threads {
        problems.push("threads_flag is true but args has no {threads}".to_string());
    }
    problems
}

/// BR1.2: every entry is one of the approved programs of its class. BR1.1:
/// each class in `full` (classes with an `APPROVAL.md`, or all four when the
/// corpus must be complete) has exactly the approved programs, ten of them.
/// Other classes may still be partly written (BR9.1).
pub fn check_composition(
    file: &str,
    index: &Index,
    comp: &Composition,
    full: &[Class],
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for class in Class::ALL {
        let entries: Vec<&IndexEntry> = index.entries.iter().filter(|e| e.class == class).collect();
        let approved = comp.of_class(class);
        for e in &entries {
            if !approved.contains(&e.program_id.as_str()) {
                findings.push(
                    Finding::new(file, format!("not one of the approved {class} programs"))
                        .program(&e.program_id),
                );
            }
        }
        if !full.contains(&class) {
            continue;
        }
        if entries.len() != comp.per_class {
            findings.push(Finding::new(
                file,
                format!(
                    "class {class} has {} programs, expected {}",
                    entries.len(),
                    comp.per_class
                ),
            ));
        }
        for name in approved {
            if !entries.iter().any(|e| e.program_id == name) {
                findings.push(
                    Finding::new(file, format!("approved {class} program is missing"))
                        .program(name),
                );
            }
        }
    }
    findings
}

/// The directories and files every program has (BR1.3 and entities.md
/// LanguageVersion.entry).
const REQUIRED_DIRS: [&str; 7] = [
    "rail",
    "rust",
    "go",
    "ocaml",
    "inputs/gen",
    "inputs/fixed",
    "expected",
];
const REQUIRED_FILES: [&str; 9] = [
    "prompt.md",
    "fairness-review.md",
    "rail/main.rlc",
    "rust/Cargo.toml",
    "rust/src/main.rs",
    "go/go.mod",
    "go/main.go",
    "ocaml/dune-project",
    "ocaml/bin/main.ml",
];

/// BR1.3: every program has the shared layout under `corpus_dir`.
pub fn check_layout(corpus_dir: &Path, index: &Index) -> Vec<Finding> {
    let mut findings = Vec::new();
    for e in &index.entries {
        let base = corpus_dir.join(e.class.name()).join(&e.program_id);
        let shown = format!("{}/{}", e.class, e.program_id);
        for part in REQUIRED_DIRS {
            if !base.join(part).is_dir() {
                findings.push(
                    Finding::new(&shown, format!("missing directory {part}/"))
                        .program(&e.program_id),
                );
            }
        }
        for part in REQUIRED_FILES {
            if !base.join(part).is_file() {
                findings.push(
                    Finding::new(&shown, format!("missing file {part}")).program(&e.program_id),
                );
            }
        }
    }
    findings
}

const CROCKFORD: &str = "0123456789abcdefghjkmnpqrstvwxyz";

/// BR3.3: definition IDs in a `main.rlc` are well formed and unique. A
/// definition is a line `(<kind> #<id> ...`; the ID is `#` and six lowercase
/// Crockford base32 characters.
pub fn check_rail_ids(file: &str, program: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_start();
        if !line.starts_with('(') {
            continue;
        }
        let Some(token) = line.split_whitespace().nth(1) else {
            continue;
        };
        let Some(token) = token.strip_prefix('#') else {
            continue;
        };
        let id = token.trim_end_matches(')');
        let well_formed = id.len() == 6 && id.chars().all(|c| CROCKFORD.contains(c));
        if !well_formed {
            findings.push(
                Finding::new(
                    file,
                    format!(
                        "line {}: definition ID `#{id}` must be `#` and six lowercase \
                         Crockford base32 characters",
                        i + 1
                    ),
                )
                .program(program),
            );
        } else if !seen.insert(id.to_string()) {
            findings.push(
                Finding::new(
                    file,
                    format!("line {}: definition ID `#{id}` is used twice", i + 1),
                )
                .program(program),
            );
        }
    }
    findings
}
