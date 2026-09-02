#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly INVENTORY="$ROOT/reference/phase14-real-consumer-inventory.json"
readonly DOCUMENT="$ROOT/COMPATIBILITY.md"
readonly CACHE_ROOT="${MANTLE_PHASE14_CONSUMER_CACHE:-$ROOT/.cache/phase14-consumers}"

verify_upstream=false
if (( $# > 1 )); then
  printf 'usage: %s [--verify-upstream]\n' "$0" >&2
  exit 2
fi
if (( $# == 1 )); then
  if [[ "$1" != "--verify-upstream" ]]; then
    printf 'unknown argument: %s\n' "$1" >&2
    exit 2
  fi
  verify_upstream=true
fi

jq --exit-status '
  . as $root |
  ($root.required_behaviors | sort) as $required |
  ($root.consumers | map(.id) | sort) as $consumer_ids |
  $root.schema_version == 1 and
  $root.status == "PINNED" and
  $root.observed_at == "2026-08-30" and
  $root.compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  $root.mantle_replacement == "io.github.rayan6ms:mantle-lavaplayer:1.0.0" and
  ($required == [
    "custom_source_or_subclass",
    "jda_style_frame_provider",
    "listeners",
    "markers",
    "normal_player_scheduler",
    "ordered_loading",
    "serialized_tracks",
    "source_configuration",
    "user_data"
  ]) and
  ($root.required_behaviors | length) == ($required | unique | length) and
  ($root.consumers | length) == 4 and
  ($consumer_ids | unique | length) == 4 and
  all($root.consumers[];
    (.repository | test("^https://github\\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")) and
    (.revision | test("^[0-9a-f]{40}$")) and
    (.revision_date | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$")) and
    (.license.spdx == "MIT" or .license.spdx == "Apache-2.0") and
    (.license.sha256 | test("^[0-9a-f]{64}$")) and
    (.dependency.coordinate | test("^dev\\.arbjerg:lavaplayer:[0-9]+\\.[0-9]+\\.[0-9]+$")) and
    (.dependency.manifest_sha256 | test("^[0-9a-f]{64}$")) and
    (.planned_compile_command | length > 0) and
    (.migration_constraints | length >= 3) and
    (.evidence | length >= 2) and
    all(.evidence[];
      (.path | test("^(?!/)(?!.*(?:^|/)\\.\\.(?:/|$)).+$")) and
      (.sha256 | test("^[0-9a-f]{64}$")) and
      (.behaviors | length > 0) and
      all(.behaviors[]; . as $behavior | any($required[]; . == $behavior))
    ) and
    ([.declared_lavaplayer_class_dispositions.A_EXACT,
      .declared_lavaplayer_class_dispositions.C_SEMANTIC,
      .declared_lavaplayer_class_dispositions.D_LEGACY] | all(. >= 0))
  ) and
  (($root.coverage | map(.behavior) | sort) == $required) and
  ($root.coverage | length) == ($root.coverage | map(.behavior) | unique | length) and
  all($root.coverage[];
    (.consumers | length > 0) and
    (.consumers | length) == (.consumers | unique | length) and
    all(.consumers[]; . as $id | any($consumer_ids[]; . == $id))
  ) and
  all($root.coverage[];
    . as $coverage |
    ($coverage.consumers | sort) ==
      ([$root.consumers[] |
        select(any(.evidence[]; any(.behaviors[]; . == $coverage.behavior))) |
        .id] | sort)
  ) and
  $root.phase_entry.completed_slice == "phase14-real-consumer-inventory" and
  $root.phase_entry.next_slice == "phase14-lavalink-source-compatibility" and
  ($root.phase_entry.kill_gate | contains("support"))
' "$INVENTORY" >/dev/null

EXPECTED_CONSUMERS="$(cat <<'EOF'
jmusicbot	jda_music_bot	https://github.com/jagrosh/MusicBot	859e5c5862decf433f8face5eaca3372d7d27b22	dev.arbjerg:lavaplayer:2.2.1	Apache-2.0	17	6	2
lavalink	audio_server_and_plugin_api	https://github.com/lavalink-devs/Lavalink	3d24006d1eed2bd9b4f5916298cf87ab34408b6f	dev.arbjerg:lavaplayer:2.2.6	MIT	33	7	0
simplevoicechat_music	minecraft_voice_chat_mod	https://github.com/ItzDerock/simplevoicechat-music	f21305f4deafc4c5869a060e8dcfbbf24d73c82b	dev.arbjerg:lavaplayer:2.2.1	Apache-2.0	14	1	0
youtube_source	third_party_source_extension	https://github.com/lavalink-devs/youtube-source	f45bbb7aebfcbc1c553769e04af6cd43afa8b7c3	dev.arbjerg:lavaplayer:2.1.1	MIT	28	1	0
EOF
)"
readonly EXPECTED_CONSUMERS
ACTUAL_CONSUMERS="$(
  jq --raw-output '
    .consumers[] |
    [.id, .role, .repository, .revision, .dependency.coordinate, .license.spdx,
     .declared_lavaplayer_class_dispositions.A_EXACT,
     .declared_lavaplayer_class_dispositions.C_SEMANTIC,
     .declared_lavaplayer_class_dispositions.D_LEGACY] |
    @tsv
  ' "$INVENTORY" | sort
)"
readonly ACTUAL_CONSUMERS
if [[ "$ACTUAL_CONSUMERS" != "$EXPECTED_CONSUMERS" ]]; then
  printf 'Phase 14 pinned consumer set or disposition summary changed.\n' >&2
  diff -u <(printf '%s\n' "$EXPECTED_CONSUMERS") \
    <(printf '%s\n' "$ACTUAL_CONSUMERS") >&2 || true
  exit 1
fi

for consumer in $(jq --raw-output '.consumers[].id' "$INVENTORY"); do
  grep --fixed-strings "\`$consumer\`" "$DOCUMENT" >/dev/null
done
for revision in $(jq --raw-output '.consumers[].revision' "$INVENTORY"); do
  grep --fixed-strings "$revision" "$DOCUMENT" >/dev/null
done
for behavior in $(jq --raw-output '.required_behaviors[]' "$INVENTORY"); do
  grep --fixed-strings "\`$behavior\`" "$DOCUMENT" >/dev/null
done

normalize_repository_url() {
  local repository_url="${1%.git}"
  printf '%s' "${repository_url%/}"
}

verify_repository() {
  local consumer_id="$1"
  local repository="$2"
  local revision="$3"
  local revision_date="$4"
  local checkout="$CACHE_ROOT/$consumer_id"

  mkdir -p "$CACHE_ROOT"
  if [[ ! -d "$checkout/.git" ]]; then
    mkdir -p "$checkout"
    git -C "$checkout" init --quiet
    git -C "$checkout" remote add origin "$repository"
  fi

  local actual_origin
  actual_origin="$(git -C "$checkout" remote get-url origin)"
  if [[ "$(normalize_repository_url "$actual_origin")" != \
        "$(normalize_repository_url "$repository")" ]]; then
    printf '%s cache origin mismatch: expected %s, got %s\n' \
      "$consumer_id" "$repository" "$actual_origin" >&2
    exit 1
  fi

  if ! git -C "$checkout" cat-file -e "$revision^{commit}" 2>/dev/null; then
    git -C "$checkout" fetch --quiet --depth=1 origin "$revision"
  fi
  local actual_date
  actual_date="$(git -C "$checkout" show -s --format='%cI' "$revision")"
  if [[ "$actual_date" != "$revision_date" ]]; then
    printf '%s revision date mismatch: expected %s, got %s\n' \
      "$consumer_id" "$revision_date" "$actual_date" >&2
    exit 1
  fi
}

verify_blob() {
  local consumer_id="$1"
  local revision="$2"
  local path="$3"
  local expected_sha256="$4"
  local checkout="$CACHE_ROOT/$consumer_id"
  local actual_sha256
  actual_sha256="$(git -C "$checkout" cat-file blob "$revision:$path" | sha256sum | awk '{print $1}')"
  if [[ "$actual_sha256" != "$expected_sha256" ]]; then
    printf '%s:%s SHA-256 mismatch: expected %s, got %s\n' \
      "$consumer_id" "$path" "$expected_sha256" "$actual_sha256" >&2
    exit 1
  fi
}

if [[ "$verify_upstream" == true ]]; then
  while IFS=$'\t' read -r consumer_id repository revision revision_date; do
    verify_repository "$consumer_id" "$repository" "$revision" "$revision_date"
  done < <(jq --raw-output \
    '.consumers[] | [.id, .repository, .revision, .revision_date] | @tsv' "$INVENTORY")

  while IFS=$'\t' read -r consumer_id revision path sha256; do
    verify_blob "$consumer_id" "$revision" "$path" "$sha256"
  done < <(jq --raw-output '
    .consumers[] as $consumer |
    ([{path: $consumer.license.path, sha256: $consumer.license.sha256},
       {path: $consumer.dependency.manifest, sha256: $consumer.dependency.manifest_sha256}] +
      $consumer.evidence)[] |
    [$consumer.id, $consumer.revision, .path, .sha256] |
    @tsv
  ' "$INVENTORY")
fi

"$ROOT/scripts/check-no-jvm-source.sh" "$ROOT"
verification_suffix=""
if [[ "$verify_upstream" == true ]]; then
  verification_suffix=" with upstream blob verification"
fi
printf 'Phase 14 consumer inventory passed: 4 pinned repositories cover all 9 required behaviors%s.\n' \
  "$verification_suffix"
