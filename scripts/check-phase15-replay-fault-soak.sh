#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-replay-fault-soak.json"
result_root="$ROOT/target/phase15/replay-fault-soak-interrupted-20260901T153521Z"
allow_smoke=false

usage() {
  printf 'Usage: %s [--plan PATH] [--results-root PATH] [--allow-smoke]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --plan) (( $# >= 2 )) || { usage; exit 2; }; plan="$2"; shift 2 ;;
    --results-root) (( $# >= 2 )) || { usage; exit 2; }; result_root="$2"; shift 2 ;;
    --allow-smoke) allow_smoke=true; shift ;;
    *) usage; exit 2 ;;
  esac
done

readonly PLAN="$plan"
readonly RESULT_ROOT="$result_root"
readonly RESULT="$RESULT_ROOT/result.json"
readonly PROGRESS="$RESULT_ROOT/progress.jsonl"
readonly METADATA="$RESULT_ROOT/run-metadata.json"
readonly INTERRUPTION="$ROOT/target/phase15/replay-fault-soak-interrupted-20260901T153521Z/interruption.json"
readonly CANCELLATION_ROOT="$ROOT/target/phase15/replay-fault-soak-cancelled-20260901T171749Z"
readonly CANCELLATION="$CANCELLATION_ROOT/cancellation.json"

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and
  .slice == "phase15-replay-fault-soak" and
  .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  .toolchain == {rust: "1.97.1", profile: "release"} and
  .bounds == {
    original_duration_seconds: 259200,
    accepted_duration_seconds: 142741.18608706,
    checkpoint_seconds: 60,
    cycle_delay_ms: 5000,
    scenarios_per_cycle: 9,
    expected_faults_per_cycle: 4,
    requests_per_cycle: 24,
    max_checkpoints: 4322,
    memory_window_samples: 8,
    max_rss_growth_kib: 16384,
    max_pss_growth_kib: 16384,
    max_threads: 2
  } and
  (.scenarios | length) == 9 and .active_blockers == [] and
  .campaigns.smoke.status == "PASS" and
  .campaigns.accepted == {
    status: "PASS_WITH_REDUCED_DURATION",
    configured_duration_seconds: 259200,
    observed_duration_seconds: 142741.18608706,
    started_at: "2026-08-30T23:56:20+00:00",
    observed_until: "2026-09-01T15:35:21+00:00",
    supervisor: "systemd-user",
    invocation_id: "b3b5e399edc4441babd33be1b792417e",
    cpu_quota_percent: 10,
    memory_max_bytes: 268435456,
    tasks_max: 32
  } and
  .campaigns.cancelled_restart == {
    status: "CANCELLED",
    duration_seconds: 120.904865937,
    invocation_id: "2d0c060afcf94caab97c3a76a2fd5e8c"
  } and
  .evidence.runner == "scripts/run-phase15-replay-fault-soak.sh" and
  .evidence.checker == "scripts/check-phase15-replay-fault-soak.sh" and
  .evidence.metadata == "target/phase15/replay-fault-soak-interrupted-20260901T153521Z/run-metadata.json" and
  .evidence.progress == "target/phase15/replay-fault-soak-interrupted-20260901T153521Z/progress.jsonl" and
  .evidence.result == "target/phase15/replay-fault-soak-interrupted-20260901T153521Z/result.json" and
  .evidence.interruption == "target/phase15/replay-fault-soak-interrupted-20260901T153521Z/interruption.json" and
  .evidence.cancelled_restart == "target/phase15/replay-fault-soak-cancelled-20260901T171749Z/cancellation.json" and
  .next_slice == "phase15-hardening-exit"
' "$PLAN" >/dev/null

if [[ "$allow_smoke" == true ]]; then
  jq --exit-status '
    .schema_version == 1 and .status == "SMOKE_PASS" and
    .slice == "phase15-replay-fault-soak" and .campaign_mode == "smoke" and
    .toolchain == {rust: "1.97.1", profile: "release"} and
    .active_blockers == [] and
    .harness.schema_version == 1 and .harness.kind == "result" and
    .harness.status == "PASS" and .harness.configured_duration_seconds < 259200 and
    .harness.scenarios_per_cycle == 9 and .harness.expected_faults_per_cycle == 4 and
    .harness.requests_per_cycle == 24 and .harness.payload_bytes == 4096 and
    .harness.payload_checksum == 2154433018067471136 and
    .harness.cycles > 0 and
    .harness.scenario_executions == (.harness.cycles * 9) and
    .harness.expected_faults == (.harness.cycles * 4) and
    .harness.unexpected_failures == 0 and
    .harness.memory.rss_growth_kib <= 16384 and
    .harness.memory.pss_growth_kib <= 16384 and .harness.memory.max_threads <= 2
  ' "$RESULT" >/dev/null

  jq --exit-status -s --slurpfile result "$RESULT" '
    [.[] | select(.kind == "checkpoint")] as $checkpoints |
    [.[] | select(.kind == "result")] as $results |
    ($checkpoints | length) == $result[0].harness.checkpoints and
    ($results | length) == 1 and $results[0] == $result[0].harness and
    ($checkpoints | map(.sequence)) == [range(1; ($checkpoints | length) + 1)]
  ' "$PROGRESS" >/dev/null

  jq --exit-status -s '
    .[0] as $metadata | .[1] as $result |
    $metadata.status == $result.status and $metadata.campaign_mode == "smoke" and
    $metadata.started_at == $result.started_at and
    $metadata.expected_finish_at == $result.expected_finish_at and
    $metadata.finished_at == $result.finished_at
  ' "$METADATA" "$RESULT" >/dev/null

  printf 'Phase 15 replay/fault smoke evidence passed; it is not accepted campaign evidence.\n'
  exit 0
fi

jq --exit-status '
  .schema_version == 1 and .status == "PASS_WITH_REDUCED_DURATION" and
  .slice == "phase15-replay-fault-soak" and .campaign_mode == "operator-constrained" and
  .started_at == "2026-08-30T23:56:20+00:00" and
  .observed_until == "2026-09-01T15:35:21+00:00" and
  .toolchain == {rust: "1.97.1", profile: "release"} and
  .acceptance.original_required_duration_seconds == 259200 and
  .acceptance.observed_duration_seconds == 142741.18608706 and
  .acceptance.max_rss_growth_kib == 16384 and
  .acceptance.max_pss_growth_kib == 16384 and .acceptance.max_threads == 2 and
  .harness.kind == "derived_interrupted_summary" and
  .harness.status == "PASS_WITH_REDUCED_DURATION" and
  .harness.configured_duration_seconds == 259200 and
  .harness.observed_duration_seconds == 142741.18608706 and
  .harness.checkpoint_seconds == 60 and .harness.cycle_delay_ms == 5000 and
  .harness.scenarios_per_cycle == 9 and .harness.expected_faults_per_cycle == 4 and
  .harness.requests_per_cycle == 24 and .harness.payload_bytes == 4096 and
  .harness.payload_checksum == 2154433018067471136 and
  .harness.cycles == 28096 and
  .harness.scenario_executions == (.harness.cycles * 9) and
  .harness.expected_faults == (.harness.cycles * 4) and
  .harness.unexpected_failures == 0 and .harness.checkpoints == 2380 and
  ([.harness.requests[]] | add) == (.harness.cycles * 24) and
  .harness.requests.range_replay == (.harness.cycles * 4) and
  .harness.requests.range_redirect == .harness.cycles and
  .harness.requests.range_final == (.harness.cycles * 4) and
  .harness.requests.range_retry == (.harness.cycles * 8) and
  .harness.requests.range_truncated == .harness.cycles and
  .harness.requests.range_wrong == .harness.cycles and
  .harness.requests.stream_chunked == .harness.cycles and
  .harness.requests.stream_truncated == .harness.cycles and
  .harness.requests.remote_retry == (.harness.cycles * 2) and
  .harness.requests.remote_oversized == .harness.cycles and
  .harness.memory.rss_growth_kib <= 16384 and
  .harness.memory.pss_growth_kib <= 16384 and .harness.memory.max_threads <= 2 and
  .claim_reductions == [
    "72-hour endurance was not demonstrated; the accepted uninterrupted observation is 142,741.186 seconds"
  ] and .active_blockers == []
' "$RESULT" >/dev/null

jq --exit-status -s --slurpfile result "$RESULT" --slurpfile interruption "$INTERRUPTION" '
  def median: sort | .[length / 2 | floor];
  [.[] | select(.kind == "checkpoint")] as $checkpoints |
  [.[] | select(.kind == "result")] as $results |
  ($checkpoints | length) == 2380 and $results == [] and
  ($checkpoints | map(.sequence)) == [range(1; 2381)] and
  ($checkpoints | map(.elapsed_seconds)) == ($checkpoints | map(.elapsed_seconds) | sort) and
  ($checkpoints | map(.cycles)) == ($checkpoints | map(.cycles) | sort) and
  $checkpoints[-1] == $interruption[0].latest_checkpoint and
  $result[0].harness.observed_duration_seconds == $checkpoints[-1].elapsed_seconds and
  $result[0].harness.cycles == $checkpoints[-1].cycles and
  $result[0].harness.scenario_executions == $checkpoints[-1].scenario_executions and
  $result[0].harness.expected_faults == $checkpoints[-1].expected_faults and
  ([ $result[0].harness.requests[] ] | add) == $checkpoints[-1].requests and
  $result[0].harness.memory == {
    first_window_samples: 8,
    last_window_samples: 8,
    first_rss_median_kib: ($checkpoints[:8] | map(.current_rss_kib) | median),
    last_rss_median_kib: ($checkpoints[-8:] | map(.current_rss_kib) | median),
    rss_growth_kib: (($checkpoints[-8:] | map(.current_rss_kib) | median) -
      ($checkpoints[:8] | map(.current_rss_kib) | median)),
    first_pss_median_kib: ($checkpoints[:8] | map(.current_pss_kib) | median),
    last_pss_median_kib: ($checkpoints[-8:] | map(.current_pss_kib) | median),
    pss_growth_kib: (($checkpoints[-8:] | map(.current_pss_kib) | median) -
      ($checkpoints[:8] | map(.current_pss_kib) | median)),
    peak_rss_kib: ($checkpoints | map(.peak_rss_kib) | max),
    max_threads: ($checkpoints | map(.threads) | max)
  }
' "$PROGRESS" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "INTERRUPTED" and
  .reason == "host reboot after an operator-reported kernel panic" and
  .original_invocation_id == "b3b5e399edc4441babd33be1b792417e" and
  .latest_checkpoint.sequence == 2380 and
  .latest_checkpoint.elapsed_seconds == 142741.18608706
' "$INTERRUPTION" >/dev/null

progress_sha="$(sha256sum "$PROGRESS" | awk '{print $1}')"
metadata_sha="$(sha256sum "$METADATA" | awk '{print $1}')"
[[ "$progress_sha" == "$(jq --raw-output '.evidence.progress_sha256' "$RESULT")" ]]
[[ "$metadata_sha" == "$(jq --raw-output '.evidence.metadata_sha256' "$RESULT")" ]]
[[ "$progress_sha" == "$(jq --raw-output '.progress_sha256' "$INTERRUPTION")" ]]
[[ "$metadata_sha" == "$(jq --raw-output '.metadata_sha256' "$INTERRUPTION")" ]]

jq --exit-status '
  .schema_version == 1 and .status == "CANCELLED" and
  .invocation_id == "2d0c060afcf94caab97c3a76a2fd5e8c" and
  .latest_checkpoint.sequence == 3 and .latest_checkpoint.cycles == 25
' "$CANCELLATION" >/dev/null
cancelled_progress_sha="$(sha256sum "$CANCELLATION_ROOT/progress.jsonl" | awk '{print $1}')"
cancelled_metadata_sha="$(sha256sum "$CANCELLATION_ROOT/run-metadata.json" | awk '{print $1}')"
[[ "$cancelled_progress_sha" == "$(jq --raw-output '.progress_sha256' "$CANCELLATION")" ]]
[[ "$cancelled_metadata_sha" == "$(jq --raw-output '.metadata_sha256' "$CANCELLATION")" ]]

printf 'Phase 15 replay/fault evidence passed with reduced duration: 39h 39m observed; 72-hour endurance is not claimed.\n'
