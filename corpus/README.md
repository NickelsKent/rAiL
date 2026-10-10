# The benchmark corpus

Forty programs in four classes of ten (pure-transform, parsing-text,
io-services, data-parallel), each written in rAiL, Rust, Go and OCaml from
one prompt and reviewed against `fairness-checklist.md`. Every version of a
program must give byte-identical output (stdout, exit code and the typed
error line) for the same input, on macOS and Linux. The corpus feeds the
suites T3, T4, T7, T9, T10 and T13 and the token measurements (spec §11.1).
It is outside the Cargo workspace (`exclude = ["corpus"]` in the root
`Cargo.toml`) and never ships.

## Layout

```
corpus/
  index.toml            every program and the data to run it (read by suites and tools)
  toolchains.toml       pinned Rust, Go, OCaml, dune; approved libraries
  fairness-checklist.md the one review checklist
  deny.toml             cargo-deny policy for the corpus Rust packages
  check.sh              the corpus check
  tools/check/          the std-only Rust checker behind check.sh
  tools/helper/         the one fixed child program process-pipeline runs
  tools/sandbox/        the macOS sandbox profile
  <class>/APPROVAL.md   written when the owner approves the class
  <class>/<program>/
    prompt.md  fairness-review.md
    rail/main.rlc  rust/  go/  ocaml/
    inputs/gen/         seeded std-only Rust generator (--seed --size --out)
    inputs/fixed/<name>/  small committed inputs (at most 8, each file at most 64 KiB)
    inputs/fixed/<name>.setup   optional run-time cases (unreadable files, read-only dirs)
    expected/<name>.out|.err|.tree   from the Rust version, via bless only
```

## Running the check

```bash
corpus/check.sh                          # every program that has an index entry
corpus/check.sh --class pure-transform
corpus/check.sh --program n-body --lang rust,go
corpus/check.sh --bless                  # write expected/ from the Rust versions (or RAIL_BLESS=1)
corpus/check.sh --help
```

The check reads the pins and the index, checks the layout, the libraries and
the fixed inputs, builds the Rust, Go and OCaml versions offline, and runs
each on every fixed input inside an OS sandbox (`sandbox-exec` on macOS,
`bwrap` on Linux) with its current directory, `HOME` and `TMPDIR` in a fresh
run directory. Nothing is ever run unconfined: a missing or failing sandbox
stops the check (exit 2). Exit codes: 0 all passed, 1 the corpus has
problems, 2 the check could not run.

Go, OCaml, dune and opam are installed by you, at the versions in
`toolchains.toml`; the checker refuses to run on other versions. On Linux,
install `bubblewrap` and allow unprivileged user namespaces.

The checker's own tests: `cargo test --locked --manifest-path corpus/tools/check/Cargo.toml`.
