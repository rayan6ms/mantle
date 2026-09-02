#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-dependency-audit.sh"
readonly PLAN="$ROOT/compatibility/phase15-dependency-audit.json"
test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" >/dev/null

bad_plan="$test_root/bad-plan.json"
jq '.campaigns.vet.status = "FAIL"' "$PLAN" >"$bad_plan"
if "$CHECKER" --plan "$bad_plan" >/dev/null 2>&1; then
  printf 'Phase 15 dependency checker accepted a failed Vet campaign.\n' >&2
  exit 1
fi

bad_results="$test_root/results"
mkdir -p "$bad_results"
cp "$ROOT/target/phase15/dependency-audit"/* "$bad_results/"
jq '.vulnerabilities.found = true | .vulnerabilities.count = 1' \
  "$bad_results/workspace-audit.json" >"$bad_results/workspace-audit.tmp"
mv "$bad_results/workspace-audit.tmp" "$bad_results/workspace-audit.json"
if "$CHECKER" --results-root "$bad_results" >/dev/null 2>&1; then
  printf 'Phase 15 dependency checker accepted an advisory finding.\n' >&2
  exit 1
fi

printf 'Phase 15 dependency checker success and failure-detection paths passed.\n'
