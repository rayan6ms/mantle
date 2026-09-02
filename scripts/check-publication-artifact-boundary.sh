#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
staging_root="$ROOT/target/publication-readiness/repository"
result="$ROOT/target/publication-readiness/artifact-boundary/result.json"

usage() {
  printf 'Usage: %s [--staging-root PATH] [--result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --staging-root) (( $# >= 2 )) || { usage; exit 2; }; staging_root="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly STAGING_ROOT="$staging_root"
readonly RESULT="$result"
readonly PLAN="$ROOT/compatibility/publication-readiness.json"
readonly JVM_REL="io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0.jar"
readonly JVM_POM_REL="io/github/rayan6ms/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0.pom"
readonly NATIVE_REL="io/github/rayan6ms/mantle-native/1.0.0/mantle-native-1.0.0-linux-x86_64.jar"
readonly NATIVE_POM_REL="io/github/rayan6ms/mantle-native/1.0.0/mantle-native-1.0.0.pom"

for command in cargo jq sha256sum unzip xmllint; do
  command -v "$command" >/dev/null || { printf 'Publication boundary requires %s\n' "$command" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and
  .phase == "publication-readiness" and
  (.completed_slices | index("publication-artifact-boundary")) != null and
  .completed_slice == "publication-central-release" and
  (.completed_slices | index("publication-central-validation-deployment")) != null and
  (.completed_slices | index("publication-central-release")) != null and
  .publication_ready == true and
  .channels.maven.status == "PUBLISHED" and
  (.channels.maven.artifact_boundary_files | length) == 4 and
  (.channels.maven.central_deployables | length) == 10 and
  .channels.maven.source_artifact == "TRUTHFUL_PLACEHOLDER_NOTICE" and
  .channels.maven.documentation_artifact == "TRUTHFUL_PLACEHOLDER_NOTICE" and
  .channels.maven.network_upload_performed == true and
  .channels.maven.deployment_id == "9205c170-5232-4817-980f-0ff92e581ee9" and
  .channels.maven.public_repository == "https://repo.maven.apache.org/maven2" and
  .channels.cargo_registry.status == "DISABLED" and
  .channels.cargo_registry.workspace_packages == 12 and
  .channels.cargo_registry.fuzz_packages == 1 and
  .channels.repository_snapshot.status == "NOT_PUBLISHED" and
  ([.gates[].id] | sort) == ["artifact_boundary", "central_metadata_and_signing", "central_public_release", "central_release_identity", "central_validation_deployment", "dependency_audits", "native_classifier_matrix", "sbom_and_provenance"] and
  (.gates[] | select(.id == "artifact_boundary") | .status) == "PASS" and
  (.gates[] | select(.id == "dependency_audits") |
    .status == "PASS" and .remaining_exact_version_exemptions == 0) and
  (.gates[] | select(.id == "native_classifier_matrix") | .status) == "PASS" and
  (.gates[] | select(.id == "central_metadata_and_signing") | .status) == "PASS" and
  (.gates[] | select(.id == "sbom_and_provenance") |
    .status == "PASS" and .subject_count == 6 and
    .sbom_format == "CycloneDX JSON 1.5") and
  (.gates[] | select(.id == "central_release_identity") | .status) == "PASS" and
  (.gates[] | select(.id == "central_validation_deployment") |
    .status == "PASS" and .validated_state == "VALIDATED" and
    .artifact_publication_performed == false) and
  (.gates[] | select(.id == "central_public_release") |
    .status == "PASS" and .terminal_state == "PUBLISHED" and
    .repository_file_count == 60) and
  .active_blockers == [] and .next_slice == null
' "$PLAN" >/dev/null

mapfile -t expected < <(jq --raw-output '.channels.maven.artifact_boundary_files[]' "$PLAN" | sort)
mapfile -t actual < <(find "$STAGING_ROOT" -type f -printf '%P\n' | sort)
if ! diff -u <(printf '%s\n' "${expected[@]}") <(printf '%s\n' "${actual[@]}"); then
  printf 'Publication staging differs from the exact four-file allowlist.\n' >&2
  exit 1
fi

jvm_entries="$(unzip -Z1 "$STAGING_ROOT/$JVM_REL")"
if awk '
  /\.class$/ {next}
  $0 == "META-INF/MANIFEST.MF" {next}
  $0 == "certificates/bundled.txt" {next}
  $0 == "certificates/dst-root-ca-x3.jks" {next}
  $0 == "com/sedmelluq/discord/lavaplayer/tools/version.txt" {next}
  {print; unexpected = 1}
  END {exit unexpected}
' <<<"$jvm_entries"; then
  :
else
  printf 'JVM artifact contains a non-allowlisted entry.\n' >&2
  exit 1
fi
[[ "$(grep -c '\.class$' <<<"$jvm_entries")" == 445 ]]

expected_native=$'META-INF/\nMETA-INF/MANIFEST.MF\nnative/\nnative/libmantle_jvm.so'
[[ "$(unzip -Z1 "$STAGING_ROOT/$NATIVE_REL" | tr -d '\r')" == "$expected_native" ]] || {
  printf 'Native artifact contains a non-allowlisted entry.\n' >&2
  exit 1
}

for archive in "$STAGING_ROOT/$JVM_REL" "$STAGING_ROOT/$NATIVE_REL"; do
  if unzip -Z1 "$archive" | awk '
    BEGIN {IGNORECASE = 1}
    /(^|\/)AGENTS\.md$/ || /(^|\/)docs?\// || /\.md$/ || /(^|\/)scripts?\// ||
    /(^|\/)tests?\// || /(^|\/)\.github\// || /(^|\/)\.git\// ||
    /(^|\/)(TASKS|STATUS|PROJECT_LEDGER|COMPATIBILITY)\.md$/ {found = 1}
    END {exit !found}
  '; then
    printf 'Publication archive exposes a forbidden local-development path: %s\n' "$archive" >&2
    exit 1
  fi
done

xmllint --noout "$STAGING_ROOT/$JVM_POM_REL" "$STAGING_ROOT/$NATIVE_POM_REL"
for pom in "$STAGING_ROOT/$JVM_POM_REL" "$STAGING_ROOT/$NATIVE_POM_REL"; do
  if rg --ignore-case 'AGENTS\.md|docs/|TASKS\.md|STATUS\.md|PROJECT_LEDGER\.md|COMPATIBILITY\.md|target/|scripts/' "$pom" >/dev/null; then
    printf 'Published POM exposes a local-development path: %s\n' "$pom" >&2
    exit 1
  fi
done

metadata="$(env -u APPIMAGE -u APPDIR cargo metadata --no-deps --format-version 1)"
jq --exit-status '
  ([.packages[].name] | sort) == [
    "mantle-audio", "mantle-audio-layout-bench", "mantle-core", "mantle-jvm",
    "mantle-jvm-gate", "mantle-media", "mantle-media-bench", "mantle-opus",
    "mantle-oracle", "mantle-reference", "mantle-worker-bench", "mantle-xaac"
  ] and all(.packages[]; .publish == [])
' <<<"$metadata" >/dev/null
jq --exit-status '.packages == null or all(.packages[]; .publish == [])' \
  < <(env -u APPIMAGE -u APPDIR cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/fuzz/Cargo.toml") >/dev/null

jq --exit-status --slurpfile plan "$PLAN" '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-artifact-boundary" and
  (.public_files | length) == 4 and
  ([.public_files[].path] | sort) == ($plan[0].channels.maven.artifact_boundary_files | sort) and
  all(.public_files[]; (.sha256 | test("^[0-9a-f]{64}$"))) and
  .cargo_registry_publication == false and
  .active_blockers == ["D-001", "native-classifier-matrix", "central-metadata-and-signing"]
' "$RESULT" >/dev/null

while IFS=$'\t' read -r path expected_sha; do
  actual_sha="$(sha256sum "$STAGING_ROOT/$path" | awk '{print $1}')"
  [[ "$actual_sha" == "$expected_sha" ]] || {
    printf 'Staged publication hash mismatch: %s\n' "$path" >&2
    exit 1
  }
done < <(jq --raw-output '.public_files[] | [.path, .sha256] | @tsv' "$RESULT")

printf 'Publication artifact boundary passed: four Maven files, private Cargo packages, and no agent, documentation, Markdown, test, script, or repository-only payload.\n'
