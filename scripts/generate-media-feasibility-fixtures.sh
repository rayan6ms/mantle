#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly FIXTURES="${FIXTURES_DIR:-$ROOT/tests/media/fixtures}"
readonly FDK_AAC_ENCODER="${FDK_AAC_ENCODER:-}"

if [[ -z "$FDK_AAC_ENCODER" || ! -x "$FDK_AAC_ENCODER" ]]; then
  printf 'Set FDK_AAC_ENCODER to the upstream fdk-aac aac-enc executable.\n' >&2
  exit 1
fi

readonly SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT
mkdir -p "$FIXTURES"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi \
  -i 'aevalsrc=0.12*sin(2*PI*220*t)+0.08*sin(2*PI*997*t)+0.04*sin(2*PI*3211*t):s=48000:d=6' \
  -af 'pan=stereo|c0=c0|c1=c0' \
  -map_metadata -1 -c:a pcm_s16le -bitexact "$FIXTURES/tone-pcm-s16le.wav"

ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a libmp3lame -b:a 192k -write_xing 0 -bitexact \
  "$FIXTURES/tone-mp3.mp3"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a libopus -b:a 128k -vbr off -application audio -bitexact \
  "$FIXTURES/tone-opus.webm"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a aac -profile:a aac_low -b:a 128k -movflags +faststart -bitexact \
  "$FIXTURES/tone-aac-lc.m4a"

"$FDK_AAC_ENCODER" -r 64000 -t 5 \
  "$FIXTURES/tone-pcm-s16le.wav" "$SCRATCH/tone-he-aac-v1.aac" >/dev/null
"$FDK_AAC_ENCODER" -r 48000 -t 29 \
  "$FIXTURES/tone-pcm-s16le.wav" "$SCRATCH/tone-he-aac-v2.aac" >/dev/null
ffmpeg -hide_banner -loglevel error -y -i "$SCRATCH/tone-he-aac-v1.aac" \
  -map_metadata -1 -c:a copy -movflags +faststart -bitexact \
  "$FIXTURES/tone-he-aac-v1.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$SCRATCH/tone-he-aac-v2.aac" \
  -map_metadata -1 -c:a copy -movflags +faststart -bitexact \
  "$FIXTURES/tone-he-aac-v2.m4a"

sha256sum "$FIXTURES"/*
