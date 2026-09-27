# Performance Requirements — walking-skeleton (U1)

The walking skeleton proves that the pieces connect. It is not a performance milestone. The spec's §11.2 targets (NFR1) apply to the release toolchain, are measured by the performance harness (U17), and count only when they come from the owner's reference machines. So the skeleton claims none of them. Its only timing requirement stops a stuck process from hanging CI. [Q5]

## Sources

- `requirements.md`: NFR1 (performance targets and how they are measured), FR27.
- Team practice (Testing Posture): performance numbers count only from the reference machines; hosted CI results are never T13 evidence.
- `functional-spec.md` for this unit: WF3, WF4, WF6.
- `nfr-requirements-questions.md` [Q5] (A: hang guard only).

## Requirements

| ID | Requirement | Measurement |
|---|---|---|
| NFR1.1 | The skeleton claims no §11.2 target. No timing taken from the skeleton or from hosted CI is recorded as evidence for NFR1 or T13. | Review: no performance claims in the skeleton's docs or CI output |
| NFR1.2 | Hang guard: every test in CI has a time limit of 60 seconds, and the recorded end-to-end verification command has a time limit of 10 minutes per platform. Exceeding a limit fails the run with a message naming what was running. | CI job and test-harness time limits |
| NFR1.3 | A program started by `run` is stopped after 10 seconds of wall-clock time (see NFR5.5 in `security-requirements.md`). This bounds the time of any `run` or `run.run` request. | Run-operation test |

## Explicit non-requirements

- No budget for build time, check time, start-up time, binary size or memory in the skeleton.
- No incremental-check cache: each request re-reads and re-checks its module. The warm cache arrives with U4 and U5, where NFR1's incremental targets apply.
- No release-mode build: the release backend is U12.
