//! The corpus vocabulary: classes, languages, and the approved program list.

use std::fmt;

/// One of the four program classes (BR1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    PureTransform,
    ParsingText,
    IoServices,
    DataParallel,
}

impl Class {
    /// The classes in delivery order (BR9.1).
    pub const ALL: [Class; 4] = [
        Class::PureTransform,
        Class::ParsingText,
        Class::IoServices,
        Class::DataParallel,
    ];

    /// The directory and index spelling.
    pub fn name(self) -> &'static str {
        match self {
            Class::PureTransform => "pure-transform",
            Class::ParsingText => "parsing-text",
            Class::IoServices => "io-services",
            Class::DataParallel => "data-parallel",
        }
    }

    /// Parses the directory and index spelling.
    pub fn parse(name: &str) -> Option<Class> {
        Class::ALL.into_iter().find(|c| c.name() == name)
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One of the four languages every program is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lang {
    Rail,
    Rust,
    Go,
    Ocaml,
}

impl Lang {
    /// Every language, in directory order.
    pub const ALL: [Lang; 4] = [Lang::Rail, Lang::Rust, Lang::Go, Lang::Ocaml];

    /// The languages the corpus check builds and runs now (BR8.1).
    pub const RUNNABLE: [Lang; 3] = [Lang::Rust, Lang::Go, Lang::Ocaml];

    /// The directory spelling.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rail => "rail",
            Lang::Rust => "rust",
            Lang::Go => "go",
            Lang::Ocaml => "ocaml",
        }
    }

    /// Parses the directory spelling.
    pub fn parse(name: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.name() == name)
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The approved program list and the per-class count (BR1.1, BR1.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composition {
    /// Programs required in every delivered class.
    pub per_class: usize,
    /// Every approved program, with its class.
    pub approved: Vec<(Class, String)>,
}

impl Composition {
    /// The 40 programs approved in Functional Design Q1, in delivery order.
    pub fn approved() -> Composition {
        const LIST: [(Class, [&str; 10]); 4] = [
            (
                Class::PureTransform,
                [
                    "binary-trees",
                    "fannkuch-redux",
                    "n-body",
                    "spectral-norm",
                    "levenshtein-batch",
                    "knapsack-dp",
                    "record-sort-dedup",
                    "run-length-codec",
                    "base64-codec",
                    "dijkstra-paths",
                ],
            ),
            (
                Class::ParsingText,
                [
                    "json-validate",
                    "csv-aggregate",
                    "word-frequency",
                    "mini-regex",
                    "markdown-lite",
                    "ini-config",
                    "log-analyze",
                    "expr-calc",
                    "utf8-stats",
                    "line-diff",
                ],
            ),
            (
                Class::IoServices,
                [
                    "file-checksum",
                    "dir-summary",
                    "kv-log-store",
                    "config-loader",
                    "batch-rename",
                    "log-rotate",
                    "process-pipeline",
                    "env-report",
                    "csv-to-json-files",
                    "incremental-backup",
                ],
            ),
            (
                Class::DataParallel,
                [
                    "parallel-sum",
                    "parallel-histogram",
                    "parallel-mandelbrot",
                    "parallel-matmul",
                    "parallel-merge-sort",
                    "parallel-word-count",
                    "parallel-prefix-scan",
                    "parallel-bfs",
                    "parallel-monte-carlo-pi",
                    "parallel-blur",
                ],
            ),
        ];
        let approved = LIST
            .iter()
            .flat_map(|(class, names)| names.iter().map(|n| (*class, (*n).to_string())))
            .collect();
        Composition {
            per_class: 10,
            approved,
        }
    }

    /// The approved programs of one class.
    pub fn of_class(&self, class: Class) -> Vec<&str> {
        self.approved
            .iter()
            .filter(|(c, _)| *c == class)
            .map(|(_, n)| n.as_str())
            .collect()
    }
}
