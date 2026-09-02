#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-realtime-sanitizer.sh"
readonly PLAN="$ROOT/compatibility/phase15-realtime-sanitizer.json"
test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" >/dev/null

bad_plan="$test_root/bad-plan.json"
jq '.campaigns.mantle_hot_path.status = "FAIL"' "$PLAN" >"$bad_plan"
if "$CHECKER" --plan "$bad_plan" >/dev/null 2>&1; then
  printf 'Phase 15 realtime checker accepted a failed hot-path campaign.\n' >&2
  exit 1
fi

bad_results="$test_root/results"
mkdir -p "$bad_results"
cp "$ROOT/target/phase15/realtime-sanitizer"/* "$bad_results/"
printf '==1==ERROR: RealtimeSanitizer: unsafe-library-call\n' >>"$bad_results/positive.log"
if "$CHECKER" --results-root "$bad_results" >/dev/null 2>&1; then
  printf 'Phase 15 realtime checker accepted a positive-path sanitizer finding.\n' >&2
  exit 1
fi

printf 'Phase 15 RealtimeSanitizer checker success and failure-detection paths passed.\n'
