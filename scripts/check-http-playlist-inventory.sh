#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly LOCK="$ROOT/reference/lavaplayer-2.2.6.lock.json"
readonly SOURCES_JAR="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6-sources.jar"
readonly INVENTORY="$ROOT/docs/media/HTTP_PLAYLIST_COMPATIBILITY.md"

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
readonly PLAYLIST_ROOT="$SOURCE_ROOT/container/playlists"
readonly STREAM_ROOT="$SOURCE_ROOT/source/stream"
readonly MPEG_TS_ROOT="$SOURCE_ROOT/container/mpegts"

assert_source_contains "$SOURCE_ROOT/source/http/HttpAudioSourceManager.java" \
  'reference.identifier.startsWith("icy://")'
assert_source_contains "$SOURCE_ROOT/source/http/HttpAudioSourceManager.java" \
  'HttpClientTools.getRedirectLocation(reference.identifier, inputStream.getCurrentResponse())'
assert_source_contains "$SOURCE_ROOT/tools/io/PersistentHttpStream.java" \
  'request.setHeader(HttpHeaders.RANGE, "bytes=" + position + "-")'
assert_source_contains "$SOURCE_ROOT/tools/io/PersistentHttpStream.java" \
  'HttpClientTools.isRetriableNetworkException(exception)'
assert_source_contains "$PLAYLIST_ROOT/M3uPlaylistContainerProbe.java" \
  'checkNextBytes(inputStream, M3U_HEADER_TAG)'
assert_source_contains "$PLAYLIST_ROOT/M3uPlaylistContainerProbe.java" \
  'line.startsWith("http://") || line.startsWith("https://") || line.startsWith("icy://")'
assert_source_contains "$PLAYLIST_ROOT/PlsPlaylistContainerProbe.java" \
  'Pattern.compile("\\s*File([0-9]+)=((?:https?|icy)://.*)\\s*")'
assert_source_contains "$PLAYLIST_ROOT/PlainPlaylistContainerProbe.java" \
  'Pattern.compile("^(?:https?|icy)://.*")'
assert_source_contains "$PLAYLIST_ROOT/HlsStreamSegmentUrlProvider.java" \
  'line.startsWith("#EXT-X-STREAM-INF")'
assert_source_contains "$STREAM_ROOT/M3uStreamSegmentUrlProvider.java" \
  'return URI.create(playlistUrl).resolve(segmentName).toString();'
assert_source_contains "$STREAM_ROOT/M3uStreamSegmentUrlProvider.java" \
  'private static final long SEGMENT_WAIT_STEP_MS = 200;'
assert_source_contains "$MPEG_TS_ROOT/MpegTsElementaryInputStream.java" \
  'public static final int ADTS_ELEMENTARY_STREAM = 0x0F;'
assert_source_contains "$MPEG_TS_ROOT/MpegTsElementaryInputStream.java" \
  'private static final int TS_PACKET_SIZE = 188;'
assert_source_contains "$MPEG_TS_ROOT/MpegTsElementaryInputStream.java" \
  'if (streamType == elementaryDataType) {'
assert_source_contains "$MPEG_TS_ROOT/MpegTsElementaryInputStream.java" \
  'if (descriptorTag == 0x48) {'
assert_source_contains "$MPEG_TS_ROOT/PesPacketInputStream.java" \
  'private static final byte[] SYNC_BYTES = new byte[]{0x00, 0x00, 0x01};'

for boundary in \
  'HTTP identifiers' \
  'HTTP object reads' \
  'M3U' \
  'PLS' \
  'PLAIN' \
  'HLS master/media playlist' \
  'HLS segment sequence' \
  'MPEG-TS ADTS'; do
  grep --fixed-strings "| $boundary |" "$INVENTORY" >/dev/null
done

printf 'HTTP/playlist inventory matches the pinned Lavaplayer 2.2.6 sources.\n'
