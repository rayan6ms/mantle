#!/usr/bin/env bash
set -euo pipefail

classifier=""
library=""
output=""
metadata=""

usage() {
  printf 'Usage: %s --classifier CLASSIFIER --library PATH --output JAR --metadata JSON\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --classifier) (( $# >= 2 )) || { usage; exit 2; }; classifier="$2"; shift 2 ;;
    --library) (( $# >= 2 )) || { usage; exit 2; }; library="$2"; shift 2 ;;
    --output) (( $# >= 2 )) || { usage; exit 2; }; output="$2"; shift 2 ;;
    --metadata) (( $# >= 2 )) || { usage; exit 2; }; metadata="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$classifier" && -n "$library" && -n "$output" && -n "$metadata" ]] || { usage; exit 2; }
for command in jar jq rustc unzip; do
  command -v "$command" >/dev/null || { printf 'Native classifier packaging requires %s\n' "$command" >&2; exit 1; }
done

case "$classifier" in
  linux-x86_64) expected_host="x86_64-unknown-linux-gnu"; expected_library="libmantle_jvm.so" ;;
  linux-aarch64) expected_host="aarch64-unknown-linux-gnu"; expected_library="libmantle_jvm.so" ;;
  macos-x86_64) expected_host="x86_64-apple-darwin"; expected_library="libmantle_jvm.dylib" ;;
  macos-aarch64) expected_host="aarch64-apple-darwin"; expected_library="libmantle_jvm.dylib" ;;
  windows-x86_64) expected_host="x86_64-pc-windows-msvc"; expected_library="mantle_jvm.dll" ;;
  *) printf 'Unsupported native classifier: %s\n' "$classifier" >&2; exit 1 ;;
esac
readonly expected_host expected_library

[[ -f "$library" ]] || { printf 'Native library is missing: %s\n' "$library" >&2; exit 1; }
[[ "$(basename "$library")" == "$expected_library" ]] || {
  printf 'Classifier %s requires library %s, got %s\n' "$classifier" "$expected_library" "$(basename "$library")" >&2
  exit 1
}

actual_host="$(rustc -vV | awk '$1 == "host:" {print $2}')"
[[ "$actual_host" == "$expected_host" ]] || {
  printf 'Classifier %s requires Rust host %s, got %s\n' "$classifier" "$expected_host" "$actual_host" >&2
  exit 1
}

mkdir -p "$(dirname "$output")" "$(dirname "$metadata")"
output="$(cd "$(dirname "$output")" && pwd)/$(basename "$output")"
metadata="$(cd "$(dirname "$metadata")" && pwd)/$(basename "$metadata")"
readonly output metadata

stage="$(mktemp -d)"
readonly stage
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/native"
cp "$library" "$stage/native/$expected_library"
jar --create --file "$output" -C "$stage" native

expected_entries="$(printf 'META-INF/\nMETA-INF/MANIFEST.MF\nnative/\nnative/%s' "$expected_library")"
actual_entries="$(unzip -Z1 "$output" | tr -d '\r')"
[[ "$actual_entries" == "$expected_entries" ]] || {
  printf 'Classifier JAR has an unexpected layout:\n%s\n' "$actual_entries" >&2
  exit 1
}

sha256_file() {
  if command -v sha256sum >/dev/null; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

jar_sha256="$(sha256_file "$output")"
library_sha256="$(sha256_file "$library")"
library_bytes="$(wc -c <"$library" | tr -d '[:space:]')"
jq -n \
  --arg classifier "$classifier" \
  --arg rust_host "$actual_host" \
  --arg library "$expected_library" \
  --arg jar "$(basename "$output")" \
  --arg jar_sha256 "$jar_sha256" \
  --arg library_sha256 "$library_sha256" \
  --argjson library_bytes "$library_bytes" \
  '{
    schema_version: 1,
    status: "PASS",
    classifier: $classifier,
    rust_host: $rust_host,
    library: $library,
    jar: $jar,
    jar_sha256: $jar_sha256,
    library_sha256: $library_sha256,
    library_bytes: $library_bytes
  }' >"$metadata"

printf 'Packaged %s for %s (%s bytes).\n' "$(basename "$output")" "$actual_host" "$library_bytes"
