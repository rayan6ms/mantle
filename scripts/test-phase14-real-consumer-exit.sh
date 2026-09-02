#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase14-real-consumer-exit.sh"
readonly PLAN="$ROOT/compatibility/phase14-real-consumer-exit.json"
readonly RESULTS="$ROOT/target/phase14"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" --validate-only >/dev/null

bad_plan="$test_root/bad-plan.json"
jq '.decision.kill_gate = "FAIL"' "$PLAN" > "$bad_plan"
if "$CHECKER" --validate-only --plan "$bad_plan" >/dev/null 2>&1; then
  printf 'Phase 14 exit checker accepted a failing kill-gate decision.\n' >&2
  exit 1
fi

bad_results="$test_root/results"
while IFS= read -r result; do
  mkdir -p "$bad_results/$(dirname "$result")"
  cp "$RESULTS/$result" "$bad_results/$result"
done < <(jq --raw-output '[.evidence_dimensions[].results[]] | unique[]' "$PLAN")
jq '.deterministic_runs.exact_match = false' \
  "$bad_results/real-consumer-behavior/result.json" > "$test_root/bad-result.json"
mv "$test_root/bad-result.json" "$bad_results/real-consumer-behavior/result.json"
if "$CHECKER" --validate-only --results-root "$bad_results" >/dev/null 2>&1; then
  printf 'Phase 14 exit checker accepted divergent real-consumer behavior.\n' >&2
  exit 1
fi

printf 'Phase 14 real-consumer exit checker success and failure paths passed.\n'
