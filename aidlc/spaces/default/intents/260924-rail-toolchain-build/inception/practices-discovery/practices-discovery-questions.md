# Practices Discovery — Questions

These questions settle how the rAiL toolchain gets built day to day: how changes reach `main`, what gets built first, how testing works, how releases are cut, and the code conventions agents follow. The release engineer drafted suggestions from the framework defaults and the rAiL spec; the quality engineer, developer and security engineer reviewed that draft. Each question lists the suggested answer first where there is one.

Already settled earlier (not asked again): the first usable slice is the agent loop, Phases 0–1 (scope Q1); build order follows the spec's phase order where parts depend on each other (scope Q7); CI runs acceptance suites at scaled-down volumes with full volumes as a release check (scope Q5); macOS and Linux are required, Windows later (scope Q3); nothing is published to a public registry (intent Q11).

## Q1. How should changes reach `main`?

The repository is on GitHub (`NickelsKent/rAiL`). The one pull request so far was merged with a merge commit, while the framework default is one squashed commit per piece of work.

A. Short-lived branches, a GitHub pull request for each, green CI required, squash-merged into one commit per piece of work (suggested)
B. Same, but keep merge commits (as PR #1 did)
C. Commit straight to `main`; CI runs afterwards
X. Other (please specify)

[Answer]: A

## Q2. Who has to approve a pull request before it merges?

AI agents will write most of the code, and you are the only human on the project.

A. Your approval alone
B. Your approval plus an automated AI review on every pull request
C. Your approval, plus a focused security review only on pull requests that touch sensitive areas (runtime memory code, the sandbox and permission checks, the agent protocol server, parsers of untrusted input)
D. Both B and C
X. Other (please specify)

[Answer]: B

## Q3. Build a thin end-to-end slice first? A walking skeleton is a minimal version that runs the whole way through, built first to prove the pieces connect before the real features go in.

Earlier you chose the agent loop (Phases 0–1) as the first usable slice and the spec's phase order for sequencing. A skeleton that reaches a native binary would pull part of Phase 2 (code generation) forward.

A. Yes, all the way to a native binary: a one-function rAiL module that is parsed, checked, served through the agent protocol server, compiled and run on macOS and Linux
B. Yes, but stop at the agent loop: a one-function module that is parsed, formatted, checked and patched through the agent protocol server, with no native code yet (keeps the phase order)
C. No skeleton: build in phase order, layer by layer
X. Other (please specify)

[Answer]: A

## Q4. When do tests get written relative to the code?

The spec already defines 14 acceptance suites (T1–T14), each with an automated pass check.

A. Tests after each layer: build a layer, then write and run its tests before the next layer (framework default)
B. Tests first (test-driven development) for everything
C. Mixed: at the start of each phase, write that phase's acceptance-suite harness first so it runs and fails; then build each layer and write its unit tests right after it (quality engineer's suggestion)
X. Other (please specify)

[Answer]: B

## Q5. What code-coverage rule should the Rust code meet before merging?

A. At least 80% line coverage, measured on Linux; the benchmark corpus, test harnesses and code generated from the grammar are excluded; the floor can go up but never down (suggested)
B. At least 80% line coverage, no exclusions or ratchet
C. No coverage number; the acceptance suites and reviews are enough
D. A stricter floor (please give the number and whether line or branch coverage)
X. Other (please specify)

[Answer]: A

## Q6. Which Rust compiler version do we build with?

Pinning one version keeps lint results and builds stable. But the memory-error checks and fuzzing in the T10 suite only work on the "nightly" (pre-release) Rust compiler.

A. Pin one stable version for building, testing and releases, plus a separate pinned nightly version used only for fuzzing, memory-error checks and `miri` (suggested)
B. Pin one stable version only; T10 waits until those tools work on stable
C. Always use the latest stable version, unpinned
X. Other (please specify)

[Answer]: A

## Q7. The spec aims for zero dependencies. What does "zero" cover for us?

The spec (§11.2, T14) says the toolchain resolves zero external packages, and the runtime depends only on the platform C library. It is unclear whether that also rules out Rust libraries ("crates") inside the compiler or in tests. Without test crates, property testing and fuzzing harnesses would have to be written by hand.

A. The runtime has zero dependencies; the compiler may use a small number of crates, each with a stated reason and your approval; test-only crates are allowed because they never ship (suggested)
B. The runtime and compiler both have zero crates; test-only crates are allowed
C. Zero crates anywhere, including tests
X. Other (please specify)

[Answer]: A

## Q8. Where do the heavy checks run?

Some checks are long or need specific hardware: full-volume suites, the 24-hour fuzz run, performance runs that must use the spec's reference machines, and model measurements that need API keys.

A. Three levels: every pull request runs a quick check on GitHub-hosted macOS and Linux; a nightly run adds larger volumes and short fuzzing; the full-volume release check runs on your own machines; performance numbers only count from your reference machines (suggested)
B. Two levels: pull-request check and release check only, no nightly
C. Everything on your own machines, connected to GitHub as self-hosted runners
X. Other (please specify)

[Answer]: A

## Q9. How are releases of the `rail` toolchain cut?

There is no hosted service, so a "deployment" here means publishing release builds for macOS and Linux.

A. A version tag on `main` (starting at `v0.1.0`), cut after the full-volume release check and your approval; each release ships checksums, a record of how it was built, and a list of its dependencies from the first release; signed, reproducible packages for rAiL users arrive with Phase 4; a pushed tag is never moved (suggested)
B. Same tags and approval, but no checksums or build records until Phase 4
C. No releases during this work; people build from source
X. Other (please specify)

[Answer]: A

## Q10. Which security checks run in CI?

There is no running web service, so the usual live-service security scanning does not apply; fuzzing, memory-error checks and the sandbox escape suite (T12) take its place.

A. Full set: known-vulnerable and banned crates, allowed sources and licenses (`cargo-deny`, also run weekly); secret scanning (`gitleaks` plus GitHub's own); builds that must match the lockfile; a short fuzz run; third-party CI actions pinned to exact versions with read-only permissions (suggested)
B. Minimal: vulnerable-crate check and secret scanning only
C. None until Phase 4
X. Other (please specify)

[Answer]: A

## Q11. Which license should the project use?

The repository has no license file. The dependency check's license allowlist depends on this.

A. MIT OR Apache-2.0, dual-licensed (the Rust ecosystem's usual choice)
B. Apache-2.0 only
C. MIT only
D. Not open source / decide later
X. Other (please specify)

[Answer]: A

## Q12. What Rust code conventions should agents follow?

The developer suggested conventions beyond the formatter and linter, because agents writing Rust need them most.

A. The full set: `rustfmt` defaults; `clippy` on all code including tests, warnings as errors; Rust 2024 edition; one Cargo workspace with several crates building one `rail` program; `unsafe` code forbidden except in allowed runtime and backend crates, each block with a safety comment; problems in the user's rAiL program are reported as diagnostics, tool failures as typed errors, and one bad request never crashes the agent protocol server; library code never prints to standard output; deterministic output (no hash-map ordering, clocks or randomness); expected-output test files updated only by an explicit "bless" step (suggested)
B. Only `rustfmt` and `clippy` now; the rest is decided during design
X. Other (please specify)

[Answer]: A

## Q13. Where does the authoritative copy of the rAiL spec live?

The only copy is in the project knowledge folder (`aidlc/spaces/default/knowledge/documents/`), which the workflow re-reads with a sync step after each edit.

A. Keep it there, and re-sync the knowledge catalog after every edit
B. Move the authoritative copy to a `spec/` folder in the repository, next to the code, and keep the knowledge copy in sync from it
X. Other (please specify)

[Answer]: B

## Q14. Which of these should become firm rules that are never broken? (select all that apply)

A. Never lower an acceptance-suite pass criterion, its recorded CI volume, or a coverage floor to make a build pass
B. Never commit signing keys, model API keys or other secrets to the repository
C. When a decision changes what the spec says, update the spec in the same change
D. Never put timestamps, absolute paths, hostnames or random seeds into compiler output
E. Every merge to `main` passes CI on both macOS and Linux
F. Never move or delete a release tag once pushed
G. Every defect fix comes with a test that reproduces it
H. Every new Rust dependency needs a stated reason and your approval
I. Never publish to a public package registry as part of this work
X. Other (please specify)

[Answer]: A, B, C, D, E, F, G, H, I

## Q15. Q3 chose a walking skeleton that reaches a native binary, but earlier (scope Q7) you chose to follow the spec's phase order, with the agent loop (Phases 0–1) as the first usable slice (scope Q1). How do these fit together?

A thin native path needs a small piece of Phase 2 (code generation) before Phase 1 is finished.

A. One-time exception: build the thin native path first as the skeleton, then continue in phase order; the agent loop stays the first usable slice
B. Change the sequencing for all the work: skeleton first, then widen every layer together, instead of phase order
C. Keep phase order strictly: the skeleton stops at the agent loop, with no native code yet
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Way of working:** short-lived branches, a GitHub pull request for each, green CI required, squash-merged into one commit per piece of work [Q1]. Each pull request needs your approval plus an automated AI review [Q2].
- **Walking skeleton:** yes. A one-function rAiL module goes through parsing, checking and the agent protocol server, then is compiled and run as a native binary on macOS and Linux [Q3]. This is a one-time exception to phase order: after the skeleton, work continues in the spec's phase order, and the agent loop stays the first usable slice [Q15].
- **Testing:** tests first (test-driven development) [Q4]. At least 80% line coverage for the Rust code, measured on Linux. The corpus, test harnesses and grammar-generated code are excluded, and the floor can only go up [Q5].
- **Rust toolchain:** one pinned stable version for building, testing and releases, plus a separately pinned nightly version used only for fuzzing, memory-error checks and `miri` [Q6].
- **Dependencies:** the runtime has zero dependencies. The compiler may use a few crates, each with a stated reason and your approval. Test-only crates are allowed [Q7]. This narrows the spec's "zero dependencies" target (§11.2), so the spec will be updated to match.
- **CI levels:** a quick check on every pull request on GitHub-hosted macOS and Linux; a nightly run with larger volumes and short fuzzing; the full-volume release check on your own machines. Performance numbers only count from your reference machines [Q8].
- **Releases:** version tags on `main` starting at `v0.1.0`, cut after the full-volume check and your approval. From the first release, each ships checksums, a build record and a dependency list. Signed, reproducible packages for rAiL users come with Phase 4. A pushed tag never moves [Q9].
- **Security checks in CI:** the full set: crate vulnerability, ban, source and license checks (also weekly); secret scanning; lockfile-exact builds; a short fuzz run; third-party CI actions pinned with read-only permissions [Q10].
- **License:** MIT OR Apache-2.0 [Q11].
- **Rust conventions:** the full set: `rustfmt`; `clippy` on all code with warnings as errors; Rust 2024; one Cargo workspace of several crates building one `rail` program; `unsafe` only in allowed crates, each use commented; typed errors, and one bad request never crashes the protocol server; no printing from library code; deterministic output; expected-output files change only through an explicit approve step [Q12].
- **Spec location:** the authoritative spec moves to a `spec/` folder in the repository, and the knowledge copy is synced from it [Q13].
- **Firm rules, all nine:** never lower a pass criterion, CI volume or coverage floor; no secrets in the repository; update the spec in the same change as any decision that alters it; no timestamps, absolute paths, hostnames or random seeds in compiler output; every merge passes CI on macOS and Linux; release tags never move; every defect fix comes with a reproducing test; every new Rust dependency needs a reason and your approval; nothing is published to a public package registry [Q14].

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
