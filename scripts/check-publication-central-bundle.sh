#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
repository_root=""
bundle=""
result=""
gpg_homedir=""

usage() {
  printf 'Usage: %s --repository-root PATH --bundle ZIP --result JSON --gpg-homedir PATH\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --repository-root) (( $# >= 2 )) || { usage; exit 2; }; repository_root="$2"; shift 2 ;;
    --bundle) (( $# >= 2 )) || { usage; exit 2; }; bundle="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    --gpg-homedir) (( $# >= 2 )) || { usage; exit 2; }; gpg_homedir="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$repository_root" && -n "$bundle" && -n "$result" && -n "$gpg_homedir" ]] || { usage; exit 2; }
readonly REPOSITORY_ROOT="$repository_root"
readonly BUNDLE="$bundle"
readonly RESULT="$result"
readonly GPG_HOMEDIR="$gpg_homedir"
readonly CONTRACT="$ROOT/compatibility/publication-central-bundle.json"

for command in diff gpg jq md5sum sha1sum sha256sum sha512sum unzip xmllint; do
  command -v "$command" >/dev/null || {
    printf 'Central bundle checking requires %s\n' "$command" >&2
    exit 1
  }
done
for input in "$REPOSITORY_ROOT" "$BUNDLE" "$RESULT" "$GPG_HOMEDIR" "$CONTRACT"; do
  [[ -e "$input" ]] || { printf 'Central bundle check input is missing: %s\n' "$input" >&2; exit 1; }
done

jq --exit-status '
  .schema_version == 1 and .status == "IMPLEMENTED" and
  .slice == "publication-central-bundle" and .version == "1.0.0" and
  .group_id == "io.github.rayan6ms" and
  .repository_url == "https://github.com/rayan6ms/mantle" and
  .publisher.api == "https://central.sonatype.com/api/v1/publisher/upload" and
  .publisher.publishing_type == "USER_MANAGED" and .publisher.network_upload == false and
  (.coordinates | length) == 2 and
  (.coordinates[] | select(.artifact_id == "mantle-lavaplayer") |
    .packaging == "jar" and .primary_jar == true and .required_companions == ["sources", "javadoc"]) and
  (.coordinates[] | select(.artifact_id == "mantle-native") |
    .packaging == "pom" and .primary_jar == false and (.classifiers | length) == 5) and
  .integrity.checksums == ["md5", "sha1", "sha256", "sha512"] and
  .integrity.detached_ascii_armored_pgp_for_every_deployable == true and
  (.external_prerequisites | length) == 3
' "$CONTRACT" >/dev/null

prefix="io/github/rayan6ms"
jvm="$prefix/mantle-lavaplayer/1.0.0/mantle-lavaplayer-1.0.0"
native="$prefix/mantle-native/1.0.0/mantle-native-1.0.0"
readonly prefix jvm native
deployables=(
  "$jvm.jar"
  "$jvm.pom"
  "$jvm-sources.jar"
  "$jvm-javadoc.jar"
  "$native.pom"
  "$native-linux-x86_64.jar"
  "$native-linux-aarch64.jar"
  "$native-macos-x86_64.jar"
  "$native-macos-aarch64.jar"
  "$native-windows-x86_64.jar"
)

expected_list="$(mktemp)"
actual_list="$(mktemp)"
bundle_root="$(mktemp -d)"
readonly expected_list actual_list bundle_root
trap 'rm -f -- "$expected_list" "$actual_list"; rm -rf -- "$bundle_root"' EXIT
for relative in "${deployables[@]}"; do
  printf '%s\n' "$relative" "$relative.asc" "$relative.md5" "$relative.sha1" \
    "$relative.sha256" "$relative.sha512"
done | LC_ALL=C sort >"$expected_list"
find "$REPOSITORY_ROOT" -type f -printf '%P\n' | LC_ALL=C sort >"$actual_list"
diff -u "$expected_list" "$actual_list" >/dev/null || {
  printf 'Central repository does not match the exact sixty-file contract.\n' >&2
  exit 1
}

for relative in "${deployables[@]}"; do
  file="$REPOSITORY_ROOT/$relative"
  [[ "$(md5sum "$file" | awk '{print $1}')" == "$(tr -d '[:space:]' <"$file.md5")" ]]
  [[ "$(sha1sum "$file" | awk '{print $1}')" == "$(tr -d '[:space:]' <"$file.sha1")" ]]
  [[ "$(sha256sum "$file" | awk '{print $1}')" == "$(tr -d '[:space:]' <"$file.sha256")" ]]
  [[ "$(sha512sum "$file" | awk '{print $1}')" == "$(tr -d '[:space:]' <"$file.sha512")" ]]
  gpg --batch --quiet --homedir "$GPG_HOMEDIR" --verify "$file.asc" "$file"
done

for companion in "$REPOSITORY_ROOT/$jvm-sources.jar" "$REPOSITORY_ROOT/$jvm-javadoc.jar"; do
  [[ "$(unzip -Z1 "$companion" | tr -d '\r')" == "NOTICE.txt" ]] || {
    printf 'Central placeholder archive has an unexpected layout: %s\n' "$companion" >&2
    exit 1
  }
  unzip -p "$companion" NOTICE.txt | grep -F 'directly from Rust' >/dev/null
  unzip -p "$companion" NOTICE.txt | grep -F 'https://github.com/rayan6ms/mantle' >/dev/null
  if unzip -Z1 "$companion" | grep -E --ignore-case '\.(java|kt|kts)$' >/dev/null; then
    printf 'Central placeholder archive contains a forbidden JVM source file: %s\n' "$companion" >&2
    exit 1
  fi
done

pom_value() {
  local pom="$1"
  local element="$2"
  xmllint --xpath "string(/*[local-name()='project']/*[local-name()='$element'])" "$pom"
}
check_common_pom() {
  local pom="$1"
  [[ "$(pom_value "$pom" groupId)" == "io.github.rayan6ms" ]]
  [[ "$(pom_value "$pom" version)" == "1.0.0" ]]
  [[ "$(pom_value "$pom" url)" == "https://github.com/rayan6ms/mantle" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='licenses']/*[local-name()='license']/*[local-name()='name'])" "$pom")" == "Apache License, Version 2.0" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='developers']/*[local-name()='developer']/*[local-name()='id'])" "$pom")" == "rayan6ms" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='developers']/*[local-name()='developer']/*[local-name()='url'])" "$pom")" == "https://github.com/rayan6ms" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='scm']/*[local-name()='connection'])" "$pom")" == "scm:git:https://github.com/rayan6ms/mantle.git" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='scm']/*[local-name()='developerConnection'])" "$pom")" == "scm:git:ssh://git@github.com/rayan6ms/mantle.git" ]]
  [[ "$(xmllint --xpath "string(/*[local-name()='project']/*[local-name()='scm']/*[local-name()='url'])" "$pom")" == "https://github.com/rayan6ms/mantle" ]]
}

jvm_pom="$REPOSITORY_ROOT/$jvm.pom"
native_pom="$REPOSITORY_ROOT/$native.pom"
xmllint --noout "$jvm_pom" "$native_pom"
check_common_pom "$jvm_pom"
check_common_pom "$native_pom"
[[ "$(pom_value "$jvm_pom" artifactId)" == "mantle-lavaplayer" ]]
[[ "$(pom_value "$jvm_pom" packaging)" == "jar" ]]
[[ -n "$(pom_value "$jvm_pom" name)" && -n "$(pom_value "$jvm_pom" description)" ]]
[[ "$(pom_value "$native_pom" artifactId)" == "mantle-native" ]]
[[ "$(pom_value "$native_pom" packaging)" == "pom" ]]
[[ -n "$(pom_value "$native_pom" name)" && -n "$(pom_value "$native_pom" description)" ]]

mapfile -t bundle_entries < <(unzip -Z1 "$BUNDLE" | tr -d '\r' | LC_ALL=C sort)
mapfile -t expected_entries <"$expected_list"
[[ "${bundle_entries[*]}" == "${expected_entries[*]}" ]] || {
  printf 'Central ZIP entries do not match repository staging.\n' >&2
  exit 1
}
unzip -q "$BUNDLE" -d "$bundle_root"
diff -qr "$REPOSITORY_ROOT" "$bundle_root" >/dev/null || {
  printf 'Central ZIP bytes do not match repository staging.\n' >&2
  exit 1
}

jq --exit-status \
  --arg bundle "$(basename "$BUNDLE")" \
  --arg bundle_sha256 "$(sha256sum "$BUNDLE" | awk '{print $1}')" '
    .schema_version == 1 and .status == "PASS" and
    .slice == "publication-central-bundle" and
    .publisher_api == "https://central.sonatype.com/api/v1/publisher/upload" and
    .publishing_type == "USER_MANAGED" and .network_upload_performed == false and
    .deployable_count == 10 and .repository_file_count == 60 and
    .bundle == $bundle and .bundle_sha256 == $bundle_sha256 and
    (.signing_fingerprint | test("^[0-9A-F]{40,64}$")) and
    .checksums == ["md5", "sha1", "sha256", "sha512"] and
    .external_prerequisites_resolved == false
  ' "$RESULT" >/dev/null

printf 'Central bundle passed: metadata, placeholders, five classifiers, checksums, signatures, and ZIP layout are valid; no upload occurred.\n'
