# Entities — acceptance-harness (U2)

The entity model for the acceptance harness. The harness runs the spec's 14 acceptance suites (T1–T14, spec §12) plus the walking-skeleton smoke suite (S0) at three levels: pull request, nightly and release. It drives `rail` only from the outside. It owns the framework and the recorded settings. It owns no suite content: each later unit adds its suites through the registration contract (C9) before writing its own code.

The two entities named in Domain Design (`components.md`: SuiteConfig and SuiteRun, both owned by AcceptanceHarness) keep their identifiers and core attributes. The other entities here are the parts of those two that the design needs to name: the recorded volumes, the settings file, one run request, the observations a suite makes, and the report.

Sources: `unit-of-work.md` (U2), `unit-of-work-story-map.md` (FR57 → U2), `requirements.md` (FR57, NFR1, NFR3, NFR6, NFR7, NFR9, NFR10), `components.md` (AcceptanceHarness, SuiteConfig, SuiteRun), `contract-summary.md` (C9, C10, E1, E2), the rAiL spec §12, the team practices (Testing Posture, firm rules), and `functional-design-questions.md` ([Q1]–[Q9]).

```yaml
entities:
  - name: SettingsFile
    description: The recorded settings for every suite, reviewed in pull requests. A lowered volume in this file breaks a firm rule, so the file itself is the evidence reviewers check. [Q1] [Q9]
    owner_component: AcceptanceHarness
    identifier: path
    attributes:
      - {name: path, type: text, required: true, unique: true, allowed: ["tests/acceptance/suites.toml"]}
      - {name: format, type: enum, required: true, allowed: [toml], constraints: "read with the approved test-only toml + serde crates (Q9)"}
      - {name: schema_version, type: integer, required: true, allowed: [1]}
      - {name: suites, type: list of SuiteConfig, required: true, min: 15, max: 15, constraints: "exactly one entry for each of S0, T1..T14, in that order (BR1.2)"}
      - {name: reference_machines, type: list of ReferenceMachine, required: true, constraints: "the owner's machines whose T13 results count (BR3.5); may be empty until U17 records them"}
    relationships:
      - {to: SuiteConfig, cardinality: "1:15", direction: "file records every suite"}

  - name: SuiteConfig
    description: The recorded settings of one suite (components.md SuiteConfig; contract C9 suite_config_file fields, extended to three volumes by Q3).
    owner_component: AcceptanceHarness
    identifier: suite_id
    attributes:
      - {name: suite_id, type: enum, required: true, unique: true, allowed: [S0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14]}
      - {name: title, type: text, required: true, constraints: "the property name from spec §12 (for S0: 'Walking-skeleton smoke')"}
      - {name: owning_unit, type: text, required: true, constraints: "the unit that implements the suite (U1 for S0; see the suite table in functional-spec.md)"}
      - {name: status, type: enum, required: true, allowed: [pending, implemented], default: pending, constraints: "pending until the owning unit registers the implementation (Q6, BR6.1)"}
      - {name: drivers, type: list of enum, required: true, allowed: [cli, rap], min: 1, constraints: "how the suite observes rail (ADR-006, contracts E1 and E2)"}
      - {name: platforms, type: list of enum, required: true, allowed: [macos, linux], min: 1}
      - {name: build, type: enum, required: true, allowed: [debug, release, sanitizer], default: debug, constraints: "which build of rail the suite runs against (BR3.6); T13 and T14 must be release; once recorded, any change is refused by the ratchet (BR2.4)"}
      - {name: process_deadline_seconds, type: integer, required: true, default: 60, min: 1, max: 90000, constraints: "per rail process (BR5.2)"}
      - {name: cross_platform, type: boolean, required: true, default: false, constraints: "true for T1, T3 and T7: observations are compared across platforms by --compare (BR7.7)"}
      - {name: volumes, type: VolumeSet, required: true}
      - {name: seed_policy, type: SeedPolicy, required: true}
      - {name: inputs, type: list of InputSource, required: true, constraints: "may be empty only while status is pending"}
      - {name: pass_criterion_ref, type: text, required: true, constraints: "points at the suite's row in spec §12; the criterion itself lives in the suite's judge step, not in this file"}
    relationships:
      - {to: VolumeSet, cardinality: "1:1", direction: "records its volumes"}
      - {to: SeedPolicy, cardinality: "1:1", direction: "records its seed policy"}
      - {to: InputSource, cardinality: "1:N", direction: "names its inputs"}
      - {to: SuiteRun, cardinality: "1:N", direction: "each SuiteRun executes one SuiteConfig"}

  - name: VolumeSet
    description: The three recorded volumes of one suite (Q3). Who sets which value follows Q4.
    owner_component: AcceptanceHarness
    identifier: "suite_id"
    attributes:
      - {name: unit_of_measure, type: text, required: true, constraints: "what one unit of volume is, e.g. 'generated modules', 'patches', 'edit tasks', 'CPU-hours', 'programs'"}
      - {name: full, type: list of Volume, required: true, min: 1, constraints: "from spec §12, recorded by U2 now (Q4); T4, T9 and T10 have two entries (a set and a count)"}
      - {name: nightly, type: list of Volume, required: "when status is implemented", constraints: "set by the owning unit in its own design (Q4); same entries as full; each entry pr <= nightly <= full (BR1.5)"}
      - {name: pr, type: list of Volume, required: "when status is implemented", constraints: "set by the owning unit in its own design (Q4)"}
    constraints:
      - "while status is pending, pr and nightly are absent (BR1.4)"
      - "unit_of_measure and each exhaustive entry's `over` never change once recorded; a wider set is a new entry (BR2.4)"

  - name: Volume
    description: One recorded volume value.
    owner_component: AcceptanceHarness
    identifier: "suite_id + level"
    attributes:
      - {name: kind, type: enum, required: true, allowed: [count, exhaustive, not_run]}
      - {name: count, type: integer, required: "when kind is count", min: 1}
      - {name: over, type: text, required: "when kind is exhaustive", constraints: "the finite set covered in full, e.g. 'every §2.7 rule'"}
      - {name: reason, type: text, required: "when kind is not_run", constraints: "why the suite cannot run at this level, e.g. T13 counts only on reference machines"}
    constraints:
      - "ordering for the ratchet: not_run < count(n) < exhaustive; counts compare by n (BR2.1)"

  - name: SeedPolicy
    description: How a suite's generated inputs are seeded (Q5).
    owner_component: AcceptanceHarness
    identifier: "suite_id"
    attributes:
      - {name: uses_seed, type: boolean, required: true, constraints: "false for suites with only fixed inputs (e.g. S0)"}
      - {name: pr_seed, type: integer, required: "when uses_seed and status is implemented", min: 0, max: 18446744073709551615, constraints: "fixed; recorded in the settings file"}
      - {name: nightly_seed, type: enum, required: true, allowed: [fresh], default: fresh}
      - {name: release_seed, type: enum, required: true, allowed: [fresh], default: fresh}

  - name: InputSource
    description: Where a suite's inputs come from.
    owner_component: AcceptanceHarness
    identifier: "suite_id + position"
    attributes:
      - {name: kind, type: enum, required: true, allowed: [generator, corpus, fixed]}
      - {name: generator, type: text, required: "when kind is generator", constraints: "name of a generator registered by the owning unit"}
      - {name: path, type: text, required: "when kind is corpus or fixed", constraints: "repository-relative; no '..', no absolute path (BR1.6); corpus paths follow the C10 layout corpus/<class>/<program>/"}

  - name: ReferenceMachine
    description: One of the owner's reference machines whose T13 results count (practices; spec §11.1).
    owner_component: AcceptanceHarness
    identifier: id
    attributes:
      - {name: id, type: text, required: true, unique: true, constraints: "short lowercase name chosen by the owner; not a hostname"}
      - {name: platform, type: enum, required: true, allowed: [macos, linux]}
      - {name: description, type: text, required: true, constraints: "CPU and memory, as spec §11.1 describes the machine"}
    constraints:
      - "declaring a reference machine is the owner's statement, made with --reference-machine; the harness does not attest the hardware"

  - name: RunRequest
    description: One invocation of the rail-accept program (Q2).
    owner_component: AcceptanceHarness
    identifier: "the command line"
    attributes:
      - {name: level, type: enum, required: true, allowed: [pr, nightly, release]}
      - {name: suite_filter, type: list of suite_id, required: false, constraints: "absent means every suite"}
      - {name: seed_override, type: integer, required: false, constraints: "--seed; only valid together with exactly one --suite (BR3.3)"}
      - {name: rail_binary, type: text, required: true, default: "target/debug/rail, built with --locked from the workspace", constraints: "--rail <path> overrides"}
      - {name: results_path, type: text, required: true, default: "target/acceptance/results-<level>.json"}
      - {name: platform, type: enum, required: true, allowed: [macos, linux], constraints: "detected from the host, never passed in"}
      - {name: reference_machine, type: text, required: false, constraints: "--reference-machine <id>; must name an entry of reference_machines (BR3.5)"}
      - {name: compare_files, type: list of text, required: false, constraints: "--compare <results file>...; compare mode runs no suite (BR7.7)"}

  - name: Suite
    description: The implementation a unit registers for one suite_id (contract C9 suite_interface).
    owner_component: AcceptanceHarness (interface); owning unit (implementation)
    identifier: suite_id
    attributes:
      - {name: generate_inputs, type: operation, required: true, constraints: "(seed, volume) -> ordered inputs; a pure function of its arguments (BR4.3)"}
      - {name: run, type: operation, required: true, constraints: "input -> Observation, through the CLI (--json) or RAP only (BR5.1)"}
      - {name: judge, type: operation, required: true, constraints: "observations -> Verdict, per the suite's spec §12 pass criterion"}
    relationships:
      - {to: SuiteConfig, cardinality: "1:1", direction: "implements the suite the settings record"}

  - name: Observation
    description: What a suite saw from rail for one input.
    owner_component: AcceptanceHarness
    identifier: "run_key + input index"
    attributes:
      - {name: driver, type: enum, required: true, allowed: [cli, rap]}
      - {name: exit_status, type: integer, required: false, constraints: "CLI runs; absent if the process was stopped"}
      - {name: stdout, type: bytes, required: false}
      - {name: stderr, type: bytes, required: false}
      - {name: messages, type: list of JSON value, required: false, constraints: "RAP responses in order"}
      - {name: timed_out, type: boolean, required: true, default: false}

  - name: SuiteRun
    description: One suite executed at one level on one platform (components.md SuiteRun).
    owner_component: AcceptanceHarness
    identifier: run_key
    attributes:
      - {name: run_key, type: text, required: true, unique: true, constraints: "suite_id + level + platform + seed"}
      - {name: suite_id, type: enum, required: true}
      - {name: level, type: enum, required: true, allowed: [pr, nightly, release]}
      - {name: volume, type: Volume, required: true, constraints: "the recorded volume for this level"}
      - {name: seed, type: integer, required: "when the suite uses a seed"}
      - {name: seed_origin, type: enum, required: "when the suite uses a seed", allowed: [recorded, fresh, override]}
      - {name: platform, type: enum, required: true, allowed: [macos, linux]}
      - {name: build, type: enum, required: true, allowed: [debug, release, sanitizer]}
      - {name: reference_machine, type: text, required: "for T13 runs that count (BR3.5)"}
      - {name: observations_path, type: text, required: "when the suite is cross_platform", constraints: "repository-relative path of the normalized observations file (BR7.7)"}
      - {name: outcome, type: enum, required: true, allowed: [pass, fail, error, pending, not_run, skipped_platform]}
      - {name: first_failure, type: FailureRecord, required: "when outcome is fail or error"}
      - {name: rerun_command, type: text, required: "when outcome is fail or error"}
      - {name: elapsed, type: duration, required: false, constraints: "terminal summary only; never written to the results file (BR7.2)"}
    relationships:
      - {to: SuiteConfig, cardinality: "N:1", direction: "each SuiteRun executes one SuiteConfig"}
      - {to: RunReport, cardinality: "N:1", direction: "belongs to one report"}

  - name: FailureRecord
    description: The first failing input of a failed or errored run (Q8).
    owner_component: AcceptanceHarness
    identifier: "run_key"
    attributes:
      - {name: input_index, type: integer, required: true, min: 0, constraints: "position in generation order"}
      - {name: input, type: text, required: true, constraints: "the input itself when it is at most 4 KiB, otherwise its repository path or generator name plus index"}
      - {name: reason, type: text, required: true, constraints: "the judge's message, or the harness error"}

  - name: RunReport
    description: The result of one RunRequest (Q8).
    owner_component: AcceptanceHarness
    identifier: results_path
    attributes:
      - {name: schema_version, type: integer, required: true, allowed: [1]}
      - {name: level, type: enum, required: true, allowed: [pr, nightly, release]}
      - {name: platform, type: enum, required: true, allowed: [macos, linux]}
      - {name: reference_machine, type: text, required: false, constraints: "the declared reference machine, when there is one (BR3.5)"}
      - {name: runs, type: list of SuiteRun, required: true, constraints: "ordered S0, T1..T14; filtered suites only"}
      - {name: overall, type: enum, required: true, allowed: [pass, fail]}
    relationships:
      - {to: SuiteRun, cardinality: "1:N", direction: "contains the runs"}
```

## Summary

| Entity | What it is |
|---|---|
| SettingsFile | `tests/acceptance/suites.toml`, the reviewed record of every suite's settings (TOML, test-only `toml` + `serde`). |
| SuiteConfig | One suite's settings: owning unit, status, drivers, platforms, build, process deadline, cross-platform flag, volumes, seed policy, inputs. |
| VolumeSet / Volume | Pull-request, nightly and full volumes; each a count, exhaustive over a named set, or not run at that level. |
| SeedPolicy | A fixed recorded seed for pull requests; fresh printed seeds for nightly and release. |
| InputSource | A generator, a corpus path (C10 layout) or fixed files. |
| ReferenceMachine | One of your reference machines whose T13 results count. |
| RunRequest | One `rail-accept` invocation: level, suite filter, optional seed override, the `rail` builds, an optional reference machine, or compare mode. |
| Suite | A unit's implementation: `generate_inputs`, `run`, `judge` (contract C9). |
| Observation | What a suite saw from `rail` for one input, through the command line or the protocol. |
| SuiteRun | One suite at one level on one platform, with its outcome. |
| FailureRecord | The first failing input and why it failed. |
| RunReport | The deterministic JSON results file for one invocation. |
