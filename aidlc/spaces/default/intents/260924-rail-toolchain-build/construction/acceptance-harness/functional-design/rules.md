# Business Rules — acceptance-harness (U2)

The rules the acceptance harness enforces. One principle runs through them: **the harness can make a suite stricter, never quietly weaker.** Recorded volumes only rise, a suite that is not built yet can never let a release pass, and a failure can always be reproduced from what the harness printed.

Sources: FR57 (acceptance harness), FR27 (the walking-skeleton check that becomes S0), FR58 and FR58.1 (spec upkeep), the cross-cutting requirements the unit must respect (NFR1 performance evidence, NFR3 determinism, NFR6 portability, NFR7.2 approved dependencies, NFR9 code quality, NFR10 lockfile-exact builds), contracts C9, C10, E1 and E2 (`contract-summary.md`), the rAiL spec §12, the team practices (Testing Posture, the firm rules, Rust conventions), and `functional-design-questions.md` ([Q1]–[Q9]).

```yaml
rules:
  # BR1 — Reading the settings file
  - id: BR1.1
    statement: The settings file is strict TOML with a known schema; anything unexpected is rejected.
    category: validation
    applies_to: SettingsFile
    trigger: every rail-accept run, before any suite starts
    logic: IF the file is missing, is not valid TOML, has schema_version other than 1, or contains a key the schema does not define THEN stop.
    violation: settings error naming the file, line and key; exit code 2; no suite runs.
    source: [FR57, Q1, Q9]
  - id: BR1.2
    statement: The file records every suite exactly once, in a fixed order.
    category: validation
    applies_to: SettingsFile
    trigger: every run
    logic: IF the suites are not exactly S0, T1, T2 ... T14, each once, in that order THEN stop.
    violation: settings error naming the missing, duplicated or misplaced suite; exit code 2.
    source: [FR57, Q6]
  - id: BR1.3
    statement: Every suite records its full volume from spec §12.
    category: validation
    applies_to: VolumeSet
    trigger: every run
    logic: IF a suite has no full volume, or its full volume is not_run THEN stop.
    violation: settings error naming the suite; exit code 2.
    source: [FR57, Q4]
  - id: BR1.4
    statement: A pending suite has only its full volume; an implemented suite has all three volumes and, if it uses a seed, a recorded pull-request seed.
    category: validation
    applies_to: SuiteConfig
    trigger: every run
    logic: IF status is pending and pr, nightly or pr_seed is present THEN stop. IF status is implemented and pr or nightly is absent, or uses_seed is true and pr_seed is absent, or inputs is empty THEN stop.
    violation: settings error naming the suite and field; exit code 2.
    source: [FR57, Q3, Q4, Q5, Q6]
  - id: BR1.5
    statement: Volumes never shrink from level to level.
    category: constraint
    applies_to: VolumeSet
    trigger: every run
    logic: IF an implemented suite has pr > nightly or nightly > full (ordering not_run < count(n) < exhaustive) THEN stop.
    violation: settings error naming the suite; exit code 2.
    source: [FR57, Q3]
  - id: BR1.6
    statement: Input paths stay inside the repository.
    category: validation
    applies_to: InputSource
    trigger: every run
    logic: IF a corpus or fixed path is absolute, contains '..', or does not exist THEN stop. IF a generator name is not registered by the owning suite THEN stop.
    violation: settings error naming the suite and path; exit code 2.
    source: [FR57, C10]
  - id: BR1.7
    statement: Platforms are limited to the two required ones.
    category: validation
    applies_to: SuiteConfig
    trigger: every run
    logic: IF platforms is empty or names anything other than macos or linux THEN stop.
    violation: settings error; exit code 2.
    source: [NFR6]

  # BR2 — The volume ratchet (firm rule: never lower a pass criterion)
  - id: BR2.1
    statement: No recorded volume may be lowered.
    category: policy
    applies_to: VolumeSet
    trigger: rail-accept --check-ratchet <base settings file>, run in CI on every pull request against the file on main
    logic: IF, for any suite and any level, the new volume is lower than the base volume (not_run < count(n) < exhaustive; counts by n) THEN fail.
    violation: ratchet failure naming the suite, level, old and new value; exit code 1; the pull request cannot merge.
    source: [FR57, Q3, practices firm rule A]
  - id: BR2.2
    statement: A suite can never go backwards in status or disappear.
    category: policy
    applies_to: SuiteConfig
    trigger: --check-ratchet
    logic: IF a suite present in the base file is missing in the new file, or its status changes from implemented to pending THEN fail.
    violation: ratchet failure naming the suite; exit code 1.
    source: [FR57, practices firm rule A]
  - id: BR2.3
    statement: Changes that cannot weaken a suite are reported, not refused.
    category: policy
    applies_to: SuiteConfig
    trigger: --check-ratchet
    logic: IF a pr_seed changed, an input source or platform was added, or a volume entry was added THEN print a notice naming the suite and field; the check still passes. A different fixed seed does not reduce how many inputs are checked. nightly_seed and release_seed can only be "fresh" (schema), so they cannot change.
    violation: none; a notice only.
    source: [FR57, NFR6, Q5]
  - id: BR2.4
    statement: What a volume measures, and what a suite reads, can never be narrowed.
    category: policy
    applies_to: VolumeSet, Volume, InputSource, SuiteConfig
    trigger: --check-ratchet
    logic: IF, for a suite present in the base file, unit_of_measure changed, an exhaustive entry's `over` text changed, a volume entry was removed, an input source was removed or replaced, a platform was removed, build changed, or process_deadline_seconds was lowered THEN fail. A wider set is added as a new entry; existing entries stay as they are.
    violation: ratchet failure naming the suite, field, old and new value; exit code 1.
    source: [FR57, NFR6, practices firm rule A]
  - id: BR2.5
    statement: The ratchet has a defined starting point and only compares like with like.
    category: validation
    applies_to: SettingsFile
    trigger: --check-ratchet
    logic: IF the base file does not exist THEN pass with the notice "no base settings; the ratchet starts from this file". IF the two files have different schema_version values THEN fail unless the harness contains a comparison for that exact pair of versions.
    violation: ratchet failure naming both schema versions; exit code 1.
    source: [FR57]

  # BR3 — Levels and selection
  - id: BR3.1
    statement: The level decides the volume and the seed.
    category: calculation
    applies_to: RunRequest
    trigger: run start
    logic: IF level is pr THEN use the pr volume and the recorded pr_seed. IF nightly THEN the nightly volume and a fresh seed. IF release THEN the full volume and a fresh seed.
    violation: none (selection rule).
    source: [FR57, Q3, Q5]
  - id: BR3.2
    statement: A suite runs only on the platforms it lists.
    category: constraint
    applies_to: SuiteRun
    trigger: run start
    logic: IF the host platform is not in the suite's platforms THEN record outcome skipped_platform without running it.
    violation: none; the outcome is recorded.
    source: [NFR6]
  - id: BR3.3
    statement: Command-line arguments are checked before anything runs.
    category: validation
    applies_to: RunRequest
    trigger: argument parsing
    logic: IF the level is missing or unknown, a --suite id is not S0..T14, --seed is given without exactly one --suite, or --rail names a missing file THEN stop.
    violation: usage error; exit code 2.
    source: [Q2, Q5]
  - id: BR3.4
    statement: A suite that cannot run at a level says so.
    category: constraint
    applies_to: SuiteRun
    trigger: run start
    logic: IF the suite's volume for this level is not_run THEN record outcome not_run with the recorded reason, without running it.
    violation: none; not_run is not a failure at pr or nightly. At release it is a failure (BR6.3).
    source: [FR57, practices T13 reference machines]
  - id: BR3.5
    statement: T13 counts only on a declared reference machine, and every reference machine must pass.
    category: policy
    applies_to: SuiteRun, RunReport
    trigger: any run that selects T13
    logic: IF the run was not started with --reference-machine <id> naming one of the settings file's reference_machines THEN record T13 as not_run with "not a reference machine", whatever the level. IF it was THEN record the id in the SuiteRun and the RunReport. The release check for T13 is complete only when results files for every listed reference machine show pass (checked by --compare, BR7.7).
    violation: T13 is not_run, which fails a release (BR6.3). A missing reference machine in --compare fails it.
    source: [NFR1, practices T13 reference machines]
  - id: BR3.6
    statement: Each suite runs against the build of rail it needs.
    category: constraint
    applies_to: SuiteConfig, RunRequest
    trigger: run start
    logic: IF a suite records build = release THEN it runs against rail built with `cargo build --locked --release -p rail` on the pinned stable toolchain. IF build = sanitizer THEN against rail and its runtime built with the pinned nightly toolchain and the sanitizer flags the owning unit records. Otherwise (build = debug) against the debug build. The harness builds each needed build once per run. T13 and T14 always record build = release. A --rail override applies only to suites whose recorded build it names (--rail release=<path>).
    violation: settings error if T13 or T14 records any build other than release; tool error (exit 2) if a needed build fails.
    source: [FR57, NFR1, NFR10, practices pinned toolchains]

  # BR4 — Seeds and reproducibility
  - id: BR4.1
    statement: A fresh seed is printed before the suite starts.
    category: policy
    applies_to: SuiteRun
    trigger: nightly and release runs of a suite that uses a seed
    logic: IF the seed is fresh THEN draw it from the operating system's random source and print "<suite_id> seed <n>" before generating inputs.
    violation: none (reporting rule).
    source: [Q5]
  - id: BR4.2
    statement: Every failure carries the command that reproduces it.
    category: policy
    applies_to: SuiteRun
    trigger: outcome fail or error
    logic: IF a suite fails or errors THEN print and record "rail-accept --level <level> --suite <id> [--seed <n>]" for that exact run.
    violation: none (reporting rule).
    source: [Q5, Q8]
  - id: BR4.3
    statement: The same seed and volume always give the same inputs, on both platforms.
    category: constraint
    applies_to: Suite
    trigger: generate_inputs
    logic: IF generate_inputs is called twice with the same seed and volume THEN it must return byte-identical inputs in the same order; it may not read clocks, the environment, hash-map ordering or the file system beyond its recorded inputs.
    violation: a harness self-test runs every registered generator twice and fails the suite with outcome error on any difference.
    source: [NFR3, NFR6, Q5]

  # BR5 — Black-box driving
  - id: BR5.1
    statement: Suites observe rail only through its command line and its protocol.
    category: constraint
    applies_to: Suite
    trigger: run
    logic: IF a suite needs a result THEN it gets it from `rail ... --json` output, exit codes and stderr (contract E2) or from RAP responses (contract E1). All suite implementations live in rail-testkit (crates/rail-testkit/src/suites/<id>.rs) and register in one static registry there. rail-testkit's normal dependencies may not include the compiler crates rail, rail-syntax, rail-check, rail-lower, rail-codegen, rail-build, rail-runtime, rail-rap, rail-tools or rail-diag. rail-json is allowed, because it is a data-format crate used to read RAP messages and write results, not part of the compiler under test.
    violation: a rail-testkit test reads its own Cargo.toml and fails if a listed compiler crate appears under [dependencies].
    source: [C9, ADR-006, E1, E2]
  - id: BR5.2
    statement: Every process the harness starts has a deadline.
    category: constraint
    applies_to: Observation
    trigger: run
    logic: IF a rail process runs longer than the suite's process_deadline_seconds (recorded in the settings file; default 60, at most 90,000 for suites such as T10 fuzzing) THEN stop it and set timed_out. A suite may also use a tighter deadline for particular inputs.
    violation: the suite's judge sees timed_out; unless the suite expects it, the outcome is error with "timed out".
    source: [FR57, skeleton performance design]
  - id: BR5.3
    statement: The rail under test is the workspace's own build unless one is named.
    category: constraint
    applies_to: RunRequest
    trigger: run start
    logic: IF --rail is absent THEN build each build the selected suites need (BR3.6) with --locked from the workspace. IF a build fails THEN stop.
    violation: tool error "could not build rail (<build>)"; exit code 2; no suite runs.
    source: [NFR10, Q2]

  # BR6 — Registration and pending suites (contract C9)
  - id: BR6.1
    statement: The settings file and the registered implementations agree.
    category: validation
    applies_to: Suite
    trigger: every run
    logic: IF a suite is implemented in the settings but no implementation is registered, or an implementation is registered but the settings say pending THEN stop.
    violation: settings error naming the suite; exit code 2.
    source: [C9, Q6]
  - id: BR6.2
    statement: A pending suite does not block everyday CI.
    category: policy
    applies_to: SuiteRun
    trigger: pr and nightly runs
    logic: IF a suite is pending THEN record outcome pending with "pending (<owning unit>)" and do not count it as a failure.
    violation: none.
    source: [Q6]
  - id: BR6.3
    statement: A release cannot pass while any suite is pending or not run.
    category: policy
    applies_to: RunReport
    trigger: release runs
    logic: IF level is release and any suite's outcome is pending or not_run THEN overall is fail.
    violation: release check fails; exit code 1.
    source: [FR57, Q6]
  - id: BR6.4
    statement: A unit adds a suite in one change, with its implementation, registration and settings together.
    category: policy
    applies_to: SuiteConfig
    trigger: a pull request that changes status from pending to implemented
    logic: IF the status flips to implemented THEN the same change registers the implementation and records the pr and nightly volumes and, if used, pr_seed (enforced by BR1.4 and BR6.1).
    violation: settings error; exit code 2.
    source: [C9, Q4, practices tests-first]

  # BR7 — Outcomes and reporting
  - id: BR7.1
    statement: Each suite ends in exactly one outcome, and an error is kept apart from a failure.
    category: calculation
    applies_to: SuiteRun
    trigger: suite end
    logic: IF the judge says the pass criterion holds THEN pass. IF it does not hold THEN fail. IF the harness or rail could not produce an observation (tool failure, crash of the harness itself, timeout the suite did not expect) THEN error.
    violation: none.
    source: [FR57, practices typed errors]
  - id: BR7.2
    statement: The results file is deterministic.
    category: constraint
    applies_to: RunReport
    trigger: run end
    logic: IF the results file is written THEN it is JSON written by rail-json, runs are ordered S0, T1..T14, and it contains no timestamps, durations, absolute paths, hostnames or usernames; the platform is written as macos or linux.
    violation: a harness test writes the same report twice from different directories and fails on any byte difference.
    source: [NFR3, Q8]
  - id: BR7.3
    statement: The first failing input is recorded.
    category: policy
    applies_to: FailureRecord
    trigger: outcome fail or error
    logic: IF a suite fails or errors THEN record the first failing input in generation order, inline when it is at most 4 KiB, otherwise by path or generator plus index.
    violation: none.
    source: [Q8]
  - id: BR7.4
    statement: The exit code says whether the run passed.
    category: calculation
    applies_to: RunReport
    trigger: run end
    logic: IF every run's outcome is pass, pending (pr, nightly), not_run (pr, nightly) or skipped_platform THEN exit 0. IF any is fail or error, or BR6.3 applies THEN exit 1. Settings, usage and build errors exit 2 (BR1, BR3.3, BR5.3). IF no selected suite actually ran THEN print the notice "no suite ran" (the exit code is unchanged).
    violation: none.
    source: [FR57, Q8]
  - id: BR7.5
    statement: One suite's failure never stops the others.
    category: policy
    applies_to: SuiteRun
    trigger: suite end
    logic: IF a suite fails or errors THEN record it and continue with the next suite.
    violation: none.
    source: [FR57, practices error isolation]
  - id: BR7.6
    statement: Only the rail-accept program prints; the harness library returns values.
    category: constraint
    applies_to: RunReport
    trigger: always
    logic: IF output is needed THEN the program prints the terminal summary (with timings) and seeds; library code returns data and never prints to standard output.
    violation: clippy print_stdout and print_stderr denied in the library (as in the existing rail-testkit).
    source: [practices Rust conventions, Q8]
  - id: BR7.7
    statement: Cross-platform pass criteria are judged by comparing the platforms' recorded observations.
    category: calculation
    applies_to: RunReport
    trigger: rail-accept --compare <results file> <results file> [...]
    logic: For suites whose spec §12 criterion is "identical on every OS" (T1, T3, T7), each platform's run writes its normalized observations (paths made relative, platform-specific fields removed by the suite's normalizer) to target/acceptance/<level>/<suite>/observations-<platform>.json, referenced from its results file. --compare reads the results files of one level, requires one per platform the suite lists, and compares the observation files byte for byte. IF any differ THEN the suite fails with the first differing input. IF a platform's file is missing THEN the suite fails. The per-platform judge still checks the within-platform criteria. In CI, each platform's job keeps its files and one final job runs --compare; at release, the owner copies the files from each machine. T13 uses the same step to require every reference machine (BR3.5).
    violation: compare failure naming the suite, the two platforms and the first differing input; exit code 1.
    source: [NFR3, NFR6, spec §12 T1 T3 T7]

  # BR9 — Contract and spec updates that land with the harness
  - id: BR9.1
    statement: The spec changes the harness depends on land in the same change as the harness.
    category: policy
    applies_to: SettingsFile
    trigger: the pull request that adds the harness
    logic: IF the harness code lands THEN the same change updates spec §12 to state the three recorded volumes (pull request, nightly, full), macOS and Linux as the required platforms, and the reference-machine rule for T13, in the repository's authoritative spec.
    violation: the pull request is not ready for review (firm rule to update the spec in the same change).
    source: [FR58, FR58.1, practices firm rule]
  - id: BR9.2
    statement: The suite registration contract matches what the harness implements.
    category: policy
    applies_to: SuiteConfig
    trigger: the pull request that adds the harness
    logic: IF the harness code lands THEN contract C9 in contract-summary.md is amended by its owner (U2) in the same change. The settings path becomes tests/acceptance/suites.toml; ci_volume is replaced by pr and nightly volumes alongside full, each with unit_of_measure; the fields build, process_deadline_seconds, drivers and status are added; and the ratchet (BR2) and --compare step (BR7.7) are named as the rules later units register under.
    violation: the pull request is not ready for review.
    source: [C9, Q3, contract ownership rule 1]

  # BR8 — The walking-skeleton smoke suite
  - id: BR8.1
    statement: S0 is the walking-skeleton check and runs at every level.
    category: policy
    applies_to: SuiteConfig
    trigger: settings
    logic: S0 is implemented from the start (owning unit U1), uses no seed, has volume count(1) at pr, nightly and full, lists macos and linux, and performs the seven walking-skeleton steps against the shared golden files.
    violation: BR1 settings errors apply.
    source: [Q7, FR27]
  - id: BR8.2
    statement: The approved verification command keeps its meaning.
    category: constraint
    applies_to: Suite
    trigger: any change to scripts/verify-skeleton.sh
    logic: IF scripts/verify-skeleton.sh is changed THEN it runs rail-accept --level pr --suite S0, prints one PASS or FAIL line per step, and exits 0 only when S0 passes.
    violation: the checkpoint verification fails, which halts the next Unit checkpoint.
    source: [Q7]
```

## Rules summary

| Group | Rules | What they protect |
|---|---|---|
| BR1 Reading the settings file | BR1.1–BR1.7 | A strict, complete, well-formed record of every suite. |
| BR2 Volume ratchet | BR2.1–BR2.5 | The firm rule: no recorded volume, unit of measure, set, input, platform, build or suite is ever lowered or narrowed. |
| BR3 Levels and selection | BR3.1–BR3.6 | The right volume, seed and build of `rail` at each level; T13 only on declared reference machines; clear usage errors. |
| BR4 Seeds | BR4.1–BR4.3 | Every failure can be reproduced from printed output. |
| BR5 Black-box driving | BR5.1–BR5.3 | Suites test `rail` as users see it, from one registry in rail-testkit, with recorded deadlines. |
| BR6 Registration and pending | BR6.1–BR6.4 | Suites land tests-first without blocking CI, and never let a release through while pending. |
| BR7 Outcomes and reporting | BR7.1–BR7.7 | One outcome per suite, deterministic results, correct exit codes, cross-platform comparison. |
| BR8 Smoke suite | BR8.1–BR8.2 | The walking-skeleton check lives in the harness without changing the approved command. |
| BR9 Contract and spec updates | BR9.1–BR9.2 | Spec §12 and contract C9 change in the same pull request as the harness. |
