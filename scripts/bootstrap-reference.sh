#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly CACHE="$ROOT/.cache"
readonly REFERENCE="$CACHE/reference/lavaplayer-2.2.6"
readonly DOWNLOADS="$CACHE/downloads"
readonly TOOLCHAINS="$CACHE/toolchains"
readonly LOCK="$ROOT/reference/lavaplayer-2.2.6.lock.json"
readonly MAVEN_VERSION="3.9.11"
readonly JDK_ARCHIVE="OpenJDK21U-jdk_x64_linux_hotspot_21.0.12_8.tar.gz"
readonly JDK_SHA256="e4446ff06a276155697597cc0f1b15da004ff083f4964a35271ecee567177370"
readonly MAVEN_SHA512="bcfe4fe305c962ace56ac7b5fc7a08b87d5abd8b7e89027ab251069faebee516b0ded8961445d6d91ec1985dfe30f8153268843c89aa392733d1a3ec956c9978"
readonly CENTRAL="https://repo1.maven.org/maven2/dev/arbjerg/lavaplayer/2.2.6"
readonly SOURCE_COMMIT="$(jq --raw-output '.release_source.commit' "$LOCK")"
readonly SOURCE_ARCHIVE="lavaplayer-$SOURCE_COMMIT.tar.gz"

verify_sha256() {
  local file="$1"
  local expected="$2"
  printf '%s  %s\n' "$expected" "$file" | sha256sum --check
}

locked_artifact_hash() {
  local section="$1"
  local file="$2"
  jq --exit-status --raw-output \
    --arg section "$section" --arg file "$file" \
    '.[$section][] | select(.file == $file) | .sha256' "$LOCK"
}

mkdir -p "$REFERENCE" "$DOWNLOADS" "$TOOLCHAINS"

for artifact in \
  lavaplayer-2.2.6.jar \
  lavaplayer-2.2.6-sources.jar \
  lavaplayer-2.2.6.pom \
  lavaplayer-2.2.6.module
do
  curl --fail --location --silent --show-error \
    "$CENTRAL/$artifact" --output "$REFERENCE/$artifact"
done

for artifact in \
  lavaplayer-2.2.6.jar \
  lavaplayer-2.2.6-sources.jar \
  lavaplayer-2.2.6.pom \
  lavaplayer-2.2.6.module
do
  verify_sha256 "$REFERENCE/$artifact" "$(locked_artifact_hash published_artifacts "$artifact")"
done

curl --fail --location --silent --show-error \
  "$(jq --raw-output '.release_source.archive_url' "$LOCK")" \
  --output "$REFERENCE/$SOURCE_ARCHIVE"
verify_sha256 "$REFERENCE/$SOURCE_ARCHIVE" "$(jq --raw-output '.release_source.sha256' "$LOCK")"

if [[ ! -x "$TOOLCHAINS/jdk-21.0.12+8/bin/java" ]]; then
  curl --fail --location --silent --show-error \
    "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12%2B8/$JDK_ARCHIVE" \
    --output "$DOWNLOADS/$JDK_ARCHIVE"
  printf '%s  %s\n' "$JDK_SHA256" "$DOWNLOADS/$JDK_ARCHIVE" | sha256sum --check
  tar -xzf "$DOWNLOADS/$JDK_ARCHIVE" -C "$TOOLCHAINS"
fi

if [[ ! -x "$TOOLCHAINS/apache-maven-$MAVEN_VERSION/bin/mvn" ]]; then
  readonly MAVEN_ARCHIVE="apache-maven-$MAVEN_VERSION-bin.tar.gz"
  curl --fail --location --silent --show-error \
    "https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/$MAVEN_VERSION/$MAVEN_ARCHIVE" \
    --output "$DOWNLOADS/$MAVEN_ARCHIVE"
  printf '%s  %s\n' "$MAVEN_SHA512" "$DOWNLOADS/$MAVEN_ARCHIVE" | sha512sum --check
  tar -xzf "$DOWNLOADS/$MAVEN_ARCHIVE" -C "$TOOLCHAINS"
fi

JAVA_HOME="$TOOLCHAINS/jdk-21.0.12+8" \
  "$TOOLCHAINS/apache-maven-$MAVEN_VERSION/bin/mvn" -q \
  -f "$REFERENCE/lavaplayer-2.2.6.pom" \
  dependency:copy-dependencies \
  -DincludeScope=runtime \
  -DoutputDirectory="$REFERENCE/dependencies"

readonly EXPECTED_RUNTIME_COUNT="$(jq '.resolved_runtime_artifacts | length' "$LOCK")"
readonly ACTUAL_RUNTIME_COUNT="$(find "$REFERENCE/dependencies" -maxdepth 1 -type f -name '*.jar' | wc -l)"
if [[ "$ACTUAL_RUNTIME_COUNT" -ne "$EXPECTED_RUNTIME_COUNT" ]]; then
  printf 'Expected %s resolved runtime artifacts, found %s.\n' \
    "$EXPECTED_RUNTIME_COUNT" "$ACTUAL_RUNTIME_COUNT" >&2
  exit 1
fi
while IFS=$'\t' read -r artifact expected; do
  verify_sha256 "$REFERENCE/dependencies/$artifact" "$expected"
done < <(jq --raw-output '.resolved_runtime_artifacts[] | [.file, .sha256] | @tsv' "$LOCK")

sha256sum \
  "$REFERENCE/lavaplayer-2.2.6.jar" \
  "$REFERENCE/lavaplayer-2.2.6-sources.jar" \
  "$REFERENCE/lavaplayer-2.2.6.pom" \
  "$REFERENCE/lavaplayer-2.2.6.module" \
  "$REFERENCE/$SOURCE_ARCHIVE" \
  "$REFERENCE"/dependencies/*.jar
