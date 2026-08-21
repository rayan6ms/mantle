#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly GATE="$ROOT/reference/phase12-source-gate.json"
readonly STATUS="$ROOT/reference/remote-source-status-2026-08-13.json"

usage() {
  printf 'Usage: %s [--offline|--live-public|--live-credentialed|--live-all]\n' "$0" >&2
}

readonly MODE="${1:---offline}"
if [[ $# -gt 1 ]] || [[ "$MODE" != "--offline" && "$MODE" != "--live-public" && \
  "$MODE" != "--live-credentialed" && "$MODE" != "--live-all" ]]; then
  usage
  exit 2
fi

run_cargo() {
  env -u APPIMAGE -u APPDIR cargo "$@"
}

require_live_environment() {
  local category="$1"
  local missing=0
  while IFS= read -r variable; do
    if [[ -z "${!variable:-}" ]]; then
      printf 'Required %s live-gate environment is unset: %s\n' "$category" "$variable" >&2
      missing=1
    fi
  done < <(jq --raw-output --arg category "$category" '
    .sources[] | select(.live.category == $category) | .live.required_environment[]
  ' "$GATE")
  if [[ "$missing" -ne 0 ]]; then
    return 1
  fi
}

jq --exit-status --slurpfile status "$STATUS" '
  .schema_version == 1 and
  .compatibility_baseline == "lavaplayer-2.2.6" and
  .source_status == "reference/remote-source-status-2026-08-13.json" and
  (.sources | length) == 11 and
  [.sources[].order] == [1,2,3,4,5,6,7,8,9,10,11] and
  ([.sources[] | [.order, .class, .source_name, .disposition]] ==
    [$status[0].sources[] | [.order, .class, .source_name, .disposition]]) and
  ([.sources[].live.category] | sort) ==
    (["credentialed","none","none","none","none","none","none",
      "public","public","public","region_conditional"] | sort) and
  all(.sources[]; (.implementation | length) > 0 and (.replay_tests | length) > 0) and
  all(.sources[] | select(.live.category != "none");
    (.live.test_target | length) > 0 and (.live.test_name | length) > 0 and
    (.live.required_environment | type) == "array" and
    (.live.optional_environment | type) == "array") and
  all(.sources[] | select(.live.category == "none"); (.live.reason | length) > 0)
' "$GATE" >/dev/null

if [[ "$MODE" == "--live-credentialed" || "$MODE" == "--live-all" ]]; then
  require_live_environment credentialed
fi

while IFS= read -r path; do
  if [[ ! -f "$ROOT/$path" ]]; then
    printf 'Phase 12 source-gate evidence is missing: %s\n' "$path" >&2
    exit 1
  fi
done < <(jq --raw-output '
  [.offline_gate.required_checkers[], .sources[].implementation[], .sources[].replay_tests[]] |
  unique[]
' "$GATE")

while IFS=$'\t' read -r test_path test_name; do
  if ! grep --fixed-strings "fn $test_name()" "$ROOT/$test_path" >/dev/null; then
    printf 'Declared live test %s is absent from %s.\n' "$test_name" "$test_path" >&2
    exit 1
  fi
done < <(jq --raw-output '
  .sources[] | select(.live.category != "none") | . as $source |
  [($source.replay_tests[] |
    select(endswith("/" + $source.live.test_target + ".rs"))), $source.live.test_name] | @tsv
' "$GATE")

while IFS= read -r checker; do
  "$ROOT/$checker"
done < <(jq --raw-output '.offline_gate.required_checkers[]' "$GATE")

run_cargo test --locked -p mantle-core -p mantle-media -p mantle-jvm --tests
printf 'Phase 12 offline source inventory and replay gate passed for all eleven sources.\n'

run_live_category() {
  local category="$1"
  while IFS=$'\t' read -r target test_name; do
    run_cargo test --locked -p mantle-media --test "$target" "$test_name" -- \
      --ignored --exact --nocapture
  done < <(jq --raw-output --arg category "$category" '
    .sources[] | select(.live.category == $category) |
    [.live.test_target, .live.test_name] | @tsv
  ' "$GATE")
}

if [[ "$MODE" == "--live-public" || "$MODE" == "--live-all" ]]; then
  run_live_category public
  run_live_category region_conditional
  printf 'Phase 12 public and region-conditional scheduled live gates passed.\n'
fi

if [[ "$MODE" == "--live-credentialed" || "$MODE" == "--live-all" ]]; then
  run_live_category credentialed
  printf 'Phase 12 caller-credentialed scheduled live gates passed.\n'
fi
