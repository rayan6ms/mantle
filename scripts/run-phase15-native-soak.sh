#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly FIXTURE_ROOT="${PHASE15_FIXTURE_ROOT:-$ROOT/.cache/performance/fixtures}"
readonly RESULT_ROOT="${PHASE15_RESULTS_ROOT:-$ROOT/target/phase15/native-soak}"
readonly BUILD_TARGET="${PHASE15_BUILD_TARGET:-$ROOT/target/phase15/native-soak-build}"
readonly RUN_DURATION_SECONDS="${MANTLE_NATIVE_SOAK_SECONDS:-86400}"
readonly CHECKPOINT_SECONDS="${MANTLE_NATIVE_SOAK_CHECKPOINT_SECONDS:-60}"
readonly CYCLE_DELAY_MS="${MANTLE_NATIVE_SOAK_CYCLE_DELAY_MS:-1000}"
readonly FULL_DURATION_SECONDS=86400
readonly FULL_CHECKPOINT_SECONDS=60
readonly FULL_CYCLE_DELAY_MS=1000
readonly MAX_MEMORY_GROWTH_KIB=8192
readonly MAX_THREADS=1
readonly PROGRESS="$RESULT_ROOT/progress.jsonl"
readonly RESULT="$RESULT_ROOT/result.json"
readonly METADATA="$RESULT_ROOT/run-metadata.json"

unset APPIMAGE APPDIR ARGV0 || true

for value in "$RUN_DURATION_SECONDS" "$CHECKPOINT_SECONDS" "$CYCLE_DELAY_MS"; do
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    printf 'Native soak bounds must be unsigned integers: %s\n' "$value" >&2
    exit 2
  fi
done

for evidence in "$PROGRESS" "$RESULT" "$METADATA"; do
  if [[ -e "$evidence" ]]; then
    printf 'Refusing to overwrite native soak evidence: %s\n' "$evidence" >&2
    exit 1
  fi
done

declare -Ar EXPECTED_HASHES=(
  [reference.wav]='bf32b086fb9c2e34e68f17a3bcb9a9af9c267a931c680306cd4e52c1ed10304f'
  [reference.mp3]='1abf476602d54b3421a268d95620360890cf013e5ad3be612992b96781a099c3'
  [reference.m4a]='8a50ebb40d55a20ccc7ef4487a0f9e5426689f4f7ced16166cf6f2f5e986b833'
  [reference.flac]='827be0ee05f1ca087fd908b1d86e0e4af27868b90e4cc6a6f13b256ded24ea88'
  [reference.webm]='95506dac7bfc5ff3f4881e5e90799ca2ce7b5aa7783a4f3eeeaebc88701a0321'
)
for filename in reference.wav reference.mp3 reference.m4a reference.flac reference.webm; do
  fixture="$FIXTURE_ROOT/$filename"
  if [[ ! -f "$fixture" ]]; then
    printf 'Missing native soak fixture: %s\n' "$fixture" >&2
    exit 1
  fi
  actual_hash="$(sha256sum "$fixture" | awk '{print $1}')"
  if [[ "$actual_hash" != "${EXPECTED_HASHES[$filename]}" ]]; then
    printf 'Native soak fixture hash mismatch: %s\n' "$fixture" >&2
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

# The checked repository-local compiler fills the host's missing C++ driver. Its extracted
# libstdc++ development symlink can lag a host update, so link the current runtime for build time.
if ! command -v c++ >/dev/null 2>&1 &&
  [[ -x "$ROOT/.cache/media-toolchains/xaac-root/usr/bin/c++" ]]; then
  export CC="$ROOT/.cache/media-toolchains/xaac-root/usr/bin/cc"
  export CXX="$ROOT/.cache/media-toolchains/xaac-root/usr/bin/c++"
  export PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/bin:$PATH"
  export LD_LIBRARY_PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  if [[ ! -e "$ROOT/.cache/media-toolchains/xaac-root/usr/lib64/libstdc++.so" &&
    -e /usr/lib64/libstdc++.so.6 ]]; then
    mkdir -p "$ROOT/.cache"
    toolchain_link_dir="$(mktemp -d "$ROOT/.cache/native-soak-toolchain.XXXXXX")"
    ln -s /usr/lib64/libstdc++.so.6 "$toolchain_link_dir/libstdc++.so"
    export LIBRARY_PATH="$toolchain_link_dir${LIBRARY_PATH:+:$LIBRARY_PATH}"
  fi
fi

mkdir -p "$RESULT_ROOT" "$BUILD_TARGET"
(cd "$ROOT" && CARGO_TARGET_DIR="$BUILD_TARGET" \
  cargo build --locked --release -p mantle-media-bench) \
  >"$RESULT_ROOT/rust-build.log" 2>&1
readonly BENCH="$BUILD_TARGET/release/mantle-media-bench"

started_at="$(date --utc --iso-8601=seconds)"
started_epoch="$(date --utc +%s)"
expected_finish_at="$(date --utc --date="@$((started_epoch + RUN_DURATION_SECONDS))" --iso-8601=seconds)"
source_commit="$(git -C "$ROOT" rev-parse HEAD)"
if [[ -z "$(git -C "$ROOT" status --porcelain)" ]]; then
  worktree_clean=true
else
  worktree_clean=false
fi
if [[ "$RUN_DURATION_SECONDS" -eq "$FULL_DURATION_SECONDS" &&
  "$CHECKPOINT_SECONDS" -eq "$FULL_CHECKPOINT_SECONDS" &&
  "$CYCLE_DELAY_MS" -eq "$FULL_CYCLE_DELAY_MS" ]]; then
  campaign_mode=full
else
  campaign_mode=smoke
fi
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
  '{schema_version: 1, status: "RUNNING", slice: "phase15-native-soak",
    campaign_mode: $campaign_mode, runner_pid: $runner_pid, started_at: $started_at,
    expected_finish_at: $expected_finish_at, supervisor: $supervisor,
    invocation_id: $invocation_id,
    bounds: {duration_seconds: $duration, checkpoint_seconds: $checkpoint,
      cycle_delay_ms: $delay}}' >"$METADATA"
run_started=true

: >"$PROGRESS"
set +e
timeout --foreground --signal=TERM --kill-after=30s "$((RUN_DURATION_SECONDS + 300))s" \
  "$BENCH" soak \
    --fixture-root "$FIXTURE_ROOT" \
    --duration-seconds "$RUN_DURATION_SECONDS" \
    --checkpoint-seconds "$CHECKPOINT_SECONDS" \
    --cycle-delay-ms "$CYCLE_DELAY_MS" | tee "$PROGRESS"
soak_status=${PIPESTATUS[0]}
set -e
if [[ "$soak_status" -ne 0 ]]; then
  printf 'Phase 15 native soak failed with exit code %s.\n' "$soak_status" >&2
  exit "$soak_status"
fi

harness_result="$(jq -c -s '
  [.[] | select(.kind == "result")] |
  if length == 1 then .[0] else error("expected exactly one soak result") end
' "$PROGRESS")"
finished_at="$(date --utc --iso-8601=seconds)"
rust_version="$(rustc --version | awk '{print $2}')"

if jq --exit-status \
  --argjson memory_limit "$MAX_MEMORY_GROWTH_KIB" \
  --argjson max_threads "$MAX_THREADS" '
    .status == "PASS" and .workloads == 5 and .sessions > 0 and
    .fingerprint_mismatches == 0 and .checkpoints >= 2 and
    .memory.rss_growth_kib <= $memory_limit and
    .memory.pss_growth_kib <= $memory_limit and
    .memory.max_threads <= $max_threads and
    (.fingerprints | length) == 5
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
  '{schema_version: 1, status: $status, slice: "phase15-native-soak",
    campaign_mode: $campaign_mode, started_at: $started_at, finished_at: $finished_at,
    expected_finish_at: $expected_finish_at,
    toolchain: {rust: $rust, profile: "release"},
    source: {commit: $source_commit, worktree_clean: $worktree_clean},
    fixtures: [
      {file: "reference.wav", sha256: "bf32b086fb9c2e34e68f17a3bcb9a9af9c267a931c680306cd4e52c1ed10304f"},
      {file: "reference.mp3", sha256: "1abf476602d54b3421a268d95620360890cf013e5ad3be612992b96781a099c3"},
      {file: "reference.m4a", sha256: "8a50ebb40d55a20ccc7ef4487a0f9e5426689f4f7ced16166cf6f2f5e986b833"},
      {file: "reference.flac", sha256: "827be0ee05f1ca087fd908b1d86e0e4af27868b90e4cc6a6f13b256ded24ea88"},
      {file: "reference.webm", sha256: "95506dac7bfc5ff3f4881e5e90799ca2ce7b5aa7783a4f3eeeaebc88701a0321"}
    ],
    acceptance: {max_rss_growth_kib: 8192, max_pss_growth_kib: 8192, max_threads: 1},
    harness: $harness,
    active_blockers: []}' >"$RESULT"

jq --arg status "$result_status" --arg finished_at "$finished_at" \
  '.status = $status | .finished_at = $finished_at' "$METADATA" >"$METADATA.tmp"
mv "$METADATA.tmp" "$METADATA"
run_finalized=true

if [[ "$result_status" == FAIL ]]; then
  printf 'Phase 15 native soak failed its bounded resource oracle.\n' >&2
  exit 1
fi
printf 'Phase 15 native soak %s passed.\n' "$campaign_mode"
