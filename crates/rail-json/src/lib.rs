//! Strict RFC 8259 JSON reader and writer for the RAP protocol boundary.
//!
//! Building block: RapServer support (JSON). Owning unit: U1 walking-skeleton (thin
//! first version; deepened by U5 agent-loop).
//!
//! The reader never panics and never recurses: containers are tracked on an
//! explicit work stack whose depth is capped at [`MAX_DEPTH`] (NFR4.3,
//! NFR4.4). Objects keep their fields in the order they were read or built,
//! and the writer emits them in that order with no whitespace, so identical
//! values always produce identical bytes (NFR3.1).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeSet;
use std::fmt;

/// The deepest container nesting the reader accepts.
pub const MAX_DEPTH: usize = 64;

/// A JSON value. Objects are ordered lists of fields with unique keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `null`
    Null,
    /// `true` or `false`
    Bool(bool),
    /// A number, kept as its validated lexeme.
    Number(Number),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object, in field order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// A string value.
    pub fn str(text: &str) -> Value {
        Value::String(text.to_string())
    }

    /// An integer value.
    pub fn int(n: i64) -> Value {
        Value::Number(Number::from_i64(n))
    }

    /// An object from `(key, value)` pairs, in the given order.
    pub fn object<'a>(fields: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
        Value::Object(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    /// The field `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The text of a string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The value of an integer number that fits in `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// The value of a Boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The items of an array.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The fields of an object.
    pub fn as_object(&self) -> Option<&Vec<(String, Value)>> {
        match self {
            Value::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// Whether this is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

/// A JSON number, stored as its exact RFC 8259 lexeme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Number(String);

impl Number {
    /// The number `n`.
    pub fn from_i64(n: i64) -> Number {
        Number(n.to_string())
    }

    /// A number from its lexeme, if the lexeme is valid JSON number syntax.
    pub fn from_lexeme(lexeme: &str) -> Option<Number> {
        (number_len(lexeme.as_bytes(), 0) == Some(lexeme.len())).then(|| Number(lexeme.to_string()))
    }

    /// The lexeme.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The value, if the lexeme is an integer that fits in `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        if self.0.bytes().any(|b| matches!(b, b'.' | b'e' | b'E')) {
            return None;
        }
        self.0.parse().ok()
    }
}

/// Why the reader rejected its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The input is not valid UTF-8.
    InvalidUtf8,
    /// The input ended inside a value.
    UnexpectedEnd,
    /// A byte that cannot appear here.
    UnexpectedByte,
    /// Non-whitespace after the value.
    TrailingData,
    /// Containers nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// An object key that appeared earlier in the same object.
    DuplicateKey,
    /// A bad escape sequence in a string.
    InvalidEscape,
    /// A control character inside a string.
    ControlCharacter,
    /// A number that does not follow the JSON grammar.
    InvalidNumber,
}

/// A located reader failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset of the failure.
    pub offset: usize,
    /// What went wrong.
    pub kind: ErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.kind {
            ErrorKind::InvalidUtf8 => "invalid UTF-8",
            ErrorKind::UnexpectedEnd => "unexpected end of input",
            ErrorKind::UnexpectedByte => "unexpected character",
            ErrorKind::TrailingData => "trailing data after the value",
            ErrorKind::TooDeep => "nesting deeper than 64 levels",
            ErrorKind::DuplicateKey => "duplicate object key",
            ErrorKind::InvalidEscape => "invalid escape sequence",
            ErrorKind::ControlCharacter => "unescaped control character in string",
            ErrorKind::InvalidNumber => "invalid number",
        };
        write!(f, "{what} at byte {}", self.offset)
    }
}

impl std::error::Error for ParseError {}

/// Parses one JSON text strictly (RFC 8259).
pub fn parse(input: &[u8]) -> Result<Value, ParseError> {
    if let Err(e) = std::str::from_utf8(input) {
        return Err(ParseError {
            offset: e.valid_up_to(),
            kind: ErrorKind::InvalidUtf8,
        });
    }
    let mut reader = Reader {
        bytes: input,
        pos: 0,
    };
    let value = reader.value()?;
    reader.skip_ws();
    if reader.pos < input.len() {
        return Err(reader.error(ErrorKind::TrailingData));
    }
    Ok(value)
}

/// Writes `value` as compact JSON: no whitespace, fields in stored order.
pub fn to_string(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => out.push_str(n.as_str()),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(fields) => {
            out.push('{');
            for (i, (key, item)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(item, out);
            }
            out.push('}');
        }
    }
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// An open container on the reader's work stack.
enum Frame {
    Array(Vec<Value>),
    Object {
        fields: Vec<(String, Value)>,
        keys: BTreeSet<String>,
        key: String,
    },
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn error(&self, kind: ErrorKind) -> ParseError {
        ParseError {
            offset: self.pos,
            kind,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == byte => {
                self.pos += 1;
                Ok(())
            }
            Some(_) => Err(self.error(ErrorKind::UnexpectedByte)),
            None => Err(self.error(ErrorKind::UnexpectedEnd)),
        }
    }

    /// Reads one complete value, containers included, with an explicit stack.
    fn value(&mut self) -> Result<Value, ParseError> {
        let mut stack: Vec<Frame> = Vec::new();
        loop {
            self.skip_ws();
            let mut done = match self.peek() {
                Some(open @ (b'[' | b'{')) => {
                    if stack.len() >= MAX_DEPTH {
                        return Err(self.error(ErrorKind::TooDeep));
                    }
                    self.pos += 1;
                    self.skip_ws();
                    let close = if open == b'[' { b']' } else { b'}' };
                    if self.peek() == Some(close) {
                        self.pos += 1;
                        if open == b'[' {
                            Value::Array(Vec::new())
                        } else {
                            Value::Object(Vec::new())
                        }
                    } else if open == b'[' {
                        stack.push(Frame::Array(Vec::new()));
                        continue;
                    } else {
                        let mut keys = BTreeSet::new();
                        let key = self.key(&mut keys)?;
                        stack.push(Frame::Object {
                            fields: Vec::new(),
                            keys,
                            key,
                        });
                        continue;
                    }
                }
                _ => self.scalar()?,
            };
            // Attach the finished value to its parent, closing containers.
            loop {
                let Some(frame) = stack.last_mut() else {
                    return Ok(done);
                };
                let closed = match frame {
                    Frame::Array(items) => {
                        items.push(done);
                        self.skip_ws();
                        match self.peek() {
                            Some(b',') => {
                                self.pos += 1;
                                None
                            }
                            Some(b']') => {
                                self.pos += 1;
                                Some(Value::Array(std::mem::take(items)))
                            }
                            Some(_) => return Err(self.error(ErrorKind::UnexpectedByte)),
                            None => return Err(self.error(ErrorKind::UnexpectedEnd)),
                        }
                    }
                    Frame::Object { fields, keys, key } => {
                        fields.push((std::mem::take(key), done));
                        self.skip_ws();
                        match self.peek() {
                            Some(b',') => {
                                self.pos += 1;
                                *key = self.key(keys)?;
                                None
                            }
                            Some(b'}') => {
                                self.pos += 1;
                                Some(Value::Object(std::mem::take(fields)))
                            }
                            Some(_) => return Err(self.error(ErrorKind::UnexpectedByte)),
                            None => return Err(self.error(ErrorKind::UnexpectedEnd)),
                        }
                    }
                };
                match closed {
                    Some(value) => {
                        stack.pop();
                        done = value;
                    }
                    None => break,
                }
            }
        }
    }

    /// Reads `"key" :`, rejecting a key already present in the object.
    fn key(&mut self, keys: &mut BTreeSet<String>) -> Result<String, ParseError> {
        self.skip_ws();
        let start = self.pos;
        if self.peek() != Some(b'"') {
            return Err(self.error(if self.peek().is_none() {
                ErrorKind::UnexpectedEnd
            } else {
                ErrorKind::UnexpectedByte
            }));
        }
        let key = self.string()?;
        if !keys.insert(key.clone()) {
            return Err(ParseError {
                offset: start,
                kind: ErrorKind::DuplicateKey,
            });
        }
        self.skip_ws();
        self.expect(b':')?;
        Ok(key)
    }

    fn scalar(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => {
                let len = number_len(self.bytes, self.pos)
                    .ok_or_else(|| self.error(ErrorKind::InvalidNumber))?;
                let lexeme = self
                    .bytes
                    .get(self.pos..self.pos + len)
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .ok_or_else(|| self.error(ErrorKind::InvalidNumber))?;
                self.pos += len;
                Ok(Value::Number(Number(lexeme.to_string())))
            }
            Some(_) => Err(self.error(ErrorKind::UnexpectedByte)),
            None => Err(self.error(ErrorKind::UnexpectedEnd)),
        }
    }

    fn literal(&mut self, word: &[u8], value: Value) -> Result<Value, ParseError> {
        if self.bytes.get(self.pos..self.pos + word.len()) == Some(word) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.error(ErrorKind::UnexpectedByte))
        }
    }

    /// Reads a string starting at its opening quote.
    fn string(&mut self) -> Result<String, ParseError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            // The whole input is valid UTF-8 and the run stops at an ASCII
            // byte, so this slice is valid UTF-8 too.
            if let Some(run) = self
                .bytes
                .get(start..self.pos)
                .and_then(|b| std::str::from_utf8(b).ok())
            {
                out.push_str(run);
            }
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    out.push(self.escape()?);
                }
                Some(_) => return Err(self.error(ErrorKind::ControlCharacter)),
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            }
        }
    }

    fn escape(&mut self) -> Result<char, ParseError> {
        let c = match self.peek() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => {
                self.pos += 1;
                return self.unicode_escape();
            }
            Some(_) => return Err(self.error(ErrorKind::InvalidEscape)),
            None => return Err(self.error(ErrorKind::UnexpectedEnd)),
        };
        self.pos += 1;
        Ok(c)
    }

    /// Reads the hex digits after `\u`, combining a surrogate pair.
    fn unicode_escape(&mut self) -> Result<char, ParseError> {
        let at = self.pos;
        let first = self.hex4()?;
        let code = match first {
            0xD800..=0xDBFF => {
                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                    return Err(ParseError {
                        offset: at,
                        kind: ErrorKind::InvalidEscape,
                    });
                }
                self.pos += 2;
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(ParseError {
                        offset: at,
                        kind: ErrorKind::InvalidEscape,
                    });
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => {
                return Err(ParseError {
                    offset: at,
                    kind: ErrorKind::InvalidEscape,
                });
            }
            other => other,
        };
        char::from_u32(code).ok_or(ParseError {
            offset: at,
            kind: ErrorKind::InvalidEscape,
        })
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = match self.peek() {
                Some(b) => (b as char)
                    .to_digit(16)
                    .ok_or_else(|| self.error(ErrorKind::InvalidEscape))?,
                None => return Err(self.error(ErrorKind::UnexpectedEnd)),
            };
            value = value * 16 + digit;
            self.pos += 1;
        }
        Ok(value)
    }
}

/// The length of the JSON number starting at `start`, if one starts there.
fn number_len(bytes: &[u8], start: usize) -> Option<usize> {
    let at = |i: usize| bytes.get(i).copied();
    let digits = |mut i: usize| {
        let from = i;
        while matches!(at(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        (i > from).then_some(i)
    };
    let mut i = start;
    if at(i) == Some(b'-') {
        i += 1;
    }
    i = match at(i) {
        Some(b'0') => i + 1,
        Some(b'1'..=b'9') => digits(i)?,
        _ => return None,
    };
    if at(i) == Some(b'.') {
        i = digits(i + 1)?;
    }
    if matches!(at(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(at(i), Some(b'+' | b'-')) {
            i += 1;
        }
        i = digits(i)?;
    }
    Some(i - start)
}
