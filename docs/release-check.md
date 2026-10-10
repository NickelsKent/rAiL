# Release check

A release (a version tag on `main`) is cut only after the full-volume release
check passes on the owner's own machines and the owner approves it by hand
(team practice, Deployment). The release check is the acceptance harness at
the `release` level: every suite at its full volume from spec §12, with fresh
seeds and no time budget.

## Run it

On each of your machines (macOS and Linux), from a clean checkout of the
commit to be tagged:

```bash
cargo run --locked --release -p rail-testkit --bin rail-accept -- --level release
```

On a reference machine, declare it so that T13 (performance targets) counts
(BR3.5, NFR1.4). The id must be one of `reference_machines` in
`tests/acceptance/suites.toml`; it is a plain name, never a hostname:

```bash
cargo run --locked --release -p rail-testkit --bin rail-accept -- --level release --reference-machine <id>
```

Each run builds the `rail` builds the suites need (`debug`, `release`, and the
sanitizer build on the pinned nightly) with `--locked`, prints each fresh seed
before the suite starts, and writes `target/acceptance/results-release.json`
plus observation files under `target/acceptance/release/`. A failure prints
the exact command that reproduces it (`rail-accept --level release --suite
<id> --seed <n>`); keep the printed run directory if you need to inspect it.

The release level fails while any suite is `pending` or `not_run` (BR6.3), so
a release cannot pass until every suite is implemented. Results from hosted CI
runners are never T13 evidence.

## Compare across machines

Copy each machine's `target/acceptance/` directory to one place, keeping one
directory per machine, then run:

```bash
rail-accept --compare <machine-a>/results-release.json <machine-b>/results-release.json [...]
```

`--compare` fails when a cross-platform suite (T1, T3, T7) observed anything
different on macOS and Linux, when a platform's results are missing, or when a
recorded reference machine has no passing T13 result (BR7.7, BR3.5). Save its
output.

## Keep the evidence

Attach every machine's results file and the `--compare` output to the
release's build record, next to its checksums, build record and dependency
list. Keep them for as long as the release exists (NFR10.4, NFR1.4). Results
files contain no timestamps, absolute paths, hostnames or usernames (BR7.2), so
they can be attached as they are.

Never lower a recorded volume, a pass criterion or a coverage floor to make a
release check pass (firm rule). Fix the defect, with a test that reproduces
it, and run the check again.
