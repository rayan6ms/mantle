#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly FIXTURES="$ROOT/tests/media/fixtures"
readonly PERFORMANCE_FIXTURES="$ROOT/.cache/performance/fixtures"
readonly JAVA_HOME="$ROOT/.cache/toolchains/jdk-21.0.12+8"
readonly RESULT_FILE="${RESULT_FILE:-$ROOT/docs/media/results/lavaplayer-2.2.6-phase6.json}"
readonly SCRATCH="$ROOT/.cache/media-reference-proof"

if [[ ! -x "$JAVA_HOME/bin/java" || ! -f "$REFERENCE/lavaplayer-2.2.6.jar" ]]; then
  "$ROOT/scripts/bootstrap-reference.sh" >/dev/null
fi
if [[ ! -f "$PERFORMANCE_FIXTURES/reference.mp3" ]]; then
  "$ROOT/scripts/generate-benchmark-fixtures.sh" >/dev/null
fi
while IFS=$'\t' read -r expected fixture; do
  printf '%s  %s\n' "$expected" "$FIXTURES/$fixture" | sha256sum --check >/dev/null
done < <(jq --raw-output '.fixtures[] | [.sha256, .file] | @tsv' "$FIXTURES/manifest.json")

mkdir -p "$(dirname "$RESULT_FILE")" "$SCRATCH"
find "$REFERENCE/dependencies" -maxdepth 1 -name '*.jar' -print | sort \
  > "$SCRATCH/classpath.txt"
readonly DEPENDENCY_CLASSPATH="$(paste -sd: "$SCRATCH/classpath.txt")"
readonly CLASSPATH="$REFERENCE/lavaplayer-2.2.6.jar:$DEPENDENCY_CLASSPATH"
readonly LD_LIBRARY_PATH="$JAVA_HOME/lib/server:$JAVA_HOME/lib"
export JAVA_HOME LD_LIBRARY_PATH

env -u APPIMAGE -u APPDIR cargo build --release --locked -p mantle-reference

readonly LOCAL_JSONL="$SCRATCH/local.jsonl"
readonly HTTP_JSON="$SCRATCH/http.json"
readonly HTTP_LOG="$SCRATCH/http.log"
: > "$LOCAL_JSONL"
: > "$HTTP_LOG"

for fixture in \
  tone-pcm-s16le.wav \
  tone-mp3.mp3 \
  tone-opus.webm \
  tone-aac-lc.m4a \
  tone-he-aac-v1.m4a \
  tone-he-aac-v2.m4a
do
  "$ROOT/target/release/mantle-reference" media-proof \
    --classpath "$CLASSPATH" \
    --input "$FIXTURES/$fixture" \
    | jq --arg fixture "$fixture" '.input = $fixture' >> "$LOCAL_JSONL"
done

"$ROOT/target/release/mantle-reference" serve --root "$PERFORMANCE_FIXTURES" \
  > /dev/null 2> "$HTTP_LOG" &
readonly SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true' EXIT

for _ in {1..50}; do
  if curl --fail --silent --head http://127.0.0.1:18080/reference.mp3 >/dev/null; then
    break
  fi
  sleep 0.1
done

"$ROOT/target/release/mantle-reference" media-proof \
  --classpath "$CLASSPATH" \
  --input http://127.0.0.1:18080/reference.mp3 \
  --http > "$HTTP_JSON"

if ! grep -Eq 'status=206 range=bytes=[0-9]+-' "$HTTP_LOG"; then
  printf 'Lavaplayer did not issue a successful HTTP range request.\n' >&2
  exit 1
fi

jq -n \
  --slurpfile local "$LOCAL_JSONL" \
  --slurpfile http "$HTTP_JSON" \
  --arg http_sha256 "$(sha256sum "$PERFORMANCE_FIXTURES/reference.mp3" | cut -d ' ' -f 1)" \
  '{
    schema_version: 1,
    lavaplayer_version: "2.2.6",
    corpus: "tests/media/fixtures/manifest.json",
    local: $local,
    http_range: ($http[0] + {
      input: "reference.mp3",
      fixture_sha256: $http_sha256,
      partial_content_observed: true
    })
  }' > "$RESULT_FILE"

jq -e '
  (.local | length) == 6 and
  all(.local[]; .seekable and .decoded_frames > 0 and .output_codec == "OPUS") and
  .http_range.partial_content_observed and
  .http_range.seekable and
  .http_range.decoded_frames > 0
' "$RESULT_FILE" >/dev/null

printf 'wrote Lavaplayer media proof to %s\n' "$RESULT_FILE"
