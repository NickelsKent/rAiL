# Entities — benchmark-corpus (U16)

The entity model for the benchmark corpus: 40 programs in four classes, each written in rAiL, Rust, Go and OCaml under the same prompt and reviewed for fairness (spec §11.1, FR54). The corpus is project tooling outside the toolchain's Cargo workspace. Later suites read it as inputs: T3, T4, T7, T9, T10 and T13, plus the §11.3 token measurements. Domain Design names one entity, CorpusProgram (owned by BenchmarkCorpus), and it keeps its identifier and attributes here. The rest are its parts.

Sources: `unit-of-work.md` (U16), `unit-of-work-story-map.md` (FR54 → U16), `requirements.md` (FR54, NFR3, NFR6), `components.md` (BenchmarkCorpus, CorpusProgram), `contract-summary.md` (C10 corpus layout), the rAiL spec §11.1, and `functional-design-questions.md` ([Q1]–[Q9]).

```yaml
entities:
  - name: Corpus
    description: The whole benchmark corpus under `corpus/` at the repository root, outside the Cargo workspace.
    owner_component: BenchmarkCorpus
    identifier: "corpus/"
    attributes:
      - {name: index, type: CorpusIndex, required: true}
      - {name: toolchains, type: ToolchainPins, required: true}
      - {name: fairness_checklist, type: text, required: true, constraints: "corpus/fairness-checklist.md; the one checklist every review applies (Q3)"}
      - {name: programs, type: list of CorpusProgram, required: true, min: 40, max: 40}
    relationships:
      - {to: CorpusProgram, cardinality: "1:40", direction: "contains"}
      - {to: ClassBatch, cardinality: "1:4", direction: "delivered in four class batches (Q7)"}

  - name: CorpusIndex
    description: The machine-readable list of programs, read by the harness suites and the measurement tools.
    owner_component: BenchmarkCorpus
    identifier: "corpus/index.toml"
    attributes:
      - {name: schema_version, type: integer, required: true, allowed: [1]}
      - {name: entries, type: list of IndexEntry, required: true, min: 40, max: 40, constraints: "ordered by class, then program_id"}
    relationships:
      - {to: IndexEntry, cardinality: "1:40", direction: "lists"}

  - name: IndexEntry
    description: One program's row in the index, with everything needed to run it (BR1.4).
    owner_component: BenchmarkCorpus
    identifier: program_id
    attributes:
      - {name: program_id, type: text, required: true, unique: true}
      - {name: class, type: enum, required: true, allowed: [pure-transform, parsing-text, io-services, data-parallel]}
      - {name: task, type: text, required: true}
      - {name: path, type: text, required: true, constraints: "corpus/<class>/<program_id>/"}
      - {name: args, type: list of text, required: true, constraints: "argument template; placeholders {input:<name>} for files the generator writes, {threads} for data-parallel, {helper} for process-pipeline"}
      - {name: input_mode, type: enum, required: true, allowed: [stdin, files, fixture-dir]}
      - {name: stdin_from, type: text, required: "when input_mode is stdin", constraints: "the generated or fixed file fed to stdin"}
      - {name: threads_flag, type: boolean, required: true, constraints: "true exactly for data-parallel programs"}
      - {name: generator_outputs, type: list of text, required: true, min: 1, constraints: "file or directory names the generator writes under --out"}
      - {name: size_range, type: text, required: true, constraints: "smallest and largest meaningful --size, as two integers"}
      - {name: reference_size, type: integer, required: true, constraints: "the size the performance comparison uses by default; suites choose their own sizes within size_range"}
      - {name: fixed_inputs, type: list of text, required: true, min: 1, max: 8}
      - {name: comparison, type: enum, required: true, allowed: [stdout-exact, stdout-and-tree], constraints: "stdout-and-tree for io-services (BR6.4)"}
      - {name: floating_point, type: boolean, required: true, constraints: "true for n-body, spectral-norm, parallel-mandelbrot (BR2.4)"}

  - name: CorpusProgram
    description: One benchmark program (components.md CorpusProgram).
    owner_component: BenchmarkCorpus
    identifier: program_id
    attributes:
      - {name: program_id, type: text, required: true, unique: true, constraints: "lowercase kebab-case, at most 32 bytes; one of the 40 approved names (BR1.2)"}
      - {name: class, type: enum, required: true, allowed: [pure-transform, parsing-text, io-services, data-parallel]}
      - {name: task, type: text, required: true, constraints: "the one-line task statement approved in Q1"}
      - {name: languages, type: list of LanguageVersion, required: true, min: 4, max: 4, constraints: "exactly rail, rust, go, ocaml"}
      - {name: prompt, type: Prompt, required: true}
      - {name: fairness_review, type: FairnessReview, required: true}
      - {name: inputs, type: InputSet, required: true}
      - {name: expected, type: list of ExpectedOutput, required: true, min: 1}
    relationships:
      - {to: Prompt, cardinality: "1:1", direction: "written from"}
      - {to: LanguageVersion, cardinality: "1:4", direction: "has a version per language"}
      - {to: FairnessReview, cardinality: "1:1", direction: "reviewed in"}
      - {to: InputSet, cardinality: "1:1", direction: "reads"}
      - {to: ExpectedOutput, cardinality: "1:N", direction: "known-good results for fixed inputs"}

  - name: Prompt
    description: The single prompt all four versions are written from (Q2). File `prompt.md`.
    owner_component: BenchmarkCorpus
    identifier: "program_id"
    attributes:
      - {name: task, type: text, required: true}
      - {name: input_format, type: text, required: true}
      - {name: output_format, type: text, required: true, constraints: "byte-exact; no timestamps, hostnames, absolute paths or hash-map ordering"}
      - {name: error_cases, type: list of text, required: true, constraints: "every failure the program must report as a typed error"}
      - {name: run_contract, type: text, required: true, constraints: "the shared command-line contract (BR6.1)"}
      - {name: model_id, type: text, required: true, constraints: "the exact model identifier that wrote the versions"}
      - {name: written_on, type: date, required: true, constraints: "the date the versions were generated (provenance only; never part of program output)"}

  - name: LanguageVersion
    description: One language's implementation of a program.
    owner_component: BenchmarkCorpus
    identifier: "program_id + language"
    attributes:
      - {name: language, type: enum, required: true, allowed: [rail, rust, go, ocaml]}
      - {name: path, type: text, required: true, constraints: "corpus/<class>/<program_id>/<language>/"}
      - {name: entry, type: text, required: true, constraints: "rail: main.rlc; rust: Cargo.toml + src/main.rs; go: go.mod + main.go; ocaml: dune-project + bin/main.ml"}
      - {name: libraries, type: list of text, required: true, constraints: "empty except rayon (rust) or domainslib (ocaml) in data-parallel programs (BR5.2)"}
      - {name: validation, type: enum, required: true, allowed: [unvalidated, validated, defective], constraints: "rail versions stay unvalidated until the units that parse, check and run them land (BR8.2)"}
      - {name: edits, type: list of text, required: false, constraints: "defect fixes after generation, each logged in the fairness review (BR3.4)"}

  - name: FairnessReview
    description: The fairness review of one program (Q3). File `fairness-review.md`.
    owner_component: BenchmarkCorpus
    identifier: "program_id"
    attributes:
      - {name: checklist_results, type: list of ChecklistResult, required: true, constraints: "one result per item of corpus/fairness-checklist.md, each pass or fail with a note"}
      - {name: reviewer, type: text, required: true, constraints: "the AI reviewer's model id; the review runs in a session separate from the writing sessions (BR4.1)"}
      - {name: outcome, type: enum, required: true, allowed: [fair, needs-changes]}
      - {name: edit_log, type: list of text, required: true, constraints: "every post-generation edit with its reason; empty when none"}

  - name: ChecklistResult
    description: The result of one fairness-checklist item for one program.
    owner_component: BenchmarkCorpus
    identifier: "program_id + item number"
    attributes:
      - {name: item, type: integer, required: true, constraints: "the item's number in corpus/fairness-checklist.md"}
      - {name: result, type: enum, required: true, allowed: [pass, fail]}
      - {name: note, type: text, required: true, constraints: "what was checked, and for a fail, what must change"}

  - name: ClassBatch
    description: One class of ten programs, delivered and approved together (Q3, Q7).
    owner_component: BenchmarkCorpus
    identifier: class
    attributes:
      - {name: class, type: enum, required: true, allowed: [pure-transform, parsing-text, io-services, data-parallel]}
      - {name: status, type: enum, required: true, allowed: [in-progress, awaiting-approval, approved]}
      - {name: approval, type: text, required: "when approved", constraints: "your approval, recorded in corpus/<class>/APPROVAL.md with the date"}

  - name: InputSet
    description: Where a program's inputs come from (Q6).
    owner_component: BenchmarkCorpus
    identifier: "program_id"
    attributes:
      - {name: generator, type: text, required: true, constraints: "corpus/<class>/<program_id>/inputs/gen/: a std-only Rust program taking --seed <u64> --size <n> --out <dir> and writing the files named in the index (BR7.1)"}
      - {name: fixed, type: list of text, required: true, min: 1, max: 8, constraints: "small committed files under inputs/fixed/, each at most 64 KiB (BR7.3)"}
      - {name: io_fixture, type: text, required: "io-services only", constraints: "a directory tree or file set the program operates on, created by the generator or committed as fixed input"}

  - name: ExpectedOutput
    description: The known-good result for one fixed input.
    owner_component: BenchmarkCorpus
    identifier: "program_id + fixed input name"
    attributes:
      - {name: stdout, type: bytes, required: true, constraints: "expected/<input>.out"}
      - {name: exit_code, type: integer, required: true, allowed: [0, 1]}
      - {name: stderr, type: text, required: "when exit_code is 1", constraints: "expected/<input>.err; the typed-error line (BR6.2)"}
      - {name: source, type: enum, required: true, allowed: [rust-reference], constraints: "produced by the Rust version, changed only through the bless step (BR7.2)"}

  - name: ToolchainPins
    description: The pinned toolchains and the approved libraries (Q8, Q9). File `corpus/toolchains.toml`.
    owner_component: BenchmarkCorpus
    identifier: "corpus/toolchains.toml"
    attributes:
      - {name: rust, type: text, required: true, constraints: "same as the workspace rust-toolchain.toml (1.98.1)"}
      - {name: go, type: text, required: true, constraints: "exact newest stable at Code Generation time"}
      - {name: ocaml, type: text, required: true, constraints: "exact newest stable 5.x at Code Generation time, plus the pinned dune version"}
      - {name: libraries, type: list of ApprovedLibrary, required: true, constraints: "rayon (rust) and domainslib (ocaml)"}

  - name: ApprovedLibrary
    description: One third-party library the owner approved for the corpus (Q9).
    owner_component: BenchmarkCorpus
    identifier: "language + name"
    attributes:
      - {name: language, type: enum, required: true, allowed: [rust, ocaml]}
      - {name: name, type: enum, required: true, allowed: [rayon, domainslib]}
      - {name: version, type: text, required: true, constraints: "exact version, plus its pinned dependencies"}
      - {name: checksum, type: text, required: true, constraints: "of the vendored sources (BR5.5)"}
      - {name: vendored_path, type: text, required: true, constraints: "under corpus/vendor/"}
      - {name: reason, type: text, required: true, constraints: "approved by the owner 2026-10-09 for the data-parallel class only"}
      - {name: allowed_class, type: enum, required: true, allowed: [data-parallel]}
```

## Summary

| Entity | What it is |
|---|---|
| Corpus | `corpus/` at the repository root, outside the Cargo workspace. |
| CorpusIndex / IndexEntry | `corpus/index.toml`: all 40 programs, with the run data suites and tools need (arguments, input mode, threads, generator outputs, sizes, comparison). |
| CorpusProgram | One program: class, task, four language versions, prompt, review, inputs, expected outputs. |
| Prompt | `prompt.md`: the one prompt all four versions come from, with the model id and date. |
| LanguageVersion | One language's implementation and whether it has been validated. |
| FairnessReview | `fairness-review.md`: checklist results, the reviewer, the outcome and every later edit. |
| ClassBatch | Ten programs delivered and approved together. |
| InputSet | A seeded Rust generator plus small fixed inputs. |
| ExpectedOutput | Known-good output, exit code and error line for each fixed input, from the Rust version. |
| ToolchainPins / ApprovedLibrary | `corpus/toolchains.toml`: pinned Rust, Go, OCaml, dune, and the two approved libraries with versions, checksums and reasons. |
| ChecklistResult | One fairness-checklist item's pass or fail with a note. |
