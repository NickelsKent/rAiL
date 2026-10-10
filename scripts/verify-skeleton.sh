#!/usr/bin/env bash
# Walking-skeleton verification (WF6, NFR9.14): the recorded checkpoint
# verification command.
#
# A thin wrapper around the acceptance harness (BR8.2): it builds
# `rail-accept` and runs `rail-accept --level pr --suite S0`. S0 builds
# `rail` and runs the seven walking-skeleton steps against fresh copies of
# the fixtures, comparing results with the golden files shared by macOS and
# Linux. Prints one `PASS n ...` or `FAIL n ...` line per step (with the
# reason on failure) and exits 0 only when S0 passes.
#
# Usage: scripts/verify-skeleton.sh
set -uo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

if ! cargo build --locked --quiet --manifest-path "$root/Cargo.toml" -p rail-testkit --bin rail-accept; then
  echo "FAIL 0 build rail-accept"
  exit 1
fi

"$root/target/debug/rail-accept" --level pr --suite S0 --root "$root" --out "$out"
status=$?

if [ "$status" -ne 0 ]; then
  echo "walking skeleton: S0 failed (rail-accept exit code $status)"
  exit 1
fi
echo "walking skeleton: all steps passed"
