//! From S-expressions to the canonical tree (BR1.1, BR1.2, BR1.4–BR1.6,
//! BR2.4, BR2.6).

use std::collections::BTreeSet;

use rail_diag::{Diagnostic, Location, SKL001};

use crate::reader::{ReadError, Sx, check_bytes, lines, read_item};
use crate::{
    Binding, Expr, ExprKind, FnDef, Module, Name, OPERATORS, Pattern, Span, TypeExpr, is_lname,
    is_uname,
};

/// Items the full language has but this toolchain build does not accept yet.
const UNSUPPORTED_ITEMS: [&str; 11] = [
    "use", "typ", "trt", "imp", "val", "tst", "allow", "meta", "ext", "req", "ens",
];

/// Expression forms the full language has but this build does not accept yet.
const UNSUPPORTED_FORMS: [&str; 7] = ["\\", ".", "upd", "!", ",", "@", ":"];

/// Parses canonical text for module `qname` (derived from its path). On
/// failure, returns the first problem as one `SKL001` diagnostic.
pub fn parse(qname: &str, bytes: &[u8]) -> Result<Module, Box<Diagnostic>> {
    let fallback_def = first_def_id(bytes);
    let mut parser = Parser {
        qname,
        def: fallback_def.clone(),
        path: String::new(),
    };
    parser.module(bytes).map_err(|e| {
        Box::new(Diagnostic::error(
            SKL001,
            Location {
                module: qname.to_string(),
                def: parser.def.clone(),
                path: parser.path.clone(),
                span: e.span,
            },
            e.message,
        ))
    })
}

/// The ID of the first `fn` line with a well-formed ID, else `#000000`
/// (BR4.1: problems outside a definition point at the first definition).
fn first_def_id(bytes: &[u8]) -> String {
    for (start, end) in lines(bytes) {
        let line = &bytes[start..end.min(bytes.len())];
        if let Some(rest) = line.strip_prefix(b"(fn ")
            && let Some(id) = rest.get(..7)
            && let Ok(id) = std::str::from_utf8(id)
            && is_defid(id)
            && rest.get(7) == Some(&b' ')
        {
            return id.to_string();
        }
    }
    "#000000".to_string()
}

/// Whether `text` is `#` plus six lowercase Crockford base32 characters.
pub(crate) fn is_defid(text: &str) -> bool {
    text.len() == 7
        && text.starts_with('#')
        && text[1..]
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'h' | b'j' | b'k' | b'm' | b'n' | b'p'..=b't' | b'v'..=b'z'))
}

struct Parser<'q> {
    qname: &'q str,
    /// The definition a diagnostic would point at.
    def: String,
    /// The anchor path a diagnostic would point at.
    path: String,
}

fn child(path: &str, index: usize) -> String {
    format!("{path}.{index}")
}

fn name(sx: &Sx) -> Option<Name> {
    match sx {
        Sx::Atom { text, span } => Some(Name {
            text: text.clone(),
            span: *span,
        }),
        Sx::List { .. } => None,
    }
}

impl Parser<'_> {
    fn module(&mut self, bytes: &[u8]) -> Result<Module, ReadError> {
        check_bytes(bytes)?;
        let first_def = self.def.clone();
        let mut module: Option<Module> = None;
        let mut ids = BTreeSet::new();
        for (index, (start, end)) in lines(bytes).into_iter().enumerate() {
            self.def = first_def.clone();
            self.path = String::new();
            let item = read_item(bytes, start, end)?;
            let Sx::List { items, span } = &item else {
                return Err(ReadError::at(
                    start,
                    "each line must hold one parenthesised item",
                ));
            };
            let head = items.first().and_then(Sx::atom).unwrap_or("");
            match (&mut module, head) {
                (None, "mod") => module = Some(self.mod_item(items, *span)?),
                (None, _) => {
                    return Err(ReadError::at(start, "the first item must be (mod ...)"));
                }
                (Some(_), "mod") => {
                    return Err(ReadError::at(
                        start,
                        "(mod ...) must appear exactly once, first",
                    ));
                }
                (Some(m), "pub") => {
                    if index != 1 {
                        return Err(ReadError::at(
                            start,
                            "(pub ...) must directly follow (mod ...)",
                        ));
                    }
                    m.pub_span = Some(*span);
                    m.exports = self.pub_item(items)?;
                }
                (Some(m), "fn") => {
                    let f = self.fn_item(items, *span, &mut ids)?;
                    if let Some(prev) = m.fns.last()
                        && prev.name.text >= f.name.text
                    {
                        self.def = f.id.text.clone();
                        return Err(ReadError::at(
                            start,
                            format!(
                                "items are not in canonical order: fn {} must come before fn {}",
                                f.name.text, prev.name.text
                            ),
                        ));
                    }
                    m.fns.push(f);
                }
                (Some(_), other) if UNSUPPORTED_ITEMS.contains(&other) => {
                    return Err(ReadError::at(
                        start,
                        format!("item `{other}` is not yet supported in this toolchain build"),
                    ));
                }
                (Some(_), other) => {
                    return Err(ReadError::at(start, format!("unknown item `{other}`")));
                }
            }
        }
        module.ok_or_else(|| ReadError::at(0, "the first item must be (mod ...)"))
    }

    fn mod_item(&mut self, items: &[Sx], span: Span) -> Result<Module, ReadError> {
        let Some(Sx::Atom { text, span: qspan }) = items.get(1).filter(|_| items.len() == 2) else {
            return Err(ReadError::over(span, "expected (mod qname)"));
        };
        if !text.split('.').all(is_lname) {
            return Err(ReadError::over(
                *qspan,
                format!("invalid module name `{text}`"),
            ));
        }
        if text.split('.').next() == Some("skel") {
            return Err(ReadError::over(
                *qspan,
                "the module name skel is reserved for the skeleton's temporary built-in",
            ));
        }
        if text != self.qname {
            return Err(ReadError::over(
                *qspan,
                format!(
                    "module name {text} does not match its path; expected {}",
                    self.qname
                ),
            ));
        }
        Ok(Module {
            qname: text.clone(),
            qname_span: *qspan,
            pub_span: None,
            exports: Vec::new(),
            fns: Vec::new(),
        })
    }

    fn pub_item(&mut self, items: &[Sx]) -> Result<Vec<Name>, ReadError> {
        let mut exports: Vec<Name> = Vec::new();
        for sx in &items[1..] {
            let export = name(sx)
                .filter(|n| is_lname(&n.text) || is_uname(&n.text))
                .ok_or_else(|| ReadError::over(sx.span(), "exports must be names"))?;
            if exports.last().is_some_and(|prev| prev.text >= export.text) {
                return Err(ReadError::over(
                    export.span,
                    "exports must be sorted and unique",
                ));
            }
            exports.push(export);
        }
        Ok(exports)
    }

    fn fn_item(
        &mut self,
        items: &[Sx],
        span: Span,
        ids: &mut BTreeSet<String>,
    ) -> Result<FnDef, ReadError> {
        let id_sx = items
            .get(1)
            .ok_or_else(|| ReadError::over(span, "malformed fn item"))?;
        let id = name(id_sx)
            .filter(|n| n.text.starts_with('#'))
            .ok_or_else(|| ReadError::over(id_sx.span(), "fn item has no definition ID"))?;
        self.path = "0".to_string();
        if !is_defid(&id.text) {
            return Err(ReadError::over(
                id.span,
                format!(
                    "invalid definition ID `{}`; expected # and 6 lowercase base32 characters",
                    id.text
                ),
            ));
        }
        self.def = id.text.clone();
        if !ids.insert(id.text.clone()) {
            return Err(ReadError::over(
                id.span,
                format!("duplicate definition ID {}", id.text),
            ));
        }
        if !(5..=6).contains(&items.len()) {
            self.path = String::new();
            return Err(ReadError::over(
                span,
                "malformed fn item; expected (fn #id name (params) (: type) body)",
            ));
        }
        self.path = "1".to_string();
        let fname = name(&items[2])
            .filter(|n| is_lname(&n.text))
            .ok_or_else(|| ReadError::over(items[2].span(), "invalid function name"))?;
        self.path = "2".to_string();
        let Sx::List {
            items: param_items, ..
        } = &items[3]
        else {
            return Err(ReadError::over(
                items[3].span(),
                "expected a parameter list",
            ));
        };
        let params = param_items
            .iter()
            .enumerate()
            .map(|(i, sx)| self.pattern(sx, &child("2", i)))
            .collect::<Result<Vec<_>, _>>()?;
        let (sig, body_sx, body_path) = if items.len() == 6 {
            self.path = "3".to_string();
            let ty = match &items[4] {
                Sx::List { items: sig, .. } if sig.len() == 2 && sig[0].atom() == Some(":") => {
                    self.type_expr(&sig[1], "3.0")?
                }
                other => {
                    return Err(ReadError::over(
                        other.span(),
                        "expected a signature (: type)",
                    ));
                }
            };
            (Some(ty), &items[5], "4")
        } else {
            (None, &items[4], "3")
        };
        let body = self.expr(body_sx, body_path)?;
        Ok(FnDef {
            id,
            name: fname,
            params,
            sig,
            body,
            span,
        })
    }

    fn pattern(&mut self, sx: &Sx, path: &str) -> Result<Pattern, ReadError> {
        self.path = path.to_string();
        match name(sx) {
            Some(n) if n.text == "_" => Ok(Pattern::Wildcard(n.span)),
            Some(n) if is_lname(&n.text) => Ok(Pattern::Name(n)),
            _ => Err(ReadError::over(
                sx.span(),
                "patterns other than a name or _ are not yet supported in this toolchain build",
            )),
        }
    }

    fn type_expr(&mut self, sx: &Sx, path: &str) -> Result<TypeExpr, ReadError> {
        self.path = path.to_string();
        match sx {
            Sx::Atom { text, span } => {
                let valid = text.bytes().next().is_some_and(|b| b.is_ascii_alphabetic())
                    && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
                if !valid {
                    return Err(ReadError::over(*span, format!("invalid type `{text}`")));
                }
                Ok(TypeExpr::Name {
                    name: Name {
                        text: text.clone(),
                        span: *span,
                    },
                    path: path.to_string(),
                })
            }
            Sx::List { items, span } => match items.first() {
                Some(Sx::Atom { text, .. }) if text == "->" => {
                    let Some(Sx::List {
                        items: params,
                        span: params_span,
                    }) = items.get(1)
                    else {
                        return Err(ReadError::over(
                            *span,
                            "expected (-> (params) result effects...)",
                        ));
                    };
                    let Some(result_sx) = items.get(2) else {
                        return Err(ReadError::over(
                            *span,
                            "expected (-> (params) result effects...)",
                        ));
                    };
                    let params_path = child(path, 0);
                    let params = params
                        .iter()
                        .enumerate()
                        .map(|(i, p)| self.type_expr(p, &child(&params_path, i)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let result = self.type_expr(result_sx, &child(path, 1))?;
                    let mut effects = Vec::new();
                    for (i, sx) in items.iter().enumerate().skip(3) {
                        self.path = child(path, i - 1);
                        effects.push(
                            name(sx).filter(|n| is_lname(&n.text)).ok_or_else(|| {
                                ReadError::over(sx.span(), "invalid effect label")
                            })?,
                        );
                    }
                    Ok(TypeExpr::Func {
                        params,
                        params_span: *params_span,
                        result: Box::new(result),
                        effects,
                        span: *span,
                        path: path.to_string(),
                    })
                }
                Some(Sx::Atom {
                    text,
                    span: head_span,
                }) if is_uname(text) && items.len() >= 2 => {
                    let args = items
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, a)| self.type_expr(a, &child(path, i)))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(TypeExpr::Apply {
                        head: Name {
                            text: text.clone(),
                            span: *head_span,
                        },
                        args,
                        span: *span,
                        path: path.to_string(),
                    })
                }
                Some(Sx::Atom { text, .. }) if text == "," || text == "=>" => Err(ReadError::over(
                    *span,
                    format!("type form `{text}` is not yet supported in this toolchain build"),
                )),
                _ => Err(ReadError::over(*span, "invalid type")),
            },
        }
    }

    fn expr(&mut self, sx: &Sx, path: &str) -> Result<Expr, ReadError> {
        self.path = path.to_string();
        let span = sx.span();
        let kind = match sx {
            Sx::Atom { text, span } => self.atom_expr(text, *span)?,
            Sx::List { items, .. } if items.is_empty() => ExprKind::Unit,
            Sx::List { items, .. } => match &items[0] {
                Sx::Atom { text, .. } if text == "let" => self.let_expr(items, span, path)?,
                Sx::Atom { text, .. } if text == "?" => self.match_expr(items, span, path)?,
                Sx::Atom { text, .. } if UNSUPPORTED_FORMS.contains(&text.as_str()) => {
                    return Err(ReadError::over(
                        span,
                        format!("form `{text}` is not yet supported in this toolchain build"),
                    ));
                }
                Sx::Atom { text, .. } if is_reference(text) => {
                    if items.len() < 2 {
                        return Err(ReadError::over(
                            span,
                            "an application needs at least one argument",
                        ));
                    }
                    let head = self.expr(&items[0], &child(path, 0))?;
                    let args = items
                        .iter()
                        .enumerate()
                        .skip(1)
                        .map(|(i, a)| self.expr(a, &child(path, i)))
                        .collect::<Result<Vec<_>, _>>()?;
                    ExprKind::Apply {
                        head: Box::new(head),
                        args,
                    }
                }
                _ => {
                    return Err(ReadError::over(
                        span,
                        "applying anything but a name is not yet supported in this toolchain build",
                    ));
                }
            },
        };
        Ok(Expr {
            kind,
            span,
            path: path.to_string(),
        })
    }

    fn atom_expr(&mut self, text: &str, span: Span) -> Result<ExprKind, ReadError> {
        let starts_number = text.bytes().next().is_some_and(|b| b.is_ascii_digit())
            || (text.starts_with('-') && text.as_bytes().get(1).is_some_and(u8::is_ascii_digit));
        if starts_number {
            return int_literal(text)
                .map(ExprKind::Int)
                .map_err(|m| ReadError::over(span, m));
        }
        match text {
            "true" => Ok(ExprKind::Bool(true)),
            "false" => Ok(ExprKind::Bool(false)),
            "_" => Err(ReadError::over(span, "`_` is not an expression")),
            t if t.starts_with('"') => Err(ReadError::over(
                span,
                "string literals are not yet supported in this toolchain build",
            )),
            t if is_reference(t) => Ok(ExprKind::Ref(t.to_string())),
            t => Err(ReadError::over(span, format!("invalid name `{t}`"))),
        }
    }

    fn let_expr(&mut self, items: &[Sx], span: Span, path: &str) -> Result<ExprKind, ReadError> {
        if items.len() < 3 {
            return Err(ReadError::over(
                span,
                "let needs at least one binding and a body",
            ));
        }
        let mut bindings = Vec::new();
        for (i, sx) in items[1..items.len() - 1].iter().enumerate() {
            let pair_path = child(path, i);
            self.path = pair_path.clone();
            let Sx::List {
                items: pair,
                span: pair_span,
            } = sx
            else {
                return Err(ReadError::over(
                    sx.span(),
                    "a let binding is (pattern value)",
                ));
            };
            if pair.len() != 2 {
                return Err(ReadError::over(
                    *pair_span,
                    "a let binding is (pattern value)",
                ));
            }
            let pattern = self.pattern(&pair[0], &child(&pair_path, 0))?;
            let value = self.expr(&pair[1], &child(&pair_path, 1))?;
            bindings.push(Binding {
                pattern,
                value,
                path: pair_path,
            });
        }
        let body_sx = &items[items.len() - 1];
        let body_path = child(path, items.len() - 2);
        if let Sx::List {
            items: inner,
            span: inner_span,
        } = body_sx
            && inner.first().and_then(Sx::atom) == Some("let")
        {
            self.path = body_path;
            return Err(ReadError::over(
                *inner_span,
                "a let whose body is a let must be merged into one let",
            ));
        }
        let body = self.expr(body_sx, &body_path)?;
        Ok(ExprKind::Let {
            bindings,
            body: Box::new(body),
        })
    }

    fn match_expr(&mut self, items: &[Sx], span: Span, path: &str) -> Result<ExprKind, ReadError> {
        let (Some(on_true), Some(on_false), 4) = (
            arm(items.get(2), "true"),
            arm(items.get(3), "false"),
            items.len(),
        ) else {
            return Err(ReadError::over(
                span,
                "only the Boolean match (? e (true a) (false b)) is supported in this toolchain build",
            ));
        };
        let scrutinee = self.expr(&items[1], &child(path, 0))?;
        let on_true = self.expr(on_true, &format!("{path}.1.1"))?;
        let on_false = self.expr(on_false, &format!("{path}.2.1"))?;
        Ok(ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            on_true: Box::new(on_true),
            on_false: Box::new(on_false),
        })
    }
}

/// The expression of a match arm `(label expr)`.
fn arm<'a>(sx: Option<&'a Sx>, label: &str) -> Option<&'a Sx> {
    match sx {
        Some(Sx::List { items, .. }) if items.len() == 2 && items[0].atom() == Some(label) => {
            items.get(1)
        }
        _ => None,
    }
}

/// Whether `text` can be a reference: a lowercase name, `alias.name`, an
/// uppercase name or an operator.
fn is_reference(text: &str) -> bool {
    if OPERATORS.contains(&text) || is_lname(text) || is_uname(text) {
        return true;
    }
    match text.split_once('.') {
        Some((alias, member)) => is_lname(alias) && is_lname(member),
        None => false,
    }
}

/// Validates an integer literal: canonical decimal, no suffix, in i64 range.
fn int_literal(text: &str) -> Result<i64, String> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let len = digits.bytes().take_while(u8::is_ascii_digit).count();
    let (number, rest) = digits.split_at(len);
    if !rest.is_empty() {
        let suffixes = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
        return Err(if suffixes.contains(&rest) {
            "integer literal suffixes are not yet supported; integer literals are i64".to_string()
        } else if rest.starts_with(['.', 'e', 'E']) {
            "float literals are not yet supported in this toolchain build".to_string()
        } else {
            format!("malformed integer literal `{text}`")
        });
    }
    if number.len() > 1 && number.starts_with('0') {
        return Err("non-canonical integer literal: leading zero".to_string());
    }
    if text == "-0" {
        return Err("non-canonical integer literal: -0 is written 0".to_string());
    }
    text.parse::<i64>()
        .map_err(|_| "integer literal out of the i64 range".to_string())
}
