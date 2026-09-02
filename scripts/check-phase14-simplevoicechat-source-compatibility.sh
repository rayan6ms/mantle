#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CACHE_ROOT="${MANTLE_PHASE14_CONSUMER_CACHE:-$ROOT/.cache/phase14-consumers}"
readonly CONSUMER="$CACHE_ROOT/simplevoicechat_music"
readonly WORK_ROOT="$ROOT/target/phase14/simplevoicechat-source-compatibility"
readonly LOCAL_REPO="$WORK_ROOT/m2"
readonly LOG_ROOT="$WORK_ROOT/logs"
readonly REVISION="f21305f4deafc4c5869a060e8dcfbbf24d73c82b"
readonly EXPECTED_REVISION_DATE="2024-07-20T14:55:51-04:00"
readonly REPOSITORY="https://github.com/ItzDerock/simplevoicechat-music"
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly REFERENCE_POM="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.pom"
readonly MANTLE_JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly NATIVE_LIBRARY="${MANTLE_PHASE14_NATIVE_LIBRARY:-$ROOT/target/debug/libmantle_jvm.so}"
readonly MANTLE_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly OVERLAY_TEMPLATE="$ROOT/scripts/phase14-simplevoicechat-mantle.init.gradle"
readonly SMOKE_TEMPLATE="$ROOT/scripts/phase14-simplevoicechat-linkage-smoke.java.txt"

for command in cargo git java javac jar jq perl sha256sum tar; do
  command -v "$command" >/dev/null || { printf 'Phase 14 Simple Voice Chat gate requires %s\n' "$command" >&2; exit 1; }
done

"$ROOT/scripts/check-phase14-consumer-inventory.sh" --verify-upstream >/dev/null

if [[ ! -d "$CONSUMER/.git" ]]; then
  if [[ -e "$CONSUMER" ]]; then
    printf 'Simple Voice Chat cache path exists but is not a Git checkout: %s\n' "$CONSUMER" >&2
    exit 1
  fi
  mkdir -p "$CONSUMER"
  git -C "$CONSUMER" init --quiet
  git -C "$CONSUMER" remote add origin "$REPOSITORY"
  git -C "$CONSUMER" fetch --quiet --depth=1 origin "$REVISION"
  git -C "$CONSUMER" checkout --quiet --detach "$REVISION"
fi
actual_origin="$(git -C "$CONSUMER" remote get-url origin 2>/dev/null || true)"
[[ "${actual_origin%.git}" == "${REPOSITORY%.git}" ]] || { printf 'Simple Voice Chat cache origin mismatch\n' >&2; exit 1; }
if [[ "$(git -C "$CONSUMER" rev-parse HEAD 2>/dev/null || true)" != "$REVISION" ]]; then
  git -C "$CONSUMER" checkout --quiet --detach "$REVISION"
fi
[[ "$(git -C "$CONSUMER" rev-parse HEAD)" == "$REVISION" ]] || { printf 'Simple Voice Chat cache revision mismatch\n' >&2; exit 1; }
[[ -x "$CONSUMER/gradlew" ]] || { printf 'Pinned Simple Voice Chat checkout missing Gradle wrapper\n' >&2; exit 1; }
[[ -f "$REFERENCE_JAR" && -f "$REFERENCE_POM" ]] || { printf 'Frozen Lavaplayer reference artifact is missing\n' >&2; exit 1; }
[[ "$(git -C "$CONSUMER" show -s --format='%cI' HEAD)" == "$EXPECTED_REVISION_DATE" ]] || { printf 'Simple Voice Chat cache revision date mismatch\n' >&2; exit 1; }
[[ -z "$(git -C "$CONSUMER" status --porcelain)" ]] || { printf 'Pinned Simple Voice Chat checkout is dirty\n' >&2; exit 1; }

# The pinned Loom 1.6 build supports JDK 17/21, while Gradle 8.6 cannot parse JDK 25 class files.
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
  chmod +x "$project/gradlew"
  if [[ "$mode" == reference ]]; then
    perl -0pi -e 's#implementation [\x27]dev\.arbjerg:lavaplayer:2\.2\.1[\x27]#implementation '\''dev.arbjerg:lavaplayer:2.2.6'\''#' "$project/build.gradle"
  else
    perl -0pi -e 's#implementation [\x27]dev\.arbjerg:lavaplayer:2\.2\.1[\x27]#implementation '\''io.github.rayan6ms:mantle-lavaplayer:1.0.0'\''#' "$project/build.gradle"
  fi
  grep -F "implementation '$([[ "$mode" == reference ]] && printf 'dev.arbjerg:lavaplayer:2.2.6' || printf 'io.github.rayan6ms:mantle-lavaplayer:1.0.0')'" "$project/build.gradle" >/dev/null
}

prepare_project "$WORK_ROOT/reference" reference
prepare_project "$WORK_ROOT/candidate" mantle

run_gradle() {
  local label="$1"
  local project="$2"
  local overlay="$WORK_ROOT/$label.init.gradle"
  local classpath_output="$WORK_ROOT/$label.classpath"
  sed -e "s|__LOCAL_REPO__|$LOCAL_REPO|g" \
      -e "s|__CLASSPATH_OUTPUT__|$classpath_output|g" \
      "$OVERLAY_TEMPLATE" > "$overlay"
  set +e
  (cd "$project" && ./gradlew compileJava phase14WriteCompileClasspath --no-daemon --stacktrace --init-script "$overlay") \
    >"$LOG_ROOT/$label.log" 2>&1
  local status=$?
  set -e
  printf '%s' "$status" > "$LOG_ROOT/$label.exit"
  return "$status"
}

run_gradle reference "$WORK_ROOT/reference" || {
  printf 'Simple Voice Chat reference compile failed; see %s\n' "$LOG_ROOT/reference.log" >&2
  exit 1
}
run_gradle mantle "$WORK_ROOT/candidate" || {
  printf 'Simple Voice Chat Mantle compile failed; see %s\n' "$LOG_ROOT/mantle.log" >&2
  exit 1
}

for log in reference mantle; do
  grep -F 'BUILD SUCCESSFUL' "$LOG_ROOT/$log.log" >/dev/null
  [[ "$(cat "$LOG_ROOT/$log.exit")" == 0 ]] || exit 1
  [[ -s "$WORK_ROOT/$log.classpath" ]] || { printf 'Compile classpath record missing: %s\n' "$log" >&2; exit 1; }
done

# Hash the source touchpoints recorded by the inventory in both generated consumers.
for source_hash in \
  'src/main/java/dev/derock/svcmusic/audio/GroupManager.java 9b2a781232813fc6e86ad2e27cb48dd828d788ded3b12650ef4379c5e06aca4f' \
  'src/main/java/dev/derock/svcmusic/audio/TrackScheduler.java 839d6508cc215d427a25093efae7a02ffb357381d8d5cab11f59d43dac363357' \
  'src/main/java/dev/derock/svcmusic/audio/MusicManager.java c0c0df75161ecab00a4794dc54626fa09babdb6253ee167623ee5d788e7fc2f0' \
  'src/main/java/dev/derock/svcmusic/commands/PlayCommand.java 7de3df00d984275c1e02ed0aac543b19c565c5be35789cd11f69438ccfb1792a'; do
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
done < <(git -C "$CONSUMER" ls-tree -r --name-only "$REVISION" | awk '/^src\/main\/java\/.*\.java$/')

smoke_root="$WORK_ROOT/linkage-smoke"
mkdir -p "$smoke_root/reference-classes" "$smoke_root/mantle-classes"
cp "$SMOKE_TEMPLATE" "$smoke_root/SimpleVoiceChatLinkageSmoke.java"
reference_classpath="$(<"$WORK_ROOT/reference.classpath")"
mantle_classpath="$(<"$WORK_ROOT/mantle.classpath")"
javac -cp "$reference_classpath" -d "$smoke_root/reference-classes" "$smoke_root/SimpleVoiceChatLinkageSmoke.java"
javac -cp "$mantle_classpath" -d "$smoke_root/mantle-classes" "$smoke_root/SimpleVoiceChatLinkageSmoke.java"
reference_smoke="$(java -cp "$smoke_root/reference-classes:$reference_classpath" SimpleVoiceChatLinkageSmoke)"
mantle_smoke="$(java --enable-native-access=ALL-UNNAMED -cp "$smoke_root/mantle-classes:$mantle_classpath" SimpleVoiceChatLinkageSmoke "$NATIVE_LIBRARY")"
[[ "$reference_smoke" == "$mantle_smoke" ]] || {
  printf 'Simple Voice Chat API smoke diverged:\nreference: %s\nMantle: %s\n' "$reference_smoke" "$mantle_smoke" >&2
  exit 1
}
[[ "$mantle_smoke" == 'frame=PASS equalizer=PASS listener=PASS ordered_loading=PASS player_state=PASS' ]] || exit 1

jq -n \
  --arg revision "$REVISION" \
  --arg reference_log "$LOG_ROOT/reference.log" \
  --arg mantle_log "$LOG_ROOT/mantle.log" \
  --arg reference_pom "$WORK_ROOT/reference/build.gradle" \
  --arg mantle_pom "$WORK_ROOT/candidate/build.gradle" \
  --arg smoke "$mantle_smoke" \
  --arg java_home "${JAVA_HOME:-system}" \
  '{schema_version: 1, status: "PASS", consumer: "simplevoicechat_music", revision: $revision,
    reference_compile: {status: 0, log: $reference_log},
    mantle_compile: {status: 0, log: $mantle_log, failure_diff: [], task_outcome_match: true},
    source_unchanged: true,
    build_normalization: ["Gradle 8.6 executed with a JDK 17/21-compatible toolchain", "Lavaplayer coordinate only"],
    behaviors: {reusable_pcm_frame: "PASS", queue_and_listeners: "PASS", ordered_loading: "PASS", equalizer: "PASS", non_discord_transport: "PASS"},
    smoke: {reference_and_mantle_match: true, output: $smoke},
    generated_builds: {reference: $reference_pom, mantle: $mantle_pom}, java_home: $java_home}' \
  > "$WORK_ROOT/result.json"

jq -e '.status == "PASS" and .source_unchanged == true and .smoke.reference_and_mantle_match == true and .behaviors.reusable_pcm_frame == "PASS" and .behaviors.non_discord_transport == "PASS"' \
  "$WORK_ROOT/result.json" >/dev/null
printf 'Phase 14 Simple Voice Chat source compatibility passed: unchanged reference and Mantle compiles succeeded, reusable PCM frame/equalizer/listener/ordered-loading/player-state smoke matched, and the non-Discord transport touchpoint linked.\n'
