#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RUNNER="$ROOT/scripts/run-phase15-replay-fault-soak.sh"
readonly CHECKER="$ROOT/scripts/check-phase15-replay-fault-soak.sh"
test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

smoke_root="$test_root/smoke"
PHASE15_RESULTS_ROOT="$smoke_root" \
MANTLE_REPLAY_FAULT_SECONDS=3 \
MANTLE_REPLAY_FAULT_CHECKPOINT_SECONDS=1 \
MANTLE_REPLAY_FAULT_CYCLE_DELAY_MS=0 \
  "$RUNNER" >"$test_root/smoke.log"
"$CHECKER" --results-root "$smoke_root" --allow-smoke >/dev/null

if PHASE15_RESULTS_ROOT="$smoke_root" \
  MANTLE_REPLAY_FAULT_SECONDS=3 \
  MANTLE_REPLAY_FAULT_CHECKPOINT_SECONDS=1 \
  MANTLE_REPLAY_FAULT_CYCLE_DELAY_MS=0 \
    "$RUNNER" >/dev/null 2>&1; then
  printf 'Phase 15 replay/fault runner overwrote retained evidence.\n' >&2
  exit 1
fi

if "$CHECKER" --results-root "$smoke_root" >/dev/null 2>&1; then
  printf 'Phase 15 replay/fault checker accepted smoke as reduced-duration campaign evidence.\n' >&2
  exit 1
fi

if PHASE15_RESULTS_ROOT="$test_root/retired-long-run" \
  MANTLE_REPLAY_FAULT_SECONDS=259200 \
    "$RUNNER" >/dev/null 2>&1; then
  printf 'Phase 15 replay/fault runner accepted a retired 72-hour campaign.\n' >&2
  exit 1
fi

bad_root="$test_root/bad"
mkdir -p "$bad_root"
cp "$smoke_root/progress.jsonl" "$smoke_root/run-metadata.json" "$bad_root/"
jq '.harness.unexpected_failures = 1' "$smoke_root/result.json" >"$bad_root/result.json"
if "$CHECKER" --results-root "$bad_root" --allow-smoke >/dev/null 2>&1; then
  printf 'Phase 15 replay/fault checker accepted an unexpected failure.\n' >&2
  exit 1
fi

printf 'Phase 15 replay/fault smoke, duration rejection, and failure paths passed.\n'
