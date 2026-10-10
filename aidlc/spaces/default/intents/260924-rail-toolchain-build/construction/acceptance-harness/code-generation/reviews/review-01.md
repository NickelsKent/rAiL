## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-09T01:44:54Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | crates/rail-testkit/src/harness/ratchet.rs > compare_settings (reference_machines handling) | The ratchet only prints a notice when `reference_machines` changes. I checked this by running `rail-accept --check-ratchet` with a base file that records one machine and a new file that records none. It printed `notice: reference_machines: the list changed`, then `ratchet: pass`, exit 0. Spec §12 says a release requires a passing T13 result from every recorded reference machine. A pull request can therefore remove a recorded machine, or empty the list, and lower the T13 pass criterion without the ratchet refusing it. This is the firm rule against lowering a pass criterion. | Refuse removal or replacement of a recorded reference machine in the ratchet (BR2.4 style), and add a test for it. Keep additions as a notice. Add the rule to the functional design in the same change. | New |
| R-02 | Minor | crates/rail-testkit/src/harness/process.rs > supervise (output reader threads) | The reader threads use `read_to_end` with no cap, so a process that floods output is buffered in memory until its deadline (60 s by default). The threat model lists "floods output" as a denial-of-service threat and relies on deadlines alone. At pipe speed that can exhaust a hosted runner before the deadline. T11 and T12 will run hostile programs through this runner. The `floods-output` helper's comment mentions an output cap, but only the `rail run` limits test uses it, not this runner. | Add a capture cap to the suite process runner. When exceeded, kill the process group and return an error outcome. Add a test using `floods-output`. | New |
| R-03 | Minor | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/code-generation/unit-test-instructions.md > Mocking and stubbing, "Time budgets" | The file says tests pass small budgets "through a test-only settings file". `code-summary.md` and the tests use the `RunOptions::budget` override instead, because the settings file only holds whole minutes. The instructions no longer describe how the tests work. | Correct the sentence to name the `RunOptions::budget` override. | New |
| R-04 | Minor | .github/workflows/ci.yml > jobs `compare` and `ratchet`; .github/workflows/nightly.yml > jobs `groups`, `acceptance`, `compare` | The new jobs have never run. They were only parsed as YAML. Unverified points: `gh run download --pattern` with the `acceptance/*/results-*.json` globs, the `fromJSON` matrix built from `--list-groups`, and the process-group kill through procps `/bin/kill` on Linux. CI is also required to pass on both macOS and Linux before merge. The permissions are read-only and the actions are pinned, and I found no policy violation. A broken compare or ratchet job would only show up on the first pull request. | Watch the first pull request's CI run on both platforms. Fix any job that fails before merge. | New |
| R-05 | Minor | crates/rail-testkit/src/harness/run.rs > observe, generate_twice | A suite's inputs and observations are held fully in memory (`Vec<Input>`, `Vec<Observation>`), and generation runs twice. S0 is unaffected. Full volumes of 10^5 to 10^6 inputs (T1, T4, T9) will make the owning units hit memory limits or force them to change the `Suite` interface late. | Record this in the Suite interface docs or the follow-ups so U3 and U7 plan for streaming or batched generation, or change the interface now while only S0 uses it. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo nextest run --locked -p rail-testkit -E 'binary(/^harness_/)'` | PASS: 73 run, 73 passed, 1 skipped (the intentional helper test) | Matches the claim. |
| Mutation checks on a scratch copy | Making `is_lower_than` always false failed 2 ratchet tests. Removing the platform comparison failed 2 ratchet tests. Removing `env_clear()` failed the `nfr5_7` environment test. | The tests exercise the rules they claim. No always-pass tests found. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS | Clean. |
| `cargo fmt --all --check` | PASS | Clean. |
| `cargo deny --locked check` | PASS: advisories, bans, licenses and sources ok | The 12 crates and Unicode-3.0 match the owner's 2026-10-08 approval, each documented in `docs/dependencies.md`. |
| `cargo tree --locked -p rail -e normal --target all -i toml` and `-i serde` | Nothing to print | Neither crate reaches the `rail` binary. |
| `scripts/check-forbid-unsafe.sh` | PASS | `forbid(unsafe_code)` holds. |
| `rail-accept --level pr` (run by hand) | PASS: S0 passes, T1 to T14 pending, exit 0. `--level release --suite T1` fails with exit 1. | Pending suites do not break pull-request CI and block a release. |
| `rail-accept --check-ratchet` on identical files | PASS, exit 0 | The CLI matches the workflow invocation. |
| `grep` for print, `HashMap`, `unsafe` and clocks in `src/harness` and `src/suites` | No hits | The library does not print and has no hash-map ordering in output. The only clock use is the budget watchdog and the run-directory name, neither of which reaches results. |
| Sensor: traceability (`traceability.json`) | PASS: 52 of 52 ids OK, each target an existing file, no upstream id missing. All 39 BR ids and all 12 NFR ids from the unit are covered. | Clean. |
| Source manifest against `git status` | PASS: every changed or untracked product path is claimed. No claim is unrelated to this unit. | Clean. |
| Plan checkboxes | 45 ticked, 0 open | The plan was followed. Deviations are listed in `code-summary.md`. |
| Firm rules | Full volumes in `suites.toml` match spec §12. Workflows keep read-only permissions and pin actions by commit. No secrets found. | No lowered volume or threshold, apart from the R-01 gap. |

### Summary

The implementation follows the approved plan. The 73 unit tests and the other checks pass, and the tests fail when the code is mutated. The one real gap is R-01: the ratchet accepts the removal of a recorded T13 reference machine, which lowers a pass criterion. That is a single Major finding, so the verdict is READY, but R-01 should be fixed before merge. The other findings are Minor.
