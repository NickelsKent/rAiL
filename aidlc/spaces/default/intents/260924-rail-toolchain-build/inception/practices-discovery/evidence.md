# Evidence — Practices Discovery

This file records what each participant looked at, what the owner decided in
the interview, how each objection from the support reviews was resolved, and
what is still open. The owner's answers are authoritative. Support suggestions
appear in `team-practices.md` and `discovered-rules.md` only where an answer
confirmed them.

## Project Type

Greenfield. The repository has no code yet. It holds `README.md`,
`.gitattributes`, `.gitignore`, `.mcp.json`, the `.claude/` framework shell and
the `aidlc/` workspace. There is no build system, CI configuration, test
configuration, linter or formatter configuration, and no `LICENSE` file. No
reverse engineering was run, so there are no brownfield inputs.

## What Each Participant Inspected

### Lead (pipeline and deploy)

| Source | What it gave |
|--------|--------------|
| `aidlc/spaces/default/memory/org.md` | The framework defaults for all five sections |
| `aidlc/spaces/default/memory/team.md` | All sections empty: no earlier baseline, not a re-run |
| `aidlc/spaces/default/memory/project.md` | Only the Ideation `## Corrections` entries; nothing about practices |
| `aidlc-state.md` | Greenfield; scope `rail-toolchain`; Test Strategy Standard; Construction checkpoints enabled; unit-major, serial construction; Operation phase skipped |
| `.claude/scopes/aidlc-rail-toolchain.md` | No `skeleton:` field and no coverage floor |
| `ideation/intent-capture/intent-statement.md` | One builder working with AI agents; the spec is updated when decisions change it [Q9]; no deployment or operations |
| `ideation/scope-definition/scope-document.md` | macOS and Linux required, Windows later; scaled CI volumes with full volumes as a release check [Q5]; phase order [Q7]; first usable slice and its exit (T1, T2, T4, T5, T6 at CI volume) [Q1]; changed Phase 1 and Phase 3 exits [Q8] [Q10]; local-only registry |
| `ideation/approval-handoff/initiative-brief.md` | Accepted risks; first usable slice = spec Phases 0–1 |
| `aidlc/spaces/default/knowledge/documents/rail-language-spec.md` (§5.5, §7.10, §7.11, §11, §12, roadmap, conclusion) | Rust-hosted compiler in one workspace producing one binary; Cranelift first; reproducible and signed builds; 14 acceptance suites; offline builds; zero-dependency target |
| Git history and remote (HEAD `76f9308`) | GitHub remote `NickelsKent/rAiL`; one author; PR #1 merged with a merge commit, not a squash |
| `.gitattributes` | LF line endings enforced |

Inferences the lead made and the interview then settled: Rust is the
implementation language (confirmed by Q12); GitHub pull requests (confirmed by
Q1); no staging or production, so "deployment" means publishing releases
(confirmed by Q9); the owner is the single approver (confirmed by Q2); the
single merge commit on PR #1 was one data point, not a practice (the owner chose
squash in Q1).

### Quality engineer (`contributions/aidlc-quality-agent.md`)

Read the spec (§2.5, §8.4, §8.7, §9, §11, §12, roadmap, risk table), the scope
document, the initiative brief, `aidlc-state.md`, the scope file and the rule
files. Contributed: an acceptance-first `custom` methodology as an option;
phase exits from the scope document; recorded CI volumes and seed policy;
coverage details (tool, metric, exclusions, Linux-only measurement, ratchet);
three CI levels; T13 evidence only from reference machines; the nightly Rust
lane for T10; the question of test-only crates; test patterns (seeded property
tests, golden files, round-trip properties, regression tests for defects); a
skeleton that stops at the agent loop as its preferred option.

### Developer (`contributions/aidlc-developer-agent.md`)

Read the spec (naming, MVP architecture, §2.7, §6.4, §7.1, §7.6, §9, R12), the
Ideation artifacts and the lead draft. Contributed: lowercase `rail` in
identifiers; Rust naming guidelines; layer boundaries (one-way dependencies,
thin CLI and RAP frontends, runtime kept apart); the three kinds of failure
(diagnostics, typed errors, panics caught at the frontend); no printing from
library code; determinism as a coding convention; `unsafe` confined to allowed
crates; reasons and approval for new dependencies; a repository layout; Rust
2024 and linting all targets; the question of where the spec lives. Supported a
skeleton that reaches a native binary.

### Security engineer (`contributions/aidlc-devsecops-agent.md`)

Read the spec (§5.5, §5.8, §5.9, §6.2, §6.5, §7.5–§7.11, §9, T10–T14), the
Ideation artifacts, `.gitignore` and the lead draft, and confirmed there is no
`.github/`, `LICENSE` or CI configuration. Contributed: the CI security check
set (`cargo-deny`, `gitleaks` and GitHub secret scanning, lockfile-exact builds,
fuzz runs, sanitizers and `miri`, pinned CI actions with read-only
permissions); an explicit statement that DAST is replaced by fuzzing,
sanitizers and T12; secret handling; release supply chain from the first
release (checksums, build record, dependency list); branch and tag protection;
dependency practices for agent-written code; a risk-based security review as an
alternative to reviewing every pull request; the license question.

## Interview Decisions

| # | Question | Owner's answer |
|---|----------|----------------|
| Q1 | How changes reach `main` | **A.** Short-lived branches, a GitHub pull request for each, green CI required, squash-merged into one commit per piece of work |
| Q2 | Pull request approval | **B.** The owner's approval plus an automated AI review on every pull request |
| Q3 | Walking skeleton | **A.** All the way to a native binary: a one-function module parsed, checked, served through the agent protocol server, compiled and run on macOS and Linux |
| Q4 | When tests are written | **B.** Tests first (test-driven development) for everything |
| Q5 | Coverage rule | **A.** At least 80% line coverage, measured on Linux; corpus, test harnesses and grammar-generated code excluded; the floor can go up but never down |
| Q6 | Rust compiler version | **A.** One pinned stable version for building, testing and releases, plus a separately pinned nightly version only for fuzzing, memory-error checks and `miri` |
| Q7 | What "zero dependencies" covers | **A.** Runtime has zero dependencies; the compiler may use a few crates, each with a reason and the owner's approval; test-only crates allowed |
| Q8 | Where heavy checks run | **A.** Three levels: quick pull-request check on hosted macOS and Linux; nightly with larger volumes and short fuzzing; full-volume release check on the owner's machines; performance numbers only from the reference machines |
| Q9 | How releases are cut | **A.** Version tags on `main` from `v0.1.0`, after the full-volume check and the owner's approval; checksums, build record and dependency list from the first release; signed, reproducible user packages with Phase 4; a pushed tag never moves |
| Q10 | Security checks in CI | **A.** Full set: `cargo-deny` (also weekly), secret scanning, lockfile-exact builds, a short fuzz run, pinned CI actions with read-only permissions |
| Q11 | License | **A.** MIT OR Apache-2.0 |
| Q12 | Rust conventions | **A.** The full set proposed by the developer |
| Q13 | Where the authoritative spec lives | **B.** A `spec/` folder in the repository; the knowledge copy is synced from it |
| Q14 | Firm rules | **A–I, all nine** |
| Q15 | Skeleton versus phase order | **A.** One-time exception: thin native path first as the skeleton, then phase order; the agent loop stays the first usable slice |
| Summary | Consolidated summary confirmation | **Looks correct** (confirmed again, unchanged, before the documents were re-saved) |

The nine firm rules from Q14 are recorded in `discovered-rules.md`, four as
`ALWAYS` rules (C, E, G, H) and five as `NEVER` rules (A, B, D, F, I).

## How Each Objection Was Resolved

| Raised by | Objection | Resolution |
|-----------|-----------|------------|
| Quality | Phase exits should come from the scope document, not the raw spec roadmap | Adopted. Testing Posture names the approved scope document as the authority for phase exits |
| Quality | "Never lowered" cannot be enforced while CI volumes are unrecorded | Adopted through firm rule A (Q14), which names the recorded CI volume. Testing Posture records that each suite's CI volume and seed policy is written down in the repository and that reducing it counts as lowering a pass criterion |
| Quality | Nothing said where T13 performance evidence comes from | Resolved by Q8: performance numbers only count from the owner's reference machines; hosted runner results are never T13 evidence |
| Quality | Pinning Rust ignores that T10 needs nightly | Resolved by Q6: a separately pinned nightly lane only for fuzzing, memory-error checks and `miri` |
| Developer | "One crate workspace" is ambiguous | Resolved by Q12: one Cargo workspace with several crates, building one `rail` program. It is recorded as a convention, not a firm rule |
| Developer | Code Style lacked error-handling, layering and `unsafe` conventions | Resolved by Q12: the full set, including typed errors, diagnostics as data, a protocol server that survives a bad request, and `unsafe` only in allowed crates |
| Developer | Where the spec lives was not settled | Resolved by Q13: the authoritative copy moves to `spec/`; the knowledge copy is synced from it |
| Security | Deferring all signing and reproducibility to Phase 4 mixes the product feature with our own supply chain | Resolved by Q9: checksums, a build record and a dependency list from the first release; signed, reproducible packages for rAiL users with Phase 4 |
| Security | The draft was silent on dependency scanning, secret scanning and DAST | Resolved by Q10: the full check set is adopted, and DAST is recorded as explicitly replaced by fuzzing, sanitizers and T12 |

Support suggestions the owner did not adopt:

- The quality engineer's `custom` acceptance-first methodology (Q4 option C)
  and its preference for a skeleton that stops at the agent loop (Q3 option B).
  The owner chose full TDD and a native-binary skeleton.
- The security engineer's risk-based security review on sensitive pull
  requests only (Q2 options C and D). The owner chose an automated AI review on
  every pull request plus the owner's approval.

## Remaining Uncertainty and Follow-up Work

1. **Exact tools and crates are deferred to design.** The interview confirmed
   the checks, not every tool. The coverage tool, the property-testing,
   golden-file and fuzzing crates, the exact Rust stable and nightly versions,
   the AI review tool for pull requests, and how the three CI levels map to
   workflow files are chosen during design. Every crate chosen still falls
   under firm rule H, and package names are confirmed in the registry, not from
   memory.
2. **Spec wording must be updated.** Q7 narrows the spec's zero-dependency
   target (§11.2 and T14's "toolchain resolves zero external packages") to a
   zero-dependency runtime. Under firm rule C, the spec text is updated in the
   same change that records this decision.
3. **Construction follow-up tasks:** add the `LICENSE` files for MIT OR
   Apache-2.0 (Q11); move the authoritative spec into `spec/` and set up the
   sync to the knowledge copy (Q13); record each acceptance suite's CI and
   release volumes and seed policy in the repository.
4. **Fuzz placement in CI.** Q8 puts short fuzzing in the nightly run, and Q10
   lists a short fuzz run among the CI security checks. Whether a smaller fuzz
   smoke run also gates each pull request is left to CI design.
5. **Walking skeleton and the scope file.** The owner affirmed a walking
   skeleton, but `.claude/scopes/aidlc-rail-toolchain.md` has no
   `skeleton: on` field. Whether the engine's skeleton checkpoint applies to
   this intent should be confirmed before Construction starts.
6. **Not asked, so not recorded as practice:** the lead's suggestion of each
   benchmark-corpus language's own formatter, the developer's repository
   layout and naming details beyond Rust conventions, branch and tag
   protection settings on GitHub, and an update bot for dependencies. These can
   be picked up in design, and none of them is a team practice yet.
7. **Public or private repository.** Not asked. It decides whether GitHub's own
   secret scanning with push protection is available at no cost; `gitleaks`
   covers secret scanning either way.
