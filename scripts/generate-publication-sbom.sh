#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
subject_root="$ROOT/target/publication-sbom-provenance/subjects"
output="$ROOT/target/publication-sbom-provenance/mantle-1.0.0.cdx.json"
result="$ROOT/target/publication-sbom-provenance/result.json"
source_date_epoch="${SOURCE_DATE_EPOCH:-$(git -C "$ROOT" show -s --format=%ct HEAD)}"

usage() {
  printf 'Usage: %s [--subject-root PATH] [--output PATH] [--result PATH] [--source-date-epoch SECONDS]\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --subject-root) (( $# >= 2 )) || { usage; exit 2; }; subject_root="$2"; shift 2 ;;
    --output) (( $# >= 2 )) || { usage; exit 2; }; output="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    --source-date-epoch) (( $# >= 2 )) || { usage; exit 2; }; source_date_epoch="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

readonly SUBJECT_ROOT="$subject_root"
readonly OUTPUT="$output"
readonly RESULT="$result"
readonly SOURCE_EPOCH="$source_date_epoch"
readonly CONTRACT="$ROOT/compatibility/publication-sbom-provenance.json"
readonly RUST_OUTPUT_PREFIX="publication-rust.cdx"

MAVEN_BIN="${MAVEN:-$(command -v mvn || true)}"
readonly MAVEN_BIN

for command in cargo date git jq sha256sum; do
  command -v "$command" >/dev/null || { printf 'Publication SBOM generation requires %s\n' "$command" >&2; exit 1; }
done
[[ -n "$MAVEN_BIN" && -x "$MAVEN_BIN" ]] || {
  printf 'Publication SBOM generation requires Maven; set MAVEN to its executable path.\n' >&2
  exit 1
}
[[ "$SOURCE_EPOCH" =~ ^[0-9]+$ ]] || { printf 'SOURCE_DATE_EPOCH must be an integer.\n' >&2; exit 2; }
[[ -d "$SUBJECT_ROOT" ]] || { printf 'Publication SBOM subject root is missing: %s\n' "$SUBJECT_ROOT" >&2; exit 1; }
[[ ! -e "$OUTPUT" ]] || { printf 'Refusing to overwrite publication SBOM: %s\n' "$OUTPUT" >&2; exit 1; }
[[ ! -e "$RESULT" ]] || { printf 'Refusing to overwrite publication SBOM result: %s\n' "$RESULT" >&2; exit 1; }

mapfile -t target_triples < <(jq -r '.subjects[] | select(.kind == "native") | .rust_target' "$CONTRACT")
readonly -a target_triples
[[ "${#target_triples[@]}" == 5 ]] || { printf 'Publication SBOM contract must declare five Rust targets.\n' >&2; exit 1; }

cyclonedx_version="$(cargo cyclonedx --version | awk '{print $2}')"
[[ "$cyclonedx_version" == "0.5.9" ]] || {
  printf 'Expected cargo-cyclonedx 0.5.9, found %s.\n' "$cyclonedx_version" >&2
  exit 1
}

mapfile -t manifest_dirs < <(
  env -u APPIMAGE -u APPDIR cargo metadata --manifest-path "$ROOT/Cargo.toml" --no-deps --format-version 1 |
    jq -r '.packages[].manifest_path | sub("/Cargo.toml$"; "")'
)
generated=()
for directory in "${manifest_dirs[@]}"; do
  for target in "${target_triples[@]}"; do
    generated+=("$directory/${RUST_OUTPUT_PREFIX}_${target}.cdx.json")
  done
done
for path in "${generated[@]}"; do
  [[ ! -e "$path" ]] || { printf 'Refusing to overwrite cargo-cyclonedx output: %s\n' "$path" >&2; exit 1; }
done

work="$(mktemp -d)"
cleanup() {
  rm -rf -- "$work"
  for path in "${generated[@]}"; do
    rm -f -- "$path"
  done
}
trap cleanup EXIT

rust_sboms=()
for target in "${target_triples[@]}"; do
  SOURCE_DATE_EPOCH="$SOURCE_EPOCH" cargo cyclonedx \
    --manifest-path "$ROOT/Cargo.toml" \
    --format json \
    --all-features \
    --target "$target" \
    --target-in-filename \
    --spec-version 1.5 \
    --override-filename "$RUST_OUTPUT_PREFIX" >/dev/null
  rust_sbom="$ROOT/crates/mantle-jvm/${RUST_OUTPUT_PREFIX}_${target}.cdx.json"
  [[ -f "$rust_sbom" ]] || { printf 'cargo-cyclonedx did not emit the %s mantle-jvm SBOM.\n' "$target" >&2; exit 1; }
  rust_sboms+=("$rust_sbom")
done
readonly -a rust_sboms
readonly RUST_SBOM="$work/publication-rust-merged.cdx.json"
jq -s '
  {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    version: 1,
    metadata: .[0].metadata,
    components: ([.[].components[]] | unique_by(.["bom-ref"]) | sort_by(.["bom-ref"])),
    dependencies: ([.[].dependencies[]] | group_by(.ref) | map({
      ref: .[0].ref,
      dependsOn: ([.[].dependsOn[]?] | unique | sort)
    }) | sort_by(.ref))
  }
' "${rust_sboms[@]}" >"$RUST_SBOM"

"$MAVEN_BIN" -q \
  -f "$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom" \
  org.cyclonedx:cyclonedx-maven-plugin:2.9.3:makeBom \
  -DoutputFormat=json \
  -DschemaVersion=1.5 \
  -DincludeBomSerialNumber=false \
  -DincludeTestScope=false \
  -DoutputDirectory="$work" \
  -DoutputName=publication-maven \
  -DskipAttach=true >/dev/null
readonly MAVEN_SBOM="$work/publication-maven.json"
[[ -f "$MAVEN_SBOM" ]] || { printf 'CycloneDX Maven plugin did not emit its SBOM.\n' >&2; exit 1; }

subjects_json="$work/subjects.json"
while IFS=$'\t' read -r filename kind classifier rust_target; do
  file="$SUBJECT_ROOT/$filename"
  [[ -f "$file" ]] || { printf 'Publication SBOM subject is missing: %s\n' "$file" >&2; exit 1; }
  jq -n \
    --arg file "$filename" \
    --arg kind "$kind" \
    --arg classifier "$classifier" \
    --arg rust_target "$rust_target" \
    --arg sha256 "$(sha256sum "$file" | awk '{print $1}')" \
    '{file: $file, kind: $kind, classifier: $classifier, rust_target: $rust_target, sha256: $sha256}'
done < <(jq -r '.subjects[] | [.file, .kind, (.classifier // ""), (.rust_target // "")] | @tsv' "$CONTRACT") | jq -s . >"$subjects_json"

timestamp="$(date -u --date="@$SOURCE_EPOCH" '+%Y-%m-%dT%H:%M:%SZ')"
serial_digest="$(jq -c 'map({file, sha256})' "$subjects_json" | sha256sum | awk '{print $1}')"
serial_number="urn:uuid:${serial_digest:0:8}-${serial_digest:8:4}-5${serial_digest:13:3}-8${serial_digest:17:3}-${serial_digest:20:12}"
mkdir -p "$(dirname "$OUTPUT")" "$(dirname "$RESULT")"
jq -n \
  --slurpfile rust "$RUST_SBOM" \
  --slurpfile maven "$MAVEN_SBOM" \
  --slurpfile subjects "$subjects_json" \
  --arg timestamp "$timestamp" \
  --arg serial_number "$serial_number" '
  ($rust[0]) as $r |
  ($maven[0]) as $m |
  ($subjects[0]) as $subjects |
  (reduce (($r.components + [$r.metadata.component])[] |
      select(.["bom-ref"] | startswith("path+file://"))) as $component ({};
      .[$component["bom-ref"]] = ("pkg:cargo/" + $component.name + "@1.0.0"))) as $local_refs |
  def normalized_ref:
    . as $ref | ($local_refs[$ref] // $ref);
  def normalized_rust_component:
    . as $component |
    if ($local_refs[$component["bom-ref"]] // null) != null then
      .["bom-ref"] = $local_refs[$component["bom-ref"]] |
      .version = "1.0.0" |
      .purl = $local_refs[$component["bom-ref"]] |
      del(.components)
    else . end;
  def artifact_component:
    . as $subject |
    {
      type: "file",
      "bom-ref": ("urn:mantle:artifact:" + $subject.file),
      name: $subject.file,
      version: "1.0.0",
      scope: "required",
      hashes: [{alg: "SHA-256", content: $subject.sha256}],
      properties: ([
        {name: "io.github.rayan6ms.mantle:subject-kind", value: $subject.kind}
      ] + if $subject.classifier == "" then [] else [{
        name: "io.github.rayan6ms.mantle:native-classifier", value: $subject.classifier
      }, {
        name: "io.github.rayan6ms.mantle:rust-target", value: $subject.rust_target
      }] end)
    };
  ($r.metadata.component | normalized_rust_component) as $rust_root |
  ($m.metadata.component) as $maven_root |
  ($subjects | map(artifact_component)) as $artifacts |
  (($r.components | map(normalized_rust_component)) + [$rust_root] +
    $m.components + [$maven_root] + $artifacts | sort_by(.["bom-ref"])) as $components |
  (($r.dependencies | map({
      ref: (.ref | normalized_ref),
      dependsOn: ((.dependsOn // []) | map(normalized_ref) | sort)
    })) + $m.dependencies +
    ($subjects | map({
      ref: ("urn:mantle:artifact:" + .file),
      dependsOn: [if .kind == "jvm" then $maven_root["bom-ref"] else $rust_root["bom-ref"] end]
    })) + [{
      ref: "urn:mantle:release:1.0.0",
      dependsOn: ($subjects | map("urn:mantle:artifact:" + .file) | sort)
    }] | sort_by(.ref)) as $dependencies |
  {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    serialNumber: $serial_number,
    version: 1,
    metadata: {
      timestamp: $timestamp,
      lifecycles: [{phase: "build"}],
      tools: {components: [
        {
          type: "application",
          author: "OWASP Foundation",
          group: "org.cyclonedx",
          name: "cargo-cyclonedx",
          version: "0.5.9"
        },
        $m.metadata.tools.components[0]
      ]},
      component: {
        type: "application",
        "bom-ref": "urn:mantle:release:1.0.0",
        group: "io.github.rayan6ms",
        name: "mantle-release",
        version: "1.0.0",
        licenses: [{license: {id: "Apache-2.0"}}],
        externalReferences: [{type: "vcs", url: "https://github.com/rayan6ms/mantle"}]
      },
      properties: [
        {name: "io.github.rayan6ms.mantle:compatibility-baseline", value: "dev.arbjerg:lavaplayer:2.2.6"},
        {name: "io.github.rayan6ms.mantle:subject-count", value: "6"}
      ]
    },
    components: $components,
    dependencies: $dependencies
  }' >"$OUTPUT"

component_count="$(jq '.components | length' "$OUTPUT")"
dependency_count="$(jq '.dependencies | length' "$OUTPUT")"
sbom_sha256="$(sha256sum "$OUTPUT" | awk '{print $1}')"
jq -n \
  --arg sbom "$(basename "$OUTPUT")" \
  --arg sbom_sha256 "$sbom_sha256" \
  --arg serial_number "$serial_number" \
  --argjson source_date_epoch "$SOURCE_EPOCH" \
  --argjson component_count "$component_count" \
  --argjson dependency_count "$dependency_count" \
  --slurpfile subjects "$subjects_json" '{
    schema_version: 1,
    status: "PASS",
    slice: "publication-sbom-provenance",
    sbom: $sbom,
    sbom_sha256: $sbom_sha256,
    serial_number: $serial_number,
    format: "CycloneDX JSON 1.5",
    source_date_epoch: $source_date_epoch,
    subject_count: ($subjects[0] | length),
    subjects: $subjects[0],
    component_count: $component_count,
    dependency_relationship_count: $dependency_count,
    generators: ["cargo-cyclonedx 0.5.9", "org.cyclonedx:cyclonedx-maven-plugin:2.9.3"],
    hosted_attestations: "PENDING"
  }' >"$RESULT"

printf 'Generated a deterministic CycloneDX SBOM for six binary release subjects.\n'
