#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly FIXTURES="$ROOT/.cache/performance/fixtures"

mkdir -p "$FIXTURES"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi \
  -i 'aevalsrc=0.12*sin(2*PI*220*t)+0.08*sin(2*PI*997*t)+0.04*sin(2*PI*3211*t):s=48000:d=90' \
  -af 'pan=stereo|c0=c0|c1=c0' \
  -c:a pcm_s16le -bitexact "$FIXTURES/reference.wav"

ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/reference.wav" \
  -c:a libmp3lame -b:a 192k -write_xing 0 -bitexact "$FIXTURES/reference.mp3"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/reference.wav" \
  -c:a aac -profile:a aac_low -b:a 128k -movflags +faststart -bitexact "$FIXTURES/reference.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/reference.wav" \
  -c:a flac -compression_level 5 -bitexact "$FIXTURES/reference.flac"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/reference.wav" \
  -c:a libopus -b:a 128k -vbr off -application audio -bitexact "$FIXTURES/reference.webm"

sha256sum "$FIXTURES"/reference.*
