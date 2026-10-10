# Fairness checklist

The one checklist every program's `fairness-review.md` applies (BR4.1). The
reviewer is an AI in a session separate from the writing sessions. For each
item, record `pass` or `fail` with a note saying what was checked and, for a
fail, what must change. Any `fail` sets the outcome to `needs-changes`.

1. **Same algorithm and data structures.** All four versions solve the task by
   the same method and use equivalent data structures. No version picks a
   cheaper algorithm than the others.
2. **Idiomatic code.** Each version is written the way a competent programmer
   in that language would write it. No version is deliberately slowed down or
   hand-tuned beyond what its prompt asks.
3. **No skipped work.** No language does less work than the others: the same
   validation, the same output formatting, the same error handling.
4. **Same error handling.** Every failure listed in the prompt is reported as
   `error: <kind>: <detail>` on stderr with exit code 1. Nothing else can
   crash, panic or raise an uncaught exception.
5. **Same output.** Output is byte-identical for every fixed input, with no
   timestamps, hostnames, absolute paths or hash-map ordering.
6. **Floating point (n-body, spectral-norm, parallel-mandelbrot,
   parallel-monte-carlo-pi only).** Only correctly rounded binary64 add,
   subtract, multiply, divide and square root, in the prompt's order. Go writes
   every product that feeds an addition through an explicit `float64()`
   conversion. OCaml binds every product to its own `let` before it is added.
   Output uses correctly rounded fixed-decimal formatting.
7. **Inputs and paths (io-services only).** The program reads and writes only
   the paths it is given, uses no network, and processes directory entries in
   byte order of their names.
8. **Libraries.** Standard library only. `rayon` (Rust) and `domainslib`
   (OCaml) appear only in data-parallel programs, at the pinned versions; Go
   uses goroutines and channels only.
9. **Parallelism (data-parallel only).** Partial results are combined in a
   fixed order, so output does not depend on the thread count.
10. **Provenance.** One prompt, one model, one session per language. The
    model id and date are recorded in `prompt.md`; every edit after generation
    is logged in the edit log with its reason and a reproducing input.
