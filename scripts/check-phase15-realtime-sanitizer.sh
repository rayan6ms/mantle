#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-realtime-sanitizer.json"
result_root="$ROOT/target/phase15/realtime-sanitizer"
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

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and .slice == "phase15-realtime-sanitizer" and
  .plan == "compatibility/phase15-realtime-sanitizer.json" and
  .toolchain.rust == "1.97.1" and .toolchain.clang == "22.1.8" and
  .toolchain.compiler_rt == "22.1.8" and
  .toolchain.container == "localhost/mantle-phase15-rtsan:fedora44-clang22" and
  .toolchain.container_image_id == "7f06e2cb3103e8e13f41f0e949c09a04ecbfcb5f724b76c671b7fc60641e2bff" and
  .hot_path.status == "PASS" and .hot_path.iterations == 8192 and
  .hot_path.queue_capacity == 8 and .hot_path.packet_bytes == 3 and
  (.hot_path.checksum | test("^[1-9][0-9]*$")) and
  .hot_path.realtime_errors == 0 and .hot_path.unique_errors == 0 and
  .negative_control.status == "DETECTED" and .negative_control.exit_code != 0 and
  .negative_control.exit_code != 124 and .negative_control.finding == "unsafe-library-call" and
  .negative_control.unsafe_function == "malloc" and .negative_control.realtime_errors == 1 and
  .negative_control.unique_errors == 1 and .active_blockers == []
' "$RESULT_ROOT/result.json" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and .slice == "phase15-realtime-sanitizer" and
  .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  .toolchain.rust == "1.97.1" and .toolchain.clang == "22.1.8" and
  .toolchain.compiler_rt == "22.1.8" and
  .bounds == {callbacks: 1, iterations: 8192, queue_capacity: 8, packet_bytes: 3,
    timeout_seconds: 30, negative_controls: 1} and
  .campaigns.mantle_hot_path.status == "PASS" and
  .campaigns.mantle_hot_path.entry == "clang::nonblocking C++ callback into Rust" and
  .campaigns.mantle_hot_path.realtime_errors == 0 and
  .campaigns.allocating_negative_control.status == "DETECTED" and
  .campaigns.allocating_negative_control.unsafe_function == "malloc" and
  .campaigns.allocating_negative_control.realtime_errors == 1 and
  .active_blockers == [] and
  .evidence.checker == "scripts/check-phase15-realtime-sanitizer.sh" and
  .evidence.result == "target/phase15/realtime-sanitizer/result.json" and
  .next_slice == "phase15-native-soak"
' "$PLAN" >/dev/null

jq --exit-status '
  .schema_version == 1 and (.status | IN("IN_PROGRESS", "COMPLETE")) and
  .phase == "phase15-hardening" and
  (.completed_slice | IN("phase15-realtime-sanitizer", "phase15-native-soak",
    "phase15-replay-fault-soak", "phase15-hardening-exit")) and
  (.campaigns[] | select(.id == "realtime_sanitizer") | .status) == "PASS" and
  .evidence.active_blockers == [] and
  (.next_slice | IN("phase15-native-soak", "phase15-replay-fault-soak",
    "phase15-hardening-exit", null))
' "$HARDENING" >/dev/null

grep --fixed-strings 'probe_status=PASS iterations=8192' "$RESULT_ROOT/positive.log" >/dev/null
grep --fixed-strings 'Total error count: 0' "$RESULT_ROOT/positive.log" >/dev/null
grep --fixed-strings 'Unique error count: 0' "$RESULT_ROOT/positive.log" >/dev/null
if grep --fixed-strings 'ERROR: RealtimeSanitizer' "$RESULT_ROOT/positive.log" >/dev/null; then
  printf 'Phase 15 realtime hot-path evidence contains an RTSan finding.\n' >&2
  exit 1
fi
grep --fixed-strings 'ERROR: RealtimeSanitizer: unsafe-library-call' "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings "unsafe function \`malloc\`" "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings 'Total error count: 1' "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings 'Unique error count: 1' "$RESULT_ROOT/negative.log" >/dev/null
grep --fixed-strings 'clang version 22.1.8' "$RESULT_ROOT/clang-version.log" >/dev/null
grep --fixed-strings 'compiler-rt-22.1.8-4.fc44.x86_64' "$RESULT_ROOT/compiler-rt-version.log" >/dev/null
grep --fixed-strings 'clang-22.1.8-4.fc44' "$ROOT/scripts/phase15-realtime-sanitizer.Containerfile" >/dev/null
grep --fixed-strings 'compiler-rt-22.1.8-4.fc44' "$ROOT/scripts/phase15-realtime-sanitizer.Containerfile" >/dev/null
grep --fixed-strings '[[clang::nonblocking]]' "$ROOT/scripts/phase15-realtime-sanitizer.cpp.txt" >/dev/null

printf 'Phase 15 RealtimeSanitizer slice passed: 8,192 hot-path iterations and one detected control.\n'
