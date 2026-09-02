#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-sanitizer-fuzz.sh"
readonly PLAN="$ROOT/compatibility/phase15-sanitizer-fuzz.json"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" >/dev/null

bad_plan="$test_root/bad-plan.json"
jq '.campaigns.thread.status = "BLOCKED"' "$PLAN" > "$bad_plan"
if "$CHECKER" --plan "$bad_plan" >/dev/null 2>&1; then
  printf 'Phase 15 sanitizer checker accepted a blocked campaign.\n' >&2
  exit 1
fi

bad_results="$test_root/results"
mkdir -p "$bad_results"
cp "$ROOT/target/phase15/sanitizer-fuzz"/*.log "$bad_results/"
cp "$ROOT/target/phase15/sanitizer-fuzz/result.json" "$bad_results/"
printf 'ThreadSanitizer: data race\n' >> "$bad_results/tsan.log"
if "$CHECKER" --results-root "$bad_results" >/dev/null 2>&1; then
  printf 'Phase 15 sanitizer checker accepted a sanitizer finding.\n' >&2
  exit 1
fi

printf 'Phase 15 sanitizer/fuzz checker success and finding-detection paths passed.\n'
