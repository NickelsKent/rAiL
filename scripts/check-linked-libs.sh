#!/usr/bin/env bash
# NFR7.6, BR5.6: a built program links only operating-system libraries.
# Usage: check-linked-libs.sh <program>
set -euo pipefail
program="$1"
here="$(cd "$(dirname "$0")" && pwd)"
case "$(uname -s)" in
  Darwin)
    list="$here/allowlists/linked-libs-macos.txt"
    found="$(otool -L "$program" | tail -n +2 | awk '{print $1}' | sort -u)" ;;
  Linux)
    list="$here/allowlists/linked-libs-linux.txt"
    found="$(ldd "$program" | awk '{print ($2 == "=>") ? $1 : $1}' | sed 's|.*/||' | sort -u)" ;;
  *) echo "unsupported platform $(uname -s)"; exit 1 ;;
esac
unexpected="$(comm -23 <(printf '%s\n' "$found") <(grep -v '^#' "$list" | sort -u))"
if [ -n "$unexpected" ]; then
  echo "libraries outside the allow-list ($list):"
  printf '%s\n' "$unexpected"
  exit 1
fi
echo "linked libraries: ok ($(printf '%s\n' "$found" | tr '\n' ' '))"
