#!/usr/bin/env bash
set -euo pipefail

unset ARGV0 || true

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly FIXTURE="$ROOT/.cache/performance/fixtures/reference.webm"
readonly RESULTS_DIR="$ROOT/docs/performance/results"
readonly REPETITIONS="${REPETITIONS:-3}"
readonly WARMUP_SECONDS="${WARMUP_SECONDS:-3}"
readonly MEASURE_SECONDS="${MEASURE_SECONDS:-8}"
readonly QUEUE_CAPACITY="${QUEUE_CAPACITY:-50}"
readonly TRACKS_PER_SHARED_WORKER=25
HOST_CPUS="$(nproc)"
readonly HOST_CPUS
if (( HOST_CPUS > 256 )); then
  DEFAULT_WORKERS=256
else
  DEFAULT_WORKERS=$HOST_CPUS
fi
readonly WORKERS="${WORKERS:-$DEFAULT_WORKERS}"
readonly TRACK_COUNTS="${TRACK_COUNTS:-1 10 50 100 250}"
readonly SYNTHETIC_WORK="${SYNTHETIC_WORK:-2000}"
readonly RESULT_FILE="${RESULT_FILE:-$RESULTS_DIR/mantle-worker-phase8-2026-08-13.jsonl}"
readonly SUMMARY_FILE="${SUMMARY_FILE:-$RESULTS_DIR/mantle-worker-phase8-2026-08-13-summary.json}"
readonly METADATA_FILE="${METADATA_FILE:-$RESULTS_DIR/mantle-worker-phase8-2026-08-13-metadata.json}"
readonly BUILD_RESULT="$ROOT/.cache/mantle-worker-phase8-build.json"
readonly HTTP_ADDRESS="127.0.0.1:18081"
readonly HTTP_URL="http://$HTTP_ADDRESS/reference.mp3"

if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
  printf 'worker architecture benchmark requires a clean worktree\n' >&2
  exit 1
fi
for command in cargo cmake curl jq nproc /usr/bin/time; do
  if ! command -v "$command" >/dev/null; then
    printf 'worker architecture benchmark requires %s\n' "$command" >&2
    exit 1
  fi
done
if [[ ! -f "$FIXTURE" ]]; then
  "$ROOT/scripts/generate-benchmark-fixtures.sh" >/dev/null
fi

mkdir -p "$RESULTS_DIR" "$ROOT/.cache"
BUILD_TARGET="$(mktemp -d "$ROOT/.cache/mantle-worker-build.XXXXXX")"
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
  cargo build --release --locked -p mantle-worker-bench -p mantle-media-bench
readonly BENCH="$BUILD_TARGET/release/mantle-worker-bench"
readonly SERVER="$BUILD_TARGET/release/mantle-media-bench"
BINARY_BYTES="$(stat -c '%s' "$BENCH")"
readonly BINARY_BYTES

: > "$RESULT_FILE"
"$SERVER" serve --root "$ROOT/.cache/performance/fixtures" --address "$HTTP_ADDRESS" \
  > /dev/null 2> "$ROOT/.cache/mantle-worker-phase8-http.log" &
SERVER_PID=$!
for _ in {1..50}; do
  if curl --fail --silent --head "$HTTP_URL" >/dev/null; then
    break
  fi
  sleep 0.1
done
curl --fail --silent --head "$HTTP_URL" >/dev/null

run_case() {
  local architecture=$1
  local workload=$2
  local tracks=$3
  local repetition=$4
  local -a input_args=()
  case "$workload" in
    opus-passthrough-local) input_args=(--input "$FIXTURE") ;;
    mp3-decode-local|mp3-equalizer-local) input_args=(--input "$ROOT/.cache/performance/fixtures/reference.mp3") ;;
    aac-decode-local) input_args=(--input "$ROOT/.cache/performance/fixtures/reference.m4a") ;;
    mp3-decode-http) input_args=(--input "$HTTP_URL") ;;
    flac-decode-local) input_args=(--input "$ROOT/.cache/performance/fixtures/reference.flac") ;;
  esac
  "$BENCH" \
    --architecture "$architecture" \
    --workload "$workload" \
    --tracks "$tracks" \
    --workers "$WORKERS" \
    --queue-capacity "$QUEUE_CAPACITY" \
    --warmup-seconds "$WARMUP_SECONDS" \
    --measure-seconds "$MEASURE_SECONDS" \
    --synthetic-work "$SYNTHETIC_WORK" \
    --repetition "$repetition" \
    "${input_args[@]}" >> "$RESULT_FILE"
}

read -r -a scales <<< "$TRACK_COUNTS"
readonly architectures=(dedicated shared-pool hybrid)
readonly workloads=(synthetic opus-passthrough-local mp3-decode-local aac-decode-local flac-decode-local mp3-equalizer-local mp3-decode-http)
for repetition in $(seq 1 "$REPETITIONS"); do
  if (( repetition % 2 == 1 )); then
    order=(dedicated shared-pool hybrid)
  else
    order=(hybrid shared-pool dedicated)
  fi
  for tracks in "${scales[@]}"; do
    for workload in "${workloads[@]}"; do
      for architecture in "${order[@]}"; do
        run_case "$architecture" "$workload" "$tracks" "$repetition"
      done
    done
  done
done

readonly EXPECTED_RUNS="$((REPETITIONS * ${#scales[@]} * ${#architectures[@]} * ${#workloads[@]}))"
jq -e -s \
  --argjson expected "$EXPECTED_RUNS" \
  --argjson repetitions "$REPETITIONS" \
  --argjson warmup_seconds "$WARMUP_SECONDS" \
  --argjson measure_seconds "$MEASURE_SECONDS" \
  --argjson workers "$WORKERS" \
  --argjson tracks_per_shared_worker "$TRACKS_PER_SHARED_WORKER" '
    length == $expected and
    all(.[];
      .frames_requested == (.tracks * $measure_seconds * 50) and
      .worker_threads == (if .architecture == "dedicated" then .tracks else
        ([((.tracks + $tracks_per_shared_worker - 1) / $tracks_per_shared_worker | floor), $workers] | min) end) and
      (.frames_delivered + .frame_underruns) == .frames_requested and
      .timestamp_regressions == 0 and
      .timestamp_discontinuities == (if .workload == "opus-passthrough-local" then .tracks else 0 end) and
      .consumed_frames_per_track.min >= (1 + ($warmup_seconds * 50)) and
      .consumed_frames_per_track.max <= (1 + (($warmup_seconds + $measure_seconds) * 50)) and
      .pss_kib.samples > 0 and
      .threads.max >= 2
    ) and
    (group_by([.workload, .tracks, .architecture]) | all(.[]; length == $repetitions)) and
    (group_by([.workload, .tracks, .repetition]) |
      all(.[]; ([.[] | select(.frame_underruns == 0) | .checksum] | unique | length) == 1))
  ' "$RESULT_FILE" >/dev/null

jq -s '
  def median: sort | .[length / 2 | floor];
  def p95: sort | .[((length * 95 + 99) / 100 | floor) - 1];
  {
    schema_version: 1,
    cases: (
      group_by([.workload, .tracks, .architecture]) |
      map({
        workload: .[0].workload,
        tracks: .[0].tracks,
        architecture: .[0].architecture,
        repetitions: length,
        worker_threads: .[0].worker_threads,
        queue_capacity: .[0].queue_capacity,
        median_cpu_core_percent: (map(.cpu_core_percent) | median),
        median_p95_pss_kib: (map(.pss_kib.p95) | median),
        median_p95_rss_kib: (map(.rss_kib.p95) | median),
        max_threads: (map(.threads.max) | max),
        p95_first_frame_latency_ms: (map(.first_frame_latency_ms.p95) | p95),
        median_involuntary_context_switches: (map(.involuntary_context_switches) | median),
        median_queue_depth: (map(.queue_depth.median) | median),
        minimum_queue_depth: (map(.queue_depth.min) | min),
        frame_underruns: (map(.frame_underruns) | add),
        skipped_deadlines: (map(.skipped_deadlines) | add),
        checksum: .[0].checksum
      })
    )
  }
  ' "$RESULT_FILE" > "$SUMMARY_FILE"

jq -n \
  --slurpfile build "$BUILD_RESULT" \
  --arg measured_at "$(date --iso-8601=seconds)" \
  --arg kernel "$(uname -srmo)" \
  --arg cpu_model "$(lscpu | sed -n 's/^Model name:[[:space:]]*//p' | head -n 1)" \
  --arg cpu_governor "$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || printf unknown)" \
  --argjson logical_cpus "$(nproc)" \
  --arg rustc "$(rustc --version)" \
  --arg cargo "$(cargo --version)" \
  --arg commit "$(git -C "$ROOT" rev-parse HEAD)" \
  --argjson repetitions "$REPETITIONS" \
  --argjson warmup_seconds "$WARMUP_SECONDS" \
  --argjson measure_seconds "$MEASURE_SECONDS" \
  --argjson queue_capacity "$QUEUE_CAPACITY" \
  --argjson workers "$WORKERS" \
  --argjson tracks_per_shared_worker "$TRACKS_PER_SHARED_WORKER" \
  --arg scales "$TRACK_COUNTS" \
  --argjson synthetic_work "$SYNTHETIC_WORK" \
  --argjson binary_bytes "$BINARY_BYTES" \
  --arg opus_sha256 "$(sha256sum "$FIXTURE" | cut -d ' ' -f 1)" \
  --arg mp3_sha256 "$(sha256sum "$ROOT/.cache/performance/fixtures/reference.mp3" | cut -d ' ' -f 1)" \
  --arg aac_sha256 "$(sha256sum "$ROOT/.cache/performance/fixtures/reference.m4a" | cut -d ' ' -f 1)" \
  --arg flac_sha256 "$(sha256sum "$ROOT/.cache/performance/fixtures/reference.flac" | cut -d ' ' -f 1)" '
  {
    schema_version: 1,
    measured_at: $measured_at,
    host: {
      kernel: $kernel,
      cpu_model: $cpu_model,
      logical_cpus: $logical_cpus,
      cpu_governor: $cpu_governor
    },
    toolchain: {rustc: $rustc, cargo: $cargo, profile: "release"},
    source: {commit: $commit, worktree_clean: true},
    benchmark: {
      repetitions: $repetitions,
      warmup_seconds: $warmup_seconds,
      measure_seconds: $measure_seconds,
      queue_capacity: $queue_capacity,
      shared_worker_limit: $workers,
      tracks_per_shared_worker: $tracks_per_shared_worker,
      track_counts: ($scales | split(" ") | map(tonumber)),
      synthetic_work_iterations: $synthetic_work,
      architecture_order: "forward on odd repetitions; reverse on even repetitions",
      idle_wait: "consumer-driven park/unpark"
    },
    clean_build: ($build[0] + {benchmark_binary_bytes: $binary_bytes}),
    fixtures: {
      opus_webm_sha256: $opus_sha256,
      mp3_sha256: $mp3_sha256,
      aac_lc_mp4_sha256: $aac_sha256,
      flac_sha256: $flac_sha256
    }
  }
  ' > "$METADATA_FILE"

printf 'wrote %s runs, summary, and metadata under %s\n' "$EXPECTED_RUNS" "$RESULTS_DIR"
