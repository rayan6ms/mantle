#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly LOCK="$ROOT/reference/lavaplayer-2.2.6.lock.json"
readonly SOURCES_JAR="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6-sources.jar"
readonly INVENTORY="$ROOT/docs/media/LOCAL_MEDIA_COMPATIBILITY.md"
readonly CONTAINER_SOURCE="com/sedmelluq/discord/lavaplayer/container/MediaContainer.java"

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

CONTAINER_ENUM="$(
  unzip -p "$SOURCES_JAR" "$CONTAINER_SOURCE" |
    sed -n '/public enum MediaContainer {/,/);/p' |
    sed -nE 's/^[[:space:]]*([A-Z0-9]+)\(.*$/\1/p'
)"
readonly CONTAINER_ENUM
readonly EXPECTED_ENUM=$'WAV\nMKV\nMP4\nFLAC\nOGG\nM3U\nPLS\nPLAIN\nMP3\nADTS\nMPEGADTS'

if [[ "$CONTAINER_ENUM" != "$EXPECTED_ENUM" ]]; then
  printf 'Pinned reference container enum no longer matches the Phase 9 inventory:\n%s\n' \
    "$CONTAINER_ENUM" >&2
  exit 1
fi

if [[ -f "$INVENTORY" ]]; then
  for probe in WAV MKV MP4 FLAC OGG M3U PLS PLAIN MP3 ADTS MPEGADTS; do
    grep --fixed-strings "| $probe |" "$INVENTORY" >/dev/null
  done
fi

assert_source_contains() {
  local path="$1"
  local text="$2"
  if ! unzip -p "$SOURCES_JAR" "$path" | grep --fixed-strings "$text" >/dev/null; then
    printf 'Reference evidence changed: %s no longer contains %s\n' "$path" "$text" >&2
    exit 1
  fi
}

readonly CONTAINER_ROOT="com/sedmelluq/discord/lavaplayer/container"
assert_source_contains "$CONTAINER_ROOT/wav/WavFileLoader.java" \
  'bitsPerSample != 16 && bitsPerSample != 24 && bitsPerSample != 32'
assert_source_contains "$CONTAINER_ROOT/wav/WavFileLoader.java" \
  'sampleRate < 100 || sampleRate > 384000'
assert_source_contains "$CONTAINER_ROOT/wav/WavFileLoader.java" \
  'channelCount < 1 || channelCount > 16'
assert_source_contains "$CONTAINER_ROOT/matroska/MatroskaContainerProbe.java" \
  'Arrays.asList(OPUS_CODEC, VORBIS_CODEC, AAC_CODEC)'
assert_source_contains "$CONTAINER_ROOT/mpeg/MpegContainerProbe.java" \
  '"soun".equals(track.handler) && "mp4a".equals(track.codecName)'
assert_source_contains "$CONTAINER_ROOT/ogg/OggTrackLoader.java" 'new OggOpusCodecHandler()'
assert_source_contains "$CONTAINER_ROOT/ogg/OggTrackLoader.java" 'new OggFlacCodecHandler()'
assert_source_contains "$CONTAINER_ROOT/ogg/OggTrackLoader.java" 'new OggVorbisCodecHandler()'
assert_source_contains "$CONTAINER_ROOT/flac/FlacMetadataReader.java" \
  'header.blockLength != FlacStreamInfo.LENGTH'
assert_source_contains "$CONTAINER_ROOT/adts/AdtsContainerProbe.java" 'return "adts";'
assert_source_contains "$CONTAINER_ROOT/adts/AdtsContainerProbe.java" \
  'reader.findPacketHeader(MediaContainerDetection.STREAM_SCAN_DISTANCE)'
assert_source_contains "$CONTAINER_ROOT/adts/AdtsStreamReader.java" \
  'int payloadLength = frameLength - 7 - (isProtectionAbsent ? 0 : 2);'
assert_source_contains "$CONTAINER_ROOT/adts/AdtsStreamReader.java" \
  'if (reader.asLong(2) != 0) {'
assert_source_contains "$CONTAINER_ROOT/mpegts/MpegAdtsContainerProbe.java" \
  'return "mpegts-adts";'

printf 'Local-media inventory matches the pinned Lavaplayer 2.2.6 sources.\n'
