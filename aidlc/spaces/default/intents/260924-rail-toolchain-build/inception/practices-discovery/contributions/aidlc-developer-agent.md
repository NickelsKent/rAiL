**Collaborator:** aidlc-developer-agent

## Contribution

Blind support review from the developer's side. My focus is naming, layer
boundaries, error handling, file organization, and code style. The repository
has no code yet, so everything below comes from the spec
(`aidlc/spaces/default/knowledge/documents/rail-language-spec.md`), the approved
Ideation artifacts, and the lead draft. Each point is a **suggestion for the
interview**, not a rule. Wherever I name a crate, directory, or lint, the exact
list belongs to the later design stages. What the interview should settle is
the convention behind it.

### 1. Naming

- **Lowercase `rail` in every identifier.** Spec "Name and file extensions" says
  `rAiL` is only a display spelling. The CLI, file names, package namespaces,
  and binary magic all use `rail` or `RAIL`. Apply the same rule to the Rust
  side: crate package names (`rail-syntax`), library names (`rail_syntax`), the
  binary (`rail`), environment variables (`RAIL_*`), and paths. Use `rAiL` only
  in prose.
- **Rust names follow the Rust API guidelines.** The lead draft already covers
  `snake_case` and `CamelCase`. Add these: `SCREAMING_SNAKE_CASE` for constants
  and statics, kebab-case for crate package names, and acronyms written as one
  word in type names (`CoreIr`, `MonoIr`, `Rir`, `RapRequest`, not `RIR` or
  `RAPRequest`). `clippy::upper_case_acronyms` already enforces the acronym rule,
  so no separate rule is needed. This follows the org.md principle that a
  linter rule wins over agent advice.
- **Domain types use the spec's own words.** Toolchain types should use the
  spec's terms (`DefId`, `DefHash`, `ModHash`, `AnchorPath`, `Diagnostic`,
  `Patch`, `Caps`) instead of synonyms. Then a spec section, a requirement ID,
  and a type name can be matched by searching. This is the Rust-side version of
  the spec's own "one spelling" principle (§2.7).
- **Diagnostic rule IDs are permanent and defined in one place.** Spec R12 says
  "rule IDs permanent". IDs such as `FMT…`, `TY020`, and `SEC001` should be
  declared once, in a single registry module or table. Code refers to that
  constant, never to a string literal typed again elsewhere. A removed rule
  keeps its ID reserved, and IDs are never renumbered.
- **Test names point back to the acceptance suites.** Acceptance-suite code
  names its suite ID (for example `t01_deterministic_parsing`), so the T1–T14
  exit criteria in the roadmap can be traced to code.

### 2. Layer boundaries

The spec's MVP diagram already gives a layer order: grammar → parser/printer
→ query engine → checker → lowering (Core, Mono, RIR) → backends → runtime,
with the RAP server on top of the query engine. Suggested conventions:

- **Dependencies point one way.** Each layer depends only on layers below it.
  No crate depends on a frontend, and no cycles are allowed (Cargo already
  rejects cycles between crates). Cargo cannot stop a lower crate from
  depending on a higher one, so this is enforced through review, plus an
  optional workspace test that reads `cargo metadata`.
- **The CLI and RAP are thin frontends over one shared service layer.** Spec
  §9 exposes each capability both ways (`rail check` ⇄ `check.run`, and so on).
  Both should call the same function and serialize the same result type, so
  their outputs cannot drift. Neither frontend should contain compiler logic.
- **The runtime is kept apart from the compiler.** The spec says the runtime
  depends on "nothing beyond the platform C library", and T14 checks this with
  `ldd`/`otool`. So the runtime crate must never depend on any compiler crate
  or third-party crate. Only `libc`-level bindings are allowed, or raw
  `extern "C"` declarations if even the `libc` crate is unwanted. It is linked
  into *generated programs*, not only into the `rail` binary, so it builds as
  its own artifact (for example a `staticlib`).
- **"One crate workspace" needs clarifying.** The spec says the compiler is
  "written in Rust as one crate workspace producing one binary". I read that as
  *one Cargo workspace with several crates, producing one `rail` executable*,
  not a single crate. A multi-crate layout is what makes the boundaries above
  enforceable, and it helps incremental Rust build times. Candidate 1 in
  `discovered-rules.md` should use the clarified wording if the owner agrees.
- **The benchmark corpus stays outside the toolchain workspace.** The Rust
  corpus programs (and the Go and OCaml ones) should live in their own
  workspace, or be excluded from the root one. Their dependencies then never
  reach the toolchain's `Cargo.lock`, which keeps the toolchain's dependency
  count honest.

### 3. Error handling

The toolchain has three kinds of failure. Keeping them apart is the most
important error-handling convention for this project.

1. **Problems in the user's rAiL program are data, not Rust errors.** Parse
   errors, type errors, and lints are `Diagnostic` values in the §6.4 schema.
   They are collected and returned; they never cause an early return or a
   panic. One bad definition must not hide diagnostics in other definitions,
   because agents rely on the full diagnostic set (§9.3).
2. **Tool and environment failures are typed `Result` errors.** Examples are
   I/O errors, a malformed RAP request, a missing vendored package, or a
   `base` hash mismatch. Each library crate defines its own error enum, which
   matches the spec's "one error type per module" stance in §7.1. Crates add
   context as an error crosses a boundary (they wrap it, never replace it). A
   frontend converts the error to a CLI exit code or a JSON-RPC error object.
   Library crates never use a catch-all error type.
3. **Compiler bugs are panics, caught at the frontend.** A broken internal
   invariant may panic. The CLI turns a panic into a structured
   internal-compiler-error report with a fixed exit code. The RAP server must
   catch a panic at the request boundary (`std::panic::catch_unwind`), return
   a JSON-RPC internal error, and keep serving, because one bad request must not
   kill the warm compiler that agents depend on. This means the release profile
   must not set `panic = "abort"` for the toolchain binary.

Supporting conventions for the interview:

- **No `unwrap()` or `expect()` on input or I/O paths in non-test code.** This
  can be enforced with `clippy::unwrap_used` at deny level in library crates.
  `expect("invariant: …")` is allowed where the message states the invariant.
- **Never write to stdout from library code.** RAP runs JSON-RPC over stdio
  (§9), so one stray `println!` corrupts the protocol stream. Only the frontend
  writes stdout. Logs and tracing go to stderr. Enforce this with
  `clippy::print_stdout` and `clippy::print_stderr` in library crates.
- **Error enums use the standard library by default.** Write plain `enum`s
  that implement `std::error::Error` and `Display`. Adding `thiserror` or
  `anyhow` is a dependency decision (see §5 below) for the owner. If it is
  added, keep `anyhow` out of library crates.
- **The rAiL runtime's own failure model is the spec's, not Rust's.** Traps,
  exit code 70, and `Err(TaskTrap)` (§7.3) are rAiL semantics that the runtime
  implements. They must not be confused with how the toolchain handles its own
  errors.

### 4. Determinism as a coding convention

T1, T3, T4, and T7 require byte-identical output across runs, machines, and
operating systems. Most ways to break that are ordinary Rust habits, so they
should be written down as code conventions and not left to the tests alone:

- Output is never produced by iterating a `HashMap` or `HashSet`. Use
  `BTreeMap`/`BTreeSet`, or sort before emitting. The spec already orders
  output by definition ID (§7.10).
- Compiler output never reads the wall clock, the current directory, the
  hostname, a username, or a random source. `clippy.toml`
  `disallowed-methods` can enforce this for the output-producing crates (for
  example `std::time::SystemTime::now` and `std::env::current_dir`). The
  exact list belongs to design.
- Floating-point formatting uses shortest round-trip output (§2.5 names Ryū).
  `format!("{}")` is not used for canonical output unless tests prove it
  matches.
- Parallel phases merge results in a deterministic order (by definition ID),
  never in completion order.

This extends the lead's candidate 7 from "what must not appear" to "how the
code avoids it".

### 5. `unsafe` and dependencies

- **`unsafe` stays in marked crates.** Compiler and tool crates declare
  `#![forbid(unsafe_code)]`. `unsafe` is allowed only in the runtime and in any
  crate that binds a native backend (LLVM bindings, for example), and each
  block carries a `// SAFETY:` comment (`clippy::undocumented_unsafe_blocks`).
  This mirrors the spec's own rule that unsafety exists only inside marked
  `ffi-module`s (§7.6).
- **New Rust dependencies need a stated reason.** A new crate in `Cargo.lock`
  is called out in the PR description, with the reason it is not written by
  hand, and the owner approves it. This mirrors spec §5.9 rule 3 (new
  dependencies need explicit approval) and the owner's own engineering rule to
  prefer the standard library. Commit `Cargo.lock`, and have CI build with
  `--locked` (this adds detail to candidate 5).
- **Generated code from the shared grammar.** The spec says one grammar
  definition generates the three parsers and printers. Two options for the
  interview: (a) generate at build time in `build.rs`, or (b) generate with a
  developer command (an `xtask`), check the output into the repo, and have CI
  fail when the checked-in output is stale. I suggest (b). The output can be
  reviewed in diffs, a normal build needs no code generation, and it fits the
  offline and reproducibility goals (T3, T14).

### 6. File organization (suggested repo layout)

This shows the conventions only. The exact crate list comes from the design
stages.

```
Cargo.toml            # workspace root: [workspace.lints], shared profile, members
rust-toolchain.toml   # pinned toolchain (lead's open question 8)
clippy.toml           # disallowed-methods / types for determinism
crates/rail-*/        # one directory per crate, kebab-case, layered as in §2
  src/…               # unit tests inline in #[cfg(test)] mod tests
  tests/              # per-crate integration tests
grammar/              # the single shared grammar source
std/                  # rAiL standard library, canonical .rlc only
tests/acceptance/     # T1–T14 harnesses, one module per suite ID
tests/golden/         # checked-in expected outputs
corpus/<program>/{rail,rust,go,ocaml}/  # separate from the toolchain workspace
xtask/                # cross-platform dev automation in Rust (no shell/Make)
```

- Lint settings live in one place: the root `[workspace.lints]` table, which
  each crate inherits with `lints.workspace = true`. Per-crate lint attributes
  are the exception, used only for the `unsafe`, `print`, and `unwrap` scoping
  above.
- Golden files are updated only through an explicit bless step (for example
  `RAIL_BLESS=1 cargo test`). CI never blesses, so a changed output always
  shows up as a reviewable diff.
- Group modules inside a crate by concept (types, effects, exhaustiveness),
  not by technical kind.
- **The spec's location needs a decision.** The only copy of the spec is under
  `aidlc/spaces/default/knowledge/documents/`, which the DocumentKB treats as
  user-owned. If candidate 3 ("update the spec in the same change") becomes a
  rule, every spec edit also needs `aidlc knowledge sync`, or the catalogued
  copy goes stale. The interview should confirm that this is the authoritative
  location, rather than a `docs/` or `spec/` directory in the repo.

### 7. Code style: additions to the lead's section

- **Rust edition and lint command.** State the edition for new crates
  (suggest 2024, the current stable edition) and the exact CI lint command:
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  `-D warnings` without `--all-targets` would miss test code.
- **Pinned formatter versions for the corpus.** `ocamlformat` output changes
  between versions and needs a pinned `.ocamlformat` version. The Go
  toolchain version should be pinned for `gofmt`. The corpus Rust programs
  should pass the same `rustfmt` and `clippy` settings as the toolchain, so the
  comparison stays idiomatic.
- **Where the locked tech stack is recorded.** Choices such as Rust, Cranelift
  then LLVM, and BLAKE3 belong under `project.md` `## Tech Stack`. The
  promotion tool writes only the five `team.md` sections and the `project.md`
  Mandated/Forbidden lists, so either put the language choice in Code Style or
  a Mandated rule (as the lead does), or accept that `## Tech Stack` is filled
  later.

### Suggested extra interview questions (developer view)

1. "One crate workspace": several crates in one Cargo workspace (suggested), or
   literally one crate?
2. Should the three failure kinds in §3 (diagnostics as data, typed `Result`
   errors, and panics caught at the frontend, with RAP surviving a panic)
   become the team's error-handling convention?
3. Should `#![forbid(unsafe_code)]` apply everywhere except the runtime and
   native-backend binding crates?
4. Should the determinism conventions in §4 be enforced with `clippy.toml`
   disallowed methods, or with tests only?
5. Should each new Rust dependency need a stated reason and owner approval in
   the PR? Is `thiserror` (or `anyhow` in the binary only) acceptable up front?
6. Should grammar-generated code be built in `build.rs`, or generated,
   checked in, and drift-checked in CI?
7. Is `aidlc/spaces/default/knowledge/documents/rail-language-spec.md` the
   authoritative spec location, or should it move into the repo?

## Positions

- AGREE: `rustfmt` defaults plus `clippy` with warnings as errors, run in CI and blocking merge. Add `--workspace --all-targets` so test code is linted too.
- AGREE: the canonical `.rlc` form is enforced by the parser and `rail fmt` formats all rAiL in the repo. This matches spec §2.5–2.7 and R11.
- AGREE: each benchmark-corpus language uses its own standard formatter. Formatter and toolchain versions should also be pinned, or the output will differ between runs.
- AGREE: candidate 2 (runtime depends only on the platform C library). It also implies the runtime is a separate crate that depends on no compiler or third-party crate.
- AGREE: candidate 7 (no timestamps, paths, hostnames, or seeds in output). I suggest extending it with the coding conventions in Contribution §4, such as no output from iterating a `HashMap`.
- AGREE: candidate 5 (offline builds). Specify a committed `Cargo.lock` and CI builds with `--locked`; vendoring can stay optional until T14 work starts.
- AGREE: the walking skeleton going all the way to a native binary. From a code-structure view, its main value is proving every layer boundary (syntax → check → lower → Cranelift → runtime) before those crates grow. A skeleton that stops at RAP would leave the riskiest boundary, compiler to runtime linkage, unproven. The phase-order trade-off [Q7] is still the owner's decision.
- OBJECT: candidate 1's wording ("one crate workspace") is ambiguous. Reword it as "one Cargo workspace, several crates, one `rail` executable", or ask the owner, before it becomes a rule.
- OBJECT: the Code Style section has no error-handling, layering, or `unsafe` conventions, and those are what agents writing Rust will need most. Add the Contribution §2, §3 and §5 items as interview questions, keeping only what the owner confirms.
- OBJECT: candidate 3 does not say where the spec lives. Its only copy is in the user-owned DocumentKB `documents/` folder, which needs `aidlc knowledge sync` after each edit. Settle the location in the interview (Contribution §6).
