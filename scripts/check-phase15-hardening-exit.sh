#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-hardening-exit.json"
validate_only=false

usage() {
  printf 'Usage: %s [--validate-only] [--plan PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --validate-only) validate_only=true; shift ;;
    --plan) (( $# >= 2 )) || { usage; exit 2; }; plan="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly PLAN="$plan"
readonly HARDENING="$ROOT/compatibility/phase15-hardening-plan.json"
readonly LEDGER="$ROOT/PROJECT_LEDGER.md"
readonly COMPATIBILITY="$ROOT/COMPATIBILITY.md"
readonly SUPERSEDING_CLOSURE="$ROOT/compatibility/publication-cargo-vet-exemption-closure.json"

validate_exit_plan() {
  jq --exit-status --slurpfile hardening "$HARDENING" '
    . as $exit | $hardening[0] as $hardening |
    .schema_version == 1 and
    (.status | IN("WAITING", "COMPLETE")) and
    .phase == "phase15-hardening-exit" and
    .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
    .hardening_plan == "compatibility/phase15-hardening-plan.json" and
    .preflight_resolution == {
      status: "RESOLVED",
      historical_result: "target/phase15/hardening-preflight/result.json",
      blocker: "B-001",
      resolution_gate: "scripts/check-phase15-sanitizer-fuzz.sh"
    } and
    ([.required_gates[].id] | sort) == [
      "concurrency_lifecycle", "dependency_audit", "native_soak",
      "realtime_sanitizer", "replay_fault_soak", "sanitizer_fuzz"
    ] and
    (.required_gates | length) == ([.required_gates[].id] | unique | length) and
    all(.required_gates[];
      (.plan | startswith("compatibility/phase15-")) and
      (.checker | startswith("scripts/check-phase15-")) and
      (.result | startswith("target/phase15/"))) and
    ([.evidence_dimensions[].dimension] | sort) == ([
      "concurrency", "fuzzing", "lifecycle", "native_sanitizers", "native_soak",
      "network_fault_soak", "property_tests", "realtime", "supply_chain",
      "undefined_behavior"
    ] | sort) and
    (.evidence_dimensions | length) ==
      ([.evidence_dimensions[].dimension] | unique | length) and
    all(.evidence_dimensions[];
      .gate as $gate | any($exit.required_gates[]; .id == $gate)) and
    .claim_reductions == [{
      gate: "replay_fault_soak",
      omitted_claim: "72-hour endurance",
      accepted_observation_seconds: 142741.18608706,
      basis: "operator-constrained acceptance of the longest uninterrupted retained evidence after a host reboot"
    }] and
    .publication.ready == false and
    .publication.constraints == [{
      ledger: "D-001",
      exact_version_exemptions: 140,
      removal_trigger: "replace all exemptions with imported or local audits before publishing a Mantle production crate"
    }] and
    .active_blockers == [] and
    $hardening.schema_version == 1 and
    $hardening.phase == "phase15-hardening" and
    $hardening.compatibility_baseline == .compatibility_baseline and
    ($hardening.campaigns | length) == 10 and
    ([ $hardening.campaigns[].id ] | unique | length) == 10 and
    (if .status == "WAITING" then
      .decision == {
        hardening: "WAITING_FOR_REPLAY",
        phase15_complete: false,
        next_action: "phase15-replay-fault-soak"
      } and
      all(.required_gates[];
        if .id == "replay_fault_soak" then .status == "RUNNING" else .status == "PASS" end) and
      $hardening.status == "IN_PROGRESS" and
      $hardening.completed_slice == "phase15-native-soak" and
      ($hardening.campaigns[] | select(.id == "native_soak") | .status) == "PASS" and
      ($hardening.campaigns[] | select(.id == "replay_fault_soak") | .status) == "RUNNING" and
      $hardening.next_slice == "phase15-replay-fault-soak"
    else
      .decision == {
        hardening: "PASS_WITH_REDUCED_DURATION",
        phase15_complete: true,
        next_action: null
      } and
      all(.required_gates[];
        if .id == "replay_fault_soak" then
          .status == "PASS_WITH_REDUCED_DURATION"
        else
          .status == "PASS"
        end) and
      $hardening.status == "COMPLETE" and
      $hardening.completed_slice == "phase15-hardening-exit" and
      all($hardening.campaigns[];
        if .id == "replay_fault_soak" then
          .status == "PASS_WITH_REDUCED_DURATION"
        else
          .status == "PASS"
        end) and
      $hardening.next_slice == null
    end)
  ' "$PLAN" >/dev/null
}

validate_references() {
  while IFS= read -r path; do
    [[ -f "$ROOT/$path" ]] || {
      printf 'Phase 15 hardening exit references a missing file: %s\n' "$path" >&2
      return 1
    }
  done < <(jq --raw-output '[
    .hardening_plan, .preflight_resolution.historical_result,
    .preflight_resolution.resolution_gate,
    .required_gates[].plan, .required_gates[].checker,
    (.required_gates[] | select(.status | IN("PASS", "PASS_WITH_REDUCED_DURATION")) | .result)
  ] | unique[]' "$PLAN")

  if [[ "$(jq --raw-output '.status' "$PLAN")" == "WAITING" ]]; then
    for path in \
      target/phase15/replay-fault-soak/run-metadata.json \
      target/phase15/replay-fault-soak/progress.jsonl; do
      [[ -s "$ROOT/$path" ]] || {
        printf 'Phase 15 running replay evidence is missing: %s\n' "$path" >&2
        return 1
      }
    done
  fi

  jq --exit-status '
    .schema_version == 1 and .status == "PASS" and
    .slice == "publication-cargo-vet-exemption-closure" and
    .baseline.exact_version_exemptions == 140 and
    .closure.remaining_exemptions == 0
  ' "$SUPERSEDING_CLOSURE" >/dev/null
  grep --fixed-strings 'D-001' "$LEDGER" | grep --fixed-strings 'is resolved' >/dev/null
  grep --fixed-strings 'D-001' "$COMPATIBILITY" | grep --fixed-strings 'is resolved' >/dev/null
}

validate_exit_plan
validate_references

if [[ "$validate_only" == true ]]; then
  printf 'Phase 15 hardening exit contract is valid with status %s.\n' \
    "$(jq --raw-output '.status' "$PLAN")"
  exit 0
fi

if [[ "$(jq --raw-output '.status' "$PLAN")" != "COMPLETE" ]]; then
  printf 'Phase 15 hardening exit is waiting for the full replay/fault soak.\n' >&2
  exit 1
fi

while IFS= read -r checker; do
  "$ROOT/$checker"
done < <(jq --raw-output '.required_gates[].checker' "$PLAN")

printf 'Phase 15 hardening exit passed with reduced duration: all 6 gates and 10 evidence dimensions passed; 72-hour endurance is not claimed, and the historical D-001 publication constraint has a validated later closure.\n'
