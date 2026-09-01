#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly STATUS="$ROOT/reference/remote-source-status-2026-08-13.json"
readonly DOCUMENT="$ROOT/docs/sources/REMOTE_SOURCE_STATUS.md"

jq --exit-status '
  ([.upstream_snapshots[].id, .external_evidence[].id]) as $evidence_ids |
  (.schema_version == 1 and
    .observed_at == "2026-08-13" and
    .compatibility_baseline == "lavaplayer-2.2.6" and
    .required_controls == [
      "bounded_response_bytes",
      "bounded_redirects",
      "bounded_retries",
      "cancellation",
      "credential_safe_diagnostics",
      "rate_limit_classification",
      "ssrf_policy",
      "timeouts"
    ] and
    (.upstream_snapshots | length == 5) and
    (all(.upstream_snapshots[];
      (.commit | test("^[0-9a-f]{40}$")) and
      (.sha256 | test("^[0-9a-f]{64}$")))) and
    (.sources | length == 11) and
    ([.sources[].order] == [1,2,3,4,5,6,7,8,9,10,11]) and
    ([.sources[].class] | unique | length == 11) and
    ([.sources[].source_name] | unique | length == 11) and
    all(.sources[].evidence[]; . as $id | any($evidence_ids[]; . == $id)))
' "$STATUS" >/dev/null

EXPECTED_SOURCES="$(cat <<'EOF'
1	YoutubeAudioSourceManager	youtube	native_required
2	YandexMusicAudioSourceManager	yandex-music	native_required
3	SoundCloudAudioSourceManager	soundcloud	native_required
4	BandcampAudioSourceManager	bandcamp	native_required
5	VimeoAudioSourceManager	vimeo	native_required
6	TwitchStreamAudioSourceManager	twitch	native_required
7	BeamAudioSourceManager	beam.pro	compatibility_only
8	GetyarnAudioSourceManager	getyarn.io	compatibility_only
9	NicoAudioSourceManager	niconico	native_required
10	HttpAudioSourceManager	http	foundation_complete
11	LocalAudioSourceManager	local	foundation_complete
EOF
)"
readonly EXPECTED_SOURCES
ACTUAL_SOURCES="$(
  jq --raw-output '.sources[] | [.order, .class, .source_name, .disposition] | @tsv' "$STATUS"
)"
readonly ACTUAL_SOURCES
if [[ "$ACTUAL_SOURCES" != "$EXPECTED_SOURCES" ]]; then
  printf 'Remote-source classification or compatibility order changed.\n' >&2
  diff -u <(printf '%s\n' "$EXPECTED_SOURCES") <(printf '%s\n' "$ACTUAL_SOURCES") >&2 || true
  exit 1
fi

EXPECTED_SNAPSHOTS="$(cat <<'EOF'
lavaplayer-head	https://github.com/lavalink-devs/lavaplayer	f09c808f7d594206ae149453970015474bbcd222	main/src/main/java/com/sedmelluq/discord/lavaplayer/source/AudioSourceManagers.java	841af5d9f1af663098913d88ad816a8f9119c2691ad95677179ea1ad7fa5b9d0
lavalink-config	https://github.com/lavalink-devs/Lavalink	d3e6039bed8dde4c7c6d15a19e6968c5a6e5cc9d	LavalinkServer/application.yml.example	25681f18581011fed12de9778937ce5753d108430e4781f7e0fde9525db51efd
youtube-source-readme	https://github.com/lavalink-devs/youtube-source	158726dc7d570e8ff32512d6ab15736d561040b7	README.md	be06d223218c896f22866fca5e73405f3c431a21b5b9acc7c76796c198e2b656
lavasrc-readme	https://github.com/topi314/LavaSrc	20d6b0bd1cdddcde86ba4d3fc8e78b97a00e159d	README.md	b5c706e84a556fa51d92c9867aced903d1be8dd3fe37d541f70a5a102a79730b
yt-dlp-niconico	https://github.com/yt-dlp/yt-dlp	81ecd58b1394793e6da9998cc19fdb45657f1685	yt_dlp/extractor/niconico.py	9a31000cb9e2fe67a17bc59b592d3d91fa3a82bfc5113bd43cb139fd9e162048
EOF
)"
readonly EXPECTED_SNAPSHOTS
ACTUAL_SNAPSHOTS="$(
  jq --raw-output \
    '.upstream_snapshots[] | [.id, .repository, .commit, .path, .sha256] | @tsv' \
    "$STATUS"
)"
readonly ACTUAL_SNAPSHOTS
if [[ "$ACTUAL_SNAPSHOTS" != "$EXPECTED_SNAPSHOTS" ]]; then
  printf 'Pinned remote-source upstream evidence changed.\n' >&2
  diff -u <(printf '%s\n' "$EXPECTED_SNAPSHOTS") \
    <(printf '%s\n' "$ACTUAL_SNAPSHOTS") >&2 || true
  exit 1
fi

jq --exit-status '
  .sources[] | select(.source_name == "youtube") |
  .remote_cipher_required == false and
  .capabilities == [
    "video",
    "playlist",
    "search",
    "music_search",
    "livestream",
    "opus_preference",
    "transcode_fallback",
    "source_details"
  ]
' "$STATUS" >/dev/null

if [[ -f "$DOCUMENT" ]]; then
  while IFS=$'\t' read -r class source_name disposition; do
    grep --fixed-strings "\`$class\`" "$DOCUMENT" >/dev/null
    grep --fixed-strings "\`$source_name\`" "$DOCUMENT" >/dev/null
    grep --fixed-strings "\`$disposition\`" "$DOCUMENT" >/dev/null
  done < <(jq --raw-output '.sources[] | [.class, .source_name, .disposition] | @tsv' "$STATUS")

  for commit in $(jq --raw-output '.upstream_snapshots[].commit' "$STATUS"); do
    grep --fixed-strings "$commit" "$DOCUMENT" >/dev/null
  done
fi

printf 'Remote-source status matches the frozen Phase 12 evidence and disposition matrix.\n'
