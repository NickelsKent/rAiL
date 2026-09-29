#!/usr/bin/env bash
# NFR5.2: a program linked with the skeleton runtime imports only the
# allowed C-library symbols. Usage: check-runtime-symbols.sh <program>
set -euo pipefail
program="$1"
here="$(cd "$(dirname "$0")" && pwd)"
case "$(uname -s)" in
  Darwin) list="$here/allowlists/runtime-symbols-macos.txt" ;;
  Linux) list="$here/allowlists/runtime-symbols-linux.txt" ;;
  *) echo "unsupported platform $(uname -s)"; exit 1 ;;
esac
found="$(nm -u "$program" | awk '{print $NF}' | sed 's/@.*//' | sort -u)"
unexpected="$(comm -23 <(printf '%s\n' "$found") <(grep -v '^#' "$list" | sort -u))"
if [ -n "$unexpected" ]; then
  echo "symbols outside the allow-list ($list):"
  printf '%s\n' "$unexpected"
  exit 1
fi
echo "runtime symbols: ok ($(printf '%s\n' "$found" | tr '\n' ' '))"
