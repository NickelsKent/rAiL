#!/usr/bin/env bash
# Walking-skeleton verification (WF6, NFR9.14). A candidate for the recorded
# verification command; the command itself is chosen at the skeleton
# checkpoint.
#
# Builds `rail`, then runs the seven WF6 steps against fresh copies of the
# fixtures and compares results with the shared golden files, so the same
# expectations hold on macOS and Linux. Prints `PASS n ...` or `FAIL n ...`
# per step, with the command and its captured output on failure, and exits
# non-zero if any step fails.
#
# Usage: scripts/verify-skeleton.sh
set -uo pipefail
# Byte-oriented string lengths for the Content-Length headers.
export LC_ALL=C

root="$(cd "$(dirname "$0")/.." && pwd)"
golden="$root/crates/rail/tests/golden"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failures=0

if ! cargo build --locked --quiet --manifest-path "$root/Cargo.toml" -p rail; then
  echo "FAIL 0 build rail"
  exit 1
fi
rail="$root/target/debug/rail"

for ws in one two; do
  mkdir -p "$work/$ws"
  cp -R "$root/fixtures/." "$work/$ws/"
done
ws="$work/one"

# run <name> <stdin-file> <args...>: runs rail in $ws, saving
# $work/<name>.out, .err and .code.
run() {
  local name="$1" input="$2"
  shift 2
  (cd "$ws" && "$rail" "$@" <"$input" >"$work/$name.out" 2>"$work/$name.err")
  echo $? >"$work/$name.code"
}

# report <n> <title> <ok> <name> <command>
report() {
  if [ "$3" = yes ]; then
    echo "PASS $1 $2"
  else
    failures=$((failures + 1))
    echo "FAIL $1 $2"
    echo "  command: $5"
    echo "  exit code: $(cat "$work/$4.code" 2>/dev/null)"
    echo "  stdout: $(cat "$work/$4.out" 2>/dev/null)"
    echo "  stderr: $(cat "$work/$4.err" 2>/dev/null)"
  fi
}

same() { cmp -s "$1" "$2" && echo yes || echo no; }
code_is() { [ "$(cat "$work/$1.code")" = "$2" ] && echo yes || echo no; }
both() { [ "$1" = yes ] && [ "$2" = yes ] && echo yes || echo no; }

frame() {
  printf 'Content-Length: %d\r\n\r\n%s' "${#1}" "$1"
}

# One JSON response per line from a stream of framed responses.
unframe() {
  perl -0777 -ne 'while (/\GContent-Length: (\d+)\r\n\r\n/gc) { print substr($_, pos($_), $1), "\n"; pos($_) += $1 }' "$1"
}

# expected_result <id> <golden file>: the framed response line for a result.
expected_result() {
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$1" "$(tr -d '\n' <"$golden/$2")"
}

: >"$work/empty"

# Step 1: check skeleton.answer reports no diagnostics.
run s1 "$work/empty" check --json skeleton/answer.rlc
report 1 "check skeleton.answer: no diagnostics" \
  "$(both "$(code_is s1 0)" "$(same "$work/s1.out" "$golden/check-answer.json")")" \
  s1 "rail check --json skeleton/answer.rlc"

# Step 2: run skeleton.answer prints 42 and exits 0.
run s2 "$work/empty" run skeleton/answer.rlc
printf '42\n' >"$work/s2.expected"
report 2 "run skeleton.answer: prints 42, exit 0" \
  "$(both "$(code_is s2 0)" "$(same "$work/s2.out" "$work/s2.expected")")" \
  s2 "rail run skeleton/answer.rlc"

# Step 3: a protocol session on skeleton.answer.
root_json="$(printf '%s' "$ws" | sed 's/\\/\\\\/g; s/"/\\"/g')"
answer='{"module":"skeleton/answer.rlc"}'
{
  frame "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"client_versions\":[\"0.1\"],\"workspace_root\":\"$root_json\"}}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tree.get\",\"params\":$answer}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"check.run\",\"params\":$answer}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"build.run\",\"params\":$answer}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"run.run\",\"params\":$answer}"
} >"$work/s3.in"
run s3 "$work/s3.in" rap
unframe "$work/s3.out" >"$work/s3.lines"
case "$(uname -s)" in Darwin) target=aarch64-macos ;; *) target=x86_64-linux ;; esac
{
  echo '{"jsonrpc":"2.0","id":1,"result":{"rap_version":"0.1","capabilities":["tree.get","check.run","build.run","run.run"]}}'
  expected_result 2 tree-answer.json
  expected_result 3 check-answer.json
  echo "{\"jsonrpc\":\"2.0\",\"id\":4,\"result\":{\"module\":\"skeleton.answer\",\"target\":\"$target\",\"mode\":\"dev\",\"path\":\".rail/build/dev/skeleton/answer\"}}"
  expected_result 5 run-answer.json
} >"$work/s3.expected"
report 3 "protocol session on skeleton.answer: run.run returns 42" \
  "$(both "$(code_is s3 0)" "$(same "$work/s3.lines" "$work/s3.expected")")" \
  s3 "rail rap < initialize, tree.get, check.run, build.run, run.run"

# Step 4: skeleton.broken is refused with the one TY001, on both surfaces.
run s4a "$work/empty" check --json skeleton/broken.rlc
run s4b "$work/empty" run --json skeleton/broken.rlc
broken='{"module":"skeleton/broken.rlc"}'
{
  frame "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"client_versions\":[\"0.1\"],\"workspace_root\":\"$root_json\"}}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"check.run\",\"params\":$broken}"
  frame "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"run.run\",\"params\":$broken}"
} >"$work/s4c.in"
run s4c "$work/s4c.in" rap
unframe "$work/s4c.out" | tail -n +2 >"$work/s4c.lines"
tool_error="$(tr -d '\n' <"$golden/run-broken.json" | sed 's/^{"error"://; s/}$//')"
{
  expected_result 2 check-broken.json
  echo "{\"jsonrpc\":\"2.0\",\"id\":3,\"error\":{\"code\":-32000,\"message\":\"skeleton.broken has blocking diagnostics\",\"data\":$tool_error}}"
} >"$work/s4c.expected"
ok4="$(both "$(both "$(code_is s4a 1)" "$(same "$work/s4a.out" "$golden/check-broken.json")")" \
  "$(both "$(code_is s4b 1)" "$(same "$work/s4b.out" "$golden/run-broken.json")")")"
ok4="$(both "$ok4" "$(same "$work/s4c.lines" "$work/s4c.expected")")"
if [ -e "$ws/.rail/build/dev/skeleton/broken" ]; then ok4=no; fi
report 4 "skeleton.broken refused with one TY001 (command line and protocol)" "$ok4" \
  s4b "rail check --json / rail run --json skeleton/broken.rlc; check.run and run.run over rail rap"

# Step 5: the server still answers after a malformed message.
{
  frame "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"client_versions\":[\"0.1\"],\"workspace_root\":\"$root_json\"}}"
  frame '{"jsonrpc":"2.0","id":2,'
  frame "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"check.run\",\"params\":$answer}"
} >"$work/s5.in"
run s5 "$work/s5.in" rap
unframe "$work/s5.out" | tail -n +2 >"$work/s5.lines"
ok5=no
if head -1 "$work/s5.lines" | grep -q '^{"jsonrpc":"2.0","id":null,"error":{"code":-32700,'; then
  expected_result 3 check-answer.json >"$work/s5.expected"
  ok5="$(same <(tail -n +2 "$work/s5.lines") "$work/s5.expected")"
fi
report 5 "server answers after a malformed message" "$ok5" s5 "rail rap < initialize, malformed JSON, check.run"

# Step 6: two builds (in two directories) are byte-identical.
run s6a "$work/empty" build skeleton/answer.rlc
ws="$work/two"
run s6b "$work/empty" build skeleton/answer.rlc
ws="$work/one"
report 6 "two builds of skeleton.answer are byte-identical" \
  "$(same "$work/one/.rail/build/dev/skeleton/answer" "$work/two/.rail/build/dev/skeleton/answer")" \
  s6b "rail build skeleton/answer.rlc (in two workspaces)"

# Step 7: steps 1-4 matched the golden files shared by both platforms.
if [ "$failures" -eq 0 ]; then
  echo "PASS 7 outputs of steps 1-4 equal the shared golden files ($(uname -s))"
else
  failures=$((failures + 1))
  echo "FAIL 7 outputs of steps 1-4 differ from the shared golden files (see above)"
fi

if [ "$failures" -ne 0 ]; then
  echo "walking skeleton: $failures step(s) failed"
  exit 1
fi
echo "walking skeleton: all steps passed"
