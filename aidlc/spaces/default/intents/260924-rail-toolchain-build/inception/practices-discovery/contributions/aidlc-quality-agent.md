**Collaborator:** aidlc-quality-agent

## Contribution

Blind quality review of the lead draft (`team-practices.md`, `discovered-rules.md`,
`evidence.md`). Evidence read: the rAiL spec (§2.5, §8.4, §8.7, §9 testing and
benchmarking rows, §11, §12, roadmap, risk table), `scope-document.md`,
`initiative-brief.md`, `aidlc-state.md` (Test Strategy: Standard; Construction
Checkpoints: enabled), `.claude/scopes/aidlc-rail-toolchain.md` (no `skeleton:`
field, no coverage floor), and `org.md` / `project.md` / `phases/inception.md`.
The project is greenfield, so there is no test code, CI setup or coverage
configuration to inspect. Everything below is a suggestion for the interview
and is not an established practice.

### 1. Testing Posture: what the draft gets right and what it leaves open

The draft's structure is correct. `Methodology` and `Ordering` are separate
fields, and the tooling and coverage notes are extra bullets that do not replace
them. The draft also keeps the `org.md` default (test-after) and does not choose
a methodology on its own. Four gaps need resolving before the owner confirms
the section:

1. **Methodology fits this project better as `custom` (acceptance-first).**
   Spec §12 defines every acceptance suite up front, with an objective,
   automated pass criterion. The spec already acts as an executable
   specification. A cadence that fits:
   - *Methodology*: `custom`
   - *Ordering*: "At the start of each phase, write that phase's
     acceptance-suite harness (from spec §12, adjusted by the scope document) so
     that it runs and fails. Then implement each testable layer and write and
     run its unit and property tests before starting the next layer. The phase
     is done when its exit suites pass at CI volume."
   Full TDD at unit level is possible, but it is costly for generated
   parser/printer code. Plain test-after gives up the main benefit the spec
   already provides. The draft's Q4 offers this choice. I recommend putting
   `custom` first as the suggested answer, with test-after as the alternative.

2. **Phase exit criteria must come from the scope document, not the raw spec
   roadmap.** The draft says "as listed in the spec roadmap". The approved
   scope document changes that roadmap in three places. Windows is not
   required. The §11.3 gate moved from the Phase 1 exit to after the full
   toolchain exists [Q8]. The Wasm host and the T11 limits may slip from the
   Phase 3 exit [Q10]. The first-usable-slice exit is T1, T2, T4, T5 and T6 at CI
   volumes [Q1]. The note should cite the scope document as the authority so
   Build and Test does not check against criteria that no longer apply.

3. **The CI volumes are part of the pass criterion, so they need recorded
   values.** [Q5] approved scaled volumes in CI, but no numbers exist yet. The
   rule "never lower a pass criterion" is only enforceable if each suite's CI
   volume (for example, T1 at 10⁴ valid modules plus 10⁴ mutations instead of
   10⁶ plus 10⁶) and its seed policy are written down once, in a checked-in
   config file. After that, reducing a CI volume counts as lowering a pass
   criterion. Suggested wording for the note: "Each suite's CI volume and
   release volume are recorded in the repo. Reducing either one counts as
   lowering a pass criterion."

4. **The 80% coverage floor is a new commitment, not a default.** `org.md`
   applies the 80% floor only to `mvp`, `enterprise`, `feature`, `infra` and
   `classic`. The custom `rail-toolchain` scope gets no floor unless the owner
   affirms one. The draft already flags this correctly. If the owner affirms a
   floor, the interview must also settle these points, or the floor cannot be
   measured consistently:
   - **Tool.** `cargo-llvm-cov` (a Rust coverage tool built on LLVM source-based
     coverage) is the usual choice. Per the engineering rules, confirm the exact
     package name in the registry when adding it, not from memory.
   - **Metric.** Line coverage (the `org.md` wording) or region/branch coverage.
     For a checker and pattern-match compiler, branch coverage says more about
     test quality. I suggest line coverage as the gate and branch coverage as a
     reported figure only.
   - **What counts.** Exclude the benchmark corpus in Rust, Go and OCaml, the
     `rail-perf` harness, the acceptance-suite harness crates and test-only
     code. Decide whether code generated from the shared grammar (spec
     "Minimum viable prototype architecture") counts. I suggest counting it,
     because round-trip correctness depends on it.
   - **Where it is measured.** Measure it on one platform (Linux) as the gate.
     The tests still run on both platforms. Measuring on both doubles CI time
     and adds no signal for platform-independent code.
   - **Fixed floor or ratchet.** Choose between a fixed 80% and a ratchet that
     never lets coverage drop below its current level once it is above 80%.
     Coverage early in Phase 0 may be low while harnesses come first.
   - **Code written in rAiL.** Coverage of the standard library in `.rlc`
     cannot be measured. The spec defines `rail test` but no coverage tool for
     rAiL. The floor therefore applies only to Rust. rAiL standard-library
     quality rests on T9 plus the `tst` items. State this explicitly.

### 2. CI quality gates: three tiers that the draft implies but does not name

The draft names everyday CI and a release check. For a solo builder with
macOS runners that are slow and costly, a named middle tier keeps pull requests
fast without dropping the heavy suites. Suggested structure (the owner should
confirm the tiers and the time budget):

| Tier | Trigger | Runs on | Contents | Blocks |
|------|---------|---------|----------|--------|
| PR gate | Every PR and every merge to `main` | Hosted macOS and Linux | `rustfmt --check`, `clippy -D warnings`, unit, integration and property tests with fixed seeds, the coverage gate (Linux), every acceptance suite whose phase has started, at CI volume | Merge |
| Scheduled (nightly) | Timer on `main` | Hosted, or the owner's machines | Acceptance suites at higher volume with **random seeds that are logged**, short fuzz runs (for example 30 minutes per target), a cross-OS output comparison for T1, T3 and T7 | Nothing directly. A failure opens a defect that must get a regression test |
| Release check | Before any version tag | The owner's reference machines | Full spec volumes, T10's 24 CPU-hour fuzz run, T13 `rail-perf` (30 runs after 5 warm-ups, bootstrap intervals per §11.1), T14 on a fresh offline machine, T12's DNS-rebinding cases | The tag |

Quality points behind this table:

- **Performance on shared runners is not valid evidence for T13.** Hosted
  runners are noisy and are not the §11.1 reference machines. T13 only counts
  when it runs on the owner's reference machines, and the release evidence
  records which machine produced each result. `rail bench` smoke runs in CI can
  catch large regressions, but they are advisory only.
- **Cross-OS determinism needs its own comparison job.** T1, T3 and T7 require
  identical outputs across operating systems. No single OS job can check that.
  A later job must collect the hashes each OS leg uploads and compare them. This
  job type should be named now, because otherwise "green on both OSes" gets
  mistaken for "identical on both OSes".
- **A suite that has passed stays in the PR gate from then on.** Once a phase's
  exit suites pass, they are regression gates for every later change. Suites for
  phases that have not started are reported as "not yet applicable". They are
  never shown as skipped-and-passing.
- **Hermetic suites need hermetic environments.** T14 (no network, fresh
  machine) and the DNS-rebinding part of T12 cannot be reproduced reliably on
  hosted runners. Place them in the release check on controlled machines.
- **PR time budget.** Put a target on the PR gate (for example, under 20 minutes
  on both OSes) so that acceptance volumes are chosen to fit it. This is an
  interview question, not a default.

### 3. Toolchain conflicts the interview must resolve

These come from the testing requirements in the spec. They interact with the
lead's Q8 (pinning the Rust toolchain) and with candidate rules 1, 2 and 5:

1. **T10 needs nightly Rust.** AddressSanitizer for the Rust runtime
   (`-Zsanitizer=address`) and `cargo-fuzz`/libFuzzer both need a nightly
   compiler. Pinning one stable toolchain, as Q8 suggests, will block T10 unless
   a separate, pinned nightly lane is allowed for sanitizer and fuzz jobs only.
   The release binary would still be built on stable. Miri, a Rust interpreter
   that checks for undefined behaviour and also runs on nightly, is worth
   adding for `unsafe` runtime code (reference counting and the allocator).
2. **"Zero dependencies" versus test crates.** §11.2 sets the toolchain's
   transitive dependency count to 0. T14 requires the toolchain to resolve zero
   external packages, and candidate rule 2 limits the runtime to the platform C
   library. The interview must confirm three things. First, whether "zero" means
   zero rAiL packages or zero Rust crates. Second, whether Rust
   dev-dependencies, which never ship in the binary, are allowed. Examples are a
   property-testing crate, a snapshot and golden-file crate, and a fuzzing
   harness. Third, whether they fall under candidate rule 5's vendoring. If test
   crates are not allowed, property testing and shrinking must be written by
   hand. That is costly and risky, which is the case where the engineering rules
   permit a dependency.
3. **Differential testing (T9) needs Rust versions of the corpus in CI.** The
   benchmark corpus is in scope [Q11]. Decide whether the Rust reference
   versions are built and run in the PR gate or only in the scheduled and
   release tiers.

### 4. Test and code patterns to suggest (for Code Generation to adopt)

These are suggestions for the interview to confirm or trim. They follow Rust
conventions, so a Code Generation agent working without project history would
likely produce them anyway:

- Unit tests sit next to the code in `#[cfg(test)] mod tests`. Cross-crate
  integration tests go in each crate's `tests/`. Acceptance suites live in a
  dedicated workspace crate, or an `xtask` command, with one entry per T-ID and
  a volume/seed parameter, so that the same code runs at CI and release volume.
- **Property tests use seeds that are fixed or logged.** Determinism is a
  product property of rAiL (§2.5, §4.8, T1, T3, T7). A test failure that cannot
  be reproduced from its seed is itself a defect.
- **Golden files** cover printer, formatter, `view`, diagnostics (§6.4 schema)
  and RAP responses. Updating golden files is an explicit "bless" step, and the
  resulting diff is reviewed in the PR.
- **Round-trip and bijection properties** (§8.4 items 1 to 4 and R9
  text-to-binary) are written as properties, not examples, from Phase 0. They
  are the unit-level form of T4.
- **Every defect gets a regression test before its fix** (the `bugfix` scope
  floor in `org.md`, applied to every defect).
- **Traceability**: each acceptance test carries its T-ID. After requirements
  analysis, each requirement-level test carries its `FR-n` ID. This lets Build
  and Test report coverage by acceptance criterion as well as by line.
- Mutation testing (`cargo-mutants`) is optional and selective, for example on
  the lint rules behind T8. T8 is already a seeded-defect suite, so I do not
  recommend a mutation-score gate.

### 5. Walking skeleton: quality view

Both options in the draft are valid. From the testing side, two things matter
more than how far the skeleton reaches:

- The verification command must **check outputs**, not just the exit code. It
  must check the canonical output bytes, the diagnostic JSON, the RAP response
  and, if native code is in scope, the program's stdout and exit code. It must
  run the same way locally and in CI (for example, one `cargo xtask skeleton`
  entry).
- The skeleton should include **the smallest working acceptance harness**: T1,
  T2 and T4 at very small volume in CI on both OSes. That proves the test
  infrastructure end to end, which the later phases depend on.

My lean is a skeleton that stops at the agent loop (parse, format, check, RAP,
patch) plus the acceptance harness. This keeps the confirmed phase order [Q7]
and the first-usable-slice definition [Q1]. Going on to Cranelift pulls Phase 2
work forward. It only proves that native execution is possible, and T9 and T10
will prove that later anyway. This is the owner's choice.

### 6. Candidate hard constraints, reviewed for quality

Of the lead's candidates, the quality view supports making these firm, if the
owner states them: **4** (CI green on macOS and Linux before merge), **6**
(never lower a pass criterion or coverage floor, including CI volumes as
described in §1.3) and **7** (no nondeterministic data in compiler output; T3
checks this).

Additional candidates for the interview. They come from the spec's determinism
properties and are not owner-stated yet:

- NEVER re-run a failing test in CI until it passes, and never mark a test
  ignored or skipped without a linked reason. A flaky test is a determinism
  defect in this project.
- ALWAYS run generated-input tests with a fixed or logged seed so that every
  failure can be reproduced.
- NEVER update golden files in bulk without reviewing the diff in the same PR.
- ALWAYS add a regression test that reproduces a defect before fixing it.

### 7. Questions to add to the interview

1. Methodology: `custom` (acceptance harness first per phase, then implement
   each layer and test it) or test-after or TDD? (Adds a recommended answer to
   the draft's Q4.)
2. Coverage: whether there is a floor, and if so the metric (line or branch),
   what is excluded (corpus, harnesses, generated grammar code), where it is
   measured (Linux only), and fixed floor or ratchet. State that `.rlc` code has
   no coverage gate.
3. CI volumes: record per-suite CI and release volumes and seed policy in the
   repo, and treat reducing them as lowering a pass criterion. Yes or no?
4. CI tiers: PR gate, nightly and release check as in the table above. What is
   the PR time budget?
5. A pinned nightly Rust lane for sanitizer, fuzz and Miri jobs only, with the
   release build kept on stable. Yes or no?
6. Rust dev-dependencies for tests (property testing, snapshots, fuzzing):
   allowed, or ruled out by the zero-dependency target?
7. T13 and T14 evidence: only from the owner's named reference machines, with
   the machine identity recorded in the release evidence?
8. Which of the four additional hard-rule candidates in §6 does the owner want
   to state?

## Positions

- AGREE: The draft keeps `Methodology` and `Ordering` as separate structured fields and leaves the methodology at the `org.md` default instead of choosing one without the owner.
- AGREE: The 80% coverage floor is marked as a suggested new commitment. The custom scope gets no floor from `org.md`, so it needs explicit affirmation.
- AGREE: Scaled CI volumes plus a full-volume release check, as already approved in [Q5].
- AGREE: Candidate rules 4, 6 and 7 are the strongest quality constraints to put to the owner.
- OBJECT: "Each phase is done when its exit suites pass, as listed in the spec roadmap" should cite the scope document. [Q1], [Q3], [Q8] and [Q10] changed the Phase 1 and Phase 3 exits and dropped Windows.
- OBJECT: "Never lowered" cannot be enforced while the CI volumes are unrecorded. Each suite's CI volume and seed policy must be written down, and reducing them must count as lowering a pass criterion.
- OBJECT: The draft does not say where T13 performance evidence comes from. Hosted CI runners are not the §11.1 reference machines and must not count as T13 evidence.
- OBJECT: The draft asks about pinning the Rust toolchain (Q8) without noting that T10 (ASan and fuzzing) needs nightly Rust. The interview must decide on a separate nightly lane for those jobs.
