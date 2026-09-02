#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
result="$ROOT/target/publication-central-release/result.json"

usage() {
  printf 'Usage: %s [--result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly RESULT="$result"
readonly CONTRACT="$ROOT/compatibility/publication-central-release.json"
readonly READINESS="$ROOT/compatibility/publication-readiness.json"
for input in "$CONTRACT" "$READINESS" "$RESULT"; do
  [[ -f "$input" ]] || { printf 'Central public release input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "OPERATOR_PUBLISHED" and
  .slice == "publication-central-release" and .version == "1.0.0" and
  .source_digest == "62d02f6f0c6a312dc1ec5fec85213dc7e24cc4c1" and
  .deployment.id == "9205c170-5232-4817-980f-0ff92e581ee9" and
  .deployment.name == "Mantle 1.0.0 validation 62d02f6" and
  .deployment.state == "PUBLISHED" and
  .public_repository.base_url == "https://repo.maven.apache.org/maven2" and
  .public_repository.group_id == "io.github.rayan6ms" and
  .public_repository.coordinates == [
    "io.github.rayan6ms:mantle-lavaplayer:1.0.0",
    "io.github.rayan6ms:mantle-native:1.0.0"
  ] and
  .public_repository.repository_file_count == 60 and
  .public_repository.repository_manifest_sha256 == "1ab7f682451ea5d3283b6cb29df20f23d71da80eca59569eea96645b3ae41a6d" and
  .release_artifacts == {
    bundle_sha256: "f677efdd40a8ae10f68e381f83c98d12ae3c43303065932f905f74de4ee43d1e",
    signing_fingerprint: "B7CEFF5211EA68CB834AED549FA70609D2B9F145",
    deployable_count: 10,
    integrity_sidecars_per_deployable: 5
  } and
  .verification.workflow == ".github/workflows/central-public-release.yml" and
  .verification.runner == "scripts/run-publication-central-release.sh" and
  .verification.checker == "scripts/check-publication-central-release.sh" and
  .verification.validation_contract == "compatibility/publication-central-validation-deployment.json" and
  .verification.public_maven_resolution == "REQUIRED" and
  .verification.public_native_loader_smoke == "REQUIRED" and
  (.hosted_evidence.status == "PENDING" or
    (.hosted_evidence.status == "PASS" and
     (.hosted_evidence.workflow_run | type) == "number" and
     (.hosted_evidence.result_sha256 | test("^[0-9a-f]{64}$")))) and
  .release_policy == {
    user_managed_validation_preceded_publication: true,
    deployment_consumer_gate_preceded_publication: true,
    artifact_publication_performed: true,
    release_complete: true
  }
' "$CONTRACT" >/dev/null

jq --exit-status --slurpfile contract "$CONTRACT" '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-central-release" and
  .source_digest == $contract[0].source_digest and
  .deployment == {id: $contract[0].deployment.id, state: "PUBLISHED"} and
  .public_repository == {
    base_url: $contract[0].public_repository.base_url,
    repository_file_count: 60,
    repository_manifest_sha256: $contract[0].public_repository.repository_manifest_sha256,
    exact_manifest_match: true
  } and
  .consumer == {
    maven_resolution: "PASS",
    classifier: "linux-x86_64",
    jvm_verification: "PASS",
    native_loader_smoke: "PASS"
  } and
  .release_complete == true
' "$RESULT" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and
  .completed_slice == "publication-central-release" and
  (.completed_slices | index("publication-central-validation-deployment")) != null and
  (.completed_slices | index("publication-central-release")) != null and
  .publication_ready == true and
  .channels.maven.status == "PUBLISHED" and
  .channels.maven.deployment_id == "9205c170-5232-4817-980f-0ff92e581ee9" and
  .channels.maven.public_repository == "https://repo.maven.apache.org/maven2" and
  (.gates[] | select(.id == "central_public_release") |
    .status == "PASS" and
    .terminal_state == "PUBLISHED" and
    .repository_file_count == 60 and
    .repository_manifest_sha256 == "1ab7f682451ea5d3283b6cb29df20f23d71da80eca59569eea96645b3ae41a6d") and
  .active_blockers == [] and .next_slice == null
' "$READINESS" >/dev/null

if jq -e '.. | objects | keys[] | select(test("^(bearer_token|token_password|authorization|credential)$"; "i"))' \
    "$CONTRACT" "$READINESS" "$RESULT" >/dev/null; then
  printf 'Central public release evidence contains a forbidden credential field.\n' >&2
  exit 1
fi

printf 'Central public release passed: terminal deployment state, exact public repository, and public consumer are locked.\n'
