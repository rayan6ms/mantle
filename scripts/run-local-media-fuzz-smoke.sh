#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly FIXTURES="$ROOT/tests/media/fixtures"
readonly FUZZ_TOOLCHAIN="${FUZZ_TOOLCHAIN:-nightly-2026-08-10}"
readonly FUZZ_RUNS="${FUZZ_RUNS:-128}"
readonly FUZZ_MAX_LEN="${FUZZ_MAX_LEN:-262144}"
readonly FUZZ_TIMEOUT_SECONDS="${FUZZ_TIMEOUT_SECONDS:-5}"
FUZZ_HOST="$(rustc "+$FUZZ_TOOLCHAIN" -vV | awk '$1 == "host:" {print $2}')"
readonly FUZZ_HOST
[[ -n "$FUZZ_HOST" ]] || { printf 'Unable to determine the fuzz toolchain host.\n' >&2; exit 1; }
readonly -a ALL_TARGETS=(
  local_wav
  local_matroska
  local_mp4
  local_flac
  local_ogg
  local_mp3
  local_adts
)

if ! [[ "$FUZZ_RUNS" =~ ^[1-9][0-9]*$ ]]; then
  printf 'FUZZ_RUNS must be a positive integer.\n' >&2
  exit 2
fi

if ! command -v cmake >/dev/null && [[ -x "$ROOT/.cache/media-toolchains/xaac-root/usr/bin/cmake" ]]; then
  export PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/bin:$PATH"
  export LD_LIBRARY_PATH="$ROOT/.cache/media-toolchains/xaac-root/usr/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
if ! command -v c++ >/dev/null && [[ -x "$ROOT/.cache/media-toolchains/gcc-cxx-root/usr/bin/g++" ]]; then
  export CXX="$ROOT/.cache/media-toolchains/gcc-cxx-root/usr/bin/g++"
fi

fuzz_command() {
  if [[ -n "${CARGO_FUZZ_BIN:-}" ]]; then
    "$CARGO_FUZZ_BIN" fuzz "$@"
  else
    cargo "+$FUZZ_TOOLCHAIN" fuzz "$@"
  fi
}

fuzz_version="$(fuzz_command --version)"
if [[ "$fuzz_version" != *"0.13.2"* ]]; then
  printf 'cargo-fuzz 0.13.2 is required; got %s\n' "$fuzz_version" >&2
  exit 2
fi

"$ROOT/scripts/check-local-media-fuzz-layout.sh" >/dev/null

is_known_target() {
  local candidate="$1"
  local target
  for target in "${ALL_TARGETS[@]}"; do
    if [[ "$candidate" == "$target" ]]; then
      return 0
    fi
  done
  return 1
}

seed_target() {
  local target="$1"
  local corpus="$2"
  local -a fixtures
  case "$target" in
    local_wav)
      fixtures=(tone-pcm-s16le-mono-8k.wav tone-pcm-s24le-extensible.wav)
      ;;
    local_matroska)
      fixtures=(tone-opus.webm tone-vorbis-tags.mkv tone-aac-lc-tags.mkv tone-he-aac-v1.mkv tone-he-aac-v2.mkv)
      ;;
    local_mp4)
      fixtures=(tone-aac-lc-metadata.m4a tone-aac-lc-fragmented.m4a tone-he-aac-v1.m4a tone-he-aac-v2.m4a)
      ;;
    local_flac)
      fixtures=(tone-flac.flac tone-metadata.flac)
      ;;
    local_ogg)
      fixtures=(tone-opus-tags.ogg tone-flac-tags.oga tone-vorbis-tags.ogg)
      ;;
    local_mp3)
      fixtures=(tone-mp3.mp3 tone-mp3-vbr-id3.mp3)
      ;;
    local_adts)
      fixtures=(tone-aac-lc.adts tone-aac-lc-crc.adts tone-he-aac-v1.adts tone-he-aac-v2.adts)
      ;;
    *)
      printf 'Unknown local-media fuzz target: %s\n' "$target" >&2
      exit 2
      ;;
  esac

  local fixture
  for fixture in "${fixtures[@]}"; do
    ln -s "$FIXTURES/$fixture" "$corpus/$fixture"
  done
}

if (($# == 0)); then
  targets=("${ALL_TARGETS[@]}")
else
  targets=("$@")
fi

for target in "${targets[@]}"; do
  if ! is_known_target "$target"; then
    printf 'Unknown local-media fuzz target: %s\n' "$target" >&2
    exit 2
  fi
done

SCRATCH="$(mktemp -d /tmp/mantle-media-fuzz.XXXXXX)"
readonly SCRATCH
trap 'find "$SCRATCH" -depth -delete' EXIT

for target in "${targets[@]}"; do
  corpus="$SCRATCH/$target"
  mkdir -p "$corpus"
  seed_target "$target" "$corpus"
  printf 'Fuzz smoke: %s (%s runs)\n' "$target" "$FUZZ_RUNS"
  (
    cd "$ROOT"
    fuzz_command run --target "$FUZZ_HOST" "$target" "$corpus" -- \
      "-runs=$FUZZ_RUNS" \
      "-max_len=$FUZZ_MAX_LEN" \
      "-timeout=$FUZZ_TIMEOUT_SECONDS" \
      -rss_limit_mb=2048 \
      -print_final_stats=1
  )
done

printf 'All local-media fuzz smoke targets passed.\n'
