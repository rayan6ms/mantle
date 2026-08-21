#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly PLAN="$ROOT/compatibility/phase13-plan.json"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly LEDGER="$ROOT/compatibility/lavaplayer-2.2.6-classification.json"
readonly DOCUMENT="$ROOT/docs/compatibility/PHASE13_JVM_INVENTORY.md"

jq --exit-status --slurpfile inventory "$INVENTORY" --slurpfile ledger "$LEDGER" '
  def symbol_records:
    [.classes[] as $class |
      {binary_name: $class.binary_name, symbol_kind: "CLASS", member_name: null, descriptor: null},
      ($class.fields[] | {
        binary_name: $class.binary_name,
        symbol_kind: "FIELD",
        member_name: .name,
        descriptor: .descriptor
      }),
      ($class.methods[] | {
        binary_name: $class.binary_name,
        symbol_kind: "METHOD",
        member_name: .name,
        descriptor: .descriptor
      })];
  def symbol_sort: sort_by(.binary_name, .symbol_kind, .member_name, .descriptor);
  def group_matches($group; $name):
    if $group.selector == "prefix" then $name | startswith($group.value)
    elif $group.selector == "exact" then $name == $group.value
    else false
    end;

  $inventory[0] as $inv |
  $ledger[0] as $classifications |
  ($inv | symbol_records) as $inventory_symbols |
  . as $plan |
  .schema_version == 1 and
  .status == "IN_PROGRESS" and
  .compatibility_baseline == "lavaplayer-2.2.6" and
  .inventory == "reference/lavaplayer-2.2.6-inventory.json" and
  .classification_ledger == "compatibility/lavaplayer-2.2.6-classification.json" and
  .totals == {
    classes: $inv.counts.exported_classes,
    fields: $inv.counts.exported_fields,
    methods: $inv.counts.exported_methods,
    symbols: ($inv.counts.exported_classes + $inv.counts.exported_fields +
      $inv.counts.exported_methods)
  } and
  .classification_policy.allowed ==
    ["A_EXACT", "B_SOURCE", "C_SEMANTIC", "D_LEGACY", "X_UNSUPPORTED"] and
  (.classification_policy.rules | length) == 4 and
  (.cohorts | length) == 6 and
  [.cohorts[].order] == [1,2,3,4,5,6] and
  ([.cohorts[].id] | unique | length) == 6 and
  ([.cohorts[].package_roots[]] | sort) ==
    ["container", "filter", "format", "natives", "player", "source", "tools", "track"] and
  ([.cohorts[].package_roots[]] | unique | length) == 8 and
  all(.cohorts[];
    . as $cohort |
    [$inv.classes[] |
      (.binary_name | split(".")[4]) as $root |
      select($cohort.package_roots | index($root))] as $classes |
    $cohort.classes == ($classes | length) and
    $cohort.fields == ($classes | map(.fields | length) | add) and
    $cohort.methods == ($classes | map(.methods | length) | add) and
    $cohort.symbols == ($classes |
      map(1 + (.fields | length) + (.methods | length)) | add) and
    ($cohort.objective | length) > 0 and ($cohort.exit_evidence | length) > 0) and
  ([.cohorts[] | {classes, fields, methods, symbols}] |
    reduce .[] as $counts ({classes: 0, fields: 0, methods: 0, symbols: 0};
      .classes += $counts.classes |
      .fields += $counts.fields |
      .methods += $counts.methods |
      .symbols += $counts.symbols)) == .totals and
  .existing_structural_slice.classes == 37 and
  .existing_structural_slice.symbols == 303 and
  .existing_structural_slice.internal_runtime_classes == 11 and
  (.existing_structural_slice.binary_names | length) == 37 and
  (.existing_structural_slice.binary_names | unique | length) == 37 and
  all(.existing_structural_slice.binary_names[];
    . as $name | any($inv.classes[]; .binary_name == $name)) and
  ([.existing_structural_slice.binary_names[] as $name |
    $inv.classes[] | select(.binary_name == $name) |
    1 + (.fields | length) + (.methods | length)] | add) == 303 and
  .artifact_workstreams.resources.expected_count == $inv.counts.non_class_resources and
  (.artifact_workstreams.resources.paths | sort) == ([$inv.resources[].path] | sort) and
  .artifact_workstreams.pom_dependencies.expected_count == $inv.counts.pom_dependencies and
  .artifact_workstreams.external_public_types.expected_count ==
    $inv.counts.external_public_types and
  ([.artifact_workstreams.external_public_types.groups[].expected_count] | add) ==
    $inv.counts.external_public_types and
  all(.artifact_workstreams.external_public_types.groups[];
    . as $group |
    $group.expected_count ==
      ([$inv.external_public_types[] as $name |
        select(group_matches($group; $name))] | length)) and
  all($inv.external_public_types[];
    . as $name |
    ([$plan.artifact_workstreams.external_public_types.groups[] |
      select(group_matches(.; $name))] | length) == 1) and
  ($classifications.symbols | map({binary_name, symbol_kind, member_name, descriptor}) |
    symbol_sort) == ($inventory_symbols | symbol_sort) and
  ($classifications.symbols | length) == .totals.symbols and
  all($classifications.symbols[];
    if .assessment == "CLASSIFIED" then
      (.classification as $classification |
        ((.tests | length) > 0) and ((.notes | length) > 0) and
        any(["A_EXACT", "B_SOURCE", "C_SEMANTIC", "D_LEGACY", "X_UNSUPPORTED"][];
          . == $classification))
    else
      .assessment == "UNASSESSED" and (has("classification") | not)
    end) and
  .cohorts[0].status == "IN_PROGRESS" and
  .cohorts[0].classified_symbols == 196 and
  .cohorts[0].remaining_symbols == 339 and
  (.cohorts[0].completed_slices | length) == 8 and
  .cohorts[0].completed_slices[0] == {
    id: "player-events",
    classes: 9,
    fields: 9,
    methods: 17,
    symbols: 35,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[1] == {
    id: "track-values",
    classes: 4,
    fields: 14,
    methods: 17,
    symbols: 35,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[2] == {
    id: "track-enums",
    classes: 3,
    fields: 19,
    methods: 6,
    symbols: 28,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[3] == {
    id: "track-info-contracts",
    classes: 3,
    fields: 0,
    methods: 23,
    symbols: 26,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[4] == {
    id: "playlist-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[5] == {
    id: "marker-handler-contracts",
    classes: 1,
    fields: 0,
    methods: 1,
    symbols: 2,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[6] == {
    id: "audio-frame-contracts",
    classes: 5,
    fields: 4,
    methods: 36,
    symbols: 45,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[7] == {
    id: "audio-configuration-contracts",
    classes: 2,
    fields: 4,
    methods: 14,
    symbols: 20,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  ([.cohorts[0].completed_slices[].symbols] | add) == .cohorts[0].classified_symbols and
  (.cohorts[0].classified_symbols + .cohorts[0].remaining_symbols) == .cohorts[0].symbols and
  ([$classifications.symbols[] | select(.assessment == "CLASSIFIED")] | length) == 196 and
  all($classifications.symbols[] | select(.assessment == "CLASSIFIED");
    . as $symbol |
    (($symbol.binary_name | contains(".player.event.")) or
      any([
        "com.sedmelluq.discord.lavaplayer.track.AudioReference",
        "com.sedmelluq.discord.lavaplayer.track.AudioItem",
        "com.sedmelluq.discord.lavaplayer.track.AudioPlaylist",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrack",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackEndReason",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackState",
        "com.sedmelluq.discord.lavaplayer.track.BasicAudioPlaylist",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarker",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler$MarkerState",
        "com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider",
        "com.sedmelluq.discord.lavaplayer.track.playback.AbstractMutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider",
        "com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.player.AudioConfiguration",
        "com.sedmelluq.discord.lavaplayer.player.AudioConfiguration$ResamplingQuality"
      ][]; . == $symbol.binary_name)) and
    .classification == "A_EXACT" and
    (.tests | index("scripts/run-jvm-gate-a.sh")) != null) and
  .phase_entry.first_execution_cohort == .cohorts[0].id and
  .phase_entry.next_slice == "frame-buffer-factory-contracts" and
  (.phase_entry.precondition | contains("Phase 12")) and
  (.phase_entry.phase_exit | contains("Revapi"))
' "$PLAN" >/dev/null

for required in \
  '399 exported classes' \
  '2,762 symbols' \
  '37 reference classes / 303 symbols' \
  'core-player-track' \
  'Phase 12'; do
  grep --fixed-strings "$required" "$DOCUMENT" >/dev/null
done

"$ROOT/scripts/check-no-jvm-source.sh"

printf 'Phase 13 inventory tracks 196 classified core-player-track symbols and 2,566 unassessed symbols.\n'
