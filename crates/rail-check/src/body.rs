//! Body typing: bidirectional checking against the signature (BR2.3, BR2.5,
//! BR2.6), name resolution with no shadowing and no unused bindings (BR1.6,
//! BR1.7), and the `log` effect (BR3.1, BR3.2).

use std::collections::{BTreeMap, BTreeSet};

use rail_diag::{FX001, SKL001, Span, TY001};
use rail_syntax::{Expr, ExprKind, FnDef, OPERATORS, Pattern};

use crate::{FnType, Sink, Ty};

/// The temporary print built-in: `(-> (i64) unit log)`.
const PRINT_BUILTIN: &str = "skel.print_i64";

struct Local {
    name: String,
    ty: Option<Ty>,
    used: bool,
    path: String,
    span: Span,
}

struct Checker<'a> {
    def: &'a str,
    log: bool,
    fns: &'a BTreeMap<&'a str, Option<&'a FnType>>,
    sink: &'a mut Sink,
    scope: Vec<Local>,
    bound: BTreeSet<String>,
}

/// Checks `f`'s body against its signature type `ty`.
pub(crate) fn check_body(
    f: &FnDef,
    ty: &FnType,
    fns: &BTreeMap<&str, Option<&FnType>>,
    sink: &mut Sink,
) {
    let mut checker = Checker {
        def: &f.id.text,
        log: ty.log,
        fns,
        sink,
        scope: Vec::new(),
        bound: BTreeSet::new(),
    };
    for (i, (p, t)) in f.params.iter().zip(&ty.params).enumerate() {
        checker.bind(p, &format!("2.{i}"), Some(t.clone()));
    }
    checker.check(&f.body, &ty.result);
    checker.release(0);
}

impl Checker<'_> {
    fn report(&mut self, rule: rail_diag::RuleId, e: &Expr, message: String) {
        self.sink.report(rule, self.def, &e.path, e.span, message);
    }

    fn mismatch(&mut self, e: &Expr, expected: &Ty, found: &Ty) {
        self.report(TY001, e, format!("expected {expected}, found {found}"));
    }

    /// Binds a pattern; a name bound earlier in this definition is an error.
    fn bind(&mut self, pattern: &Pattern, path: &str, ty: Option<Ty>) {
        let Pattern::Name(name) = pattern else {
            return;
        };
        let mut used = false;
        if !self.bound.insert(name.text.clone()) {
            self.sink.report(
                SKL001,
                self.def,
                path,
                name.span,
                format!("{} is already bound in this definition", name.text),
            );
            if let Some(earlier) = self.scope.iter_mut().rev().find(|l| l.name == name.text) {
                earlier.used = true;
            }
            used = true;
        }
        self.scope.push(Local {
            name: name.text.clone(),
            ty,
            used,
            path: path.to_string(),
            span: name.span,
        });
    }

    /// Leaves scope down to `depth` locals, reporting unused bindings.
    fn release(&mut self, depth: usize) {
        while self.scope.len() > depth {
            let Some(local) = self.scope.pop() else {
                return;
            };
            if !local.used {
                self.sink.report(
                    SKL001,
                    self.def,
                    &local.path,
                    local.span,
                    format!("{} is never used; name it _", local.name),
                );
            }
        }
    }

    fn lookup(&mut self, name: &str) -> Option<Option<Ty>> {
        let local = self.scope.iter_mut().rev().find(|l| l.name == name)?;
        local.used = true;
        Some(local.ty.clone())
    }

    /// Checks `e` against `expected`.
    fn check(&mut self, e: &Expr, expected: &Ty) {
        match &e.kind {
            ExprKind::Match {
                scrutinee,
                on_true,
                on_false,
            } => {
                self.check(scrutinee, &Ty::Bool);
                self.check(on_true, expected);
                self.check(on_false, expected);
            }
            ExprKind::Let { bindings, body } => {
                let depth = self.bind_all(bindings);
                self.check(body, expected);
                self.release(depth);
            }
            ExprKind::Apply { head, args } if is_constructor(head) => {
                let Ty::Result(ok, err) = expected else {
                    self.args(args);
                    self.report(TY001, e, format!("expected {expected}, found a Result"));
                    return;
                };
                let payload = if matches!(&head.kind, ExprKind::Ref(n) if n == "Ok") {
                    ok
                } else {
                    err
                };
                if args.len() != 1 {
                    self.args(args);
                    self.report(
                        TY001,
                        e,
                        format!("{} takes 1 argument, found {}", ref_name(head), args.len()),
                    );
                    return;
                }
                self.check(&args[0], payload);
            }
            _ => {
                if let Some(found) = self.infer(e)
                    && &found != expected
                {
                    self.mismatch(e, expected, &found);
                }
            }
        }
    }

    fn bind_all(&mut self, bindings: &[rail_syntax::Binding]) -> usize {
        let depth = self.scope.len();
        for b in bindings {
            let ty = self.infer(&b.value);
            self.bind(&b.pattern, &format!("{}.0", b.path), ty);
        }
        depth
    }

    /// Infers the arguments only for their effect on name use.
    fn args(&mut self, args: &[Expr]) {
        for a in args {
            self.infer(a);
        }
    }

    /// Infers `e`'s type; `None` after an error that makes it unknown.
    fn infer(&mut self, e: &Expr) -> Option<Ty> {
        match &e.kind {
            ExprKind::Int(_) => Some(Ty::I64),
            ExprKind::Bool(_) => Some(Ty::Bool),
            ExprKind::Unit => Some(Ty::Unit),
            ExprKind::Ref(name) => {
                if let Some(ty) = self.lookup(name) {
                    return ty;
                }
                let message = if self.is_callable(name) {
                    "function values are not yet supported in this toolchain build".to_string()
                } else {
                    unknown(name)
                };
                self.report(SKL001, e, message);
                None
            }
            ExprKind::Let { bindings, body } => {
                let depth = self.bind_all(bindings);
                let ty = self.infer(body);
                self.release(depth);
                ty
            }
            ExprKind::Match {
                scrutinee,
                on_true,
                on_false,
            } => {
                self.check(scrutinee, &Ty::Bool);
                match self.infer(on_true) {
                    Some(ty) => {
                        self.check(on_false, &ty);
                        Some(ty)
                    }
                    None => self.infer(on_false),
                }
            }
            ExprKind::Apply { head, args } => self.apply(e, head, args),
        }
    }

    fn is_callable(&self, name: &str) -> bool {
        OPERATORS.contains(&name)
            || name == "Ok"
            || name == "Err"
            || name == PRINT_BUILTIN
            || self.fns.contains_key(name)
    }

    fn apply(&mut self, e: &Expr, head: &Expr, args: &[Expr]) -> Option<Ty> {
        let name = ref_name(head);
        if self.lookup(name).is_some() {
            self.args(args);
            self.report(
                SKL001,
                head,
                "calling a local value is not yet supported in this toolchain build".to_string(),
            );
            return None;
        }
        if OPERATORS.contains(&name) {
            return self.operator(e, name, args);
        }
        if name == "Ok" || name == "Err" {
            self.args(args);
            self.report(
                TY001,
                e,
                format!("the Result type of {name} is not known here"),
            );
            return None;
        }
        if name == PRINT_BUILTIN {
            let ty = FnType {
                params: vec![Ty::I64],
                result: Ty::Unit,
                log: true,
            };
            return self.call(e, name, &ty, args);
        }
        match self.fns.get(name).copied() {
            Some(Some(ty)) => self.call(e, name, ty, args),
            Some(None) => {
                self.args(args);
                None
            }
            None => {
                self.args(args);
                let message = if name.starts_with("skel.") {
                    format!("{name} does not exist; skel.print_i64 is the only skel built-in")
                } else {
                    unknown(name)
                };
                self.report(SKL001, head, message);
                None
            }
        }
    }

    fn call(&mut self, e: &Expr, name: &str, ty: &FnType, args: &[Expr]) -> Option<Ty> {
        if args.len() != ty.params.len() {
            self.args(args);
            let noun = if ty.params.len() == 1 {
                "argument"
            } else {
                "arguments"
            };
            self.report(
                TY001,
                e,
                format!(
                    "{name} takes {} {noun}, found {}",
                    ty.params.len(),
                    args.len()
                ),
            );
            return Some(ty.result.clone());
        }
        for (a, p) in args.iter().zip(&ty.params) {
            self.check(a, p);
        }
        if ty.log && !self.log {
            self.report(
                FX001,
                e,
                "this call needs the log effect, but the signature does not declare it".to_string(),
            );
        }
        Some(ty.result.clone())
    }

    fn operator(&mut self, e: &Expr, op: &str, args: &[Expr]) -> Option<Ty> {
        if args.len() != 2 {
            self.args(args);
            self.report(
                TY001,
                e,
                format!("{op} takes 2 arguments, found {}", args.len()),
            );
            return None;
        }
        match op {
            "+" | "-" | "*" | "/" | "%" => {
                self.check(&args[0], &Ty::I64);
                self.check(&args[1], &Ty::I64);
                Some(Ty::I64)
            }
            "<" | "<=" | ">" | ">=" => {
                self.check(&args[0], &Ty::I64);
                self.check(&args[1], &Ty::I64);
                Some(Ty::Bool)
            }
            _ => {
                match self.infer(&args[0]) {
                    Some(t @ (Ty::I64 | Ty::Bool)) => self.check(&args[1], &t),
                    Some(other) => {
                        self.report(
                            TY001,
                            &args[0],
                            format!("{op} compares i64 or bool values, found {other}"),
                        );
                        self.infer(&args[1]);
                    }
                    None => {
                        self.infer(&args[1]);
                    }
                }
                Some(Ty::Bool)
            }
        }
    }
}

fn ref_name(e: &Expr) -> &str {
    match &e.kind {
        ExprKind::Ref(name) => name,
        _ => "",
    }
}

fn is_constructor(head: &Expr) -> bool {
    matches!(&head.kind, ExprKind::Ref(n) if n == "Ok" || n == "Err")
}

fn unknown(name: &str) -> String {
    if name.starts_with("skel.") {
        format!("{name} does not exist; skel.print_i64 is the only skel built-in")
    } else {
        format!("unknown name {name}")
    }
}
