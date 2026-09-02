#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
result="$ROOT/target/publication-central-validation-deployment/result.json"

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
readonly CONTRACT="$ROOT/compatibility/publication-central-validation-deployment.json"
for input in "$CONTRACT" "$RESULT"; do
  [[ -f "$input" ]] || { printf 'Central validation deployment input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "OPERATOR_VALIDATED" and
  .slice == "publication-central-validation-deployment" and .version == "1.0.0" and
  .source_digest == "62d02f6f0c6a312dc1ec5fec85213dc7e24cc4c1" and
  .deployment.id == "9205c170-5232-4817-980f-0ff92e581ee9" and
  .deployment.name == "Mantle 1.0.0 validation 62d02f6" and
  .deployment.publishing_type == "USER_MANAGED" and
  .deployment.portal_state == "VALIDATED" and
  .deployment.validated_components == 3 and .deployment.total_components == 3 and
  .deployment.download_base == "https://central.sonatype.com/api/v1/publisher/deployment/9205c170-5232-4817-980f-0ff92e581ee9/download" and
  .bundle.sha256 == "f677efdd40a8ae10f68e381f83c98d12ae3c43303065932f905f74de4ee43d1e" and
  .bundle.repository_file_count == 60 and
  .bundle.signing_fingerprint == "B7CEFF5211EA68CB834AED549FA70609D2B9F145" and
  (.deployables | length) == 10 and
  ([.deployables[].path] | length) == ([.deployables[].path] | unique | length) and
  all(.deployables[]; (.sha256 | test("^[0-9a-f]{64}$"))) and
  ([.deployables[].path] | map(select(endswith(".jar"))) | length) == 8 and
  ([.deployables[].path] | map(select(endswith(".pom"))) | length) == 2 and
  .consumer_gate.workflow == ".github/workflows/central-validation-deployment.yml" and
  .consumer_gate.runner == "scripts/run-publication-central-validation-deployment.sh" and
  .consumer_gate.checker == "scripts/check-publication-central-validation-deployment.sh" and
  .consumer_gate.github_actions_secret == "CENTRAL_PORTAL_TOKEN" and
  .consumer_gate.platform_classifier == "linux-x86_64" and
  .consumer_gate.maven_resolution == "REQUIRED" and
  .consumer_gate.native_loader_smoke == "REQUIRED" and
  .release_policy == {
    artifact_publication_performed: false,
    automatic_publication_allowed: false,
    publish_requires_separate_explicit_action: true
  } and
  (.hosted_consumer_evidence.status == "PENDING" or
    (.hosted_consumer_evidence.status == "PASS" and
     (.hosted_consumer_evidence.workflow_run | type) == "number" and
     (.hosted_consumer_evidence.result_sha256 | test("^[0-9a-f]{64}$")))) and
  .next_slice == "publication-explicit-release-decision"
' "$CONTRACT" >/dev/null

jq --exit-status --slurpfile contract "$CONTRACT" '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-central-validation-deployment" and
  .source_digest == $contract[0].source_digest and
  .deployment == {
    id: $contract[0].deployment.id,
    name: $contract[0].deployment.name,
    state: "VALIDATED",
    download_base: $contract[0].deployment.download_base
  } and
  .bundle_sha256 == $contract[0].bundle.sha256 and
  .deployables == {downloaded: 10, sha256_match: true} and
  .consumer == {
    maven_resolution: "PASS",
    classifier: "linux-x86_64",
    jvm_verification: "PASS",
    native_loader_smoke: "PASS"
  } and
  .release_policy == {
    artifact_publication_performed: false,
    publish_action_invoked: false
  } and
  .next_slice == "publication-explicit-release-decision"
' "$RESULT" >/dev/null

for input in "$CONTRACT" "$RESULT"; do
  if jq -e '.. | objects | keys[] | select(test("^(bearer_token|token_password|authorization|credential)$"; "i"))' \
      "$input" >/dev/null; then
    printf 'Central validation deployment evidence contains a forbidden credential field: %s\n' "$input" >&2
    exit 1
  fi
done

printf 'Central validation deployment passed: exact USER_MANAGED deployment, deployable digests, Maven resolution, JVM verification, and native loading are locked without publication.\n'
