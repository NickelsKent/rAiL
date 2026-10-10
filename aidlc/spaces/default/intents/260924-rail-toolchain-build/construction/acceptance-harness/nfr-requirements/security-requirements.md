# Security Requirements — acceptance-harness (U2)

The acceptance harness is test tooling in the never-shipped `rail-testkit` crate. It holds no user data and serves no one. Its security job is narrower but real. It starts `rail` and, through `rail`, programs that later suites write to be hostile (T11 resource bombs, T12 capability escape attempts). It runs on GitHub-hosted runners and on your own machines, which may hold secrets such as model API keys. And its settings file is the evidence behind a firm rule, so it must not be quietly weakened. This file also carries the harness's other quality requirements (time budgets, retries, retention). A library unit produces no separate performance, reliability or observability document, so they live here with their NFR IDs.

## Sources

- `requirements.md`: FR57 (acceptance harness), NFR1 (performance evidence from reference machines), NFR3 (determinism), NFR5 (capability safety), NFR6 (portability), NFR7 (dependencies), NFR9 (code quality, test-first), NFR10 (supply-chain hygiene), C6 (firm rules).
- `functional-spec.md` and `rules.md` (acceptance-harness Functional Design): workflows WF1–WF6, rules BR1–BR9 (ratchet, levels, seeds, black-box driving, reporting, reference machines, cross-platform comparison).
- `contract-summary.md`: C9 (suite registration), C10 (corpus layout), E1 (RAP), E2 (command line).
- Team practices: Way of Working (CI security checks), Testing Posture (three CI levels, T13 only from reference machines), firm rules.
- `nfr-requirements-questions.md` [Q1]–[Q7], confirmed.

## Scope and data classification

| Data | Classification | Where it lives |
|---|---|---|
| `tests/acceptance/suites.toml` (volumes, seeds, inputs, reference machine ids) | Internal, integrity-critical: it is the evidence for the firm rule on volumes | Repository, reviewed in pull requests |
| Generated inputs, observations, results files | Internal | `target/acceptance/`, CI artifacts |
| Secrets present on the host (for example model API keys used by U17's measurement tools, CI tokens) | Restricted | Host environment only; must never reach `rail` or its programs |
| Adversarial programs (T11, T12) | Internal, hostile by design | Repository (suite inputs), run only under the harness |

No personal data is processed. No regulatory framework applies to the harness itself.

## Trust boundaries

1. **Harness → `rail` process.** The harness trusts nothing `rail` prints. Output is parsed as JSON by `rail-json` (strict, bounded) or compared as bytes. A crash, hang or malformed output becomes an `error` outcome, never a harness crash (BR7.1).
2. **`rail` → programs it runs.** Suite programs may be hostile. The harness's defence is the process environment it gives `rail` and the per-run temporary directory. Real containment of hostile programs is the job of the toolchain under test (U7 capabilities, U15 sandbox), which is exactly what T11 and T12 check.
3. **Pull request → settings file.** A pull request can edit `suites.toml`. The ratchet (BR2) is the control that stops a pull request from lowering what the suites check.
4. **Host → harness.** The host's environment may contain secrets. The harness passes only an allow-list on (NFR5.7).

## Threat model (STRIDE)

| Threat | Example | Control |
|---|---|---|
| Spoofing | A run on a hosted runner claims to be a reference machine to produce T13 evidence | `--reference-machine` must name a recorded id, and the id is written into the results; T13 never counts on hosted CI (NFR1.4). Declaring a machine is your own statement, not a hardware attestation (accepted) |
| Tampering | A pull request lowers a volume, narrows a set, or drops an input so a suite checks less | Ratchet check on every pull request (BR2.1–BR2.5, NFR9.15); owner review of every pull request |
| Tampering | A retry or flaky rerun hides a nondeterministic failure | No retries at any level (NFR3.6) |
| Repudiation | A release is cut without evidence that the suites passed | Release results kept with the release's build record for as long as the release exists (NFR10.4) |
| Information disclosure | A hostile T12 program, or `rail` itself, reads an API key from the environment and prints it into logs or results | Scrubbed environment allow-list (NFR5.7); results contain no environment values (BR7.2) |
| Information disclosure | Results or observation files leak absolute paths, usernames or hostnames | Deterministic results with no absolute paths, hostnames or usernames (BR7.2, NFR3.7) |
| Denial of service | A suite program loops or floods output and stalls CI | Per-process deadlines (BR5.2); level time budgets that fail the run (NFR9.16–NFR9.18) |
| Elevation of privilege | A hostile program writes outside its run directory on a developer or release machine | Every run gets a fresh temporary `HOME` and `TMPDIR` and a fresh working directory, removed afterwards (NFR5.8). Stronger confinement is what T11 and T12 test in the toolchain itself |

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR1.4 | T13 results count only from a run started with `--reference-machine <id>`, where `<id>` names an entry in the settings file's `reference_machines`. Any other T13 run is `not_run` with "not a reference machine". The release check requires a passing T13 result from every recorded reference machine through `--compare` (BR3.5, BR7.7). | Harness tests: T13 without the flag, with an unknown id, and with a valid id; compare with a reference machine missing |
| NFR3.6 | The harness never retries a failed suite, at any level. An intermittent failure is a defect and is fixed with a reproducing test. [Q5] | Harness test: a fake suite that fails once and then passes is reported as `fail`, and the harness runs it only once |
| NFR3.7 | The results file and observation files contain no timestamps, durations, absolute paths, hostnames, usernames or environment values, and the same run writes the same bytes from any directory (BR7.2). | Harness test writing the same report from two directories |
| NFR5.7 | Every `rail` process the harness starts gets only this environment: `PATH` (the harness's own), `LANG=C.UTF-8`, `TZ=UTC`, and `HOME` and `TMPDIR` set to a fresh per-run temporary directory. A suite may add variables only by listing their names and values in its code, and never by copying them from the host. Nothing else is passed on, including API keys and CI tokens. [Q4] | Harness test: a fake suite runs a helper that prints its environment; the output contains exactly the allow-list, even when the test sets an extra variable such as `RAIL_TEST_SECRET` on the harness |
| NFR5.8 | Each suite run gets a fresh temporary directory for its working directory, `HOME` and `TMPDIR`. The harness removes it when the suite ends, unless the suite fails, in which case it is kept and its path printed so the failure can be inspected. Suite inputs are copied in; the repository is never the working directory of a suite program. | Harness tests for removal after pass and retention after fail |
| NFR9.15 | The ratchet check (BR2.1–BR2.5) runs on every pull request against the settings file on `main`, and a failure blocks the merge like any other CI failure. | CI configuration review; ratchet tests |
| NFR9.16 | A pull-request run of the whole harness (`--level pr`) takes at most 10 minutes per platform, across all implemented suites. [Q1] | The harness measures the run and fails with "over time budget" above 10 minutes (NFR9.18) |
| NFR9.17 | The nightly run is split into parallel suite-group jobs per platform. Each group has a harness budget of 5 hours, so it fits under GitHub's 6-hour job limit together with checkout and builds, and the whole nightly finishes in about 6 hours of wall-clock time. The groups are recorded in the nightly workflow, every implemented suite is in exactly one group, and `--compare` runs after all groups finish. [Q2] [Q7] | CI configuration review; a check in the ratchet job that every implemented suite appears in exactly one nightly group |
| NFR9.18 | The terminal summary shows each suite's elapsed time and its share of the level's budget. A pull-request run, or a nightly group, that exceeds its budget fails with "over time budget" and names the slowest suites. The fix is to make a suite faster; volumes are never lowered to meet a budget (firm rule). The release check has no time budget. [Q3] | Harness test with a fake slow suite and a small test budget |
| NFR10.3 | No secret is needed to build or run the harness at the pull-request and nightly levels, and none is stored in the repository. Reference machine ids are plain names, not hostnames or credentials. | `gitleaks` in CI |
| NFR10.4 | CI keeps pull-request and nightly results files and observation files as build artifacts for 90 days. Release-check results files and the `--compare` output are stored with each release's build record and kept for as long as the release exists. [Q6] | CI configuration review; release checklist |
| NFR7.7 | The `toml` crate, with `serde` and `serde`'s derive support for reading typed settings, is a test-only dependency of `rail-testkit`, approved with the reason "read the reviewed suite settings file `tests/acceptance/suites.toml`". It never enters the `rail` binary or any program `rail` links. Its full dependency tree passes the `cargo-deny` licence, advisory, ban and source checks. (Functional Design Q9) | `cargo tree -p rail` shows no `toml` or `serde`; `cargo deny check` in CI |

## Licences

The new crates and everything they pull in must carry licences the `cargo-deny` configuration already allows for the project's MIT OR Apache-2.0 choice. If a transitive crate needs a licence outside that list, stop and ask for approval before changing `deny.toml`.

## Out of scope for the harness

- Containing hostile programs beyond the per-run directory and scrubbed environment. That is what the toolchain's own capability checks and sandbox provide, and what T11 and T12 verify.
- Attesting reference-machine hardware. Declaring a reference machine is your own statement.
- Live-service scanning (DAST). There is no running service (team practice).

## Assumptions & Open Questions

- [assumption] The GitHub-hosted job limit is 6 hours, as stated when the nightly budget was chosen. If it changes, the 5-hour group budget is revisited, never the volumes.
- [assumption] Keeping a failed suite's temporary directory on a CI runner is harmless, because runners are discarded after the job. On your own machines the printed path lets you delete it.
- Open question for the CI Pipeline stage: how the nightly suite groups are named and balanced, and where release results are attached to the build record.
