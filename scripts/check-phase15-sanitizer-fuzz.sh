#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-sanitizer-fuzz.json"
result_root="$ROOT/target/phase15/sanitizer-fuzz"
while (( $# > 0 )); do
  case "$1" in
    --plan)
      (( $# >= 2 )) || exit 2
      plan="$2"
      shift 2
      ;;
    --results-root)
      (( $# >= 2 )) || exit 2
      result_root="$2"
      shift 2
      ;;
    *)
      printf 'Usage: %s [--plan PATH] [--results-root PATH]\n' "$0" >&2
      exit 2
      ;;
  esac
done
readonly PLAN="$plan"
readonly HARDENING="$ROOT/compatibility/phase15-hardening-plan.json"
readonly RESULT_ROOT="$result_root"

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and .slice == "phase15-sanitizer-fuzz" and
  .plan == "compatibility/phase15-sanitizer-fuzz.json" and .targets == 7 and
  .address_runs_per_target == 128 and .leak_runs_per_target == 128 and
  .thread_runs_per_target == 8 and .miri_tests == 20 and .active_blockers == []
' "$RESULT_ROOT/result.json" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and
  .slice == "phase15-sanitizer-fuzz" and
  .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  .toolchain.rust == "nightly-2026-08-10" and
  .toolchain.cargo_fuzz == "0.13.2" and
  (.targets | sort) == ["local_adts", "local_flac", "local_matroska", "local_mp3", "local_mp4", "local_ogg", "local_wav"] and
  .bounds.targets == 7 and .bounds.max_len_bytes == 262144 and
  .bounds.timeout_seconds == 5 and .bounds.rss_limit_mb == 2048 and
  .bounds.address_runs_per_target == 128 and .bounds.leak_runs_per_target == 128 and
  .bounds.thread_runs_per_target == 8 and
  .campaigns.address.status == "PASS" and .campaigns.address.runs_per_target == 128 and
  .campaigns.leak.status == "PASS" and .campaigns.leak.runs_per_target == 128 and
  .campaigns.leak.mode == "AddressSanitizer with detect_leaks=1" and
  .campaigns.thread.status == "PASS" and .campaigns.thread.runs_per_target == 8 and
  .campaigns.thread.build_std == true and
  .campaigns.miri.status == "PASS" and .campaigns.miri.tests == 20 and
  .active_blockers == [] and .next_slice == "phase15-concurrency-lifecycle"
' "$PLAN" >/dev/null

jq --exit-status '
  .schema_version == 1 and (.status == "IN_PROGRESS" or .status == "COMPLETE") and
  .phase == "phase15-hardening" and
  (.campaigns[] | select(.id == "fuzzing") | .status) == "PASS" and
  (.campaigns[] | select(.id == "asan_lsan_tsan") | .status) == "PASS"
' "$HARDENING" >/dev/null

for log in asan-lsan.log tsan.log miri.log; do
  test -s "$RESULT_ROOT/$log"
done
[[ "$(rg -c 'Fuzz smoke:' "$RESULT_ROOT/asan-lsan.log")" == 7 ]]
[[ "$(rg -c 'Fuzz smoke:' "$RESULT_ROOT/tsan.log")" == 7 ]]
[[ "$(rg -c 'Done 128 runs in' "$RESULT_ROOT/asan-lsan.log")" == 7 ]]
[[ "$(rg -c 'Done 8 runs in' "$RESULT_ROOT/tsan.log")" == 7 ]]
grep --fixed-strings 'All local-media fuzz smoke targets passed.' "$RESULT_ROOT/asan-lsan.log" >/dev/null
grep --fixed-strings 'All local-media fuzz smoke targets passed.' "$RESULT_ROOT/tsan.log" >/dev/null
grep --fixed-strings 'test result: ok. 20 passed; 0 failed' "$RESULT_ROOT/miri.log" >/dev/null
if rg --fixed-strings 'AddressSanitizer:DEADLYSIGNAL' "$RESULT_ROOT/asan-lsan.log" >/dev/null ||
   rg --fixed-strings 'ThreadSanitizer: data race' "$RESULT_ROOT/tsan.log" >/dev/null; then
  printf 'Phase 15 sanitizer evidence contains a sanitizer finding.\n' >&2
  exit 1
fi

printf 'Phase 15 sanitizer/fuzz slice passed: 7 targets, ASan+LSan, TSan, and 20 Miri tests.\n'
