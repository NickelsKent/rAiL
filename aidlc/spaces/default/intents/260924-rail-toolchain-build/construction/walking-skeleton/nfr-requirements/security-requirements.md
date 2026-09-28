# Security Requirements — walking-skeleton (U1)

The walking skeleton has no users, accounts, network access or stored data. Its security job is narrower, and it matters because later units build on it:

- It reads **untrusted input**: rAiL source written by agents, and protocol messages from agents.
- It **builds and runs native code** from that input.
- It must do both without crashing, without escaping the workspace, and without letting anything but protocol messages reach the protocol stream.

## Sources

- `requirements.md`: NFR3 (determinism), NFR4 (memory safety), NFR5 (capability safety), NFR7 (dependencies), NFR10 (supply-chain hygiene), FR13.1, FR13.2, FR27.
- `functional-spec.md` and `rules.md` for this unit: workflows WF1–WF6, rules BR1.x–BR6.x.
- `contract-summary.md`: E1 (RAP), E2 (CLI), E7 (compiled program interface).
- Team practices: firm rules on secrets, unsafe code, deterministic output, and the CI security checks.
- `nfr-requirements-questions.md` [Q1]–[Q7], confirmed.

## Scope and data classification

| Asset | Classification | Notes |
|---|---|---|
| rAiL source files in the workspace | Internal, untrusted content | Written by agents; parsed and compiled |
| Protocol messages on standard input | Untrusted | JSON from any agent that starts `rail rap` |
| Built executables in the workspace build directory | Internal | Derived from untrusted source |
| Secrets, credentials, personal data | None handled | The skeleton reads no environment secrets and stores nothing |

No regulatory framework applies (no personal, payment or health data). The compliance points that do apply are licences and the supply-chain rules below.

## Trust boundaries

1. **Agent → protocol server** (standard input): untrusted bytes, framing and JSON.
2. **Workspace files → parser**: untrusted source text.
3. **Toolchain → system `cc`**: a child process the toolchain starts to link.
4. **Toolchain → built program**: a child process running code compiled from untrusted source.

## Threat model (STRIDE)

| Threat | Where | Risk | Requirement |
|---|---|---|---|
| Tampering / DoS: malformed or oversized protocol messages crash or exhaust the server | Boundary 1 | High | NFR4.3, NFR4.4, NFR4.5 |
| DoS: deeply nested JSON or source overflows the toolchain's stack | Boundaries 1, 2 | High | NFR4.4 |
| Information disclosure / tampering: a module path escapes the workspace (`..`, absolute path, symlink) | Boundary 2 | Medium | NFR5.3 |
| Elevation / tampering: command injection through the link step | Boundary 3 | Medium | NFR5.4 |
| DoS: a built program never ends and blocks the protocol server | Boundary 4 | Medium | NFR5.5 |
| Elevation: a built program gets ambient authority | Boundary 4 | Low (no capabilities exist yet) | NFR5.1, NFR5.2 |
| Tampering: program output corrupts the protocol stream | Boundaries 1, 4 | Medium | NFR5.6 |
| Supply chain: a vulnerable or unapproved crate, or a leaked secret | Repository and CI | Medium | NFR7.4, NFR7.5, NFR10.1 |
| Repudiation | — | Not applicable | Nothing is attributed to users; audit logging arrives with U15 |

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR4.1 | Safe Rust everywhere except the runtime crate and the Cranelift backend crate. Every `unsafe` block there carries a safety comment. `unsafe` is forbidden (`#![forbid(unsafe_code)]`) in every other crate. | clippy and a CI check that `forbid(unsafe_code)` is present in the other crates |
| NFR4.2 | Compiled programs use checked arithmetic. Overflow, and division or remainder by zero, trap with exit code 70 and never produce a wrong value (BR5.3, BR5.4). | Run tests for each trap kind on both platforms |
| NFR4.3 | The protocol server's JSON parser is written in-house and follows the JSON standard (RFC 8259) strictly. It accepts only valid UTF-8, rejects invalid input with a structured error (`-32700`), and never panics on any input. Duplicate object keys are rejected. [Q1] [Q6] | JSONTestSuite cases (accepted, rejected); seeded random-input test on stable in every CI run; coverage-guided fuzz target |
| NFR4.4 | Fixed input limits. A protocol message body is at most 16 MiB (a larger `Content-Length` is answered with `-32600`, and the body is read and discarded so framing recovers). JSON nesting is at most 64 levels. A source file is at most 16 MiB. rAiL expression nesting is at most 256 levels. Anything over a source limit is reported as `SKL001` with its location. No limit is enforced by crashing. | Boundary tests at limit and limit + 1 |
| NFR4.5 | The coverage-guided fuzz target for the JSON parser runs in the nightly CI run. A crash, hang or panic fails the run, and every fuzz finding is fixed with a reproducing test. [Q6] [Q7] | Nightly CI job |
| NFR5.1 | Compiled skeleton programs receive no capabilities. `Caps` is empty, and the only side effect a program can have is writing to standard output through `skel.print_i64`. | Type-checker tests (BR2.2, BR2.7); review of the runtime's C-library calls |
| NFR5.2 | The runtime calls only these C-library functions: `write`, `exit`, and what the platform needs to start a process. No file, network, process or environment access. | CI symbol check on the runtime library (undefined symbols against an allow-list) |
| NFR5.3 | Module paths resolve only inside the workspace root given to `initialize`, or the current directory on the command line. Absolute paths, `..` segments, and paths whose resolved target (after following symlinks) leaves the root are rejected with `module.not_found`. The build writes only inside the workspace's build directory. | Path tests: `..`, absolute, symlink out of the root |
| NFR5.4 | The link step starts `cc` directly with an argument list, never through a shell. No part of a module name or path is interpreted by a shell. | Test with module and directory names containing spaces and shell metacharacters |
| NFR5.5 | `run` gives the program a fixed wall-clock limit of 10 seconds. When it is exceeded, the program is killed and the operation fails with `run.failed` ("time limit exceeded"). The program starts with no arguments, empty standard input, and its own captured output streams. | Test of the run operation with a test-only helper executable that never ends (skeleton rAiL cannot yet express an endless loop without running out of stack) |
| NFR5.6 | While `rail rap` runs, only protocol frames are written to its standard output. Program output is captured from the child process and returned inside the `run.run` result (BR6.5). | Byte-level stream test |
| NFR7.4 | The only compiler crates the skeleton adds are from the pre-approved Cranelift family (listed in `tech-stack-decisions.md`), each with a recorded reason. No JSON crate and no test crate. [Q1] [Q4] | `cargo-deny` bans/sources config; review of `Cargo.lock` |
| NFR7.5 | `libfuzzer-sys` is used only in a separate fuzz crate that is excluded from the shipped `rail` binary and from coverage, approved with the reason "coverage-guided fuzzing of the in-house JSON parser at the protocol boundary". [Q7] | `cargo tree` of the `rail` binary shows no `libfuzzer-sys` |
| NFR10.1 | From the first pull request, CI runs the affirmed security checks. That covers `cargo-deny` (advisories, bans, sources and licences; advisories also weekly), `gitleaks` plus GitHub secret scanning, and builds and tests with `--locked`. Third-party actions are pinned to exact commits with read-only default permissions. | CI configuration review |
| NFR10.2 | No secret is needed to build, test or verify the skeleton, and none is stored in the repository. | `gitleaks` in CI |

## Licences

Every dependency's licence must fit **MIT OR Apache-2.0** and pass `cargo-deny`'s licence check. That includes the Cranelift crates, `libfuzzer-sys`, and the JSONTestSuite files kept as test data. The exact licence of each is checked against its published metadata when it is added. It is not assumed here.

## Out of scope for the skeleton

- Authentication and authorisation: the protocol is a local stdio process started by the agent's own user, so there is nobody to authenticate.
- Encryption: no network and no stored secrets.
- Sandboxing and the no-sandbox warning: U7 (FR35) and U15 (FR36).
- Audit log: U15 (FR37).
