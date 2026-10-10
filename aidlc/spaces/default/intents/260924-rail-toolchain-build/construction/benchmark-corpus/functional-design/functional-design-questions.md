# Functional Design — benchmark-corpus (U16) — Questions

The benchmark corpus is 40 programs in four classes of ten: pure data transformation, parsing and text, I/O services with typed errors, and data-parallel compute (spec §11.1, FR54). Each program is written in rAiL, Rust, Go and OCaml under the same prompt and reviewed for fairness. Later suites run it: T3 (deterministic builds), T4 (round trips), T7 (cross-platform), T9 (functional correctness, with each rAiL program checked against its Rust twin), T10 (memory safety) and T13 (performance). The model-token measurements in §11.3 also tokenize it.

Already settled (not asked again):
- All four languages and the model measurements are in this work (scope Q11).
- The layout is `corpus/<class>/<program>/{rail,rust,go,ocaml}/` plus `inputs/` and `expected/`, with `prompt.md` and `fairness-review.md` per program (contract C10).
- The rAiL versions are written in canonical text straight from the spec, before the parser exists. A version that later proves invalid is a defect, fixed with a reproducing test (Units Generation).
- The corpus sits outside the toolchain's Cargo workspace (Units Generation).
- Rust 1.9x, Go 1.2x and OCaml 5.x (spec §11.1).

Note: Go and OCaml are not installed on this Mac yet. Code Generation will need them, and probably CI too.

## Q1. How are the 40 programs chosen?

The spec fixes the four classes and ten programs per class, but not which programs.

A. I propose a named list of 40 programs, ten per class, each with a one-line task statement, as part of this design for you to approve. The choices draw on well-known benchmark tasks where they fit the class, such as n-body, JSON parsing, or word frequency (recommended)
B. Adopt an existing benchmark collection as closely as the classes allow
C. You supply the list
X. Other (please specify)

[Answer]: A

## Q2. How is "written by an agent under the same prompt" carried out?

Spec §11.1 says each program is written once in rAiL and once idiomatically in Rust, Go and OCaml by an agent under the same prompt. The prompt and the model affect fairness and the token measurements.

A. Each program has one `prompt.md`: the task statement, input and output format, and the error cases. The same model writes all four versions from that prompt, each in a separate session given only the prompt and that language's reference (the spec for rAiL). `prompt.md` records the model id and the date. Hand edits afterwards are allowed only to fix defects, and each edit is noted in `fairness-review.md` (recommended)
B. As A, but the versions may be freely hand-tuned after generation
X. Other (please specify)

[Answer]: A

## Q3. Who reviews each program for fairness, and against what?

A. A written checklist, applied to every program: same algorithm and data structures, idiomatic code, no language given work the others skip, the same error handling, and the same output. An AI reviewer fills in `fairness-review.md` for each program. You approve each class of ten as a batch (recommended)
B. As A, but you review every program yourself
C. AI review only, with no sign-off from you
X. Other (please specify)

[Answer]: A

## Q4. What libraries may the Rust, Go and OCaml versions use?

Third-party libraries can make a comparison unfair in either direction, and the rAiL versions can only use rAiL's own standard library.

A. Each language's standard library only, including its own concurrency (Rust `std::thread`, Go goroutines and channels, OCaml 5 `Domain`). No third-party packages anywhere, so builds need no network or package manager (recommended)
B. Standard libraries, plus each language's most common parallelism library (for example Rust `rayon`, OCaml `domainslib`) for the data-parallel class
X. Other (please specify)

[Answer]: B

## Q5. What may the I/O services class do?

rAiL's file, process and environment modules come with Unit 7. Its networking modules (TCP, UDP, DNS, TLS) come much later, with Unit 9. Programs that use the network cannot run in rAiL until then.

A. Files, standard input and output, processes and environment only, with typed errors for every failure (missing files, bad input, permission denied). No networking (recommended)
B. Mostly as A, plus up to three programs that serve or call a local loopback network service. Those run in rAiL only after Unit 9
X. Other (please specify)

[Answer]: A

## Q6. How are inputs and expected outputs produced?

T9 compares each rAiL program with its Rust twin on 10,000 generated inputs. Other suites need fixed inputs and known-good outputs.

A. Each program has a deterministic input generator written in Rust, std only, that takes a seed and a size, plus a few small fixed input files. The Rust version is the reference: the committed expected outputs for the fixed inputs come from it, and are changed only through the explicit bless step. Large inputs are generated at run time, never committed (recommended)
B. Fixed input files only, committed, with expected outputs written by hand
X. Other (please specify)

[Answer]: A

## Q7. Is the whole corpus written in this unit?

40 programs in four languages is 160 source programs plus prompts, reviews, generators and expected outputs. Only the Rust, Go and OCaml versions can be run and checked now. The rAiL versions wait until later units can parse, check and run them.

A. All 40 programs, in all four languages, in this unit, delivered one class at a time with your approval of each class (recommended)
B. One class of ten now; the remaining thirty before the first suite that needs the full corpus (T9 with Unit 7)
X. Other (please specify)

[Answer]: A

## Q8. Which toolchain versions are pinned, and where?

A. Rust uses the workspace's pinned stable (1.98.1). Go and OCaml use the newest stable releases at the time of Code Generation, checked against their official sites rather than from memory, and recorded in `corpus/toolchains.toml`. Changing a pinned version is a reviewed change (recommended)
B. Whatever version is installed on each machine
X. Other (please specify)

[Answer]: A

## Follow-up questions

## Q9. Which parallelism libraries do you approve for the data-parallel class?

You chose to allow each language's common parallelism library in the data-parallel class (Q4). Your practice is that every new Rust dependency needs a stated reason and your approval. The corpus is outside the toolchain workspace and never ships, but the same rule is the safe reading, and the OCaml and Go equivalents should be treated the same way. These libraries also mean the corpus build needs a package manager. To keep builds offline and reproducible, they would be pinned by lockfile and vendored into the repository.

A. Rust: `rayon` (and its few dependencies). OCaml: `domainslib`. Go: none, since goroutines and channels are its standard way. Each is used only in the ten data-parallel programs, pinned and vendored, and listed with its reason in `corpus/toolchains.toml` (recommended)
B. Rust `rayon` only; the OCaml versions use the standard `Domain` module
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Program list:** I propose the 40 programs below, ten per class, each with a one-line task, for you to approve as part of this design (Q1):
  - **Pure data transformation:** binary-trees (allocate and walk trees), fannkuch-redux (permutation flips), n-body (planet simulation steps), spectral-norm (matrix eigenvalue estimate), levenshtein-batch (edit distances for word pairs), knapsack-dp (0/1 knapsack by dynamic programming), record-sort-dedup (sort records, drop duplicates), run-length-codec (encode and decode), base64-codec (encode and decode), dijkstra-paths (shortest paths in a weighted graph).
  - **Parsing and text:** json-validate (parse and re-print JSON), csv-aggregate (group and sum CSV columns), word-frequency (top words in a text), mini-regex (match a small regex subset), markdown-lite (a Markdown subset to HTML), ini-config (parse INI files with error locations), log-analyze (count and classify log lines), expr-calc (parse and evaluate arithmetic), utf8-stats (count characters and categories in UTF-8 text), line-diff (line diff by longest common subsequence).
  - **I/O services with typed errors:** file-checksum (copy a file and verify its checksum), dir-summary (sizes and counts for a directory tree), kv-log-store (append-only key-value store in a file), config-loader (load and validate a configuration file), batch-rename (rename files by pattern, with a dry run), log-rotate (rotate log files by size, keeping a fixed number), process-pipeline (run a child process and capture its output), env-report (a report driven by environment variables), csv-to-json-files (convert files, reporting per-file errors), incremental-backup (copy only changed files).
  - **Data-parallel compute:** parallel-sum (sum-reduce a large array), parallel-histogram, parallel-mandelbrot (render tiles), parallel-matmul (dense matrix multiply), parallel-merge-sort, parallel-word-count (count words over shards), parallel-prefix-scan, parallel-bfs (breadth-first search on a large graph), parallel-monte-carlo-pi (seeded per-task random streams), parallel-blur (2-D image stencil).
- **Authoring:** one `prompt.md` per program (task, input and output format, error cases). The same model writes all four versions, each in a separate session given only the prompt and that language's reference. The model id and date are recorded. Later hand edits are only for defects and are noted in `fairness-review.md` (Q2).
- **Fairness review:** a written checklist applied to every program. An AI reviewer fills in `fairness-review.md`, and you approve each class of ten as a batch (Q3).
- **Libraries:** standard libraries only, except in the ten data-parallel programs, which may use Rust `rayon` and OCaml `domainslib`. Go uses only its standard goroutines and channels. Both libraries are approved, pinned by lockfile, vendored so builds work offline, and listed with their reasons in `corpus/toolchains.toml` (Q4, Q9).
- **I/O class:** files, standard input and output, processes and environment only, with typed errors for every failure. No networking (Q5).
- **Inputs and outputs:** a seeded, std-only Rust input generator per program plus a few small fixed input files. The Rust version is the reference for committed expected outputs, which change only through the bless step. Large inputs are generated at run time, never committed (Q6).
- **Scope:** all 40 programs in all four languages in this unit, delivered one class at a time, with your approval of each class (Q7).
- **Toolchains:** Rust uses the workspace's pinned 1.98.1. Go and OCaml use the newest stable releases at Code Generation time, checked on their official sites and recorded in `corpus/toolchains.toml`. A pin change is a reviewed change (Q8).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
