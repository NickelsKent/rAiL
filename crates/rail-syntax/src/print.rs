//! The canonical printer: `print(parse(bytes)) == bytes` (BR1.3).

use crate::{Expr, ExprKind, FnDef, Module, Pattern, TypeExpr};

/// Prints a module in canonical text.
pub fn print(module: &Module) -> String {
    let mut out = format!("(mod {})\n", module.qname);
    if module.pub_span.is_some() {
        out.push_str("(pub");
        for export in &module.exports {
            out.push(' ');
            out.push_str(&export.text);
        }
        out.push_str(")\n");
    }
    for f in &module.fns {
        print_fn(f, &mut out);
        out.push('\n');
    }
    out
}

fn print_fn(f: &FnDef, out: &mut String) {
    out.push_str("(fn ");
    out.push_str(&f.id.text);
    out.push(' ');
    out.push_str(&f.name.text);
    out.push_str(" (");
    for (i, p) in f.params.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        print_pattern(p, out);
    }
    out.push_str(") ");
    if let Some(sig) = &f.sig {
        out.push_str("(: ");
        print_type(sig, out);
        out.push_str(") ");
    }
    print_expr(&f.body, out);
    out.push(')');
}

fn print_pattern(p: &Pattern, out: &mut String) {
    match p {
        Pattern::Wildcard(_) => out.push('_'),
        Pattern::Name(n) => out.push_str(&n.text),
    }
}

fn print_type(t: &TypeExpr, out: &mut String) {
    match t {
        TypeExpr::Name { name, .. } => out.push_str(&name.text),
        TypeExpr::Apply { head, args, .. } => {
            out.push('(');
            out.push_str(&head.text);
            for a in args {
                out.push(' ');
                print_type(a, out);
            }
            out.push(')');
        }
        TypeExpr::Func {
            params,
            result,
            effects,
            ..
        } => {
            out.push_str("(-> (");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                print_type(p, out);
            }
            out.push_str(") ");
            print_type(result, out);
            for e in effects {
                out.push(' ');
                out.push_str(&e.text);
            }
            out.push(')');
        }
    }
}

/// Canonical text of a signature type, e.g. `(-> (i64) i64)`.
pub fn type_text(t: &TypeExpr) -> String {
    let mut out = String::new();
    print_type(t, &mut out);
    out
}

fn print_expr(e: &Expr, out: &mut String) {
    match &e.kind {
        ExprKind::Int(n) => out.push_str(&n.to_string()),
        ExprKind::Bool(true) => out.push_str("true"),
        ExprKind::Bool(false) => out.push_str("false"),
        ExprKind::Unit => out.push_str("()"),
        ExprKind::Ref(name) => out.push_str(name),
        ExprKind::Apply { head, args } => {
            out.push('(');
            print_expr(head, out);
            for a in args {
                out.push(' ');
                print_expr(a, out);
            }
            out.push(')');
        }
        ExprKind::Let { bindings, body } => {
            out.push_str("(let");
            for b in bindings {
                out.push_str(" (");
                print_pattern(&b.pattern, out);
                out.push(' ');
                print_expr(&b.value, out);
                out.push(')');
            }
            out.push(' ');
            print_expr(body, out);
            out.push(')');
        }
        ExprKind::Match {
            scrutinee,
            on_true,
            on_false,
        } => {
            out.push_str("(? ");
            print_expr(scrutinee, out);
            out.push_str(" (true ");
            print_expr(on_true, out);
            out.push_str(") (false ");
            print_expr(on_false, out);
            out.push_str("))");
        }
    }
}
