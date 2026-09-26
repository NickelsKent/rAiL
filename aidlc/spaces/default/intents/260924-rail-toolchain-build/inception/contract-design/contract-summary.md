# Contract Summary — rAiL Toolchain

This summary covers every boundary the units must honour so they can be built one after another without rework:

- **Internal contracts** (C-series) sit between units inside the `rail` workspace. [Q1]
- **External contracts** (E-series) are what agents, humans and other programs consume.

Sources:

- The unit DAG in `unit-of-work-dependency.md`; every edge is covered by at least one contract below.
- `unit-of-work.md` and `components.md`.
- `requirements.md` (FR/NFR IDs).
- The rAiL spec, which already fixes most shapes: §2.5 binary layout, §5.3–§5.4 manifest and lockfile, §6.4 diagnostics, §7.9 audit log, §7.11 provenance, §9.4 patches.
- `contract-design-questions.md` ([Q1]–[Q5]).

## Contracts

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| C1 | U3 syntax (thin first version in U1) | U4, U5, U6, U10, U13, U14 | In-process Rust interface | U3 |
| C2 | U4 checker-and-lints (workspace and typing) | U5, U6, U10, U13 | In-process Rust interface | U4 |
| C3 | U4 checker-and-lints (diagnostic stream) | U5, U13; U2 and U17 via JSON | Rust interface + shared JSON schema | U4 |
| C4 | U5 agent-loop (ToolServices operation set) | U7, U10, U12, U13, U14, U15 (they add operations) | In-process Rust interface | U5 |
| C5 | U6 execution-core (RIR and codegen backend interface) | U12, U13, U15 | In-process Rust interface | U6 |
| C6 | U6 execution-core (runtime ABI) | U7, U8, U9, every compiled rAiL program | Static linking; binary layout | U6 |
| C7 | U7 system-and-stdlib (capabilities and run manifest) | U8, U9, U15 | Runtime Rust interface + canonical-text file | U7 |
| C8 | U9 crypto-and-net (crypto primitives) | U10 (Trust), U7 (`std.hash` BLAKE3/SHA-256 bindings) | In-process Rust interface | U9 |
| C9 | U2 acceptance-harness (suite registration) | U3, U5, U7, U8, U10, U13, U15, U17 register suites | Harness Rust interface + recorded config file | U2 |
| C10 | U16 benchmark-corpus (corpus layout) | U2, U3, U7, U8, U10, U17 | Shared directory layout | U16 |
| C11 | U10 supply-chain (RegistryServer) | U10 (PackageManager, Trust), U11 | HTTP resource API [Q4] | U10 |
| E1 | U5 agent-loop (RapServer; methods added by later units) | External: AI agents | JSON-RPC 2.0 over stdio, `Content-Length` framing [Q2] | U5 |
| E2 | U5 agent-loop (Cli; commands added by later units) | External: owner, scripts, CI | Command line; human text or `--json` | U5 |
| E3 | U3 syntax | External: agents, humans, tools | File formats `.rlc` `.rlb` `.rlh` `.rlmap` | U3 |
| E4 | U5 agent-loop | External: agents | Patch and conflict format (canonical text) | U5 |
| E5 | U10 supply-chain | External: package authors, tools | File formats `rail.pkg` `rail.lock` `rail.review`, package archive, provenance record, advisory database | U10 |
| E6 | U7 system-and-stdlib; U15 wasm-sandbox | External: operators of rAiL programs | Run manifest (U7); audit log (U15) | U7 / U15 |
| E7 | U6 execution-core + U7 system-and-stdlib | External: operating system, callers of compiled programs | Process interface: exit codes, stdout/stderr, C ABI for `main` and `ffi` | U6 |
| E8 | U14 debugger | External: editors and debug clients | Debug Adapter Protocol | U14 |

**Every DAG edge is covered:**

| DAG edge | Contracts |
|---|---|
| syntax → walking-skeleton | C1 |
| syntax → acceptance-harness | C9 |
| syntax → benchmark-corpus | C10 |
| checker-and-lints → syntax | C1 |
| agent-loop, execution-core → checker-and-lints | C2, C3 |
| system-and-stdlib → agent-loop, execution-core | C4, C6 |
| concurrency-and-system-std → system-and-stdlib | C6, C7 |
| crypto-and-net → concurrency-and-system-std | C6, C7 |
| supply-chain → agent-loop, execution-core, crypto-and-net | C4, C5, C8, C11 |
| first-party-packages → crypto-and-net, supply-chain | C11, E5 |
| release-backend → execution-core | C5 |
| deep-analysis → agent-loop, execution-core, system-and-stdlib | C3, C4, C5 |
| debugger → agent-loop, execution-core | C4, C5, E8 |
| wasm-sandbox → system-and-stdlib, concurrency | C5, C7 |
| benchmark-corpus → walking-skeleton, acceptance-harness | C10, C9 |
| measurement-tools → system-and-stdlib, benchmark-corpus | E2, C10 |
| acceptance-harness → walking-skeleton | E1, E2 |

## Internal contract specs

Each internal contract is a small public Rust interface in the provider's crate [Q1]. The blocks below pin the operations, data shapes and errors. Exact Rust spelling is settled in Functional Design and must match these shapes. Every fallible operation returns a typed error enum; nothing panics across a boundary (practices: Rust conventions).

### C1 — Canonical tree

```yaml
contract: C1-canonical-tree
provider: u3-syntax
kind: rust-interface
version: 0.1
types:
  Module: {qname: QName, items: "ordered per §2.5", modhash: ModHash}
  Definition: {defid: DefId, kind: "typ|trt|imp|fn|val|tst|allow|meta|ext|req|ens", name: Name, defhash: DefHash}
  DefId: "30-bit value, rendered '#' + 6 lowercase Crockford base32"
  AnchorPath: "defid + '/' + binding name or child path (§2.6)"
  DefHash: "BLAKE3-256 over binary tree, locals as de Bruijn indices, IDs and meta excluded (R10)"
  ModHash: "BLAKE3-256 over the sorted definition table"
  SourceMap: {module: QName, entries: "(map start end #defid path) lines, total both ways (§8.3)"}
operations:
  parse_text: {in: "bytes (.rlc)", out: Module, errors: [SyntaxError]}
  print_text: {in: Module, out: "bytes (.rlc), canonical"}
  encode_binary: {in: Module, out: "bytes (.rlb)"}
  decode_binary: {in: "bytes (.rlb)", out: Module, errors: [DecodeError]}
  view: {in: "Module, options {ids: bool}", out: "bytes (.rlh)", errors: []}
  absorb: {in: "bytes (.rlh), previous Module?", out: Module, errors: [SyntaxError]}
  resolve_anchor: {in: "Module, AnchorPath", out: NodeRef, errors: [AnchorNotFound]}
  source_map: {in: Module, out: SourceMap}
  fresh_defid: {in: "Module", out: DefId, note: "the only non-deterministic operation; seedable for tests"}
errors:
  SyntaxError: "list of FMT diagnostics in the C3 shape, each with a located span and a deterministic autofix"
  DecodeError: "byte offset + reason; never a partial Module"
  AnchorNotFound: "the unresolved path"
guarantees:
  - "print_text(parse_text(b)) == b for canonical b; absorb(view(m)) == m byte for byte (FR5.1)"
  - "text <-> binary is a bijection (FR3)"
  - "no clock, no randomness except fresh_defid, no hash-map order in any output (NFR3)"
```

### C2 — Workspace and typed definitions

```yaml
contract: C2-workspace-typing
provider: u4-checker-and-lints
kind: rust-interface
version: 0.1
types:
  Workspace: {root: Path, modules: "QName -> Module", effect_bounds: "PackageName -> EffectRow (default: no dependencies)"}
  TypedDefinition: {defhash: DefHash, type: Type, effect_row: EffectRow, resolved_traits: "list"}
  EffectRow: "set of the 12 labels + at most one row variable"
  PublicApi: {module: QName, exports: "name -> signature + effect row + type definition"}
operations:
  set_module: {in: Module, out: "invalidated DefIds"}
  remove_module: {in: QName, out: "invalidated DefIds"}
  set_effect_bounds: {in: "PackageName -> EffectRow", note: "supplied by U10; the checker never calls the package system (ADR-010)"}
  check_definition: {in: DefId, out: "TypedDefinition + diagnostics (C3)"}
  check_module: {in: QName, out: "TypedDefinition list + diagnostics (C3)"}
  public_api: {in: QName, out: PublicApi, errors: [ModuleHasErrors]}
  import_graph: {out: "acyclic module graph", errors: [ImportCycle]}
guarantees:
  - "memoized by defhash: re-checking after an edit recomputes only edited definitions and dependents (FR7)"
  - "results independent of evaluation order and thread count"
```

### C3 — Diagnostic stream

```yaml
contract: C3-diagnostics
provider: u4-checker-and-lints
kind: shared-schema (JSON Schema, draft 2020-12) + rust-interface
version: 0.1
schema:
  type: object
  required: [rule, level, loc, message]
  properties:
    rule: {type: string, pattern: "^[A-Z]{2,4}[0-9]{3}$", description: "permanent, never reused (FR11.3)"}
    level: {enum: [E, S, W, O, A]}
    severity: {enum: [low, med, high, crit], description: "only for level S"}
    loc:
      type: object
      required: [module, def, path]
      properties:
        module: {type: string}
        def: {type: string, pattern: "^#[0-9a-hjkmnp-tv-z]{6}$"}
        path: {type: string}
        span: {type: array, items: {type: integer}, minItems: 2, maxItems: 2, description: "byte range in canonical text"}
    message: {type: string}
    remedy: {type: object, description: "a structured patch (E4)"}
    autofix: {type: object, properties: {patch: {type: object}, deterministic: {type: boolean}, semantics_preserving: {type: boolean}}}
    confidence: {type: number, minimum: 0, maximum: 1, description: "1.0 for sound rules; heuristic rules never block"}
    impact: {type: object, properties: {axis: {enum: [performance, security, correctness, maintainability]}, estimate: {type: string}}}
ordering: "stream sorted by (module, def, path, span, rule) so identical input gives identical output"
rust_interface:
  register_rule: "Rule {id, family, level, autofix_kind, analysis: tree|lowered} — U13 adds deep rules through this"
  run_check: "Workspace -> ordered diagnostic stream + blocking verdict per mode (dev, release, --strict)"
  overrides: "allow items applied per §6.5; expired overrides become findings again"
```

### C4 — ToolServices operation set

```yaml
contract: C4-tool-services
provider: u5-agent-loop
kind: rust-interface
version: 0.1
shape:
  Operation: {name: "dotted RAP method name, e.g. patch.apply", params: "JSON Schema", result: "JSON Schema", cancellable: bool}
  register_operation: "later units (U7, U10, U12, U13, U14, U15) add operations; each gets a CLI command and a RAP method from the same definition"
  Context: {workspace: "C2 handle", cancel_token: CancelToken, deadline: "optional instant"}
rules:
  - "an operation is orchestration only; domain logic lives in its owning unit (ADR-003)"
  - "an operation either commits its workspace changes completely or not at all; cancellation and deadline expiry roll back (Q5)"
  - "one failing operation returns a structured error and never poisons the next request (FR13.1)"
  - "results use one JSON schema for CLI --json and RAP (ADR-007)"
errors:
  ToolError: {code: "stable string, e.g. patch.base_mismatch", message: string, data: "operation-specific object"}
```

### C5 — RIR and codegen backends

```yaml
contract: C5-rir-backends
provider: u6-execution-core
kind: rust-interface
version: 0.1
types:
  RirModule: {functions: "IrFunction list in definition-ID order", layouts: "unboxed layouts", constants: "compile-time val data"}
  IrFunction: {instance_key: "defhash + type arguments", blocks: "SSA CFG", ops: "incl. inc, dec, reuse, drop, tail markers"}
  serialization: "deterministic binary encoding for the artifact cache, versioned"
interfaces:
  Backend:
    compile: {in: "RirModule, target (x86-64|aarch64|wasm32), mode (dev|release|wasm)", out: "object bytes", errors: [BackendError]}
    guarantees: ["guaranteed tail calls for all tail-marked calls", "deterministic output for identical input"]
  implementers: {dev: "U6 CraneliftBackend", release: "U12 LlvmBackend", wasm: "U15 WasmBackend"}
  lowered_view: "read-only Mono IR / RIR access for deep lints (U13) and the verifier (U13)"
```

### C6 — Runtime ABI

```yaml
contract: C6-runtime-abi
provider: u6-execution-core
kind: binary-layout
version: 0.1
object_header: "8 bytes: u32 refcount (sticky-overflow bit), u16 constructor tag, u8 field-scan info, u8 flags (shared, static, arena) (§4.4, R8)"
rc_entry_points: ["rail_rt_inc", "rail_rt_dec", "rail_rt_reuse", "rail_rt_drop", "rail_rt_mark_shared"]
trap_entry: "rail_rt_trap(kind, defid, anchor) — kinds: abort, overflow, bounds, div_zero, stack_limit"
evidence: "service-effect evidence (clk, rnd, log, par) in the first argument registers (R7)"
entry: "C-ABI main shim builds Caps from the run manifest (C7) and calls rail main(caps)"
linking: "static library; depends on nothing beyond the platform C library (NFR7.1)"
calling_convention: "internal register convention with guaranteed tail calls; C ABI only at main and ffi (§4.5)"
```

### C7 — Capabilities and run manifest

```yaml
contract: C7-capabilities
provider: u7-system-and-stdlib
kind: runtime rust-interface + canonical-text file
version: 0.1
manifest_file:
  format: "canonical text, one grant per line, sorted; carries a format version line"
  grants:
    fs: "(fs ro|rw <root>)"
    net: "(net <host> <port>)"
    proc: "(proc <absolute executable path>)"
    env: "(env <VARIABLE>)"
    clk: "(clk)"
    rnd: "(rnd seed <n>|os)"
  limits: "(limit fuel|memory|wall|handles|tasks|output <value>) — enforced when U15 ships"
  default: "empty manifest grants nothing (FR35)"
runtime_interface:
  build_caps: {in: RunManifest, out: Caps, note: "the only way to create capabilities"}
  scope: "narrowing only; never widens (R14)"
  os_ops: "the std.sys operations; each checks its capability and returns a typed IoErr or Unsupported"
consumers_note: "U8 (scheduler, proc/env/time/rand), U9 (net), U15 (sandbox host enforces the same grants, ADR-011)"
```

### C8 — Crypto primitives

```yaml
contract: C8-crypto-primitives
provider: u9-crypto-and-net
kind: rust-interface
version: 0.1
operations:
  blake3: "bytes -> 32 bytes"
  sha256: "bytes -> 32 bytes"
  hmac_sha256: "key, bytes -> 32 bytes"
  ed25519_sign: "secret key, message -> signature"
  ed25519_verify: "public key, message, signature -> bool"
  tls: "sans-I/O TLS 1.3 client and server state machines (consumed by std.net only)"
guarantees:
  - "published test vectors pass for every primitive (FR32.1)"
  - "constant-time on secret-dependent paths (FR32.3)"
  - "secret key material zeroed on drop"
note: "until U9 lands, U3 needs BLAKE3 for defhash: U3 may use an approved crate or a minimal in-house BLAKE3 that U9 later replaces behind this same interface"
```

### C9 — Acceptance suite registration

```yaml
contract: C9-suite-registration
provider: u2-acceptance-harness
kind: rust-interface + recorded config file
version: 0.1
suite_config_file:
  path: "tests/acceptance/suites.toml or canonical text equivalent (chosen in U2 design)"
  fields: [suite_id (T1..T14), ci_volume, full_volume, seed_policy, platforms, inputs]
  rule: "lowering ci_volume is lowering a pass criterion (firm rule A)"
suite_interface:
  generate_inputs: "seed -> input set (logged seed)"
  run: "input -> observed outcome via CLI --json or RAP only (black box, ADR-006)"
  judge: "outcomes -> pass/fail per the spec's pass criterion"
```

### C10 — Benchmark corpus layout

```yaml
contract: C10-corpus-layout
provider: u16-benchmark-corpus
kind: shared directory layout
version: 0.1
layout: "corpus/<class>/<program>/{rail,rust,go,ocaml}/ + inputs/ + expected/"
classes: [pure-transform, parsing-text, io-services, data-parallel]
per_program: [prompt.md, fairness-review.md, inputs (seeded generator or fixed files), expected outputs]
rule: "rAiL versions are canonical text written from the spec; a version that later proves invalid is a defect fixed with a reproducing test"
```

### C11 — Registry protocol (HTTP)

```yaml
openapi: 3.1.0
info: {title: rAiL local registry, version: 0.1.0}
servers: [{url: "http://127.0.0.1:{port}"}]
paths:
  /v0/index/{name}:
    get: {summary: "all releases of a package, sorted by version", responses: {"200": {description: "canonical-text index: name, version, archive BLAKE3, publisher fingerprint, log index, yanked"}, "404": {description: unknown package}}}
  /v0/archive/{blake3}:
    get: {summary: "canonical binary package archive by content hash", responses: {"200": {description: archive bytes}, "404": {description: unknown hash}}}
  /v0/log/head:
    get: {summary: "signed transparency-log tree head", responses: {"200": {description: "tree size, root hash, signature"}}}
  /v0/log/proof/{index}:
    get: {summary: "inclusion proof for a log entry against the current head", responses: {"200": {description: "audit path"}, "404": {description: unknown index}}}
  /v0/advisories:
    get: {summary: "advisory database in its documented canonical-text format (E5)", responses: {"200": {description: advisory file}}}
  /v0/publish:
    post: {summary: "publish a signed archive", requestBody: {description: "archive + publisher signature"}, responses: {"201": {description: "log index + countersignature"}, "400": {description: "invalid archive or signature"}, "409": {description: "version already exists (versions are immutable)"}}}
x-rules:
  - "TLS supported, not required on localhost; clients verify signatures, hashes and inclusion proofs regardless of transport (Q4)"
  - "every response body is deterministic for the same registry state"
  - "errors: JSON {code, message}; clients retry only 5xx and connection failures, with a bounded retry count"
```

## External contract specs

### E1 — rAiL Agent Protocol (RAP)

```yaml
protocol: RAP
transport: "JSON-RPC 2.0 over stdin/stdout; each message preceded by 'Content-Length: <n>\r\n\r\n' (Q2)"
version_negotiation:
  initialize: {params: {client_versions: "list of supported RAP versions", workspace_root: Path}, result: {rap_version: "chosen version", capabilities: "list of methods and optional features"}}
methods: "one per §9.1 row: tree.get, fmt.apply, check.run, test.run, bench.run, debug.*, prof.run, deps.graph, audit.run, equiv.run, diff.run, patch.apply, query.run, verify.run, plus build.run and run.run; each method's params and result are published as JSON Schema generated from its C4 Operation"
notifications:
  $/cancelRequest: {params: {id: "request id"}, effect: "request ends with error code -32800 'cancelled'; workspace unchanged (Q5)"}
  $/progress: {params: {id, message, fraction?}, note: "optional progress for long operations"}
deadlines: "any request may carry params._deadline_ms; on expiry the request ends with error code -32801 'deadline exceeded'; workspace unchanged (Q5)"
errors:
  standard: "JSON-RPC codes -32700..-32603"
  rail: "-32000 tool error with data {code: stable ToolError code, details}; -32800 cancelled; -32801 deadline exceeded"
rules:
  - "stdout carries only protocol messages (FR13.2); logs go to stderr"
  - "a malformed or failing request never ends the server (FR13.1)"
  - "addresses are module#defid/anchor-path; line/column accepted only as fallback and converted (FR14)"
  - "patch.apply fails fast on a stale base hash and returns the intervening patch (FR15.1)"
```

### E2 — Command line

```yaml
interface: rail CLI
output: "human-readable text by default; --json prints exactly the RAP result for the same operation (ADR-007)"
exit_codes: {0: success, 1: "command ran and found blocking problems (e.g. check errors, failing tests)", 2: "usage error", 3: "tool failure (I/O, internal)"}
stdout_stderr: "results on stdout; progress and logs on stderr"
commands: "one per C4 Operation (parse, fmt, view, absorb, check, patch, merge, test, bench, prof, debug, deps, audit, equiv, diff, query, verify, build, run, publish, vendor, registry serve)"
version: "rail --version prints toolchain version and supported RAP and file-format versions"
```

### E3 — Source file formats

```yaml
formats:
  .rlc: "canonical text, grammar §2.3, formatting §2.5; format version = language edition in the mod header area (settled in U3 design)"
  .rlb: "binary per §2.5 / R9: magic RAIL, u16 format version, u32 section count, fixed section order"
  .rlh: "human form per §2.4 and R11; editable only when formatter-canonical; optional --#id markers"
  .rlmap: "canonical text lines (map start end #defid path)"
versioning: "readers reject unknown major versions with a located error; additive sections/fields are ignored by older readers from 1.0 (Q3)"
```

### E4 — Patch and conflict format

```yaml
format: "canonical text per §9.4"
header: "(patch <module> base:blake3:<modhash> ...)"
operations: [set, ins, add, del, ren, mov, meta]
atomicity: "all operations apply or none; failures reported by operation index"
conflicts: "(conflict #id/path ours theirs base) items, themselves valid patches (§9.5)"
version: "patch format version carried by the RAP version; stand-alone patch files carry a (patch-format N) line"
```

### E5 — Package and supply-chain formats

```yaml
rail.pkg: "canonical text per §5.3: (pkg name version) (edition N) (dep name min (fx ...)) (dev ...) (ffi name file version blake3:...)"
rail.lock: "canonical text, sorted; per package: exact version, source, archive BLAKE3, publisher fingerprint, resolved effect bound (§5.4)"
rail.review: "canonical text; signed approval records for S-crit overrides and reviewed modhashes (§6.5, FR19.3)"
package_archive: "canonical binary archive; content-addressed by BLAKE3"
provenance_record: "canonical text: source modhash set, lockfile hash, compiler hash, target, mode, policy digest; Ed25519 signature added by Trust (§7.11, ADR-009)"
advisory_database: "canonical text; one (advisory id package affected-versions severity summary) item per advisory, sorted by id"
versioning: "each file carries a format version item (Q3)"
```

### E6 — Run manifest and audit log

```yaml
run_manifest: "see C7 manifest_file; owned by U7"
audit_log:
  owner: u15-wasm-sandbox (writer lives in SystemInterface)
  format: "JSON lines, one event per line, hash-chained: {seq, prev_hash, time_ns, event: grant|use|trap|limit, defid, operation, args_redacted}"
  rule: "secrets always redacted; append-only; chain verifiable offline (§7.9)"
```

### E7 — Compiled program interface

```yaml
exit_codes: {0: "main returned Ok", 1: "main returned Err (error printed to stderr in structured form)", 70: "unhandled trap, structured trace on stderr (§7.3)"}
streams: "stdout and stderr are bytes; no newline translation (§4.9)"
abi: "C ABI only at main and ffi-module ext functions (§4.5)"
linking: "statically linked except the platform C library (macOS) (§4.3)"
```

### E8 — Debugger

```yaml
protocol: "Debug Adapter Protocol over stdio (Should — may slip)"
breakpoints: "source references use module#defid/anchor-path; human-form spans are mapped through source maps"
values: "printed in human form"
```

## Contract ownership rules

1. **Each contract has one owner**, shown in the table above. Only the owner changes it, and every change updates this summary in the same pull request. [Q1]
2. **Later units add; they do not change.** Units that add operations (C4), rules (C3), backends (C5) or suites (C9) use the extension points the owner defines.
3. **Breaking changes are agreed first.** A breaking change to an internal contract is recorded as a decision (ADR) and made together with every consumer's update in the same change. A breaking change to an external contract before 1.0 needs a version bump and a spec update. From 1.0 it needs a new major version. [Q3]
4. **Additive changes are safe.** New optional fields, methods, rules and sections are additive. Readers and clients ignore unknown fields. RAP clients learn the available methods from `initialize`.
5. **Determinism is part of every contract.** Identical inputs give identical bytes on every contract surface: diagnostics order, RIR, artifacts, registry responses and audit logs (NFR3).
6. **Errors are always typed values.** Nothing panics across a boundary. Every boundary returns typed errors, and every external surface returns structured errors with stable codes.

## Assumptions & Open Questions

- [assumption] The RAP error codes `-32800` and `-32801` are unused by JSON-RPC 2.0 and by common clients. U5 design confirms them.
- [assumption] Localhost-only HTTP without TLS is acceptable for the registry, because integrity never depends on the transport. [Q4]

| Contract | Question | Blocks |
|---|---|---|
| C1 / C8 | Before U9 exists, does U3 use an approved BLAKE3 crate or a minimal in-house BLAKE3, to be replaced behind C8 later? | U3 (syntax) |
| C9 | Is the suite config file TOML or canonical text? TOML needs an approved crate. | U2 (acceptance-harness) |
| E3 | Where does the format version of canonical text live, given that the grammar has no edition form in modules? | U3 (syntax) |
| C6 | What exact register assignment carries the evidence parameters on x86-64 and AArch64? | U6 (execution-core) |
| E1 | Does the `run.run` method stream program output as progress notifications, or only return it at the end? | U5 (agent-loop), U7 (system-and-stdlib) |
