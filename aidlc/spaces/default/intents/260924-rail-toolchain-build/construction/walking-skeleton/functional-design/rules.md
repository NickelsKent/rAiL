# Business Rules — walking-skeleton (U1)

The rules the walking skeleton enforces. One principle runs through them: **the skeleton never accepts a program it cannot fully check.** Anything outside its small language subset is rejected with a located diagnostic instead of being passed through unchecked. Later units widen what is accepted; they never have to tighten something the skeleton let through.

Sources: FR27 (the walking skeleton), the rAiL spec (§2.2–§2.7, §3.1, §3.3, §6.1–§6.4, §7.3, §9.1), contracts C1–C4, E1, E2, E7 (`contract-summary.md`), the cross-cutting requirements the unit must respect (FR13.1, FR13.2, NFR3, NFR6, NFR7.1), the firm rules in `team-practices.md`, and `functional-design-questions.md` ([Q1]–[Q7]).

```yaml
rules:
  # BR1 — Reading canonical text (Syntax)
  - id: BR1.1
    statement: The skeleton accepts only the forms its program needs; every other form is rejected.
    category: validation
    applies_to: SourceModule
    trigger: parse
    logic: IF an item or expression uses a form outside {mod, pub, fn, integer literal, true, false, (), reference, application, let, Boolean match} THEN reject it.
    violation: SKL001 at the form's span, message names the form and says it is not yet supported.
    source: [FR27, Q2]
  - id: BR1.2
    statement: Input must already be in exact canonical layout; the skeleton does not reformat.
    category: validation
    applies_to: SourceModule
    trigger: parse
    logic: IF the bytes are not UTF-8 NFC, use anything but LF, lack a final LF, carry a BOM, use other than single spaces between tokens, put more than one item on a line, or order items other than mod, pub, then fn sorted by name THEN reject.
    violation: SKL001 at the first offending byte; the autofix arrives with the full formatter (U3).
    source: [FR27, spec §2.5, Q2]
  - id: BR1.3
    statement: Printing an accepted module reproduces its input exactly.
    category: constraint
    applies_to: SourceModule
    trigger: every successful parse
    logic: IF a module parses THEN print(parse(bytes)) equals bytes, byte for byte.
    violation: internal failure (ToolError), never a silent difference; covered by round-trip tests.
    source: [FR27, spec §2.10, NFR3]
  - id: BR1.4
    statement: A truncated or malformed input fails at a precise location.
    category: validation
    applies_to: SourceModule
    trigger: parse
    logic: IF parsing cannot continue (unbalanced parentheses, bad token, bad ID) THEN stop and report the byte where it failed.
    violation: SKL001 with the failing span; no partial module is returned.
    source: [FR27, spec §1.5]
  - id: BR1.5
    statement: Definition IDs are supplied by the author, well formed and unique.
    category: validation
    applies_to: Definition
    trigger: parse
    logic: IF a fn item has no ID, an ID that is not '#' plus 6 lowercase Crockford base32 characters, or an ID used twice in the module THEN reject.
    violation: SKL001 at the ID's span. The skeleton never assigns IDs.
    source: [FR27, spec §2.6]
  - id: BR1.6
    statement: The name 'skel' is reserved for the skeleton's temporary built-in.
    category: constraint
    applies_to: SourceModule, Expr
    trigger: parse and name resolution
    logic: IF the first dotted segment of a module's qname equals exactly 'skel' (so 'skel' and 'skel.x' are rejected, while 'skeleton.answer' is accepted), or a reference names skel.<anything> other than skel.print_i64 THEN reject.
    violation: SKL001 at the name's span.
    source: [Q1, Q6]
  - id: BR1.7
    statement: Names inside a definition are never shadowed and every binding is used.
    category: validation
    applies_to: Definition
    trigger: name resolution
    logic: IF a let or parameter binding reuses a name already bound in the definition, or a named binding is never used (instead of '_') THEN reject. A reference to an unknown name is also rejected.
    violation: SKL001 at the binding's span (the full rules arrive as FMT and UNU001 with U3/U4).
    source: [FR27, spec §2.7 rule 3]

  # BR2 — Type checking (TypeChecker)
  - id: BR2.1
    statement: Every function carries an explicit signature and is therefore exported.
    category: validation
    applies_to: Definition
    trigger: check
    logic: IF a fn has no signature THEN reject (the skeleton has no inference). Because canonical text allows a signature only on exported functions, IF a fn with a signature is missing from the pub item THEN reject.
    violation: SKL001 at the fn's name.
    source: [FR27, Q3, spec §2.2]
  - id: BR2.2
    statement: Only the skeleton's types exist.
    category: validation
    applies_to: FunctionType
    trigger: check
    logic: IF a signature mentions a type other than i64, bool, unit, Caps, or Result with ok type unit and err type i64, bool or unit THEN reject.
    violation: SKL001 at the type's span.
    source: [FR27, Q3]
  - id: BR2.3
    statement: Every expression has the type its context requires.
    category: validation
    applies_to: Expr
    trigger: check
    logic: IF an expression's type differs from the type required by its position (argument, let body, match arm, function body, match scrutinee) THEN reject; both match arms must have the same type.
    violation: TY001 at the anchor path and span of the smallest mismatching node, message names expected and found types.
    source: [FR27, Q3, spec §6.2]
  - id: BR2.4
    statement: Integer literals are i64.
    category: validation
    applies_to: Expr
    trigger: parse
    logic: IF an integer literal carries a type suffix, or its value is outside the i64 range THEN reject.
    violation: SKL001 at the literal's span.
    source: [FR27, Q3, spec §2.3]
  - id: BR2.5
    statement: Operators have fixed skeleton types.
    category: calculation
    applies_to: Expr
    trigger: check
    logic: IF the operator is + - * / % THEN both arguments are i64 and the result is i64. IF it is < <= > >= THEN both are i64 and the result is bool. IF it is == or != THEN both arguments have the same type, i64 or bool, and the result is bool.
    violation: TY001 on the mismatching argument.
    source: [FR27, Q2, Q3]
  - id: BR2.6
    statement: The only conditional is the Boolean match with arms true then false.
    category: validation
    applies_to: Expr
    trigger: parse and check
    logic: IF a match has other than exactly two arms (true a) and (false b), in that order, THEN reject (SKL001). IF its scrutinee is not bool THEN TY001.
    violation: SKL001 or TY001 as stated.
    source: [FR27, Q2, spec §2.2, §2.7 rule 1]
  - id: BR2.7
    statement: A function named main has the fixed entry-point shape, and Caps and Result appear nowhere else.
    category: validation
    applies_to: Definition
    trigger: check
    logic: IF a module defines a function named main whose type is not (-> (Caps) (Result unit E) [log]) with E in i64, bool, unit THEN reject. IF Caps appears anywhere other than as main's single parameter type, or Result anywhere other than as main's result type THEN reject.
    violation: TY001 at the offending type's span in the signature.
    source: [FR27, Q1, Q3, spec §7.5, R14]
  - id: BR2.8
    statement: A module can be built only if it has an entry point.
    category: validation
    applies_to: SourceModule
    trigger: build and run
    logic: IF a module that checked cleanly has no exported function named main THEN the build is refused. (A module without main is still a valid module for parse and check.)
    violation: ToolError build.no_entry; no artifact is produced.
    source: [FR27, Q1]

  # BR3 — Effects (TypeChecker)
  - id: BR3.1
    statement: The temporary print built-in has a fixed signature and carries the log effect.
    category: constraint
    applies_to: SkelPrintBuiltin
    trigger: check
    logic: skel.print_i64 has type (-> (i64) unit log) and is available in every module without an import.
    violation: calling it with a non-i64 argument is TY001.
    source: [Q1, Q6]
  - id: BR3.2
    statement: A function must declare the log effect when its body can print.
    category: validation
    applies_to: Definition
    trigger: check
    logic: IF a function's body calls skel.print_i64, or calls a function whose signature declares log, and its own signature does not declare log THEN reject at that call.
    violation: FX001 at the call's anchor path and span (no autofix in the skeleton).
    source: [Q6, spec §3.3, §6.2]
  - id: BR3.3
    statement: log is the only effect label in the skeleton.
    category: validation
    applies_to: FunctionType
    trigger: check
    logic: IF a signature lists any effect label other than log, or an effect row variable THEN reject.
    violation: SKL001 at the label's span.
    source: [Q3, Q6]

  # BR4 — Diagnostics (LintEngine)
  - id: BR4.1
    statement: Diagnostics use the real diagnostic shape and only three rule IDs.
    category: constraint
    applies_to: Diagnostic
    trigger: every reported problem
    logic: Every diagnostic has rule, level, loc (module, def, path, span), message and confidence as in contract C3. The skeleton emits only TY001, FX001 and SKL001. A problem outside any definition is located on the module's first definition, or '#000000' when there is none.
    violation: schema tests fail; not a runtime condition.
    source: [FR27, Q2, Q3, spec §6.4]
  - id: BR4.2
    statement: Every skeleton diagnostic is an error, and any error blocks building and running.
    category: policy
    applies_to: CheckResult, BuildArtifact
    trigger: check, build, run
    logic: Every diagnostic has level E. IF a module's CheckResult has any diagnostic THEN it is blocking, and build and run refuse it.
    violation: build and run return ToolError build.blocked carrying the CheckResult; no artifact is produced or run.
    source: [FR27, Q5, spec §1.1, §6.1]
  - id: BR4.3
    statement: Check output is deterministic.
    category: constraint
    applies_to: CheckResult
    trigger: check
    logic: Diagnostics are sorted by (module, def, path, span, rule); identical input gives identical bytes on every run, directory and platform.
    violation: determinism tests fail.
    source: [NFR3, NFR6, spec §6.4]
  - id: BR4.4
    statement: The SKL rule family is temporary and its IDs are never reused.
    category: policy
    applies_to: Diagnostic
    trigger: rule registration
    logic: SKL001 means 'outside what this toolchain build can check yet'. It is registered once; when U3 and U4 cover the language it is retired and its ID is never assigned to another rule.
    violation: registry test fails if SKL001 is redefined or reused.
    source: [Q2, FR11.3]

  # BR5 — Building and running (BuildDriver, Lowering, CraneliftBackend, RuntimeCore)
  - id: BR5.1
    statement: The skeleton builds only dev-mode native binaries for the host.
    category: constraint
    applies_to: BuildArtifact
    trigger: build
    logic: The target is the host platform (x86-64 Linux or AArch64 macOS) and the mode is dev, using Cranelift.
    violation: any other target or mode request is ToolError rap.unsupported_param (protocol) or a usage error (command line).
    source: [FR27]
  - id: BR5.2
    statement: Build and run always check first.
    category: policy
    applies_to: BuildArtifact
    trigger: build and run
    logic: A build runs the full check; only a non-blocking CheckResult proceeds to lowering, code generation and linking. run builds before it runs.
    violation: see BR4.2.
    source: [FR27, spec §1.1]
  - id: BR5.3
    statement: The program's exit code reflects main's result.
    category: calculation
    applies_to: RunResult
    trigger: program exit
    logic: IF main returns Ok THEN exit code 0. IF it returns Err THEN exit code 1. IF an arithmetic overflow or a division or remainder by zero occurs THEN the program traps and exits with code 70 and a one-line trap description on standard error.
    violation: n/a (behaviour definition).
    source: [FR27, spec §3.1, §7.3, E7]
  - id: BR5.4
    statement: Arithmetic is checked.
    category: calculation
    applies_to: IrFunction
    trigger: execution
    logic: + - * trap on overflow; / and % trap on a zero divisor and on the one overflowing case (minimum i64 divided by -1).
    violation: trap, exit code 70 (BR5.3).
    source: [spec §3.1, NFR4]
  - id: BR5.5
    statement: Builds are deterministic and contain no machine details.
    category: constraint
    applies_to: BuildArtifact
    trigger: build
    logic: The same module, platform and toolchain give a byte-identical executable; no timestamps, absolute paths, hostnames or random seeds are written into it; its location is reported as a workspace-relative path.
    violation: determinism test fails; treated as a defect.
    source: [NFR3, firm rule 'no timestamps, absolute paths, hostnames or random seeds in compiler output']
  - id: BR5.6
    statement: The runtime depends only on the platform C library.
    category: constraint
    applies_to: BuildArtifact
    trigger: link
    logic: The skeleton runtime (entry, print, traps, exit) uses nothing beyond the platform C library; the linked program shows only operating-system libraries.
    violation: dependency check fails in CI.
    source: [NFR7.1]
  - id: BR5.7
    statement: skel.print_i64 writes one line to standard output.
    category: calculation
    applies_to: SkelPrintBuiltin
    trigger: call
    logic: Write the argument in decimal (a leading '-' for negatives, no leading zeros, no '+') followed by one LF; all output is written out before the program exits, whatever the exit code.
    violation: n/a (behaviour definition).
    source: [Q1, Q6]

  # BR6 — Tool surface (ToolServices, Cli, RapServer)
  - id: BR6.1
    statement: Command line and protocol are two faces of the same four operations.
    category: constraint
    applies_to: ToolServices
    trigger: any request
    logic: The operations are parse (rail parse, tree.get), check (rail check, check.run), build (rail build, build.run) and run (rail run, run.run). Each takes a module path; --json on the command line prints exactly the protocol result.
    violation: parity tests fail.
    source: [FR27, Q4, spec §9.1, ADR-007]
  - id: BR6.2
    statement: Command-line exit codes follow the agreed contract.
    category: policy
    applies_to: Cli
    trigger: command exit
    logic: parse, check, build: 0 success, 1 blocking diagnostics, 2 usage error, 3 tool failure. run: when the program ran, the program's own exit code; 1 when blocked by diagnostics; 2 usage error; 3 tool failure. With --json the result distinguishes a blocked run from a program that returned Err.
    violation: n/a (behaviour definition).
    source: [E2, E7]
  - id: BR6.3
    statement: The protocol session starts with initialize.
    category: validation
    applies_to: RapSession
    trigger: every request
    logic: IF a method other than initialize arrives before a successful initialize THEN return ToolError rap.not_initialized. initialize accepts client_versions containing "0.1" and a workspace_root, and returns rap_version "0.1" plus the methods it serves. A second initialize after a successful one is an invalid request (-32600) and does not reset the session.
    violation: JSON-RPC error -32000 with data {code: rap.not_initialized}; the session stays open.
    source: [E1, Q4]
  - id: BR6.4
    statement: One bad request never ends the server.
    category: policy
    applies_to: RapSession
    trigger: every message
    logic: Malformed framing or JSON gives -32700; an invalid request gives -32600; an unknown method gives -32601; bad params give -32602; a tool failure gives -32000 with a ToolError. In every case the server answers and keeps serving. Unknown or unsupported params (depth, focus, _deadline_ms, a target other than the host) are rejected with rap.unsupported_param rather than ignored.
    violation: n/a (behaviour definition); covered by malformed-request tests.
    source: [FR13.1, E1]
  - id: BR6.5
    statement: Standard output carries only protocol messages while the server runs.
    category: constraint
    applies_to: RapSession
    trigger: always
    logic: Logs go to standard error. run.run runs the program as a separate process and captures its output, so program output never reaches the protocol stream.
    violation: tests fail if any non-protocol byte appears on the server's standard output.
    source: [FR13.2]
  - id: BR6.6
    statement: run.run returns program output at the end.
    category: calculation
    applies_to: RunResult
    trigger: run.run
    logic: The result is {exit_code, stdout, stderr} after the program ends; nothing is streamed.
    violation: n/a (behaviour definition).
    source: [Q7]
```

## Rules summary

| ID | Rule | Category | Violation |
|---|---|---|---|
| BR1.1 | Only the skeleton's forms are accepted | validation | SKL001 |
| BR1.2 | Input must be in exact canonical layout | validation | SKL001 |
| BR1.3 | Printing reproduces the input exactly | constraint | internal failure |
| BR1.4 | Malformed input fails at a precise location | validation | SKL001 |
| BR1.5 | Author-supplied, well-formed, unique IDs | validation | SKL001 |
| BR1.6 | `skel` is reserved for the temporary built-in | constraint | SKL001 |
| BR1.7 | No shadowing, no unused named bindings, no unknown names | validation | SKL001 |
| BR2.1 | Every function has a signature and is exported | validation | SKL001 |
| BR2.2 | Only `i64 bool unit Caps Result` | validation | SKL001 |
| BR2.3 | Expressions have the required type | validation | TY001 |
| BR2.4 | Integer literals are unsuffixed `i64` | validation | SKL001 |
| BR2.5 | Fixed operator types | calculation | TY001 |
| BR2.6 | Boolean match only, arms `true` then `false` | validation | SKL001 / TY001 |
| BR2.7 | `main` has the fixed shape; `Caps` and `Result` only there | validation | TY001 |
| BR2.8 | Building needs an exported `main` | validation | build.no_entry |
| BR3.1 | `skel.print_i64 : (-> (i64) unit log)` | constraint | TY001 |
| BR3.2 | Callers of printing code declare `log` | validation | FX001 |
| BR3.3 | `log` is the only effect label | validation | SKL001 |
| BR4.1 | Real diagnostic shape; only TY001, FX001, SKL001 | constraint | schema tests |
| BR4.2 | All diagnostics are errors and block build and run | policy | build.blocked |
| BR4.3 | Sorted, deterministic diagnostics | constraint | determinism tests |
| BR4.4 | SKL001 is temporary and never reused | policy | registry test |
| BR5.1 | Dev-mode host builds only | constraint | unsupported param / usage error |
| BR5.2 | Build and run always check first | policy | build.blocked |
| BR5.3 | Exit 0 on Ok, 1 on Err, 70 on trap | calculation | — |
| BR5.4 | Checked arithmetic traps | calculation | exit 70 |
| BR5.5 | Byte-identical builds, no machine details | constraint | determinism test |
| BR5.6 | Runtime uses only the platform C library | constraint | CI dependency check |
| BR5.7 | Print writes one decimal line, flushed before exit | calculation | — |
| BR6.1 | Command line and protocol share four operations | constraint | parity tests |
| BR6.2 | Agreed command-line exit codes | policy | — |
| BR6.3 | `initialize` first | validation | rap.not_initialized |
| BR6.4 | A bad request never ends the server | policy | JSON-RPC error |
| BR6.5 | Protocol-only standard output | constraint | stream tests |
| BR6.6 | `run.run` returns output at the end | calculation | — |
