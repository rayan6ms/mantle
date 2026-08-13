#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly SUMMARY_FILE="${1:-$ROOT/docs/performance/results/mantle-worker-phase8-2026-08-13-summary.json}"
readonly OUTPUT_FILE="${2:-$ROOT/docs/performance/results/mantle-worker-phase8-2026-08-13-gate.json}"
readonly ARCHITECTURE="${3:-hybrid}"
readonly REFERENCE_SUMMARY="$ROOT/docs/performance/results/lavaplayer-2.2.6-2026-08-10-summary.json"
readonly REFERENCE_RAW="$ROOT/docs/performance/results/lavaplayer-2.2.6-2026-08-10.jsonl"

for input in "$SUMMARY_FILE" "$REFERENCE_SUMMARY" "$REFERENCE_RAW"; do
  if [[ ! -f "$input" ]]; then
    printf 'worker scale gate requires %s\n' "$input" >&2
    exit 1
  fi
done

jq -n \
  --arg architecture "$ARCHITECTURE" \
  --slurpfile mantle "$SUMMARY_FILE" \
  --slurpfile reference "$REFERENCE_SUMMARY" \
  --slurpfile reference_raw "$REFERENCE_RAW" '
  def median: sort | .[length / 2 | floor];
  def reference_case($workload; $tracks):
    $reference[0][] | select(.workload == $workload and .tracks == $tracks);
  def reference_involuntary($workload; $tracks):
    [$reference_raw[] |
      select(.workload == $workload and .tracks == $tracks) |
      .involuntary_context_switches] | median;
  [
    $mantle[0].cases[] |
    select(.architecture == $architecture and .tracks <= 100 and .workload != "synthetic") as $m |
    reference_case($m.workload; $m.tracks) as $r |
    {
      workload: $m.workload,
      tracks: $m.tracks,
      cpu_core_percent: $m.median_cpu_core_percent,
      cpu_ceiling: ($r.cpu_median *
        (if $m.workload == "opus-passthrough-local" then 1 else 0.8 end)),
      pss_mib: ($m.median_p95_pss_kib / 1024),
      pss_mib_ceiling: ($r.pss_mib_median * 0.75),
      threads: $m.max_threads,
      thread_ceiling: ([$r.threads_median, 32] | min),
      first_frame_ms: $m.p95_first_frame_latency_ms,
      first_frame_ms_ceiling: $r.first_frame_ms_median,
      involuntary_context_switches: $m.median_involuntary_context_switches,
      involuntary_context_switch_ceiling: reference_involuntary($m.workload; $m.tracks),
      frame_underruns: $m.frame_underruns,
      skipped_deadlines: $m.skipped_deadlines
    } |
    . + {
      passed: (
        .cpu_core_percent <= .cpu_ceiling and
        .pss_mib <= .pss_mib_ceiling and
        .threads <= .thread_ceiling and
        .first_frame_ms <= .first_frame_ms_ceiling and
        .involuntary_context_switches <= .involuntary_context_switch_ceiling and
        .frame_underruns == 0 and
        .skipped_deadlines == 0
      )
    }
  ] as $rows |
  {
    schema_version: 1,
    architecture: $architecture,
    expected_matched_rows: 24,
    matched_rows: ($rows | length),
    passed: (($rows | length) == 24 and all($rows[]; .passed)),
    rows: $rows
  }
  ' > "$OUTPUT_FILE"

jq -e '.passed' "$OUTPUT_FILE" >/dev/null
printf 'worker scale gate passed for %s across 24 matched rows\n' "$ARCHITECTURE"

