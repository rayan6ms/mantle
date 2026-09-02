#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CHECKER="$ROOT/scripts/check-publication-central-release.sh"

test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT
fixture="$test_root/result.json"

jq -n '{
  schema_version: 1,
  status: "PASS",
  slice: "publication-central-release",
  source_digest: "62d02f6f0c6a312dc1ec5fec85213dc7e24cc4c1",
  deployment: {id: "9205c170-5232-4817-980f-0ff92e581ee9", state: "PUBLISHED"},
  public_repository: {
    base_url: "https://repo.maven.apache.org/maven2",
    repository_file_count: 60,
    repository_manifest_sha256: "1ab7f682451ea5d3283b6cb29df20f23d71da80eca59569eea96645b3ae41a6d",
    exact_manifest_match: true
  },
  consumer: {
    maven_resolution: "PASS",
    classifier: "linux-x86_64",
    jvm_verification: "PASS",
    native_loader_smoke: "PASS"
  },
  release_complete: true
}' >"$fixture"

"$CHECKER" --result "$fixture" >/dev/null

for mutation in \
  '.deployment.state = "PUBLISHING"' \
  '.public_repository.repository_file_count = 59' \
  '.public_repository.exact_manifest_match = false' \
  '.consumer.maven_resolution = "FAIL"' \
  '.consumer.native_loader_smoke = "FAIL"' \
  '.release_complete = false'; do
  candidate="$test_root/$(printf '%s' "$mutation" | sha256sum | cut -c1-12).json"
  jq "$mutation" "$fixture" >"$candidate"
  if "$CHECKER" --result "$candidate" >/dev/null 2>&1; then
    printf 'Central public release checker accepted forbidden mutation: %s\n' "$mutation" >&2
    exit 1
  fi
done

secret_leak="$test_root/secret-leak.json"
jq '.deployment.credential = "forbidden-secret-material"' "$fixture" >"$secret_leak"
if "$CHECKER" --result "$secret_leak" >/dev/null 2>&1; then
  printf 'Central public release checker accepted retained credential material.\n' >&2
  exit 1
fi

printf 'Central release success, terminal-state, file-count, manifest, Maven, native, completion, and secret-leak regressions passed.\n'
