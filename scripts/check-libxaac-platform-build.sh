#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REVISION="8d9809f480fb56c68ff6b76927aceb382d55045e"
readonly REPOSITORY="https://android.googlesource.com/platform/external/libxaac"
readonly SOURCE="${LIBXAAC_SOURCE:-$ROOT/.cache/libxaac-platform-source}"
readonly PATCH="$ROOT/third_party/libxaac/patches/0001-bound-decoder-indices-and-lifetimes.patch"

if [[ ! -f "$SOURCE/CMakeLists.txt" ]]; then
  mkdir -p "$SOURCE"
  if [[ ! -d "$SOURCE/.git" ]]; then
    git -C "$SOURCE" init --quiet
    git -C "$SOURCE" config core.autocrlf false
  fi
  if ! git -C "$SOURCE" remote get-url origin >/dev/null 2>&1; then
    git -C "$SOURCE" remote add origin "$REPOSITORY"
  fi
  git -C "$SOURCE" fetch --quiet --depth=1 origin "$REVISION"
  git -C "$SOURCE" checkout --quiet --detach FETCH_HEAD
fi

actual_revision="$(git -C "$SOURCE" rev-parse HEAD)"
if [[ "$actual_revision" != "$REVISION" ]]; then
  printf 'libxaac revision mismatch: expected %s, got %s\n' \
    "$REVISION" "$actual_revision" >&2
  exit 1
fi

if git -C "$SOURCE" apply --reverse --check "$PATCH" >/dev/null 2>&1; then
  : # The exact patch is already present in a reusable source cache.
else
  if ! git -C "$SOURCE" diff --quiet --; then
    printf 'libxaac source has unexpected tracked changes before patching\n' >&2
    exit 1
  fi
  git -C "$SOURCE" apply --check "$PATCH"
  git -C "$SOURCE" apply "$PATCH"
fi

VERIFY_INDEX="$(mktemp "$ROOT/.cache/libxaac-verify-index.XXXXXX")"
readonly VERIFY_INDEX
trap 'rm -f "$VERIFY_INDEX"' EXIT
rm -f "$VERIFY_INDEX"
GIT_INDEX_FILE="$VERIFY_INDEX" git -C "$SOURCE" read-tree HEAD
GIT_INDEX_FILE="$VERIFY_INDEX" git -C "$SOURCE" apply --cached "$PATCH"
expected_tree="$(GIT_INDEX_FILE="$VERIFY_INDEX" git -C "$SOURCE" write-tree)"
GIT_INDEX_FILE="$VERIFY_INDEX" git -C "$SOURCE" add --update
actual_tree="$(GIT_INDEX_FILE="$VERIFY_INDEX" git -C "$SOURCE" write-tree)"
if [[ "$actual_tree" != "$expected_tree" ]]; then
  printf 'libxaac source differs from the tracked Mantle patch set\n' >&2
  exit 1
fi
git -C "$SOURCE" diff --check

mkdir -p "$ROOT/.cache"
readonly BUILD="${LIBXAAC_BUILD_DIR:-$(mktemp -d "$ROOT/.cache/libxaac-platform-build.XXXXXX")}"
cmake -S "$SOURCE" -B "$BUILD" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_COMPILE_WARNING_AS_ERROR=ON \
  -DCMAKE_POSITION_INDEPENDENT_CODE=ON
cmake --build "$BUILD" --config Release --target libxaacdec \
  --parallel "${LIBXAAC_JOBS:-2}"

archive="$(find "$BUILD" -type f \( -name 'libxaacdec.a' -o -name 'libxaacdec.lib' \) -print -quit)"
if [[ -z "$archive" ]]; then
  printf 'libxaac decoder build completed without a static archive\n' >&2
  exit 1
fi

printf 'built libxaac decoder revision %s at %s\n' "$REVISION" "$archive"
