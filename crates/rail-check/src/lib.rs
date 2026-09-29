//! Name resolution, monomorphic typing and the `log` effect check.
//!
//! Building block: QueryEngine + TypeChecker (contract C2, skeleton subset).
//! Owning unit: U1 walking-skeleton (thin first version; completed by U4).
//!
//! Types are `i64`, `bool`, `unit`, plus `Caps` and `(Result unit E)` for
//! `main` only. Every function has an explicit, exported signature, and `log`
//! is the only effect (BR1.6, BR1.7, BR2.1–BR2.7, BR3.1–BR3.3).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod body;
mod sig;

use std::collections::BTreeMap;
use std::fmt;

use rail_diag::{CheckResult, Diagnostic, Location, RuleId, Span};
use rail_syntax::Module;

/// A skeleton type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// `i64`
    I64,
    /// `bool`
    Bool,
    /// `unit`
    Unit,
    /// The opaque capability record `main` receives.
    Caps,
    /// `(Result ok err)`, only as `main`'s result.
    Result(Box<Ty>, Box<Ty>),
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::I64 => f.write_str("i64"),
            Ty::Bool => f.write_str("bool"),
            Ty::Unit => f.write_str("unit"),
            Ty::Caps => f.write_str("Caps"),
            Ty::Result(ok, err) => write!(f, "(Result {ok} {err})"),
        }
    }
}

/// A checked function signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnType {
    /// Parameter types.
    pub params: Vec<Ty>,
    /// Result type.
    pub result: Ty,
    /// Whether the signature declares `log`.
    pub log: bool,
}

/// A function that checked cleanly (a thin C2 `TypedDefinition`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedFn {
    /// Definition ID.
    pub defid: String,
    /// Function name.
    pub name: String,
    /// Its type; the effect row equals the declared effects.
    pub ty: FnType,
}

/// A module whose check found no diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedModule {
    /// The parsed module.
    pub module: Module,
    /// One entry per function, in module order.
    pub fns: Vec<TypedFn>,
}

impl TypedModule {
    /// Whether the module has an (exported) `main`, so it can be built.
    pub fn has_main(&self) -> bool {
        self.fns.iter().any(|f| f.name == "main")
    }
}

/// The outcome of checking one module.
#[derive(Clone, Debug)]
pub struct Checked {
    /// Sorted diagnostics and the blocking verdict.
    pub result: CheckResult,
    /// The typed module, present exactly when there is no diagnostic.
    pub typed: Option<TypedModule>,
}

/// Parses and checks canonical text for module `qname`.
pub fn check(qname: &str, bytes: &[u8]) -> Checked {
    match rail_syntax::parse(qname, bytes) {
        Ok(module) => check_module(module),
        Err(diagnostic) => Checked {
            result: CheckResult::new(qname, vec![*diagnostic]),
            typed: None,
        },
    }
}

/// Checks a parsed module.
pub fn check_module(module: Module) -> Checked {
    let mut sink = Sink {
        module: module.qname.clone(),
        diagnostics: Vec::new(),
    };
    let first_def = module
        .fns
        .first()
        .map_or_else(|| "#000000".to_string(), |f| f.id.text.clone());
    for export in &module.exports {
        if !module.fns.iter().any(|f| f.name.text == export.text) {
            sink.report(
                rail_diag::SKL001,
                &first_def,
                "",
                export.span,
                format!("{} is exported but not defined", export.text),
            );
        }
    }
    let sigs: Vec<Option<FnType>> = module
        .fns
        .iter()
        .map(|f| sig::check_signature(f, &module, &mut sink))
        .collect();
    let table: BTreeMap<&str, Option<&FnType>> = module
        .fns
        .iter()
        .zip(&sigs)
        .map(|(f, s)| (f.name.text.as_str(), s.as_ref()))
        .collect();
    for (f, s) in module.fns.iter().zip(&sigs) {
        if let Some(ty) = s {
            body::check_body(f, ty, &table, &mut sink);
        }
    }
    let result = CheckResult::new(module.qname.clone(), sink.diagnostics);
    let typed = (!result.blocking).then(|| TypedModule {
        fns: module
            .fns
            .iter()
            .zip(sigs)
            .filter_map(|(f, s)| {
                s.map(|ty| TypedFn {
                    defid: f.id.text.clone(),
                    name: f.name.text.clone(),
                    ty,
                })
            })
            .collect(),
        module,
    });
    Checked { result, typed }
}

/// Collects diagnostics for one module.
pub(crate) struct Sink {
    module: String,
    diagnostics: Vec<Diagnostic>,
}

impl Sink {
    pub(crate) fn report(
        &mut self,
        rule: RuleId,
        def: &str,
        path: &str,
        span: Span,
        message: String,
    ) {
        self.diagnostics.push(Diagnostic::error(
            rule,
            Location {
                module: self.module.clone(),
                def: def.to_string(),
                path: path.to_string(),
                span,
            },
            message,
        ));
    }
}
