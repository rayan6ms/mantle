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
readonly STATUS_ENDPOINT='https://central.sonatype.com/api/v1/publisher/status?id=00000000-0000-0000-0000-000000000000'

for command in base64 curl gpg jq; do
  command -v "$command" >/dev/null || {
    printf 'Central release identity validation requires %s\n' "$command" >&2
    exit 1
  }
done
[[ -f "$CONTRACT" ]] || { printf 'Central release identity contract is missing.\n' >&2; exit 1; }
[[ -n "${CENTRAL_PORTAL_TOKEN:-}" ]] || {
  printf 'CENTRAL_PORTAL_TOKEN is required and must contain only the base64 Portal credential.\n' >&2
  exit 1
}

fingerprint="$(jq -r '.signing_key.primary_fingerprint' "$CONTRACT")"
keyserver="$(jq -r '.signing_key.keyserver' "$CONTRACT")"
namespace="$(jq -r '.namespace.group_id' "$CONTRACT")"
readonly fingerprint keyserver namespace

decoded_token="$(printf '%s' "$CENTRAL_PORTAL_TOKEN" | base64 --decode 2>/dev/null)" || {
  printf 'CENTRAL_PORTAL_TOKEN is not valid base64.\n' >&2
  exit 1
}
[[ "$decoded_token" == *:* ]] || {
  printf 'CENTRAL_PORTAL_TOKEN does not decode to the required Portal credential pair.\n' >&2
  exit 1
}
unset decoded_token

key_home="$(mktemp -d)"
readonly key_home
chmod 700 "$key_home"
trap 'rm -rf -- "$key_home"' EXIT
gpg --batch --quiet --homedir "$key_home" --keyserver "$keyserver" --recv-keys "$fingerprint"

remote_fingerprint="$(gpg --batch --homedir "$key_home" --with-colons --fingerprint 2>/dev/null |
  awk -F: '$1 == "fpr" {print $10; exit}')"
[[ "$remote_fingerprint" == "$fingerprint" ]] || {
  printf 'The distributed release key fingerprint does not match the contract.\n' >&2
  exit 1
}

primary_record="$(gpg --batch --homedir "$key_home" --with-colons --list-keys "$fingerprint" 2>/dev/null |
  awk -F: '$1 == "pub" {print $2 "\t" $7 "\t" $12; exit}')"
IFS=$'\t' read -r primary_validity primary_expiry primary_capabilities <<<"$primary_record"
[[ "$primary_validity" != r && "$primary_validity" != e && "$primary_capabilities" == *s* ]] || {
  printf 'The distributed release key is revoked, expired, or lacks primary signing capability.\n' >&2
  exit 1
}
now_epoch="$(date -u +%s)"
[[ "$primary_expiry" =~ ^[0-9]+$ && "$primary_expiry" -gt "$now_epoch" ]] || {
  printf 'The distributed release key has no future expiry.\n' >&2
  exit 1
}
if gpg --batch --homedir "$key_home" --with-colons --list-keys "$fingerprint" 2>/dev/null |
    awk -F: '$1 == "sub" && $12 ~ /s/ {found = 1} END {exit !found}'; then
  printf 'The release key unexpectedly contains a signing subkey.\n' >&2
  exit 1
fi

portal_http_status="$(
  printf 'header = "Authorization: Bearer %s"\n' "$CENTRAL_PORTAL_TOKEN" |
    curl --silent --show-error --output /dev/null --write-out '%{http_code}' --request POST \
      --config - --connect-timeout 10 --max-time 30 "$STATUS_ENDPOINT"
)"
[[ "$portal_http_status" == 404 ]] || {
  printf 'Central Portal authentication probe failed with HTTP %s.\n' "$portal_http_status" >&2
  exit 1
}

mkdir -p "$(dirname "$RESULT")"
jq -n \
  --arg namespace "$namespace" \
  --arg fingerprint "$fingerprint" \
  --arg keyserver "$keyserver" \
  --arg secret_name "$(jq -r '.portal_token.github_actions_secret' "$CONTRACT")" \
  --argjson portal_http_status "$portal_http_status" '
  {
    schema_version: 1,
    status: "PASS",
    slice: "publication-central-release-identity",
    namespace: {group_id: $namespace, portal_status: "VERIFIED"},
    signing_key: {
      primary_fingerprint: $fingerprint,
      keyserver: $keyserver,
      isolated_round_trip: "PASS"
    },
    portal_token: {
      github_actions_secret: $secret_name,
      authentication_probe: "PASS",
      authentication_probe_http_status: $portal_http_status
    },
    release_policy: {
      publishing_type: "USER_MANAGED",
      network_upload_performed: false,
      artifact_publication_performed: false
    },
    next_slice: "publication-central-validation-deployment"
  }' >"$RESULT"

printf 'Central release identity passed: verified namespace, distributed key, and Portal authentication; no upload occurred.\n'
