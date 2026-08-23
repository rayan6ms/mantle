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
  .existing_structural_slice.classes == 49 and
  .existing_structural_slice.symbols == 363 and
  .existing_structural_slice.internal_runtime_classes == 11 and
  (.existing_structural_slice.binary_names | length) == 49 and
  (.existing_structural_slice.binary_names | unique | length) == 49 and
  all(.existing_structural_slice.binary_names[];
    . as $name | any($inv.classes[]; .binary_name == $name)) and
  ([.existing_structural_slice.binary_names[] as $name |
    $inv.classes[] | select(.binary_name == $name) |
    1 + (.fields | length) + (.methods | length)] | add) == 363 and
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
  .cohorts[0].status == "COMPLETE" and
  .cohorts[0].classified_symbols == 535 and
  .cohorts[0].remaining_symbols == 0 and
  (.cohorts[0].completed_slices | length) == 38 and
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
  .cohorts[0].completed_slices[8] == {
    id: "frame-buffer-factory-contracts",
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
  .cohorts[0].completed_slices[9] == {
    id: "audio-frame-buffer-contracts",
    classes: 2,
    fields: 0,
    methods: 12,
    symbols: 14,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[10] == {
    id: "audio-frame-rebuilder-contracts",
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
  .cohorts[0].completed_slices[11] == {
    id: "terminator-audio-frame-contracts",
    classes: 1,
    fields: 1,
    methods: 8,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[12] == {
    id: "reference-mutable-audio-frame-contracts",
    classes: 1,
    fields: 0,
    methods: 8,
    symbols: 9,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[13] == {
    id: "audio-frame-provider-tools-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[14] == {
    id: "audio-processing-context-contracts",
    classes: 1,
    fields: 5,
    methods: 1,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[15] == {
    id: "audio-player-options-contracts",
    classes: 1,
    fields: 3,
    methods: 1,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[16] == {
    id: "decoded-track-holder-contracts",
    classes: 1,
    fields: 1,
    methods: 1,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[17] == {
    id: "track-state-listener-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[18] == {
    id: "audio-output-hook-contracts",
    classes: 2,
    fields: 0,
    methods: 2,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[19] == {
    id: "audio-load-result-handler-contracts",
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
  .cohorts[0].completed_slices[20] == {
    id: "audio-player-lifecycle-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[21] == {
    id: "functional-result-handler-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[22] == {
    id: "audio-player-interface-contracts",
    classes: 1,
    fields: 0,
    methods: 14,
    symbols: 15,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[23] == {
    id: "audio-player-manager-interface-contracts",
    classes: 1,
    fields: 0,
    methods: 27,
    symbols: 28,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[24] == {
    id: "default-audio-player-contracts",
    classes: 1,
    fields: 0,
    methods: 21,
    symbols: 22,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[25] == {
    id: "default-audio-player-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 29,
    symbols: 30,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[26] == {
    id: "internal-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[27] == {
    id: "audio-track-executor-contracts",
    classes: 1,
    fields: 0,
    methods: 10,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[28] == {
    id: "local-audio-track-executor-callback-contracts",
    classes: 2,
    fields: 0,
    methods: 2,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[29] == {
    id: "local-audio-track-executor-contracts",
    classes: 1,
    fields: 0,
    methods: 20,
    symbols: 21,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[30] == {
    id: "track-marker-tracker-contracts",
    classes: 1,
    fields: 0,
    methods: 10,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[31] == {
    id: "base-audio-track-contracts",
    classes: 1,
    fields: 2,
    methods: 25,
    symbols: 28,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[32] == {
    id: "primordial-audio-track-executor-contracts",
    classes: 1,
    fields: 0,
    methods: 16,
    symbols: 17,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[33] == {
    id: "delegated-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[34] == {
    id: "audio-track-info-builder-contracts",
    classes: 1,
    fields: 0,
    methods: 19,
    symbols: 20,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[35] == {
    id: "abstract-audio-frame-buffer-contracts",
    classes: 1,
    fields: 7,
    methods: 8,
    symbols: 16,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[36] == {
    id: "allocating-audio-frame-buffer-contracts",
    classes: 1,
    fields: 0,
    methods: 12,
    symbols: 13,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[0].completed_slices[37] == {
    id: "non-allocating-audio-frame-buffer-contracts",
    classes: 1,
    fields: 0,
    methods: 12,
    symbols: 13,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  ([.cohorts[0].completed_slices[].symbols] | add) == .cohorts[0].classified_symbols and
  (.cohorts[0].classified_symbols + .cohorts[0].remaining_symbols) == .cohorts[0].symbols and
  .cohorts[1].status == "IN_PROGRESS" and
  .cohorts[1].classified_symbols == 385 and
  .cohorts[1].remaining_symbols == 313 and
  (.cohorts[1].completed_slices | length) == 58 and
  .cohorts[1].completed_slices[0] == {
    id: "audio-source-manager-interface-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[1] == {
    id: "audio-source-managers-contracts",
    classes: 1,
    fields: 0,
    methods: 7,
    symbols: 8,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[2] == {
    id: "probing-audio-source-manager-contracts",
    classes: 1,
    fields: 1,
    methods: 5,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[3] == {
    id: "local-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 9,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[4] == {
    id: "local-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[5] == {
    id: "local-seekable-input-stream-contracts",
    classes: 1,
    fields: 0,
    methods: 12,
    symbols: 13,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[6] == {
    id: "heartbeating-http-stream-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "scripts/check-remote-source-status.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0021-bounded-current-niconico-source.md"
    ]
  } and
  .cohorts[1].completed_slices[7] == {
    id: "nico-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 11,
    symbols: 12,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_niconico.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0021-bounded-current-niconico-source.md"
    ]
  } and
  .cohorts[1].completed_slices[8] == {
    id: "nico-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_niconico.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0021-bounded-current-niconico-source.md"
    ]
  } and
  .cohorts[1].completed_slices[9] == {
    id: "default-sound-cloud-data-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[10] == {
    id: "default-sound-cloud-data-reader-contracts",
    classes: 1,
    fields: 0,
    methods: 11,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[11] == {
    id: "default-sound-cloud-format-handler-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[12] == {
    id: "default-sound-cloud-playlist-loader-contracts",
    classes: 1,
    fields: 5,
    methods: 6,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[13] == {
    id: "default-sound-cloud-track-format-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[14] == {
    id: "sound-cloud-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 18,
    symbols: 19,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[15] == {
    id: "sound-cloud-audio-source-manager-builder-contracts",
    classes: 1,
    fields: 0,
    methods: 9,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[16] == {
    id: "sound-cloud-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[17] == {
    id: "sound-cloud-client-id-tracker-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[18] == {
    id: "sound-cloud-data-loader-contracts",
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
  .cohorts[1].completed_slices[19] == {
    id: "sound-cloud-data-reader-contracts",
    classes: 1,
    fields: 0,
    methods: 9,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[20] == {
    id: "sound-cloud-format-handler-contracts",
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
  .cohorts[1].completed_slices[21] == {
    id: "sound-cloud-helper-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT/C_SEMANTIC/D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[22] == {
    id: "sound-cloud-http-context-filter-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[23] == {
    id: "sound-cloud-m3u-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[24] == {
    id: "sound-cloud-m3u-info-contracts",
    classes: 1,
    fields: 2,
    methods: 1,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[25] == {
    id: "sound-cloud-mp3-segment-decoder-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[26] == {
    id: "sound-cloud-opus-segment-decoder-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_soundcloud.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0015-bounded-soundcloud-source.md"
    ]
  } and
  .cohorts[1].completed_slices[27] == {
    id: "sound-cloud-playlist-loader-contracts",
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
  .cohorts[1].completed_slices[28] == {
    id: "sound-cloud-segment-decoder-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[29] == {
    id: "sound-cloud-segment-decoder-factory-contracts",
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
  .cohorts[1].completed_slices[30] == {
    id: "sound-cloud-track-format-contracts",
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
  .cohorts[1].completed_slices[31] == {
    id: "m3u-stream-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[32] == {
    id: "m3u-stream-segment-url-provider-contracts",
    classes: 3,
    fields: 5,
    methods: 16,
    symbols: 24,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[33] == {
    id: "mpeg-ts-m3u-stream-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[34] == {
    id: "twitch-constants-contracts",
    classes: 1,
    fields: 0,
    methods: 1,
    symbols: 2,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[35] == {
    id: "twitch-stream-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 16,
    symbols: 17,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_twitch.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0018-bounded-twitch-live-source.md"
    ]
  } and
  .cohorts[1].completed_slices[36] == {
    id: "twitch-stream-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 7,
    symbols: 8,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_twitch.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0018-bounded-twitch-live-source.md"
    ]
  } and
  .cohorts[1].completed_slices[37] == {
    id: "twitch-stream-segment-url-provider-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_twitch.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0018-bounded-twitch-live-source.md"
    ]
  } and
  .cohorts[1].completed_slices[38] == {
    id: "vimeo-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 12,
    symbols: 13,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_vimeo.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0017-bounded-vimeo-source.md"
    ]
  } and
  .cohorts[1].completed_slices[39] == {
    id: "vimeo-playback-format-contracts",
    classes: 1,
    fields: 2,
    methods: 1,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[40] == {
    id: "vimeo-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_vimeo.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0017-bounded-vimeo-source.md"
    ]
  } and
  .cohorts[1].completed_slices[41] == {
    id: "yandex-abstract-api-loader-contracts",
    classes: 1,
    fields: 1,
    methods: 3,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[42] == {
    id: "yandex-api-extractor-contracts",
    classes: 1,
    fields: 0,
    methods: 1,
    symbols: 2,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[43] == {
    id: "default-yandex-music-direct-url-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[44] == {
    id: "default-yandex-music-playlist-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[45] == {
    id: "default-yandex-music-track-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[46] == {
    id: "default-yandex-search-provider-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[47] == {
    id: "yandex-http-context-filter-contracts",
    classes: 1,
    fields: 0,
    methods: 7,
    symbols: 8,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[48] == {
    id: "yandex-music-api-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[49] == {
    id: "yandex-music-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 20,
    symbols: 21,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[50] == {
    id: "yandex-music-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "A_EXACT/C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_yandex_music.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0014-bounded-yandex-music-source.md"
    ]
  } and
  .cohorts[1].completed_slices[51] == {
    id: "yandex-music-direct-url-loader-contracts",
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
  .cohorts[1].completed_slices[52] == {
    id: "yandex-music-playlist-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[53] == {
    id: "yandex-music-search-result-loader-contracts",
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
  .cohorts[1].completed_slices[54] == {
    id: "yandex-music-track-loader-contracts",
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
  .cohorts[1].completed_slices[55] == {
    id: "yandex-music-utils-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[56] == {
    id: "default-youtube-link-router-contracts",
    classes: 1,
    fields: 0,
    methods: 9,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[57] == {
    id: "default-youtube-playlist-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  ([.cohorts[1].completed_slices[].symbols] | add) == .cohorts[1].classified_symbols and
  (.cohorts[1].classified_symbols + .cohorts[1].remaining_symbols) == .cohorts[1].symbols and
  ([$classifications.symbols[] | select(.assessment == "CLASSIFIED")] | length) == 920 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "A_EXACT")] | length) == 850 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "C_SEMANTIC")] | length) == 64 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "D_LEGACY")] | length) == 6 and
  all($classifications.symbols[] | select(.assessment == "CLASSIFIED");
    . as $symbol |
    (($symbol.binary_name | contains(".player.event.")) or
      any([
        "com.sedmelluq.discord.lavaplayer.track.AudioReference",
        "com.sedmelluq.discord.lavaplayer.player.AudioLoadResultHandler",
        "com.sedmelluq.discord.lavaplayer.player.AudioPlayerLifecycleManager",
        "com.sedmelluq.discord.lavaplayer.player.AudioPlayer",
        "com.sedmelluq.discord.lavaplayer.player.AudioPlayerManager",
        "com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayer",
        "com.sedmelluq.discord.lavaplayer.player.DefaultAudioPlayerManager",
        "com.sedmelluq.discord.lavaplayer.player.FunctionalResultHandler",
        "com.sedmelluq.discord.lavaplayer.player.hook.AudioOutputHook",
        "com.sedmelluq.discord.lavaplayer.player.hook.AudioOutputHookFactory",
        "com.sedmelluq.discord.lavaplayer.track.AudioItem",
        "com.sedmelluq.discord.lavaplayer.track.AudioPlaylist",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrack",
        "com.sedmelluq.discord.lavaplayer.track.InternalAudioTrack",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackInfo",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackEndReason",
        "com.sedmelluq.discord.lavaplayer.track.AudioTrackState",
        "com.sedmelluq.discord.lavaplayer.track.BasicAudioPlaylist",
        "com.sedmelluq.discord.lavaplayer.track.BaseAudioTrack",
        "com.sedmelluq.discord.lavaplayer.track.DecodedTrackHolder",
        "com.sedmelluq.discord.lavaplayer.track.DelegatedAudioTrack",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarker",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarkerHandler$MarkerState",
        "com.sedmelluq.discord.lavaplayer.track.TrackMarkerTracker",
        "com.sedmelluq.discord.lavaplayer.track.TrackStateListener",
        "com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoProvider",
        "com.sedmelluq.discord.lavaplayer.track.info.AudioTrackInfoBuilder",
        "com.sedmelluq.discord.lavaplayer.track.playback.AbstractAudioFrameBuffer",
        "com.sedmelluq.discord.lavaplayer.track.playback.AllocatingAudioFrameBuffer",
        "com.sedmelluq.discord.lavaplayer.track.playback.NonAllocatingAudioFrameBuffer",
        "com.sedmelluq.discord.lavaplayer.track.playback.AbstractMutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBuffer",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameBufferFactory",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameConsumer",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProvider",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameProviderTools",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioProcessingContext",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioTrackExecutor",
        "com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor",
        "com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor$ReadExecutor",
        "com.sedmelluq.discord.lavaplayer.track.playback.LocalAudioTrackExecutor$SeekExecutor",
        "com.sedmelluq.discord.lavaplayer.track.playback.AudioFrameRebuilder",
        "com.sedmelluq.discord.lavaplayer.track.playback.ImmutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.MutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.PrimordialAudioTrackExecutor",
        "com.sedmelluq.discord.lavaplayer.track.playback.ReferenceMutableAudioFrame",
        "com.sedmelluq.discord.lavaplayer.track.playback.TerminatorAudioFrame",
        "com.sedmelluq.discord.lavaplayer.player.AudioConfiguration",
        "com.sedmelluq.discord.lavaplayer.player.AudioConfiguration$ResamplingQuality",
        "com.sedmelluq.discord.lavaplayer.player.AudioPlayerOptions",
        "com.sedmelluq.discord.lavaplayer.source.AudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.AudioSourceManagers",
        "com.sedmelluq.discord.lavaplayer.source.ProbingAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalSeekableInputStream",
        "com.sedmelluq.discord.lavaplayer.source.nico.HeartbeatingHttpStream",
        "com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataLoader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudDataReader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudFormatHandler",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudPlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.DefaultSoundCloudTrackFormat",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioSourceManager$Builder",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudClientIdTracker",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataLoader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudDataReader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudFormatHandler",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHelper",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHttpContextFilter",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uInfo",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudMp3SegmentDecoder",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudOpusSegmentDecoder",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudPlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudSegmentDecoder$Factory",
        "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudTrackFormat",
        "com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider",
        "com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider$ChannelStreamInfo",
        "com.sedmelluq.discord.lavaplayer.source.stream.M3uStreamSegmentUrlProvider$SegmentInfo",
        "com.sedmelluq.discord.lavaplayer.source.stream.MpegTsM3uStreamAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchConstants",
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamSegmentUrlProvider",
        "com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager$PlaybackFormat",
        "com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader$ApiExtractor",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicDirectUrlLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicPlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicTrackLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexSearchProvider",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexHttpContextFilter",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicApiLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicDirectUrlLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicPlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicSearchResultLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicTrackLoader",
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicUtils",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeLinkRouter",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubePlaylistLoader"
      ][]; . == $symbol.binary_name)) and
    (if $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubePlaylistLoader" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "load")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexMusicAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadItem")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.YandexHttpContextFilter" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "onRequest")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexSearchProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadSearchResult")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicTrackLoader" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadTrack")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicPlaylistLoader" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadPlaylist")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.DefaultYandexMusicDirectUrlLoader" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "getDirectUrl")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.yamusic.AbstractYandexMusicApiLoader" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "extractFromApi")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_yandex_music.rs")) != null and
        (.tests | index("docs/architecture/ADR-0014-bounded-yandex-music-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_vimeo.rs")) != null and
        (.tests | index("docs/architecture/ADR-0017-bounded-vimeo-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.vimeo.VimeoAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or
          ($symbol.member_name | IN("loadItem", "getVideoFromApi", "getPlaybackFormat")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_vimeo.rs")) != null and
        (.tests | index("docs/architecture/ADR-0017-bounded-vimeo-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamSegmentUrlProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "fetchSegmentPlaylistUrl")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_twitch.rs")) != null and
        (.tests | index("docs/architecture/ADR-0018-bounded-twitch-live-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_twitch.rs")) != null and
        (.tests | index("docs/architecture/ADR-0018-bounded-twitch-live-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.twitch.TwitchStreamAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or
          ($symbol.member_name | IN("<init>", "getClientId", "getDeviceId", "loadItem",
            "getHttpInterface", "configureRequests", "configureBuilder", "fetchAccessToken")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_twitch.rs")) != null and
        (.tests | index("docs/architecture/ADR-0018-bounded-twitch-live-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.nico.HeartbeatingHttpStream"
      then .classification == "D_LEGACY" and
        (.tests | index("scripts/check-remote-source-status.sh")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioSourceManager" and
          ($symbol.symbol_kind == "CLASS" or
            ($symbol.member_name == "<init>" and
              $symbol.descriptor == "(Ljava/lang/String;Ljava/lang/String;)V") or
            ($symbol.member_name | IN("loadItem", "getHttpInterface", "configureRequests", "configureBuilder")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_niconico.rs")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.nico.NicoAudioTrack" and
          ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_niconico.rs")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudAudioTrack" and
          ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudClientIdTracker" and
          ($symbol.symbol_kind == "CLASS" or
            ($symbol.member_name | IN("updateClientId", "getClientId")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHelper" and
          $symbol.member_name == "redirectMobileLink"
      then .classification == "D_LEGACY" and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHelper" and
          ($symbol.symbol_kind == "CLASS" or
            ($symbol.member_name | IN("loadPlaybackUrl", "resolveShortTrackUrl")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudHttpContextFilter" and
          ($symbol.symbol_kind == "CLASS" or
            ($symbol.member_name | IN("onRequest", "onRequestResponse")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudM3uAudioTrack" and
          ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudMp3SegmentDecoder" and
          ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "playStream")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      elif $symbol.binary_name ==
          "com.sedmelluq.discord.lavaplayer.source.soundcloud.SoundCloudOpusSegmentDecoder" and
          ($symbol.symbol_kind == "CLASS" or
            ($symbol.member_name | IN("prepareStream", "playStream")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_soundcloud.rs")) != null and
        (.tests | index("docs/architecture/ADR-0015-bounded-soundcloud-source.md")) != null
      else .classification == "A_EXACT"
      end) and
    (.tests | index("scripts/run-jvm-gate-a.sh")) != null) and
  .phase_entry.first_execution_cohort == .cohorts[0].id and
  .phase_entry.next_slice == "default-youtube-track-details-contracts" and
  (.phase_entry.precondition | contains("Phase 12")) and
  (.phase_entry.phase_exit | contains("Revapi"))
' "$PLAN" >/dev/null

for required in \
  '399 exported classes' \
  '2,762 symbols' \
  '126 reference classes / 949 symbols' \
  'C_SEMANTIC' \
  'D_LEGACY' \
  'core-player-track' \
  'Phase 12'; do
  grep --fixed-strings "$required" "$DOCUMENT" >/dev/null
done

"$ROOT/scripts/check-no-jvm-source.sh"

printf 'Phase 13 inventory tracks 920 classified symbols and 1,842 unassessed symbols.\n'
