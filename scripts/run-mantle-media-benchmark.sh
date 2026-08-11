#!/usr/bin/env bash
set -euo pipefail

unset ARGV0 || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly FIXTURES="$ROOT/.cache/performance/fixtures"
readonly RESULTS_DIR="$ROOT/docs/performance/results"
readonly REPETITIONS="${REPETITIONS:-5}"
readonly RESULT_FILE="${RESULT_FILE:-$RESULTS_DIR/mantle-media-phase6-2026-08-11.jsonl}"
readonly SUMMARY_FILE="${SUMMARY_FILE:-$RESULTS_DIR/mantle-media-phase6-2026-08-11-summary.json}"
readonly METADATA_FILE="${METADATA_FILE:-$RESULTS_DIR/mantle-media-phase6-2026-08-11-metadata.json}"
readonly BUILD_RESULT="$ROOT/.cache/mantle-media-phase6-build.json"
readonly HTTP_LOG="$ROOT/.cache/mantle-media-phase6-http.log"
readonly HTTP_ADDRESS="127.0.0.1:18081"
readonly HTTP_URL="http://$HTTP_ADDRESS/reference.mp3"
SOURCE_COMMIT="$(git -C "$ROOT" rev-parse HEAD)"
readonly SOURCE_COMMIT
SOURCE_WORKTREE_CLEAN="$(if [[ -z "$(git -C "$ROOT" status --porcelain)" ]]; then printf true; else printf false; fi)"
readonly SOURCE_WORKTREE_CLEAN

if [[ ! -f "$FIXTURES/reference.mp3" ]]; then
  "$ROOT/scripts/generate-benchmark-fixtures.sh" >/dev/null
fi

mkdir -p "$RESULTS_DIR" "$ROOT/.cache"
BUILD_TARGET="$(mktemp -d "$ROOT/.cache/mantle-media-build.XXXXXX")"
readonly BUILD_TARGET
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$BUILD_TARGET"
}
trap cleanup EXIT

/usr/bin/time -f '{"elapsed_seconds":%e,"peak_rss_kib":%M}' -o "$BUILD_RESULT" \
  env -u APPIMAGE -u APPDIR -u ARGV0 CARGO_TARGET_DIR="$BUILD_TARGET" \
  cargo build --release --locked -p mantle-media-bench

readonly BENCH="$BUILD_TARGET/release/mantle-media-bench"
BINARY_BYTES="$(stat -c '%s' "$BENCH")"
readonly BINARY_BYTES
: > "$RESULT_FILE"
: > "$HTTP_LOG"

"$BENCH" serve --root "$FIXTURES" --address "$HTTP_ADDRESS" > /dev/null 2> "$HTTP_LOG" &
SERVER_PID=$!
for _ in {1..50}; do
  if curl --fail --silent --head "$HTTP_URL" >/dev/null; then
    break
  fi
  sleep 0.1
done
curl --fail --silent --head "$HTTP_URL" >/dev/null

run_case() {
  local workload=$1
  local input=$2
  local repetition=$3
  shift 3
  "$BENCH" run \
    --workload "$workload" \
    --input "$input" \
    --repetition "$repetition" \
    "$@" >> "$RESULT_FILE"
}

for repetition in $(seq 1 "$REPETITIONS"); do
  run_case wav-decode-local "$FIXTURES/reference.wav" "$repetition"
  run_case mp3-decode-local "$FIXTURES/reference.mp3" "$repetition"
  run_case aac-decode-local "$FIXTURES/reference.m4a" "$repetition"
  run_case opus-passthrough-local "$FIXTURES/reference.webm" "$repetition"
  run_case mp3-decode-http "$HTTP_URL" "$repetition" --http
  run_case mp3-seek-local "$FIXTURES/reference.mp3" "$repetition" --seek
  run_case mp3-seek-http "$HTTP_URL" "$repetition" --http --seek
done

jq -e -s \
  --argjson expected "$((REPETITIONS * 7))" '
    length == $expected and
    all(.[]; .output_units > 0 and .peak_rss_kib > 0 and .threads >= 1) and
    all(.[] | select(.seek_latency_ms == null); .realtime_multiple > 1.0) and
    all(.[] | select(.seek_latency_ms != null); .seek_latency_ms.samples == 10)
  ' "$RESULT_FILE" >/dev/null

RANGE_REOPEN_COUNT="$(grep -Ec 'fixture_response status=206 range_start=[1-9][0-9]*' "$HTTP_LOG" || true)"
readonly RANGE_REOPEN_COUNT
if [[ "$RANGE_REOPEN_COUNT" -eq 0 ]]; then
  printf 'Mantle HTTP benchmark did not reopen a nonzero byte range.\n' >&2
  exit 1
fi

jq -s '
  def median: sort | .[length / 2 | floor];
  def p95: sort | .[((length * 95 + 99) / 100 | floor) - 1];
  group_by(.workload) |
  {
    schema_version: 1,
    workloads: map({
      workload: .[0].workload,
      repetitions: length,
      codec: .[0].codec,
      input_mode: .[0].input_mode,
      median_load_ms: (map(.load_elapsed_ms) | median),
      p95_first_output_ms: (map(.first_output_elapsed_ms) | p95),
      median_processing_ms: (map(.processing_elapsed_ms) | median),
      median_cpu_ms: (map(.cpu_time_ms) | median),
      median_realtime_multiple: (map(select(.realtime_multiple != null) | .realtime_multiple) | if length == 0 then null else median end),
      p95_seek_ms: (map(select(.seek_latency_ms != null) | .seek_latency_ms.p95) | if length == 0 then null else p95 end),
      p95_peak_rss_kib: (map(.peak_rss_kib) | p95),
      p95_current_pss_kib: (map(.current_pss_kib) | p95),
      max_threads: (map(.threads) | max),
      checksum: .[0].checksum,
      output_units: .[0].output_units
    })
  }
' "$RESULT_FILE" > "$SUMMARY_FILE"

jq -n \
  --slurpfile build "$BUILD_RESULT" \
  --arg measured_at "$(date --iso-8601=seconds)" \
  --arg kernel "$(uname -srmo)" \
  --arg cpu_model "$(lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -n 1)" \
  --argjson logical_cpus "$(nproc)" \
  --arg rustc "$(rustc --version)" \
  --arg cargo "$(cargo --version)" \
  --arg profile "release" \
  --arg commit "$SOURCE_COMMIT" \
  --argjson worktree_clean "$SOURCE_WORKTREE_CLEAN" \
  --argjson repetitions "$REPETITIONS" \
  --argjson http_nonzero_range_responses "$RANGE_REOPEN_COUNT" \
  --argjson binary_bytes "$BINARY_BYTES" \
  --arg wav_sha256 "$(sha256sum "$FIXTURES/reference.wav" | cut -d ' ' -f 1)" \
  --arg mp3_sha256 "$(sha256sum "$FIXTURES/reference.mp3" | cut -d ' ' -f 1)" \
  --arg aac_sha256 "$(sha256sum "$FIXTURES/reference.m4a" | cut -d ' ' -f 1)" \
  --arg opus_sha256 "$(sha256sum "$FIXTURES/reference.webm" | cut -d ' ' -f 1)" '
  {
    schema_version: 1,
    measured_at: $measured_at,
    host: {kernel: $kernel, cpu_model: $cpu_model, logical_cpus: $logical_cpus},
    toolchain: {rustc: $rustc, cargo: $cargo, profile: $profile},
    source: {commit: $commit, worktree_clean: $worktree_clean},
    repetitions: $repetitions,
    http_nonzero_range_responses: $http_nonzero_range_responses,
    clean_build: ($build[0] + {benchmark_binary_bytes: $binary_bytes}),
    fixtures: {
      wav_sha256: $wav_sha256,
      mp3_sha256: $mp3_sha256,
      aac_lc_sha256: $aac_sha256,
      opus_sha256: $opus_sha256
    }
  }
' > "$METADATA_FILE"

printf 'wrote %s runs, summary, and metadata under %s\n' "$((REPETITIONS * 7))" "$RESULTS_DIR"
