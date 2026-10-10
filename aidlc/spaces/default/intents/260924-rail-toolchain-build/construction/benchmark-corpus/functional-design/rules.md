# Business Rules — benchmark-corpus (U16)

The rules the benchmark corpus follows. One principle runs through them: **every comparison is like for like.** Each program asks the same thing of every language, gets the same inputs, and must give byte-identical results, so a speed or token difference reflects the language rather than the program.

Sources: FR54 (benchmark corpus), FR58 (spec upkeep), the cross-cutting requirements NFR3 (determinism), NFR5 (capability safety), NFR6 (portability) and NFR7 (dependencies), contracts C9 and C10 (corpus layout), spec §11.1, the team practices (firm rules, Testing Posture, dependency approval), and `functional-design-questions.md` ([Q1]–[Q9]).

```yaml
rules:
  # BR1 — Composition
  - id: BR1.1
    statement: The corpus has exactly 40 programs, ten in each of the four classes.
    category: constraint
    applies_to: Corpus
    trigger: corpus check (every pull request that touches corpus/)
    logic: IF corpus/index.toml does not list exactly ten programs for each of pure-transform, parsing-text, io-services and data-parallel THEN fail.
    violation: corpus check fails naming the class and its count.
    source: [FR54, spec §11.1]
  - id: BR1.4
    statement: The index carries everything a suite or tool needs to run a program without reading its prompt.
    category: validation
    applies_to: CorpusIndex
    trigger: corpus check
    logic: IF an index entry lacks its run data (argument template, input mode, threads flag, generator mode, generator size range, reference size, fixed input names, output comparison) THEN fail. Suite volumes are not stored here; they belong to the suites' settings (contract C9).
    violation: corpus check fails naming the program and the missing field.
    source: [FR54, C9, C10]
  - id: BR1.2
    statement: The program list is the approved list.
    category: policy
    applies_to: CorpusProgram
    trigger: corpus check
    logic: IF a program_id is not one of the 40 approved in functional-spec.md, or an approved one is missing THEN fail. Replacing a program needs your approval and is recorded in the class's APPROVAL.md; a program is never simply removed, because suites use the whole corpus as an input set.
    violation: corpus check fails naming the unexpected or missing program.
    source: [FR54, Q1]
  - id: BR1.3
    statement: Every program follows the shared layout.
    category: validation
    applies_to: CorpusProgram
    trigger: corpus check
    logic: IF corpus/<class>/<program_id>/ lacks prompt.md, fairness-review.md, rail/, rust/, go/, ocaml/, inputs/gen/, inputs/fixed/ or expected/ THEN fail.
    violation: corpus check fails naming the program and the missing part.
    source: [C10]

  # BR2 — The shared run contract
  - id: BR2.1
    statement: Every version of a program takes its input the same way and writes the same output.
    category: constraint
    applies_to: LanguageVersion
    trigger: corpus check
    logic: IF a version reads input other than as stated in its prompt (stdin, or file and directory paths given as arguments) or writes results anywhere but stdout (and, for io-services, the paths named in its arguments) THEN it does not conform.
    violation: the version is marked defective and fixed with a reproducing test.
    source: [FR54, Q2]
  - id: BR2.2
    statement: Output is deterministic and identical across languages and platforms.
    category: constraint
    applies_to: LanguageVersion
    trigger: corpus check
    logic: IF for any fixed input a version's stdout, exit code or error line differs from the expected output, or depends on time, hash-map ordering, thread scheduling, absolute paths or the host THEN fail.
    violation: corpus check fails naming the program, language, input and first differing byte.
    source: [NFR3, NFR6, Q6]
  - id: BR2.3
    statement: Data-parallel results do not depend on the number of threads.
    category: constraint
    applies_to: LanguageVersion
    trigger: corpus check, data-parallel class
    logic: Each data-parallel program takes --threads <n>. IF its output for a fixed input differs between 1 thread and the machine's core count THEN fail. Floating-point reductions use a fixed reduction order so results are reproducible.
    violation: corpus check fails naming the program and thread counts.
    source: [NFR3, Q4]
  - id: BR2.4
    statement: Floating-point programs use only correctly rounded operations, so every language and platform gets the same bits.
    category: constraint
    applies_to: Prompt, LanguageVersion
    trigger: authoring and fairness review
    logic: 'Programs that compute in floating point (n-body, spectral-norm, parallel-mandelbrot) use only IEEE-754 binary64 addition, subtraction, multiplication, division and square root, in the evaluation order written in the prompt, and print with correctly rounded fixed-decimal formatting. They use no other math-library function and no fused multiply-add: the Go versions write every product that feeds an addition through an explicit float64() conversion, which the Go specification defines as preventing fusion. Programs whose task does not need floating point use integers (parallel-blur, parallel-monte-carlo-pi with an integer hit test). Comparisons stay byte-exact; no tolerance is ever used, because T9 requires zero output differences.'
    violation: a version that differs is defective and is fixed to follow the prompt's operation order (BR3.4).
    source: [NFR3, NFR6, spec §12 T7 T9]
  - id: BR2.5
    statement: Output never depends on file-system order or timestamps.
    category: constraint
    applies_to: LanguageVersion
    trigger: io-services and parsing programs
    logic: Directory entries are processed and printed sorted by the byte order of their names. No program prints or decides on modification times; incremental-backup compares content checksums recorded in its manifest. Empty directories in fixtures are created by the generator, since git does not store them.
    violation: corpus check fails on the differing output.
    source: [NFR3, NFR6]

  # BR3 — Authoring
  - id: BR3.1
    statement: All four versions of a program come from one prompt.
    category: policy
    applies_to: Prompt
    trigger: authoring
    logic: The task, input and output format, error cases and run contract are written once in prompt.md. Each version is generated from that prompt alone plus that language's reference (the spec for rAiL).
    violation: a version written from a different or edited prompt is regenerated.
    source: [FR54, spec §11.1, Q2]
  - id: BR3.2
    statement: The same model writes all four versions, each in its own session.
    category: policy
    applies_to: Prompt
    trigger: authoring
    logic: prompt.md records the model id and the date. IF a version was written by another model, or in a session that saw another language's version THEN it is regenerated.
    violation: the fairness review fails the program until regenerated.
    source: [Q2]
  - id: BR3.3
    statement: rAiL versions are written in canonical text straight from the spec.
    category: policy
    applies_to: LanguageVersion
    trigger: authoring
    logic: The rail/ version is main.rlc in canonical text (spec §2). Every definition carries a definition ID chosen by the author ('#' and six Crockford base32 characters, as in the walking skeleton), unique within the module; the corpus check verifies the ID format and uniqueness. It is not run through any tool that does not yet exist.
    violation: corpus check fails on a malformed or duplicate ID; later validation applies BR8.2.
    source: [Units Generation U16, spec §2.6]
  - id: BR3.4
    statement: After generation, a version is edited only to fix a defect, and every edit is logged.
    category: policy
    applies_to: LanguageVersion
    trigger: any change to a version after generation
    logic: IF a version changes after generation THEN fairness-review.md gains an edit-log line with the reason, and the fix comes with a reproducing test (a new fixed input or check). Performance tuning of one language is not a defect fix.
    violation: the fairness review is reopened.
    source: [Q2, practices defect rule]

  # BR4 — Fairness
  - id: BR4.1
    statement: Every program is reviewed against the one fairness checklist.
    category: policy
    applies_to: FairnessReview
    trigger: after all four versions exist
    logic: An AI reviewer, in a session separate from the writing sessions (it may be the same model), records its model id and pass or fail with a note for each item of corpus/fairness-checklist.md, sets the outcome, and names its model id. The checklist covers same algorithm and data structures, idiomatic code, no language skipping work the others do, the same error handling, and the same output.
    violation: outcome needs-changes; the program goes back to authoring.
    source: [Q3]
  - id: BR4.2
    statement: A class counts as delivered only after you approve it.
    category: policy
    applies_to: ClassBatch
    trigger: all ten programs of a class reviewed fair and checked
    logic: The class status becomes awaiting-approval; you approve the ten together; the approval and date are recorded in corpus/<class>/APPROVAL.md.
    violation: the class stays awaiting-approval and the next class does not start.
    source: [Q3, Q7]

  # BR5 — Libraries and toolchains
  - id: BR5.1
    statement: Versions use their standard library only.
    category: policy
    applies_to: LanguageVersion
    trigger: corpus check
    logic: IF a version's manifest (Cargo.toml, go.mod, dune-project or opam file) names a dependency other than those allowed by BR5.2 THEN fail.
    violation: corpus check fails naming the program and dependency.
    source: [Q4]
  - id: BR5.2
    statement: Only the data-parallel class may use the two approved parallelism libraries, pinned and vendored.
    category: policy
    applies_to: ToolchainPins
    trigger: corpus check
    logic: rayon (Rust) and domainslib (OCaml) are allowed only in data-parallel programs, at the exact versions in corpus/toolchains.toml, vendored under corpus/vendor/ so builds need no network. Go uses goroutines and channels only.
    violation: corpus check fails naming the program and library.
    source: [Q4, Q9, practices dependency approval]
  - id: BR5.3
    statement: Toolchains are pinned and changed only by a reviewed change.
    category: policy
    applies_to: ToolchainPins
    trigger: corpus check
    logic: The corpus check reads corpus/toolchains.toml and refuses to run if the installed rustc, go, ocaml or dune version differs from the pin. The Rust pin equals the workspace's rust-toolchain.toml.
    violation: tool error naming the toolchain and both versions.
    source: [Q8]
  - id: BR5.4
    statement: The corpus stays outside the toolchain workspace.
    category: constraint
    applies_to: Corpus
    trigger: build of the rail workspace
    logic: corpus/ is excluded from the root Cargo workspace; each Rust version is its own small package with its own lockfile; nothing in the workspace depends on the corpus.
    violation: workspace build or cargo-deny would include corpus crates; the corpus check fails.
    source: [Units Generation U16, NFR7]
  - id: BR5.5
    statement: The corpus's own Rust dependencies pass the same supply-chain checks as the workspace.
    category: policy
    applies_to: ToolchainPins
    trigger: every pull request that touches corpus/
    logic: Every Rust package in the corpus commits its lockfile. CI runs cargo deny check (advisories, bans, licences, sources) over the corpus Rust packages with corpus/deny.toml, which allows only the standard library plus rayon and its pinned dependencies. Vendored OCaml packages are pinned by checksum in corpus/toolchains.toml.
    violation: CI fails like the workspace's supply-chain job.
    source: [Q9, practices supply-chain checks]

  # BR6 — I/O services and errors
  - id: BR6.1
    statement: Every program follows one command-line contract.
    category: constraint
    applies_to: Prompt
    trigger: authoring
    logic: The program is started with arguments only (input paths, --threads for data-parallel, the operation's targets for io-services) and reads no environment variables except those its prompt names (env-report only).
    violation: the version is defective.
    source: [Q2, Q5]
  - id: BR6.2
    statement: Failures are typed errors with one fixed shape.
    category: validation
    applies_to: ExpectedOutput
    trigger: any expected failure
    logic: 'IF a program fails for a reason its prompt lists THEN it exits 1 and writes exactly one line "error: <kind>: <detail>" to stderr, where kind is from the prompt''s error list and detail contains no absolute paths. Any other failure (crash, panic, uncaught exception) is a defect.'
    violation: corpus check fails naming the program, language and input.
    source: [FR54, Q5]
  - id: BR6.3
    statement: The I/O class uses no network.
    category: constraint
    applies_to: CorpusProgram
    trigger: authoring and corpus check
    logic: io-services programs use files, stdin and stdout, processes and environment only. IF a version opens a socket THEN it does not conform.
    violation: the version is defective.
    source: [Q5]
  - id: BR6.4
    statement: I/O programs touch only the paths they are given.
    category: constraint
    applies_to: CorpusProgram
    trigger: corpus check, io-services
    logic: The check runs each io-services version with its current directory, HOME and TMPDIR inside a fresh temporary directory holding its fixture, and passes only relative paths. Beside that directory it places a canary directory. After the run, IF the canary changed, or the run directory's contents differ from the expected tree listing (expected/<input>.tree, names and content checksums, no timestamps) THEN fail. Reads outside the given paths cannot be detected mechanically; they are a fairness-checklist item instead.
    violation: corpus check fails naming the program and path.
    source: [Q5, NFR5]
  - id: BR6.5
    statement: Error cases that depend on the machine are created at run time.
    category: constraint
    applies_to: InputSet
    trigger: corpus check, io-services
    logic: Permission-denied and unreadable-file cases are created by the checker in the run directory just before the run (for example a file with no permissions), because git cannot store them. IF the check runs as the root user THEN those cases are reported as skipped with a notice and counted as not run, never as passed.
    violation: none; the skip is reported.
    source: [Q5]
  - id: BR6.6
    statement: process-pipeline runs one fixed helper.
    category: constraint
    applies_to: CorpusProgram
    trigger: process-pipeline
    logic: The helper is corpus/tools/helper, a std-only Rust program built by the checker, whose behaviour (echo, exit with a given status, write to stderr) is chosen by its arguments. Its path is passed to process-pipeline as an argument. No version runs any other program.
    violation: the version is defective.
    source: [Q5]


  # BR7 — Inputs and expected outputs
  - id: BR7.1
    statement: Generated inputs depend only on seed and size.
    category: constraint
    applies_to: InputSet
    trigger: corpus check
    logic: Each generator is a std-only Rust program taking --seed, --size and --out <dir>. It writes the program's inputs into that directory under the names in the index (a single stream, several files, or a fixture tree). IF two runs with the same arguments, on either platform, give a different set of files or different bytes THEN fail.
    violation: corpus check fails naming the generator.
    source: [Q6, NFR3, NFR6]
  - id: BR7.2
    statement: Expected outputs come from the Rust version and change only through the bless step.
    category: policy
    applies_to: ExpectedOutput
    trigger: creating or changing expected/
    logic: Expected outputs are produced by running the Rust version with RAIL_BLESS=1 through the corpus check and reviewed in the pull request; they are never hand-edited.
    violation: a hand-edited expected file fails review.
    source: [Q6, practices bless rule]
  - id: BR7.3
    statement: Committed inputs are small; large inputs are generated.
    category: constraint
    applies_to: InputSet
    trigger: corpus check
    logic: IF a committed fixed input exceeds 64 KiB, or a program has more than eight fixed inputs THEN fail. Large inputs come from the generator at run time and are never committed.
    violation: corpus check fails naming the file.
    source: [Q6]

  # BR8 — Validation
  - id: BR8.1
    statement: The Rust, Go and OCaml versions are checked now, on both platforms.
    category: policy
    applies_to: LanguageVersion
    trigger: every pull request that touches corpus/, and nightly
    logic: The corpus check builds each Rust, Go and OCaml version with its pinned toolchain and runs it on every fixed input, comparing with expected/ (BR2.2) and with 1 and n threads (BR2.3).
    violation: CI fails naming the program, language and input.
    source: [FR54, NFR6]
  - id: BR8.2
    statement: rAiL versions are validated as the units that can parse, check and run them land.
    category: policy
    applies_to: LanguageVersion
    trigger: the suites of U3 (parse), U4 (check), U6 and U7 (run)
    logic: Each owning unit's suites include the corpus as inputs. IF a rAiL version fails to parse, check or produce the expected output THEN it is a corpus defect, fixed with a reproducing test, and its edit is logged (BR3.4).
    violation: the owning unit's suite fails until fixed.
    source: [Units Generation U16]

  # BR9 — Delivery
  - id: BR9.1
    statement: The corpus is delivered one class at a time, in a fixed order.
    category: policy
    applies_to: ClassBatch
    trigger: Code Generation
    logic: Classes are delivered in the order pure-transform, parsing-text, io-services, data-parallel. A class starts only after the previous one is approved (BR4.2).
    violation: none (ordering rule).
    source: [Q7]
  - id: BR9.2
    statement: The spec records the corpus conventions in the same change as the corpus.
    category: policy
    applies_to: Corpus
    trigger: the first corpus pull request
    logic: spec/rail-language-spec.md §11.1 gains a short note naming the corpus location, the library policy (standard libraries; rayon and domainslib in the data-parallel class only), no networking in the I/O class, and the floating-point rule (BR2.4); the knowledge copy is re-synced.
    violation: the pull request is not ready for review (firm rule).
    source: [FR58, practices firm rule]
```

## Rules summary

| Group | Rules | What they protect |
|---|---|---|
| BR1 Composition | BR1.1–BR1.4 | Exactly the 40 approved programs, ten per class, in the shared layout, with complete run data in the index. |
| BR2 Run contract | BR2.1–BR2.5 | The same input, the same byte-identical output, independent of threads, platform, language, file-system order and timestamps; correctly rounded floating point only. |
| BR3 Authoring | BR3.1–BR3.4 | One prompt, one model, separate sessions, and logged defect-only edits. |
| BR4 Fairness | BR4.1–BR4.2 | The checklist review, and your approval of each class. |
| BR5 Libraries and toolchains | BR5.1–BR5.5 | Standard libraries, with the two approved parallelism libraries; pinned toolchains; the corpus kept out of the workspace. |
| BR6 I/O and errors | BR6.1–BR6.6 | One command-line contract, one typed-error shape, no network, checked confinement, run-time permission cases, one fixed helper. |
| BR7 Inputs and outputs | BR7.1–BR7.3 | Seeded generators, Rust-reference outputs changed only through bless, small committed inputs. |
| BR8 Validation | BR8.1–BR8.2 | Rust, Go and OCaml checked now on both platforms; rAiL checked as the toolchain grows. |
| BR9 Delivery | BR9.1–BR9.2 | One class at a time, in a fixed order; the spec note lands with the corpus. |
