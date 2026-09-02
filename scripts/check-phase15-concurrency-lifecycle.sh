#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-concurrency-lifecycle.json"
result_root="$ROOT/target/phase15/concurrency-lifecycle"
while (( $# > 0 )); do
  case "$1" in
    --plan) (( $# >= 2 )) || exit 2; plan="$2"; shift 2 ;;
    --results-root) (( $# >= 2 )) || exit 2; result_root="$2"; shift 2 ;;
    *) printf 'Usage: %s [--plan PATH] [--results-root PATH]\n' "$0" >&2; exit 2 ;;
  esac
done
readonly PLAN="$plan"
readonly RESULT_ROOT="$result_root"
readonly HARDENING="$ROOT/compatibility/phase15-hardening-plan.json"
jq --exit-status '.schema_version == 1 and .status == "PASS" and .slice == "phase15-concurrency-lifecycle" and .plan == "compatibility/phase15-concurrency-lifecycle.json" and .loom_models == 1 and .loom_status == "PASS" and .real_thread_workers == 4 and .real_thread_attempts_per_worker == 128 and .jni.explicit_wrappers == 2048 and .jni.gc_wrappers == 1024 and .jni.classloader_collected == true and .jni.leak_manager == "PASS" and .jni.dispatcher_exit == "PASS" and .active_blockers == []' "$RESULT_ROOT/result.json" >/dev/null
jq --exit-status '.schema_version == 1 and .status == "COMPLETE" and .slice == "phase15-concurrency-lifecycle" and .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and .bounds.loom_models == 1 and .bounds.real_thread_workers == 4 and .bounds.real_thread_attempts_per_worker == 128 and .bounds.jni_explicit_wrappers == 2048 and .bounds.jni_gc_wrappers == 1024 and .campaigns.loom.status == "PASS" and .campaigns.real_thread_queue.status == "PASS" and .campaigns.jni_leak_stress.status == "PASS" and .campaigns.jni_leak_stress.classloader_collected == true and .active_blockers == [] and .next_slice == "phase15-dependency-audit"' "$PLAN" >/dev/null
jq --exit-status '.schema_version == 1 and (.status == "IN_PROGRESS" or .status == "COMPLETE") and .phase == "phase15-hardening" and (.campaigns[] | select(.id == "loom") | .status) == "PASS" and (.campaigns[] | select(.id == "jni_leak_stress") | .status) == "PASS"' "$HARDENING" >/dev/null
for log in loom-queue.log core-lifecycle.log jni-lifecycle.log; do test -s "$RESULT_ROOT/$log"; done
grep --fixed-strings 'phase15_bounded_release_queue_survives_concurrent_workers_and_shutdown ... ok' "$RESULT_ROOT/loom-queue.log" >/dev/null
grep --fixed-strings 'phase15_loom_terminal_release_is_exactly_once ... ok' "$RESULT_ROOT/loom-queue.log" >/dev/null
grep --fixed-strings 'test result: ok.' "$RESULT_ROOT/core-lifecycle.log" >/dev/null
grep --fixed-strings '"probe":"lifetime"' "$RESULT_ROOT/jni-lifecycle.log" >/dev/null
grep --fixed-strings '"explicit_wrappers":2048' "$RESULT_ROOT/jni-lifecycle.log" >/dev/null
grep --fixed-strings '"gc_wrappers":1024' "$RESULT_ROOT/jni-lifecycle.log" >/dev/null
grep --fixed-strings '"probe":"classloader","collected":true' "$RESULT_ROOT/jni-lifecycle.log" >/dev/null
grep --fixed-strings 'JNI lifecycle probes passed.' "$RESULT_ROOT/jni-lifecycle.log" >/dev/null
printf 'Phase 15 concurrency/lifecycle slice passed: Loom, bounded queue stress, and JNI lifecycle probes.\n'
