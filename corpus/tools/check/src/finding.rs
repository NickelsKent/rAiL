//! One problem found in the corpus, named by file, program and field so a
//! reader can go straight to it.

use std::fmt;

/// A problem the corpus check found (as opposed to a tool failure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The file the problem is in (relative to the corpus directory when it
    /// is a corpus file).
    pub file: String,
    /// The program concerned, when there is one.
    pub program: Option<String>,
    /// The field concerned, when there is one.
    pub field: Option<String>,
    /// What is wrong.
    pub message: String,
}

impl Finding {
    /// A finding about `file` with no program or field.
    pub fn new(file: impl Into<String>, message: impl Into<String>) -> Finding {
        Finding {
            file: file.into(),
            program: None,
            field: None,
            message: message.into(),
        }
    }

    /// Names the program concerned.
    pub fn program(mut self, program: impl Into<String>) -> Finding {
        self.program = Some(program.into());
        self
    }

    /// Names the field concerned.
    pub fn field(mut self, field: impl Into<String>) -> Finding {
        self.field = Some(field.into());
        self
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file)?;
        if let Some(program) = &self.program {
            write!(f, ": program {program}")?;
        }
        if let Some(field) = &self.field {
            write!(f, ": field {field}")?;
        }
        write!(f, ": {}", self.message)
    }
}
