#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly PLAN="$ROOT/compatibility/phase14-real-consumer-behavior.json"
readonly INVENTORY="$ROOT/reference/phase14-real-consumer-inventory.json"
readonly WORK_ROOT="$ROOT/target/phase14/real-consumer-behavior"
readonly SOURCE_GATES="$ROOT/target/phase14"
readonly REFERENCE_JAR="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar"
readonly MANTLE_JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly NATIVE_LIBRARY="${MANTLE_PHASE14_NATIVE_LIBRARY:-$ROOT/target/debug/libmantle_jvm.so}"
readonly TEMPLATE="$ROOT/scripts/phase14-real-consumer-behavior.java.txt"

for command in cargo find java javac jq sha256sum; do
  command -v "$command" >/dev/null || {
    printf 'Phase 14 real-consumer behavior gate requires %s\n' "$command" >&2
    exit 1
  }
done

[[ -f "$PLAN" && -f "$INVENTORY" && -f "$TEMPLATE" ]] || {
  printf 'Phase 14 behavior plan, inventory, or Java template is missing\n' >&2
  exit 1
}
[[ -f "$REFERENCE_JAR" ]] || {
  printf 'Frozen Lavaplayer reference JAR is missing: %s\n' "$REFERENCE_JAR" >&2
  exit 1
}

jq --exit-status --slurpfile inventory "$INVENTORY" '
  .schema_version == 1 and
  .status == "PASS" and
  .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  .mantle_artifact == "io.github.rayan6ms:mantle-lavaplayer:1.0.0" and
  .native_classifier == "linux-x86_64" and
  ([.scenarios[].id] | length) == ([.scenarios[].id] | unique | length) and
  ([.scenarios[].behaviors[]] | sort | unique) == ($inventory[0].required_behaviors | sort) and
  ([.scenarios[].consumers[]] | sort | unique) == ($inventory[0].consumers | map(.id) | sort) and
  all(.scenarios[]; (.checks | length) > 0) and
  all(.scenarios[];
    . as $scenario |
    all($scenario.behaviors[];
      . as $behavior |
      all(($inventory[0].coverage[] | select(.behavior == $behavior) | .consumers)[];
        . as $consumer | any($scenario.consumers[]; . == $consumer))))
' "$PLAN" >/dev/null

# These two source gates create isolated unchanged consumer builds used directly below.
"$ROOT/scripts/check-phase14-jmusicbot-source-compatibility.sh" >/dev/null
"$ROOT/scripts/check-phase14-youtube-source-spi-compatibility.sh" >/dev/null

env -u APPIMAGE -u APPDIR cargo build --locked -q -p mantle-jvm
if [[ ! -f "$MANTLE_JAR" ]]; then
  mkdir -p "$(dirname "$MANTLE_JAR")"
  env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
    --reference-jar "$REFERENCE_JAR" --output "$MANTLE_JAR" --expected-abi 1
fi
[[ -f "$NATIVE_LIBRARY" ]] || {
  printf 'Mantle native library is missing: %s\n' "$NATIVE_LIBRARY" >&2
  exit 1
}

rm -rf "$WORK_ROOT"
mkdir -p "$WORK_ROOT/reference-classes" "$WORK_ROOT/mantle-classes" "$WORK_ROOT/logs"
cp "$TEMPLATE" "$WORK_ROOT/Phase14RealConsumerBehavior.java"

readonly JMB_ROOT="$SOURCE_GATES/jmusicbot-source-compatibility"
readonly YT_ROOT="$SOURCE_GATES/youtube-source-spi-compatibility"
readonly JMB_REFERENCE_CLASSES="$JMB_ROOT/reference/target/classes"
readonly JMB_MANTLE_CLASSES="$JMB_ROOT/candidate/target/classes"
readonly YT_REFERENCE_CLASSES="$YT_ROOT/reference/v2/build/classes/java/main:$YT_ROOT/reference/common/build/classes/java/main"
readonly YT_MANTLE_CLASSES="$YT_ROOT/candidate/v2/build/classes/java/main:$YT_ROOT/candidate/common/build/classes/java/main"

for required in \
  "$JMB_REFERENCE_CLASSES/com/jagrosh/jmusicbot/audio/QueuedTrack.class" \
  "$JMB_MANTLE_CLASSES/com/jagrosh/jmusicbot/audio/QueuedTrack.class" \
  "$YT_ROOT/reference/common/build/classes/java/main/dev/lavalink/youtube/YoutubeAudioSourceManager.class" \
  "$YT_ROOT/candidate/common/build/classes/java/main/dev/lavalink/youtube/YoutubeAudioSourceManager.class"; do
  [[ -f "$required" ]] || { printf 'Required compiled consumer class is missing: %s\n' "$required" >&2; exit 1; }
done

jmusicbot_dependencies=""
while IFS= read -r dependency; do
  case "$dependency" in
    */dev/arbjerg/lavaplayer/*|*/io/github/rayan6ms/mantle-lavaplayer/*|*/dev/lavalink/youtube/common/*)
      continue
      ;;
  esac
  jmusicbot_dependencies="${jmusicbot_dependencies:+$jmusicbot_dependencies:}$dependency"
done < <(find "$JMB_ROOT/m2" -type f -name '*.jar' | sort)

reference_youtube_classpath="$(<"$YT_ROOT/reference.runtime.classpath")"
mantle_youtube_classpath="$(<"$YT_ROOT/mantle.runtime.classpath")"
readonly reference_youtube_classpath mantle_youtube_classpath

reference_classpath="$YT_REFERENCE_CLASSES:$JMB_REFERENCE_CLASSES:$REFERENCE_JAR:$reference_youtube_classpath:$jmusicbot_dependencies"
mantle_classpath="$YT_MANTLE_CLASSES:$JMB_MANTLE_CLASSES:$MANTLE_JAR:$mantle_youtube_classpath:$jmusicbot_dependencies"
readonly reference_classpath mantle_classpath

if printf '%s' "$mantle_classpath" | tr ':' '\n' | grep -E '/dev/arbjerg/lavaplayer/' >/dev/null; then
  printf 'Mantle behavior classpath retained a reference Lavaplayer artifact\n' >&2
  exit 1
fi

javac --release 11 -cp "$reference_classpath" \
  -d "$WORK_ROOT/reference-classes" "$WORK_ROOT/Phase14RealConsumerBehavior.java"
javac --release 11 -cp "$mantle_classpath" \
  -d "$WORK_ROOT/mantle-classes" "$WORK_ROOT/Phase14RealConsumerBehavior.java"

java -Xverify:all -cp "$WORK_ROOT/reference-classes:$reference_classpath" \
  Phase14RealConsumerBehavior >"$WORK_ROOT/reference.json" 2>"$WORK_ROOT/logs/reference.stderr"
java --enable-native-access=ALL-UNNAMED -Xverify:all \
  -cp "$WORK_ROOT/mantle-classes:$mantle_classpath" \
  Phase14RealConsumerBehavior "$NATIVE_LIBRARY" \
  >"$WORK_ROOT/mantle.json" 2>"$WORK_ROOT/logs/mantle.stderr"

jq --exit-status '
  .normal_player_scheduler == "PASS" and
  .jda_style_frame_provider == "PASS" and
  .listeners == "PASS" and
  .ordered_loading == "PASS" and
  .user_data == "PASS" and
  .markers == "PASS" and
  .serialized_tracks == "PASS" and
  .source_configuration == "PASS" and
  .custom_source_or_subclass == "PASS" and
  (.serialized_sha256 | test("^[0-9a-f]{64}$"))
' "$WORK_ROOT/reference.json" "$WORK_ROOT/mantle.json" >/dev/null
cmp "$WORK_ROOT/reference.json" "$WORK_ROOT/mantle.json"

jq -n \
  --slurpfile reference "$WORK_ROOT/reference.json" \
  --slurpfile mantle "$WORK_ROOT/mantle.json" \
  --arg plan "$PLAN" \
  --arg reference_output "$WORK_ROOT/reference.json" \
  --arg mantle_output "$WORK_ROOT/mantle.json" \
  --arg native "$NATIVE_LIBRARY" \
  '{schema_version: 1, status: "PASS", slice: "phase14-real-consumer-behavior",
    deterministic_runs: {reference: $reference[0], mantle: $mantle[0], exact_match: true},
    direct_consumer_classes: {
      jmusicbot: ["QueuedTrack", "RequestMetadata", "TransformativeAudioSourceManager"],
      youtube_source: ["YoutubeAudioSourceManager", "YoutubeAudioTrack"]},
    interaction_shapes: {
      lavalink: ["player scheduler", "listeners", "mutable frame", "markers", "MessageInput/MessageOutput"],
      simplevoicechat_music: ["player scheduler", "listeners", "mutable frame", "ordered loading", "source configuration"]},
    selected_native_artifact: {classifier: "linux-x86_64", path: $native},
    plan: $plan, outputs: {reference: $reference_output, mantle: $mantle_output}}' \
  > "$WORK_ROOT/result.json"

jq --exit-status '.status == "PASS" and .deterministic_runs.exact_match == true and
  (.deterministic_runs.reference | del(.serialized_sha256) | to_entries | all(.value == "PASS"))' \
  "$WORK_ROOT/result.json" >/dev/null

printf 'Phase 14 real-consumer behavior passed: all 9 required behaviors matched the frozen reference exactly across unchanged JMusicBot/youtube-source classes and Lavalink/Simple Voice Chat interaction shapes.\n'
