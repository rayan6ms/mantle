#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly CLASSIFICATION="$ROOT/compatibility/lavaplayer-2.2.6-classification.json"
TEMP_DIR="$(mktemp -d)"
readonly TEMP_DIR
readonly JAR_TOOL="$ROOT/.cache/toolchains/jdk-21.0.12+8/bin/jar"
readonly JAVAP_TOOL="$ROOT/.cache/toolchains/jdk-21.0.12+8/bin/javap"
trap 'rm -rf -- "$TEMP_DIR"' EXIT

run_cargo() {
  env -u APPIMAGE -u APPDIR cargo "$@"
}

cd "$ROOT"
"$ROOT/scripts/bootstrap-reference.sh" >/dev/null

run_cargo run --locked --quiet -p mantle-reference -- inventory \
  --jar "$REFERENCE/lavaplayer-2.2.6.jar" \
  --sources-jar "$REFERENCE/lavaplayer-2.2.6-sources.jar" \
  --pom "$REFERENCE/lavaplayer-2.2.6.pom" \
  --module "$REFERENCE/lavaplayer-2.2.6.module" \
  --output "$TEMP_DIR/inventory.json"

run_cargo run --locked --quiet -p mantle-reference -- seed-classification \
  --inventory "$TEMP_DIR/inventory.json" \
  --output "$TEMP_DIR/classification.json"

cmp "$INVENTORY" "$TEMP_DIR/inventory.json"

jq --exit-status --slurpfile seed "$TEMP_DIR/classification.json" '
  def identity: {binary_name, symbol_kind, member_name, descriptor};
  .schema_version == $seed[0].schema_version and
  .reference == $seed[0].reference and
  ([.symbols[] | identity] == [$seed[0].symbols[] | identity]) and
  (.status == "INITIAL_UNASSESSED" or .status == "IN_PROGRESS" or .status == "COMPLETE") and
  all(.symbols[];
    if .assessment == "CLASSIFIED" then
      (.classification == "A_EXACT" or .classification == "B_SOURCE" or
       .classification == "C_SEMANTIC" or .classification == "D_LEGACY" or
       .classification == "X_UNSUPPORTED") and
      (.notes | length) > 0 and (.tests | length) > 0
    else
      .assessment == "UNASSESSED" and (has("classification") | not)
    end)
' "$CLASSIFICATION" >/dev/null

JAR_ENTRY_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | wc -l)"
readonly JAR_ENTRY_COUNT
JAR_CLASS_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | awk '/\.class$/ { count++ } END { print count + 0 }')"
readonly JAR_CLASS_COUNT
JAR_FILE_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | awk '! /\/$/ { count++ } END { print count + 0 }')"
readonly JAR_FILE_COUNT
EXPECTED_ENTRY_COUNT="$(jq '.counts.jar_entries' "$INVENTORY")"
readonly EXPECTED_ENTRY_COUNT
EXPECTED_CLASS_COUNT="$(jq '.counts.class_entries' "$INVENTORY")"
readonly EXPECTED_CLASS_COUNT
EXPECTED_RESOURCE_COUNT="$(jq '.counts.non_class_resources' "$INVENTORY")"
readonly EXPECTED_RESOURCE_COUNT

if [[ "$JAR_ENTRY_COUNT" -ne "$EXPECTED_ENTRY_COUNT" ||
      "$JAR_CLASS_COUNT" -ne "$EXPECTED_CLASS_COUNT" ||
      "$((JAR_FILE_COUNT - JAR_CLASS_COUNT))" -ne "$EXPECTED_RESOURCE_COUNT" ]]; then
  printf 'Independent JDK archive counts disagree with the generated inventory.\n' >&2
  exit 1
fi

"$JAVAP_TOOL" -classpath "$REFERENCE/lavaplayer-2.2.6.jar" -s \
  com.sedmelluq.discord.lavaplayer.player.AudioPlayer |
  grep --fixed-strings 'descriptor: (Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Z)Z' >/dev/null

printf 'Reference contract inventory and classification identities reproduced exactly.\n'
