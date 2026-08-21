#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly LOCK="$ROOT/reference/lavaplayer-2.2.6.lock.json"
readonly SOURCES_JAR="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6-sources.jar"
readonly GENERATED="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly INVENTORY="$ROOT/docs/sources/SOURCE_MANAGER_COMPATIBILITY.md"

if [[ ! -f "$SOURCES_JAR" ]]; then
  printf 'Pinned sources JAR is missing; run scripts/bootstrap-reference.sh first.\n' >&2
  exit 1
fi

EXPECTED_SHA256="$(
  jq --exit-status --raw-output \
    '.published_artifacts[] | select(.file == "lavaplayer-2.2.6-sources.jar") | .sha256' \
    "$LOCK"
)"
readonly EXPECTED_SHA256
printf '%s  %s\n' "$EXPECTED_SHA256" "$SOURCES_JAR" | sha256sum --check >/dev/null

assert_source_contains() {
  local path="$1"
  local expected="$2"
  if ! unzip -p "$SOURCES_JAR" "$path" | grep --fixed-strings "$expected" >/dev/null; then
    printf 'Reference evidence changed: %s no longer contains %s\n' "$path" "$expected" >&2
    exit 1
  fi
}

readonly SOURCE_ROOT="com/sedmelluq/discord/lavaplayer"
readonly REGISTRY="$SOURCE_ROOT/source/AudioSourceManagers.java"
readonly MANAGER="$SOURCE_ROOT/player/DefaultAudioPlayerManager.java"
readonly ORDERED="$SOURCE_ROOT/tools/OrderedExecutor.java"

mapfile -t expected_remote <<'EOF'
YoutubeAudioSourceManager
YandexMusicAudioSourceManager
SoundCloudAudioSourceManager
BandcampAudioSourceManager
VimeoAudioSourceManager
TwitchStreamAudioSourceManager
BeamAudioSourceManager
GetyarnAudioSourceManager
NicoAudioSourceManager
HttpAudioSourceManager
EOF

jq --exit-status --argjson expected "$(printf '%s\n' "${expected_remote[@]}" | jq -R . | jq -s .)" \
  '.built_in_sources.remote_registration_order == $expected and
   .built_in_sources.local_registration_order == ["LocalAudioSourceManager"]' \
  "$GENERATED" >/dev/null

previous_line=0
for class in "${expected_remote[@]}"; do
  line="$(unzip -p "$SOURCES_JAR" "$REGISTRY" | grep -n -m1 \
    "registerSourceManager.*${class}" | cut -d: -f1)"
  if [[ -z "$line" || "$line" -le "$previous_line" ]]; then
    printf 'Remote registration order changed at %s.\n' "$class" >&2
    exit 1
  fi
  previous_line="$line"
done

assert_source_contains "$REGISTRY" \
  'playerManager.registerSourceManager(new LocalAudioSourceManager(containerRegistry));'
assert_source_contains "$MANAGER" 'private static final int MAXIMUM_LOAD_REDIRECTS = 5;'
assert_source_contains "$MANAGER" 'private static final int DEFAULT_LOADER_POOL_SIZE = 10;'
assert_source_contains "$MANAGER" 'private static final int LOADER_QUEUE_CAPACITY = 5000;'
assert_source_contains "$MANAGER" 'sourceManagers.add(sourceManager);'
assert_source_contains "$MANAGER" 'Collections.unmodifiableList(sourceManagers)'
assert_source_contains "$MANAGER" 'klass.isAssignableFrom(sourceManager.getClass())'
assert_source_contains "$MANAGER" 'reference.containerDescriptor != null && !(sourceManager instanceof ProbingAudioSourceManager)'
assert_source_contains "$MANAGER" 'if (item instanceof AudioReference) {'
assert_source_contains "$MANAGER" 'output.writeUTF(sourceManager.getSourceName());'
assert_source_contains "$MANAGER" 'if (sourceName.equals(sourceManager.getSourceName())) {'
assert_source_contains "$ORDERED" 'ConcurrentMap<Object, BlockingQueue<Runnable>> states;'
assert_source_contains "$ORDERED" 'BlockingQueue<Runnable> existing = states.putIfAbsent(runnable.key, newQueue);'
assert_source_contains "$ORDERED" 'while ((next = queue.poll()) != null) {'

while IFS=$'\t' read -r path source_name; do
  assert_source_contains "$SOURCE_ROOT/source/$path" "return \"$source_name\";"
  grep --fixed-strings "| \`$source_name\` |" "$INVENTORY" >/dev/null
done <<'EOF'
youtube/YoutubeAudioSourceManager.java	youtube
yamusic/YandexMusicAudioSourceManager.java	yandex-music
soundcloud/SoundCloudAudioSourceManager.java	soundcloud
bandcamp/BandcampAudioSourceManager.java	bandcamp
vimeo/VimeoAudioSourceManager.java	vimeo
twitch/TwitchStreamAudioSourceManager.java	twitch
beam/BeamAudioSourceManager.java	beam.pro
getyarn/GetyarnAudioSourceManager.java	getyarn.io
nico/NicoAudioSourceManager.java	niconico
http/HttpAudioSourceManager.java	http
local/LocalAudioSourceManager.java	local
EOF

printf 'Source-manager inventory matches the pinned Lavaplayer 2.2.6 sources.\n'
