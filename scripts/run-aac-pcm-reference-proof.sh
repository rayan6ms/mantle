#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly FIXTURES="$ROOT/tests/media/fixtures"
readonly JAVA_HOME="$ROOT/.cache/toolchains/jdk-21.0.12+8"
readonly RESULT_FILE="${RESULT_FILE:-$ROOT/docs/media/results/lavaplayer-2.2.6-aac-pcm-2026-08-11.json}"
readonly SCRATCH="$ROOT/.cache/aac-pcm-reference-proof"

if [[ ! -x "$JAVA_HOME/bin/java" || ! -f "$REFERENCE/lavaplayer-2.2.6.jar" ]]; then
  "$ROOT/scripts/bootstrap-reference.sh" >/dev/null
fi
while IFS=$'\t' read -r expected fixture; do
  printf '%s  %s\n' "$expected" "$FIXTURES/$fixture" | sha256sum --check >/dev/null
done < <(jq --raw-output '.fixtures[] | select(.codec == "AAC") | [.sha256, .file] | @tsv' "$FIXTURES/manifest.json")

mkdir -p "$(dirname "$RESULT_FILE")" "$SCRATCH"
find "$REFERENCE/dependencies" -maxdepth 1 -name '*.jar' -print | sort \
  > "$SCRATCH/classpath.txt"
readonly DEPENDENCY_CLASSPATH="$(paste -sd: "$SCRATCH/classpath.txt")"
readonly CLASSPATH="$REFERENCE/lavaplayer-2.2.6.jar:$DEPENDENCY_CLASSPATH"
readonly LD_LIBRARY_PATH="$JAVA_HOME/lib/server:$JAVA_HOME/lib"
export JAVA_HOME LD_LIBRARY_PATH

env -u APPIMAGE -u APPDIR cargo build --release --locked -p mantle-reference

readonly CHUNKS_20MS_JSONL="$SCRATCH/chunks-20ms.jsonl"
readonly DECODER_GRANULARITY_JSONL="$SCRATCH/decoder-granularity.jsonl"
: > "$CHUNKS_20MS_JSONL"
: > "$DECODER_GRANULARITY_JSONL"
for fixture in tone-aac-lc.m4a tone-he-aac-v1.m4a tone-he-aac-v2.m4a; do
  sha256="$(sha256sum "$FIXTURES/$fixture" | cut -d ' ' -f 1)"
  "$ROOT/target/release/mantle-reference" media-proof \
    --classpath "$CLASSPATH" \
    --input "$FIXTURES/$fixture" \
    --pcm \
    | jq --arg fixture "$fixture" --arg sha256 "$sha256" '
        .input = $fixture
        | .fixture_sha256 = $sha256
        | .decoded_samples_per_channel = (.decoded_bytes / (.output_channels * 2))
      ' >> "$CHUNKS_20MS_JSONL"
  "$ROOT/target/release/mantle-reference" media-proof \
    --classpath "$CLASSPATH" \
    --input "$FIXTURES/$fixture" \
    --pcm \
    --pcm-chunk-samples 64 \
    --decode-only \
    | jq --arg fixture "$fixture" --arg sha256 "$sha256" '
        .input = $fixture
        | .fixture_sha256 = $sha256
        | .decoded_samples_per_channel = (.decoded_bytes / (.output_channels * 2))
      ' >> "$DECODER_GRANULARITY_JSONL"
done

jq -n \
  --slurpfile chunks_20ms "$CHUNKS_20MS_JSONL" \
  --slurpfile decoder_granularity "$DECODER_GRANULARITY_JSONL" \
  '{
    schema_version: 2,
    measured_at: "2026-08-11",
    lavaplayer_version: "2.2.6",
    corpus: "tests/media/fixtures/manifest.json",
    requested_output_format: "DISCORD_PCM_S16_LE",
    codec_label_note: "Lavaplayer 2.2.6 reports PCM_S16_BE from Pcm16AudioDataFormat.codecName even for the selected little-endian format.",
    measurement_note: "The 64-sample output size divides every AAC decoder frame in this corpus and exposes totals hidden by the normal 960-sample framing boundary.",
    chunks_20ms: $chunks_20ms,
    decoder_granularity: $decoder_granularity
  }' > "$RESULT_FILE"

jq -e '
  (.chunks_20ms | length) == 3 and
  (.decoder_granularity | length) == 3 and
  all((.chunks_20ms + .decoder_granularity)[]; .seekable and .output_sample_rate == 48000 and .output_channels == 2 and .last_frame_trailing_zero_bytes == 0) and
  all(.chunks_20ms[]; .output_chunk_samples == 960) and
  all(.decoder_granularity[]; .output_chunk_samples == 64) and
  ([.chunks_20ms[] | {key: .input, value: {frames: .decoded_frames, bytes: .decoded_bytes, samples: .decoded_samples_per_channel, first: .first_timecode_ms, last: .last_timecode_ms}}] | from_entries) == {
    "tone-aac-lc.m4a": {"frames": 300, "bytes": 1152000, "samples": 288000, "first": 0, "last": 5980},
    "tone-he-aac-v1.m4a": {"frames": 305, "bytes": 1171200, "samples": 292800, "first": 0, "last": 6080},
    "tone-he-aac-v2.m4a": {"frames": 307, "bytes": 1178880, "samples": 294720, "first": 0, "last": 6120}
  } and
  ([.decoder_granularity[] | {key: .input, value: {frames: .decoded_frames, bytes: .decoded_bytes, samples: .decoded_samples_per_channel}}] | from_entries) == {
    "tone-aac-lc.m4a": {"frames": 4512, "bytes": 1155072, "samples": 288768},
    "tone-he-aac-v1.m4a": {"frames": 4576, "bytes": 1171456, "samples": 292864},
    "tone-he-aac-v2.m4a": {"frames": 4608, "bytes": 1179648, "samples": 294912}
  }
' "$RESULT_FILE" >/dev/null

printf 'wrote Lavaplayer AAC PCM proof to %s\n' "$RESULT_FILE"
