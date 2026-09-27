# Functional Design — walking-skeleton (U1) — Questions

The walking skeleton is a thin slice that runs end to end: one small canonical rAiL module is parsed, formatted, checked, served through the agent protocol, compiled to a native binary and run on macOS and Linux. It proves the pieces connect; every part stays deliberately minimal and is deepened by later units. These questions fix how small "minimal" is and what the end-to-end demonstration shows.

Already settled: the skeleton reaches a native binary on both platforms (practices); one recorded command you approve verifies it (chosen at the skeleton checkpoint); tests come first; RAP uses JSON-RPC 2.0 with `Content-Length` framing and an `initialize` call (contracts); the command line prints human text by default and `--json` on request.

## Q1. What does the skeleton program do, and how do we see that it worked?

The spec gives `main` the shape `main(caps) -> Result[unit, E]` (exit 0 on `Ok`, 1 on `Err`). Printing to the terminal in the full language goes through the standard library and capabilities, which come later.

A. A pure function (for example `answer` returning `42` computed by arithmetic) called from `main`; `main` returns `Ok` when the result is correct and `Err` otherwise, so success is seen as exit code 0 and failure as exit code 1; nothing is printed (recommended; uses only language features the full design keeps)
B. The same, plus a skeleton-only built-in that prints the result, removed once the standard library's output exists
C. The spec's Example 2 (`count_lines`), which needs part of `std.text`
X. Other (please specify)

[Answer]: B

## Q2. Which language forms must the skeleton understand?

A. Only what the skeleton program needs: `mod`, `pub`, `fn` with a signature, application, integer and Boolean literals, the arithmetic and comparison operators, `let`, and a Boolean match `(? c (true a) (false b))`; every other form is rejected with a located "not yet supported" diagnostic (recommended)
B. The full canonical form set from the start, parsed and printed but not all checked or compiled
X. Other (please specify)

[Answer]: A

## Q3. How much type checking does the skeleton do?

A. Monomorphic only: every function has an explicit signature; types limited to `i64`, `bool` and `unit` plus the `Result` needed by `main`; mismatches produce a `TY001` diagnostic in the real diagnostic shape (recommended)
B. Real inference from the start for the forms in Q2
X. Other (please specify)

[Answer]: A

## Q4. Which agent protocol methods does the skeleton serve?

A. `initialize`, `tree.get`, `check.run`, `build.run` and `run.run`, so an agent can drive the whole slice over the protocol (recommended)
B. `initialize`, `tree.get` and `check.run` only; building and running are command-line only in the skeleton
X. Other (please specify)

[Answer]: A

## Q5. Should the skeleton also demonstrate the failure path?

A. Yes: a second module with a deliberate type error shows the located diagnostic coming back through both the command line and the protocol, and the build refusing to run (recommended; proves the "errors block builds" path connects too)
B. No: happy path only
X. Other (please specify)

[Answer]: A

## Follow-up questions

Your answer to Q1 (B) adds a temporary print built-in. That raises two details the design needs.

## Q6. How should the temporary print built-in fit the language's rules?

In the full language every effect a function has shows in its signature, and output goes through capabilities from `main`. The skeleton has neither the standard library nor capabilities yet. The built-in prints one `i64` in decimal followed by a newline to standard output, lives under a reserved `skel` name that user code cannot define, and is deleted when the standard library's output lands (U7).

A. It carries the `log` effect label: any function that calls it, including `main`, must list `log` in its signature, and the skeleton checker reports a missing label as a located diagnostic; this keeps "effects are visible in signatures" true even in the skeleton (recommended)
B. It is typed as a plain function with no effect label, recorded as a known, temporary exception to effect visibility that U7 removes
X. Other (please specify)

[Answer]: A

## Q7. How does the protocol's run method return what the program printed?

The contracts left open whether `run.run` streams program output or returns it at the end (open question on the agent protocol, E1). The skeleton program now prints, so the skeleton has to pick one.

A. Return it at the end: the result carries the exit code plus the captured standard output and standard error as text; streaming, if wanted, is decided when the agent loop (U5) and the standard library (U7) are built (recommended)
B. Stream it while the program runs, as progress notifications, and return only the exit code at the end
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
