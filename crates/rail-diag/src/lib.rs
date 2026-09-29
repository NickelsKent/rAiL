//! Diagnostic shape (contract C3) and the fixed rule registry.
//!
//! Building block: LintEngine. Owning unit: U1 walking-skeleton (thin first
//! version; completed by U4 checker-and-lints).
//!
//! Every skeleton diagnostic is an error (level `E`) and blocks building
//! (BR4.2). Diagnostics are always sorted by (module, def, path, span, rule)
//! so identical input gives identical output (BR4.3).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::cmp::Ordering;
use std::collections::BTreeMap;

use rail_json::{Number, Value};

/// A permanent rule identifier such as `TY001` (never reused, FR11.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(&'static str);

impl RuleId {
    /// A rule identifier; validity is checked when it is registered.
    pub const fn new(id: &'static str) -> RuleId {
        RuleId(id)
    }

    /// The identifier text.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Type mismatch.
pub const TY001: RuleId = RuleId("TY001");
/// Effect used but not declared in the signature.
pub const FX001: RuleId = RuleId("FX001");
/// Outside what this toolchain build can check yet. Temporary: retired when
/// U3 and U4 cover the language, and never assigned to another rule (BR4.4).
pub const SKL001: RuleId = RuleId("SKL001");

/// A byte range `[start, end)` in the canonical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// First byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

impl Span {
    /// The range `[start, end)`.
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

/// Where a diagnostic points: module, definition ID, anchor path and span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    /// Module qname.
    pub module: String,
    /// Definition ID, `#` plus six base32 characters (`#000000` when the
    /// module has no definition).
    pub def: String,
    /// Anchor path inside the definition (empty for a module-level problem).
    pub path: String,
    /// Byte range in the canonical text.
    pub span: Span,
}

/// Diagnostic level. The skeleton emits only errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Error: the program is invalid; always blocks.
    E,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::E => "E",
        }
    }
}

/// One problem found in a module (contract C3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Rule identifier.
    pub rule: RuleId,
    /// Level; always `E` in the skeleton.
    pub level: Level,
    /// Location.
    pub loc: Location,
    /// Human-readable message.
    pub message: String,
}

impl Diagnostic {
    /// An error diagnostic.
    pub fn error(rule: RuleId, loc: Location, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            rule,
            level: Level::E,
            loc,
            message: message.into(),
        }
    }

    /// The C3 JSON object, fields in schema order. Confidence is 1.0 because
    /// every skeleton rule is sound.
    pub fn to_json(&self) -> Value {
        let span = Value::Array(vec![
            Value::Number(Number::from_i64(saturating_i64(self.loc.span.start))),
            Value::Number(Number::from_i64(saturating_i64(self.loc.span.end))),
        ]);
        Value::object([
            ("rule", Value::str(self.rule.as_str())),
            ("level", Value::str(self.level.as_str())),
            (
                "loc",
                Value::object([
                    ("module", Value::str(&self.loc.module)),
                    ("def", Value::str(&self.loc.def)),
                    ("path", Value::str(&self.loc.path)),
                    ("span", span),
                ]),
            ),
            ("message", Value::str(&self.message)),
            ("confidence", Value::Number(confidence_one())),
        ])
    }

    /// One line for humans: `error RULE module #def/path [start,end]: message`.
    pub fn render(&self) -> String {
        format!(
            "error {} {} {}/{} [{},{}]: {}",
            self.rule.as_str(),
            self.loc.module,
            self.loc.def,
            self.loc.path,
            self.loc.span.start,
            self.loc.span.end,
            self.message
        )
    }

    /// The deterministic order of BR4.3: (module, def, path, span, rule).
    pub fn order(&self, other: &Diagnostic) -> Ordering {
        (
            &self.loc.module,
            &self.loc.def,
            &self.loc.path,
            self.loc.span,
            self.rule,
        )
            .cmp(&(
                &other.loc.module,
                &other.loc.def,
                &other.loc.path,
                other.loc.span,
                other.rule,
            ))
    }
}

fn saturating_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn confidence_one() -> Number {
    Number::from_lexeme("1.0").unwrap_or_else(|| Number::from_i64(1))
}

/// The outcome of checking one module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckResult {
    /// Module qname.
    pub module: String,
    /// Diagnostics in BR4.3 order.
    pub diagnostics: Vec<Diagnostic>,
    /// True exactly when a diagnostic is present (BR4.2).
    pub blocking: bool,
}

impl CheckResult {
    /// Sorts the diagnostics and derives `blocking`.
    pub fn new(module: impl Into<String>, mut diagnostics: Vec<Diagnostic>) -> CheckResult {
        diagnostics.sort_by(Diagnostic::order);
        diagnostics.dedup();
        let blocking = !diagnostics.is_empty();
        CheckResult {
            module: module.into(),
            diagnostics,
            blocking,
        }
    }

    /// `{module, diagnostics, blocking}`.
    pub fn to_json(&self) -> Value {
        Value::object([
            ("module", Value::str(&self.module)),
            (
                "diagnostics",
                Value::Array(self.diagnostics.iter().map(Diagnostic::to_json).collect()),
            ),
            ("blocking", Value::Bool(self.blocking)),
        ])
    }
}

/// A registered rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleInfo {
    /// Identifier.
    pub id: RuleId,
    /// One-line summary.
    pub summary: &'static str,
}

impl RuleInfo {
    /// Whether `id` matches the C3 pattern `^[A-Z]{2,4}[0-9]{3}$`.
    pub fn valid_id(id: &str) -> bool {
        let letters = id.bytes().take_while(u8::is_ascii_uppercase).count();
        let rest = id.get(letters..).unwrap_or("");
        (2..=4).contains(&letters) && rest.len() == 3 && rest.bytes().all(|b| b.is_ascii_digit())
    }
}

/// Why a rule could not be registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// The identifier does not match the C3 pattern.
    InvalidId(&'static str),
    /// The identifier is already taken; identifiers are never reused.
    AlreadyRegistered(&'static str),
}

/// The rule registry: a fixed table built once at start-up.
#[derive(Clone, Debug)]
pub struct Registry {
    rules: BTreeMap<&'static str, RuleInfo>,
}

impl Registry {
    /// The skeleton's registry: exactly TY001, FX001 and SKL001 (BR4.1).
    pub fn skeleton() -> Registry {
        let mut registry = Registry {
            rules: BTreeMap::new(),
        };
        for info in [
            RuleInfo {
                id: TY001,
                summary: "type mismatch",
            },
            RuleInfo {
                id: FX001,
                summary: "effect used but not declared in the signature",
            },
            RuleInfo {
                id: SKL001,
                summary: "outside what this toolchain build can check yet (temporary)",
            },
        ] {
            // The three built-in identifiers are valid and distinct.
            let _ = registry.register(info);
        }
        registry
    }

    /// Adds a rule; an identifier is never registered twice (BR4.4).
    pub fn register(&mut self, info: RuleInfo) -> Result<(), RegistryError> {
        let id = info.id.as_str();
        if !RuleInfo::valid_id(id) {
            return Err(RegistryError::InvalidId(id));
        }
        if self.rules.contains_key(id) {
            return Err(RegistryError::AlreadyRegistered(id));
        }
        self.rules.insert(id, info);
        Ok(())
    }

    /// A registered rule.
    pub fn get(&self, id: &str) -> Option<&RuleInfo> {
        self.rules.get(id)
    }

    /// Registered identifiers, sorted.
    pub fn ids(&self) -> Vec<&'static str> {
        self.rules.keys().copied().collect()
    }
}
