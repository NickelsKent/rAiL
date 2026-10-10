# Logical Components — acceptance-harness (U2)

The harness's parts inside `crates/rail-testkit`, where each quality pattern applies, and how a failure in one part is contained. Everything here is test tooling that is never shipped.

## Sources

- `functional-spec.md` (WF1–WF6), `rules.md` (BR1–BR9) and `entities.md` from Functional Design.
- `security-requirements.md` and `tech-stack-decisions.md` from NFR Requirements.
- `security-design.md` from this stage.
- `contract-summary.md`: C9 (suite registration), E1 (RAP), E2 (command line).

## Components and where NFR patterns apply

| Component | Location in `rail-testkit` | Responsibility | NFR patterns applied |
|---|---|---|---|
| Settings reader | `harness/settings.rs` | Reads and validates `tests/acceptance/suites.toml` (BR1), including `[budgets]` and each suite's `nightly_group` | Strict typed TOML with unknown keys refused; `toml` + `serde` isolated to this crate (NFR7.7); budgets and groups recorded, not passed by workflows (NFR9.16, NFR9.17) |
| Ratchet | `harness/ratchet.rs` | Compares a base and a new settings file (BR2) | Always run from `main`'s build (NFR9.15) |
| Registry | `suites/mod.rs` | The one static list of suite implementations (BR5.1, BR6.1) | No compiler-crate dependencies |
| Suites | `suites/s0.rs`, later `suites/t1.rs` … | `generate_inputs`, `run`, `judge` per suite (C9) | Deterministic generators (BR4.3); black-box driving only (BR5.1) |
| Process runner | `lib.rs` (existing, extended) | Starts `rail` in its own process group with deadlines, captures output, drives RAP sessions, and refuses new processes once a run is stopped | Scrubbed environment (NFR5.7); run directories (NFR5.8); per-suite deadlines and process-group kills (BR5.2, NFR9.18) |
| Build step | `harness/build.rs` | Builds the debug, release and sanitizer `rail` once per run (BR3.6) | Untimed, full environment, `--locked` [NFR Design Q2] |
| Scheduler and budget watchdog | `harness/run.rs` | Runs selected suites in order, one attempt each, and enforces the budget | No retries (NFR3.6); stop at budget (NFR9.18) |
| Reporter | `harness/report.rs` | Writes the results file and observation files; prints the terminal summary from `rail-accept` only | Deterministic `rail-json` output (NFR3.7); no printing from library code (BR7.6) |
| Comparer | `harness/compare.rs` | `--compare` across platforms and reference machines (BR7.7, BR3.5) | Byte comparison of normalized observations (NFR1.4) |
| `rail-accept` program | `bin/rail-accept.rs` | Argument parsing, exit codes, terminal output | Usage errors exit 2 (BR3.3) |

## Failure domains and blast radius

| Failure | Contained to | How |
|---|---|---|
| A `rail` process crashes, hangs or floods output | The one suite run | Deadline kills its whole process group, including grandchildren; outcome `error`; other suites continue (BR5.2, BR7.5) |
| A hostile suite program writes files | Its run directory | Fresh `work/`, `home/`, `tmp/` per run (NFR5.8) |
| A suite exceeds the time budget | That run | Watchdog stops it; remaining suites `not_run`; results still written (NFR9.18) |
| The settings file is invalid | The whole invocation, before anything runs | Settings error, exit 2 (BR1) |
| A `rail` build fails | The whole invocation, before anything runs | Tool error, exit 2 (BR5.3) |
| A pull request weakens the ratchet code | Nothing, for that pull request | The ratchet runs from `main` (NFR9.15) |
| A generator is nondeterministic | That suite | Double-generation self-check gives `error` (BR4.3) |

## Shared resources

| Resource | Shared by | Rule |
|---|---|---|
| `tests/acceptance/suites.toml` | Every unit that adds a suite | Changes only upward, checked by the ratchet; one entry per suite (BR1.2, BR2) |
| `crates/rail-testkit` | U1 (existing tools), U2, every suite-adding unit | Suites live in `suites/<id>.rs`; no compiler crate as a normal dependency (BR5.1) |
| `target/acceptance/` | All harness runs on one machine | Files are named by level and suite; each run overwrites its own level's files |
| The `rail` builds | All suites in one run | Built once per run; suites never rebuild `rail` themselves |

## Handoff to later units

- A suite-adding unit writes `suites/<id>.rs`, adds it to the registry, and flips its settings entry to `implemented` with its pull-request and nightly volumes in the same pull request (BR6.4, WF3).
- It fits its pull-request volume into the 10-minute budget alongside the suites already implemented, and its nightly volume into its nightly group's 5-hour budget (NFR9.16, NFR9.17).
- It gets only the scrubbed environment and adds literal variables it needs. It never receives host secrets (NFR5.7).
