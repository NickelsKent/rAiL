# Entities — walking-skeleton (U1)

The entity model for the walking skeleton: the smallest working slice that parses, checks, serves over the agent protocol, builds and runs one canonical rAiL module. Every entity here is a **thin first version** of an entity owned by a later unit (see `components.md`, Entity Ownership). The skeleton keeps each entity's identity and core shape as agreed in Contract Design (C1–C4, E1, E2, E7), so later units extend these entities instead of replacing them. Fields the full design has but the skeleton does not fill are listed under `deferred`, so their absence is deliberate and visible.

Sources: `unit-of-work.md` (U1), `components.md`, `contract-summary.md`, `requirements.md` (FR27 and the cross-cutting NFRs), the rAiL spec (§2.2–§2.6, §3.1, §6.1–§6.4, §7.3, §9.1), and `functional-design-questions.md` ([Q1]–[Q7]).

```yaml
entities:
  - name: SourceModule
    description: One canonical-text (.rlc) module the skeleton accepts. Thin version of Module (Syntax, contract C1).
    owner_component: Syntax
    identifier: qname
    attributes:
      - {name: qname, type: text, required: true, unique: true, constraints: "dotted lowercase path; equals the file path relative to the workspace root with '/' replaced by '.'; first segment must not be 'skel' (BR1.6)"}
      - {name: source_bytes, type: bytes, required: true, constraints: "exact canonical layout (BR1.2)"}
      - {name: exports, type: list of name, required: true, constraints: "the single (pub ...) item; sorted; every function in the module is listed (BR2.1)"}
      - {name: definitions, type: list of Definition, required: true, min: 1, constraints: "functions only in the skeleton; sorted by name (BR1.2)"}
    deferred: [modhash, use items, typ/trt/imp/val/tst/allow/meta items, source map]
    relationships:
      - {to: Definition, cardinality: "1:N", direction: "module contains definitions"}

  - name: Definition
    description: One fn item. Thin version of Definition (Syntax, C1).
    owner_component: Syntax
    identifier: defid
    attributes:
      - {name: defid, type: text, required: true, unique: "within module", constraints: "'#' + 6 lowercase Crockford base32 characters; written by the author, never assigned by the skeleton (BR1.5)"}
      - {name: name, type: text, required: true, unique: "within module", constraints: "snake_case, at most 32 bytes"}
      - {name: kind, type: enum, required: true, allowed: [fn]}
      - {name: params, type: list of Pattern, required: true, constraints: "each a lowercase binding name or '_'"}
      - {name: signature, type: FunctionType, required: true, constraints: "explicit on every function (BR2.1)"}
      - {name: body, type: Expr, required: true}
    deferred: [defhash, meta attachments, the other ten item kinds]
    relationships:
      - {to: SourceModule, cardinality: "N:1", direction: "belongs to one module"}
      - {to: TypedDefinition, cardinality: "1:0..1", direction: "checked form, present only when the definition checks cleanly"}

  - name: Expr
    description: One expression node of the canonical tree. Only the forms the skeleton program needs exist (Q2).
    owner_component: Syntax
    identifier: "defid + anchor path"
    attributes:
      - {name: form, type: enum, required: true, allowed: [int_literal, bool_literal, unit_literal, reference, application, let, bool_match]}
      - {name: anchor_path, type: text, required: true, constraints: "address of the node inside its definition (spec §2.6); used in every diagnostic location"}
      - {name: span, type: "byte range", required: true, constraints: "start and end byte offsets in the canonical text"}
      - {name: children, type: list of Expr, required: false}
    constraints:
      - "int_literal has no type suffix and is an i64 (BR2.4)"
      - "reference is a local binding, a function of the same module, an operator (+ - * / % == != < <= > >=), a Result constructor (Ok, Err), or skel.print_i64 (BR1.6)"
      - "application has one or more arguments (spec §2.3)"
      - "let binds one or more patterns in sequence; each pattern is a binding name or '_'"
      - "bool_match has exactly the arms (true a) then (false b) (BR2.6)"
    deferred: [lambda, general match and patterns, field access, record update, propagate, tuple, vector, ascription, float/char/string literals]

  - name: SkeletonType
    description: The types the skeleton checker knows (Q3). Thin version of the type language in TypeChecker.
    owner_component: TypeChecker
    identifier: canonical spelling
    attributes:
      - {name: kind, type: enum, required: true, allowed: [i64, bool, unit, Result, Caps, function]}
      - {name: result_ok, type: SkeletonType, required: "when kind = Result", allowed_values: [unit]}
      - {name: result_err, type: SkeletonType, required: "when kind = Result", allowed_values: [i64, bool, unit]}
    constraints:
      - "Caps is opaque in the skeleton: it has no fields and can only be received by main (BR2.7)"
      - "Result appears only as the return type of main (BR2.7)"

  - name: FunctionType
    description: A function signature, (-> (params) result effects).
    owner_component: TypeChecker
    identifier: canonical spelling
    attributes:
      - {name: param_types, type: list of SkeletonType, required: true}
      - {name: result_type, type: SkeletonType, required: true}
      - {name: effects, type: set of effect label, required: true, allowed: [log], default: "empty", constraints: "only the log label exists in the skeleton (BR3.3)"}

  - name: TypedDefinition
    description: The checked form of a definition. Thin version of TypedDefinition (TypeChecker, C2).
    owner_component: TypeChecker
    identifier: "qname + defid"
    attributes:
      - {name: defid, type: text, required: true, references: Definition.defid}
      - {name: type, type: FunctionType, required: true}
      - {name: effect_row, type: set of effect label, required: true, constraints: "equals the declared effects; the skeleton never infers a wider row (BR3.2)"}
    deferred: [defhash key, resolved_traits, generalisation]

  - name: Diagnostic
    description: One problem found in a module. Uses the real diagnostic shape (C3) so agents and the harness never see a skeleton-only format.
    owner_component: LintEngine
    identifier: "(module, def, path, span, rule)"
    attributes:
      - {name: rule, type: text, required: true, allowed: [TY001, FX001, SKL001], constraints: "pattern ^[A-Z]{2,4}[0-9]{3}$ (BR4.1)"}
      - {name: level, type: enum, required: true, allowed: [E], constraints: "every skeleton diagnostic is an error and blocks builds (BR4.2)"}
      - {name: loc.module, type: text, required: true, references: SourceModule.qname}
      - {name: loc.def, type: text, required: true, references: Definition.defid, constraints: "for a problem outside any definition (the mod or pub line, or layout before the first item), the defid of the first definition in the module; if there is none, '#000000' (BR4.1)"}
      - {name: loc.path, type: text, required: true}
      - {name: loc.span, type: "byte range", required: true}
      - {name: message, type: text, required: true}
      - {name: confidence, type: number, required: true, default: 1.0}
    deferred: [severity, remedy, autofix, impact]
    relationships:
      - {to: SourceModule, cardinality: "N:1", direction: "raised in one module"}
      - {to: Definition, cardinality: "N:1", direction: "points at one definition anchor"}

  - name: CheckResult
    description: The outcome of checking one module.
    owner_component: ToolServices
    identifier: module qname
    attributes:
      - {name: module, type: text, required: true}
      - {name: diagnostics, type: list of Diagnostic, required: true, constraints: "sorted by (module, def, path, span, rule) (BR4.3)"}
      - {name: blocking, type: bool, required: true, constraints: "true exactly when any diagnostic is present (BR4.2)"}

  - name: IrFunction
    description: A lowered function ready for code generation. Thin version of IrFunction (Lowering, C5).
    owner_component: Lowering
    identifier: "qname + defid"
    attributes:
      - {name: defid, type: text, required: true, references: TypedDefinition.defid}
      - {name: blocks, type: "SSA control-flow graph", required: true}
    deferred: [Core IR and Mono IR levels, reference-counting operations, tail markers, serialisation for the cache]

  - name: BuildArtifact
    description: A native executable built from one module. Thin version of BuildArtifact (BuildDriver).
    owner_component: BuildDriver
    identifier: "module qname + target + mode"
    attributes:
      - {name: module, type: text, required: true}
      - {name: target, type: enum, required: true, allowed: [x86_64-linux, aarch64-macos], constraints: "the host platform only (BR5.1)"}
      - {name: mode, type: enum, required: true, allowed: [dev]}
      - {name: path, type: text, required: true, constraints: "workspace-relative, never absolute (BR5.5)"}
    deferred: [content_hash, provenance record, artifact cache entry, release and wasm modes, cross-compilation]

  - name: RunResult
    description: What one run of a built program produced (Q7).
    owner_component: ToolServices
    identifier: none (value)
    attributes:
      - {name: exit_code, type: integer, required: true, allowed: [0, 1, 70]}
      - {name: stdout, type: text, required: true}
      - {name: stderr, type: text, required: true}

  - name: RapSession
    description: One agent connection to the protocol server. Thin version of Session (RapServer).
    owner_component: RapServer
    identifier: one per server process
    attributes:
      - {name: state, type: enum, required: true, allowed: [awaiting_initialize, ready, closed], default: awaiting_initialize}
      - {name: workspace_root, type: path, required: "after initialize"}
      - {name: rap_version, type: text, required: "after initialize", allowed: ["0.1"]}
    deferred: [opened_modules cache, cancellation, deadlines, progress notifications]

  - name: ToolError
    description: A structured failure of a tool operation (C4); never a crash.
    owner_component: ToolServices
    identifier: code
    attributes:
      - {name: code, type: text, required: true, allowed: [module.not_found, module.unreadable, build.blocked, build.no_entry, build.failed, run.failed, rap.not_initialized, rap.unsupported_param]}
      - {name: message, type: text, required: true}
      - {name: data, type: object, required: false, constraints: "for build.blocked: the CheckResult"}

  - name: SkelPrintBuiltin
    description: The temporary print built-in (Q1 B, Q6 A). Not part of the language; deleted when the standard library's output lands (U7).
    owner_component: RuntimeCore (behaviour) and TypeChecker (signature)
    identifier: skel.print_i64
    attributes:
      - {name: signature, type: FunctionType, required: true, constraints: "(-> (i64) unit log)"}
      - {name: output, type: text, constraints: "the argument in decimal, a leading '-' for negatives, followed by one LF, on standard output (BR5.7)"}
```

## Summary

| Entity | What it is in the skeleton | Owner (later unit that completes it) |
|---|---|---|
| SourceModule | One canonical `.rlc` file, functions only | Syntax (U3) |
| Definition | One `fn` with an author-written ID and an explicit signature | Syntax (U3) |
| Expr | Seven expression forms: three literals, reference, application, `let`, Boolean match | Syntax (U3) |
| SkeletonType / FunctionType | `i64`, `bool`, `unit`, `Result` (for `main`), opaque `Caps`, functions with at most the `log` effect | TypeChecker (U4) |
| TypedDefinition | Checked function with its declared effects | TypeChecker (U4) |
| Diagnostic / CheckResult | Real C3 shape, three rule IDs, always level `E` | LintEngine (U4) |
| IrFunction | SSA function for Cranelift | Lowering (U6) |
| BuildArtifact | Dev-mode native binary for the host | BuildDriver (U6) |
| RunResult | Exit code plus captured output | ToolServices (U5/U7) |
| RapSession / ToolError | One stdio session; typed tool failures | RapServer, ToolServices (U5) |
| SkelPrintBuiltin | Temporary `skel.print_i64` with the `log` effect | Removed by U7 |

A **SourceModule** contains one or more **Definitions**. Each Definition has an **Expr** body and a **FunctionType**. Checking turns each clean Definition into a **TypedDefinition** and reports problems as **Diagnostics** in a **CheckResult**. Lowering produces **IrFunctions**, the build produces a **BuildArtifact**, and running it produces a **RunResult**. A **RapSession** drives the same operations as the command line, and failures come back as **ToolErrors**.
