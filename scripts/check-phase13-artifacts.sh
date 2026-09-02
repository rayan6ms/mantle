#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly CONTRACT="$ROOT/compatibility/mantle-1.0-artifact-contract.json"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"

for command in cargo jar jnativescan jq sha256sum unzip xmllint; do
  command -v "$command" >/dev/null || {
    printf 'Phase 13 artifact gate requires %s\n' "$command" >&2
    exit 1
  }
done

if [[ ! -f "$JAR" ]]; then
  if [[ ! -f "$REFERENCE_JAR" ]]; then
    printf 'Phase 13 reference JAR not found: %s\n' "$REFERENCE_JAR" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$JAR")"
  env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
    --reference-jar "$REFERENCE_JAR" --output "$JAR" --expected-abi 1
fi

jq -e '
  .schema_version == 1 and
  .status == "COMPLETE" and
  .coordinate.group_id == "io.github.rayan6ms" and
  .coordinate.artifact_id == "mantle-lavaplayer" and
  .coordinate.version == "1.0.0" and
  .packaging.strategy == "thin-compatibility-jar-plus-platform-native-artifacts" and
  .packaging.bundled_or_extracted_native_libraries == false and
  (.resources | length) == 4 and
  (.reference_dependencies | length) == 12 and
  ([.reference_dependencies[] | select(.disposition == "replace-mantle-native")] | length) == 1 and
  ([.reference_dependencies[] | select(.disposition != "replace-mantle-native")] | length) == 11 and
  ([.external_public_type_groups[].expected_count] | add) == 35
' "$CONTRACT" >/dev/null

jq -e --slurpfile contract "$CONTRACT" '
  . as $inventory |
  $inventory.counts.non_class_resources == 4 and
  $inventory.counts.pom_dependencies == 12 and
  $inventory.counts.external_public_types == 35 and
  all($inventory.external_public_types[];
    . as $name |
    ([$contract[0].external_public_type_groups[] as $group |
      select(($group.selector == "exact" and $name == $group.value) or
             ($group.selector == "prefix" and ($name | startswith($group.value))))] | length) == 1) and
  all($contract[0].external_public_type_groups[];
    . as $group |
    ([$inventory.external_public_types[] |
      select(($group.selector == "exact" and . == $group.value) or
             ($group.selector == "prefix" and startswith($group.value)))] | length) == $group.expected_count)
' "$INVENTORY" >/dev/null

mapfile -t actual_resources < <(unzip -Z1 "$JAR" | awk '!/\.class$/ { print }' | sort)
mapfile -t expected_resources < <(jq -r '.resources[].path' "$CONTRACT" | sort)
if ! diff -u <(printf '%s\n' "${expected_resources[@]}") <(printf '%s\n' "${actual_resources[@]}"); then
  printf 'Phase 13 emitted resource set differs from its contract\n' >&2
  exit 1
fi

while IFS=$'\t' read -r path expected_sha; do
  actual_sha="$(unzip -p "$JAR" "$path" | sha256sum | cut -d' ' -f1)"
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    printf 'Phase 13 resource hash mismatch for %s\n' "$path" >&2
    exit 1
  fi
done < <(jq -r '.resources[] | select(has("sha256")) | [.path, .sha256] | @tsv' "$CONTRACT")

manifest="$(unzip -p "$JAR" META-INF/MANIFEST.MF | tr -d '\r')"
while IFS=$'\t' read -r key value; do
  grep -Fx "$key: $value" <<<"$manifest" >/dev/null || {
    printf 'Phase 13 manifest is missing %s: %s\n' "$key" "$value" >&2
    exit 1
  }
done < <(jq -r '.resources[] | select(.path == "META-INF/MANIFEST.MF") | .required_attributes | to_entries[] | [.key, .value] | @tsv' "$CONTRACT")
if grep -F 'Enable-Native-Access:' <<<"$manifest" >/dev/null; then
  printf 'Library JAR must not claim executable-JAR native access\n' >&2
  exit 1
fi

xmllint --noout "$POM"
[[ "$(xmllint --xpath 'string(/*[local-name()="project"]/*[local-name()="groupId"])' "$POM")" == "io.github.rayan6ms" ]]
[[ "$(xmllint --xpath 'string(/*[local-name()="project"]/*[local-name()="artifactId"])' "$POM")" == "mantle-lavaplayer" ]]
[[ "$(xmllint --xpath 'string(/*[local-name()="project"]/*[local-name()="version"])' "$POM")" == "1.0.0" ]]
[[ "$(xmllint --xpath 'count(/*[local-name()="project"]/*[local-name()="dependencies"]/*[local-name()="dependency"])' "$POM")" == "11" ]]
[[ "$(xmllint --xpath 'count(//*[local-name()="dependency"][*[local-name()="groupId"]="dev.arbjerg" and *[local-name()="artifactId"]="lavaplayer-natives"])' "$POM")" == "0" ]]

while IFS=$'\t' read -r coordinate scope; do
  IFS=: read -r group artifact version <<<"$coordinate"
  xpath="count(//*[local-name()='dependency'][*[local-name()='groupId']='$group' and *[local-name()='artifactId']='$artifact' and *[local-name()='version']='$version' and *[local-name()='scope']='$scope'])"
  [[ "$(xmllint --xpath "$xpath" "$POM")" == "1" ]] || {
    printf 'Published POM mapping missing for %s (%s)\n' "$coordinate" "$scope" >&2
    exit 1
  }
done < <(jq -r '.reference_dependencies[] | select(.disposition != "replace-mantle-native") | [.coordinate, .published_scope] | @tsv' "$CONTRACT")

jar --describe-module --file "$JAR" 2>&1 |
  grep -F 'io.github.rayan6ms.mantle.lavaplayer automatic' >/dev/null
native_access="$(jnativescan --class-path "$JAR" --print-native-access)"
[[ "$native_access" == "ALL-UNNAMED" ]] || {
  printf 'Unexpected native-access scan result: %s\n' "$native_access" >&2
  exit 1
}

printf 'Phase 13 artifact contract passed: 4 resources, 12 dependency decisions, 35 external public types, Mantle coordinates, and explicit native access.\n'
