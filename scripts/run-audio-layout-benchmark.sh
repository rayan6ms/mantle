#!/usr/bin/env bash
set -euo pipefail

unset ARGV0 || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RESULTS_DIR="$ROOT/docs/performance/results"
readonly REPETITIONS="${REPETITIONS:-9}"
readonly ITERATIONS="${ITERATIONS:-500000}"
readonly RESULT_FILE="${RESULT_FILE:-$RESULTS_DIR/mantle-audio-layout-2026-08-11.jsonl}"
readonly SUMMARY_FILE="${SUMMARY_FILE:-$RESULTS_DIR/mantle-audio-layout-2026-08-11-summary.json}"
readonly METADATA_FILE="${METADATA_FILE:-$RESULTS_DIR/mantle-audio-layout-2026-08-11-metadata.json}"

if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
  printf 'audio-layout benchmark requires a clean worktree\n' >&2
  exit 1
fi

mkdir -p "$RESULTS_DIR" "$ROOT/.cache"
BUILD_TARGET="$(mktemp -d "$ROOT/.cache/mantle-audio-layout-build.XXXXXX")"
readonly BUILD_TARGET
cleanup() {
  rm -rf "$BUILD_TARGET"
}
trap cleanup EXIT

CARGO_TARGET_DIR="$BUILD_TARGET" cargo build \
  --release --locked -p mantle-audio-layout-bench
readonly BENCH="$BUILD_TARGET/release/mantle-audio-layout-bench"

: > "$RESULT_FILE"
run_case() {
  local layout=$1
  local workload=$2
  local repetition=$3
  "$BENCH" \
    --layout "$layout" \
    --workload "$workload" \
    --repetition "$repetition" \
    --iterations "$ITERATIONS" >> "$RESULT_FILE"
}

for repetition in $(seq 1 "$REPETITIONS"); do
  for workload in stereo-volume stereo-channel-filter mono-to-stereo-volume; do
    if (( repetition % 2 == 1 )); then
      run_case interleaved "$workload" "$repetition"
      run_case planar "$workload" "$repetition"
    else
      run_case planar "$workload" "$repetition"
      run_case interleaved "$workload" "$repetition"
    fi
  done
done

jq -e -s \
  --argjson expected "$((REPETITIONS * 6))" \
  --argjson repetitions "$REPETITIONS" '
    length == $expected and
    all(.[]; .iterations > 0 and .nanoseconds_per_frame > 0) and
    (group_by([.workload, .layout]) | all(.[]; length == $repetitions)) and
    (group_by(.workload) | all(.[]; map(.checksum) | unique | length == 1))
  ' "$RESULT_FILE" >/dev/null

jq -s '
  def median: sort | .[length / 2 | floor];
  def p95: sort | .[((length * 95 + 99) / 100 | floor) - 1];
  {
    schema_version: 1,
    cases: (
      group_by([.workload, .layout]) |
      map({
        workload: .[0].workload,
        layout: .[0].layout,
        repetitions: length,
        iterations_per_repetition: .[0].iterations,
        min_nanoseconds_per_frame: (map(.nanoseconds_per_frame) | min),
        median_nanoseconds_per_frame: (map(.nanoseconds_per_frame) | median),
        p95_nanoseconds_per_frame: (map(.nanoseconds_per_frame) | p95),
        max_nanoseconds_per_frame: (map(.nanoseconds_per_frame) | max),
        checksum: .[0].checksum
      })
    )
  }
' "$RESULT_FILE" > "$SUMMARY_FILE"

jq -n \
  --arg measured_at "$(date --iso-8601=seconds)" \
  --arg kernel "$(uname -srmo)" \
  --arg cpu_model "$(lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -n 1)" \
  --argjson logical_cpus "$(nproc)" \
  --arg rustc "$(rustc --version)" \
  --arg cargo "$(cargo --version)" \
  --arg commit "$(git -C "$ROOT" rev-parse HEAD)" \
  --argjson repetitions "$REPETITIONS" \
  --argjson iterations "$ITERATIONS" '
  {
    schema_version: 1,
    measured_at: $measured_at,
    host: {kernel: $kernel, cpu_model: $cpu_model, logical_cpus: $logical_cpus},
    toolchain: {rustc: $rustc, cargo: $cargo, profile: "release"},
    source: {commit: $commit, worktree_clean: true},
    repetitions: $repetitions,
    iterations_per_repetition: $iterations,
    frame: {sample_rate: 48000, channels: 2, samples_per_channel: 960, duration_ms: 20},
    timing: "wall-clock monotonic Instant; layouts alternate first position by repetition"
  }
' > "$METADATA_FILE"

printf 'wrote %s runs, summary, and metadata under %s\n' \
  "$((REPETITIONS * 6))" "$RESULTS_DIR"
