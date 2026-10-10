## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-10T05:27:00Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/benchmark-corpus/nfr-design/security-design.md > Boundary 1, Linux bullet and Assumptions & Open Questions | Linux bwrap binds the whole root read-only, so an AI-written program can read any file the user can read (for example ~/.ssh on a developer Linux box). The macOS profile restricts reads to binary, system libraries and run dir. The two platforms enforce different confinement, and Q1 raised "reads elsewhere" as a gap. Separately, GitHub-hosted ubuntu-latest (24.04) restricts unprivileged user namespaces by default via AppArmor, so bwrap likely fails there. The design records this only as an assumption, with fail-closed behaviour and no planned CI mitigation, so the required Linux corpus check may not run at all. | Specify a Linux read policy matching macOS (bind only /usr, /lib*, /etc minimal, the program binary and the run dir, not the whole root). Name the CI step that makes bwrap usable on the runner (for example the sysctl or AppArmor change), or a fallback, instead of leaving it as an assumption. | New |
| R-02 | Minor | security-design.md > Boundary 1 vs security-requirements.md NFR5.9 | NFR5.9 requires current directory, HOME and TMPDIR inside the run directory and relative paths only. The design does not say the sandbox wrapper preserves this. bwrap gives a private /tmp, which could conflict with TMPDIR in the run dir. The permission-denied "skipped as root" case (BR6.5) is not reconciled with bwrap user-namespace uid mapping, where the program may appear as root. | State that the wrapper sets the environment per NFR5.9, and say how root detection works under bwrap. | New |
| R-03 | Minor | security-design.md > Boundary 2 Offline, and Boundary 1 | The sandbox covers running programs only. Building the 120 programs (rayon dependency build scripts, proc macros, dune and opam actions) runs unsandboxed on the host. Offline mode and vendoring limit exposure but do not confine it. | State that builds are unsandboxed and why that is acceptable (vendored, reviewed sources, build scripts approved per crate), or sandbox the build with network denied. | New |
| R-04 | Minor | security-design.md > Time limits | The design promises that on a budget stop the checker stops the running program, but gives no per-program deadline value and no process-group kill behaviour. The threat model names per-run deadlines. sandbox-exec has no die-with-parent equivalent, so orphaned children are possible on macOS. | Record a per-program deadline and kill the whole process group on timeout. | New |
| R-05 | Minor | security-design.md > Boundary 1, Linux bullet | CI installs bubblewrap from the runner package manager, which is neither version-pinned nor covered by the NFR10.5 pin rule. | Note the unpinned apt package as an accepted risk, or pin the version. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| required-sections / upstream-coverage / traceability | Manual check: Sources, Assumptions & Open Questions present; all 9 NFR IDs (NFR3.8, 3.9, 5.9, 6.3, 7.8, 9.19, 9.20, 10.5, 10.6) appear in traceability.json with non-empty targets and match security-requirements.md | PASS |
| mermaid | Single flowchart LR block with quoted labels, syntactically plausible, text fallback present | PASS |
| linter / type-check | Not applicable (markdown and JSON only); traceability.json parses | PASS |
| Existing CI pins | ci.yml uses exact-commit pins with version comments, consistent with the design's pin rule | Consistent |

### Summary

The design covers every NFR with a concrete target and matches the owner's two answers. The main weakness is that the Linux sandbox is weaker than the macOS one on reads, and its viability on hosted runners is left as an assumption (R-01, Major, within the two-Major allowance). The remaining items are minor refinements for Code Generation.
