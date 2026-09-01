#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
artifact_root="$ROOT/target/publication-native-matrix/artifacts"
result="$ROOT/target/publication-native-matrix/result.json"

usage() {
  printf 'Usage: %s [--artifact-root PATH] [--result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --artifact-root) (( $# >= 2 )) || { usage; exit 2; }; artifact_root="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly ARTIFACT_ROOT="$artifact_root"
readonly RESULT="$result"
readonly CONTRACT="$ROOT/compatibility/publication-native-matrix.json"

for command in file grep jq strings unzip; do
  command -v "$command" >/dev/null || { printf 'Native matrix checking requires %s\n' "$command" >&2; exit 1; }
done
jq --exit-status '
  .schema_version == 1 and (.status == "VALIDATING" or .status == "PASS") and
  .slice == "publication-native-matrix" and .version == "1.0.0" and
  .artifact_id == "mantle-native" and (.classifiers | length) == 5 and
  ([.classifiers[].id] | sort) == [
    "linux-aarch64", "linux-x86_64", "macos-aarch64", "macos-x86_64", "windows-x86_64"
  ]
' "$CONTRACT" >/dev/null

sha256_file() {
  if command -v sha256sum >/dev/null; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

work="$(mktemp -d)"
readonly work
trap 'rm -rf "$work"' EXIT
records="$work/records.jsonl"
: >"$records"

while IFS=$'\t' read -r classifier rust_host library binary_format; do
  jar_path="$ARTIFACT_ROOT/mantle-native-1.0.0-$classifier.jar"
  metadata_path="$ARTIFACT_ROOT/mantle-native-1.0.0-$classifier.json"
  [[ -f "$jar_path" && -f "$metadata_path" ]] || {
    printf 'Missing classifier artifact or metadata: %s\n' "$classifier" >&2
    exit 1
  }

  jq --exit-status \
    --arg classifier "$classifier" \
    --arg rust_host "$rust_host" \
    --arg library "$library" \
    --arg jar "$(basename "$jar_path")" '
      .schema_version == 1 and .status == "PASS" and
      .classifier == $classifier and .rust_host == $rust_host and
      .library == $library and .jar == $jar and
      (.jar_sha256 | test("^[0-9a-f]{64}$")) and
      (.library_sha256 | test("^[0-9a-f]{64}$")) and .library_bytes > 0
    ' "$metadata_path" >/dev/null

  expected_entries="$(printf 'META-INF/\nMETA-INF/MANIFEST.MF\nnative/\nnative/%s' "$library")"
  actual_entries="$(unzip -Z1 "$jar_path" | tr -d '\r')"
  [[ "$actual_entries" == "$expected_entries" ]] || {
    printf 'Unexpected archive layout for %s:\n%s\n' "$classifier" "$actual_entries" >&2
    exit 1
  }

  extract_root="$work/$classifier"
  mkdir -p "$extract_root"
  unzip -q "$jar_path" -d "$extract_root"
  binary="$extract_root/native/$library"
  file_description="$(file -b "$binary")"
  case "$binary_format" in
    elf-x86_64) [[ "$file_description" == *"ELF 64-bit LSB shared object"* && "$file_description" == *"x86-64"* ]] ;;
    elf-aarch64) [[ "$file_description" == *"ELF 64-bit LSB shared object"* && "$file_description" == *"ARM aarch64"* ]] ;;
    mach-o-x86_64) [[ "$file_description" == *"Mach-O 64-bit"* && "$file_description" == *"x86_64"* ]] ;;
    mach-o-arm64) [[ "$file_description" == *"Mach-O 64-bit"* && "$file_description" == *"arm64"* ]] ;;
    pe-x86_64) [[ "$file_description" == *"PE32+"* && "$file_description" == *"x86-64"* && "$file_description" == *"DLL"* ]] ;;
    *) printf 'Unknown binary format contract: %s\n' "$binary_format" >&2; exit 1 ;;
  esac || {
    printf 'Binary format mismatch for %s: %s\n' "$classifier" "$file_description" >&2
    exit 1
  }

  if strings "$binary" | grep -E --ignore-case \
    '/home/runner/work/|[A-Z]:\\a\\|AGENTS\.md|PROJECT_LEDGER\.md|TASKS\.md|STATUS\.md|(^|/)docs/' >/dev/null; then
    printf 'Native classifier exposes a private development path: %s\n' "$classifier" >&2
    exit 1
  fi

  jar_sha256="$(sha256_file "$jar_path")"
  library_sha256="$(sha256_file "$binary")"
  [[ "$jar_sha256" == "$(jq -r '.jar_sha256' "$metadata_path")" ]]
  [[ "$library_sha256" == "$(jq -r '.library_sha256' "$metadata_path")" ]]

  jq -n \
    --arg classifier "$classifier" \
    --arg rust_host "$rust_host" \
    --arg library "$library" \
    --arg binary_format "$binary_format" \
    --arg jar_sha256 "$jar_sha256" \
    --arg library_sha256 "$library_sha256" \
    --arg file_description "$file_description" \
    --argjson library_bytes "$(jq '.library_bytes' "$metadata_path")" \
    '{classifier: $classifier, rust_host: $rust_host, library: $library,
      binary_format: $binary_format, jar_sha256: $jar_sha256,
      library_sha256: $library_sha256, library_bytes: $library_bytes,
      file_description: $file_description}' >>"$records"
done < <(jq -r '.classifiers[] | [.id, .rust_host, .library, .binary_format] | @tsv' "$CONTRACT")

mkdir -p "$(dirname "$RESULT")"
jq -s '{
  schema_version: 1,
  status: "PASS",
  slice: "publication-native-matrix",
  classifier_count: length,
  classifiers: .
}' "$records" >"$RESULT"
jq --exit-status '.status == "PASS" and .classifier_count == 5' "$RESULT" >/dev/null

printf 'Publication native matrix passed: five classifier JARs match their Rust hosts, binary formats, private-path policy, and recorded hashes.\n'
