#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CACHE_ROOT="${MANTLE_PHASE14_CONSUMER_CACHE:-$ROOT/.cache/phase14-consumers}"
readonly CONSUMER="$CACHE_ROOT/lavalink"
readonly WORK_ROOT="$ROOT/target/phase14/lavalink-source-compatibility"
readonly LOCAL_REPO="$WORK_ROOT/m2"
readonly LOG_ROOT="$WORK_ROOT/logs"
readonly NATIVE_CLASSIFIER="${MANTLE_PHASE14_NATIVE_CLASSIFIER:-linux-x86_64}"
readonly NATIVE_OUTPUT="$WORK_ROOT/native"
readonly REVISION="3d24006d1eed2bd9b4f5916298cf87ab34408b6f"
readonly EXPECTED_REVISION_DATE="2026-08-25T20:45:48+02:00"
readonly REPOSITORY="https://github.com/lavalink-devs/Lavalink"
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly MANTLE_JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly PUBLISHED_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly NATIVE_LIBRARY="${MANTLE_PHASE14_NATIVE_LIBRARY:-$ROOT/target/debug/libmantle_jvm.so}"
readonly OVERLAY_TEMPLATE="$ROOT/scripts/phase14-lavalink-mantle.init.gradle"
readonly OVERLAY="$WORK_ROOT/mantle.init.gradle"

for command in cargo git javac java jq jar sha256sum unzip; do
  command -v "$command" >/dev/null || { printf 'Phase 14 Lavalink gate requires %s\n' "$command" >&2; exit 1; }
done

if [[ ! -d "$CONSUMER/.git" ]]; then
  if [[ -e "$CONSUMER" ]]; then
    printf 'Lavalink cache path exists but is not a Git checkout: %s\n' "$CONSUMER" >&2
    exit 1
  fi
  mkdir -p "$CONSUMER"
  git -C "$CONSUMER" init --quiet
  git -C "$CONSUMER" remote add origin "$REPOSITORY"
  git -C "$CONSUMER" fetch --quiet --depth=1 origin "$REVISION"
  git -C "$CONSUMER" checkout --quiet --detach "$REVISION"
fi
[[ -x "$CONSUMER/gradlew" ]] || { printf 'Pinned Lavalink checkout missing: %s\n' "$CONSUMER" >&2; exit 1; }
[[ -f "$REFERENCE_JAR" ]] || { printf 'Reference Lavaplayer JAR missing: %s\n' "$REFERENCE_JAR" >&2; exit 1; }

actual_origin="$(git -C "$CONSUMER" remote get-url origin 2>/dev/null || true)"
[[ "${actual_origin%.git}" == "${REPOSITORY%.git}" ]] || {
  printf 'Lavalink cache origin mismatch: expected %s, got %s\n' "$REPOSITORY" "$actual_origin" >&2
  exit 1
}
[[ "$(git -C "$CONSUMER" rev-parse HEAD)" == "$REVISION" ]] || {
  printf 'Lavalink cache revision mismatch\n' >&2
  exit 1
}
[[ "$(git -C "$CONSUMER" show -s --format='%cI' HEAD)" == "$EXPECTED_REVISION_DATE" ]] || {
  printf 'Lavalink cache revision date mismatch\n' >&2
  exit 1
}
[[ -z "$(git -C "$CONSUMER" status --porcelain)" ]] || {
  printf 'Pinned Lavalink checkout is dirty; refusing to test edited upstream source\n' >&2
  exit 1
}

env -u APPIMAGE -u APPDIR cargo build --locked -q -p mantle-jvm
if [[ ! -f "$MANTLE_JAR" ]]; then
  mkdir -p "$(dirname "$MANTLE_JAR")"
  env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
    --reference-jar "$REFERENCE_JAR" --output "$MANTLE_JAR" --expected-abi 1
fi
MANTLE_ARTIFACT_JAR="$MANTLE_JAR" "$ROOT/scripts/check-phase13-artifacts.sh" >/dev/null

rm -rf "$WORK_ROOT"
mkdir -p "$LOCAL_REPO" "$LOG_ROOT" "$NATIVE_OUTPUT"

artifact_dir="$LOCAL_REPO/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
mkdir -p "$artifact_dir"
cp "$MANTLE_JAR" "$artifact_dir/mantle-lavaplayer-1.0.0.jar"
cp "$PUBLISHED_POM" "$artifact_dir/mantle-lavaplayer-1.0.0.pom"

case "$NATIVE_CLASSIFIER" in
  linux-x86_64) [[ "$(basename "$NATIVE_LIBRARY")" == "libmantle_jvm.so" ]] || { printf 'Linux native library must be libmantle_jvm.so\n' >&2; exit 1; } ;;
  *) printf 'Unsupported local native classifier: %s\n' "$NATIVE_CLASSIFIER" >&2; exit 1 ;;
esac
[[ -f "$NATIVE_LIBRARY" ]] || { printf 'Mantle native library missing: %s\n' "$NATIVE_LIBRARY" >&2; exit 1; }
native_stage="$WORK_ROOT/native-stage/native"
mkdir -p "$native_stage"
cp "$NATIVE_LIBRARY" "$native_stage/$(basename "$NATIVE_LIBRARY")"
native_jar="$WORK_ROOT/mantle-native-1.0.0-$NATIVE_CLASSIFIER.jar"
jar --create --file "$native_jar" -C "$WORK_ROOT/native-stage" native
native_dir="$LOCAL_REPO/io/github/rayan6ms/mantle-native/1.0.0"
mkdir -p "$native_dir"
cp "$native_jar" "$native_dir/"
cp "$ROOT/compatibility/mantle-native-1.0.0.pom" "$native_dir/mantle-native-1.0.0.pom"

sed -e "s|__LOCAL_REPO__|$LOCAL_REPO|g" \
    -e "s|__NATIVE_CLASSIFIER__|$NATIVE_CLASSIFIER|g" \
    -e "s|__NATIVE_OUTPUT__|$NATIVE_OUTPUT|g" \
    "$OVERLAY_TEMPLATE" > "$OVERLAY"

run_build() {
  local label="$1"
  shift
  set +e
  (cd "$CONSUMER" && ./gradlew "$@" --no-daemon --offline --stacktrace --init-script "$OVERLAY") \
    >"$LOG_ROOT/$label.log" 2>&1
  local status=$?
  set -e
  printf '%s' "$status" > "$LOG_ROOT/$label.exit"
  return "$status"
}

set +e
(cd "$CONSUMER" && ./gradlew :plugin-api:compileKotlin :Lavalink-Server:compileKotlin \
  --no-daemon --stacktrace) >"$LOG_ROOT/reference.log" 2>&1
reference_status=$?
set -e
printf '%s' "$reference_status" > "$LOG_ROOT/reference.exit"
[[ "$reference_status" == 0 ]] || {
  printf 'Lavalink reference compile failed; see %s\n' "$LOG_ROOT/reference.log" >&2
  exit 1
}

run_build mantle :plugin-api:compileKotlin :Lavalink-Server:compileKotlin :Lavalink-Server:verifyMantleNative || {
  printf 'Lavalink Mantle compile failed; see %s\n' "$LOG_ROOT/mantle.log" >&2
  exit 1
}

grep -F 'BUILD SUCCESSFUL' "$LOG_ROOT/reference.log" "$LOG_ROOT/mantle.log" >/dev/null
[[ "$(cat "$LOG_ROOT/reference.exit")" == 0 && "$(cat "$LOG_ROOT/mantle.exit")" == 0 ]] || exit 1
[[ -f "$NATIVE_OUTPUT/mantle-native.path" ]] || { printf 'Native path record missing\n' >&2; exit 1; }
native_path="$(<"$NATIVE_OUTPUT/mantle-native.path")"
[[ "$native_path" = /* && -f "$native_path" ]] || { printf 'Native path is not verified: %s\n' "$native_path" >&2; exit 1; }
[[ "$(jar --list --file "$native_jar" | tr -d '\r')" == $'META-INF/\nMETA-INF/MANIFEST.MF\nnative/\nnative/libmantle_jvm.so' ]] || {
  printf 'Selected native classifier has an unexpected JAR layout\n' >&2
  exit 1
}
smoke_root="$WORK_ROOT/native-loader-smoke"
mkdir -p "$smoke_root/classes"
cp "$ROOT/scripts/phase14-native-loader-smoke.java.txt" "$smoke_root/NativeLoaderSmoke.java"
javac -cp "$MANTLE_JAR" -d "$smoke_root/classes" "$smoke_root/NativeLoaderSmoke.java"
java --enable-native-access=ALL-UNNAMED -cp "$MANTLE_JAR:$smoke_root/classes" NativeLoaderSmoke "$native_path"

for forbidden in lavaplayer-natives lavaplayerNativesJar; do
  if grep -F "$forbidden" "$LOG_ROOT/mantle.log" >/dev/null; then
    printf 'Mantle overlay log contains forbidden legacy native integration: %s\n' "$forbidden" >&2
    exit 1
  fi
done

jq -n \
  --arg revision "$REVISION" \
  --arg reference_log "$LOG_ROOT/reference.log" \
  --arg mantle_log "$LOG_ROOT/mantle.log" \
  --arg jar "$artifact_dir/mantle-lavaplayer-1.0.0.jar" \
  --arg pom "$artifact_dir/mantle-lavaplayer-1.0.0.pom" \
  --arg native_coordinate "io.github.rayan6ms:mantle-native:1.0.0:$NATIVE_CLASSIFIER" \
  --arg native_path "$native_path" \
  --arg overlay "$OVERLAY" \
  '{schema_version: 1, status: "PASS", consumer: "lavalink", revision: $revision,
    reference_compile: {status: 0, log: $reference_log},
    mantle_compile: {status: 0, log: $mantle_log, failure_diff: [], task_outcome_match: true},
    artifact: {coordinate: "io.github.rayan6ms:mantle-lavaplayer:1.0.0", jar: $jar, pom: $pom},
    native: {coordinate: $native_coordinate, path: $native_path, loader_smoke: "PASS"}, overlay: $overlay,
    source_unchanged: true, legacy_filename_packaging_disabled: true}' \
  > "$WORK_ROOT/result.json"

jq -e '.status == "PASS" and .source_unchanged == true and .legacy_filename_packaging_disabled == true and .reference_compile.status == 0 and .mantle_compile.status == 0' "$WORK_ROOT/result.json" >/dev/null
printf 'Phase 14 Lavalink source compatibility passed: unchanged reference and Mantle compiles succeeded, plugin API linkage held, and the %s native classifier resolved to an absolute verified path.\n' "$NATIVE_CLASSIFIER"
