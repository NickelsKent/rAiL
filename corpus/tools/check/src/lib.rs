//! The benchmark corpus check (WF3). Library code never prints to stdout;
//! the binary's `main` prints the report.

pub mod budget;
pub mod build;
pub mod check;
pub mod cli;
pub mod expected;
pub mod finding;
pub mod index;
pub mod inputs;
pub mod manifest;
pub mod model;
pub mod pins;
pub mod probe;
pub mod proc;
pub mod sandbox;
pub mod setup;
pub mod sha256;
pub mod toml;
pub mod tree;
