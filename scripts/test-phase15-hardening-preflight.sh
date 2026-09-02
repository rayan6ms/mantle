#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-hardening-preflight.sh"
readonly RESULT="$ROOT/target/phase15/hardening-preflight/result.json"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

# Force the sanitizer-probe failure so this regression is independent of the host toolchain.
CXX="$test_root/missing-cxx" "$CHECKER" --report-only >/dev/null
jq --exit-status '
  .schema_version == 1 and .slice == "phase15-hardening-preflight" and
  .checks.property_tests.status == "PASS" and
  .status == "BLOCKED" and .campaigns_ready == false and .active_blockers == 1
' "$RESULT" >/dev/null

set +e
CXX="$test_root/missing-cxx" "$CHECKER" >/dev/null 2>&1
rc=$?
set -e
if [[ "$rc" -eq 0 ]]; then
  printf 'Phase 15 preflight failed to enforce its sanitizer blocker.\n' >&2
  exit 1
fi

printf 'Phase 15 hardening preflight success and blocker-enforcement paths passed.\n'
