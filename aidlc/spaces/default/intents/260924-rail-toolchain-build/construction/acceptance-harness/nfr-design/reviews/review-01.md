## Review

**Verdict:** NOT-READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-29T23:12:19Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Critical | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Dependency isolation (NFR7.7) | The designed check reads Cargo.lock and fails if rail-testkit is reachable from rail. Cargo.lock (and crates/rail/Cargo.toml) already list rail-testkit as a dev-dependency of rail, rail-build, rail-rap and rail-tools, so the check fails on day one. Cargo.lock does not distinguish dev edges from normal edges. The stated goal (nothing only rail-testkit uses reaches the shipped binary) is also not what the check measures. | Redefine the check so it matches the goal: for example run cargo tree -p rail -e normal --locked (or cargo metadata with dependency kinds) and assert that toml, serde and rail-testkit are absent. Alternatively walk Cargo.lock but exclude dev edges using the manifests. Say in the design which one is used. | New |
| R-02 | Major | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Time budgets; logical-components.md > Scheduler and budget watchdog | The watchdog is said to stop the running suite's rail processes, but no mechanism is designed. The existing testkit deadline only calls child.kill() on the direct child (crates/rail-testkit/src/lib.rs). Programs that rail run or rail build spawn (T11 and T12 hostile programs) survive as orphans, and can outlive the run on a CI runner or an owner machine. Suites run in-process, so how the watchdog thread interrupts a suite mid-judge is also unspecified. | Specify process-group creation and group kill (Unix), how child processes are tracked per suite, and a cancellation signal that the suite runner honours. Add a test with a rail stand-in that spawns a grandchild and check that it is killed on deadline and on budget. | New |
| R-03 | Major | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Boundary 3 and Time budgets (nightly group check) | rail-accept must verify from main's build that every implemented suite appears in exactly one nightly group, but the groups are defined in nightly.yml (YAML) and the only parsing dependencies approved are toml and serde. The source of the group data is not specified. The design also leaves the mechanism to the CI Pipeline stage, so NFR9.17 verification has no defined implementation here. | Decide where the groups are recorded (for example a groups table in suites.toml, which the ratchet already protects) and have the workflow read it, or define a plain-text extraction rule for nightly.yml. State the choice in the design. | New |
| R-04 | Minor | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Boundary 3; Trust boundaries table row Pull request to settings file | The ratchet is protected by building from main, but the pull-request workflow file, the --budget values and the job that invokes the ratchet are all editable in the same pull request. A change that removes the job or raises the budget is not caught by the ratchet. The control is owner review only, which the design does not state. | State that workflow edits and budget values are covered by owner review, or add a check such as a CODEOWNERS entry or a comparison of budget values against main. | New |
| R-05 | Minor | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Retention (NFR10.4) | The artifact upload will not run when a suite fails or the budget is exceeded unless the step is marked to run always, and those are the runs whose results matter most. The existing nightly.yml uploads with a pinned action, but the design does not say the step must run on failure. | Require an always-run condition on the upload step, and that a missing results file is itself an error. | New |
| R-06 | Minor | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/acceptance-harness/nfr-design/security-design.md > Boundary 1 | Setting HOME to a fresh directory on macOS may break xcrun and the linker. The design records this only as an assumption based on the walking skeleton, which may not have used a changed HOME. | Add a test on macOS runners that links a program with the scrubbed environment, and name the fallback (a literal variable) in the design. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| traceability | PASS (checked by hand) | All 12 NFR IDs from security-requirements.md (NFR1.4, 3.6, 3.7, 5.7, 5.8, 7.7, 9.15 to 9.18, 10.3, 10.4) are declared with non-empty targets. The NFR7.7 target is unsound (R-01). |
| required-sections, upstream-coverage | Not run | Not run separately; sections Sources and Assumptions & Open Questions are present in both documents. |
| mermaid check | Plausible | The flowchart LR block has balanced brackets and quoted labels, and a text fallback follows it. |
| linter, type-check | Not applicable | Only Markdown and JSON artifacts are under review. |

### Summary

Coverage of the NFR IDs is complete, but the NFR7.7 dependency check is contradicted by the repository (rail already has a dev-dependency on rail-testkit), so it cannot pass as designed. The budget watchdog and the nightly-group check also lack implementable mechanisms.
