#!/usr/bin/env bash
# The corpus check (WF3): builds the std-only checker and runs it over
# corpus/. Pass --help for the options. Touches only corpus/ (builds and run
# directories go to a temporary directory).
#
#   corpus/check.sh                       every delivered class
#   corpus/check.sh --class pure-transform
#   corpus/check.sh --program n-body
#   corpus/check.sh --bless               write expected/ from the Rust versions
#                                         (or RAIL_BLESS=1)
#
# Exit codes: 0 all passed, 1 the corpus has problems, 2 the check could not
# run (usage, toolchain or sandbox problem).
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# CORPUS_CHECK_TARGET_DIR lets a caller (the checker's own tests) build elsewhere.
target="${CORPUS_CHECK_TARGET_DIR:-$here/tools/check/target}"

if ! cargo build --locked --quiet --manifest-path "$here/tools/check/Cargo.toml" --target-dir "$target"; then
  echo "error: cannot build the corpus checker" >&2
  exit 2
fi

# A later --corpus-dir on the command line overrides this one.
exec "$target/debug/corpus-check" --corpus-dir "$here" "$@"
