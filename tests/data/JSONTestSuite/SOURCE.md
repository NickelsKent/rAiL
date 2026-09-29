# JSONTestSuite (vendored test data)

- Upstream: https://github.com/nst/JSONTestSuite
- Commit: `1ef36fa01286573e846ac449e8683f8833c5b26a` (2024-11-22)
- Vendored: 2026-09-27, the 318 files of `test_parsing/`, unmodified.
- Licence: MIT, Copyright (c) 2016 Nicolas Seriot. The upstream licence text is
  in `LICENSE` next to this file. MIT is compatible with rAiL's
  MIT OR Apache-2.0 licence.

Used by `crates/rail-json/tests/json_test_suite.rs` (NFR4.3): every `y_` case
must be accepted, every `n_` case rejected, and no case may panic. The one
deliberate difference: `y_object_duplicated_key.json` and
`y_object_duplicated_key_and_value.json` are rejected, because NFR4.3
requires duplicate object keys to be rejected; the test asserts that they fail
with the duplicate-key error and nothing else. The `i_`
(implementation-defined) cases only have to finish without a panic. The files
are also the seed corpus for the `json_parse` fuzz target.

`.gitattributes` marks the directory binary so line-ending normalisation never
changes a test case.
