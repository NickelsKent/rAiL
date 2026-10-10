# Code Summary — acceptance-harness (U2)

The acceptance harness is built in the never-shipped `rail-testkit` crate: a library (`harness`, `suites`) and the `rail-accept` program. It reads `tests/acceptance/suites.toml` (all 15 suites: S0 implemented, T1–T14 pending for their owning units), runs suites at the `pr`, `nightly` or `release` level, drives `rail` only through its command line and RAP, checks the volume ratchet, compares platforms, and writes deterministic results files. `scripts/verify-skeleton.sh` is now a thin wrapper around `rail-accept --level pr --suite S0`. The work was done tests-first (TDD): each layer went Red, Green, Refactor. The Red output is recorded below.

## Files created and modified

Created:

- `crates/rail-testkit/src/harness/mod.rs`, `settings.rs` (typed settings and validation), `ratchet.rs`, `build.rs` (`rail` builds and the `cargo metadata` walk), `process.rs` (run directories, scrubbed environment, process-group kills, stop signal), `run.rs` (scheduler, seeds, SplitMix64, budget watchdog), `report.rs` (results model, results and observation writers), `compare.rs`.
- `crates/rail-testkit/src/suites/mod.rs` (the `Suite` interface, `Input`, `Observation`, `Registry`) and `suites/s0.rs` (the seven walking-skeleton steps).
- `crates/rail-testkit/src/bin/rail-accept.rs` (the program) and `src/bin/fake-rail.rs` (the test-only stand-in for `rail`).
- Tests: `crates/rail-testkit/tests/harness_{settings,report,deps,ratchet,process,run,compare,cli,s0}.rs`, shared helpers in `tests/common/mod.rs`, and fixtures in `tests/data/harness/` (`valid.toml`, `unknown-key.toml`, `bad-schema.toml`, `not-toml.toml`).
- `tests/acceptance/suites.toml`, `docs/release-check.md`, and `spec/rail-language-spec.md` (the new authoritative spec copy, with §12 updated).

Modified:

- `crates/rail-testkit/Cargo.toml`: adds `rail-json` (workspace), `toml` 1.1.7 (features `std`, `serde`, `parse`) and `serde` 1.0.229 (`derive`).
- `Cargo.lock`, `deny.toml` (12 crates added to `[bans].allow`, `Unicode-3.0` added to `[licenses].allow`) and `docs/dependencies.md`.
- `crates/rail-testkit/src/lib.rs`: a new header comment, the `harness` and `suites` modules, and `run_process_with_deadline` now running the process in its own process group and killing the whole group at the deadline.
- `scripts/verify-skeleton.sh`: now a wrapper around `rail-accept`. It keeps its name, prints one PASS or FAIL line per step, and exits 0 only when S0 passes.
- `.github/workflows/ci.yml`: the `test` job adds a pull-request-level harness step and uploads `target/acceptance/` (90 days, `if: always()`). New `compare` and `ratchet` jobs.
- `.github/workflows/nightly.yml`: new `groups`, `acceptance` (one job per group × platform, `timeout-minutes: 345`) and `compare` jobs. The fuzz job is unchanged.
- Outside the repository code (record and knowledge paths): `inception/contract-design/contract-summary.md` (C9 amended to version 0.2, its open question closed) and `aidlc/spaces/default/knowledge/documents/rail-language-spec.md` (copied back from `spec/`).

## Key decisions

- **Owner decision, 2026-10-08 (dependencies).** The first `cargo deny --locked check` after adding `toml` and `serde` failed: 10 transitive crates were missing from the bans list, and `unicode-ident`'s licence includes `Unicode-3.0`. Work stopped and was reported. The owner chose option A:
  - approve all 12 crates: `toml`, `serde`, `serde_core`, `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `serde_spanned`, `toml_datetime`, `toml_parser`, `winnow`;
  - add `Unicode-3.0` to `[licenses].allow`.

  Each crate is recorded in `docs/dependencies.md` with the reason "test-only, pulled in by the approved toml/serde (derive) used by rail-testkit to read tests/acceptance/suites.toml; never reaches the rail binary". `Unicode-3.0` is recorded as the permissive licence of `unicode-ident`'s Unicode data tables, used at build time only through `serde_derive`. `toml` is built without its `display` feature, so `toml_writer` is in `Cargo.lock` but is not compiled.
- **Settings shape.** Each suite is a `[[suites]]` table:
  - `seed_policy` is an inline table, and `volumes` has `unit_of_measure`, `pr`, `nightly` and `full`;
  - volume entries are `{ kind = "count" | "exhaustive" | "not_run", ... }`, and inputs are `{ kind = "generator" | "corpus" | "fixed", ... }`;
  - `deny_unknown_fields` is set on every struct and tagged enum;
  - errors read `<file>:<line>: suites[<id>].<key>: <message>`; the line comes from the toml error span or from the suite's `id` key.
- **`sanitizer_rustflags`** is required once an implemented suite records `build = "sanitizer"`. Pending T10 records none yet; U7 sets it when it implements T10. This follows the plan's note that "T10's owning unit (U7) sets its value".
- **Recorded `pr_seed` range.** TOML integers are signed 64-bit, so a recorded `pr_seed` is at most `i64::MAX`. Fresh seeds use the full `u64` range, are written exactly to results, and are accepted by `--seed`.
- **S0 inputs.** S0 has seven inputs, one per step; volume `count n` runs the seven steps n times. Each step records its mismatches as findings, and the judge fails a step that has findings. Step 7 passes only when steps 1–4 of the same round passed. Each step uses fresh workspaces under its run's `work/`. RAP sessions send all frames on standard input and read framed responses, as the old script did.
- **Results and observation paths.** Observation paths are relative to the results file's directory (`pr/T1/observations-macos.json`) rather than to the repository. That way `--compare` works on downloaded CI artifacts and on files copied from release machines. Nightly group runs write `results-nightly-<group>.json`.
- **Extra command-line options.** `--settings`, `--root` and `--out` were added so tests can use fixture settings and temporary output. The defaults are the workspace the program was built from, `tests/acceptance/suites.toml` and `target/acceptance/`.
- **Budgets in tests.** The budget comes from the settings file (whole minutes). Tests pass a seconds-scale override through the library's `RunOptions::budget` instead of a test settings file, because the file only holds whole minutes. Workflows pass no budget.
- **Watchdog interval.** The budget watchdog checks the clock every 250 ms. That meets "at least once a second" and keeps the 1-second budget tests short.
- **Downloading artifacts in the compare jobs.** No workflow pins `actions/download-artifact`, and a commit SHA must not be guessed from memory. The compare jobs therefore use the runner's preinstalled `gh run download` with the run's automatic, read-only `GITHUB_TOKEN` (`actions: read`). See the concerns below.
- **Process-group kill.** `/bin/kill -KILL -- -<pgid>` is started as an argument list. After a process exits, its group is killed as well, so descendants cannot hold its output pipes open.
- **Ratchet treatment of the extra fields.** Changes to `reference_machines`, `sanitizer_rustflags`, budgets and nightly groups are reported as notices, not refusals. `cross_platform` true → false, a removed driver, and `uses_seed` true → false on an implemented suite are refused, as the plan decided.
- **Forbid check for `build.rs`.** `scripts/check-forbid-unsafe.sh` treats every file named `build.rs` as a crate root. `harness/build.rs` therefore repeats `#![forbid(unsafe_code)]` at module level; the crate root already forbids unsafe code. The script was not changed.

## Tests and command results

Red evidence (the plan file is left as approved, apart from checkbox ticks; see Deviations):

- Step 2: with empty test files, the unit-scoped command builds and finds 0 tests. nextest 0.9.146 then reports `error: no tests to run`, because its default is `--no-tests=fail`.
- Step 3 (data model), unit-scoped command:
  ```
  error[E0432]: unresolved imports `rail_testkit::harness::settings::Build`, `...::Level`, `...::Platform`, `...::Settings`, `...::SettingsError`, `...::Status`, `...::SuiteId`, `...::Volume`
  error[E0432]: unresolved import `rail_testkit::suites::Registry`
  error[E0432]: unresolved imports `rail_testkit::suites::Input`, `rail_testkit::suites::Observation`, `rail_testkit::suites::Suite`
  error: could not compile `rail-testkit` (test "harness_settings") due to 5 previous errors
  ```
- Step 6 (data access):
  ```
  error[E0432]: unresolved imports `rail_testkit::harness::build::cargo_metadata`, `...::has_dev_dependency`, `...::normal_dependency_path`, `...::parse_metadata`
  error[E0432]: unresolved imports `rail_testkit::harness::report::ObservationWriter`, `...::observations_rel_path`, `...::results_file_name`, `...::results_json`, `...::write_results`
  error[E0599]: no associated function or constant named `load` found for struct `Settings` in the current scope
  ```
- Step 9 (business logic):
  ```
  error[E0432]: unresolved imports `rail_testkit::harness::ratchet::RatchetReport`, `...::check_ratchet`, `...::compare_settings`
  error[E0432]: unresolved imports `rail_testkit::harness::process::RunDir`, `rail_testkit::harness::process::StopSignal`
  error[E0599]: no associated function or constant named `new` found for struct `RunContext` in the current scope
  error[E0432]: unresolved imports `rail_testkit::harness::run::Events`, `...::RunOptions`, `...::SplitMix64`, `...::exit_code`, `...::fresh_seed`, `...::run_suites`
  error[E0432]: unresolved import `rail_testkit::harness::compare::compare`
  ```
- Step 12 (command line and S0), `--no-fail-fast` against an empty `main` and the S0 stub: 73 tests run, 64 passed, 9 failed. The failures were all seven `harness_cli` tests and both `harness_s0` tests.

Green and Refactor: settings 14/14 (Step 4); 25/25 after Step 7; 64 pass and 1 skipped after Step 10; 73 pass and 1 skipped after Step 13.

One test-helper defect was found during Step 10 and fixed before the step went green. The fake suite set its own 30 s deadline, which overrode the suite's recorded 1 s deadline. The harness itself was correct.

Final runs (macOS AArch64, stable 1.98.1):

| Command | Result |
|---|---|
| `cargo nextest run --locked -p rail-testkit -E 'binary(/^harness_/)'` | 73 passed, 1 skipped |
| `cargo nextest run --locked -p rail-testkit` | 83 passed, 1 skipped |
| `cargo nextest run --locked -p rail -p rail-json -p rail-diag -p rail-syntax -p rail-check -p rail-lower -p rail-codegen -p rail-build -p rail-tools -p rail-rap` | 111 passed |
| `cargo fmt --all --check` (and `rustfmt --edition 2024 --check crates/rail-runtime/src/lib.rs`) | clean |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | clean (exit 0) |
| `scripts/check-forbid-unsafe.sh` | `forbid(unsafe_code): ok` |
| `cargo deny --locked check` | `advisories ok, bans ok, licenses ok, sources ok` (the existing `hashbrown` duplicate warning only) |
| `cargo tree --locked -p rail -e normal --target all -i toml` | nothing to print |
| `scripts/verify-skeleton.sh` | PASS 1–7, `walking skeleton: all steps passed`, exit 0 |

The skipped test is `helper_print_runner_environment`. It is ignored on purpose: its parent test starts it with `RAIL_TEST_SECRET` set, so no test needs `unsafe` `set_var`.

## Deviations

- **No notes inside the plan file.** Red notes added to `code-generation-plan.md` changed its Plan Approval fingerprint. The plan-approval guard then blocked every workspace write. The notes were removed, which restored the approved text; checkbox ticks are tolerated. The Red evidence is in this file instead.
- **Settings fixtures.** Apart from three separate fixture files, the invalid settings variants are derived inside the tests from `valid.toml` by small text edits. They are not one committed file per rule.
- **Knowledge sync.** `aidlc knowledge sync` fails with `error: unknown command 'knowledge'` in this install; the command is `aidlc engine knowledge sync`. The edited spec was copied back over the knowledge copy byte for byte, and `aidlc engine knowledge sync` then refreshed the DocumentKB catalog entry (`changed documents/rail-language-spec.md`). The Step 17 spec checkbox is ticked.
- The S0 test that corrupts a golden file builds `rail` through the library's `build_rail`. It then runs `rail-accept --root <copy> --rail debug=<path>` against a temporary copy of the fixtures, the golden files and the settings file.

## Unverified items

- **Linux.** Nothing ran on Linux here, so these are unverified there:
  - the process-group kill via procps `/bin/kill -KILL -- -<pgid>`;
  - the environment allow-list test;
  - S0 linking through the scrubbed environment (only `PATH`, `LANG`, `TZ`, `HOME`, `TMPDIR`);
  - the expected build target `x86_64-linux`.
- **GitHub Actions.** The new jobs (`compare`, `ratchet`, nightly `groups`/`acceptance`/`compare`) have only been parsed as YAML, never run. In particular:
  - `gh run download` with `--pattern` and the `fromJSON` matrix are untested;
  - the ratchet job's "no base harness" path is the one the first pull request will take.
- **Sanitizer build.** It has never been built. `build_rail` uses `cargo +nightly-2026-09-27` with `RUSTFLAGS` and `target/sanitizer`. U7 may need `-Zbuild-std` or a `--target` flag when it implements T10.

## Follow-ups

- **Compare-job token.** The compare jobs use the automatic `GITHUB_TOKEN` (read-only, `actions: read`) to download artifacts. If the owner prefers no token in harness jobs (NFR10.3 says the harness itself needs no secret, and it does not), pin `actions/download-artifact` to a verified commit SHA instead.
- When the skeleton's pull request merges, rebase `bolt/acceptance-harness` onto `main` (P1). The first pull request's ratchet job passes with "no base harness; the ratchet starts after this merge".
- Owning units (U3, U5, U7, U8, U10, U13, U15, U17) register their suites through the amended C9. For each: add `suites/<id>.rs`, register it in `Registry::production`, and set `status`, the pr/nightly volumes, `pr_seed`, `inputs` and `nightly_group` in the same change.
- Optional: `scripts/check-forbid-unsafe.sh` could match only `build.rs` files that sit next to a `Cargo.toml`. That would remove the need for the module-level attribute in `harness/build.rs`.
