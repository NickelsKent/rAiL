//! The JSON form of a parsed module for `tree.get` and `rail parse --json`.

use rail_diag::Span;
use rail_json::Value;
use rail_syntax::{Expr, ExprKind, FnDef, Module, Pattern};

fn span(s: Span) -> Value {
    let n = |x: usize| Value::int(i64::try_from(x).unwrap_or(i64::MAX));
    Value::Array(vec![n(s.start), n(s.end)])
}

fn pattern(p: &Pattern) -> Value {
    match p {
        Pattern::Wildcard(_) => Value::str("_"),
        Pattern::Name(n) => Value::str(&n.text),
    }
}

/// `{qname, exports, definitions}`.
pub(crate) fn tree_to_json(module: &Module) -> Value {
    Value::object([
        ("qname", Value::str(&module.qname)),
        (
            "exports",
            Value::Array(module.exports.iter().map(|e| Value::str(&e.text)).collect()),
        ),
        (
            "definitions",
            Value::Array(module.fns.iter().map(definition).collect()),
        ),
    ])
}

fn definition(f: &FnDef) -> Value {
    Value::object([
        ("defid", Value::str(&f.id.text)),
        ("name", Value::str(&f.name.text)),
        ("kind", Value::str("fn")),
        (
            "params",
            Value::Array(f.params.iter().map(pattern).collect()),
        ),
        (
            "signature",
            f.sig
                .as_ref()
                .map_or(Value::Null, |t| Value::str(&rail_syntax::type_text(t))),
        ),
        ("span", span(f.span)),
        ("body", node(&f.body)),
    ])
}

fn node(e: &Expr) -> Value {
    let mut fields = vec![
        ("form", Value::str(form(&e.kind))),
        ("path", Value::str(&e.path)),
        ("span", span(e.span)),
    ];
    match &e.kind {
        ExprKind::Int(n) => fields.push(("value", Value::int(*n))),
        ExprKind::Bool(b) => fields.push(("value", Value::Bool(*b))),
        ExprKind::Unit => {}
        ExprKind::Ref(name) => fields.push(("name", Value::str(name))),
        ExprKind::Apply { head, args } => {
            let children = std::iter::once(node(head))
                .chain(args.iter().map(node))
                .collect();
            fields.push(("children", Value::Array(children)));
        }
        ExprKind::Let { bindings, body } => {
            let bindings = bindings
                .iter()
                .map(|b| {
                    Value::object([
                        ("pattern", pattern(&b.pattern)),
                        ("path", Value::str(&b.path)),
                        ("value", node(&b.value)),
                    ])
                })
                .collect();
            fields.push(("bindings", Value::Array(bindings)));
            fields.push(("body", node(body)));
        }
        ExprKind::Match {
            scrutinee,
            on_true,
            on_false,
        } => {
            fields.push(("scrutinee", node(scrutinee)));
            fields.push(("on_true", node(on_true)));
            fields.push(("on_false", node(on_false)));
        }
    }
    Value::object(fields)
}

fn form(kind: &ExprKind) -> &'static str {
    match kind {
        ExprKind::Int(_) => "int_literal",
        ExprKind::Bool(_) => "bool_literal",
        ExprKind::Unit => "unit_literal",
        ExprKind::Ref(_) => "reference",
        ExprKind::Apply { .. } => "application",
        ExprKind::Let { .. } => "let",
        ExprKind::Match { .. } => "bool_match",
    }
}
