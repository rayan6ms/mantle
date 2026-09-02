#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-concurrency-lifecycle.sh"
readonly PLAN="$ROOT/compatibility/phase15-concurrency-lifecycle.json"
test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" >/dev/null
bad_plan="$test_root/bad-plan.json"
jq '.campaigns.loom.status = "BLOCKED"' "$PLAN" > "$bad_plan"
if "$CHECKER" --plan "$bad_plan" >/dev/null 2>&1; then
  printf 'Phase 15 concurrency checker accepted a blocked campaign.\n' >&2
  exit 1
fi

bad_results="$test_root/results"
mkdir -p "$bad_results"
cp "$ROOT/target/phase15/concurrency-lifecycle"/*.log "$bad_results/"
cp "$ROOT/target/phase15/concurrency-lifecycle/result.json" "$bad_results/"
sed -i 's/"probe":"classloader","collected":true/"probe":"classloader","collected":false/' "$bad_results/jni-lifecycle.log"
if "$CHECKER" --results-root "$bad_results" >/dev/null 2>&1; then
  printf 'Phase 15 concurrency checker accepted a failed classloader probe.\n' >&2
  exit 1
fi

printf 'Phase 15 concurrency/lifecycle checker success and failure-detection paths passed.\n'
