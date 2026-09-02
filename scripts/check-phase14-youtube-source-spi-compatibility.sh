#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CACHE_ROOT="${MANTLE_PHASE14_CONSUMER_CACHE:-$ROOT/.cache/phase14-consumers}"
readonly CONSUMER="$CACHE_ROOT/youtube_source"
readonly WORK_ROOT="$ROOT/target/phase14/youtube-source-spi-compatibility"
readonly LOCAL_REPO="$WORK_ROOT/m2"
readonly LOG_ROOT="$WORK_ROOT/logs"
readonly REVISION="f45bbb7aebfcbc1c553769e04af6cd43afa8b7c3"
readonly EXPECTED_REVISION_DATE="2026-08-19T17:33:31+01:00"
readonly REPOSITORY="https://github.com/lavalink-devs/youtube-source"
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly REFERENCE_POM="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.pom"
readonly MANTLE_JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly MANTLE_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly NATIVE_LIBRARY="${MANTLE_PHASE14_NATIVE_LIBRARY:-$ROOT/target/debug/libmantle_jvm.so}"
readonly OVERLAY_TEMPLATE="$ROOT/scripts/phase14-youtube-source-mantle.init.gradle"
readonly SMOKE_TEMPLATE="$ROOT/scripts/phase14-youtube-source-spi-smoke.java.txt"

for command in cargo git java javac jq perl sha256sum tar; do
  command -v "$command" >/dev/null || { printf 'Phase 14 youtube-source gate requires %s\n' "$command" >&2; exit 1; }
done

"$ROOT/scripts/check-phase14-consumer-inventory.sh" --verify-upstream >/dev/null

if [[ ! -d "$CONSUMER/.git" ]]; then
  if [[ -e "$CONSUMER" ]]; then
    printf 'youtube-source cache path exists but is not a Git checkout: %s\n' "$CONSUMER" >&2
    exit 1
  fi
  mkdir -p "$CONSUMER"
  git -C "$CONSUMER" init --quiet
  git -C "$CONSUMER" remote add origin "$REPOSITORY"
  git -C "$CONSUMER" fetch --quiet --depth=1 origin "$REVISION"
fi
actual_origin="$(git -C "$CONSUMER" remote get-url origin 2>/dev/null || true)"
[[ "${actual_origin%.git}" == "${REPOSITORY%.git}" ]] || { printf 'youtube-source cache origin mismatch\n' >&2; exit 1; }
if [[ "$(git -C "$CONSUMER" rev-parse HEAD 2>/dev/null || true)" != "$REVISION" ]]; then
  git -C "$CONSUMER" checkout --quiet --detach "$REVISION"
fi
[[ "$(git -C "$CONSUMER" rev-parse HEAD)" == "$REVISION" ]] || { printf 'youtube-source cache revision mismatch\n' >&2; exit 1; }
[[ "$(git -C "$CONSUMER" show -s --format='%cI' HEAD)" == "$EXPECTED_REVISION_DATE" ]] || { printf 'youtube-source cache revision date mismatch\n' >&2; exit 1; }
[[ -z "$(git -C "$CONSUMER" status --porcelain)" ]] || { printf 'Pinned youtube-source checkout is dirty\n' >&2; exit 1; }
[[ -x "$CONSUMER/gradlew" ]] || { printf 'Pinned youtube-source checkout missing Gradle wrapper\n' >&2; exit 1; }
[[ -f "$REFERENCE_JAR" && -f "$REFERENCE_POM" ]] || { printf 'Frozen Lavaplayer reference artifact is missing\n' >&2; exit 1; }

# Gradle 8.10 does not support the workspace JDK 25; use the compatible JDK 21/17 toolchain.
if [[ -z "${MANTLE_PHASE14_JAVA_HOME:-}" ]]; then
  for candidate_java_home in /opt/adoptium/jdk-21.0.11+10 /opt/adoptium/jdk-17.0.19+10; do
    if [[ -x "$candidate_java_home/bin/java" ]]; then
      export JAVA_HOME="$candidate_java_home"
      export PATH="$JAVA_HOME/bin:$PATH"
      break
    fi
  done
else
  export JAVA_HOME="$MANTLE_PHASE14_JAVA_HOME"
  export PATH="$JAVA_HOME/bin:$PATH"
fi

env -u APPIMAGE -u APPDIR cargo build --locked -q -p mantle-jvm
if [[ ! -f "$MANTLE_JAR" ]]; then
  mkdir -p "$(dirname "$MANTLE_JAR")"
  env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
    --reference-jar "$REFERENCE_JAR" --output "$MANTLE_JAR" --expected-abi 1
fi
MANTLE_ARTIFACT_JAR="$MANTLE_JAR" "$ROOT/scripts/check-phase13-artifacts.sh" >/dev/null
[[ -f "$NATIVE_LIBRARY" ]] || { printf 'Mantle native library missing: %s\n' "$NATIVE_LIBRARY" >&2; exit 1; }

rm -rf "$WORK_ROOT"
mkdir -p "$LOCAL_REPO" "$LOG_ROOT" "$WORK_ROOT/reference" "$WORK_ROOT/candidate"

reference_m2="$LOCAL_REPO/dev/arbjerg/lavaplayer/2.2.6"
mkdir -p "$reference_m2"
cp "$REFERENCE_JAR" "$reference_m2/lavaplayer-2.2.6.jar"
cp "$REFERENCE_POM" "$reference_m2/lavaplayer-2.2.6.pom"
[[ "$(sha256sum "$reference_m2/lavaplayer-2.2.6.jar" | awk '{print $1}')" == \
   "84aba896d988e12ea24c25f87f2e88eca4be7adac31893eacabf93401da1282d" ]] || {
  printf 'Frozen Lavaplayer reference JAR hash mismatch\n' >&2
  exit 1
}
mantle_m2="$LOCAL_REPO/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
mkdir -p "$mantle_m2"
cp "$MANTLE_JAR" "$mantle_m2/mantle-lavaplayer-1.0.0.jar"
cp "$MANTLE_POM" "$mantle_m2/mantle-lavaplayer-1.0.0.pom"

prepare_project() {
  local project="$1"
  local mode="$2"
  git -C "$CONSUMER" archive "$REVISION" | tar -x -C "$project"
  cp -a "$CONSUMER/.git" "$project/.git"
  chmod +x "$project/gradlew"
  if [[ "$mode" == reference ]]; then
    perl -0pi -e 's#compileOnly\(libs\.lavaplayer\.v2\)#compileOnly("dev.arbjerg:lavaplayer:2.2.6")#; s#testImplementation\(libs\.lavaplayer\.v2\)#testImplementation("dev.arbjerg:lavaplayer:2.2.6")#' "$project/v2/build.gradle.kts"
  else
    perl -0pi -e 's#compileOnly\(libs\.lavaplayer\.v2\)#compileOnly("io.github.rayan6ms:mantle-lavaplayer:1.0.0")#; s#testImplementation\(libs\.lavaplayer\.v2\)#testImplementation("io.github.rayan6ms:mantle-lavaplayer:1.0.0")#' "$project/v2/build.gradle.kts"
  fi
  if [[ "$mode" == reference ]]; then
    grep -F 'compileOnly("dev.arbjerg:lavaplayer:2.2.6")' "$project/v2/build.gradle.kts" >/dev/null
  else
    grep -F 'compileOnly("io.github.rayan6ms:mantle-lavaplayer:1.0.0")' "$project/v2/build.gradle.kts" >/dev/null
  fi
}

prepare_project "$WORK_ROOT/reference" reference
prepare_project "$WORK_ROOT/candidate" mantle

run_gradle() {
  local label="$1"
  local project="$2"
  local overlay="$WORK_ROOT/$label.init.gradle"
  local classpath_output="$WORK_ROOT/$label.classpath"
  local runtime_classpath_output="$WORK_ROOT/$label.runtime.classpath"
  sed -e "s|__LOCAL_REPO__|$LOCAL_REPO|g" \
      -e "s|__CLASSPATH_OUTPUT__|$classpath_output|g" \
      -e "s|__RUNTIME_CLASSPATH_OUTPUT__|$runtime_classpath_output|g" \
      "$OVERLAY_TEMPLATE" > "$overlay"
  set +e
  (cd "$project" && ./gradlew :v2:compileJava :v2:phase14WriteCompileClasspath --no-daemon --stacktrace --init-script "$overlay") \
    >"$LOG_ROOT/$label.log" 2>&1
  local status=$?
  set -e
  printf '%s' "$status" > "$LOG_ROOT/$label.exit"
  return "$status"
}

run_gradle reference "$WORK_ROOT/reference" || { printf 'youtube-source reference compile failed; see %s\n' "$LOG_ROOT/reference.log" >&2; exit 1; }
run_gradle mantle "$WORK_ROOT/candidate" || { printf 'youtube-source Mantle compile failed; see %s\n' "$LOG_ROOT/mantle.log" >&2; exit 1; }

for log in reference mantle; do
  grep -F 'BUILD SUCCESSFUL' "$LOG_ROOT/$log.log" >/dev/null
  [[ "$(cat "$LOG_ROOT/$log.exit")" == 0 ]] || exit 1
  [[ -s "$WORK_ROOT/$log.classpath" ]] || { printf 'Compile classpath record missing: %s\n' "$log" >&2; exit 1; }
  [[ -s "$WORK_ROOT/$log.runtime.classpath" ]] || { printf 'Runtime classpath record missing: %s\n' "$log" >&2; exit 1; }
done
resolved_reference_jar="$(tr ':' '\n' < "$WORK_ROOT/reference.classpath" | awk '/\/lavaplayer-2\.2\.6\.jar$/ {print; exit}')"
[[ -f "$resolved_reference_jar" ]] || { printf 'Resolved reference Lavaplayer JAR missing\n' >&2; exit 1; }
[[ "$(sha256sum "$resolved_reference_jar" | awk '{print $1}')" == \
   "84aba896d988e12ea24c25f87f2e88eca4be7adac31893eacabf93401da1282d" ]] || {
  printf 'Resolved reference Lavaplayer JAR hash mismatch\n' >&2
  exit 1
}
resolved_mantle_jar="$(tr ':' '\n' < "$WORK_ROOT/mantle.classpath" | awk '/\/mantle-lavaplayer-1\.0\.0\.jar$/ {print; exit}')"
[[ "$resolved_mantle_jar" == "$mantle_m2/mantle-lavaplayer-1.0.0.jar" ]] || {
  printf 'Resolved candidate did not use the isolated Mantle artifact\n' >&2
  exit 1
}
if tr ':' '\n' < "$WORK_ROOT/mantle.classpath" | grep -E '/dev\.arbjerg/lavaplayer/' >/dev/null; then
  printf 'Candidate compile classpath retained the reference Lavaplayer artifact\n' >&2
  exit 1
fi

for source_hash in \
  'common/src/main/java/dev/lavalink/youtube/YoutubeAudioSourceManager.java 5be746b2aa9325b9acb8c6aef9d1fb4d46aeffbb6f5d67fb8fcef7c8409f6de8' \
  'common/src/main/java/dev/lavalink/youtube/track/YoutubeAudioTrack.java 06069af5415293a6060c3575305078f7e46e4710bc482a90507bd5cf19785086'; do
  source_path="${source_hash% *}"
  expected_hash="${source_hash##* }"
  for project in reference candidate; do
    [[ "$(sha256sum "$WORK_ROOT/$project/$source_path" | awk '{print $1}')" == "$expected_hash" ]] || {
      printf 'Pinned source changed in generated %s consumer: %s\n' "$project" "$source_path" >&2
      exit 1
    }
  done
done
while IFS= read -r source_path; do
  expected_hash="$(git -C "$CONSUMER" show "$REVISION:$source_path" | sha256sum | awk '{print $1}')"
  for project in reference candidate; do
    [[ "$(sha256sum "$WORK_ROOT/$project/$source_path" | awk '{print $1}')" == "$expected_hash" ]] || {
      printf 'Pinned Java source changed in generated %s consumer: %s\n' "$project" "$source_path" >&2
      exit 1
    }
  done
done < <(git -C "$CONSUMER" ls-tree -r --name-only "$REVISION" | awk '/^(common|v2)\/src\/main\/java\/.*\.java$/')

smoke_root="$WORK_ROOT/spi-smoke"
mkdir -p "$smoke_root/reference-classes" "$smoke_root/mantle-classes"
cp "$SMOKE_TEMPLATE" "$smoke_root/YoutubeSourceSpiSmoke.java"
reference_classpath="$(<"$WORK_ROOT/reference.classpath")"
mantle_classpath="$(<"$WORK_ROOT/mantle.classpath")"
reference_runtime="$(<"$WORK_ROOT/reference.runtime.classpath")"
mantle_runtime="$(<"$WORK_ROOT/mantle.runtime.classpath")"
reference_outputs="$WORK_ROOT/reference/v2/build/classes/java/main:$WORK_ROOT/reference/common/build/classes/java/main:$reference_classpath"
mantle_outputs="$WORK_ROOT/candidate/v2/build/classes/java/main:$WORK_ROOT/candidate/common/build/classes/java/main:$mantle_classpath"
javac -cp "$reference_outputs" -d "$smoke_root/reference-classes" "$smoke_root/YoutubeSourceSpiSmoke.java"
javac -cp "$mantle_outputs" -d "$smoke_root/mantle-classes" "$smoke_root/YoutubeSourceSpiSmoke.java"
reference_smoke="$(java -cp "$smoke_root/reference-classes:$reference_runtime:$reference_classpath" YoutubeSourceSpiSmoke 2>"$LOG_ROOT/reference-smoke.stderr")"
mantle_smoke="$(java --enable-native-access=ALL-UNNAMED -cp "$smoke_root/mantle-classes:$mantle_runtime:$mantle_classpath" YoutubeSourceSpiSmoke "$NATIVE_LIBRARY" 2>"$LOG_ROOT/mantle-smoke.stderr")"
[[ "$reference_smoke" == "$mantle_smoke" ]] || {
  printf 'youtube-source SPI smoke diverged:\nreference: %s\nMantle: %s\n' "$reference_smoke" "$mantle_smoke" >&2
  exit 1
}
[[ "$mantle_smoke" == 'source_manager=PASS delegated_track=PASS http=PASS serialization=PASS executor=PASS container_spi=PASS' ]] || exit 1

jq -n \
  --arg revision "$REVISION" \
  --arg reference_log "$LOG_ROOT/reference.log" \
  --arg mantle_log "$LOG_ROOT/mantle.log" \
  --arg reference_build "$WORK_ROOT/reference/v2/build.gradle.kts" \
  --arg mantle_build "$WORK_ROOT/candidate/v2/build.gradle.kts" \
  --arg smoke "$mantle_smoke" \
  --arg reference_smoke_stderr "$LOG_ROOT/reference-smoke.stderr" \
  --arg mantle_smoke_stderr "$LOG_ROOT/mantle-smoke.stderr" \
  --arg java_home "${JAVA_HOME:-system}" \
  '{schema_version: 1, status: "PASS", consumer: "youtube_source", revision: $revision,
    reference_compile: {status: 0, log: $reference_log},
    mantle_compile: {status: 0, log: $mantle_log, failure_diff: [], task_outcome_match: true},
    source_unchanged: true,
    build_normalization: ["Gradle 8.10 executed with a JDK 17/21-compatible toolchain", "v2 compileOnly and testImplementation Lavaplayer coordinate only", "v2 dependency resolution targets JVM 11 while Java bytecode remains targeted to 8", "commons-io:2.13.0 added only to the generated smoke runtime because Lavaplayer is compileOnly"],
    behaviors: {source_manager: "PASS", delegated_track: "PASS", http_facade: "PASS", serialization: "PASS", executor: "PASS", container_spi: "PASS"},
    smoke: {reference_and_mantle_match: true, output: $smoke, reference_stderr: $reference_smoke_stderr, mantle_stderr: $mantle_smoke_stderr},
    generated_builds: {reference: $reference_build, mantle: $mantle_build}, java_home: $java_home}' \
  > "$WORK_ROOT/result.json"

jq -e '.status == "PASS" and .source_unchanged == true and .smoke.reference_and_mantle_match == true and (.behaviors | to_entries | all(.value == "PASS"))' "$WORK_ROOT/result.json" >/dev/null
printf 'Phase 14 youtube-source SPI compatibility passed: unchanged v2/common sources compiled against reference and Mantle, and source-manager/delegated-track/HTTP/serialization/executor/container SPI smoke matched.\n'
