#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RESULT_ROOT="${PHASE15_RESULTS_ROOT:-$ROOT/target/phase15/replay-fault-soak-smoke-manual}"
readonly BUILD_TARGET="${PHASE15_BUILD_TARGET:-$ROOT/target/phase15/native-soak-build}"
readonly RUN_DURATION_SECONDS="${MANTLE_REPLAY_FAULT_SECONDS:-3}"
readonly CHECKPOINT_SECONDS="${MANTLE_REPLAY_FAULT_CHECKPOINT_SECONDS:-1}"
readonly CYCLE_DELAY_MS="${MANTLE_REPLAY_FAULT_CYCLE_DELAY_MS:-0}"
readonly MAX_SMOKE_DURATION_SECONDS=300
readonly MAX_MEMORY_GROWTH_KIB=16384
readonly MAX_THREADS=2
readonly PROGRESS="$RESULT_ROOT/progress.jsonl"
readonly RESULT="$RESULT_ROOT/result.json"
readonly METADATA="$RESULT_ROOT/run-metadata.json"

unset APPIMAGE APPDIR ARGV0 || true

for value in "$RUN_DURATION_SECONDS" "$CHECKPOINT_SECONDS" "$CYCLE_DELAY_MS"; do
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    printf 'Replay/fault soak bounds must be unsigned integers: %s\n' "$value" >&2
    exit 2
  fi
done
if (( RUN_DURATION_SECONDS > MAX_SMOKE_DURATION_SECONDS )); then
  printf 'Long replay/fault campaigns are retired; smoke duration may not exceed %s seconds.\n' \
    "$MAX_SMOKE_DURATION_SECONDS" >&2
  exit 2
fi
for evidence in "$PROGRESS" "$RESULT" "$METADATA"; do
  if [[ -e "$evidence" ]]; then
    printf 'Refusing to overwrite replay/fault soak evidence: %s\n' "$evidence" >&2
    exit 1
  fi
done

toolchain_link_dir=""
run_started=false
run_finalized=false
cleanup() {
  exit_status=$?
  if [[ "$run_started" == true && "$run_finalized" == false && -f "$METADATA" ]]; then
    interrupted_at="$(date --utc --iso-8601=seconds)"
    if jq --arg interrupted_at "$interrupted_at" --argjson exit_code "$exit_status" \
      '.status = "INCOMPLETE" | .finished_at = $interrupted_at | .exit_code = $exit_code' \
      "$METADATA" >"$METADATA.tmp"; then
      mv "$METADATA.tmp" "$METADATA"
    fi
  fi
  if [[ -n "$toolchain_link_dir" && -d "$toolchain_link_dir" ]]; then
    [[ ! -L "$toolchain_link_dir/libstdc++.so" ]] || unlink "$toolchain_link_dir/libstdc++.so"
    rmdir "$toolchain_link_dir"
  fi
}
trap cleanup EXIT

if ! command -v c++ >/dev/null 2>&1 &&
  [[ -x "$ROOT/.cache/media-toolchains/xaac-root/usr/bin/c++" ]]; then
  export CC="$ROOT/.cache/media-toolchains/xaac-root/usr/bin/cc"
  export CXX="$ROOT/.cache/media-toolchains/xaac-root/usr/bin/c++"
  export PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/bin:$PATH"
  export LD_LIBRARY_PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  if [[ ! -e "$ROOT/.cache/media-toolchains/xaac-root/usr/lib64/libstdc++.so" &&
    -e /usr/lib64/libstdc++.so.6 ]]; then
    mkdir -p "$ROOT/.cache"
    toolchain_link_dir="$(mktemp -d "$ROOT/.cache/replay-soak-toolchain.XXXXXX")"
    ln -s /usr/lib64/libstdc++.so.6 "$toolchain_link_dir/libstdc++.so"
    export LIBRARY_PATH="$toolchain_link_dir${LIBRARY_PATH:+:$LIBRARY_PATH}"
  fi
fi

mkdir -p "$RESULT_ROOT" "$BUILD_TARGET"
(cd "$ROOT" && CARGO_TARGET_DIR="$BUILD_TARGET" \
  cargo build --locked --release -p mantle-media-bench --bin mantle-replay-fault-soak) \
  >"$RESULT_ROOT/rust-build.log" 2>&1
readonly BENCH="$BUILD_TARGET/release/mantle-replay-fault-soak"

started_at="$(date --utc --iso-8601=seconds)"
started_epoch="$(date --utc +%s)"
expected_finish_at="$(date --utc --date="@$((started_epoch + RUN_DURATION_SECONDS))" --iso-8601=seconds)"
source_commit="$(git -C "$ROOT" rev-parse HEAD)"
if [[ -z "$(git -C "$ROOT" status --porcelain)" ]]; then
  worktree_clean=true
else
  worktree_clean=false
fi
campaign_mode=smoke
if [[ -n "${INVOCATION_ID:-}" ]]; then
  supervisor=systemd-user
else
  supervisor=interactive
fi

jq -n \
  --arg started_at "$started_at" \
  --arg expected_finish_at "$expected_finish_at" \
  --arg campaign_mode "$campaign_mode" \
  --arg supervisor "$supervisor" \
  --arg invocation_id "${INVOCATION_ID:-}" \
  --argjson runner_pid "$$" \
  --argjson duration "$RUN_DURATION_SECONDS" \
  --argjson checkpoint "$CHECKPOINT_SECONDS" \
  --argjson delay "$CYCLE_DELAY_MS" \
  '{schema_version: 1, status: "RUNNING", slice: "phase15-replay-fault-soak",
    campaign_mode: $campaign_mode, runner_pid: $runner_pid, started_at: $started_at,
    expected_finish_at: $expected_finish_at, supervisor: $supervisor,
    invocation_id: $invocation_id,
    bounds: {duration_seconds: $duration, checkpoint_seconds: $checkpoint,
      cycle_delay_ms: $delay}}' >"$METADATA"
run_started=true

: >"$PROGRESS"
set +e
timeout --foreground --signal=TERM --kill-after=30s "$((RUN_DURATION_SECONDS + 300))s" \
  "$BENCH" \
    --duration-seconds "$RUN_DURATION_SECONDS" \
    --checkpoint-seconds "$CHECKPOINT_SECONDS" \
    --cycle-delay-ms "$CYCLE_DELAY_MS" | tee "$PROGRESS"
soak_status=${PIPESTATUS[0]}
set -e
if [[ "$soak_status" -ne 0 ]]; then
  printf 'Phase 15 replay/fault soak failed with exit code %s.\n' "$soak_status" >&2
  exit "$soak_status"
fi

harness_result="$(jq -c -s '
  [.[] | select(.kind == "result")] |
  if length == 1 then .[0] else error("expected exactly one replay/fault result") end
' "$PROGRESS")"
finished_at="$(date --utc --iso-8601=seconds)"
rust_version="$(rustc --version | awk '{print $2}')"

if jq --exit-status \
  --argjson memory_limit "$MAX_MEMORY_GROWTH_KIB" \
  --argjson max_threads "$MAX_THREADS" '
    .status == "PASS" and .scenarios_per_cycle == 9 and
    .expected_faults_per_cycle == 4 and .requests_per_cycle == 24 and
    .payload_bytes == 4096 and .payload_checksum == 2154433018067471136 and
    .cycles > 0 and .scenario_executions == (.cycles * 9) and
    .expected_faults == (.cycles * 4) and .unexpected_failures == 0 and
    .checkpoints >= 2 and .memory.rss_growth_kib <= $memory_limit and
    .memory.pss_growth_kib <= $memory_limit and .memory.max_threads <= $max_threads
  ' <<<"$harness_result" >/dev/null; then
  if [[ "$campaign_mode" == full ]]; then
    result_status=PASS
  else
    result_status=SMOKE_PASS
  fi
else
  result_status=FAIL
fi

jq -n \
  --arg status "$result_status" \
  --arg campaign_mode "$campaign_mode" \
  --arg started_at "$started_at" \
  --arg finished_at "$finished_at" \
  --arg expected_finish_at "$expected_finish_at" \
  --arg rust "$rust_version" \
  --arg source_commit "$source_commit" \
  --argjson worktree_clean "$worktree_clean" \
  --argjson harness "$harness_result" \
  '{schema_version: 1, status: $status, slice: "phase15-replay-fault-soak",
    campaign_mode: $campaign_mode, started_at: $started_at, finished_at: $finished_at,
    expected_finish_at: $expected_finish_at,
    toolchain: {rust: $rust, profile: "release"},
    source: {commit: $source_commit, worktree_clean: $worktree_clean},
    acceptance: {max_rss_growth_kib: 16384, max_pss_growth_kib: 16384,
      max_threads: 2},
    harness: $harness, active_blockers: []}' >"$RESULT"

jq --arg status "$result_status" --arg finished_at "$finished_at" \
  '.status = $status | .finished_at = $finished_at' "$METADATA" >"$METADATA.tmp"
mv "$METADATA.tmp" "$METADATA"
run_finalized=true

if [[ "$result_status" == FAIL ]]; then
  printf 'Phase 15 replay/fault soak failed its bounded oracle.\n' >&2
  exit 1
fi
printf 'Phase 15 replay/fault soak %s passed.\n' "$campaign_mode"
