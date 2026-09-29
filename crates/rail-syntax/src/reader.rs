//! Layout validation and the S-expression layer (BR1.2, BR1.4, NFR4.4).
//!
//! The reader checks canonical layout byte by byte and turns each line into
//! one parenthesised S-expression. It keeps nesting on an explicit stack
//! capped at [`MAX_NESTING`], so no input can exhaust the call stack.

use crate::{MAX_NESTING, Span};

/// A located problem: byte offset, span length and message.
#[derive(Debug)]
pub(crate) struct ReadError {
    pub(crate) span: Span,
    pub(crate) message: String,
}

impl ReadError {
    pub(crate) fn at(offset: usize, message: impl Into<String>) -> ReadError {
        ReadError {
            span: Span::new(offset, offset + 1),
            message: message.into(),
        }
    }

    pub(crate) fn over(span: Span, message: impl Into<String>) -> ReadError {
        ReadError {
            span,
            message: message.into(),
        }
    }
}

/// An S-expression: an atom or a parenthesised list.
#[derive(Debug)]
pub(crate) enum Sx {
    Atom { text: String, span: Span },
    List { items: Vec<Sx>, span: Span },
}

impl Sx {
    pub(crate) fn span(&self) -> Span {
        match self {
            Sx::Atom { span, .. } | Sx::List { span, .. } => *span,
        }
    }

    pub(crate) fn atom(&self) -> Option<&str> {
        match self {
            Sx::Atom { text, .. } => Some(text),
            Sx::List { .. } => None,
        }
    }
}

/// Checks the whole-file layout rules that do not depend on structure:
/// ASCII only (so trivially NFC, and no BOM), no CR, no tabs or other
/// control characters, a final LF and no empty lines.
pub(crate) fn check_bytes(bytes: &[u8]) -> Result<(), ReadError> {
    for (offset, &b) in bytes.iter().enumerate() {
        let problem = match b {
            b'\n' | b' '..=b'~' => continue,
            b'\r' => "carriage return; canonical text uses LF line endings only",
            b'\t' => "tab character; canonical text separates tokens with one space",
            0x80..=0xff => "non-ASCII text is not yet supported in this toolchain build",
            _ => "control character in canonical text",
        };
        return Err(ReadError::at(offset, problem));
    }
    if bytes.is_empty() {
        return Err(ReadError::at(
            0,
            "empty module; the first item must be (mod ...)",
        ));
    }
    if bytes.last() != Some(&b'\n') {
        return Err(ReadError::over(
            Span::new(bytes.len(), bytes.len()),
            "missing final LF",
        ));
    }
    let mut line_start = 0;
    for (offset, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            if offset == line_start {
                return Err(ReadError::at(
                    offset,
                    "empty line; canonical text has one item per line",
                ));
            }
            line_start = offset + 1;
        }
    }
    Ok(())
}

/// The lines of a layout-checked file: `(start, end)` without the LF.
pub(crate) fn lines(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (offset, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            out.push((start, offset));
            start = offset + 1;
        }
    }
    out
}

fn starts_element(b: Option<u8>) -> bool {
    matches!(b, Some(b) if b != b' ' && b != b')' && b != b'\n')
}

fn ends_atom(b: u8) -> bool {
    matches!(b, b' ' | b'(' | b')' | b'\n')
}

/// Reads the item on one line; `end` is the offset of its LF.
pub(crate) fn read_item(bytes: &[u8], start: usize, end: usize) -> Result<Sx, ReadError> {
    let at = |i: usize| bytes.get(i).copied();
    if at(start) != Some(b'(') {
        return Err(ReadError::at(
            start,
            "each line must hold one parenthesised item",
        ));
    }
    // Open lists: (items, offset of the opening parenthesis).
    let mut stack: Vec<(Vec<Sx>, usize)> = Vec::new();
    let mut pos = start;
    loop {
        // `pos` is at the start of an element.
        let element = match at(pos) {
            Some(b'(') => {
                if stack.len() >= MAX_NESTING {
                    return Err(ReadError::at(pos, "nesting deeper than 256 levels"));
                }
                stack.push((Vec::new(), pos));
                pos += 1;
                if at(pos) == Some(b' ') {
                    return Err(ReadError::at(pos, "space after an opening parenthesis"));
                }
                if at(pos) != Some(b')') {
                    continue;
                }
                None
            }
            Some(b'"') => {
                let from = pos;
                pos += 1;
                loop {
                    match at(pos) {
                        Some(b'"') => break,
                        Some(b'\\') if at(pos + 1) != Some(b'\n') => pos += 2,
                        Some(b'\n') | None => {
                            return Err(ReadError::at(pos.min(end), "unterminated string literal"));
                        }
                        Some(_) => pos += 1,
                    }
                }
                pos += 1;
                Some(Sx::Atom {
                    text: String::from_utf8_lossy(&bytes[from..pos]).into_owned(),
                    span: Span::new(from, pos),
                })
            }
            _ => {
                let from = pos;
                while at(pos).is_some_and(|b| !ends_atom(b)) {
                    pos += 1;
                }
                Some(Sx::Atom {
                    text: String::from_utf8_lossy(&bytes[from..pos]).into_owned(),
                    span: Span::new(from, pos),
                })
            }
        };
        let mut finished = element;
        // Handle what follows a completed element (or an empty list at `pos`).
        loop {
            if let Some(done) = finished.take() {
                match stack.last_mut() {
                    Some((items, _)) => items.push(done),
                    None => {
                        if at(pos) == Some(b')') {
                            return Err(ReadError::at(pos, "unbalanced closing parenthesis"));
                        }
                        if pos != end {
                            return Err(ReadError::at(pos, "more than one item on a line"));
                        }
                        return Ok(done);
                    }
                }
            }
            match at(pos) {
                Some(b')') => {
                    let Some((items, open)) = stack.pop() else {
                        return Err(ReadError::at(pos, "unbalanced closing parenthesis"));
                    };
                    pos += 1;
                    finished = Some(Sx::List {
                        items,
                        span: Span::new(open, pos),
                    });
                }
                Some(b' ') => {
                    if !starts_element(at(pos + 1)) {
                        return Err(ReadError::at(
                            pos,
                            "tokens are separated by exactly one space",
                        ));
                    }
                    pos += 1;
                    break;
                }
                Some(b'\n') | None => {
                    return Err(ReadError::at(pos.min(end), "missing closing parenthesis"));
                }
                Some(_) => {
                    return Err(ReadError::at(pos, "missing space between tokens"));
                }
            }
        }
    }
}
