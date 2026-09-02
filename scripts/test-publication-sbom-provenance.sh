#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly STAGER="$ROOT/scripts/stage-publication-attestation-subjects.sh"
readonly GENERATOR="$ROOT/scripts/generate-publication-sbom.sh"
readonly CHECKER="$ROOT/scripts/check-publication-sbom-provenance.sh"

for command in cargo cmp jq sha256sum; do
  command -v "$command" >/dev/null || {
    printf 'Publication SBOM regression test requires %s\n' "$command" >&2
    exit 1
  }
done

test_root="$(mktemp -d)"
readonly test_root
cleanup() {
  if [[ "${MANTLE_KEEP_TEST_OUTPUT:-0}" == 1 ]]; then
    printf 'Retained publication SBOM test output at %s\n' "$test_root" >&2
  else
    rm -rf -- "$test_root"
  fi
}
trap cleanup EXIT

mkdir -p "$test_root/native"
printf 'fixture JVM classes\n' >"$test_root/mantle-lavaplayer-1.0.0.jar"
while IFS= read -r filename; do
  printf 'fixture native binary for %s\n' "$filename" >"$test_root/native/$filename"
done < <(jq -r '.subjects[] | select(.kind == "native") | .file' \
  "$ROOT/compatibility/publication-sbom-provenance.json")

subjects="$test_root/subjects"
checksums="$test_root/subjects.sha256"
sbom="$test_root/mantle-1.0.0.cdx.json"
result="$test_root/result.json"
"$STAGER" \
  --jvm-jar "$test_root/mantle-lavaplayer-1.0.0.jar" \
  --native-artifact-root "$test_root/native" \
  --output-root "$subjects" \
  --checksums "$checksums" >/dev/null
"$GENERATOR" \
  --subject-root "$subjects" \
  --output "$sbom" \
  --result "$result" \
  --source-date-epoch 0 >/dev/null
"$CHECKER" --subject-root "$subjects" --checksums "$checksums" --sbom "$sbom" --result "$result" >/dev/null

repeated_sbom="$test_root/repeated.cdx.json"
repeated_result="$test_root/repeated-result.json"
"$GENERATOR" \
  --subject-root "$subjects" \
  --output "$repeated_sbom" \
  --result "$repeated_result" \
  --source-date-epoch 0 >/dev/null
cmp "$sbom" "$repeated_sbom" >/dev/null || {
  printf 'Publication SBOM generation is not reproducible for identical inputs.\n' >&2
  exit 1
}

tampered_subjects="$test_root/tampered-subjects"
cp -a "$subjects" "$tampered_subjects"
printf 'tamper\n' >>"$tampered_subjects/mantle-native-1.0.0-linux-x86_64.jar"
if "$CHECKER" --subject-root "$tampered_subjects" --checksums "$checksums" --sbom "$sbom" --result "$result" >/dev/null 2>&1; then
  printf 'Publication SBOM checker accepted a tampered binary subject.\n' >&2
  exit 1
fi

leaked_sbom="$test_root/leaked.cdx.json"
jq '.metadata.properties += [{name: "leak", value: "file:///home/private/mantle"}]' "$sbom" >"$leaked_sbom"
leaked_result="$test_root/leaked-result.json"
jq --arg sbom "$(basename "$leaked_sbom")" --arg sha "$(sha256sum "$leaked_sbom" | awk '{print $1}')" \
  '.sbom = $sbom | .sbom_sha256 = $sha' "$result" >"$leaked_result"
if "$CHECKER" --subject-root "$subjects" --checksums "$checksums" --sbom "$leaked_sbom" --result "$leaked_result" >/dev/null 2>&1; then
  printf 'Publication SBOM checker accepted a local filesystem path.\n' >&2
  exit 1
fi

missing_edge="$test_root/missing-edge.cdx.json"
jq '(.dependencies[] | select(.ref == "urn:mantle:release:1.0.0") | .dependsOn) |= .[1:]' \
  "$sbom" >"$missing_edge"
missing_result="$test_root/missing-result.json"
jq --arg sbom "$(basename "$missing_edge")" --arg sha "$(sha256sum "$missing_edge" | awk '{print $1}')" \
  '.sbom = $sbom | .sbom_sha256 = $sha' "$result" >"$missing_result"
if "$CHECKER" --subject-root "$subjects" --checksums "$checksums" --sbom "$missing_edge" --result "$missing_result" >/dev/null 2>&1; then
  printf 'Publication SBOM checker accepted an incomplete release dependency edge.\n' >&2
  exit 1
fi

missing_socket="$test_root/missing-socket.cdx.json"
jq 'del(.components[] | select(.purl == "pkg:cargo/socket2@0.6.5"))' \
  "$sbom" >"$missing_socket"
missing_socket_result="$test_root/missing-socket-result.json"
jq --arg sbom "$(basename "$missing_socket")" --arg sha "$(sha256sum "$missing_socket" | awk '{print $1}')" \
  '.sbom = $sbom | .sbom_sha256 = $sha | .component_count -= 1' "$result" >"$missing_socket_result"
if "$CHECKER" --subject-root "$subjects" --checksums "$checksums" --sbom "$missing_socket" --result "$missing_socket_result" >/dev/null 2>&1; then
  printf 'Publication SBOM checker accepted omission of the routed-socket production dependency.\n' >&2
  exit 1
fi

missing_serial="$test_root/missing-serial.cdx.json"
jq 'del(.serialNumber)' "$sbom" >"$missing_serial"
missing_serial_result="$test_root/missing-serial-result.json"
jq --arg sbom "$(basename "$missing_serial")" --arg sha "$(sha256sum "$missing_serial" | awk '{print $1}')" \
  '.sbom = $sbom | .sbom_sha256 = $sha' "$result" >"$missing_serial_result"
if "$CHECKER" --subject-root "$subjects" --checksums "$checksums" --sbom "$missing_serial" --result "$missing_serial_result" >/dev/null 2>&1; then
  printf 'Publication SBOM checker accepted a document without the GitHub-required serial number.\n' >&2
  exit 1
fi

printf 'Publication SBOM reproducibility, subject-tamper, path-leak, dependency-edge, and serial regressions passed.\n'
