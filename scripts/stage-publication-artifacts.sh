#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
output_root="$ROOT/target/publication-readiness/repository"
jvm_jar="${MANTLE_PUBLICATION_JVM_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
native_jar="${MANTLE_PUBLICATION_NATIVE_JAR:-$ROOT/target/phase14/lavalink-source-compatibility/mantle-native-1.0.0-linux-x86_64.jar}"

usage() {
  printf 'Usage: %s [--output-root PATH] [--jvm-jar PATH] [--native-jar PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --output-root) (( $# >= 2 )) || { usage; exit 2; }; output_root="$2"; shift 2 ;;
    --jvm-jar) (( $# >= 2 )) || { usage; exit 2; }; jvm_jar="$2"; shift 2 ;;
    --native-jar) (( $# >= 2 )) || { usage; exit 2; }; native_jar="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly OUTPUT_ROOT="$output_root"
readonly JVM_JAR="$jvm_jar"
readonly NATIVE_JAR="$native_jar"
readonly JVM_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly NATIVE_POM="$ROOT/compatibility/mantle-native-1.0.0.pom"
readonly RESULT_ROOT="$ROOT/target/publication-readiness/artifact-boundary"
readonly RESULT="$RESULT_ROOT/result.json"

[[ ! -e "$OUTPUT_ROOT" ]] || {
  printf 'Refusing to overwrite publication staging: %s\n' "$OUTPUT_ROOT" >&2
  exit 1
}
for source in "$JVM_JAR" "$NATIVE_JAR" "$JVM_POM" "$NATIVE_POM"; do
  [[ -f "$source" ]] || { printf 'Publication input is missing: %s\n' "$source" >&2; exit 1; }
done

jvm_dir="$OUTPUT_ROOT/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
native_dir="$OUTPUT_ROOT/io/github/rayan6ms/mantle-native/1.0.0"
mkdir -p "$jvm_dir" "$native_dir" "$RESULT_ROOT"
cp "$JVM_JAR" "$jvm_dir/mantle-lavaplayer-1.0.0.jar"
cp "$JVM_POM" "$jvm_dir/mantle-lavaplayer-1.0.0.pom"
cp "$NATIVE_JAR" "$native_dir/mantle-native-1.0.0-linux-x86_64.jar"
cp "$NATIVE_POM" "$native_dir/mantle-native-1.0.0.pom"

jvm_sha="$(sha256sum "$jvm_dir/mantle-lavaplayer-1.0.0.jar" | awk '{print $1}')"
jvm_pom_sha="$(sha256sum "$jvm_dir/mantle-lavaplayer-1.0.0.pom" | awk '{print $1}')"
native_sha="$(sha256sum "$native_dir/mantle-native-1.0.0-linux-x86_64.jar" | awk '{print $1}')"
native_pom_sha="$(sha256sum "$native_dir/mantle-native-1.0.0.pom" | awk '{print $1}')"

jq -n \
  --arg staging_root "target/publication-readiness/repository" \
  --arg jvm_sha "$jvm_sha" \
  --arg jvm_pom_sha "$jvm_pom_sha" \
  --arg native_sha "$native_sha" \
  --arg native_pom_sha "$native_pom_sha" \
  '{
    schema_version: 1,
    status: "PASS",
    slice: "publication-artifact-boundary",
    staging_root: $staging_root,
    public_files: [
      {path: "io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0.jar", sha256: $jvm_sha},
      {path: "io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0.pom", sha256: $jvm_pom_sha},
      {path: "io/github/rayan6ms/mantle-native/1.0.0/mantle-native-1.0.0-linux-x86_64.jar", sha256: $native_sha},
      {path: "io/github/rayan6ms/mantle-native/1.0.0/mantle-native-1.0.0.pom", sha256: $native_pom_sha}
    ],
    excluded_classes: ["agent files", "development documentation", "Markdown", "tests", "scripts", "plans", "local evidence", "repository metadata"],
    cargo_registry_publication: false,
    active_blockers: ["D-001", "native-classifier-matrix", "central-metadata-and-signing"]
  }' >"$RESULT"

printf 'Staged four allowlisted publication files; no source, documentation, or repository snapshot was staged.\n'
