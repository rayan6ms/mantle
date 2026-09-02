#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
store_path="$ROOT/supply-chain"
result="$ROOT/target/publication-cargo-vet-exemption-closure/result.json"
vet_result="$ROOT/target/publication-cargo-vet-exemption-closure/cargo-vet.json"

usage() {
  printf 'Usage: %s [--store-path PATH] [--result PATH] [--vet-result PATH]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --store-path) (( $# >= 2 )) || { usage; exit 2; }; store_path="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    --vet-result) (( $# >= 2 )) || { usage; exit 2; }; vet_result="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly STORE_PATH="$store_path"
readonly RESULT="$result"
readonly VET_RESULT="$vet_result"
readonly CONFIG="$STORE_PATH/config.toml"
readonly AUDITS="$STORE_PATH/audits.toml"
readonly IMPORTS_LOCK="$STORE_PATH/imports.lock"
readonly CONTRACT="$ROOT/compatibility/publication-cargo-vet-exemption-closure.json"
readonly READINESS="$ROOT/compatibility/publication-readiness.json"

for command in cargo jq sha256sum; do
  command -v "$command" >/dev/null || {
    printf 'Publication Cargo Vet checking requires %s\n' "$command" >&2
    exit 1
  }
done
for input in "$CONFIG" "$AUDITS" "$IMPORTS_LOCK" "$CONTRACT" "$READINESS" "$RESULT" "$VET_RESULT"; do
  [[ -f "$input" ]] || { printf 'Publication Cargo Vet check input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-cargo-vet-exemption-closure" and .version == "1.0.0" and
  .toolchain.cargo_vet == "0.10.2" and
  .baseline == {
    imports: 6, fully_audited_packages: 30, exact_version_exemptions: 140,
    safe_to_deploy_exemptions: 120, safe_to_run_exemptions: 20
  } and
  .closure.local_exact_version_audits == 141 and
  .closure.safe_to_deploy_audits == 121 and
  .closure.safe_to_run_audits == 20 and
  .closure.remaining_exemptions == 0 and
  .closure.fully_audited_packages == 171 and
  (.closure.canonical_tuple_sha256 | test("^[0-9a-f]{64}$")) and
  .review_scope.package_files == 7616 and .review_scope.package_lines == 4018147 and
  .review_scope.rust_files == 5318 and
  .review_scope.packages_with_unsafe_lexical_surface == 85 and
  .review_scope.packages_with_build_scripts == 35 and
  .review_scope.packages_with_native_source == 5 and
  .review_scope.procedural_macro_packages == 8 and
  (.review_scope.method | length) == 4 and
  .policy.publisher_trust_used_for_closure == false and
  .policy.additional_registry_imports_added == false and
  .policy.network_upload_performed == false and
  .hosted_evidence.status == "PASS" and
  .hosted_evidence.source_digest == "3b9864cf1a4ad792535f467b6c02984148621977" and
  .hosted_evidence.ci_run == 33664503665 and .hosted_evidence.ci_jobs_passed == 22 and
  .hosted_evidence.supply_chain_job == "PASS" and
  .hosted_evidence.native_matrix_run == 33664503912 and .hosted_evidence.native_jobs_passed == 6 and
  .next_slice == "publication-central-release-identity"
' "$CONTRACT" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and
  .completed_slice == "publication-central-release" and
  (.completed_slices | index("publication-cargo-vet-exemption-closure")) != null and
  (.completed_slices | index("publication-central-release-identity")) != null and
  (.completed_slices | index("publication-central-validation-deployment")) != null and
  (.completed_slices | index("publication-central-release")) != null and
  .publication_ready == true and
  (.gates[] | select(.id == "dependency_audits") |
    .status == "PASS" and .imports == 6 and .local_exact_version_audits == 141 and
    .fully_audited_packages == 171 and .remaining_exact_version_exemptions == 0 and
    .checker == "scripts/check-publication-cargo-vet-exemption-closure.sh" and
    .hosted_evidence == "https://github.com/rayan6ms/mantle/actions/runs/33664503665") and
  .active_blockers == [] and .next_slice == null
' "$READINESS" >/dev/null

tuples="$(mktemp)"
live_vet="$(mktemp)"
readonly tuples live_vet
trap 'rm -f -- "$tuples" "$live_vet"' EXIT

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

[[ "$(wc -l <"$tuples" | tr -d ' ')" == 141 ]]
[[ "$(awk -F '\t' '$3 == "safe-to-deploy" {count += 1} END {print count + 0}' "$tuples")" == 121 ]]
[[ "$(awk -F '\t' '$3 == "safe-to-run" {count += 1} END {print count + 0}' "$tuples")" == 20 ]]
[[ "$(sha256sum "$tuples" | awk '{print $1}')" == "$(jq -r '.closure.canonical_tuple_sha256' "$CONTRACT")" ]]
[[ "$(awk '/^\[imports\./ {count += 1} END {print count + 0}' "$CONFIG")" == 6 ]]
[[ "$(awk '/^\[\[exemptions\./ {count += 1} END {print count + 0}' "$CONFIG")" == 0 ]]
[[ "$(awk '/^who = "OpenAI Codex for Mantle <noreply@openai.com>"$/ {count += 1} END {print count + 0}' "$AUDITS")" == 141 ]]
if grep -Eq '^\[\[(wildcard-audits|trusted)' "$AUDITS"; then
  printf 'Publication closure may not use local wildcard or publisher-trust records.\n' >&2
  exit 1
fi
if grep -Eq '^delta = ' "$AUDITS"; then
  printf 'Publication closure expected exact-version full local audits.\n' >&2
  exit 1
fi

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-cargo-vet-exemption-closure" and
  .toolchain.cargo_vet == "0.10.2" and
  .audit_graph.imports == 6 and
  .audit_graph.local_exact_version_audits == 141 and
  .audit_graph.safe_to_deploy_audits == 121 and
  .audit_graph.safe_to_run_audits == 20 and
  .audit_graph.config_exemptions == 0 and
  .audit_graph.fully_audited_packages == 171 and
  .audit_graph.vet_exempted_packages == 0 and
  .audit_graph.canonical_tuple_sha256 == $tuple_sha and
  .release_constraints == [] and .network_upload_performed == false
' --arg tuple_sha "$(sha256sum "$tuples" | awk '{print $1}')" "$RESULT" >/dev/null

[[ "$(jq -r '.inputs.audits_sha256' "$RESULT")" == "$(sha256sum "$AUDITS" | awk '{print $1}')" ]]
[[ "$(jq -r '.inputs.config_sha256' "$RESULT")" == "$(sha256sum "$CONFIG" | awk '{print $1}')" ]]
[[ "$(jq -r '.inputs.imports_lock_sha256' "$RESULT")" == "$(sha256sum "$IMPORTS_LOCK" | awk '{print $1}')" ]]
[[ "$(jq -r '.inputs.cargo_lock_sha256' "$RESULT")" == "$(sha256sum "$ROOT/Cargo.lock" | awk '{print $1}')" ]]
jq --exit-status '.conclusion == "success" and (.vetted_fully | length) == 171 and (.vetted_with_exemptions | length) == 0' "$VET_RESULT" >/dev/null

unset APPIMAGE APPDIR
(
  cd "$ROOT"
  cargo vet --store-path "$STORE_PATH" --locked --output-format json --output-file "$live_vet" >/dev/null
)
jq --exit-status '.conclusion == "success" and (.vetted_fully | length) == 171 and (.vetted_with_exemptions | length) == 0' "$live_vet" >/dev/null

printf 'Publication Cargo Vet closure passed: 171 fully audited packages and zero exemptions.\n'
