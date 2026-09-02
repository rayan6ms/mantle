#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly STAGER="$ROOT/scripts/stage-publication-central-bundle.sh"
readonly CHECKER="$ROOT/scripts/check-publication-central-bundle.sh"

for command in gpg jq unzip zip; do
  command -v "$command" >/dev/null || {
    printf 'Central bundle regression test requires %s\n' "$command" >&2
    exit 1
  }
done

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

inputs="$test_root/inputs"
native_root="$inputs/native"
mkdir -p "$inputs/jvm/com/example" "$native_root"
printf 'fixture class bytes\n' >"$inputs/jvm/com/example/Mantle.class"
(cd "$inputs/jvm" && zip -q -X -r "$inputs/mantle-lavaplayer-1.0.0.jar" .)

for classifier in linux-x86_64 linux-aarch64 macos-x86_64 macos-aarch64 windows-x86_64; do
  fixture="$test_root/$classifier"
  mkdir -p "$fixture/native"
  printf 'fixture native bytes for %s\n' "$classifier" >"$fixture/native/library"
  (cd "$fixture" && zip -q -X -r "$native_root/mantle-native-1.0.0-$classifier.jar" .)
done

gpg_home="$test_root/gpg"
mkdir -m 700 "$gpg_home"
gpg --batch --quiet --homedir "$gpg_home" --passphrase '' \
  --quick-generate-key 'Mantle publication test <publication-test@invalid.example>' ed25519 sign 1d
key_id="$(gpg --batch --homedir "$gpg_home" --with-colons --list-secret-keys |
  awk -F: '$1 == "sec" {print $5; exit}')"
[[ -n "$key_id" ]]

repository="$test_root/repository"
bundle="$test_root/central-bundle.zip"
result="$test_root/result.json"
"$STAGER" \
  --jvm-jar "$inputs/mantle-lavaplayer-1.0.0.jar" \
  --native-artifact-root "$native_root" \
  --output-root "$repository" \
  --bundle "$bundle" \
  --result "$result" \
  --gpg-homedir "$gpg_home" \
  --gpg-key "$key_id" >/dev/null
"$CHECKER" \
  --repository-root "$repository" \
  --bundle "$bundle" \
  --result "$result" \
  --gpg-homedir "$gpg_home" >/dev/null

missing_signature="$test_root/missing-signature"
cp -a "$repository" "$missing_signature"
rm "$missing_signature/io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0-sources.jar.asc"
if "$CHECKER" \
  --repository-root "$missing_signature" \
  --bundle "$bundle" \
  --result "$result" \
  --gpg-homedir "$gpg_home" >/dev/null 2>&1; then
  printf 'Central bundle checker accepted a missing detached signature.\n' >&2
  exit 1
fi

stale_metadata="$test_root/stale-metadata"
cp -a "$repository" "$stale_metadata"
sed -i 's#https://github.com/rayan6ms/mantle#https://github.com/rayan6ms/stale-repository#g' \
  "$stale_metadata/io/github/rayan6ms/mantle-native/1.0.0/mantle-native-1.0.0.pom"
if "$CHECKER" \
  --repository-root "$stale_metadata" \
  --bundle "$bundle" \
  --result "$result" \
  --gpg-homedir "$gpg_home" >/dev/null 2>&1; then
  printf 'Central bundle checker accepted stale repository metadata.\n' >&2
  exit 1
fi

forbidden_source="$test_root/forbidden-source"
cp -a "$repository" "$forbidden_source"
source_jar="$forbidden_source/io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0-sources.jar"
mkdir -p "$test_root/java/com/example"
printf 'final class Forbidden {}\n' >"$test_root/java/com/example/Forbidden.java"
(cd "$test_root/java" && zip -q "$source_jar" com/example/Forbidden.java)
if "$CHECKER" \
  --repository-root "$forbidden_source" \
  --bundle "$bundle" \
  --result "$result" \
  --gpg-homedir "$gpg_home" >/dev/null 2>&1; then
  printf 'Central bundle checker accepted Java source in the placeholder archive.\n' >&2
  exit 1
fi

printf 'Central bundle success, metadata, signature, and forbidden-source regressions passed.\n'
