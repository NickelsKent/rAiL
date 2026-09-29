//! Compiles `rail-runtime` (a `no_std` static library) with the same `rustc`
//! that builds `rail`, for the host target, into `OUT_DIR`. `rail-build`
//! embeds the archive so every build links the matching runtime.

#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let crates = manifest.parent().expect("crates directory").to_path_buf();
    let source = crates.join("rail-runtime/src/lib.rs");
    println!("cargo:rerun-if-changed={}", source.display());
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("librail_runtime.a");
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let target = env::var("TARGET").expect("TARGET");
    let status = Command::new(&rustc)
        .args([
            "--crate-name",
            "rail_runtime",
            "--crate-type",
            "staticlib",
            "--edition",
            "2024",
        ])
        .args([
            "-C",
            "panic=abort",
            "-C",
            "opt-level=2",
            "-C",
            "debuginfo=0",
        ])
        // Fat LTO keeps only what the runtime uses, so the archive needs
        // nothing beyond the C library (no unwinding personality).
        .args(["-C", "codegen-units=1", "-C", "lto=fat"])
        .arg("--target")
        .arg(&target)
        .arg(format!("--remap-path-prefix={}=rail", crates.display()))
        .arg("-o")
        .arg(&out)
        .arg(&source)
        .status()
        .expect("run rustc for rail-runtime");
    assert!(status.success(), "compiling rail-runtime failed");
}
