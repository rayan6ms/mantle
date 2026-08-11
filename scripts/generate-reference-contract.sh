#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly CLASSIFICATION="$ROOT/compatibility/lavaplayer-2.2.6-classification.json"
readonly TEMP_DIR="$(mktemp -d)"
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
cmp "$CLASSIFICATION" "$TEMP_DIR/classification.json"

readonly JAR_ENTRY_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | wc -l)"
readonly JAR_CLASS_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | awk '/\.class$/ { count++ } END { print count + 0 }')"
readonly JAR_FILE_COUNT="$("$JAR_TOOL" tf "$REFERENCE/lavaplayer-2.2.6.jar" | awk '! /\/$/ { count++ } END { print count + 0 }')"
readonly EXPECTED_ENTRY_COUNT="$(jq '.counts.jar_entries' "$INVENTORY")"
readonly EXPECTED_CLASS_COUNT="$(jq '.counts.class_entries' "$INVENTORY")"
readonly EXPECTED_RESOURCE_COUNT="$(jq '.counts.non_class_resources' "$INVENTORY")"

if [[ "$JAR_ENTRY_COUNT" -ne "$EXPECTED_ENTRY_COUNT" ||
      "$JAR_CLASS_COUNT" -ne "$EXPECTED_CLASS_COUNT" ||
      "$((JAR_FILE_COUNT - JAR_CLASS_COUNT))" -ne "$EXPECTED_RESOURCE_COUNT" ]]; then
  printf 'Independent JDK archive counts disagree with the generated inventory.\n' >&2
  exit 1
fi

"$JAVAP_TOOL" -classpath "$REFERENCE/lavaplayer-2.2.6.jar" -s \
  com.sedmelluq.discord.lavaplayer.player.AudioPlayer |
  grep --fixed-strings 'descriptor: (Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Z)Z' >/dev/null

jq --exit-status '
  .status == "INITIAL_UNASSESSED" and
  ([.symbols[].assessment] | all(. == "UNASSESSED")) and
  ([.symbols[] | has("classification")] | all(. == false))
' "$CLASSIFICATION" >/dev/null

printf 'Reference contract reproduced exactly.\n'
