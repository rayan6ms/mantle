#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly FIXTURES="$ROOT/.cache/performance/fixtures"
readonly JAVA_HOME="$ROOT/.cache/toolchains/jdk-21.0.12+8"
readonly RESULTS_DIR="$ROOT/docs/performance/results"
readonly REPETITIONS="${REPETITIONS:-3}"
readonly WARMUP_SECONDS="${WARMUP_SECONDS:-3}"
readonly MEASURE_SECONDS="${MEASURE_SECONDS:-8}"
readonly RESULT_FILE="${RESULT_FILE:-$RESULTS_DIR/lavaplayer-2.2.6-2026-08-10.jsonl}"

if [[ ! -x "$JAVA_HOME/bin/java" || ! -f "$REFERENCE/lavaplayer-2.2.6.jar" ]]; then
  "$ROOT/scripts/bootstrap-reference.sh" >/dev/null
fi
if [[ ! -f "$FIXTURES/reference.webm" ]]; then
  "$ROOT/scripts/generate-benchmark-fixtures.sh" >/dev/null
fi

mkdir -p "$RESULTS_DIR"
find "$REFERENCE/dependencies" -maxdepth 1 -name '*.jar' -print | sort > "$ROOT/.cache/reference-classpath.txt"
readonly DEPENDENCY_CLASSPATH="$(paste -sd: "$ROOT/.cache/reference-classpath.txt")"
readonly CLASSPATH="$REFERENCE/lavaplayer-2.2.6.jar:$DEPENDENCY_CLASSPATH"
readonly LD_LIBRARY_PATH="$JAVA_HOME/lib/server:$JAVA_HOME/lib"

export JAVA_HOME LD_LIBRARY_PATH
env -u APPIMAGE -u APPDIR cargo build --release --locked -p mantle-reference

"$ROOT/target/release/mantle-reference" serve --root "$FIXTURES" \
  >"$ROOT/.cache/performance-http.log" 2>&1 &
readonly SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true' EXIT

for _ in {1..50}; do
  if curl --fail --silent --head http://127.0.0.1:18080/reference.mp3 >/dev/null; then
    break
  fi
  sleep 0.1
done

: > "$RESULT_FILE"
: > "$ROOT/.cache/performance-benchmark.stderr.log"

run_case() {
  local workload=$1
  local input=$2
  local tracks=$3
  local repetition=$4
  shift 4
  "$ROOT/target/release/mantle-reference" benchmark \
    --classpath "$CLASSPATH" \
    --workload "$workload" \
    --input "$input" \
    --tracks "$tracks" \
    --warmup-seconds "$WARMUP_SECONDS" \
    --measure-seconds "$MEASURE_SECONDS" \
    --repetition "$repetition" \
    "$@" >> "$RESULT_FILE" 2>>"$ROOT/.cache/performance-benchmark.stderr.log"
}

for repetition in $(seq 1 "$REPETITIONS"); do
  run_case idle "" 0 "$repetition"
  for tracks in 1 10 50 100; do
    run_case opus-passthrough-local "$FIXTURES/reference.webm" "$tracks" "$repetition"
    run_case mp3-decode-local "$FIXTURES/reference.mp3" "$tracks" "$repetition"
    run_case aac-decode-local "$FIXTURES/reference.m4a" "$tracks" "$repetition"
    run_case flac-decode-local "$FIXTURES/reference.flac" "$tracks" "$repetition"
    run_case mp3-equalizer-local "$FIXTURES/reference.mp3" "$tracks" "$repetition" --filter
    run_case mp3-decode-http "http://127.0.0.1:18080/reference.mp3" "$tracks" "$repetition" --http
  done
  run_case mp3-seek-local "$FIXTURES/reference.mp3" 1 "$repetition" --seek
  run_case mp3-seek-http "http://127.0.0.1:18080/reference.mp3" 1 "$repetition" --http --seek
done

jq -e -s \
  --argjson expected "$((REPETITIONS * 27))" \
  'length == $expected and all(.frame_underruns == 0)' \
  "$RESULT_FILE" >/dev/null

printf 'wrote %s benchmark runs to %s\n' "$((REPETITIONS * 27))" "$RESULT_FILE"
