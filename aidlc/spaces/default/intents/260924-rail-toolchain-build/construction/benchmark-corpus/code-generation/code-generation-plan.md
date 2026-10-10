# Code Generation Plan — benchmark-corpus (U16)

This plan builds the benchmark corpus: the 40 approved programs in four classes, each in rAiL, Rust, Go and OCaml, written from one prompt per program and reviewed for fairness. It also builds the std-only Rust corpus checker with its sandbox wrapper, the per-program input generators, the shared helper, the index and toolchain pins, CI for macOS and Linux, and the spec note. The corpus lives in `corpus/`, outside the Cargo workspace. Tests come first (TDD): the checker is built Red, Green, Refactor before any program, and each program's fixed inputs and expected outputs are its tests. Classes are delivered one at a time, each approved by you before the next starts.

## Sources

- Functional design: `construction/benchmark-corpus/functional-design/` (`functional-spec.md` with the approved program list and WF1–WF4, `rules.md` BR1.1–BR9.2, `entities.md`).
- NFR requirements: `construction/benchmark-corpus/nfr-requirements/` (`security-requirements.md` NFR3.8, NFR3.9, NFR5.9, NFR6.3, NFR7.8, NFR9.19, NFR9.20, NFR10.5, NFR10.6; `tech-stack-decisions.md`).
- NFR design: `construction/benchmark-corpus/nfr-design/security-design.md`.
- Contracts: `inception/contract-design/contract-summary.md` (C10). Unit: `inception/units-generation/unit-of-work.md` (U16). Requirement: FR54.
- There are no user stories. Each step traces to FR54 and the rules and NFR requirements it implements.

## Decisions carried from the design reviews

| Point | Resolution in this plan |
|---|---|
| Linux sandbox reads (NFR design review R-01) | `bwrap` binds only what a program needs, read-only: `/usr`, `/lib*`, `/etc/ld.so*`, the program's own build directory and its run directory. The rest of the file system is not visible. A self-test checks that reading the user's home directory fails. |
| bwrap on GitHub runners (NFR design review R-01) | The Linux job enables unprivileged user namespaces with `sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0` (the documented setting on current Ubuntu images) before the sandbox self-test. If the self-test still fails, the job fails closed and you are told; it never runs programs unconfined. |
| Run rules inside the sandbox (R-02) | The wrapper keeps `HOME` and `TMPDIR` inside the run directory and passes only relative paths. The root check for permission-denied cases uses the real user id, read before the sandbox starts. |
| Builds unsandboxed (R-03) | Builds run offline from vendored, reviewed sources only. Whether `rayon`'s crates use build scripts is shown to you with its crate list. Builds themselves are not sandboxed. |
| Per-program deadline and orphans (R-04) | Each program run has a 60-second deadline. Every program starts in its own process group, and the whole group is killed at the deadline (as in the acceptance harness), which also covers macOS, where `sandbox-exec` has no die-with-parent. |
| bubblewrap version (R-05) | CI installs the distribution's `bubblewrap` package; the checker records its version in its output and checks it against a minimum in `corpus/toolchains.toml`. |
| OCaml floating point (FD review R-01) | Strict, as you chose: products bound to their own `let` before adding; a mismatch with Rust is a defect reported to you and reworked before the class is approved. |
| parallel-monte-carlo-pi (FD review R-08) | Its final division and print are floating point, so its index entry has `floating_point = true` and it follows BR2.4. |
| Index field names (FD review R-09) | The index uses exactly the `IndexEntry` attribute names from `entities.md`. |

## Prerequisites (before Step 1)

- [x] **P1. Bolt branch.** Create `bolt/benchmark-corpus` from the current `bolt/acceptance-harness` head (the corpus check joins the same CI). Nothing is pushed or opened as part of this plan.
- [x] **P2. Go, OCaml, dune, opam.** These are not installed on your Mac. You install them yourself, because downloading and running installers is yours to do: Go from `https://go.dev/dl/`, and OCaml with opam from `https://opam.ocaml.org/`, then `opam install dune`. Code generation records the exact installed versions in `corpus/toolchains.toml`, after checking they are the newest stable releases on the official sites. Code generation waits for your confirmation that they are installed.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "Tests come first everywhere: at the start of each phase we write that phase's acceptance-suite harness so that it runs and fails, and for each unit we write its tests so that they fail before we write the code they check, then write the code until they pass.",
  "scope": "rail-toolchain",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "Tests are part of every Bolt. The work is not finished until its tests are\nwritten first and pass.\n\n- **Methodology**: tdd\n- **Ordering**: Tests come first everywhere: at the start of each phase we write that phase's acceptance-suite harness so that it runs and fails, and for each unit we write its tests so that they fail before we write the code they check, then write the code until they pass.\n\nAdditional notes (these do not replace the two fields above):\n\n- The spec's acceptance suites (T1–T14) are the top-level proof of\n  correctness. Each phase's exit criteria come from the approved scope\n  document, not from the raw spec roadmap, because the scope document changed\n  several exits (for example, the first usable slice exits when T1, T2, T4, T5\n  and T6 pass at CI volume).\n- Everyday CI runs the acceptance suites at scaled-down volumes, and the spec's\n  full volumes run as the release check. Each suite's CI volume and seed policy\n  is recorded in the repository. Reducing a recorded CI volume counts as\n  lowering a pass criterion.\n- Coverage: at least **80% line coverage** for the Rust code, measured on\n  Linux, required before merge. The benchmark corpus, the test harnesses and\n  code generated from the grammar are excluded. The floor can go up but never\n  down. rAiL code (such as the standard library) has no coverage gate; its\n  quality rests on the acceptance suites.\n- CI runs at three levels:\n  - **Every pull request:** a quick check on GitHub-hosted macOS and Linux\n    runners.\n  - **Nightly:** larger volumes and short fuzz runs.\n  - **Release check:** full volumes, on the owner's own machines.\n- Performance numbers (T13) count only when they come from the owner's\n  reference machines. Results from hosted CI runners are never T13 evidence.\n- Every defect fix comes with a test that reproduces the defect.\n- Coverage floors, acceptance-suite pass criteria and recorded CI volumes are\n  never lowered to make a build pass."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:b9183e58b54fa6604be6ddd107184202051b33ba4f1c00854e5d1855278b3767",
  "contract_sha256": "sha256:b8ec90f0e1a2d843dec5f3f2a9212be14ca6a78cba2c41a0b0386c3c9bb1fb25"
}
```

## How the contract's layers map to this unit

| Contract layer | In the corpus |
|---|---|
| Data model | The index, toolchain pins and their validation (BR1, BR5.3) |
| Repository / data access | Reading the corpus tree, running generators, writing expected outputs through bless (BR7) |
| Business logic | The checker: composition, determinism, comparison, thread counts, sandboxed runs, confinement checks, budget (BR2, BR6, BR8.1, NFR5.9, NFR9.19) |
| API / endpoint | `corpus/check.sh` and the checker's command line; the programs themselves, whose fixed inputs and expected outputs are their tests |
| Frontend | Not applicable |

Each Red step runs the unit-scoped command from `unit-test-instructions.md` and records the failing output in `code-summary.md` (not in this plan, whose text is fixed by your approval).

## Plan

### Step 1 — Project structure and production configuration skeleton

- [x] Create `corpus/` with `index.toml` (schema version 1, no entries yet), `toolchains.toml` (Rust 1.98.1 from `rust-toolchain.toml`; Go, OCaml and dune as installed in P2; the bubblewrap minimum version), `fairness-checklist.md`, `deny.toml`, `README.md`, and one directory per class. Add `corpus/` to the root `Cargo.toml` `exclude` list so the workspace never builds it (BR5.4).
- [x] Create the checker package `corpus/tools/check/` (std-only Rust, own lockfile), the helper `corpus/tools/helper/`, the macOS sandbox profile `corpus/tools/sandbox/macos.sb`, and `corpus/check.sh`.

### Step 2 — Bootstrap the test runner and the unit-scoped command

- [x] Add the checker's test files and small synthetic corpora under `corpus/tools/check/tests/`.
- [x] Confirm `cargo test --locked --manifest-path corpus/tools/check/Cargo.toml` runs (zero tests).

### Step 3 — Data model — Red

- [x] Tests for the index (ten programs per class, approved names, layout, all `IndexEntry` fields, definition-ID format and uniqueness in `main.rlc`) and the toolchain pins (a version mismatch is refused). Record the failing output.

### Step 4 — Data model — Green

- [x] Implement index and pin parsing with a small std-only reader for the TOML subset these files use (the corpus adds no crates to the checker), and the validations (BR1.1–BR1.4, BR3.3, BR5.3).

### Step 5 — Data model — Refactor

- [x] Tidy messages so each names the file, program and field; tests stay green.

### Step 6 — Repository / data access — Red

- [x] Tests: a generator run twice gives identical files (BR7.1); a committed fixed input over 64 KiB, or more than eight, is refused (BR7.3); bless writes `expected/` from the Rust version only, and normal mode never writes (BR7.2); tree listings ignore timestamps (BR2.5). Record the failing output.

### Step 7 — Repository / data access — Green

- [x] Implement generator invocation, size checks, bless mode and tree listings with content checksums (a std-only SHA-256 in the checker, checked against published test vectors).

### Step 8 — Repository / data access — Refactor

- [x] Tidy; tests stay green.

### Step 9 — Business logic — Red

- [x] Tests on synthetic programs: a one-byte output mismatch fails with the first differing byte (BR2.2); a thread-count-dependent program fails (BR2.3); a program writing outside its run directory, reading the user's home directory, or opening a socket is stopped by the sandbox and reported (NFR5.9); a missing sandbox tool refuses to run anything; a program past its 60-second deadline is killed with its whole process group; permission-denied cases are created at run time and skipped with a notice as root (BR6.5); the canary and tree check catch a stray write (BR6.4); a disallowed dependency in a manifest fails (BR5.1, BR5.2); the budget stop records the rest as not run (NFR9.19). Record the failing output.

### Step 10 — Business logic — Green

- [x] Implement the build step (offline, pinned toolchains, vendored libraries), the sandboxed runner (`sandbox-exec` on macOS, `bwrap` on Linux, each with a self-test), comparisons, thread-count runs, confinement checks, manifest checks and the budget.

### Step 11 — Business logic — Refactor

- [x] Tidy; tests stay green.

### Step 12 — API / endpoint — Red

- [x] Tests for `corpus/check.sh` and the checker's command line: `--class`, `--program`, `--bless`, exit codes 0/1/2, and one output line per program and language. Record the failing output.

### Step 13 — API / endpoint — Green

- [x] Implement the command line and `check.sh`.

### Step 14 — API / endpoint — Refactor

- [x] Tidy; tests stay green.

### Step 15 — Frontend behavior

- [x] Not applicable: the corpus has no user interface.

### Step 16 — Class 1: pure-transform (WF1, WF2)

- [ ] For each of the ten approved programs: write `prompt.md` (task, byte-exact formats, error cases, run contract, and the operation order for n-body and spectral-norm); generate the rail, Rust, Go and OCaml versions from it, each in its own session with one model, recording the model id and date; write the generator and fixed inputs; add the index entry; bless expected outputs from Rust; and run the corpus check until all three runnable languages pass on this Mac.
- [ ] Run the AI fairness review for each program in its own session, filling in `fairness-review.md`.
- [ ] **Stop for your approval of the class.** Record it in `corpus/pure-transform/APPROVAL.md`. Class 2 does not start until then.

### Step 17 — Class 2: parsing-text

- [ ] As Step 16, for the ten parsing-text programs, then **stop for your approval**.

### Step 18 — Class 3: io-services

- [ ] As Step 16, for the ten io-services programs, with fixtures, tree listings, run-time permission cases, and the helper for process-pipeline, then **stop for your approval**.

### Step 19 — Class 4: data-parallel

- [ ] Before writing any program: resolve `rayon` and `domainslib` at their newest versions, vendor them under `corpus/vendor/`, and **stop to show you the full resolved crate list for `rayon`** (name, version, licence, reason, build scripts) and the OCaml package list with licences, for your approval before the lockfiles are committed (NFR7.8).
- [ ] Then as Step 16, for the ten data-parallel programs (`--threads`, fixed reduction order), and **stop for your approval**.

### Step 20 — Environment and build configuration

- [ ] `.github/workflows/corpus.yml`: the corpus job on every pull request on macOS and Linux, passing early when nothing in `corpus/`, `rust-toolchain.toml` or the workflow changed; `actions/setup-go` and `ocaml/setup-ocaml` pinned to commit SHAs read from GitHub; the bubblewrap install and the user-namespace setting on Linux; build caching keyed on the pins and lockfiles; `cargo deny check` over the corpus Rust packages; `timeout-minutes: 45`. A nightly run with `timeout-minutes: 120`.
- [ ] `docs/corpus.md`: running the check locally, installing the toolchains and bubblewrap, and the class approval process.

### Step 21 — Documentation and traceability

- [ ] **Spec (BR9.2).** Add the corpus conventions to §11.1 of `spec/rail-language-spec.md` (location, library policy, no networking in the I/O class, the floating-point rule) and re-sync the knowledge copy with `aidlc engine knowledge sync`.
- [ ] Write `code-summary.md` (with the Red output), `source-manifest.json` and `traceability.json` for this unit.

## Traceability

| Plan steps | Implements |
|---|---|
| 1, 3–5 | BR1.1–BR1.4, BR3.3, BR5.3, BR5.4 |
| 6–8 | BR2.5, BR7.1–BR7.3 |
| 9–11 | BR2.2, BR2.3, BR5.1, BR5.2, BR6.4, BR6.5, BR8.1, NFR3.8, NFR3.9, NFR5.9, NFR9.19, NFR9.20 |
| 12–14 | BR8.1 |
| 16–19 | FR54, BR1.2, BR2.1, BR2.4, BR3.1–BR3.4, BR4.1, BR4.2, BR6.1–BR6.3, BR6.6, BR9.1, NFR7.8 |
| 20 | BR5.5, NFR6.3, NFR10.5, NFR10.6 |
| 21 | BR9.2, BR8.2 (handoff), FR54 |
