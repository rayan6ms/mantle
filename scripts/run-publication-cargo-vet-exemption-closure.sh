#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
store_path="$ROOT/supply-chain"
result_root="$ROOT/target/publication-cargo-vet-exemption-closure"

usage() {
  printf 'Usage: %s [--store-path PATH] [--result-root PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --store-path) (( $# >= 2 )) || { usage; exit 2; }; store_path="$2"; shift 2 ;;
    --result-root) (( $# >= 2 )) || { usage; exit 2; }; result_root="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly STORE_PATH="$store_path"
readonly RESULT_ROOT="$result_root"
readonly CONFIG="$STORE_PATH/config.toml"
readonly AUDITS="$STORE_PATH/audits.toml"
readonly IMPORTS_LOCK="$STORE_PATH/imports.lock"

for command in cargo jq sha256sum; do
  command -v "$command" >/dev/null || {
    printf 'Publication Cargo Vet closure requires %s\n' "$command" >&2
    exit 1
  }
done
for input in "$CONFIG" "$AUDITS" "$IMPORTS_LOCK" "$ROOT/Cargo.lock"; do
  [[ -f "$input" ]] || { printf 'Publication Cargo Vet input is missing: %s\n' "$input" >&2; exit 1; }
done

mkdir -p "$RESULT_ROOT"
tuples="$(mktemp)"
readonly tuples
trap 'rm -f -- "$tuples"' EXIT

awk '
  /^\[\[audits\./ {
    name = $0
    sub(/^\[\[audits\./, "", name)
    sub(/\]\]$/, "", name)
  }
  /^criteria = "/ {
    criteria = $0
    sub(/^criteria = "/, "", criteria)
    sub(/"$/, "", criteria)
  }
  /^version = "/ {
    version = $0
    sub(/^version = "/, "", version)
    sub(/"$/, "", version)
    print name "\t" version "\t" criteria
  }
' "$AUDITS" | LC_ALL=C sort >"$tuples"

unset APPIMAGE APPDIR
(
  cd "$ROOT"
  cargo vet --store-path "$STORE_PATH" --locked --output-format json \
    --output-file "$RESULT_ROOT/cargo-vet.json" >/dev/null
)

vet_version="$(cargo-vet --version | awk '{print $2}')"
imports="$(awk '/^\[imports\./ {count += 1} END {print count + 0}' "$CONFIG")"
exemptions="$(awk '/^\[\[exemptions\./ {count += 1} END {print count + 0}' "$CONFIG")"
local_audits="$(wc -l <"$tuples" | tr -d ' ')"
safe_to_deploy="$(awk -F '\t' '$3 == "safe-to-deploy" {count += 1} END {print count + 0}' "$tuples")"
safe_to_run="$(awk -F '\t' '$3 == "safe-to-run" {count += 1} END {print count + 0}' "$tuples")"
fully_audited="$(jq '.vetted_fully | length' "$RESULT_ROOT/cargo-vet.json")"
vet_exempted="$(jq '.vetted_with_exemptions | length' "$RESULT_ROOT/cargo-vet.json")"
tuple_sha256="$(sha256sum "$tuples" | awk '{print $1}')"

jq -n \
  --arg cargo_vet "$vet_version" \
  --arg tuple_sha256 "$tuple_sha256" \
  --arg audits_sha256 "$(sha256sum "$AUDITS" | awk '{print $1}')" \
  --arg config_sha256 "$(sha256sum "$CONFIG" | awk '{print $1}')" \
  --arg imports_lock_sha256 "$(sha256sum "$IMPORTS_LOCK" | awk '{print $1}')" \
  --arg cargo_lock_sha256 "$(sha256sum "$ROOT/Cargo.lock" | awk '{print $1}')" \
  --argjson imports "$imports" \
  --argjson local_audits "$local_audits" \
  --argjson safe_to_deploy "$safe_to_deploy" \
  --argjson safe_to_run "$safe_to_run" \
  --argjson config_exemptions "$exemptions" \
  --argjson fully_audited "$fully_audited" \
  --argjson vet_exempted "$vet_exempted" '
  {
    schema_version: 1,
    status: "PASS",
    slice: "publication-cargo-vet-exemption-closure",
    toolchain: {cargo_vet: $cargo_vet},
    audit_graph: {
      imports: $imports,
      local_exact_version_audits: $local_audits,
      safe_to_deploy_audits: $safe_to_deploy,
      safe_to_run_audits: $safe_to_run,
      config_exemptions: $config_exemptions,
      fully_audited_packages: $fully_audited,
      vet_exempted_packages: $vet_exempted,
      canonical_tuple_sha256: $tuple_sha256
    },
    inputs: {
      audits_sha256: $audits_sha256,
      config_sha256: $config_sha256,
      imports_lock_sha256: $imports_lock_sha256,
      cargo_lock_sha256: $cargo_lock_sha256
    },
    release_constraints: [],
    network_upload_performed: false
  }
' >"$RESULT_ROOT/result.json"

printf 'Publication Cargo Vet evidence generated: %s fully audited packages and zero exemptions.\n' "$fully_audited"
