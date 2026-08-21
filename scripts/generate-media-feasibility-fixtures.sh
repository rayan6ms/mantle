#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly FIXTURES="${FIXTURES_DIR:-$ROOT/tests/media/fixtures}"
readonly FDK_AAC_ENCODER="${FDK_AAC_ENCODER:-}"

if [[ -z "$FDK_AAC_ENCODER" || ! -x "$FDK_AAC_ENCODER" ]]; then
  printf 'Set FDK_AAC_ENCODER to the upstream fdk-aac aac-enc executable.\n' >&2
  exit 1
fi

mkdir -p "$FIXTURES"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi \
  -i 'aevalsrc=0.12*sin(2*PI*220*t)+0.08*sin(2*PI*997*t)+0.04*sin(2*PI*3211*t):s=48000:d=6' \
  -af 'pan=stereo|c0=c0|c1=c0' \
  -map_metadata -1 -c:a pcm_s16le -bitexact "$FIXTURES/tone-pcm-s16le.wav"

ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 1 \
  -map_metadata -1 -ac 1 -ar 8000 -c:a pcm_s16le -bitexact \
  "$FIXTURES/tone-pcm-s16le-mono-8k.wav"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 0.25 \
  -map_metadata -1 -ac 2 -ar 384000 -c:a pcm_s16le -bitexact \
  "$FIXTURES/tone-pcm-s16le-stereo-384k.wav"

ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 0.5 \
  -map_metadata -1 -c:a pcm_s24le -write_channel_mask 1 -bitexact \
  "$FIXTURES/tone-pcm-s24le-extensible.wav"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 0.5 \
  -map_metadata -1 -c:a pcm_s32le -write_channel_mask 1 -bitexact \
  "$FIXTURES/tone-pcm-s32le-extensible.wav"

ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a libmp3lame -b:a 192k -write_xing 0 -bitexact \
  "$FIXTURES/tone-mp3.mp3"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle VBR Title' \
  -metadata artist='Mantle VBR Artist' \
  -metadata TSRC='BRMNT2600002' \
  -c:a libmp3lame -q:a 2 -write_xing 1 -id3v2_version 3 -bitexact \
  "$FIXTURES/tone-mp3-vbr-id3.mp3"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a libopus -b:a 128k -vbr off -application audio -bitexact \
  "$FIXTURES/tone-opus.webm"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle Matroska Vorbis Title' \
  -metadata artist='Mantle Matroska Vorbis Artist' \
  -metadata ISRC='BRMNT2600006' \
  -c:a libvorbis -q:a 4 -bitexact "$FIXTURES/tone-vorbis-tags.mkv"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle Ogg Opus Title' \
  -metadata artist='Mantle Ogg Opus Artist' \
  -metadata ISRC='BRMNT2600003' \
  -c:a libopus -b:a 128k -vbr off -application audio -bitexact \
  "$FIXTURES/tone-opus-tags.ogg"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle Ogg Vorbis Title' \
  -metadata artist='Mantle Ogg Vorbis Artist' \
  -metadata ISRC='BRMNT2600004' \
  -c:a libvorbis -q:a 4 -bitexact "$FIXTURES/tone-vorbis-tags.ogg"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle Ogg FLAC Title' \
  -metadata artist='Mantle Ogg FLAC Artist' \
  -metadata ISRC='BRMNT2600005' \
  -c:a flac -strict experimental -bitexact "$FIXTURES/tone-flac-tags.oga"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a aac -profile:a aac_low -b:a 128k -movflags +faststart -bitexact \
  "$FIXTURES/tone-aac-lc.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 -c:a aac -profile:a aac_low -b:a 128k -bitexact \
  "$FIXTURES/tone-aac-lc.adts"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-aac-lc.adts" \
  -map_metadata -1 \
  -metadata service_provider='Mantle Provider' \
  -metadata service_name='Mantle Service' \
  -c:a copy -f mpegts -mpegts_flags +resend_headers -bitexact \
  "$FIXTURES/tone-aac-lc.ts"
perl - "$FIXTURES/tone-aac-lc.adts" "$FIXTURES/tone-aac-lc-crc.adts" <<'PERL'
use strict;
use warnings;

my ($input_path, $output_path) = @ARGV;
open my $input, '<:raw', $input_path or die "open $input_path: $!";
local $/;
my $data = <$input>;
close $input or die "close $input_path: $!";

my $position = 0;
my $output = '';
while ($position < length $data) {
  die "truncated ADTS header at $position\n" if length($data) - $position < 7;
  my @header = unpack 'C7', substr($data, $position, 7);
  die "invalid ADTS sync at $position\n"
    unless $header[0] == 0xff && ($header[1] & 0xf6) == 0xf0;
  die "input already carries an ADTS CRC at $position\n" unless $header[1] & 1;
  my $frame_bytes = (($header[3] & 3) << 11) | ($header[4] << 3) | ($header[5] >> 5);
  die "invalid ADTS frame length at $position\n"
    if $frame_bytes < 7 || $position + $frame_bytes > length $data;

  my $crc_frame_bytes = $frame_bytes + 2;
  $header[1] &= 0xfe;
  $header[3] = ($header[3] & 0xfc) | (($crc_frame_bytes >> 11) & 3);
  $header[4] = ($crc_frame_bytes >> 3) & 0xff;
  $header[5] = ($header[5] & 0x1f) | (($crc_frame_bytes & 7) << 5);
  $output .= pack('C7', @header) . "\0\0" . substr($data, $position + 7, $frame_bytes - 7);
  $position += $frame_bytes;
}

open my $output_file, '>:raw', $output_path or die "open $output_path: $!";
print {$output_file} $output or die "write $output_path: $!";
close $output_file or die "close $output_path: $!";
PERL
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-aac-lc.m4a" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle MP4 Title' \
  -metadata artist='Mantle MP4 Artist' \
  -c:a copy -movflags +faststart -bitexact \
  "$FIXTURES/tone-aac-lc-metadata.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-aac-lc.m4a" -t 2 \
  -map_metadata -1 \
  -metadata title='Mantle Matroska AAC Title' \
  -metadata artist='Mantle Matroska AAC Artist' \
  -c:a copy -bitexact "$FIXTURES/tone-aac-lc-tags.mkv"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" -t 2 \
  -map_metadata -1 -ar 24000 -c:a aac -profile:a aac_low -b:a 96k -bitexact \
  "$FIXTURES/tone-aac-lc-24k.mkv"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-aac-lc.m4a" -t 4 \
  -map_metadata -1 \
  -metadata title='Mantle Fragmented Title' \
  -metadata artist='Mantle Fragmented Artist' \
  -c:a copy -movflags +empty_moov+default_base_moof+global_sidx \
  -frag_duration 500000 -bitexact \
  "$FIXTURES/tone-aac-lc-fragmented.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 -c:a flac -bitexact "$FIXTURES/tone-flac.flac"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-pcm-s16le.wav" \
  -map_metadata -1 \
  -metadata title='Mantle Fixture Title' \
  -metadata artist='Mantle Fixture Artist' \
  -metadata ISRC='BRMNT2600001' \
  -c:a flac -bitexact "$FIXTURES/tone-metadata.flac"

"$FDK_AAC_ENCODER" -r 64000 -t 5 \
  "$FIXTURES/tone-pcm-s16le.wav" "$FIXTURES/tone-he-aac-v1.adts" >/dev/null
"$FDK_AAC_ENCODER" -r 48000 -t 29 \
  "$FIXTURES/tone-pcm-s16le.wav" "$FIXTURES/tone-he-aac-v2.adts" >/dev/null
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-he-aac-v1.adts" \
  -map_metadata -1 -c:a copy -movflags +faststart -bitexact \
  "$FIXTURES/tone-he-aac-v1.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-he-aac-v2.adts" \
  -map_metadata -1 -c:a copy -movflags +faststart -bitexact \
  "$FIXTURES/tone-he-aac-v2.m4a"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-he-aac-v1.m4a" \
  -map_metadata -1 -c:a copy -bitexact "$FIXTURES/tone-he-aac-v1.mkv"
ffmpeg -hide_banner -loglevel error -y -i "$FIXTURES/tone-he-aac-v2.m4a" \
  -map_metadata -1 -c:a copy -bitexact "$FIXTURES/tone-he-aac-v2.mkv"

sha256sum "$FIXTURES"/*
