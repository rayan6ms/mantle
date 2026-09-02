#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
jvm_jar=""
native_artifact_root=""
output_root=""
bundle=""
result=""
gpg_homedir=""
gpg_key=""

usage() {
  printf 'Usage: %s --jvm-jar PATH --native-artifact-root PATH --output-root PATH --bundle ZIP --result JSON --gpg-homedir PATH --gpg-key KEY\n' "$0" >&2
}

while (( $# > 0 )); do
  case "$1" in
    --jvm-jar) (( $# >= 2 )) || { usage; exit 2; }; jvm_jar="$2"; shift 2 ;;
    --native-artifact-root) (( $# >= 2 )) || { usage; exit 2; }; native_artifact_root="$2"; shift 2 ;;
    --output-root) (( $# >= 2 )) || { usage; exit 2; }; output_root="$2"; shift 2 ;;
    --bundle) (( $# >= 2 )) || { usage; exit 2; }; bundle="$2"; shift 2 ;;
    --result) (( $# >= 2 )) || { usage; exit 2; }; result="$2"; shift 2 ;;
    --gpg-homedir) (( $# >= 2 )) || { usage; exit 2; }; gpg_homedir="$2"; shift 2 ;;
    --gpg-key) (( $# >= 2 )) || { usage; exit 2; }; gpg_key="$2"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

[[ -n "$jvm_jar" && -n "$native_artifact_root" && -n "$output_root" && -n "$bundle" &&
   -n "$result" && -n "$gpg_homedir" && -n "$gpg_key" ]] || { usage; exit 2; }

for command in gpg jq md5sum sha1sum sha256sum sha512sum unzip xmllint zip; do
  command -v "$command" >/dev/null || {
    printf 'Central bundle staging requires %s\n' "$command" >&2
    exit 1
  }
done

readonly JVM_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly NATIVE_POM="$ROOT/compatibility/mantle-native-1.0.0.pom"
readonly CONTRACT="$ROOT/compatibility/publication-central-bundle.json"
readonly JVM_JAR="$jvm_jar"
readonly NATIVE_ARTIFACT_ROOT="$native_artifact_root"
readonly OUTPUT_ROOT="$output_root"
readonly BUNDLE="$bundle"
readonly RESULT="$result"
readonly GPG_HOMEDIR="$gpg_homedir"
readonly GPG_KEY="$gpg_key"

[[ ! -e "$OUTPUT_ROOT" ]] || {
  printf 'Refusing to overwrite Central repository staging: %s\n' "$OUTPUT_ROOT" >&2
  exit 1
}
[[ ! -e "$BUNDLE" ]] || {
  printf 'Refusing to overwrite Central deployment bundle: %s\n' "$BUNDLE" >&2
  exit 1
}
for source in "$JVM_JAR" "$JVM_POM" "$NATIVE_POM" "$CONTRACT"; do
  [[ -f "$source" ]] || { printf 'Central bundle input is missing: %s\n' "$source" >&2; exit 1; }
done
[[ -d "$GPG_HOMEDIR" ]] || { printf 'GPG home is missing: %s\n' "$GPG_HOMEDIR" >&2; exit 1; }
gpg --batch --homedir "$GPG_HOMEDIR" --list-secret-keys "$GPG_KEY" >/dev/null 2>&1 || {
  printf 'The selected Central signing key is not available in the provided GPG home.\n' >&2
  exit 1
}

mapfile -t classifiers < <(jq -r '.coordinates[] | select(.artifact_id == "mantle-native") | .classifiers[]' "$CONTRACT")
[[ "${#classifiers[@]}" == 5 ]] || { printf 'Central contract must declare five native classifiers.\n' >&2; exit 1; }
for classifier in "${classifiers[@]}"; do
  native="$NATIVE_ARTIFACT_ROOT/mantle-native-1.0.0-$classifier.jar"
  [[ -f "$native" ]] || { printf 'Native classifier input is missing: %s\n' "$native" >&2; exit 1; }
done

mkdir -p "$(dirname "$OUTPUT_ROOT")" "$(dirname "$BUNDLE")" "$(dirname "$RESULT")"
mkdir -p "$OUTPUT_ROOT/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
mkdir -p "$OUTPUT_ROOT/io/github/rayan6ms/mantle-native/1.0.0"
jvm_dir="$OUTPUT_ROOT/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
native_dir="$OUTPUT_ROOT/io/github/rayan6ms/mantle-native/1.0.0"
readonly jvm_dir native_dir

cp "$JVM_JAR" "$jvm_dir/mantle-lavaplayer-1.0.0.jar"
cp "$JVM_POM" "$jvm_dir/mantle-lavaplayer-1.0.0.pom"
cp "$NATIVE_POM" "$native_dir/mantle-native-1.0.0.pom"
for classifier in "${classifiers[@]}"; do
  cp "$NATIVE_ARTIFACT_ROOT/mantle-native-1.0.0-$classifier.jar" \
    "$native_dir/mantle-native-1.0.0-$classifier.jar"
done

placeholder_root="$(mktemp -d)"
readonly placeholder_root
trap 'rm -rf -- "$placeholder_root"' EXIT
printf '%s\n' \
  'Mantle emits its Lavaplayer-compatible JVM classes directly from Rust; there are no Java, Kotlin, or Kotlin script source files.' \
  'Project source and compatibility documentation: https://github.com/rayan6ms/mantle' \
  >"$placeholder_root/NOTICE.txt"
touch -t 202001010000 "$placeholder_root/NOTICE.txt"
(cd "$placeholder_root" && zip -q -X "$jvm_dir/mantle-lavaplayer-1.0.0-sources.jar" NOTICE.txt)
printf '%s\n' \
  'Mantle emits its Lavaplayer-compatible JVM classes directly from Rust, so conventional source-generated Javadoc is unavailable.' \
  'Public compatibility documentation: https://github.com/rayan6ms/mantle' \
  >"$placeholder_root/NOTICE.txt"
touch -t 202001010000 "$placeholder_root/NOTICE.txt"
(cd "$placeholder_root" && zip -q -X "$jvm_dir/mantle-lavaplayer-1.0.0-javadoc.jar" NOTICE.txt)

mapfile -t deployables < <(find "$OUTPUT_ROOT" -type f -printf '%p\n' | LC_ALL=C sort)
[[ "${#deployables[@]}" == 10 ]] || {
  printf 'Expected ten Central deployables before signatures/checksums, found %s.\n' "${#deployables[@]}" >&2
  exit 1
}

for file in "${deployables[@]}"; do
  md5sum "$file" | awk '{print $1}' >"$file.md5"
  sha1sum "$file" | awk '{print $1}' >"$file.sha1"
  sha256sum "$file" | awk '{print $1}' >"$file.sha256"
  sha512sum "$file" | awk '{print $1}' >"$file.sha512"
  gpg --batch --yes --homedir "$GPG_HOMEDIR" --local-user "$GPG_KEY" \
    --armor --detach-sign --output "$file.asc" "$file"
done

(cd "$OUTPUT_ROOT" && find . -type f -printf '%P\n' | LC_ALL=C sort | zip -q -X "$BUNDLE" -@)
bundle_sha256="$(sha256sum "$BUNDLE" | awk '{print $1}')"
signing_fingerprint="$(gpg --batch --homedir "$GPG_HOMEDIR" --with-colons --fingerprint "$GPG_KEY" |
  awk -F: '$1 == "fpr" {print $10; exit}')"
file_count="$(find "$OUTPUT_ROOT" -type f | wc -l | tr -d '[:space:]')"

jq -n \
  --arg bundle "$(basename "$BUNDLE")" \
  --arg bundle_sha256 "$bundle_sha256" \
  --arg signing_fingerprint "$signing_fingerprint" \
  --argjson file_count "$file_count" \
  '{
    schema_version: 1,
    status: "PASS",
    slice: "publication-central-bundle",
    publisher_api: "https://central.sonatype.com/api/v1/publisher/upload",
    publishing_type: "USER_MANAGED",
    network_upload_performed: false,
    deployable_count: 10,
    repository_file_count: $file_count,
    bundle: $bundle,
    bundle_sha256: $bundle_sha256,
    signing_fingerprint: $signing_fingerprint,
    checksums: ["md5", "sha1", "sha256", "sha512"],
    external_prerequisites_resolved: false
  }' >"$RESULT"

printf 'Staged a signed Central USER_MANAGED bundle with ten deployables and no network upload.\n'
