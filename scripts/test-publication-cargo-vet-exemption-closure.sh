#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RUNNER="$ROOT/scripts/run-publication-cargo-vet-exemption-closure.sh"
readonly CHECKER="$ROOT/scripts/check-publication-cargo-vet-exemption-closure.sh"
test_root="$(mktemp -d)"
readonly test_root
trap 'rm -rf -- "$test_root"' EXIT

result_root="$test_root/result"
"$RUNNER" --result-root "$result_root" >/dev/null
"$CHECKER" --result "$result_root/result.json" --vet-result "$result_root/cargo-vet.json" >/dev/null

reintroduced="$test_root/reintroduced"
cp -a "$ROOT/supply-chain" "$reintroduced"
printf '\n[[exemptions.aes]]\nversion = "0.9.2"\ncriteria = "safe-to-deploy"\n' >>"$reintroduced/config.toml"
if "$CHECKER" --store-path "$reintroduced" --result "$result_root/result.json" \
    --vet-result "$result_root/cargo-vet.json" >/dev/null 2>&1; then
  printf 'Publication Cargo Vet checker accepted a reintroduced exemption.\n' >&2
  exit 1
fi

weakened="$test_root/weakened"
cp -a "$ROOT/supply-chain" "$weakened"
sed -i '0,/criteria = "safe-to-deploy"/s//criteria = "safe-to-run"/' "$weakened/audits.toml"
if "$CHECKER" --store-path "$weakened" --result "$result_root/result.json" \
    --vet-result "$result_root/cargo-vet.json" >/dev/null 2>&1; then
  printf 'Publication Cargo Vet checker accepted a weakened local audit.\n' >&2
  exit 1
fi

stale_result="$test_root/stale-result.json"
jq '.audit_graph.fully_audited_packages = 169' "$result_root/result.json" >"$stale_result"
if "$CHECKER" --result "$stale_result" --vet-result "$result_root/cargo-vet.json" >/dev/null 2>&1; then
  printf 'Publication Cargo Vet checker accepted stale result counts.\n' >&2
  exit 1
fi

printf 'Publication Cargo Vet success, exemption, criterion, and stale-result regressions passed.\n'
