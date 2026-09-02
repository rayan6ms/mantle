#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-native-soak.json"
result_root="$ROOT/target/phase15/native-soak"
allow_smoke=false
while (( $# > 0 )); do
  case "$1" in
    --plan) (( $# >= 2 )) || exit 2; plan="$2"; shift 2 ;;
    --results-root) (( $# >= 2 )) || exit 2; result_root="$2"; shift 2 ;;
    --allow-smoke) allow_smoke=true; shift ;;
    *) printf 'Usage: %s [--plan PATH] [--results-root PATH] [--allow-smoke]\n' "$0" >&2; exit 2 ;;
  esac
done
readonly PLAN="$plan"
readonly RESULT_ROOT="$result_root"
readonly RESULT="$RESULT_ROOT/result.json"
readonly PROGRESS="$RESULT_ROOT/progress.jsonl"
readonly METADATA="$RESULT_ROOT/run-metadata.json"
readonly HARDENING="$ROOT/compatibility/phase15-hardening-plan.json"

jq --exit-status -s --argjson allow_smoke "$allow_smoke" '
  .[0] as $plan | .[1] as $result |
  $plan.schema_version == 1 and
  ($plan.status | IN("READY", "RUNNING", "COMPLETE")) and
  $plan.slice == "phase15-native-soak" and
  $plan.compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  $plan.toolchain == {rust: "1.97.1", profile: "release"} and
  $plan.bounds == {duration_seconds: 86400, checkpoint_seconds: 60,
    cycle_delay_ms: 1000, workloads: 5, max_checkpoints: 1442,
    memory_window_samples: 8, max_rss_growth_kib: 8192,
    max_pss_growth_kib: 8192, max_threads: 1} and
  ($plan.workloads | length) == 5 and
  $plan.active_blockers == [] and
  $plan.evidence.runner == "scripts/run-phase15-native-soak.sh" and
  $plan.evidence.checker == "scripts/check-phase15-native-soak.sh" and
  $plan.evidence.progress == "target/phase15/native-soak/progress.jsonl" and
  $plan.evidence.result == "target/phase15/native-soak/result.json" and
  $plan.next_slice == "phase15-replay-fault-soak" and
  $result.schema_version == 1 and $result.slice == "phase15-native-soak" and
  $result.toolchain == {rust: "1.97.1", profile: "release"} and
  $result.active_blockers == [] and
  $result.acceptance == {max_rss_growth_kib: 8192, max_pss_growth_kib: 8192,
    max_threads: 1} and
  ($result.fixtures | length) == 5 and
  all($plan.workloads[];
    . as $workload |
    any($result.fixtures[];
      .file == $workload.file and .sha256 == $workload.sha256) and
    any($result.harness.fingerprints[];
      .workload == $workload.id and
      .output_units == $workload.fingerprint.output_units and
      .decoded_samples == $workload.fingerprint.decoded_samples and
      .encoded_bytes == $workload.fingerprint.encoded_bytes and
      .checksum == $workload.fingerprint.checksum)) and
  $result.harness.schema_version == 1 and $result.harness.kind == "result" and
  $result.harness.status == "PASS" and $result.harness.workloads == 5 and
  $result.harness.sessions > 0 and $result.harness.completed_cycles > 0 and
  $result.harness.fingerprint_mismatches == 0 and
  $result.harness.elapsed_seconds >= $result.harness.configured_duration_seconds and
  $result.harness.elapsed_seconds < ($result.harness.configured_duration_seconds + 60) and
  $result.harness.checkpoints >= 2 and $result.harness.checkpoints <= 1442 and
  $result.harness.memory.first_window_samples == ([$result.harness.checkpoints, 8] | min) and
  $result.harness.memory.last_window_samples == ([$result.harness.checkpoints, 8] | min) and
  $result.harness.memory.rss_growth_kib <= 8192 and
  $result.harness.memory.pss_growth_kib <= 8192 and
  $result.harness.memory.max_threads <= 1 and
  (if $allow_smoke then
    $result.status == "SMOKE_PASS" and $result.campaign_mode == "smoke" and
    $result.harness.configured_duration_seconds < 86400 and
    $result.harness.checkpoint_seconds >= 1 and
    $result.harness.cycle_delay_ms <= 1000
  else
    $result.status == "PASS" and $result.campaign_mode == "full" and
    $result.harness.configured_duration_seconds == 86400 and
    $result.harness.checkpoint_seconds == 60 and
    $result.harness.cycle_delay_ms == 1000 and
    $result.harness.checkpoints >= 1441
  end)
' "$PLAN" "$RESULT" >/dev/null

jq --exit-status -s --slurpfile result "$RESULT" '
  [.[] | select(.kind == "checkpoint")] as $checkpoints |
  [.[] | select(.kind == "result")] as $results |
  ($checkpoints | length) == $result[0].harness.checkpoints and
  ($results | length) == 1 and $results[0] == $result[0].harness and
  ($checkpoints | map(.checkpoint)) == [range(1; ($checkpoints | length) + 1)] and
  ($checkpoints | map(.elapsed_seconds)) == ($checkpoints | map(.elapsed_seconds) | sort) and
  ($checkpoints | map(.sessions)) == ($checkpoints | map(.sessions) | sort) and
  ($checkpoints[-1].sessions <= $result[0].harness.sessions)
' "$PROGRESS" >/dev/null

jq --exit-status -s '
  .[0] as $metadata | .[1] as $result |
  $metadata.schema_version == 1 and $metadata.slice == "phase15-native-soak" and
  $metadata.status == $result.status and
  $metadata.campaign_mode == $result.campaign_mode and
  $metadata.started_at == $result.started_at and
  $metadata.expected_finish_at == $result.expected_finish_at and
  $metadata.finished_at == $result.finished_at and
  $metadata.bounds.duration_seconds == $result.harness.configured_duration_seconds and
  $metadata.bounds.checkpoint_seconds == $result.harness.checkpoint_seconds and
  $metadata.bounds.cycle_delay_ms == $result.harness.cycle_delay_ms
' "$METADATA" "$RESULT" >/dev/null

jq --exit-status '
  .schema_version == 1 and (.status | IN("IN_PROGRESS", "COMPLETE")) and
  .phase == "phase15-hardening" and
  (.completed_slice | IN("phase15-native-soak", "phase15-replay-fault-soak",
    "phase15-hardening-exit")) and
  (.campaigns[] | select(.id == "native_soak") | .status) == "PASS" and
  .evidence.active_blockers == [] and
  (.next_slice | IN("phase15-replay-fault-soak", "phase15-hardening-exit", null))
' "$HARDENING" >/dev/null

if [[ "$allow_smoke" == true ]]; then
  printf 'Phase 15 native soak smoke evidence passed; it is not full-campaign evidence.\n'
else
  printf 'Phase 15 native soak passed: 24 hours, five workloads, and bounded resource growth.\n'
fi
