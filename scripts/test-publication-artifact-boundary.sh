#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-publication-artifact-boundary.sh"
readonly STAGING="$ROOT/target/publication-readiness/repository"
readonly RESULT="$ROOT/target/publication-readiness/artifact-boundary/result.json"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

"$CHECKER" >/dev/null

extra_root="$test_root/extra"
cp -a "$STAGING" "$extra_root"
printf 'local agent guidance\n' >"$extra_root/AGENTS.md"
if "$CHECKER" --staging-root "$extra_root" --result "$RESULT" >/dev/null 2>&1; then
  printf 'Publication boundary accepted an extra agent file.\n' >&2
  exit 1
fi

archive_root="$test_root/archive"
cp -a "$STAGING" "$archive_root"
mkdir -p "$test_root/injected/docs"
printf 'local design notes\n' >"$test_root/injected/docs/internal.md"
jvm="$archive_root/io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0.jar"
(cd "$test_root/injected" && zip -q "$jvm" docs/internal.md)
bad_result="$test_root/bad-result.json"
jq --arg sha "$(sha256sum "$jvm" | awk '{print $1}')" '
  (.public_files[] | select(.path | endswith("mantle-lavaplayer-1.0.0.jar")) | .sha256) = $sha
' "$RESULT" >"$bad_result"
if "$CHECKER" --staging-root "$archive_root" --result "$bad_result" >/dev/null 2>&1; then
  printf 'Publication boundary accepted documentation inside the JVM archive.\n' >&2
  exit 1
fi

printf 'Publication artifact boundary success and forbidden-file paths passed.\n'
