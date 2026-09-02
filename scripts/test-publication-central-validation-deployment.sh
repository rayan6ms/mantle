#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-publication-central-validation-deployment.sh"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT
fixture="$test_root/result.json"

jq -n '{
  schema_version: 1,
  status: "PASS",
  slice: "publication-central-validation-deployment",
  source_digest: "62d02f6f0c6a312dc1ec5fec85213dc7e24cc4c1",
  deployment: {
    id: "9205c170-5232-4817-980f-0ff92e581ee9",
    name: "Mantle 1.0.0 validation 62d02f6",
    state: "VALIDATED",
    download_base: "https://central.sonatype.com/api/v1/publisher/deployment/9205c170-5232-4817-980f-0ff92e581ee9/download"
  },
  bundle_sha256: "f677efdd40a8ae10f68e381f83c98d12ae3c43303065932f905f74de4ee43d1e",
  deployables: {downloaded: 10, sha256_match: true},
  consumer: {
    maven_resolution: "PASS",
    classifier: "linux-x86_64",
    jvm_verification: "PASS",
    native_loader_smoke: "PASS"
  },
  release_policy: {artifact_publication_performed: false, publish_action_invoked: false},
  next_slice: "publication-explicit-release-decision"
}' >"$fixture"

"$CHECKER" --result "$fixture" >/dev/null

for mutation in \
  '.deployment.state = "PUBLISHED"' \
  '.deployment.id = "00000000-0000-0000-0000-000000000000"' \
  '.deployables.sha256_match = false' \
  '.consumer.maven_resolution = "FAIL"' \
  '.consumer.native_loader_smoke = "FAIL"' \
  '.release_policy.publish_action_invoked = true'; do
  candidate="$test_root/$(printf '%s' "$mutation" | sha256sum | cut -c1-12).json"
  jq "$mutation" "$fixture" >"$candidate"
  if "$CHECKER" --result "$candidate" >/dev/null 2>&1; then
    printf 'Central validation checker accepted forbidden mutation: %s\n' "$mutation" >&2
    exit 1
  fi
done

secret_leak="$test_root/secret-leak.json"
jq '.deployment.authorization = "forbidden-secret-material"' "$fixture" >"$secret_leak"
if "$CHECKER" --result "$secret_leak" >/dev/null 2>&1; then
  printf 'Central validation checker accepted retained token material.\n' >&2
  exit 1
fi

printf 'Central validation success, state, identity, digest, Maven, native, publication, and secret-leak regressions passed.\n'
