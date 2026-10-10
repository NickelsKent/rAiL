//! A small std-only reader for the TOML subset the corpus files use:
//! comments, `key = value` with strings, integers, booleans, arrays (also
//! over several lines) and inline tables, `[table]` and `[[array]]` headers
//! with dotted names. No dotted keys, dates, floats or multi-line strings.

use std::collections::BTreeMap;
use std::fmt;

/// A parsed TOML value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    Array(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

/// A syntax error, with its 1-based line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TomlError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    /// The value under `key` when this is a table.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_table().and_then(|t| t.get(key))
    }
}

type Table = BTreeMap<String, Value>;

/// Parses `text`; the result is a table.
pub fn parse(text: &str) -> Result<Value, TomlError> {
    let mut root = Table::new();
    let mut cursor: Vec<String> = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    while let Some((idx, raw)) = lines.next() {
        let line_no = idx + 1;
        let err = |message: &str| TomlError {
            line: line_no,
            message: message.to_string(),
        };
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("[[") {
            let name = rest
                .strip_suffix("]]")
                .ok_or_else(|| err("unterminated [[header]]"))?;
            let path = header_path(name).ok_or_else(|| err("bad table name"))?;
            let (last, parent) = path.split_last().expect("non-empty path");
            let table = descend(&mut root, parent).ok_or_else(|| err("bad table path"))?;
            match table
                .entry(last.clone())
                .or_insert_with(|| Value::Array(Vec::new()))
            {
                Value::Array(items) => items.push(Value::Table(Table::new())),
                _ => return Err(err("name is already used by a non-array value")),
            }
            cursor = path;
        } else if let Some(rest) = line.strip_prefix('[') {
            let name = rest
                .strip_suffix(']')
                .ok_or_else(|| err("unterminated [header]"))?;
            let path = header_path(name).ok_or_else(|| err("bad table name"))?;
            let (last, parent) = path.split_last().expect("non-empty path");
            let table = descend(&mut root, parent).ok_or_else(|| err("bad table path"))?;
            if table.contains_key(last) {
                return Err(err("table defined twice"));
            }
            table.insert(last.clone(), Value::Table(Table::new()));
            cursor = path;
        } else {
            let (key, value_text) = line
                .split_once('=')
                .ok_or_else(|| err("expected `key = value`"))?;
            let key = parse_key(key.trim()).ok_or_else(|| err("bad key"))?;
            let mut value_text = value_text.trim().to_string();
            while unbalanced(&value_text) {
                match lines.next() {
                    Some((_, more)) => {
                        value_text.push(' ');
                        value_text.push_str(strip_comment(more).trim());
                    }
                    None => return Err(err("unterminated value")),
                }
            }
            let mut p = Parser {
                chars: value_text.chars().collect(),
                pos: 0,
            };
            let value = p.value().map_err(|m| err(&m))?;
            p.skip_ws();
            if p.pos != p.chars.len() {
                return Err(err("unexpected text after value"));
            }
            let table = descend(&mut root, &cursor).ok_or_else(|| err("bad table path"))?;
            if table.insert(key, value).is_some() {
                return Err(err("duplicate key"));
            }
        }
    }
    Ok(Value::Table(root))
}

/// The table at `path`, entering the last element of any array on the way.
fn descend<'a>(root: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
    let mut table = root;
    for key in path {
        let slot = table
            .entry(key.clone())
            .or_insert_with(|| Value::Table(Table::new()));
        table = match slot {
            Value::Table(t) => t,
            Value::Array(items) => match items.last_mut() {
                Some(Value::Table(t)) => t,
                _ => return None,
            },
            _ => return None,
        };
    }
    Some(table)
}

fn header_path(name: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = name
        .split('.')
        .map(|p| parse_key(p.trim()))
        .collect::<Option<_>>()?;
    if parts.is_empty() { None } else { Some(parts) }
}

fn parse_key(key: &str) -> Option<String> {
    if key.len() >= 2 && key.starts_with('"') && key.ends_with('"') {
        return Some(key[1..key.len() - 1].to_string());
    }
    let ok = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    ok.then(|| key.to_string())
}

/// Removes a trailing `# comment` that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match in_str {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    in_str = None;
                }
            }
            None => match c {
                '"' | '\'' => in_str = Some(c),
                '#' => return &line[..i],
                _ => {}
            },
        }
    }
    line
}

/// Whether brackets or braces opened in `text` are still open.
fn unbalanced(text: &str) -> bool {
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        match in_str {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    in_str = None;
                }
            }
            None => match c {
                '"' | '\'' => in_str = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                _ => {}
            },
        }
    }
    depth > 0
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn skip_ws(&mut self) {
        while self.chars.get(self.pos).is_some_and(|c| c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some('"') => self.basic_string().map(Value::Str),
            Some('\'') => self.literal_string().map(Value::Str),
            Some('[') => self.array(),
            Some('{') => self.inline_table(),
            Some(c) if c == '-' || c == '+' || c.is_ascii_digit() => self.integer(),
            Some(_) => self.keyword(),
            None => Err("missing value".to_string()),
        }
    }

    fn keyword(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.pos += 1;
        }
        match self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .as_str()
        {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err("unknown value".to_string()),
        }
    }

    fn integer(&mut self) -> Result<Value, String> {
        let start = self.pos;
        self.pos += 1;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos]
            .iter()
            .filter(|c| **c != '_')
            .collect();
        text.parse::<i64>()
            .map(Value::Int)
            .map_err(|_| format!("bad integer `{text}`"))
    }

    fn basic_string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err("unterminated string".to_string()),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('r') => out.push('\r'),
                        _ => return Err("unsupported escape".to_string()),
                    }
                    self.pos += 1;
                }
                Some(c) => {
                    out.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    fn literal_string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '\'' {
                let s = self.chars[start..self.pos].iter().collect();
                self.pos += 1;
                return Ok(s);
            }
            self.pos += 1;
        }
        Err("unterminated string".to_string())
    }

    fn array(&mut self) -> Result<Value, String> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                None => return Err("unterminated array".to_string()),
                _ => {}
            }
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some(']') => {}
                _ => return Err("expected `,` or `]`".to_string()),
            }
        }
    }

    fn inline_table(&mut self) -> Result<Value, String> {
        self.pos += 1;
        let mut table = Table::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('}') => {
                    self.pos += 1;
                    return Ok(Value::Table(table));
                }
                None => return Err("unterminated inline table".to_string()),
                _ => {}
            }
            let start = self.pos;
            while self.peek().is_some_and(|c| c != '=' && c != '}') {
                self.pos += 1;
            }
            let key_text: String = self.chars[start..self.pos].iter().collect();
            let key = parse_key(key_text.trim()).ok_or("bad inline table key")?;
            if self.peek() != Some('=') {
                return Err("expected `=` in inline table".to_string());
            }
            self.pos += 1;
            let value = self.value()?;
            if table.insert(key, value).is_some() {
                return Err("duplicate key".to_string());
            }
            self.skip_ws();
            match self.peek() {
                Some(',') => self.pos += 1,
                Some('}') => {}
                _ => return Err("expected `,` or `}`".to_string()),
            }
        }
    }
}
