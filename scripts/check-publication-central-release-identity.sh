#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
result="$ROOT/target/publication-central-release-identity/result.json"

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
readonly CONTRACT="$ROOT/compatibility/publication-central-release-identity.json"
readonly READINESS="$ROOT/compatibility/publication-readiness.json"
for command in jq; do
  command -v "$command" >/dev/null || {
    printf 'Central release identity checking requires %s\n' "$command" >&2
    exit 1
  }
done
for input in "$CONTRACT" "$READINESS" "$RESULT"; do
  [[ -f "$input" ]] || { printf 'Central release identity input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "OPERATOR_VERIFIED" and
  .slice == "publication-central-release-identity" and .version == "1.0.0" and
  .namespace == {
    group_id: "io.github.rayan6ms",
    portal_status: "VERIFIED",
    organization: "Rayan6ms",
    observed_at: "2026-09-02T02:05:33Z"
  } and
  .signing_key.primary_fingerprint == "B7CEFF5211EA68CB834AED549FA70609D2B9F145" and
  .signing_key.primary_signing_capability == true and
  .signing_key.signing_subkey == false and
  .signing_key.expires_at_epoch == 1862422660 and
  .signing_key.keyserver == "hkps://keyserver.ubuntu.com" and
  .signing_key.isolated_round_trip == "PASS" and
  .signing_key.secret_key_location == "OPERATOR_LOCAL_ONLY" and
  .portal_token.name == "Mantle-1-0-release-2026-09" and
  .portal_token.scope == "UNLIMITED" and
  .portal_token.expires_on == "2026-10-01" and
  .portal_token.github_actions_secret == "CENTRAL_PORTAL_TOKEN" and
  .portal_token.authentication_probe == "PASS" and
  .portal_token.authentication_probe_http_status == 404 and
  .portal_token.secret_material_retained_in_repository == false and
  .release_policy.publishing_type == "USER_MANAGED" and
  .release_policy.automatic_publication_allowed == false and
  .release_policy.validation_upload_performed == false and
  .release_policy.artifact_publication_performed == false and
  .release_policy.publish_requires_separate_explicit_action == true and
  .next_slice == "publication-central-validation-deployment"
' "$CONTRACT" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-central-release-identity" and
  .namespace == {group_id: "io.github.rayan6ms", portal_status: "VERIFIED"} and
  .signing_key == {
    primary_fingerprint: "B7CEFF5211EA68CB834AED549FA70609D2B9F145",
    keyserver: "hkps://keyserver.ubuntu.com",
    isolated_round_trip: "PASS"
  } and
  .portal_token == {
    github_actions_secret: "CENTRAL_PORTAL_TOKEN",
    authentication_probe: "PASS",
    authentication_probe_http_status: 404
  } and
  .release_policy == {
    publishing_type: "USER_MANAGED",
    network_upload_performed: false,
    artifact_publication_performed: false
  } and
  .next_slice == "publication-central-validation-deployment"
' "$RESULT" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "IN_PROGRESS" and
  .completed_slice == "publication-central-release-identity" and
  (.completed_slices | index("publication-central-release-identity")) != null and
  .publication_ready == false and
  (.gates[] | select(.id == "central_release_identity") |
    .status == "PASS" and
    .checker == "scripts/check-publication-central-release-identity.sh" and
    .workflow == ".github/workflows/central-release-identity.yml" and
    .publishing_type == "USER_MANAGED" and
    .network_upload_performed == false) and
  .active_blockers == ["central-validation-deployment"] and
  .next_slice == "publication-central-validation-deployment"
' "$READINESS" >/dev/null

for input in "$CONTRACT" "$READINESS" "$RESULT"; do
  if jq -e '.. | objects | keys[] | select(test("^(bearer_token|token_password|authorization|credential)$"; "i"))' \
      "$input" >/dev/null; then
    printf 'Central release identity evidence contains a forbidden credential field: %s\n' "$input" >&2
    exit 1
  fi
done

printf 'Central release identity passed: namespace, signing key, token authentication, and USER_MANAGED separation are locked.\n'
