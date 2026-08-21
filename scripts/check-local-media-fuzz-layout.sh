#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly MANIFEST="$ROOT/fuzz/Cargo.toml"
readonly LOCK="$ROOT/fuzz/Cargo.lock"
readonly -a EXPECTED_TARGETS=(
  local_wav
  local_matroska
  local_mp4
  local_flac
  local_ogg
  local_mp3
  local_adts
)

mapfile -t actual_targets < <(sed -nE 's/^name = "(local_[a-z0-9_]+)"$/\1/p' "$MANIFEST")
if [[ "${actual_targets[*]}" != "${EXPECTED_TARGETS[*]}" ]]; then
  printf 'Local-media fuzz targets differ from the seven Phase 9 boundaries.\n' >&2
  printf 'Expected: %s\nActual:   %s\n' "${EXPECTED_TARGETS[*]}" "${actual_targets[*]}" >&2
  exit 1
fi

for target in "${EXPECTED_TARGETS[@]}"; do
  test -f "$ROOT/fuzz/fuzz_targets/$target.rs"
  grep --fixed-strings "fuzz_target!(" "$ROOT/fuzz/fuzz_targets/$target.rs" >/dev/null
  grep --fixed-strings "$target" "$ROOT/scripts/run-local-media-fuzz-smoke.sh" >/dev/null
done

grep --fixed-strings 'libfuzzer-sys = "=0.4.13"' "$MANIFEST" >/dev/null

awk '
  function check_package() {
    if (name ~ /^symphonia/ && version != "0.6.0") {
      printf "Fuzz lock drifted from production: %s is %s, expected 0.6.0\n", name, version > "/dev/stderr"
      failed = 1
    }
    if (name ~ /^symphonia/) {
      symphonia_packages += 1
    }
    if (name == "libfuzzer-sys" && version != "0.4.13") {
      printf "Fuzz lock has libfuzzer-sys %s, expected 0.4.13\n", version > "/dev/stderr"
      failed = 1
    }
  }
  /^\[\[package\]\]$/ {
    check_package()
    name = ""
    version = ""
    next
  }
  /^name = / {
    name = $0
    sub(/^name = "/, "", name)
    sub(/"$/, "", name)
    next
  }
  /^version = / {
    version = $0
    sub(/^version = "/, "", version)
    sub(/"$/, "", version)
  }
  END {
    check_package()
    if (symphonia_packages != 13) {
      printf "Fuzz lock has %d Symphonia packages, expected 13\n", symphonia_packages > "/dev/stderr"
      failed = 1
    }
    exit failed
  }
' "$LOCK"

printf 'Local-media fuzz layout and dependency pins are complete.\n'
