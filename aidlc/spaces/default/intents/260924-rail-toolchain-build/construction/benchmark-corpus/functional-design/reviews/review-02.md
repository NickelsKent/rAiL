## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-09T05:46:10Z
**Iteration:** 2

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260924-rail-toolchain-build/construction/benchmark-corpus/functional-design/rules.md > BR2.4; functional-spec.md > approved programs note on floating point | Mostly fixed: the design now limits floating point to n-body, spectral-norm and parallel-mandelbrot, allows only correctly rounded binary64 operations, forbids libm and FMA, mandates fixed decimals, and gives Go an explicit mechanism (float64() conversion). It gives no mechanism for OCaml, and the owner's reference machine is macOS AArch64. As far as I know, ocamlopt's arm64 backend selects fused multiply-add for `a*.b+.c` (I could not verify this offline, so it should be tested). If so, OCaml on the Mac disagrees with Rust and Go on Mandelbrot escape counts and n-body energies. BR2.4's only remedy is "fixed to follow the prompt's operation order", which does not prevent fusion. Rust is safe by default. | Add the OCaml mechanism to BR2.4, or state that Code Generation must first test whether the pinned ocamlopt fuses on arm64 and, if it does, name the fix (for example, a product bound to a non-inlined or boxed value, or an explicit rounding helper). Record the result in the fairness checklist. The corpus check on the Mac already detects a mismatch, so this is a correctness-of-fix gap rather than a silent hole. | Unresolved |
| R-02 | Major | functional-design/entities.md > IndexEntry; rules.md > BR1.4, BR7.1 | Resolved. IndexEntry now defines the argument template, input mode, stdin source, threads flag, generator outputs, size range, reference size, fixed inputs, comparison mode and floating-point flag. The generator contract is `--seed --size --out <dir>` and covers several files and fixture trees. Suite volumes are placed in the suite settings per contract C9. | None. | Resolved |
| R-03 | Major | functional-design/rules.md > BR2.5, BR6.4, BR6.5, BR6.6; functional-spec.md > WF3 step 6 | Resolved. Confinement is enforced by a canary directory plus a final tree listing, and reads outside given paths are explicitly moved to the fairness checklist. Permission cases are created at run time and skipped visibly under root. The process-pipeline helper is named and built by the checker. Sorted traversal and checksum-based change detection replace mtimes, and the generator creates empty directories. | None. | Resolved |
| R-04 | Minor | functional-design/entities.md > IndexEntry, ApprovedLibrary, ChecklistResult | Resolved. All three are now defined as entities with identifiers and attributes. | None. | Resolved |
| R-05 | Minor | functional-design/entities.md > FairnessReview.reviewer; rules.md > BR4.1 | Resolved. Both now say: separate session, any model, model id recorded. | None. | Resolved |
| R-06 | Minor | functional-design/rules.md > BR5.4, BR5.5, BR9.2 | Resolved. Corpus lockfiles are checked by `cargo deny` with `corpus/deny.toml`, vendored OCaml packages are pinned by checksum, and BR9.2 lists the §11.1 spec update in the same change. Spec §11.1 does not yet carry it, which is expected until the corpus pull request. | None. | Resolved |
| R-07 | Minor | functional-design/rules.md > BR3.3 | Resolved. The definition-ID format is stated (`#` plus six Crockford base32 characters), and the check verifies format and uniqueness. | None. | Resolved |
| R-08 | Minor | functional-design/functional-spec.md > floating-point note and parallel-monte-carlo-pi row; entities.md > IndexEntry.floating_point | The spec says only three programs compute in floating point, but parallel-monte-carlo-pi divides hits by samples and prints pi to six decimals. That is a floating-point division and print, so `floating_point` would be false for a program that does floating point. | Mark parallel-monte-carlo-pi `floating_point: true` and bring it under BR2.4 (one correctly rounded division, fixed decimals), or print only the integers and drop the pi line. Update the "only three" sentence to match. | New |
| R-09 | Minor | functional-design/rules.md > BR1.4 vs entities.md > IndexEntry | BR1.4 names "generator mode" and "generator size range", while the entity fields are `generator_outputs`, `size_range`, `reference_size` and `stdin_from`. The wording could lead to a checker that tests the wrong field names. | Align BR1.4's field list with the IndexEntry attribute names. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| fenced yaml parse (entities.md, rules.md) | PASS: 13 entities, 33 rules, 33 unique BR IDs | Structure is valid. |
| traceability.json | Every BRx.y ID it cites exists in rules.md (no unresolved IDs) | Valid; coverage of FR54, NFR3, NFR5, NFR6 and NFR7 resolves. |
| sensors (required-sections, upstream-coverage, linter, type-check) | Not run: the engine sensors are not runnable from the review scope | Structural checks done by hand above. |

### Summary

The revision resolves the run-descriptor and I/O-class gaps (R-02, R-03) and all the Minor items. One Major remains, because OCaml has no fused-multiply-add mitigation on the owner's arm64 Mac. That is within the READY threshold (zero Critical, at most two Major), and the corpus check will catch a mismatch. Fix R-01 and R-08 before Code Generation starts the pure-transform class.
