#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT

validate_only=false
plan="$ROOT/compatibility/phase14-real-consumer-exit.json"
results_root="$ROOT/target/phase14"

usage() {
  printf 'Usage: %s [--validate-only] [--plan PATH] [--results-root PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --validate-only)
      validate_only=true
      shift
      ;;
    --plan)
      (( $# >= 2 )) || { usage; exit 2; }
      plan="$2"
      shift 2
      ;;
    --results-root)
      (( $# >= 2 )) || { usage; exit 2; }
      results_root="$2"
      shift 2
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

readonly plan results_root
readonly INVENTORY="$ROOT/reference/phase14-real-consumer-inventory.json"
readonly BEHAVIOR_PLAN="$ROOT/compatibility/phase14-real-consumer-behavior.json"
readonly LEDGER="$ROOT/PROJECT_LEDGER.md"
readonly COMPATIBILITY="$ROOT/COMPATIBILITY.md"

validate_exit_plan() {
  jq --exit-status \
    --slurpfile inventory "$INVENTORY" \
    --slurpfile behavior_plan "$BEHAVIOR_PLAN" \
    --from-file /dev/stdin "$plan" >/dev/null <<'JQ'
    def sorted_coverage:
      sort_by(.behavior) | map({behavior, consumers: (.consumers | sort)});
    def consumer_behaviors($coverage; $id):
      [$coverage[] | select(.consumers | index($id)) | .behavior] | sort;

    . as $exit |
    $inventory[0] as $inv |
    $behavior_plan[0] as $behavior |
    .schema_version == 1 and
    .status == "COMPLETE" and
    .phase == "phase14-real-consumer-compatibility" and
    .compatibility_baseline == $inv.compatibility_baseline and
    .mantle_artifact == $inv.mantle_replacement and
    .inventory == "reference/phase14-real-consumer-inventory.json" and
    .behavior_plan == "compatibility/phase14-real-consumer-behavior.json" and
    ([.evidence_dimensions[].dimension] | sort) ==
      (["artifact", "behavior", "binary", "native_loading", "semantic", "serialization",
        "source", "spi"] | sort) and
    (.evidence_dimensions | length) ==
      ([.evidence_dimensions[].dimension] | unique | length) and
    all(.evidence_dimensions[]; (.gates | length) > 0 and (.results | length) > 0) and
    ([.consumers[].id] | sort) == ([$inv.consumers[].id] | sort) and
    (.consumers | length) == ([.consumers[].id] | unique | length) and
    all(.consumers[];
      . as $consumer |
      any($inv.consumers[];
        .id == $consumer.id and .revision == $consumer.revision) and
      ($consumer.behaviors | sort) == consumer_behaviors($inv.coverage; $consumer.id)) and
    (.behavior_coverage | sorted_coverage) == ($inv.coverage | sorted_coverage) and
    ([.behavior_coverage[].behavior] | sort) == ($inv.required_behaviors | sort) and
    ([.behavior_coverage[].behavior] | unique | length) == ($inv.required_behaviors | length) and
    ([.behavior_coverage[].behavior] | sort) ==
      ([$behavior.scenarios[].behaviors[]] | unique | sort) and
    ([.migration_boundaries[].ledger] | sort) ==
      ["C-004", "C-005", "C-006", "C-007", "C-008"] and
    (.migration_boundaries | length) == ([.migration_boundaries[].ledger] | unique | length) and
    all(.migration_boundaries[];
      . as $boundary |
      any($exit.consumers[]; .id == $boundary.consumer) and
      any($exit.evidence_dimensions[].gates[]; . == $boundary.regression_gate) and
      any(["boundary_redesign", "build_normalization", "claim_reduction", "supported"][];
        . == $boundary.resolution) and
      (.regression_gate | startswith("scripts/check-phase14-"))) and
    .decision.kill_gate == "PASS" and
    .decision.material_unsupported_dependencies == [] and
    (.decision.documented_claim_reductions | length) == 1 and
    .decision.documented_claim_reductions[0].ledger == "C-005" and
    (.decision.documented_claim_reductions[0].scope | contains("Beam and Getyarn")) and
    (.decision.compatibility_claim | contains("all nine required deterministic behaviors")) and
    (.decision.compatibility_claim | contains("not claimed operational")) and
    .decision.next_phase == "phase15-hardening"
JQ
}

validate_result_files() {
  while IFS= read -r result; do
    [[ -f "$results_root/$result" ]] || {
      printf 'Phase 14 exit evidence is missing: %s\n' "$results_root/$result" >&2
      return 1
    }
  done < <(jq --raw-output '[.evidence_dimensions[].results[]] | unique[]' "$plan")

  while IFS=$'\t' read -r consumer revision result; do
    jq --exit-status --arg consumer "$consumer" --arg revision "$revision" '
      .schema_version == 1 and .status == "PASS" and
      .consumer == $consumer and .revision == $revision and
      .source_unchanged == true and
      .reference_compile.status == 0 and .mantle_compile.status == 0
    ' "$results_root/$result" >/dev/null
  done < <(jq --raw-output '.consumers[] | [.id, .revision, .result] | @tsv' "$plan")

  jq --exit-status '
    .legacy_filename_packaging_disabled == true and
    .native.loader_smoke == "PASS" and
    (.native.coordinate | endswith(":linux-x86_64")) and
    (.native.path | startswith("/"))
  ' "$results_root/lavalink-source-compatibility/result.json" >/dev/null

  jq --exit-status '
    .legacy_linkage == {beam: "LINKAGE_ONLY", getyarn: "LINKAGE_ONLY", smoke: "PASS"} and
    (.behaviors | to_entries | all(.value == "PASS"))
  ' "$results_root/jmusicbot-source-compatibility/result.json" >/dev/null

  for result in \
    "$results_root/simplevoicechat-source-compatibility/result.json" \
    "$results_root/youtube-source-spi-compatibility/result.json"; do
    jq --exit-status '
      .smoke.reference_and_mantle_match == true and
      (.behaviors | to_entries | all(.value == "PASS"))
    ' "$result" >/dev/null
  done

  jq --exit-status --slurpfile inventory "$INVENTORY" '
    . as $result |
    .status == "PASS" and .slice == "phase14-real-consumer-behavior" and
    .deterministic_runs.exact_match == true and
    .deterministic_runs.reference == .deterministic_runs.mantle and
    (.deterministic_runs.mantle.serialized_sha256 | test("^[0-9a-f]{64}$")) and
    all($inventory[0].required_behaviors[];
      . as $required | $result.deterministic_runs.mantle[$required] == "PASS") and
    ([.direct_consumer_classes | keys[]] | sort) == ["jmusicbot", "youtube_source"] and
    ([.interaction_shapes | keys[]] | sort) == ["lavalink", "simplevoicechat_music"]
  ' "$results_root/real-consumer-behavior/result.json" >/dev/null
}

validate_references() {
  while IFS= read -r path; do
    [[ -f "$ROOT/$path" ]] || {
      printf 'Phase 14 exit gate references a missing file: %s\n' "$path" >&2
      return 1
    }
  done < <(jq --raw-output '
    [.inventory, .behavior_plan, .evidence_dimensions[].gates[],
     .consumers[].source_gate, .migration_boundaries[].regression_gate] | unique[]
  ' "$plan")

  while IFS=$'\t' read -r ledger gate; do
    entry="$(awk -v id="$ledger" '
      $0 ~ "^\\[" id "\\]" {inside = 1}
      inside {print}
      inside && $0 ~ /^\[C-[0-9]+\]/ && $0 !~ "^\\[" id "\\]" {exit}
    ' "$LEDGER")"
    [[ -n "$entry" ]] || { printf 'Ledger entry is missing: %s\n' "$ledger" >&2; return 1; }
    grep --fixed-strings 'Decision:' <<<"$entry" >/dev/null
    grep --fixed-strings 'Regression test:' <<<"$entry" >/dev/null
    grep --fixed-strings "$gate" <<<"$entry" >/dev/null
  done < <(jq --raw-output '.migration_boundaries[] | [.ledger, .regression_gate] | @tsv' "$plan")

  for phrase in \
    'Kill-gate D' \
    'Beam and Getyarn' \
    'scripts/check-phase14-real-consumer-exit.sh'; do
    grep --fixed-strings "$phrase" "$COMPATIBILITY" >/dev/null
  done
}

if [[ "$validate_only" == false ]]; then
  "$ROOT/scripts/check-phase14-consumer-inventory.sh"
  while IFS= read -r gate; do
    "$ROOT/$gate"
  done < <(jq --raw-output '[.consumers[].source_gate] | unique[]' "$plan")
  "$ROOT/scripts/check-phase14-real-consumer-behavior.sh"
fi

validate_exit_plan
validate_references
validate_result_files

printf 'Phase 14 real-consumer exit passed: 4 pinned consumers, all 9 required behaviors, 8 evidence dimensions, and 5 explicit migration decisions support Kill-gate D PASS.\n'
