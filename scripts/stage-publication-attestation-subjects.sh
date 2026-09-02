#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
jvm_jar=""
native_artifact_root=""
output_root="$ROOT/target/publication-sbom-provenance/subjects"
checksums="$ROOT/target/publication-sbom-provenance/subjects.sha256"

usage() {
  printf 'Usage: %s --jvm-jar PATH --native-artifact-root PATH [--output-root PATH] [--checksums PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --jvm-jar) (( $# >= 2 )) || { usage; exit 2; }; jvm_jar="$2"; shift 2 ;;
    --native-artifact-root) (( $# >= 2 )) || { usage; exit 2; }; native_artifact_root="$2"; shift 2 ;;
    --output-root) (( $# >= 2 )) || { usage; exit 2; }; output_root="$2"; shift 2 ;;
    --checksums) (( $# >= 2 )) || { usage; exit 2; }; checksums="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$jvm_jar" && -n "$native_artifact_root" ]] || { usage; exit 2; }
readonly JVM_JAR="$jvm_jar"
readonly NATIVE_ARTIFACT_ROOT="$native_artifact_root"
readonly OUTPUT_ROOT="$output_root"
readonly CHECKSUMS="$checksums"
readonly CONTRACT="$ROOT/compatibility/publication-sbom-provenance.json"

for command in jq sha256sum; do
  command -v "$command" >/dev/null || {
    printf 'Publication attestation staging requires %s\n' "$command" >&2
    exit 1
  }
done

[[ ! -e "$OUTPUT_ROOT" ]] || {
  printf 'Refusing to overwrite publication attestation subjects: %s\n' "$OUTPUT_ROOT" >&2
  exit 1
}
[[ ! -e "$CHECKSUMS" ]] || {
  printf 'Refusing to overwrite publication attestation checksums: %s\n' "$CHECKSUMS" >&2
  exit 1
}
[[ -f "$JVM_JAR" ]] || { printf 'JVM attestation subject is missing: %s\n' "$JVM_JAR" >&2; exit 1; }

mkdir -p "$OUTPUT_ROOT" "$(dirname "$CHECKSUMS")"
cp "$JVM_JAR" "$OUTPUT_ROOT/mantle-lavaplayer-1.0.0.jar"

while IFS= read -r filename; do
  source="$NATIVE_ARTIFACT_ROOT/$filename"
  [[ -f "$source" ]] || { printf 'Native attestation subject is missing: %s\n' "$source" >&2; exit 1; }
  cp "$source" "$OUTPUT_ROOT/$filename"
done < <(jq -r '.subjects[] | select(.kind == "native") | .file' "$CONTRACT")

mapfile -t expected < <(jq -r '.subjects[].file' "$CONTRACT" | LC_ALL=C sort)
mapfile -t actual < <(find "$OUTPUT_ROOT" -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)
[[ "${expected[*]}" == "${actual[*]}" ]] || {
  printf 'Staged attestation subjects differ from the six-file contract.\n' >&2
  exit 1
}

(cd "$OUTPUT_ROOT" && sha256sum "${expected[@]}") >"$CHECKSUMS"
printf 'Staged six binary attestation subjects and their SHA-256 manifest.\n'
