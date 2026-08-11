#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

mapfile -d '' forbidden < <(
  find "$ROOT" \
    -type d \( -name .git -o -name .cache -o -name target \) -prune -o \
    -type f \( -name '*.java' -o -name '*.kt' -o -name '*.kts' \) -print0
)

if (( ${#forbidden[@]} > 0 )); then
  printf 'Java/Kotlin source is forbidden in the Mantle repository:\n' >&2
  printf '  %s\n' "${forbidden[@]}" >&2
  exit 1
fi

printf 'no Java/Kotlin source files found\n'
