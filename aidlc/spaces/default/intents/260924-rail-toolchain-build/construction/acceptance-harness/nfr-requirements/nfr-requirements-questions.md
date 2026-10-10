# NFR Requirements — acceptance-harness (U2) — Questions

Most of the harness's quality requirements are already settled. They come from the requirements (determinism, macOS and Linux, lockfile-exact builds, secret scanning), from the firm rules (never lower a recorded volume), and from the functional design (the ratchet, deadlines, deterministic results, T13 only on reference machines). The `toml` and `serde` crates were approved as test-only dependencies in Functional Design (Q9).

What is still open is the harness's own time budgets and how it treats the programs and processes it runs. These choices bound the pull-request and nightly volumes that later units pick.

## Q1. How long may a pull-request run of the harness take?

Every pull request runs `rail-accept --level pr` on GitHub-hosted macOS and Linux. Later units choose their suites' pull-request volumes (Functional Design Q4) to fit inside this budget, so it caps how much each suite can check per pull request. The budget covers only the harness run, not the rest of CI.

A. At most 10 minutes per platform for the whole harness run, across all implemented suites (recommended)
B. At most 20 minutes per platform
C. At most 5 minutes per platform
X. Other (please specify)

[Answer]: A

## Q2. How long may the nightly run take?

The nightly run uses the nightly volumes and includes the short fuzz runs your practices call for.

A. At most 2 hours per platform (recommended)
B. At most 6 hours per platform
C. No fixed budget: nightly volumes are chosen per suite and the total is whatever they add up to
X. Other (please specify)

[Answer]: B

## Q3. What happens when a suite exceeds the budget for its level?

Volumes can never be lowered (firm rule), so a suite that grows too slow can't be fixed by shrinking its volume.

A. The harness reports each suite's share of the budget in the terminal summary. A pull-request or nightly run that goes over budget fails with "over time budget", so the owning unit must make the suite faster, not smaller (recommended)
B. Over-budget runs pass but print a warning
X. Other (please specify)

[Answer]: A

## Q4. What environment do the `rail` processes the harness starts get?

Some suites run adversarial programs (T11 resource bombs, T12 capability escape attempts), and your machines may hold secrets such as the model API keys used by the measurement tools. Whatever the harness passes on, `rail` and the programs it runs can read.

A. A scrubbed, allow-listed environment: only `PATH`, `LANG=C.UTF-8`, `TZ=UTC`, and `HOME` and `TMPDIR` pointing into a fresh per-run temporary directory. Everything else, including API keys, is not passed on. A suite may add named variables it needs (recommended)
B. The harness passes its own environment through unchanged
X. Other (please specify)

[Answer]: A

## Q5. May the harness retry a failed suite?

A retry can hide nondeterminism, and determinism is one of the properties the suites exist to prove (NFR3).

A. Never. A failure is a failure at every level; a suite that fails intermittently is a defect, fixed with a reproducing test (recommended)
B. One automatic retry at the pull-request level only, with the first failure still printed
X. Other (please specify)

[Answer]: A

## Q6. How long are results kept?

The results files, and for T1, T3 and T7 the observation files, are evidence: pull-request results for review, and release results for the release record.

A. CI keeps pull-request and nightly results as build artifacts for 30 days. Release-check results are kept with each release's build record, for as long as the release exists (recommended)
B. CI keeps results for 90 days. Release results as in A
C. Results are only printed; nothing is kept
X. Other (please specify)

[Answer]: B

## Follow-up questions

## Q7. How should the 6-hour nightly budget fit GitHub's job limit?

You chose a nightly budget of 6 hours per platform (Q2). A single job on a GitHub-hosted runner is stopped after 6 hours, and that limit also has to cover checking out the code and building `rail` (including the release and sanitizer builds). So one job cannot use the full 6 hours for the harness.

A. Split the nightly run into several parallel jobs per platform, each running a group of suites with a harness budget of 5 hours, leaving room for checkout and builds under GitHub's 6-hour limit. The whole nightly still finishes within about 6 hours of wall-clock time because the groups run in parallel, and the `--compare` step runs after all groups finish (recommended)
B. Keep one job per platform and set the harness budget to 5 hours, leaving an hour for checkout and builds
C. Run the nightly on your own machines instead of GitHub-hosted runners, with the full 6-hour budget
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Pull-request budget:** the whole `rail-accept --level pr` run takes at most 10 minutes per platform, across all implemented suites. Later units fit their pull-request volumes inside it (Q1).
- **Nightly budget:** the nightly run is split into parallel suite-group jobs per platform. Each group has a 5-hour harness budget so it fits under GitHub's 6-hour job limit with checkout and builds, and the whole nightly finishes in about 6 hours of wall-clock time. `--compare` runs after all groups finish (Q2, Q7).
- **Over budget:** the terminal summary shows each suite's share of the budget. A pull-request or nightly run (or nightly group) that goes over its budget fails with "over time budget", so the owning unit must make the suite faster, never smaller (Q3).
- **Environment:** `rail` processes get only `PATH`, `LANG=C.UTF-8`, `TZ=UTC`, and `HOME` and `TMPDIR` in a fresh per-run temporary directory. Nothing else is passed on, including API keys. A suite may add named variables it needs (Q4).
- **Retries:** never, at any level. An intermittent failure is a defect, fixed with a reproducing test (Q5).
- **Retention:** CI keeps pull-request and nightly results and observation files as build artifacts for 90 days. Release-check results are kept with each release's build record for as long as the release exists (Q6).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
