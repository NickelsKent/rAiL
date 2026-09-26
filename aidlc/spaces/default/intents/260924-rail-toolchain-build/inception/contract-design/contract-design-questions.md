# Contract Design — Questions

A contract is the agreement at a boundary: what crosses it, in what shape, how, and what happens on failure. rAiL has two kinds of boundary:

- **Inside the `rail` program**, between units that are Rust crates in one workspace. Examples: the canonical tree, typed definitions, the diagnostic stream, RIR, and the runtime ABI.
- **Outside it**, where agents, you or other programs consume rAiL. Examples: the agent protocol (RAP), the command line's JSON output, the registry protocol, and the file formats (`.rlc`, `.rlb`, `.rlh`, `.rlmap`, `rail.pkg`, `rail.lock`, `rail.review`, run manifests, patches, audit logs).

Most shapes are already fixed by the spec (the diagnostic schema, the patch format, the binary layout, the lockfile fields). These questions cover how the contracts are written down, versioned and made safe on failure.

## Q1. How should contracts between the internal units be pinned?

These boundaries are Rust APIs between crates of one workspace, built and released together.

A. Each boundary is a small public Rust interface (types and traits) in the provider's crate, listed in the contract summary with its operations, data shapes and error types; a change to it is a contract change that updates the summary in the same pull request (recommended)
B. A separate "interface" crate per boundary that both sides depend on
C. Prose descriptions only; interfaces emerge during Code Generation
X. Other (please specify)

[Answer]: A

## Q2. How should the agent protocol (RAP) frame messages over standard input and output?

JSON-RPC 2.0 is fixed by the spec. How each message is delimited is not.

A. `Content-Length` headers before each message, as the Language Server Protocol does; handles large payloads and is familiar to agent tooling; every method's parameters and results are specified as JSON Schema (recommended)
B. One JSON message per line (newline-delimited); simpler, but every message must avoid raw newlines
X. Other (please specify)

[Answer]: A

## Q3. How are external contracts versioned before and after 1.0?

External contracts are RAP, the command line's `--json` output, the registry protocol and the file formats.

A. Each external contract carries its own version. RAP negotiates it in an `initialize` call, and file formats carry a format version (the binary form already has one). Before 1.0 (versions 0.x) a breaking change only needs a version bump and a spec update. From 1.0, breaking changes need a new major version, and additive changes must be ignorable by older readers (recommended)
B. No versioning until 1.0; everything may change freely until then
C. Strict compatibility from the first release
X. Other (please specify)

[Answer]: A

## Q4. What protocol does the local registry speak?

The registry runs only locally. Package integrity already comes from Ed25519 signatures, BLAKE3 hashes and transparency-log proofs, not from the transport.

A. HTTP with a small resource-style API (fetch index, fetch archive by hash, fetch log proofs and advisories, publish). TLS is supported but not required on localhost, because integrity never depends on the transport (recommended)
B. HTTPS required always, including on localhost
C. No network protocol: the client reads the registry's directory directly
X. Other (please specify)

[Answer]: A

## Q5. Should long-running RAP requests support cancellation and deadlines?

Some requests take a long time: building, running tests, bounded model checking, fuzzing-style property tests. An agent may want to stop one, or give it a time limit.

A. Yes: a cancel notification for any in-flight request, an optional per-request deadline, and a structured "cancelled" or "deadline exceeded" error; cancelled work never leaves the warm workspace half-updated (recommended)
B. Cancellation only, no deadlines
C. Neither; the agent restarts the server to stop work
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Internal contracts** between units are small public Rust interfaces (types and traits) in the provider's crate. The contract summary lists each one's operations, data shapes and error types. Changing one is a contract change, and the summary is updated in the same pull request. [Q1]
- **RAP framing:** JSON-RPC 2.0 over standard input and output, with `Content-Length` headers before each message, as in the Language Server Protocol. Every method's parameters and results are specified as JSON Schema. [Q2]
- **Versioning:** each external contract carries its own version. That covers RAP (negotiated in `initialize`), the `--json` output, the registry protocol and every file format (a format version inside each file). Before 1.0, a breaking change needs a version bump and a spec update. From 1.0, breaking changes need a new major version, and additive changes must be ignorable by older readers. [Q3]
- **Registry protocol:** HTTP with a small resource-style API (index, archive by hash, log proofs, advisories, publish). TLS is supported but not required on localhost, because integrity never depends on the transport. [Q4]
- **Cancellation and deadlines:** RAP supports a cancel notification for any in-flight request and an optional per-request deadline. Both end with a structured "cancelled" or "deadline exceeded" error. Cancelled work never leaves the warm workspace half-updated. [Q5]

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
