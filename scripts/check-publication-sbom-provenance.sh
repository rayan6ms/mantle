#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
subject_root="$ROOT/target/publication-sbom-provenance/subjects"
checksums="$ROOT/target/publication-sbom-provenance/subjects.sha256"
sbom="$ROOT/target/publication-sbom-provenance/mantle-1.0.0.cdx.json"
result="$ROOT/target/publication-sbom-provenance/result.json"

usage() {
  printf 'Usage: %s [--subject-root PATH] [--checksums PATH] [--sbom PATH] [--result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --subject-root) (( $# >= 2 )) || { usage; exit 2; }; subject_root="$2"; shift 2 ;;
    --checksums) (( $# >= 2 )) || { usage; exit 2; }; checksums="$2"; shift 2 ;;
    --sbom) (( $# >= 2 )) || { usage; exit 2; }; sbom="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly SUBJECT_ROOT="$subject_root"
readonly CHECKSUMS="$checksums"
readonly SBOM="$sbom"
readonly RESULT="$result"
readonly CONTRACT="$ROOT/compatibility/publication-sbom-provenance.json"
readonly READINESS="$ROOT/compatibility/publication-readiness.json"
readonly WORKFLOW="$ROOT/.github/workflows/native-classifier-matrix.yml"

for command in diff grep jq sha256sum; do
  command -v "$command" >/dev/null || { printf 'Publication SBOM checking requires %s\n' "$command" >&2; exit 1; }
done
for input in "$SUBJECT_ROOT" "$CHECKSUMS" "$SBOM" "$RESULT" "$CONTRACT" "$READINESS" "$WORKFLOW"; do
  [[ -e "$input" ]] || { printf 'Publication SBOM check input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-sbom-provenance" and .version == "1.0.0" and
  .sbom.format == "CycloneDX JSON" and .sbom.spec_version == "1.5" and
  (.sbom.serial_number | startswith("deterministic RFC 4122-shaped UUID")) and
  .sbom.subject_count == 6 and
  .sbom.rust_generator == {
    name: "cargo-cyclonedx", version: "0.5.9",
    scope: "union of the mantle-jvm production graph for the five supported release targets",
    dev_dependencies_included: false
  } and
  .sbom.maven_generator.coordinate == "org.cyclonedx:cyclonedx-maven-plugin:2.9.3" and
  ([.subjects[].file] | length == 6 and length == (unique | length)) and
  ([.subjects[] | select(.kind == "native") | .classifier] | sort) == [
    "linux-aarch64", "linux-x86_64", "macos-aarch64", "macos-x86_64", "windows-x86_64"
  ] and
  ([.subjects[] | select(.kind == "native") | .rust_target] | sort) == [
    "aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"
  ] and
  .attestations.action == "actions/attest@v4" and
  .attestations.predicate_types == ["SLSA build provenance", "CycloneDX SBOM"] and
  .hosted_evidence.status == "PASS" and
  .hosted_evidence.workflow_run == 33577053812 and .hosted_evidence.jobs_passed == 6 and
  .hosted_evidence.subjects_verified == 6 and
  .hosted_evidence.source_digest == "8371b73c51bc93cc66b887333ef095aa84a0f2f4" and
  .hosted_evidence.signer_workflow == "github.com/rayan6ms/mantle/.github/workflows/native-classifier-matrix.yml" and
  .hosted_evidence.provenance_attestation == {
    id: 44571212,
    url: "https://github.com/rayan6ms/mantle/attestations/44571212",
    predicate_type: "https://slsa.dev/provenance/v1",
    sigstore_log_index: 2681978438
  } and
  .hosted_evidence.sbom_attestation == {
    id: 44571213,
    url: "https://github.com/rayan6ms/mantle/attestations/44571213",
    predicate_type: "https://cyclonedx.org/bom",
    sigstore_log_index: 2681978494
  } and
  .regressions.github_cyclonedx_serial_number.failing_run == 33576693730 and
  .regressions.github_cyclonedx_serial_number.test == "scripts/test-publication-sbom-provenance.sh"
' "$CONTRACT" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "IN_PROGRESS" and
  (.completed_slices | index("publication-sbom-provenance")) != null and
  (.gates[] | select(.id == "sbom_and_provenance") |
    .status == "PASS" and .subject_count == 6 and
    .sbom_format == "CycloneDX JSON 1.5" and
    .hosted_evidence == "https://github.com/rayan6ms/mantle/actions/runs/33577053812") and
  .active_blockers == ["D-001", "central-release-identity"] and
  .next_slice == "publication-cargo-vet-exemption-closure"
' "$READINESS" >/dev/null

mapfile -t expected < <(jq -r '.subjects[].file' "$CONTRACT" | LC_ALL=C sort)
mapfile -t actual < <(find "$SUBJECT_ROOT" -maxdepth 1 -type f -printf '%f\n' | LC_ALL=C sort)
if ! diff -u <(printf '%s\n' "${expected[@]}") <(printf '%s\n' "${actual[@]}") >/dev/null; then
  printf 'Publication attestation subject files differ from the exact contract.\n' >&2
  exit 1
fi

(cd "$SUBJECT_ROOT" && sha256sum --check "$CHECKSUMS") >/dev/null
mapfile -t checksum_names < <(awk '{sub(/^\*/, "", $2); print $2}' "$CHECKSUMS" | LC_ALL=C sort)
[[ "${expected[*]}" == "${checksum_names[*]}" ]] || {
  printf 'Attestation checksum manifest does not name exactly the six subjects.\n' >&2
  exit 1
}

serial_digest="$(jq -c '[.subjects[] | {file, sha256}]' "$RESULT" | sha256sum | awk '{print $1}')"
expected_serial="urn:uuid:${serial_digest:0:8}-${serial_digest:8:4}-5${serial_digest:13:3}-8${serial_digest:17:3}-${serial_digest:20:12}"

if grep -E 'path\+file://|file://|/home/|/Users/|[A-Za-z]:\\\\|github/workspace|mantle-reference|mantle-jvm-gate|mantle-media-bench|loom@' "$SBOM" >/dev/null; then
  printf 'Publication SBOM exposes a local path or development-only component.\n' >&2
  exit 1
fi

jq --exit-status '
  ([.metadata.component["bom-ref"]] + [.components[]."bom-ref"]) as $refs |
  .bomFormat == "CycloneDX" and .specVersion == "1.5" and .version == 1 and
  (.serialNumber | test("^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-5[0-9a-f]{3}-8[0-9a-f]{3}-[0-9a-f]{12}$")) and
  .metadata.component == {
    type: "application",
    "bom-ref": "urn:mantle:release:1.0.0",
    group: "io.github.rayan6ms",
    name: "mantle-release",
    version: "1.0.0",
    licenses: [{license: {id: "Apache-2.0"}}],
    externalReferences: [{type: "vcs", url: "https://github.com/rayan6ms/mantle"}]
  } and
  .metadata.tools.components[0].name == "cargo-cyclonedx" and
  .metadata.tools.components[0].version == "0.5.9" and
  .metadata.tools.components[1].name == "cyclonedx-maven-plugin" and
  .metadata.tools.components[1].version == "2.9.3" and
  (.components | length) == 131 and
  ([.components[] | select(.["bom-ref"] | startswith("urn:mantle:artifact:"))] | length) == 6 and
  ([.components[] | select((.purl // "") | startswith("pkg:cargo/"))] | length) == 108 and
  ([.components[] | select(.["bom-ref"] | startswith("pkg:maven/"))] | length) == 17 and
  ([.components[]."bom-ref"] | length == (unique | length)) and
  all(.dependencies[];
    . as $edge |
    (($refs | index($edge.ref)) != null) and
    (($edge.dependsOn // []) | all(. as $dependency | ($refs | index($dependency)) != null))) and
  (.dependencies[] | select(.ref == "urn:mantle:release:1.0.0") | .dependsOn | length) == 6 and
  (.dependencies[] | select(.ref == "urn:mantle:artifact:mantle-lavaplayer-1.0.0.jar") |
    .dependsOn == ["pkg:maven/io.github.rayan6ms/mantle-lavaplayer@1.0.0?type=jar"]) and
  ([.dependencies[] | select(.ref | startswith("urn:mantle:artifact:mantle-native-")) |
    select(.dependsOn == ["pkg:cargo/mantle-jvm@1.0.0"])] | length) == 5 and
  ([.components[] | select(.["bom-ref"] | startswith("urn:mantle:artifact:mantle-native-")) |
    select(([.properties[] | select(.name == "io.github.rayan6ms.mantle:rust-target")] | length) == 1)] | length) == 5 and
  ([.components[].name] | index("loom")) == null and
  ([.components[].name] | index("mantle-reference")) == null
' "$SBOM" >/dev/null

while IFS=$'\t' read -r filename expected_sha; do
  component_sha="$(jq -r --arg ref "urn:mantle:artifact:$filename" '
    .components[] | select(.["bom-ref"] == $ref) | .hashes[] |
    select(.alg == "SHA-256") | .content
  ' "$SBOM")"
  [[ "$component_sha" == "$expected_sha" ]] || {
    printf 'SBOM hash does not match attestation subject: %s\n' "$filename" >&2
    exit 1
  }
done < <(awk '{sub(/^\*/, "", $2); print $2 "\t" $1}' "$CHECKSUMS")

jq --exit-status \
  --arg sbom "$(basename "$SBOM")" \
  --arg sha "$(sha256sum "$SBOM" | awk '{print $1}')" \
  --arg serial "$expected_serial" '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-sbom-provenance" and
  .sbom == $sbom and .sbom_sha256 == $sha and
  .serial_number == $serial and
  .format == "CycloneDX JSON 1.5" and
  .subject_count == 6 and (.subjects | length) == 6 and
  .component_count == 131 and .dependency_relationship_count == 132 and
  .generators == ["cargo-cyclonedx 0.5.9", "org.cyclonedx:cyclonedx-maven-plugin:2.9.3"] and
  .hosted_attestations == "PENDING"
' "$RESULT" >/dev/null

[[ "$(jq -r '.serialNumber' "$SBOM")" == "$expected_serial" ]] || {
  printf 'SBOM serial number is not derived from the six attestation subjects.\n' >&2
  exit 1
}

[[ "$(grep -c -- 'uses: actions/attest@v4' "$WORKFLOW")" == 2 ]]
grep -F 'id-token: write' "$WORKFLOW" >/dev/null
grep -F 'attestations: write' "$WORKFLOW" >/dev/null
grep -F 'artifact-metadata: write' "$WORKFLOW" >/dev/null
[[ "$(grep -c -- 'subject-checksums: target/publication-sbom-provenance/subjects.sha256' "$WORKFLOW")" == 2 ]]
grep -F 'sbom-path: target/publication-sbom-provenance/mantle-1.0.0.cdx.json' "$WORKFLOW" >/dev/null

printf 'Publication SBOM/provenance preflight passed: six hashed subjects, complete production graphs, and two hosted attestation modes are wired.\n'
