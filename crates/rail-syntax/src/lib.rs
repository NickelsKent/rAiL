//! Canonical-text (`.rlc`) parser and printer for the skeleton form set.
//!
//! Building block: Syntax (contract C1). Owning unit: U1 walking-skeleton (thin
//! first version; completed by U3 syntax).
//!
//! The skeleton accepts only exact canonical text in its small form set:
//! `mod`, `pub`, `fn` with a signature, integer and Boolean literals, `()`,
//! references, application, `let` and the Boolean match. Anything else is a
//! located `SKL001` diagnostic; the parser stops at the first problem and
//! never returns a partial module (BR1.1–BR1.6, BR2.4).
//!
//! **Anchor paths.** Every node carries a path of dot-separated child indexes.
//! A form's children are its elements after the form keyword (`fn`, `let`,
//! `?`, `:`, `->`); every element of an application or type application is a
//! child, head included. In a `fn` item the children are `0` ID, `1` name,
//! `2` parameters, `3` signature and `4` body (`3` when there is no
//! signature). So `4` is a body, `4.2` its third element, `2.0` the first
//! parameter and `3.0.1` the result type of the signature.

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod print;
mod reader;
mod tree;

pub use print::{print, type_text};
pub use rail_diag::Span;
pub use tree::parse;

/// The deepest parenthesis nesting the parser accepts (NFR4.4).
pub const MAX_NESTING: usize = 256;

/// The longest name the parser accepts, in bytes (spec §2.6).
pub const MAX_NAME_BYTES: usize = 32;

/// A name or other atom with its location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    /// The text.
    pub text: String,
    /// Where it is.
    pub span: Span,
}

/// One accepted module (a thin version of C1 `Module`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    /// Module qname.
    pub qname: String,
    /// Location of the qname in the `mod` item.
    pub qname_span: Span,
    /// Location of the `pub` item, when there is one.
    pub pub_span: Option<Span>,
    /// Exported names, sorted.
    pub exports: Vec<Name>,
    /// Functions, sorted by name.
    pub fns: Vec<FnDef>,
}

/// One `fn` item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnDef {
    /// Definition ID, e.g. `#sk0a1b`.
    pub id: Name,
    /// Function name.
    pub name: Name,
    /// Parameters.
    pub params: Vec<Pattern>,
    /// The signature type (the `T` of `(: T)`), when present.
    pub sig: Option<TypeExpr>,
    /// The body.
    pub body: Expr,
    /// The whole item.
    pub span: Span,
}

/// A binding pattern: a name or `_`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// `_`
    Wildcard(Span),
    /// A binding name.
    Name(Name),
}

impl Pattern {
    /// Where the pattern is.
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span) => *span,
            Pattern::Name(name) => name.span,
        }
    }
}

/// A type in a signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExpr {
    /// A type name such as `i64` or `Caps`.
    Name {
        /// The name.
        name: Name,
        /// Anchor path.
        path: String,
    },
    /// A type application such as `(Result unit unit)`.
    Apply {
        /// The constructor.
        head: Name,
        /// The arguments.
        args: Vec<TypeExpr>,
        /// Where it is.
        span: Span,
        /// Anchor path.
        path: String,
    },
    /// A function type `(-> (params) result effects...)`.
    Func {
        /// Parameter types.
        params: Vec<TypeExpr>,
        /// Location of the parameter list.
        params_span: Span,
        /// Result type.
        result: Box<TypeExpr>,
        /// Effect labels.
        effects: Vec<Name>,
        /// Where it is.
        span: Span,
        /// Anchor path.
        path: String,
    },
}

impl TypeExpr {
    /// Anchor path.
    pub fn path(&self) -> &str {
        match self {
            TypeExpr::Name { path, .. }
            | TypeExpr::Apply { path, .. }
            | TypeExpr::Func { path, .. } => path,
        }
    }

    /// Location.
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Name { name, .. } => name.span,
            TypeExpr::Apply { span, .. } | TypeExpr::Func { span, .. } => *span,
        }
    }
}

/// An expression node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    /// What it is.
    pub kind: ExprKind,
    /// Where it is.
    pub span: Span,
    /// Anchor path inside its definition.
    pub path: String,
}

/// The skeleton's seven expression forms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    /// An `i64` literal.
    Int(i64),
    /// `true` or `false`.
    Bool(bool),
    /// `()`
    Unit,
    /// A reference: a local, a function, an operator, `Ok`/`Err` or
    /// `skel.print_i64`.
    Ref(String),
    /// `(f a...)`
    Apply {
        /// The applied reference.
        head: Box<Expr>,
        /// One or more arguments.
        args: Vec<Expr>,
    },
    /// `(let (p e)... body)`
    Let {
        /// Sequential bindings.
        bindings: Vec<Binding>,
        /// The body.
        body: Box<Expr>,
    },
    /// `(? e (true a) (false b))`
    Match {
        /// The Boolean scrutinee.
        scrutinee: Box<Expr>,
        /// The `true` arm's expression.
        on_true: Box<Expr>,
        /// The `false` arm's expression.
        on_false: Box<Expr>,
    },
}

/// One `(pattern value)` pair of a `let`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The pattern (anchor path `<path>.0`).
    pub pattern: Pattern,
    /// The bound value (anchor path `<path>.1`).
    pub value: Expr,
    /// Anchor path of the pair.
    pub path: String,
}

/// Whether `text` is a lowercase name: `[a-z][a-z0-9_]*`, at most 32 bytes.
pub fn is_lname(text: &str) -> bool {
    let mut bytes = text.bytes();
    text.len() <= MAX_NAME_BYTES
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Whether `text` is an uppercase name: `[A-Z][A-Za-z0-9]*`, at most 32 bytes.
pub fn is_uname(text: &str) -> bool {
    let mut bytes = text.bytes();
    text.len() <= MAX_NAME_BYTES
        && bytes.next().is_some_and(|b| b.is_ascii_uppercase())
        && bytes.all(|b| b.is_ascii_alphanumeric())
}

/// The operators the skeleton knows (BR2.5).
pub const OPERATORS: [&str; 11] = ["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">="];
