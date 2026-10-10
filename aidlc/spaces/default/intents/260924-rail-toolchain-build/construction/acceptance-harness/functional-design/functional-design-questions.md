# Functional Design — acceptance-harness (U2) — Questions

The acceptance harness is the framework that runs the spec's 14 acceptance suites (T1–T14, spec §12). It holds each suite's recorded volumes and seed policy, and it drives `rail` only from the outside, through the command line and the agent protocol. It holds no suite content of its own: each later unit adds its suites here before writing its code.

Already settled (not asked again):
- Everyday CI runs scaled-down volumes, and the spec's full volumes run as a separate release check (scope Q5).
- CI runs at three levels: a quick check on every pull request (GitHub-hosted macOS and Linux), a nightly run with larger volumes and short fuzzing, and the full-volume release check on your own machines (practices Q8).
- Reducing a recorded CI volume counts as lowering a pass criterion, which is a firm rule (practices Q14).
- Suites drive `rail` as a black box, through `--json` command output or the protocol only (contract C9, ADR-006).
- Each suite has `generate_inputs` (seed to inputs, with the seed logged), `run` and `judge` steps (contract C9).
- macOS and Linux are required. Windows comes later.
- New Rust crates need a stated reason and your approval. Test-only crates are allowed on the same terms.

## Q1. What format should the recorded suite settings file use?

Contract design left this open (contract C9). The file records each suite's volumes, seed policy, platforms and inputs, and changes to it are reviewed in pull requests because lowering a volume breaks a firm rule. TOML would need a new crate. The workspace already has its own strict JSON reader and writer (`rail-json`). Canonical rAiL text does not exist yet: it arrives with the syntax unit (U3).

A. JSON, read with the in-house `rail-json` crate: no new dependency, and the output is deterministic. Stored at `tests/acceptance/suites.json` (recommended)
B. TOML, read with the `toml` crate. Easier to edit by hand, but it needs a new crate and your approval
C. JSON now, switched to canonical rAiL text once the syntax unit (U3) lands
D. A Rust table compiled into the harness, with no separate file
X. Other (please specify)

[Answer]: B

## Q2. How is the harness packaged and run?

The walking skeleton already created a test-only crate, `rail-testkit`, which is never shipped. It holds golden files, a process runner and a protocol client, and its header says U2 extends it. CI will call the harness at the three levels.

A. Extend `rail-testkit` with the harness library and a small `rail-accept` program, run as `cargo run -p rail-testkit --bin rail-accept -- --level pr|nightly|release [--suite T1]` (recommended)
B. A new crate, `rail-accept`, that depends on `rail-testkit`
C. No separate program: each suite is a `cargo test` integration test, and an environment variable picks the volume
X. Other (please specify)

[Answer]: A

## Q3. How many volumes does each suite record?

Contract C9 lists two volumes per suite: a CI volume and a full volume. Your practices have three CI levels: pull request, nightly and release.

A. Three recorded volumes per suite (pull request, nightly and full). Lowering any of them counts as lowering a pass criterion (recommended)
B. Two volumes as in the contract (CI and full). The nightly run uses the CI volume
C. Two volumes. The nightly run uses a fixed multiple of the CI volume, set once in the harness
X. Other (please specify)

[Answer]: A

## Q4. Who sets each suite's scaled-down volumes, and when?

Requirements analysis left the exact CI volumes as an open design question. The full volumes come straight from spec §12 (for example 10⁶ generated modules for T1). A scaled-down volume that is set too high slows every pull request. One set too low cannot be raised later without effort, and cannot be lowered at all.

A. The harness records every suite's full volume from spec §12 now. Each scaled-down volume is set by the unit that adds that suite, in that unit's design, and is recorded when the suite lands (recommended)
B. The harness sets every suite's scaled-down volumes now, for all 14 suites
X. Other (please specify)

[Answer]: A

## Q5. How are seeds chosen for generated inputs?

Several suites generate their inputs from a seed (for example T1's generated modules and T9's generated inputs). The firm rule against random seeds applies to compiler output, not to test inputs, but a failure must always be reproducible.

A. Pull request runs use fixed seeds recorded in the settings file. Nightly and release runs draw a fresh seed, print it, and on failure print the exact command that reruns with that seed (recommended)
B. Fixed recorded seeds at every level
C. A fresh seed at every level, always printed
X. Other (please specify)

[Answer]: A

## Q6. How do suites that no unit has added yet appear in the harness?

Your testing practice says each phase starts by writing its acceptance suites so that they run and fail. The harness lands before most suites exist, and a suite that fails on every pull request would block all merges.

A. All 14 suites are registered now with their full volumes. A suite with no implementation yet reports "pending (owning unit)" and does not run at the pull request and nightly levels. The release check reports every pending suite as a failure, so no release can pass while any suite is pending (recommended)
B. Only suites that have an implementation are registered. Later units add both the settings and the code
C. All 14 are registered, and an unimplemented suite fails at every level, including pull requests
X. Other (please specify)

[Answer]: A

## Q7. Should the walking-skeleton check become part of the harness?

`scripts/verify-skeleton.sh` is the verification command you approved. It is re-run at every Unit checkpoint, and CI runs it on every pull request.

A. Yes. Add it as a smoke suite that runs at every level. The script stays as a thin wrapper around the harness, so the approved command does not change (recommended)
B. No. Keep the script separate from the harness
X. Other (please specify)

[Answer]: A

## Q8. What does a harness run report?

The results feed CI logs, and later the release check and any release evidence. Output must not depend on clocks or hash-map ordering (practices: Rust conventions).

A. A short human summary on the terminal, plus a deterministic JSON results file per run. The file holds, for each suite: level, volume, seed, platform, outcome, and the first failing input when there is one. Timings go only to the terminal summary, never into the results file (recommended)
B. The human summary only
X. Other (please specify)

[Answer]: A

## Follow-up questions

## Q9. Which TOML reader do you approve for the settings file?

You chose TOML for the settings file (Q1). Your practice is that every new Rust crate needs a stated reason and your approval, test-only crates included. The harness is test tooling and is never shipped, so this would be a test-only dependency of `rail-testkit`. Reason: read `tests/acceptance/suites.toml`, the reviewed record of each suite's volumes, seeds, platforms and inputs. The widely used `toml` crate also pulls in `serde` and a handful of supporting crates, and all of them go through the `cargo-deny` licence and advisory checks.

A. Approve the `toml` crate (with `serde` for reading into typed settings), as a test-only dependency of `rail-testkit` (recommended if you want standard TOML)
B. No new crate: write a small in-house reader for the TOML subset the file needs (tables, strings, integers, arrays), rejecting anything else with a located error
C. Go back to JSON with the in-house `rail-json` reader (Q1 option A)
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Settings file:** TOML at `tests/acceptance/suites.toml` (Q1), read with the `toml` crate plus `serde`, approved as a test-only dependency of `rail-testkit` (Q9).
- **Packaging:** the harness extends `rail-testkit` with a library and a `rail-accept` program, run as `--level pr|nightly|release [--suite T1]` (Q2).
- **Volumes:** each suite records three volumes (pull request, nightly and full). Lowering any of them counts as lowering a pass criterion (Q3).
- **Who sets volumes:** the harness records every suite's full volume from spec §12 now. The unit that adds a suite sets its pull-request and nightly volumes in its own design (Q4).
- **Seeds:** pull-request runs use fixed seeds recorded in the settings file. Nightly and release runs draw a fresh seed, print it, and on failure print the exact rerun command (Q5).
- **Pending suites:** all 14 suites are registered now. An unimplemented suite shows "pending (owning unit)" and is skipped at the pull-request and nightly levels. The release check counts it as a failure (Q6).
- **Skeleton check:** becomes a smoke suite that runs at every level. `scripts/verify-skeleton.sh` stays as a thin wrapper, so the approved verification command is unchanged (Q7).
- **Reporting:** a terminal summary plus a deterministic JSON results file per run (level, volume, seed, platform, outcome, first failing input). Timings appear only in the terminal summary (Q8).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
