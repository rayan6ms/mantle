#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
plan="$ROOT/compatibility/phase15-dependency-audit.json"
result_root="$ROOT/target/phase15/dependency-audit"
while (( $# > 0 )); do
  case "$1" in
    --plan) (( $# >= 2 )) || exit 2; plan="$2"; shift 2 ;;
    --results-root) (( $# >= 2 )) || exit 2; result_root="$2"; shift 2 ;;
    *) printf 'Usage: %s [--plan PATH] [--results-root PATH]\n' "$0" >&2; exit 2 ;;
  esac
done
readonly PLAN="$plan"
readonly RESULT_ROOT="$result_root"
readonly HARDENING="$ROOT/compatibility/phase15-hardening-plan.json"
readonly SUPERSEDING_CLOSURE="$ROOT/compatibility/publication-cargo-vet-exemption-closure.json"

jq --exit-status '
  .schema_version == 1 and .status == "PASS" and .slice == "phase15-dependency-audit" and
  .plan == "compatibility/phase15-dependency-audit.json" and
  .toolchain.rust == "1.97.1" and .toolchain.cargo_audit == "0.22.2" and
  .toolchain.cargo_deny == "0.20.2" and .toolchain.cargo_vet == "0.10.2" and
  .advisory_database.advisories >= 1200 and
  (.advisory_database.commit | test("^[0-9a-f]{40}$")) and
  .lockfiles.workspace == {dependencies: 182, vulnerabilities: 0, warnings: 0} and
  .lockfiles.fuzz == {dependencies: 105, vulnerabilities: 0, warnings: 0} and
  .deny.status == "PASS" and .deny.duplicate_warnings == 2 and
  (.deny.accepted_duplicates | sort) == ["getrandom", "windows-sys"] and
  .deny.summary.advisories.errors == 0 and .deny.summary.bans.errors == 0 and
  .deny.summary.licenses.errors == 0 and .deny.summary.sources.errors == 0 and
  .vet.status == "PASS_WITH_EXEMPTIONS" and .vet.imports == 6 and
  .vet.fully_audited == 30 and .vet.exemptions == 140 and
  .vet.safe_to_deploy_exemptions == 120 and .vet.safe_to_run_exemptions == 20 and
  .release_constraints == ["D-001"] and .active_blockers == []
' "$RESULT_ROOT/result.json" >/dev/null

jq --exit-status '
  .schema_version == 1 and .status == "COMPLETE" and .slice == "phase15-dependency-audit" and
  .compatibility_baseline == "dev.arbjerg:lavaplayer:2.2.6" and
  .toolchain == {rust: "1.97.1", cargo_audit: "0.22.2", cargo_deny: "0.20.2", cargo_vet: "0.10.2"} and
  .bounds.lockfiles == 2 and .bounds.workspace_dependencies == 182 and
  .bounds.fuzz_dependencies == 105 and .bounds.audit_imports == 6 and
  .bounds.fully_audited_dependencies == 30 and .bounds.exact_version_exemptions == 140 and
  .bounds.safe_to_deploy_exemptions == 120 and .bounds.safe_to_run_exemptions == 20 and
  .bounds.accepted_duplicate_crates == 2 and
  .campaigns.workspace_advisories.status == "PASS" and .campaigns.workspace_advisories.vulnerabilities == 0 and
  .campaigns.fuzz_advisories.status == "PASS" and .campaigns.fuzz_advisories.vulnerabilities == 0 and
  .campaigns.deny.status == "PASS" and
  (.campaigns.deny.accepted_duplicates | sort) == ["getrandom", "windows-sys"] and
  .campaigns.vet.status == "PASS_WITH_EXEMPTIONS" and
  (.campaigns.vet.imports | sort) == ["bytecode-alliance", "embark-studios", "google", "isrg", "mozilla", "zcash"] and
  (.release_constraints|length) == 1 and .active_blockers == [] and
  .evidence.checker == "scripts/check-phase15-dependency-audit.sh" and
  .evidence.result == "target/phase15/dependency-audit/result.json" and
  .next_slice == "phase15-realtime-sanitizer"
' "$PLAN" >/dev/null

jq --exit-status '
  .schema_version == 1 and (.status == "IN_PROGRESS" or .status == "COMPLETE") and
  .phase == "phase15-hardening" and
  (.campaigns[] | select(.id == "dependency_audit") | .status) == "PASS"
' "$HARDENING" >/dev/null

jq --exit-status '.vulnerabilities.found == false and .vulnerabilities.count == 0 and .warnings == {} and .lockfile["dependency-count"] == 182' "$RESULT_ROOT/workspace-audit.json" >/dev/null
jq --exit-status '.vulnerabilities.found == false and .vulnerabilities.count == 0 and .warnings == {} and .lockfile["dependency-count"] == 105' "$RESULT_ROOT/fuzz-audit.json" >/dev/null
jq -cs --exit-status '
  ([.[] | select(.type == "diagnostic" and .fields.code == "duplicate") | .fields.message] | sort) ==
    ["found 2 duplicate entries for crate '\''getrandom'\''", "found 2 duplicate entries for crate '\''windows-sys'\''"] and
  ([.[] | select(.type == "summary")][0].fields |
    .advisories.errors == 0 and .bans.errors == 0 and .licenses.errors == 0 and .sources.errors == 0)
' "$RESULT_ROOT/cargo-deny.jsonl" >/dev/null
jq --exit-status '.conclusion == "success" and (.vetted_fully|length) == 30 and (.vetted_with_exemptions|length) == 140' "$RESULT_ROOT/cargo-vet.json" >/dev/null

[[ "$(rg -c '^\[imports\.' "$ROOT/supply-chain/config.toml")" == 6 ]]
jq --exit-status '
  .schema_version == 1 and .status == "PASS" and
  .slice == "publication-cargo-vet-exemption-closure" and
  .baseline == {
    imports: 6,
    fully_audited_packages: 30,
    exact_version_exemptions: 140,
    safe_to_deploy_exemptions: 120,
    safe_to_run_exemptions: 20
  } and
  .closure.remaining_exemptions == 0
' "$SUPERSEDING_CLOSURE" >/dev/null
[[ "$(awk '/^\[\[exemptions\./ {count += 1} END {print count + 0}' "$ROOT/supply-chain/config.toml")" == 0 ]]
(( $(rg -c '^\[\[audits\.' "$ROOT/supply-chain/audits.toml") >= 140 ))
(( $(rg -c 'criteria = "safe-to-deploy"' "$ROOT/supply-chain/audits.toml") >= 120 ))
(( $(rg -c 'criteria = "safe-to-run"' "$ROOT/supply-chain/audits.toml") >= 20 ))
test -s "$ROOT/supply-chain/imports.lock"

printf 'Phase 15 dependency audit passed: historical evidence and its superseding Vet closure are valid.\n'
