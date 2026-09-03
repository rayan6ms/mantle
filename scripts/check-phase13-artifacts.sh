#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly CONTRACT="$ROOT/compatibility/mantle-1.0-artifact-contract.json"
readonly INVENTORY="$ROOT/reference/lavaplayer-2.2.6-inventory.json"
readonly POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"

for command in cargo jar java jq sha256sum unzip; do
  command -v "$command" >/dev/null || {
    printf 'Phase 13 artifact gate requires %s\n' "$command" >&2
    exit 1
  }
done

PYTHON=""
for candidate in python3 python; do
  if command -v "$candidate" >/dev/null; then
    PYTHON="$candidate"
    break
  fi
done
if [[ -z "$PYTHON" ]]; then
  printf 'Phase 13 artifact gate requires Python 3\n' >&2
  exit 1
fi
readonly PYTHON

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

actual_resources="$(unzip -Z1 "$JAR" | awk '!/\.class$/ { print }' | tr -d '\r' | sort)"
expected_resources="$(jq -r '.resources[].path' "$CONTRACT" | tr -d '\r' | sort)"
if [[ "$actual_resources" != "$expected_resources" ]]; then
  diff -u <(printf '%s\n' "$expected_resources") <(printf '%s\n' "$actual_resources") || true
  printf 'Phase 13 emitted resource set differs from its contract\n' >&2
  exit 1
fi

while IFS=$'\t' read -r path expected_sha; do
  actual_sha="$(unzip -p "$JAR" "$path" | sha256sum | cut -d' ' -f1)"
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    printf 'Phase 13 resource hash mismatch for %s\n' "$path" >&2
    exit 1
  fi
done < <(jq -r '.resources[] | select(has("sha256")) | [.path, .sha256] | @tsv' "$CONTRACT" | tr -d '\r')

manifest="$(unzip -p "$JAR" META-INF/MANIFEST.MF | tr -d '\r')"
while IFS=$'\t' read -r key value; do
  grep -Fx "$key: $value" <<<"$manifest" >/dev/null || {
    printf 'Phase 13 manifest is missing %s: %s\n' "$key" "$value" >&2
    exit 1
  }
done < <(jq -r '.resources[] | select(.path == "META-INF/MANIFEST.MF") | .required_attributes | to_entries[] | [.key, .value] | @tsv' "$CONTRACT" | tr -d '\r')
if grep -F 'Enable-Native-Access:' <<<"$manifest" >/dev/null; then
  printf 'Library JAR must not claim executable-JAR native access\n' >&2
  exit 1
fi

"$PYTHON" - "$POM" "$CONTRACT" <<'PY'
import json
import sys
import xml.etree.ElementTree as ET

pom_path, contract_path = sys.argv[1:]
root = ET.parse(pom_path).getroot()
namespace = {"m": "http://maven.apache.org/POM/4.0.0"}


def required_text(parent, name):
    element = parent.find(f"m:{name}", namespace)
    if element is None or element.text is None:
        raise SystemExit(f"Published POM is missing {name}")
    return element.text.strip()


actual_identity = tuple(required_text(root, name) for name in ("groupId", "artifactId", "version"))
expected_identity = ("io.github.rayan6ms", "mantle-lavaplayer", "1.0.0")
if actual_identity != expected_identity:
    raise SystemExit(f"Unexpected published POM identity: {actual_identity!r}")

dependencies = root.findall("m:dependencies/m:dependency", namespace)
actual_dependencies = sorted(
    (
        ":".join(required_text(dependency, name) for name in ("groupId", "artifactId", "version")),
        required_text(dependency, "scope"),
    )
    for dependency in dependencies
)
with open(contract_path, encoding="utf-8") as contract_file:
    contract = json.load(contract_file)
expected_dependencies = sorted(
    (entry["coordinate"], entry["published_scope"])
    for entry in contract["reference_dependencies"]
    if entry["disposition"] != "replace-mantle-native"
)
if actual_dependencies != expected_dependencies:
    raise SystemExit(
        "Published POM dependency mapping differs from its contract:\n"
        f"expected={expected_dependencies!r}\nactual={actual_dependencies!r}"
    )
PY

jar --describe-module --file "$JAR" 2>&1 |
  grep -F 'io.github.rayan6ms.mantle.lavaplayer automatic' >/dev/null
if command -v jnativescan >/dev/null; then
  native_access="$(jnativescan --class-path "$JAR" --print-native-access | tr -d '\r')"
  [[ "$native_access" == "ALL-UNNAMED" ]] || {
    printf 'Unexpected native-access scan result: %s\n' "$native_access" >&2
    exit 1
  }
else
  java_version="$(java -XshowSettings:properties -version 2>&1 |
    awk -F'= ' '/java.specification.version/ { print $2; exit }' | tr -d '\r')"
  java_major="${java_version#1.}"
  if [[ ! "$java_major" =~ ^[0-9]+$ ]] || ((java_major >= 24)); then
    printf 'Phase 13 artifact gate requires jnativescan on JDK 24 or newer (detected %s)\n' "$java_version" >&2
    exit 1
  fi
  printf 'Phase 13 native-access scan deferred on JDK %s; JDK 25/26 matrix lanes enforce it.\n' "$java_version"
fi

printf 'Phase 13 artifact contract passed: 4 resources, 12 dependency decisions, 35 external public types, and Mantle coordinates.\n'
