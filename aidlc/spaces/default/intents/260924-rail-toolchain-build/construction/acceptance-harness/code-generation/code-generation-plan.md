# Code Generation Plan — acceptance-harness (U2)

This plan builds the acceptance harness: a library and a `rail-accept` program inside the never-shipped `rail-testkit` crate. The harness reads the reviewed settings file `tests/acceptance/suites.toml` and registers all 15 suites (S0 and T1–T14). It runs them at the pull-request, nightly or release level, and drives `rail` only through its command line and protocol. It also checks the volume ratchet, compares platforms, and writes deterministic results. S0, the walking-skeleton check, is the only suite implemented in this unit; T1–T14 are registered as pending for their owning units. Tests come first (TDD): each layer goes Red, Green, Refactor.

## Sources

- Functional design: `construction/acceptance-harness/functional-design/` (`functional-spec.md` WF1–WF6, `rules.md` BR1.1–BR9.2, `entities.md`).
- NFR requirements: `construction/acceptance-harness/nfr-requirements/` (`security-requirements.md` NFR1.4, NFR3.6, NFR3.7, NFR5.7, NFR5.8, NFR7.7, NFR9.15–NFR9.18, NFR10.3, NFR10.4; `tech-stack-decisions.md`).
- NFR design: `construction/acceptance-harness/nfr-design/` (`security-design.md`, `logical-components.md`).
- Contracts: `inception/contract-design/contract-summary.md` (C9, C10, E1, E2). Unit: `inception/units-generation/unit-of-work.md` (U2). Requirement: FR57.
- There are no user stories (the User Stories stage was skipped). Each step traces to FR57 and to the rules (BRx.y) and NFR requirements (NFRx.y) it implements.

## Decisions carried from the design reviews

The design reviews left these points for code generation. They are resolved here so you can approve them with the plan.

| Point | Resolution in this plan |
|---|---|
| `--compare` with suites that did not run (FD review R-10) | A cross-platform suite that is `pending` or `not_run` on every compared platform is skipped by `--compare`. If it ran on some platforms and not others, `--compare` fails it. |
| `build` changes (FD review R-11) | The ratchet refuses a `build` change only once a suite is `implemented`. While a suite is pending, its owning unit may change `build` in the same pull request that implements it. |
| Fields not yet ratcheted (FD review R-12) | The ratchet also refuses `cross_platform` true → false, removing a driver, and `uses_seed` true → false for implemented suites. |
| Sanitizer flags (FD review R-13) | A new settings field `sanitizer_rustflags` (text) is required when `build = "sanitizer"`. The harness passes it as `RUSTFLAGS` to the nightly build. T10's owning unit (U7) sets its value. |
| New settings fields from NFR design | `[budgets]` (`pr_minutes = 10`, `nightly_group_minutes = 300`), `nightly_group` on every implemented suite, and `reference_machines`. |
| Dependency isolation check (NFR review R-02, NFR design review R-01) | Walks normal edges from `rail` in `cargo metadata` output; fails if `rail-testkit` is reachable; fails if `rail-testkit` is not found as a dev-dependency of `rail` at all. |
| Grandchild kill and missing results file (NFR design review, optional) | Both get tests: a stand-in that starts a sleeping grandchild, and `--compare` on a missing results file failing with a clear message. |

## Prerequisites (before Step 1)

- [x] **P1. Bolt branch.** Create `bolt/acceptance-harness` from the current `bolt/walking-skeleton` head. The walking skeleton is not merged into `main` yet, and the harness builds on it. When the skeleton's pull request merges, this branch is rebased onto `main` before its own pull request is opened. Nothing is pushed or opened as part of this plan; pushing and pull requests stay your call.
- [x] **P2. Tools.** Confirm `cargo-nextest` and `cargo-deny` are installed (they were for the skeleton). No new tools are needed.

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

| Contract layer | In the harness |
|---|---|
| Data model | The typed settings (`SuiteConfig`, `VolumeSet`, `Volume`, `SeedPolicy`, `InputSource`, budgets, reference machines) and their validation (BR1, BR6.1), plus the results model (`SuiteRun`, `RunReport`) |
| Repository / data access | Reading `suites.toml` from disk, writing results and observation files, reading `cargo metadata` output |
| Business logic | The ratchet (BR2), levels and seeds (BR3, BR4), the process runner with environment scrubbing, run directories and process-group kills (BR5, NFR5.7, NFR5.8), builds (BR3.6), the scheduler with budgets (BR6, BR7, NFR9.18), and the comparer (BR7.7, BR3.5) |
| API / endpoint | The `rail-accept` command line (BR3.3, BR7.4) and the S0 suite, which drives the real `rail` (BR8) |
| Frontend | Not applicable: the harness has no user interface |

Each Red step runs the unit-scoped command from `unit-test-instructions.md` and records the failing output in the step's notes before its Green step starts.

## Plan

### Step 1 — Project structure and production configuration skeleton

- [x] In `crates/rail-testkit/Cargo.toml`, add `toml` and `serde` (with its `derive` feature) as normal dependencies, and `rail-json` as a workspace dependency. Confirm the newest `toml` and `serde` releases on crates.io, not from memory, and pin them through `Cargo.lock`. Reason recorded in `docs/dependencies.md`: "read the reviewed suite settings file `tests/acceptance/suites.toml`" (approved, Functional Design Q9). (NFR7.7)
- [x] Run `cargo deny --locked check`. If a transitive crate fails a licence, ban or source check, stop and report it to you before changing `deny.toml`.
- [x] Create empty modules `harness/{mod,settings,ratchet,build,process,run,report,compare}.rs` and `suites/{mod,s0}.rs` under `crates/rail-testkit/src/`, and `src/bin/rail-accept.rs` with an empty `main`. `#![forbid(unsafe_code)]` and the existing clippy print denials stay on the library. (BR5.1, BR7.6)

### Step 2 — Bootstrap the test runner and the unit-scoped command

- [x] Add empty test files `crates/rail-testkit/tests/harness_settings.rs`, `harness_ratchet.rs`, `harness_process.rs`, `harness_run.rs`, `harness_report.rs`, `harness_compare.rs`, `harness_deps.rs`, `harness_cli.rs` and `harness_s0.rs`.
- [x] Add test fixtures under `crates/rail-testkit/tests/data/harness/`: small valid and invalid settings files, and a stand-in `rail` program (`src/bin/fake-rail.rs`, test-only behaviour selected by its first argument: print, sleep, fail, start a sleeping grandchild, print its environment).
- [x] Confirm the command from `unit-test-instructions.md` runs (zero tests) on this machine: `cargo nextest run --locked -p rail-testkit -E 'binary(/^harness_/)'`.

### Step 3 — Data model — Red

- [x] `harness_settings.rs`: one test per validation rule, using files in `tests/data/harness/`: unknown key, wrong `schema_version` (BR1.1); missing, duplicated or misordered suite (BR1.2); missing or `not_run` full volume (BR1.3); pending suite with pr/nightly/pr_seed, implemented suite without them or with empty inputs (BR1.4); pr > nightly > full ordering (BR1.5); absolute or `..` input path, unknown generator (BR1.6); platform other than macos/linux (BR1.7); T13 or T14 with a non-release build, and `build = "sanitizer"` without `sanitizer_rustflags` (BR3.6); implemented suite without `nightly_group` (NFR9.17); a settings/registry mismatch (BR6.1). A valid file with all 15 suites loads.
- [x] Record the failing output.

### Step 4 — Data model — Green

- [x] Implement the typed settings in `harness/settings.rs` with `serde` (`deny_unknown_fields` everywhere), the `Volume` ordering (`not_run` < `count(n)` < `exhaustive`), and validation returning a typed `SettingsError` with file, line and key. Implement the static registry in `suites/mod.rs` (S0 only).
- [x] Implement the results model (`SuiteRun`, `Outcome`, `FailureRecord`, `RunReport`) in `harness/report.rs` as plain data.

### Step 5 — Data model — Refactor

- [x] Tidy error messages so each names the file, line and key; keep the tests green.

### Step 6 — Repository / data access — Red

- [x] `harness_report.rs`: the results file for the same run is byte-identical when written from two different directories; it contains no absolute paths, timings, hostnames or usernames; runs are ordered S0, T1..T14 (BR7.2, NFR3.7). Observation files are written under `target/acceptance/<level>/<suite>/observations-<platform>.json` (BR7.7).
- [x] `harness_deps.rs`: the dependency walk over `cargo metadata` normal edges finds no path from `rail` to `rail-testkit`, and does find `rail-testkit` as a dev-dependency of `rail`. A synthetic metadata document with a normal edge to `rail-testkit` fails. A test reading `rail-testkit`'s `Cargo.toml` fails if a compiler crate (`rail`, `rail-syntax`, `rail-check`, `rail-lower`, `rail-codegen`, `rail-build`, `rail-runtime`, `rail-rap`, `rail-tools`, `rail-diag`) appears under `[dependencies]` (BR5.1, NFR7.7).
- [x] Record the failing output.

### Step 7 — Repository / data access — Green

- [x] Implement settings loading from disk, the results and observation writers with `rail-json`, and the metadata walk (`cargo metadata --format-version 1 --locked`, read with `rail-json`).

### Step 8 — Repository / data access — Refactor

- [x] Keep writers streaming-friendly for large observation files; tests stay green.

### Step 9 — Business logic — Red

- [x] `harness_ratchet.rs`: one base/new pair per refusal: count lowered, exhaustive → count, count → not_run, volume entry removed, `unit_of_measure` changed, `over` changed, input source removed or replaced, platform removed, `build` changed on an implemented suite, deadline lowered, `cross_platform` true → false, driver removed, `uses_seed` true → false, suite removed, implemented → pending, differing schema versions. Pairs that pass with notices: raised volume, added entry or input, changed `pr_seed`, changed budget or `nightly_group`, `build` change on a pending suite. A missing base file passes with its notice (BR2.1–BR2.5).
- [x] `harness_process.rs`: a `rail` stand-in sees exactly the allow-listed environment even when `RAIL_TEST_SECRET` is set on the test (NFR5.7); suite-added literal variables arrive; each run gets fresh `home/`, `tmp/` and `work/` directories, removed after a pass and kept (path printed) after a failure (NFR5.8); a process past its deadline is killed together with a grandchild it started (BR5.2); after a stop, the run context refuses to start new processes.
- [x] `harness_run.rs`, with fake suites registered only in tests: the level selects volume and seed (BR3.1); a fresh seed is printed before inputs are generated, and `--seed` overrides (BR4.1); a failure records the rerun command and first failing input (BR4.2, BR7.3); a generator called twice with the same seed gives identical inputs, and a nondeterministic one is `error` (BR4.3); pending, `not_run` and other-platform suites get their outcomes (BR3.2, BR3.4, BR6.2); release fails on pending or `not_run` (BR6.3); T13 without a valid `--reference-machine` is `not_run` (BR3.5); a failing suite does not stop the next (BR7.5); a suite that fails once and then passes is reported `fail` and run once (NFR3.6); a budget overrun stops the running suite as `error` "over time budget", marks the rest `not_run` "budget exhausted", still writes results and exits 1 (NFR9.18); exit codes 0/1/2 (BR7.4); the "no suite ran" notice.
- [x] `harness_compare.rs`: identical observation files pass; a one-byte difference fails naming the first differing input; a missing platform file fails; a suite pending or `not_run` on every platform is skipped, and on only some platforms fails; T13 requires every recorded reference machine; a missing results file fails with a clear message (BR7.7, BR3.5).
- [x] Record the failing output.

### Step 10 — Business logic — Green

- [x] `harness/ratchet.rs`: the comparisons above.
- [x] `harness/process.rs` (extending the existing runner in `lib.rs`): `env_clear` plus allow-list, per-run `TempDir` layout, each process in its own process group (`CommandExt::process_group(0)`), group kill with `/bin/kill -KILL -- -<pgid>` started as an argument list, and a stop flag in the run context. The existing `run_process_with_deadline` uses the group kill too, and its tests keep passing.
- [x] `harness/build.rs`: debug, release and sanitizer builds once per run, `--locked`, in the harness's own environment and before the budget clock; `--rail <build>=<path>` overrides (BR3.6, BR5.3).
- [x] `harness/run.rs`: the scheduler, seeds (8 bytes from `/dev/urandom`), a SplitMix64 input generator shared by suites, the budget watchdog (checks once a second), outcomes and exit codes.
- [x] `harness/compare.rs`: the comparer.

### Step 11 — Business logic — Refactor

- [x] Keep functions short and error types typed; tests stay green.

### Step 12 — API / endpoint — Red

- [x] `harness_cli.rs`, running the built `rail-accept`: usage errors exit 2 (unknown level or suite, `--seed` without exactly one `--suite`, missing `--rail` file) (BR3.3); `--list-groups` prints the nightly groups from the settings as a JSON array; `--check-ratchet <base> --against <new>` exit codes; `--compare` exit codes; the terminal summary shows per-suite elapsed time and budget share, and only the program prints (BR7.6, NFR9.18).
- [x] `harness_s0.rs`: S0 run through `rail-accept --level pr --suite S0` against the real `rail` passes on this machine, through the scrubbed environment; a corrupted golden file makes it fail with the step's name (BR8.1).
- [x] Record the failing output.

### Step 13 — API / endpoint — Green

- [x] `src/bin/rail-accept.rs`: argument parsing with the standard library, wiring to the library, terminal summary.
- [x] `suites/s0.rs`: the seven walking-skeleton steps from `scripts/verify-skeleton.sh`, ported to Rust against the shared golden files in `crates/rail/tests/golden/`, using the RAP client and process runner.
- [x] `tests/acceptance/suites.toml`: schema version 1; `[budgets]` with `pr_minutes = 10` and `nightly_group_minutes = 300`; an empty `reference_machines` list; S0 implemented (count 1 at every level, macos and linux, debug build, `nightly_group = "smoke"`); T1–T14 pending with owning unit, drivers, build, `cross_platform`, `unit_of_measure` and full volumes exactly as in the suite table of `functional-spec.md`.
- [x] `scripts/verify-skeleton.sh`: becomes a thin wrapper that runs `rail-accept --level pr --suite S0` and still prints one `PASS n` or `FAIL n` line per step and exits 0 only when S0 passes (BR8.2). The approved verification command stays `scripts/verify-skeleton.sh`.

### Step 14 — API / endpoint — Refactor

- [x] Tidy the summary output; tests stay green.

### Step 15 — Frontend behavior

- [x] Not applicable: the harness has no user interface. (Red, Green and Refactor for this layer are skipped.)

### Step 16 — Environment and build configuration

- [x] `.github/workflows/ci.yml`: in the `test` job, run `rail-accept --level pr` on both platforms and upload `target/acceptance/` with `actions/upload-artifact` pinned to the same commit as in `nightly.yml`, `retention-days: 90`, `if: always()` (NFR10.4). Add a `ratchet` job that checks out `main` into a separate directory, builds its `rail-accept` with `--locked`, and runs `--check-ratchet` against the pull request's `suites.toml`, passing with a notice when `main` has no harness yet (NFR9.15). Add a `compare` job after both platform jobs that downloads their results and runs `--compare` (BR7.7). Add `-p rail-testkit` harness tests to the existing test step (they already run via `-p rail-testkit`). Keep every existing step.
- [x] `.github/workflows/nightly.yml`: add a `groups` job running `rail-accept --list-groups`, and a matrix job over its output running `rail-accept --level nightly --group <name>` on both platforms, with `timeout-minutes: 345`, uploading results as above, then a `compare` job (NFR9.17). The existing fuzz job is unchanged.
- [x] `docs/release-check.md`: how to run the release check on your machines (`rail-accept --level release [--reference-machine <id>]` on each, then `--compare` over all results), and that the results files and `--compare` output are attached to the release's build record (NFR10.4, NFR1.4).

### Step 17 — Documentation and traceability

- [x] **Spec (BR9.1, firm rule).** The authoritative spec is supposed to live in `spec/`, but that folder does not exist yet. Create `spec/rail-language-spec.md` as a byte-for-byte copy of the catalogued knowledge document, then update §12 there: three recorded volumes (pull request, nightly, full), macOS and Linux as the required platforms (Windows later), T13 counted only on declared reference machines, and the S0 smoke suite. Re-sync the knowledge-folder copy from `spec/` with the knowledge tool so the two match.
- [x] **Contract C9 (BR9.2).** Amend C9 in `inception/contract-design/contract-summary.md` as written in the C9 amendment table of `functional-spec.md`, plus the fields added since (`[budgets]`, `nightly_group`, `sanitizer_rustflags`), and close the C9 open question (TOML chosen).
- [x] Update `crates/rail-testkit/src/lib.rs`'s header comment to describe the harness.
- [x] Write `code-summary.md`, `source-manifest.json` and `traceability.json` for this unit.

## Traceability

| Plan steps | Implements |
|---|---|
| 3–5 | BR1.1–BR1.7, BR3.6 (settings side), BR6.1, NFR9.17 (group field) |
| 6–8 | BR5.1, BR7.2, BR7.7 (files), NFR3.7, NFR7.7 |
| 9–11 | BR2.1–BR2.5, BR3.1–BR3.6, BR4.1–BR4.3, BR5.2, BR5.3, BR6.2, BR6.3, BR7.1, BR7.3–BR7.5, BR7.7, NFR1.4, NFR3.6, NFR5.7, NFR5.8, NFR9.18 |
| 12–14 | BR3.3, BR6.4, BR7.6, BR8.1, BR8.2, NFR9.16 |
| 16 | NFR9.15, NFR9.17, NFR10.3, NFR10.4 |
| 17 | BR9.1, BR9.2, FR57 |
