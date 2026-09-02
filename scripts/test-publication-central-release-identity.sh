#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-publication-central-release-identity.sh"
readonly CONTRACT="$ROOT/compatibility/publication-central-release-identity.json"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

fixture="$test_root/result.json"
jq -n '{
  schema_version: 1,
  status: "PASS",
  slice: "publication-central-release-identity",
  namespace: {group_id: "io.github.rayan6ms", portal_status: "VERIFIED"},
  signing_key: {
    primary_fingerprint: "B7CEFF5211EA68CB834AED549FA70609D2B9F145",
    keyserver: "hkps://keyserver.ubuntu.com",
    isolated_round_trip: "PASS"
  },
  portal_token: {
    github_actions_secret: "CENTRAL_PORTAL_TOKEN",
    authentication_probe: "PASS",
    authentication_probe_http_status: 404
  },
  release_policy: {
    publishing_type: "USER_MANAGED",
    network_upload_performed: false,
    artifact_publication_performed: false
  },
  next_slice: "publication-central-validation-deployment"
}' >"$fixture"

"$CHECKER" --result "$fixture" >/dev/null

unverified="$test_root/unverified.json"
jq '.namespace.portal_status = "PENDING"' "$fixture" >"$unverified"
if "$CHECKER" --result "$unverified" >/dev/null 2>&1; then
  printf 'Central identity checker accepted an unverified namespace.\n' >&2
  exit 1
fi

wrong_key="$test_root/wrong-key.json"
jq '.signing_key.primary_fingerprint = "0000000000000000000000000000000000000000"' \
  "$fixture" >"$wrong_key"
if "$CHECKER" --result "$wrong_key" >/dev/null 2>&1; then
  printf 'Central identity checker accepted a different signing key.\n' >&2
  exit 1
fi

automatic="$test_root/automatic.json"
jq '.release_policy.publishing_type = "AUTOMATIC"' "$fixture" >"$automatic"
if "$CHECKER" --result "$automatic" >/dev/null 2>&1; then
  printf 'Central identity checker accepted automatic publication.\n' >&2
  exit 1
fi

uploaded="$test_root/uploaded.json"
jq '.release_policy.network_upload_performed = true' "$fixture" >"$uploaded"
if "$CHECKER" --result "$uploaded" >/dev/null 2>&1; then
  printf 'Central identity checker accepted an identity run that uploaded artifacts.\n' >&2
  exit 1
fi

secret_leak="$test_root/secret-leak.json"
jq '.portal_token.bearer_token = "forbidden-secret-material"' "$fixture" >"$secret_leak"
if "$CHECKER" --result "$secret_leak" >/dev/null 2>&1; then
  printf 'Central identity checker accepted retained token material.\n' >&2
  exit 1
fi

if grep -Eiq '"(bearer_token|token_password|authorization)"[[:space:]]*:' "$CONTRACT"; then
  printf 'Central identity contract contains a forbidden credential field.\n' >&2
  exit 1
fi

printf 'Central identity success, namespace, key, publication-mode, upload, and secret-leak regressions passed.\n'
