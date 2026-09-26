# Team Practices — rAiL Toolchain

How we build the rAiL toolchain, as affirmed by the owner in the practices-discovery interview (Q1–Q15) and its confirmed summary.

## Way of Working

We use **trunk-based development** on `main`. All work lands on `main` through
short-lived branches, usually merged within one or two days. We avoid
long-lived branches because they pile up merge debt.

For Construction worktrees, the base branch is `main` and the merge target is
`main`.

Every branch reaches `main` through a GitHub pull request on
`NickelsKent/rAiL`. A pull request merges only when CI is green, and we
**squash-merge** it, so each piece of work (each Bolt) becomes one commit on
`main`, named after the Bolt. The full commit history stays on the source
branch until the worktree is discarded, and the AI-DLC audit log keeps the full
event record. We do not commit straight to `main`.

AI agents write most of the code, and the owner is the only human on the
project. Every pull request needs two things before it merges: an automated AI
review, and the owner's approval.

CI runs the full set of security checks below. Unless noted, each runs on
every pull request and a failure blocks the merge:

- Crate checks with `cargo-deny`: known-vulnerable crates, banned crates,
  allowed sources and allowed licenses. The vulnerability check also runs on a
  weekly schedule, because new advisories appear without any code change.
- Secret scanning with `gitleaks`, plus GitHub's own secret scanning.
- Builds and tests that must match the committed lockfile exactly.
- A short fuzz run, at least in the nightly run (see Testing Posture). The
  long fuzz run belongs to the release check.
- Third-party CI actions pinned to exact versions, with read-only permissions
  by default.

There is no running web service, so live-service scanning (DAST) does not
apply to us. Fuzzing, memory-error checks (sanitizers) and the sandbox escape
suite (T12) take its place. This is a deliberate decision, not a gap.

The project is licensed **MIT OR Apache-2.0** (dual license). The dependency
license check allows licenses that fit this choice.

The authoritative copy of the rAiL spec lives in a `spec/` folder in the
repository, next to the code. The copy in the project knowledge folder is
synced from it and is never edited on its own. When a decision changes what the
spec says, the spec changes in the same pull request.

Releases come from tags on `main`, never from long-lived release branches.

## Walking Skeleton

We build a walking skeleton first. A walking skeleton is a minimal version that
runs the whole way through, built first to prove the pieces connect before the
real features go in.

Our skeleton is a one-function rAiL module that is parsed, checked, served
through the agent protocol server, then compiled and run as a native binary on
both macOS and Linux.

Reaching a native binary pulls a thin piece of spec Phase 2 (code generation)
forward. This is a **one-time exception** to phase order. After the skeleton,
work continues in the spec's phase order, and the agent loop (spec Phases 0–1)
stays the first usable slice.

The skeleton is verified with one recorded command that the owner approves and
that demonstrates this end-to-end result on both platforms. The owner approves
the working skeleton before later Units start.

## Testing Posture

Tests are part of every Bolt. The work is not finished until its tests are
written first and pass.

- **Methodology**: tdd
- **Ordering**: Tests come first everywhere: at the start of each phase we write that phase's acceptance-suite harness so that it runs and fails, and for each unit we write its tests so that they fail before we write the code they check, then write the code until they pass.

Additional notes (these do not replace the two fields above):

- The spec's acceptance suites (T1–T14) are the top-level proof of
  correctness. Each phase's exit criteria come from the approved scope
  document, not from the raw spec roadmap, because the scope document changed
  several exits (for example, the first usable slice exits when T1, T2, T4, T5
  and T6 pass at CI volume).
- Everyday CI runs the acceptance suites at scaled-down volumes, and the spec's
  full volumes run as the release check. Each suite's CI volume and seed policy
  is recorded in the repository. Reducing a recorded CI volume counts as
  lowering a pass criterion.
- Coverage: at least **80% line coverage** for the Rust code, measured on
  Linux, required before merge. The benchmark corpus, the test harnesses and
  code generated from the grammar are excluded. The floor can go up but never
  down. rAiL code (such as the standard library) has no coverage gate; its
  quality rests on the acceptance suites.
- CI runs at three levels:
  - **Every pull request:** a quick check on GitHub-hosted macOS and Linux
    runners.
  - **Nightly:** larger volumes and short fuzz runs.
  - **Release check:** full volumes, on the owner's own machines.
- Performance numbers (T13) count only when they come from the owner's
  reference machines. Results from hosted CI runners are never T13 evidence.
- Every defect fix comes with a test that reproduces the defect.
- Coverage floors, acceptance-suite pass criteria and recorded CI volumes are
  never lowered to make a build pass.

## Deployment

rAiL has no hosted service, so there are no staging or production servers.
Here, "deployment" means **publishing release builds of the `rail` toolchain**
for macOS and Linux. Windows comes later.

- Every merge to `main` runs CI on both macOS and Linux. This takes the place
  of the default "deploy to staging on merge".
- A **release** is a version tag on `main`, starting at `v0.1.0`. A release is
  cut only after the full-volume release check passes and the owner approves it
  by hand. The owner's approval takes the place of the default tech lead and
  product owner sign-off.
- From the first release, every release ships checksums, a record of how it was
  built, and a list of its dependencies.
- Signed, reproducible packages for rAiL users arrive with spec Phase 4.
- Nothing is published to a public package registry. The package registry runs
  locally only.
- A pushed release tag is never moved or deleted. Rollback means the previous
  tagged release stays available, and a bad release is fixed with a new tag.

## Code Style

We use the standard tools and let them decide. When an agent suggests a style
change, it checks the formatter and linter configuration first, and only makes
the suggestion if those tools do not already cover it.

**Rust toolchain**

- One pinned stable Rust version is used for building, testing and releases.
- A separate pinned nightly version is used only for fuzzing, memory-error
  checks (sanitizers) and `miri`. Nothing we release is built with nightly.

**Dependencies**

- The runtime has zero dependencies beyond the platform C library.
- The compiler may use a small number of crates. Each one needs a stated reason
  and the owner's approval.
- Test-only crates are allowed, because they never ship. They also need a
  stated reason and the owner's approval.
- This narrows the spec's "zero dependencies" target (§11.2, T14), and the
  spec is updated to match.

**Rust conventions**

- `rustfmt` with its default settings.
- `clippy` on all code, including tests, with warnings treated as errors. Both
  run in CI, and a failure blocks the merge.
- Rust 2024 edition.
- One Cargo workspace with several crates, building one `rail` program.
- `unsafe` code is forbidden except in the allowed runtime and backend crates,
  and every `unsafe` block carries a safety comment.
- Problems in the user's rAiL program are reported as diagnostics. Tool
  failures are typed errors. One bad request never crashes the agent protocol
  server.
- Library code never prints to standard output.
- Output is deterministic: it never depends on hash-map ordering, clocks or
  randomness.
- Expected-output test files change only through an explicit "bless" step, and
  the resulting diff is reviewed in the pull request.
- Names follow Rust's own conventions: `snake_case` for functions and modules,
  `CamelCase` for types.

**rAiL source files and the repository**

- rAiL source files (`.rlc`, `.rlh`) use the canonical form, which the parser
  enforces. Once `rail fmt` exists, it formats all rAiL files in the repository.
- LF line endings everywhere, as `.gitattributes` already enforces.
