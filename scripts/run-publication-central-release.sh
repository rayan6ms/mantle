#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
result="$ROOT/target/publication-central-release/result.json"

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
readonly CONTRACT="$ROOT/compatibility/publication-central-release.json"
readonly VALIDATION="$ROOT/compatibility/publication-central-validation-deployment.json"
readonly SMOKE_TEMPLATE="$ROOT/scripts/central-validation-consumer-smoke.java.txt"
readonly MAVEN="${MAVEN:-mvn}"

for command in curl java javac jq sha256sum unzip; do
  command -v "$command" >/dev/null || {
    printf 'Central public release verification requires %s\n' "$command" >&2
    exit 1
  }
done
command -v "$MAVEN" >/dev/null || {
  printf 'Central public release verification requires Maven: %s\n' "$MAVEN" >&2
  exit 1
}
for input in "$CONTRACT" "$VALIDATION" "$SMOKE_TEMPLATE"; do
  [[ -f "$input" ]] || { printf 'Central public release input is missing: %s\n' "$input" >&2; exit 1; }
done

work_root="$(mktemp -d)"
readonly work_root
trap 'rm -rf -- "$work_root"' EXIT
mkdir -p "$work_root/repository" "$work_root/m2" "$work_root/classes"
base_url="$(jq -r '.public_repository.base_url' "$CONTRACT")"
readonly base_url

while IFS= read -r deployable; do
  for suffix in '' .asc .md5 .sha1 .sha256 .sha512; do
    relative_file="$deployable$suffix"
    destination="$work_root/repository/$relative_file"
    mkdir -p "$(dirname "$destination")"
    curl --silent --show-error --fail --location \
      --connect-timeout 10 --max-time 120 \
      --output "$destination" "$base_url/$relative_file"
  done
done < <(jq -r '.deployables[].path' "$VALIDATION")

while IFS=$'\t' read -r deployable expected_sha; do
  actual_sha="$(sha256sum "$work_root/repository/$deployable" | awk '{print $1}')"
  [[ "$actual_sha" == "$expected_sha" ]] || {
    printf 'Public Central deployable digest mismatch: %s\n' "$deployable" >&2
    exit 1
  }
done < <(jq -r '.deployables[] | [.path, .sha256] | @tsv' "$VALIDATION")

manifest="$work_root/repository.sha256"
(cd "$work_root/repository" &&
  find . -type f -printf '%P\0' | LC_ALL=C sort -z | xargs -0 sha256sum) >"$manifest"
[[ "$(wc -l <"$manifest")" == 60 ]]
manifest_sha="$(sha256sum "$manifest" | awk '{print $1}')"
readonly manifest_sha
[[ "$manifest_sha" == "$(jq -r '.public_repository.repository_manifest_sha256' "$CONTRACT")" ]] || {
  printf 'The public Central 60-file manifest differs from the signed release repository.\n' >&2
  exit 1
}

pom="$work_root/pom.xml"
cat >"$pom" <<'EOF'
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>io.github.rayan6ms.validation</groupId>
  <artifactId>public-central-consumer</artifactId>
  <version>1.0.0</version>
  <dependencies>
    <dependency>
      <groupId>io.github.rayan6ms</groupId>
      <artifactId>mantle-lavaplayer</artifactId>
      <version>1.0.0</version>
    </dependency>
    <dependency>
      <groupId>io.github.rayan6ms</groupId>
      <artifactId>mantle-native</artifactId>
      <version>1.0.0</version>
      <classifier>linux-x86_64</classifier>
      <type>jar</type>
    </dependency>
  </dependencies>
</project>
EOF

"$MAVEN" --batch-mode --quiet --no-transfer-progress --file "$pom" \
  -Dmaven.repo.local="$work_root/m2" \
  dependency:build-classpath -Dmdep.outputFile="$work_root/classpath.txt"
classpath="$(<"$work_root/classpath.txt")"
readonly classpath
mantle_jar="$(tr ':' '\n' <<<"$classpath" | awk '/\/mantle-lavaplayer-1\.0\.0\.jar$/ {print; exit}')"
native_jar="$(tr ':' '\n' <<<"$classpath" | awk '/\/mantle-native-1\.0\.0-linux-x86_64\.jar$/ {print; exit}')"
[[ -f "$mantle_jar" && -f "$native_jar" ]] || {
  printf 'Public Maven resolution did not return both Mantle coordinates.\n' >&2
  exit 1
}
[[ "$(sha256sum "$mantle_jar" | awk '{print $1}')" == \
  "$(jq -r '.deployables[] | select(.path | endswith("mantle-lavaplayer-1.0.0.jar")) | .sha256' "$VALIDATION")" ]]
[[ "$(sha256sum "$native_jar" | awk '{print $1}')" == \
  "$(jq -r '.deployables[] | select(.path | endswith("mantle-native-1.0.0-linux-x86_64.jar")) | .sha256' "$VALIDATION")" ]]

unzip -q "$native_jar" native/libmantle_jvm.so -d "$work_root/native"
native_library="$work_root/native/native/libmantle_jvm.so"
readonly native_library
cp "$SMOKE_TEMPLATE" "$work_root/CentralValidationConsumerSmoke.java"
javac --release 11 -cp "$classpath" -d "$work_root/classes" \
  "$work_root/CentralValidationConsumerSmoke.java"
smoke_output="$(java --enable-native-access=ALL-UNNAMED -Xverify:all \
  -cp "$work_root/classes:$classpath" CentralValidationConsumerSmoke "$native_library")"
[[ "$smoke_output" == "central-validation-consumer-smoke-ok" ]]

mkdir -p "$(dirname "$RESULT")"
jq -n \
  --arg deployment_id "$(jq -r '.deployment.id' "$CONTRACT")" \
  --arg source_digest "$(jq -r '.source_digest' "$CONTRACT")" \
  --arg base_url "$base_url" \
  --arg manifest_sha "$manifest_sha" '
  {
    schema_version: 1,
    status: "PASS",
    slice: "publication-central-release",
    source_digest: $source_digest,
    deployment: {id: $deployment_id, state: "PUBLISHED"},
    public_repository: {
      base_url: $base_url,
      repository_file_count: 60,
      repository_manifest_sha256: $manifest_sha,
      exact_manifest_match: true
    },
    consumer: {
      maven_resolution: "PASS",
      classifier: "linux-x86_64",
      jvm_verification: "PASS",
      native_loader_smoke: "PASS"
    },
    release_complete: true
  }' >"$RESULT"

printf 'Central public release passed: all 60 public files match the signed repository and the public Maven/JVM/native consumer succeeded.\n'
