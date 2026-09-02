#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RESULT_ROOT="${PHASE15_RESULTS_ROOT:-$ROOT/target/phase15/dependency-audit}"

unset APPIMAGE APPDIR
mkdir -p "$RESULT_ROOT"

cd "$ROOT"

cargo audit --deny warnings --json \
  >"$RESULT_ROOT/workspace-audit.json" 2>"$RESULT_ROOT/workspace-audit.log"
cargo audit --file fuzz/Cargo.lock --deny warnings --json \
  >"$RESULT_ROOT/fuzz-audit.json" 2>"$RESULT_ROOT/fuzz-audit.log"
cargo deny --format json check \
  >"$RESULT_ROOT/cargo-deny.log" 2>"$RESULT_ROOT/cargo-deny.jsonl"
cargo vet --locked --output-format json \
  >"$RESULT_ROOT/cargo-vet.json" 2>"$RESULT_ROOT/cargo-vet.log"

audit_version="$(cargo-audit --version | awk '{print $2}')"
deny_version="$(cargo-deny --version | awk '{print $2}')"
vet_version="$(cargo-vet --version | awk '{print $2}')"
rust_version="$(rustc --version | awk '{print $2}')"
workspace_dependencies="$(jq -r '.lockfile["dependency-count"]' "$RESULT_ROOT/workspace-audit.json")"
fuzz_dependencies="$(jq -r '.lockfile["dependency-count"]' "$RESULT_ROOT/fuzz-audit.json")"
advisory_count="$(jq -r '.database["advisory-count"]' "$RESULT_ROOT/workspace-audit.json")"
advisory_commit="$(jq -r '.database["last-commit"]' "$RESULT_ROOT/workspace-audit.json")"
deny_summary="$(jq -cs '[.[] | select(.type == "summary")][0].fields' "$RESULT_ROOT/cargo-deny.jsonl")"
duplicate_warnings="$(jq -cs '[.[] | select(.type == "diagnostic" and .fields.code == "duplicate")]|length' "$RESULT_ROOT/cargo-deny.jsonl")"
fully_audited="$(jq -r '.vetted_fully|length' "$RESULT_ROOT/cargo-vet.json")"
exempted="$(jq -r '.vetted_with_exemptions|length' "$RESULT_ROOT/cargo-vet.json")"
imports="$(rg -c '^\[imports\.' supply-chain/config.toml)"
safe_to_deploy="$(rg -c 'criteria = "safe-to-deploy"' supply-chain/config.toml)"
safe_to_run="$(rg -c 'criteria = "safe-to-run"' supply-chain/config.toml)"

jq -n \
  --arg rust "$rust_version" \
  --arg cargo_audit "$audit_version" \
  --arg cargo_deny "$deny_version" \
  --arg cargo_vet "$vet_version" \
  --arg advisory_commit "$advisory_commit" \
  --argjson advisory_count "$advisory_count" \
  --argjson workspace_dependencies "$workspace_dependencies" \
  --argjson fuzz_dependencies "$fuzz_dependencies" \
  --argjson deny_summary "$deny_summary" \
  --argjson duplicate_warnings "$duplicate_warnings" \
  --argjson fully_audited "$fully_audited" \
  --argjson exempted "$exempted" \
  --argjson imports "$imports" \
  --argjson safe_to_deploy "$safe_to_deploy" \
  --argjson safe_to_run "$safe_to_run" \
  '{
    schema_version: 1,
    status: "PASS",
    slice: "phase15-dependency-audit",
    plan: "compatibility/phase15-dependency-audit.json",
    toolchain: {rust: $rust, cargo_audit: $cargo_audit, cargo_deny: $cargo_deny, cargo_vet: $cargo_vet},
    advisory_database: {advisories: $advisory_count, commit: $advisory_commit},
    lockfiles: {
      workspace: {dependencies: $workspace_dependencies, vulnerabilities: 0, warnings: 0},
      fuzz: {dependencies: $fuzz_dependencies, vulnerabilities: 0, warnings: 0}
    },
    deny: {status: "PASS", summary: $deny_summary, duplicate_warnings: $duplicate_warnings,
      accepted_duplicates: ["getrandom", "windows-sys"]},
    vet: {status: "PASS_WITH_EXEMPTIONS", imports: $imports, fully_audited: $fully_audited,
      exemptions: $exempted, safe_to_deploy_exemptions: $safe_to_deploy,
      safe_to_run_exemptions: $safe_to_run},
    release_constraints: ["D-001"],
    active_blockers: []
  }' >"$RESULT_ROOT/result.json"

printf 'Phase 15 dependency audit passed with locked advisory, deny, and vet evidence.\n'
