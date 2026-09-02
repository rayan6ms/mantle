#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-phase15-hardening-exit.sh"
readonly PLAN="$ROOT/compatibility/phase15-hardening-exit.json"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" --validate-only >/dev/null

bad_completion="$test_root/bad-completion.json"
jq '
  .status = "COMPLETE" |
  .decision = {hardening: "PASS", phase15_complete: true, next_action: null}
' "$PLAN" > "$bad_completion"
if "$CHECKER" --validate-only --plan "$bad_completion" >/dev/null 2>&1; then
  printf 'Phase 15 exit checker accepted a full pass without the replay duration claim reduction.\n' >&2
  exit 1
fi

bad_publication="$test_root/bad-publication.json"
jq '.publication.ready = true' "$PLAN" > "$bad_publication"
if "$CHECKER" --validate-only --plan "$bad_publication" >/dev/null 2>&1; then
  printf 'Phase 15 exit checker accepted publication with Cargo Vet exemptions.\n' >&2
  exit 1
fi

bad_coverage="$test_root/bad-coverage.json"
jq 'del(.evidence_dimensions[-1])' "$PLAN" > "$bad_coverage"
if "$CHECKER" --validate-only --plan "$bad_coverage" >/dev/null 2>&1; then
  printf 'Phase 15 exit checker accepted incomplete evidence coverage.\n' >&2
  exit 1
fi

if [[ "$(jq --raw-output '.status' "$PLAN")" == "WAITING" ]]; then
  if "$CHECKER" >/dev/null 2>&1; then
    printf 'Phase 15 exit checker accepted a waiting campaign.\n' >&2
    exit 1
  fi
else
  "$CHECKER" >/dev/null
fi

printf 'Phase 15 hardening exit checker success and failure paths passed.\n'
