#!/usr/bin/env bash
# NFR4.1: every crate root except rail-codegen and rail-runtime (and the
# build script of rail-build) must forbid unsafe code.
set -euo pipefail
cd "$(dirname "$0")/.."
status=0
while IFS= read -r root; do
  case "$root" in
    crates/rail-codegen/src/lib.rs | crates/rail-runtime/src/lib.rs) continue ;;
  esac
  if ! grep -q '^#!\[forbid(unsafe_code)\]' "$root"; then
    echo "missing #![forbid(unsafe_code)]: $root"
    status=1
  fi
done < <(find crates \( -path '*/target' -o -path '*/tests' \) -prune -o \( -path '*/src/lib.rs' -o -path '*/src/main.rs' -o -path '*/src/bin/*.rs' -o -name build.rs \) -print | sort)
if [ "$status" -eq 0 ]; then
  echo "forbid(unsafe_code): ok"
fi
exit "$status"
