#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly PLAN="$ROOT/compatibility/phase15-hardening-plan.json"
readonly LEDGER="$ROOT/PROJECT_LEDGER.md"
readonly OUT_ROOT="$ROOT/target/phase15/hardening-preflight"
readonly RESULT="$OUT_ROOT/result.json"

report_only=false
if (( $# > 1 )) || { (( $# == 1 )) && [[ "$1" != "--report-only" ]]; }; then
  printf 'Usage: %s [--report-only]\n' "$0" >&2
  exit 2
fi
if (( $# == 1 )); then
  report_only=true
fi

jq --exit-status '
  .schema_version == 1 and (.status | IN("IN_PROGRESS", "COMPLETE")) and
  .phase == "phase15-hardening" and
  (.completed_slice | IN("phase15-hardening-preflight", "phase15-sanitizer-fuzz",
    "phase15-concurrency-lifecycle", "phase15-dependency-audit",
    "phase15-realtime-sanitizer", "phase15-native-soak",
    "phase15-replay-fault-soak", "phase15-hardening-exit")) and
  ([.campaigns[].id] | sort) ==
    ["asan_lsan_tsan", "dependency_audit", "fuzzing", "jni_leak_stress", "loom",
     "miri", "native_soak", "property_tests", "realtime_sanitizer", "replay_fault_soak"] and
  ([.campaigns[].id] | unique | length) == (.campaigns | length) and
  (.evidence.checker | startswith("scripts/check-phase15-")) and
  (.evidence.result | startswith("target/phase15/")) and
  ((.evidence.active_blockers == ["B-001"]) or (.evidence.active_blockers == [])) and
  (.next_slice | IN("phase15-sanitizer-fuzz", "phase15-concurrency-lifecycle",
    "phase15-dependency-audit", "phase15-realtime-sanitizer", "phase15-native-soak",
    "phase15-replay-fault-soak", "phase15-hardening-exit", null))
' "$PLAN" >/dev/null
grep --fixed-strings '[B-001]' "$LEDGER" >/dev/null
grep --fixed-strings 'scripts/check-phase15-hardening-preflight.sh' "$LEDGER" >/dev/null

mkdir -p "$OUT_ROOT"
rust_status=0
set +e
(cd "$ROOT" && env -u APPIMAGE -u APPDIR cargo test --workspace --all-targets --all-features --locked) \
  >"$OUT_ROOT/rust-tests.log" 2>&1
rust_status=$?
set -e

fuzz_status=0
fuzz_reason=""
if ! command -v cargo-fuzz >/dev/null 2>&1; then
  fuzz_status=1
  fuzz_reason="cargo-fuzz 0.13.2 is not installed"
elif [[ "$(cargo-fuzz --version 2>&1)" != *"0.13.2"* ]]; then
  fuzz_status=1
  fuzz_reason="cargo-fuzz 0.13.2 is required"
else
  cxx="${CXX:-}"
  if [[ -z "$cxx" ]]; then
    cxx="$(command -v c++ 2>/dev/null || true)"
  fi
  cc="${CC:-}"
  if [[ -z "$cc" ]]; then
    cc="$(command -v cc 2>/dev/null || true)"
  fi
  probe_root="$(mktemp -d)"
  trap 'rm -rf -- "$probe_root"' EXIT
  if [[ -z "$cc" ]] || ! printf 'int main(void){return 0;}\n' | "$cc" -x c -fsanitize=address -o "$probe_root/c-sanitizer" - >/"$probe_root/cc.stdout" 2>"$probe_root/cc.stderr"; then
    fuzz_status=1
    fuzz_reason="C AddressSanitizer compiler/linker probe failed"
  elif [[ -z "$cxx" ]] || ! printf '#include <cstddef>\nint main(){return 0;}\n' | "$cxx" -x c++ -std=c++17 -fsanitize=address -o "$probe_root/cxx-sanitizer" - \
      >"$probe_root/cxx.stdout" 2>"$probe_root/cxx.stderr"; then
    fuzz_status=1
    fuzz_reason="C++ AddressSanitizer compiler/linker probe failed"
  fi
  {
    printf 'cc=%s\n' "$cc"
    printf 'cxx=%s\n' "$cxx"
    printf 'reason=%s\n' "$fuzz_reason"
    cat "$probe_root/cc.stderr" "$probe_root/cxx.stderr" 2>/dev/null || true
  } >"$OUT_ROOT/sanitizer-probe.log"
fi

if (( rust_status == 0 && fuzz_status == 0 )); then
  overall="PASS"
  blocker_count=0
else
  overall="BLOCKED"
  blocker_count=1
fi

jq -n \
  --arg status "$overall" \
  --arg plan "compatibility/phase15-hardening-plan.json" \
  --arg rust_log "target/phase15/hardening-preflight/rust-tests.log" \
  --arg sanitizer_log "target/phase15/hardening-preflight/sanitizer-probe.log" \
  --arg reason "$fuzz_reason" \
  --argjson rust_status "$rust_status" \
  --argjson fuzz_status "$fuzz_status" \
  --argjson blocker_count "$blocker_count" \
  '{schema_version: 1, status: $status, slice: "phase15-hardening-preflight", plan: $plan,
    checks: {property_tests: {status: (if $rust_status == 0 then "PASS" else "FAIL" end), log: $rust_log},
      sanitizer_toolchain: {status: (if $fuzz_status == 0 then "READY" else "BLOCKED" end), log: $sanitizer_log, reason: $reason}},
    campaigns_ready: ($status == "PASS"), active_blockers: $blocker_count}' >"$RESULT"

if [[ "$overall" != "PASS" && "$report_only" == false ]]; then
  printf 'Phase 15 hardening preflight is blocked: %s. See %s.\n' "$fuzz_reason" "$RESULT" >&2
  exit 1
fi
printf 'Phase 15 hardening preflight recorded: %s.\n' "$overall"
