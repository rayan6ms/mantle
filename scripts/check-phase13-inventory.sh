#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly PLAN="$ROOT/compatibility/phase13-plan.json"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly LEDGER="$ROOT/compatibility/lavaplayer-2.2.6-classification.json"
readonly DOCUMENT="$ROOT/docs/compatibility/PHASE13_JVM_INVENTORY.md"

jq --exit-status --slurpfile inventory "$INVENTORY" --slurpfile ledger "$LEDGER" \
  --from-file /dev/stdin "$PLAN" >/dev/null <<'JQ'
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
  .existing_structural_slice.classes == 60 and
  .existing_structural_slice.symbols == 498 and
  .existing_structural_slice.internal_runtime_classes == 11 and
  (.existing_structural_slice.binary_names | length) == 60 and
  (.existing_structural_slice.binary_names | unique | length) == 60 and
  all(.existing_structural_slice.binary_names[];
    . as $name | any($inv.classes[]; .binary_name == $name)) and
  ([.existing_structural_slice.binary_names[] as $name |
    $inv.classes[] | select(.binary_name == $name) |
    1 + (.fields | length) + (.methods | length)] | add) == 498 and
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
  .cohorts[1].status == "COMPLETE" and
  .cohorts[1].classified_symbols == 698 and
  .cohorts[1].remaining_symbols == 0 and
  (.cohorts[1].completed_slices | length) == 104 and
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
  .cohorts[1].completed_slices[58] == {
    id: "default-youtube-track-details-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[59] == {
    id: "default-youtube-track-details-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 7,
    symbols: 8,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[60] == {
    id: "youtube-cached-player-script-contracts",
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
  .cohorts[1].completed_slices[61] == {
    id: "youtube-info-status-contracts",
    classes: 1,
    fields: 7,
    methods: 2,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[62] == {
    id: "youtube-access-token-tracker-contracts",
    classes: 1,
    fields: 0,
    methods: 8,
    symbols: 9,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[63] == {
    id: "youtube-cached-auth-script-contracts",
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
  .cohorts[1].completed_slices[64] == {
    id: "youtube-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 21,
    symbols: 22,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[65] == {
    id: "youtube-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[66] == {
    id: "youtube-cipher-operation-contracts",
    classes: 2,
    fields: 6,
    methods: 3,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[67] == {
    id: "youtube-client-config-contracts",
    classes: 2,
    fields: 6,
    methods: 20,
    symbols: 28,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[68] == {
    id: "youtube-constants-contracts",
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
  .cohorts[1].completed_slices[69] == {
    id: "youtube-format-info-contracts",
    classes: 1,
    fields: 7,
    methods: 3,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[70] == {
    id: "youtube-http-context-filter-contracts",
    classes: 1,
    fields: 1,
    methods: 7,
    symbols: 9,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[71] == {
    id: "youtube-link-router-contracts",
    classes: 2,
    fields: 0,
    methods: 8,
    symbols: 10,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[72] == {
    id: "youtube-mix-loader-contracts",
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
  .cohorts[1].completed_slices[73] == {
    id: "youtube-mix-provider-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[74] == {
    id: "youtube-mpeg-stream-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[75] == {
    id: "youtube-payload-helper-contracts",
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
  .cohorts[1].completed_slices[76] == {
    id: "youtube-persistent-http-stream-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/src/http_input.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[77] == {
    id: "youtube-playlist-loader-contracts",
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
  .cohorts[1].completed_slices[78] == {
    id: "youtube-search-music-provider-contracts",
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
  .cohorts[1].completed_slices[79] == {
    id: "youtube-search-music-result-loader-contracts",
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
  .cohorts[1].completed_slices[80] == {
    id: "youtube-search-provider-contracts",
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
  .cohorts[1].completed_slices[81] == {
    id: "youtube-search-result-loader-contracts",
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
  .cohorts[1].completed_slices[82] == {
    id: "youtube-signature-cipher-contracts",
    classes: 1,
    fields: 0,
    methods: 8,
    symbols: 9,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[83] == {
    id: "youtube-signature-cipher-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-media/tests/phase12_youtube.rs",
      "docs/architecture/ADR-0013-ordered-youtube-client-foundation.md"
    ]
  } and
  .cohorts[1].completed_slices[84] == {
    id: "youtube-signature-resolver-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[85] == {
    id: "youtube-track-details-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[86] == {
    id: "youtube-track-details-loader-contracts",
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
  .cohorts[1].completed_slices[87] == {
    id: "youtube-track-format-contracts",
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
  .cohorts[1].completed_slices[88] == {
    id: "youtube-track-json-data-contracts",
    classes: 1,
    fields: 3,
    methods: 3,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[1].completed_slices[89] == {
    id: "legacy-adaptive-formats-extractor-contracts",
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
  .cohorts[1].completed_slices[90] == {
    id: "legacy-dash-mpd-formats-extractor-contracts",
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
  .cohorts[1].completed_slices[91] == {
    id: "legacy-stream-map-formats-extractor-contracts",
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
  .cohorts[1].completed_slices[92] == {
    id: "offline-youtube-track-format-extractor-contracts",
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
  .cohorts[1].completed_slices[93] == {
    id: "streaming-data-formats-extractor-contracts",
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
  .cohorts[1].completed_slices[94] == {
    id: "youtube-track-format-extractor-contracts",
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
  .cohorts[1].completed_slices[95] == {
    id: "bandcamp-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 11,
    symbols: 12,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_bandcamp.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0016-bounded-bandcamp-source.md"
    ]
  } and
  .cohorts[1].completed_slices[96] == {
    id: "bandcamp-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_bandcamp.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0016-bounded-bandcamp-source.md"
    ]
  } and
  .cohorts[1].completed_slices[97] == {
    id: "beam-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 10,
    symbols: 11,
    classification: "MIXED_A_EXACT_D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_beam.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0019-compatibility-only-beam-source.md"
    ]
  } and
  .cohorts[1].completed_slices[98] == {
    id: "beam-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "MIXED_A_EXACT_D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_beam.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0019-compatibility-only-beam-source.md"
    ]
  } and
  .cohorts[1].completed_slices[99] == {
    id: "beam-segment-url-provider-contracts",
    classes: 1,
    fields: 0,
    methods: 4,
    symbols: 5,
    classification: "MIXED_A_EXACT_D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-media/tests/phase12_beam.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0019-compatibility-only-beam-source.md"
    ]
  } and
  .cohorts[1].completed_slices[100] == {
    id: "getyarn-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 10,
    symbols: 11,
    classification: "MIXED_A_EXACT_D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase12_getyarn.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0020-compatibility-only-getyarn-source.md"
    ]
  } and
  .cohorts[1].completed_slices[101] == {
    id: "getyarn-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "MIXED_A_EXACT_D_LEGACY",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase12_getyarn.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0020-compatibility-only-getyarn-source.md"
    ]
  } and
  .cohorts[1].completed_slices[102] == {
    id: "http-audio-source-manager-contracts",
    classes: 1,
    fields: 0,
    methods: 13,
    symbols: 14,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/load_bridge.rs",
      "crates/mantle-media/tests/phase11_sources.rs",
      "crates/mantle-media/tests/phase12_remote_http.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0005-bounded-blocking-http-media-input.md"
    ]
  } and
  .cohorts[1].completed_slices[103] == {
    id: "http-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "crates/mantle-jvm/src/playback_bridge.rs",
      "crates/mantle-media/tests/phase6_http.rs",
      "crates/mantle-media/tests/phase11_sources.rs",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0005-bounded-blocking-http-media-input.md"
    ]
  } and
  ([.cohorts[1].completed_slices[].symbols] | add) == .cohorts[1].classified_symbols and
  (.cohorts[1].classified_symbols + .cohorts[1].remaining_symbols) == .cohorts[1].symbols and
  .cohorts[2].status == "COMPLETE" and
  .cohorts[2].classified_symbols == 219 and
  .cohorts[2].remaining_symbols == 0 and
  (.cohorts[2].completed_slices | length) == 35 and
  .cohorts[2].completed_slices[0] == {
    id: "audio-filter-interface-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[1] == {
    id: "audio-filter-chain-contracts",
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
  .cohorts[2].completed_slices[2] == {
    id: "audio-pipeline-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[3] == {
    id: "audio-pipeline-factory-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[4] == {
    id: "audio-post-processor-contracts",
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
  .cohorts[2].completed_slices[5] == {
    id: "buffering-post-processor-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[6] == {
    id: "channel-count-pcm-audio-filter-contracts",
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
  .cohorts[2].completed_slices[7] == {
    id: "composite-audio-filter-contracts",
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
  .cohorts[2].completed_slices[8] == {
    id: "filter-chain-builder-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[9] == {
    id: "final-pcm-audio-filter-contracts",
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
  .cohorts[2].completed_slices[10] == {
    id: "float-pcm-audio-filter-contracts",
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
  .cohorts[2].completed_slices[11] == {
    id: "pcm-filter-factory-contracts",
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
  .cohorts[2].completed_slices[12] == {
    id: "pcm-format-contracts",
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
  .cohorts[2].completed_slices[13] == {
    id: "resampling-pcm-audio-filter-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "crates/mantle-audio/src/resample.rs",
      "docs/architecture/ADR-0007-bounded-pcm-transforms.md"
    ]
  } and
  .cohorts[2].completed_slices[14] == {
    id: "short-pcm-audio-filter-contracts",
    classes: 2,
    fields: 0,
    methods: 3,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[15] == {
    id: "universal-pcm-audio-filter-contracts",
    classes: 1,
    fields: 0,
    methods: 0,
    symbols: 1,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[16] == {
    id: "user-provided-audio-filters-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[17] == {
    id: "converter-audio-filter-contracts",
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
  .cohorts[2].completed_slices[18] == {
    id: "to-float-audio-filter-contracts",
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
  .cohorts[2].completed_slices[19] == {
    id: "to-short-audio-filter-contracts",
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
  .cohorts[2].completed_slices[20] == {
    id: "to-split-short-audio-filter-contracts",
    classes: 1,
    fields: 0,
    methods: 5,
    symbols: 6,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0022-bounded-split-short-converter.md"
    ]
  } and
  .cohorts[2].completed_slices[21] == {
    id: "equalizer-contracts",
    classes: 3,
    fields: 2,
    methods: 12,
    symbols: 17,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[22] == {
    id: "volume-contracts",
    classes: 3,
    fields: 0,
    methods: 9,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[23] == {
    id: "audio-data-format-contracts",
    classes: 1,
    fields: 3,
    methods: 11,
    symbols: 15,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[24] == {
    id: "audio-data-format-tools-contracts",
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
  .cohorts[2].completed_slices[25] == {
    id: "audio-player-input-stream-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[26] == {
    id: "opus-audio-data-format-contracts",
    classes: 1,
    fields: 1,
    methods: 9,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[27] == {
    id: "pcm16-audio-data-format-contracts",
    classes: 1,
    fields: 2,
    methods: 9,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[28] == {
    id: "standard-audio-data-formats-contracts",
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
  .cohorts[2].completed_slices[29] == {
    id: "audio-chunk-decoder-contracts",
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
  .cohorts[2].completed_slices[30] == {
    id: "audio-chunk-encoder-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[31] == {
    id: "opus-chunk-decoder-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[32] == {
    id: "opus-chunk-encoder-contracts",
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
  .cohorts[2].completed_slices[33] == {
    id: "pcm-chunk-decoder-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[2].completed_slices[34] == {
    id: "pcm-chunk-encoder-contracts",
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
  ([.cohorts[2].completed_slices[].symbols] | add) == .cohorts[2].classified_symbols and
  (.cohorts[2].classified_symbols + .cohorts[2].remaining_symbols) == .cohorts[2].symbols and
  .cohorts[3].status == "IN_PROGRESS" and
  .cohorts[3].classified_symbols == 547 and
  .cohorts[3].remaining_symbols == 277 and
  (.cohorts[3].completed_slices | length) == 65 and
  .cohorts[3].completed_slices[0] == {
    id: "formats-contracts",
    classes: 1,
    fields: 7,
    methods: 1,
    symbols: 9,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[25] == {
    id: "flac-frame-header-reader-contracts",
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
  .cohorts[3].completed_slices[26] == {
    id: "flac-frame-info-contracts",
    classes: 2,
    fields: 7,
    methods: 3,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[27] == {
    id: "flac-frame-reader-contracts",
    classes: 1,
    fields: 1,
    methods: 2,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[28] == {
    id: "flac-sub-frame-reader-contracts",
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
  .cohorts[3].completed_slices[29] == {
    id: "matroska-aac-track-consumer-contracts",
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
  .cohorts[3].completed_slices[30] == {
    id: "matroska-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 2,
    symbols: 3,
    classification: "MIXED_A_EXACT_C_SEMANTIC",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs",
      "docs/architecture/ADR-0023-bounded-matroska-track-selection.md"
    ]
  } and
  .cohorts[3].completed_slices[31] == {
    id: "matroska-container-probe-contracts",
    classes: 1,
    fields: 3,
    methods: 5,
    symbols: 9,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[32] == {
    id: "matroska-opus-track-consumer-contracts",
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
  .cohorts[3].completed_slices[33] == {
    id: "matroska-streaming-file-contracts",
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
  .cohorts[3].completed_slices[34] == {
    id: "matroska-track-consumer-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[35] == {
    id: "matroska-vorbis-track-consumer-contracts",
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
  .cohorts[3].completed_slices[36] == {
    id: "matroska-block-contracts",
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
  .cohorts[3].completed_slices[37] == {
    id: "matroska-cue-point-contracts",
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
  .cohorts[3].completed_slices[38] == {
    id: "matroska-ebml-reader-contracts",
    classes: 2,
    fields: 3,
    methods: 6,
    symbols: 11,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[39] == {
    id: "matroska-element-contracts",
    classes: 1,
    fields: 6,
    methods: 12,
    symbols: 19,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[40] == {
    id: "matroska-element-type-contracts",
    classes: 2,
    fields: 54,
    methods: 5,
    symbols: 61,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[41] == {
    id: "matroska-file-reader-contracts",
    classes: 1,
    fields: 0,
    methods: 14,
    symbols: 15,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[42] == {
    id: "matroska-file-track-contracts",
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
  .cohorts[3].completed_slices[43] == {
    id: "matroska-mutable-element-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[44] == {
    id: "mp3-audio-track-contracts",
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
  .cohorts[3].completed_slices[45] == {
    id: "mp3-constant-rate-seeker-contracts",
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
  .cohorts[3].completed_slices[46] == {
    id: "mp3-container-probe-contracts",
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
  .cohorts[3].completed_slices[47] == {
    id: "mp3-frame-reader-contracts",
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
  .cohorts[3].completed_slices[48] == {
    id: "mp3-seeker-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[49] == {
    id: "mp3-stream-seeker-contracts",
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
  .cohorts[3].completed_slices[50] == {
    id: "mp3-track-provider-contracts",
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
  .cohorts[3].completed_slices[51] == {
    id: "mp3-xing-seeker-contracts",
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
  .cohorts[3].completed_slices[52] == {
    id: "mpeg-aac-track-consumer-contracts",
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
  .cohorts[3].completed_slices[53] == {
    id: "mpeg-audio-track-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[54] == {
    id: "mpeg-container-probe-contracts",
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
  .cohorts[3].completed_slices[55] == {
    id: "mpeg-file-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[56] == {
    id: "mpeg-noop-track-consumer-contracts",
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
  .cohorts[3].completed_slices[57] == {
    id: "mpeg-track-consumer-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[58] == {
    id: "mpeg-track-info-contracts",
    classes: 1,
    fields: 6,
    methods: 1,
    symbols: 8,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[59] == {
    id: "mpeg-track-info-builder-contracts",
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
  .cohorts[3].completed_slices[60] == {
    id: "mpeg-file-track-provider-contracts",
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
  .cohorts[3].completed_slices[61] == {
    id: "mpeg-parse-stop-checker-contracts",
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
  .cohorts[3].completed_slices[62] == {
    id: "mpeg-reader-contracts",
    classes: 1,
    fields: 2,
    methods: 9,
    symbols: 12,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[63] == {
    id: "mpeg-reader-chain-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[64] == {
    id: "mpeg-section-handler-contracts",
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
  .cohorts[3].completed_slices[1] == {
    id: "media-container-contracts",
    classes: 1,
    fields: 12,
    methods: 3,
    symbols: 16,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[2] == {
    id: "media-container-descriptor-contracts",
    classes: 1,
    fields: 2,
    methods: 2,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[3] == {
    id: "media-container-detection-contracts",
    classes: 1,
    fields: 3,
    methods: 5,
    symbols: 9,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[4] == {
    id: "media-container-detection-result-contracts",
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
  .cohorts[3].completed_slices[5] == {
    id: "media-container-hints-contracts",
    classes: 1,
    fields: 2,
    methods: 2,
    symbols: 5,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[6] == {
    id: "media-container-probe-contracts",
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
  .cohorts[3].completed_slices[7] == {
    id: "media-container-registry-contracts",
    classes: 1,
    fields: 1,
    methods: 4,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[8] == {
    id: "adts-audio-track-contracts",
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
  .cohorts[3].completed_slices[9] == {
    id: "adts-container-probe-contracts",
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
  .cohorts[3].completed_slices[10] == {
    id: "adts-packet-header-contracts",
    classes: 1,
    fields: 5,
    methods: 2,
    symbols: 8,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[11] == {
    id: "adts-stream-provider-contracts",
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
  .cohorts[3].completed_slices[12] == {
    id: "adts-stream-reader-contracts",
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
  .cohorts[3].completed_slices[13] == {
    id: "aac-packet-router-contracts",
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
  .cohorts[3].completed_slices[14] == {
    id: "opus-packet-router-contracts",
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
  .cohorts[3].completed_slices[15] == {
    id: "flac-audio-track-contracts",
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
  .cohorts[3].completed_slices[16] == {
    id: "flac-container-probe-contracts",
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
  .cohorts[3].completed_slices[17] == {
    id: "flac-file-loader-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[18] == {
    id: "flac-metadata-header-contracts",
    classes: 1,
    fields: 6,
    methods: 1,
    symbols: 8,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[19] == {
    id: "flac-metadata-reader-contracts",
    classes: 1,
    fields: 0,
    methods: 3,
    symbols: 4,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[20] == {
    id: "flac-seek-point-contracts",
    classes: 1,
    fields: 4,
    methods: 1,
    symbols: 6,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[21] == {
    id: "flac-stream-info-contracts",
    classes: 1,
    fields: 11,
    methods: 1,
    symbols: 13,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[22] == {
    id: "flac-track-info-contracts",
    classes: 1,
    fields: 6,
    methods: 1,
    symbols: 8,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[23] == {
    id: "flac-track-info-builder-contracts",
    classes: 1,
    fields: 0,
    methods: 6,
    symbols: 7,
    classification: "A_EXACT",
    evidence: [
      "scripts/run-jvm-gate-a.sh",
      "tools/jvm-gate/src/emitter.rs",
      "tools/jvm-gate/src/main.rs"
    ]
  } and
  .cohorts[3].completed_slices[24] == {
    id: "flac-track-provider-contracts",
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
  ([.cohorts[3].completed_slices[].symbols] | add) == .cohorts[3].classified_symbols and
  (.cohorts[3].classified_symbols + .cohorts[3].remaining_symbols) == .cohorts[3].symbols and
  ([$classifications.symbols[] | select(.assessment == "CLASSIFIED")] | length) == 1999 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "A_EXACT")] | length) == 1852 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegReader$Chain" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegSectionHandler" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 2 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "C_SEMANTIC")] | length) == 131 and
  ([$classifications.symbols[] |
    select(.assessment == "CLASSIFIED" and .classification == "D_LEGACY")] | length) == 16 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.common.AacPacketRouter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.common.OpusPacketRouter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaTrackConsumer" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaVorbisTrackConsumer" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select((.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaBlock" or
      .binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MutableMatroskaBlock") and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 14 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaCuePoint" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select((.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaEbmlReader" or
      .binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaEbmlReader$Type") and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 11 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElement" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 19 and
  ([$classifications.symbols[] |
    select((.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElementType" or
      .binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElementType$DataType") and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 61 and
  ([$classifications.symbols[] |
    select((.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack" or
      .binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack$AudioDetails" or
      .binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack$Type") and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 28 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.format.MutableMatroskaElement" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3AudioTrack" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3ConstantRateSeeker" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3ContainerProbe" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3FrameReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3Seeker" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3StreamSeeker" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3TrackProvider" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 17 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3XingSeeker" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacAudioTrack" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacContainerProbe" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacFileLoader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacMetadataHeader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacMetadataReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacSeekPoint" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacStreamInfo" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 13 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackInfo" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackInfoBuilder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackProvider" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameHeaderReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select((.binary_name ==
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameInfo" or
        .binary_name ==
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameInfo$ChannelDelta") and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacSubFrameReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaAacTrackConsumer" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaAudioTrack" and
      .assessment == "CLASSIFIED" and
      (if .member_name == "<init>" then .classification == "A_EXACT"
       else .classification == "C_SEMANTIC" and
         (.tests | index("docs/architecture/ADR-0023-bounded-matroska-track-selection.md")) != null
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaContainerProbe" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 9 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaOpusTrackConsumer" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaStreamingFile" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 11 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.Formats" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 9 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerDetectionResult" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerHints" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerProbe" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerRegistry" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.adts.AdtsAudioTrack" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.adts.AdtsContainerProbe" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.adts.AdtsPacketHeader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 8 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.adts.AdtsStreamProvider" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.adts.AdtsStreamReader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainer" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 16 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.container.MediaContainerDetection" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 9 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.AudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.AudioFilterChain" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.AudioPipeline" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.converter.ToFloatAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select((.binary_name | startswith(
        "com.sedmelluq.discord.lavaplayer.filter.equalizer.")) and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 17 and
  ([$classifications.symbols[] |
    select((.binary_name | startswith(
        "com.sedmelluq.discord.lavaplayer.filter.volume.")) and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.AudioDataFormat" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 15 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.AudioDataFormatTools" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.AudioPlayerInputStream" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.OpusAudioDataFormat" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 11 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.Pcm16AudioDataFormat" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.StandardAudioDataFormats" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkDecoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkEncoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.OpusChunkDecoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.OpusChunkEncoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.PcmChunkDecoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.format.transcoder.PcmChunkEncoder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.converter.ToShortAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.converter.ToSplitShortAudioFilter" and
      .assessment == "CLASSIFIED" and
      ((.classification == "A_EXACT" and
        (.member_name == "<init>" or .descriptor == "([[SII)V")) or
       (.classification == "C_SEMANTIC" and
        (.symbol_kind == "CLASS" or
          .descriptor == "([[FII)V" or .descriptor == "([SII)V" or
          .descriptor == "(Ljava/nio/ShortBuffer;)V") and
        (.tests | index("docs/architecture/ADR-0022-bounded-split-short-converter.md")) != null)) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.converter.ConverterAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.AudioPipelineFactory" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.AudioPostProcessor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.BufferingPostProcessor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.ChannelCountPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 9 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.FloatPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 2 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.PcmFormat" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.ResamplingPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "C_SEMANTIC" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null and
      (.tests | index("crates/mantle-audio/src/resample.rs")) != null and
      (.tests | index("docs/architecture/ADR-0007-bounded-pcm-transforms.md")) != null)] |
    length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.ShortPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.SplitShortPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 2 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.UniversalPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 1 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.UserProvidedAudioFilters" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 2 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.CompositeAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.FilterChainBuilder" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.filter.FinalPcmAudioFilter" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 9 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetailsLoader" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 2 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackFormat" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioTrack" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "process"
       then .classification == "C_SEMANTIC" and
         (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_bandcamp.rs")) != null and
         (.tests | index("docs/architecture/ADR-0016-bounded-bandcamp-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioSourceManager" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "loadItem"
       then .classification == "D_LEGACY" and
         (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_getyarn.rs")) != null and
         (.tests | index("docs/architecture/ADR-0020-compatibility-only-getyarn-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 11 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioTrack" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "process"
       then .classification == "D_LEGACY" and
         (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_getyarn.rs")) != null and
         (.tests | index("docs/architecture/ADR-0020-compatibility-only-getyarn-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 4 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioSourceManager" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "loadItem"
       then .classification == "C_SEMANTIC" and
         (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase11_sources.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_remote_http.rs")) != null and
         (.tests | index("docs/architecture/ADR-0005-bounded-blocking-http-media-input.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 14 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioTrack" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "process"
       then .classification == "C_SEMANTIC" and
         (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase6_http.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase11_sources.rs")) != null and
         (.tests | index("docs/architecture/ADR-0005-bounded-blocking-http-media-input.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 6 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioSourceManager" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "loadItem"
       then .classification == "D_LEGACY" and
         (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
         (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 11 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioTrack" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "process"
       then .classification == "D_LEGACY" and
         (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
         (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.beam.BeamSegmentUrlProvider" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "fetchSegmentPlaylistUrl"
       then .classification == "D_LEGACY" and
         (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
         (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 5 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackJsonData" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 7 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyAdaptiveFormatsExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.OfflineYoutubeTrackFormatExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.StreamingDataFormatsExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.YoutubeTrackFormatExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyStreamMapFormatsExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyDashMpdFormatsExtractor" and
      .assessment == "CLASSIFIED" and .classification == "A_EXACT" and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 3 and
  ([$classifications.symbols[] |
    select(.binary_name ==
      "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioSourceManager" and
      .assessment == "CLASSIFIED" and
      (if .symbol_kind == "CLASS" or .member_name == "loadItem"
       then .classification == "C_SEMANTIC" and
         (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
         (.tests | index("crates/mantle-media/tests/phase12_bandcamp.rs")) != null and
         (.tests | index("docs/architecture/ADR-0016-bounded-bandcamp-source.md")) != null
       else .classification == "A_EXACT"
       end) and
      (.tests | index("scripts/run-jvm-gate-a.sh")) != null and
      (.tests | index("tools/jvm-gate/src/emitter.rs")) != null and
      (.tests | index("tools/jvm-gate/src/main.rs")) != null)] | length) == 12 and
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
        "com.sedmelluq.discord.lavaplayer.filter.AudioFilter",
        "com.sedmelluq.discord.lavaplayer.container.Formats",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainer",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerDescriptor",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerDetection",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerDetectionResult",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerHints",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.MediaContainerRegistry",
        "com.sedmelluq.discord.lavaplayer.container.adts.AdtsAudioTrack",
        "com.sedmelluq.discord.lavaplayer.container.adts.AdtsContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.adts.AdtsPacketHeader",
        "com.sedmelluq.discord.lavaplayer.container.adts.AdtsStreamProvider",
        "com.sedmelluq.discord.lavaplayer.container.adts.AdtsStreamReader",
        "com.sedmelluq.discord.lavaplayer.container.common.AacPacketRouter",
        "com.sedmelluq.discord.lavaplayer.container.common.OpusPacketRouter",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacAudioTrack",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacFileLoader",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacMetadataHeader",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacMetadataReader",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacSeekPoint",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacStreamInfo",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackInfo",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackInfoBuilder",
        "com.sedmelluq.discord.lavaplayer.container.flac.FlacTrackProvider",
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameHeaderReader",
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameInfo",
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameInfo$ChannelDelta",
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacFrameReader",
        "com.sedmelluq.discord.lavaplayer.container.flac.frame.FlacSubFrameReader",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaAacTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaAudioTrack",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaOpusTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaStreamingFile",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaVorbisTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaBlock",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MutableMatroskaBlock",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaCuePoint",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaEbmlReader",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaEbmlReader$Type",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElement",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElementType",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaElementType$DataType",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileReader",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack$AudioDetails",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MatroskaFileTrack$Type",
        "com.sedmelluq.discord.lavaplayer.container.matroska.format.MutableMatroskaElement",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3AudioTrack",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3ConstantRateSeeker",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3ContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3FrameReader",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3Seeker",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3StreamSeeker",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3TrackProvider",
        "com.sedmelluq.discord.lavaplayer.container.mp3.Mp3XingSeeker",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegAacTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegAudioTrack",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegContainerProbe",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegFileLoader",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegNoopTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegTrackConsumer",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegTrackInfo",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.MpegTrackInfo$Builder",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegFileTrackProvider",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegParseStopChecker",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegReader",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegReader$Chain",
        "com.sedmelluq.discord.lavaplayer.container.mpeg.reader.MpegSectionHandler",
        "com.sedmelluq.discord.lavaplayer.filter.AudioFilterChain",
        "com.sedmelluq.discord.lavaplayer.filter.AudioPipeline",
        "com.sedmelluq.discord.lavaplayer.filter.AudioPipelineFactory",
        "com.sedmelluq.discord.lavaplayer.filter.AudioPostProcessor",
        "com.sedmelluq.discord.lavaplayer.filter.BufferingPostProcessor",
        "com.sedmelluq.discord.lavaplayer.filter.ChannelCountPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.CompositeAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.FilterChainBuilder",
        "com.sedmelluq.discord.lavaplayer.filter.FinalPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.FloatPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.PcmFilterFactory",
        "com.sedmelluq.discord.lavaplayer.filter.PcmFormat",
        "com.sedmelluq.discord.lavaplayer.filter.ResamplingPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.ShortPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.SplitShortPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.UniversalPcmAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.UserProvidedAudioFilters",
        "com.sedmelluq.discord.lavaplayer.filter.converter.ConverterAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.converter.ToFloatAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.converter.ToShortAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.converter.ToSplitShortAudioFilter",
        "com.sedmelluq.discord.lavaplayer.filter.equalizer.Equalizer",
        "com.sedmelluq.discord.lavaplayer.filter.equalizer.EqualizerConfiguration",
        "com.sedmelluq.discord.lavaplayer.filter.equalizer.EqualizerFactory",
        "com.sedmelluq.discord.lavaplayer.filter.volume.AudioFrameVolumeChanger",
        "com.sedmelluq.discord.lavaplayer.filter.volume.PcmVolumeProcessor",
        "com.sedmelluq.discord.lavaplayer.filter.volume.VolumePostProcessor",
        "com.sedmelluq.discord.lavaplayer.format.AudioDataFormat",
        "com.sedmelluq.discord.lavaplayer.format.AudioDataFormatTools",
        "com.sedmelluq.discord.lavaplayer.format.AudioPlayerInputStream",
        "com.sedmelluq.discord.lavaplayer.format.OpusAudioDataFormat",
        "com.sedmelluq.discord.lavaplayer.format.Pcm16AudioDataFormat",
        "com.sedmelluq.discord.lavaplayer.format.StandardAudioDataFormats",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkDecoder",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.AudioChunkEncoder",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.OpusChunkDecoder",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.OpusChunkEncoder",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.PcmChunkDecoder",
        "com.sedmelluq.discord.lavaplayer.format.transcoder.PcmChunkEncoder",
        "com.sedmelluq.discord.lavaplayer.source.AudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.AudioSourceManagers",
        "com.sedmelluq.discord.lavaplayer.source.ProbingAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.local.LocalSeekableInputStream",
        "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamSegmentUrlProvider",
        "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioTrack",
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
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubePlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetails",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader$CachedPlayerScript",
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader$InfoStatus",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker$CachedAuthScript",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioSourceManager",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeCipherOperation",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeCipherOperationType",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeClientConfig",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeClientConfig$AndroidVersion",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeConstants",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeFormatInfo",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeHttpContextFilter",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeLinkRouter",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeLinkRouter$Routes",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMixLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMixProvider",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMpegStreamAudioTrack",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePayloadHelper",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePersistentHttpStream",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePlaylistLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchMusicProvider",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchMusicResultLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchProvider",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchResultLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureCipher",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureCipherManager",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureResolver",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetails",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackDetailsLoader",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackFormat",
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeTrackJsonData",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyAdaptiveFormatsExtractor",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyDashMpdFormatsExtractor",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.LegacyStreamMapFormatsExtractor",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.OfflineYoutubeTrackFormatExtractor",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.StreamingDataFormatsExtractor",
        "com.sedmelluq.discord.lavaplayer.source.youtube.format.YoutubeTrackFormatExtractor"
      ][]; . == $symbol.binary_name)) and
    (if $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.filter.converter.ToSplitShortAudioFilter"
      then if $symbol.symbol_kind == "CLASS" or
          ($symbol.descriptor | IN("([[FII)V", "([SII)V", "(Ljava/nio/ShortBuffer;)V"))
        then .classification == "C_SEMANTIC" and
          (.tests | index("docs/architecture/ADR-0022-bounded-split-short-converter.md")) != null
        else .classification == "A_EXACT"
        end
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.filter.ResamplingPcmAudioFilter"
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-audio/src/resample.rs")) != null and
        (.tests | index("docs/architecture/ADR-0007-bounded-pcm-transforms.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureCipherManager"
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSignatureCipher" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "transform")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadSearchResult")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeSearchMusicProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadSearchMusicResult")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubePersistentHttpStream" and
        ($symbol.symbol_kind == "CLASS" or
          ($symbol.member_name | IN("getConnectUrl", "internalRead", "internalSkip")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/src/http_input.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMpegStreamAudioTrack"
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeMixProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "load")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeHttpContextFilter" and
        ($symbol.symbol_kind == "CLASS" or
          ($symbol.member_name | IN("onRequest", "onRequestResponse")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or
          ($symbol.member_name == "<init>" and $symbol.descriptor != "()V") or
          ($symbol.member_name | IN("loadItem", "loadTrackWithVideoId")))
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.YoutubeAccessTokenTracker" and
        ($symbol.symbol_kind == "CLASS" or
          (["updateMasterToken", "updateAccessToken", "updateVisitorId", "getMasterToken",
            "getAccessToken", "getVisitorId"] | index($symbol.member_name)) != null)
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetailsLoader" and
        ($symbol.symbol_kind == "CLASS" or
          (["loadDetails", "loadBaseResponse", "loadTrackInfoFromInnertube", "augmentWithPlayerScript"] |
            index($symbol.member_name)) != null)
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.youtube.DefaultYoutubeTrackDetails" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "getFormats")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-media/tests/phase12_youtube.rs")) != null and
        (.tests | index("docs/architecture/ADR-0013-ordered-youtube-client-foundation.md")) != null
      elif $symbol.binary_name ==
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
        "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadItem")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_bandcamp.rs")) != null and
        (.tests | index("docs/architecture/ADR-0016-bounded-bandcamp-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.bandcamp.BandcampAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_bandcamp.rs")) != null and
        (.tests | index("docs/architecture/ADR-0016-bounded-bandcamp-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadItem")
      then .classification == "D_LEGACY" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
        (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "D_LEGACY" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
        (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.beam.BeamSegmentUrlProvider" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "fetchSegmentPlaylistUrl")
      then .classification == "D_LEGACY" and
        (.tests | index("crates/mantle-media/tests/phase12_beam.rs")) != null and
        (.tests | index("docs/architecture/ADR-0019-compatibility-only-beam-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadItem")
      then .classification == "D_LEGACY" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_getyarn.rs")) != null and
        (.tests | index("docs/architecture/ADR-0020-compatibility-only-getyarn-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "D_LEGACY" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_getyarn.rs")) != null and
        (.tests | index("docs/architecture/ADR-0020-compatibility-only-getyarn-source.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioSourceManager" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "loadItem")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/load_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase11_sources.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase12_remote_http.rs")) != null and
        (.tests | index("docs/architecture/ADR-0005-bounded-blocking-http-media-input.md")) != null
      elif $symbol.binary_name ==
        "com.sedmelluq.discord.lavaplayer.source.http.HttpAudioTrack" and
        ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("crates/mantle-jvm/src/playback_bridge.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase6_http.rs")) != null and
        (.tests | index("crates/mantle-media/tests/phase11_sources.rs")) != null and
        (.tests | index("docs/architecture/ADR-0005-bounded-blocking-http-media-input.md")) != null
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
          "com.sedmelluq.discord.lavaplayer.container.matroska.MatroskaAudioTrack" and
          ($symbol.symbol_kind == "CLASS" or $symbol.member_name == "process")
      then .classification == "C_SEMANTIC" and
        (.tests | index("docs/architecture/ADR-0023-bounded-matroska-track-selection.md")) != null
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
  .phase_entry.next_slice == "mpeg-section-info-contracts" and
  (.phase_entry.precondition | contains("Phase 12")) and
  (.phase_entry.phase_exit | contains("Revapi"))
JQ

for required in \
  '399 exported classes' \
  '2,762 symbols' \
  '283 reference classes / 2,004 symbols' \
  'C_SEMANTIC' \
  'D_LEGACY' \
  'core-player-track' \
  'Phase 12'; do
  grep --fixed-strings "$required" "$DOCUMENT" >/dev/null
done

"$ROOT/scripts/check-no-jvm-source.sh"

printf 'Phase 13 inventory tracks 1,999 classified symbols and 763 unassessed symbols.\n'
