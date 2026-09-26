# Requirement-to-Unit Map — rAiL Toolchain

This plan has no user stories (the User Stories stage is skipped), so every functional requirement from `requirements.md` is mapped to the unit that implements it. Each requirement and sub-requirement has exactly one implementing unit. Units listed in `unit-of-work.md` as "extending" a building block implement only the requirements mapped to them here.

## Requirement map

| Requirement | Unit ID | Directory |
|---|---|---|
| FR1 | U3 | u3-syntax |
| FR1.1 | U3 | u3-syntax |
| FR2 | U3 | u3-syntax |
| FR2.1 | U3 | u3-syntax |
| FR2.2 | U3 | u3-syntax |
| FR2.3 | U3 | u3-syntax |
| FR2.4 | U3 | u3-syntax |
| FR2.5 | U3 | u3-syntax |
| FR3 | U3 | u3-syntax |
| FR4 | U3 | u3-syntax |
| FR4.1 | U3 | u3-syntax |
| FR4.2 | U3 | u3-syntax |
| FR4.3 | U3 | u3-syntax |
| FR5 | U3 | u3-syntax |
| FR5.1 | U3 | u3-syntax |
| FR5.2 | U3 | u3-syntax |
| FR5.3 | U3 | u3-syntax |
| FR5.4 | U3 | u3-syntax |
| FR5.5 | U3 | u3-syntax |
| FR6 | U3 | u3-syntax |
| FR7 | U4 | u4-checker-and-lints |
| FR8 | U4 | u4-checker-and-lints |
| FR8.1 | U4 | u4-checker-and-lints |
| FR8.2 | U4 | u4-checker-and-lints |
| FR9 | U4 | u4-checker-and-lints |
| FR9.1 | U4 | u4-checker-and-lints |
| FR10 | U4 | u4-checker-and-lints |
| FR11 | U4 | u4-checker-and-lints |
| FR11.1 | U4 | u4-checker-and-lints |
| FR11.2 | U4 | u4-checker-and-lints |
| FR11.3 | U4 | u4-checker-and-lints |
| FR11.4 | U4 | u4-checker-and-lints |
| FR11.5 | U4 | u4-checker-and-lints |
| FR12 | U4 | u4-checker-and-lints |
| FR13 | U5 | u5-agent-loop |
| FR13.1 | U5 | u5-agent-loop |
| FR13.2 | U5 | u5-agent-loop |
| FR14 | U5 | u5-agent-loop |
| FR15 | U5 | u5-agent-loop |
| FR15.1 | U5 | u5-agent-loop |
| FR16 | U5 | u5-agent-loop |
| FR17 | U3 | u3-syntax |
| FR18 | U3 | u3-syntax |
| FR18.1 | U3 | u3-syntax |
| FR18.2 | U3 | u3-syntax |
| FR19 | U5 | u5-agent-loop |
| FR19.1 | U5 | u5-agent-loop |
| FR19.2 | U5 | u5-agent-loop |
| FR19.3 | U5 | u5-agent-loop |
| FR20 | U5 | u5-agent-loop |
| FR21 | U6 | u6-execution-core |
| FR22 | U6 | u6-execution-core |
| FR23 | U6 | u6-execution-core |
| FR24 | U6 | u6-execution-core |
| FR24.1 | U6 | u6-execution-core |
| FR24.2 | U6 | u6-execution-core |
| FR25 | U7 | u7-system-and-stdlib |
| FR26 | U7 | u7-system-and-stdlib |
| FR26.1 | U7 | u7-system-and-stdlib |
| FR26.2 | U7 | u7-system-and-stdlib |
| FR26.3 | U7 | u7-system-and-stdlib |
| FR27 | U1 | u1-walking-skeleton |
| FR28 | U8 | u8-concurrency-and-system-std |
| FR29 | U8 | u8-concurrency-and-system-std |
| FR30 | U7 | u7-system-and-stdlib |
| FR31 | U8 | u8-concurrency-and-system-std |
| FR32 | U9 | u9-crypto-and-net |
| FR32.1 | U9 | u9-crypto-and-net |
| FR32.2 | U9 | u9-crypto-and-net |
| FR32.3 | U9 | u9-crypto-and-net |
| FR32.4 | U9 | u9-crypto-and-net |
| FR33 | U7 | u7-system-and-stdlib |
| FR34 | U7 | u7-system-and-stdlib |
| FR35 | U7 | u7-system-and-stdlib |
| FR36 | U15 | u15-wasm-sandbox |
| FR37 | U15 | u15-wasm-sandbox |
| FR38 | U10 | u10-supply-chain |
| FR39 | U10 | u10-supply-chain |
| FR40 | U10 | u10-supply-chain |
| FR41 | U10 | u10-supply-chain |
| FR42 | U10 | u10-supply-chain |
| FR43 | U10 | u10-supply-chain |
| FR44 | U10 | u10-supply-chain |
| FR45 | U10 | u10-supply-chain |
| FR46 | U10 | u10-supply-chain |
| FR47 | U11 | u11-first-party-packages |
| FR48 | U12 | u12-release-backend |
| FR49 | U12 | u12-release-backend |
| FR50 | U13 | u13-deep-analysis |
| FR51 | U13 | u13-deep-analysis |
| FR52 | U13 | u13-deep-analysis |
| FR53 | U14 | u14-debugger |
| FR54 | U16 | u16-benchmark-corpus |
| FR55 | U17 | u17-measurement-tools |
| FR56 | U17 | u17-measurement-tools |
| FR57 | U2 | u2-acceptance-harness |
| FR58 | — (N/A) | Process requirement (spec upkeep in the repository), carried by every unit under firm rule C rather than one unit |
| FR58.1 | — (N/A) | Process requirement (spec upkeep in the repository), carried by every unit under firm rule C rather than one unit |
| FR59 | — (Deferred) | Windows support is deferred beyond this work (Could); no unit |

## Cross-cutting requirements

These apply to every unit, not to one:

- NFR3 (determinism), NFR4 (memory safety), NFR5 (capability safety), NFR6 (portability), NFR9 (code quality) and NFR10 (supply-chain hygiene).
- The spec-upkeep rule (FR58).
- NFR1 (performance targets) is measured by U17 and delivered mainly by U6 and U12.
- NFR2 (agent reliability) is measured by U17.
- NFR7 (dependencies) is enforced in every unit, with a runtime check owned by U6.
- NFR8 (TLS assurance) belongs to U9.

## Implementation order within each unit

Inside every unit, work follows the tests-first practice:

1. The unit's acceptance suites are added to the harness (U2) and run, failing.
2. Then, for each layer, the unit tests are written and fail, and the code follows.

Where a unit has several requirements, the order within it is dependency-first and is fixed in each unit's Functional Design:

- **U3:** FR1 → FR2 → FR3 → FR4 → FR5 → FR17 → FR18 → FR6.
- **U4:** FR7 → FR8 → FR9 → FR10 → FR11 → FR12.
- **U5:** FR15 → FR16 → FR14 → FR20 → FR19 → FR13.
- **U6:** FR21 → FR22 → FR23 → FR24.
- **U7:** FR30 → FR35 → FR25 → FR33 → FR34 → FR26.
- **U8:** FR28 → FR29 → FR31.
- **U10:** FR38 → FR39 → FR41 → FR40 → FR42 → FR43 → FR44 → FR45 → FR46.
- **U12:** FR49 → FR48.
- **U13:** FR50 → FR51 → FR52.
- **U15:** FR37 → FR36.
- **U17:** FR55 → FR56.

## Coverage check

- **Every requirement is assigned.** 96 of 99 requirement IDs map to exactly one unit. FR58 and FR58.1 are process requirements carried by every unit. FR59 (Windows) is deferred.
- **Every unit has at least one requirement:**

| Unit | IDs | Unit | IDs |
|---|---|---|---|
| U1 | 1 | U10 | 9 |
| U2 | 1 | U11 | 1 |
| U3 | 24 | U12 | 2 |
| U4 | 14 | U13 | 3 |
| U5 | 12 | U14 | 1 |
| U6 | 6 | U15 | 2 |
| U7 | 9 | U16 | 1 |
| U8 | 3 | U17 | 2 |
| U9 | 5 | | |
