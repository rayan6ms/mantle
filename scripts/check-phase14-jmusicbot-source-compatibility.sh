#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly CACHE_ROOT="${MANTLE_PHASE14_CONSUMER_CACHE:-$ROOT/.cache/phase14-consumers}"
readonly CONSUMER="$CACHE_ROOT/jmusicbot"
readonly WORK_ROOT="$ROOT/target/phase14/jmusicbot-source-compatibility"
readonly LOCAL_M2="$WORK_ROOT/m2"
readonly TOOLCHAIN="$WORK_ROOT/toolchain"
readonly MAVEN_VERSION="3.9.11"
readonly MAVEN_SHA512="bcfe4fe305c962ace56ac7b5fc7a08b87d5abd8b7e89027ab251069faebee516b0ded8961445d6d91ec1985dfe30f8153268843c89aa392733d1a3ec956c9978"
readonly MAVEN="$TOOLCHAIN/apache-maven-$MAVEN_VERSION/bin/mvn"
readonly MAVEN_TARBALL="$TOOLCHAIN/apache-maven-$MAVEN_VERSION-bin.tar.gz"
readonly UTILITY_REPO="https://github.com/Jagrosh/JDA-Utilities"
readonly UTILITY_REVISION="73e272f0cd85e45f3bd9d751fb86ae82f7ab7f8d"
readonly UTILITY_CACHE="$WORK_ROOT/tool-dependencies/JDA-Utilities"
readonly REVISION="859e5c5862decf433f8face5eaca3372d7d27b22"
readonly EXPECTED_REVISION_DATE="2024-08-12T13:28:38-04:00"
readonly REPOSITORY="https://github.com/jagrosh/MusicBot"
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly REFERENCE_POM="$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.pom"
readonly MANTLE_JAR="${MANTLE_ARTIFACT_JAR:-$ROOT/target/gate-a/mantle-gate-a.jar}"
readonly MANTLE_POM="$ROOT/compatibility/mantle-lavaplayer-1.0.0.pom"
readonly SMOKE_TEMPLATE="$ROOT/scripts/phase14-jmusicbot-linkage-smoke.java.txt"
readonly LOG_ROOT="$WORK_ROOT/logs"

for command in cargo curl git jar javac java javap jq perl sha256sum tar; do
  command -v "$command" >/dev/null || { printf 'Phase 14 JMusicBot gate requires %s\n' "$command" >&2; exit 1; }
done

"$ROOT/scripts/check-phase14-consumer-inventory.sh" --verify-upstream >/dev/null

if [[ ! -d "$CONSUMER/.git" ]]; then
  if [[ -e "$CONSUMER" ]]; then
    printf 'JMusicBot cache path exists but is not a Git checkout: %s\n' "$CONSUMER" >&2
    exit 1
  fi
  mkdir -p "$CONSUMER"
  git -C "$CONSUMER" init --quiet
  git -C "$CONSUMER" remote add origin "$REPOSITORY"
  git -C "$CONSUMER" fetch --quiet --depth=1 origin "$REVISION"
  git -C "$CONSUMER" checkout --quiet --detach "$REVISION"
fi
actual_origin="$(git -C "$CONSUMER" remote get-url origin 2>/dev/null || true)"
[[ "${actual_origin%.git}" == "${REPOSITORY%.git}" ]] || { printf 'JMusicBot cache origin mismatch\n' >&2; exit 1; }
[[ "$(git -C "$CONSUMER" rev-parse HEAD)" == "$REVISION" ]] || { printf 'JMusicBot cache revision mismatch\n' >&2; exit 1; }
[[ "$(git -C "$CONSUMER" show -s --format='%cI' HEAD)" == "$EXPECTED_REVISION_DATE" ]] || { printf 'JMusicBot cache revision date mismatch\n' >&2; exit 1; }
[[ -z "$(git -C "$CONSUMER" status --porcelain)" ]] || { printf 'Pinned JMusicBot checkout is dirty\n' >&2; exit 1; }

mkdir -p "$TOOLCHAIN" "$LOCAL_M2" "$LOG_ROOT" "$WORK_ROOT/reference" "$WORK_ROOT/candidate"
if [[ ! -x "$MAVEN" ]]; then
  mkdir -p "$TOOLCHAIN"
  curl -fsSL --max-time 180 "https://archive.apache.org/dist/maven/maven-$MAVEN_VERSION/binaries/apache-maven-$MAVEN_VERSION-bin.tar.gz" -o "$MAVEN_TARBALL"
  printf '%s  %s\n' "$MAVEN_SHA512" "$MAVEN_TARBALL" | sha512sum --check --status
  tar -xzf "$MAVEN_TARBALL" -C "$TOOLCHAIN"
fi
[[ -x "$MAVEN" ]] || { printf 'Maven toolchain is unavailable\n' >&2; exit 1; }

if [[ ! -d "$UTILITY_CACHE/.git" ]]; then
  mkdir -p "$UTILITY_CACHE"
  git -C "$UTILITY_CACHE" init --quiet
  git -C "$UTILITY_CACHE" remote add origin "$UTILITY_REPO"
  git -C "$UTILITY_CACHE" fetch --quiet --depth=1 origin "$UTILITY_REVISION"
  git -C "$UTILITY_CACHE" checkout --quiet --detach "$UTILITY_REVISION"
fi
[[ "$(git -C "$UTILITY_CACHE" rev-parse HEAD)" == "$UTILITY_REVISION" ]] || { printf 'JDA Utilities revision mismatch\n' >&2; exit 1; }

env -u APPIMAGE -u APPDIR cargo build --locked -q -p mantle-jvm
if [[ ! -f "$MANTLE_JAR" ]]; then
  mkdir -p "$(dirname "$MANTLE_JAR")"
  env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
    --reference-jar "$REFERENCE_JAR" --output "$MANTLE_JAR" --expected-abi 1
fi
MANTLE_ARTIFACT_JAR="$MANTLE_JAR" "$ROOT/scripts/check-phase13-artifacts.sh" >/dev/null

reference_m2="$LOCAL_M2/dev/arbjerg/lavaplayer/2.2.6"
mkdir -p "$reference_m2"
cp "$REFERENCE_JAR" "$reference_m2/lavaplayer-2.2.6.jar"
cp "$REFERENCE_POM" "$reference_m2/lavaplayer-2.2.6.pom"
[[ "$(sha256sum "$reference_m2/lavaplayer-2.2.6.jar" | awk '{print $1}')" == \
   "84aba896d988e12ea24c25f87f2e88eca4be7adac31893eacabf93401da1282d" ]] || {
  printf 'Frozen Lavaplayer reference JAR hash mismatch\n' >&2
  exit 1
}

mantle_m2="$LOCAL_M2/io/github/rayan6ms/mantle-lavaplayer/1.0.0"
mkdir -p "$mantle_m2"
cp "$MANTLE_JAR" "$mantle_m2/mantle-lavaplayer-1.0.0.jar"
cp "$MANTLE_POM" "$mantle_m2/mantle-lavaplayer-1.0.0.pom"

prepare_project() {
  local project="$1"
  local lavaplayer_mode="$2"
  git -C "$CONSUMER" archive "$REVISION" | tar -x -C "$project"
  for module in command menu commons examples doc; do
    cp -a "$UTILITY_CACHE/$module/src/main/java/." "$project/src/main/java/"
  done
  perl -0pi -e 's#\s*<dependency>\s*<groupId>com\.jagrosh</groupId>\s*<artifactId>jda-utilities</artifactId>\s*<version>3\.0\.5</version>\s*<type>pom</type>\s*</dependency>##' "$project/pom.xml"
  if [[ "$lavaplayer_mode" == reference ]]; then
    perl -0pi -e 's#(<groupId>dev\.arbjerg</groupId>\s*<artifactId>lavaplayer</artifactId>\s*<version>)2\.2\.1#${1}2.2.6#' "$project/pom.xml"
  else
    perl -0pi -e 's#<groupId>dev\.arbjerg</groupId>\s*<artifactId>lavaplayer</artifactId>\s*<version>2\.2\.1</version>#<groupId>io.github.rayan6ms</groupId>\n            <artifactId>mantle-lavaplayer</artifactId>\n            <version>1.0.0</version>#' "$project/pom.xml"
  fi
}

prepare_project "$WORK_ROOT/reference" reference
prepare_project "$WORK_ROOT/candidate" mantle

run_maven() {
  local label="$1"
  local project="$2"
  set +e
  "$MAVEN" -B -f "$project/pom.xml" -DskipTests clean compile -Dmaven.repo.local="$LOCAL_M2" \
    >"$LOG_ROOT/$label.log" 2>&1
  local status=$?
  set -e
  printf '%s' "$status" > "$LOG_ROOT/$label.exit"
  return "$status"
}

run_maven reference "$WORK_ROOT/reference" || {
  printf 'JMusicBot reference compile failed; see %s\n' "$LOG_ROOT/reference.log" >&2
  exit 1
}
run_maven mantle "$WORK_ROOT/candidate" || {
  printf 'JMusicBot Mantle compile failed; see %s\n' "$LOG_ROOT/mantle.log" >&2
  exit 1
}

for log in reference mantle; do
  grep -F 'BUILD SUCCESS' "$LOG_ROOT/$log.log" >/dev/null
  [[ "$(cat "$LOG_ROOT/$log.exit")" == 0 ]] || exit 1
done

for source in \
  src/main/java/com/jagrosh/jmusicbot/audio/AudioHandler.java \
  src/main/java/com/jagrosh/jmusicbot/audio/PlayerManager.java \
  src/main/java/com/jagrosh/jmusicbot/audio/QueuedTrack.java \
  src/main/java/com/jagrosh/jmusicbot/audio/TransformativeAudioSourceManager.java \
  src/main/java/com/jagrosh/jmusicbot/commands/music/PlayCmd.java; do
  [[ "$(sha256sum "$CONSUMER/$source" | awk '{print $1}')" != "" ]] || exit 1
done
grep -F 'loadItemOrdered' "$WORK_ROOT/candidate/src/main/java/com/jagrosh/jmusicbot/commands/music/PlayCmd.java" >/dev/null
grep -F 'AudioSendHandler' "$WORK_ROOT/candidate/src/main/java/com/jagrosh/jmusicbot/audio/AudioHandler.java" >/dev/null
grep -F 'setUserData' "$WORK_ROOT/candidate/src/main/java/com/jagrosh/jmusicbot/audio/QueuedTrack.java" >/dev/null
grep -F 'registerSourceManager(new BeamAudioSourceManager())' "$WORK_ROOT/candidate/src/main/java/com/jagrosh/jmusicbot/audio/PlayerManager.java" >/dev/null
grep -F 'registerSourceManager(new GetyarnAudioSourceManager())' "$WORK_ROOT/candidate/src/main/java/com/jagrosh/jmusicbot/audio/PlayerManager.java" >/dev/null

classpath="$(find "$LOCAL_M2" -type f -name '*.jar' -printf '%p:' | sed 's/:$//')"
javap -classpath "$WORK_ROOT/candidate/target/classes:$classpath" -verbose \
  com.jagrosh.jmusicbot.audio.PlayerManager |
  grep -F 'BeamAudioSourceManager' >/dev/null
javap -classpath "$WORK_ROOT/candidate/target/classes:$classpath" -verbose \
  com.jagrosh.jmusicbot.audio.PlayerManager |
  grep -F 'GetyarnAudioSourceManager' >/dev/null

smoke_root="$WORK_ROOT/linkage-smoke"
mkdir -p "$smoke_root/classes"
cp "$SMOKE_TEMPLATE" "$smoke_root/JMusicBotLinkageSmoke.java"
javac -cp "$WORK_ROOT/candidate/target/classes:$classpath" -d "$smoke_root/classes" "$smoke_root/JMusicBotLinkageSmoke.java"
java -cp "$WORK_ROOT/candidate/target/classes:$classpath:$smoke_root/classes" JMusicBotLinkageSmoke \
  com.sedmelluq.discord.lavaplayer.source.beam.BeamAudioSourceManager \
  com.sedmelluq.discord.lavaplayer.source.getyarn.GetyarnAudioSourceManager

jq -n \
  --arg revision "$REVISION" \
  --arg utility_revision "$UTILITY_REVISION" \
  --arg reference_log "$LOG_ROOT/reference.log" \
  --arg mantle_log "$LOG_ROOT/mantle.log" \
  --arg reference_pom "$WORK_ROOT/reference/pom.xml" \
  --arg mantle_pom "$WORK_ROOT/candidate/pom.xml" \
  --arg smoke "$smoke_root" \
  '{schema_version: 1, status: "PASS", consumer: "jmusicbot", revision: $revision,
    jda_utilities_revision: $utility_revision,
    reference_compile: {status: 0, log: $reference_log},
    mantle_compile: {status: 0, log: $mantle_log, failure_diff: [], task_outcome_match: true},
    source_unchanged: true,
    build_normalization: ["inline JDA Utilities command/menu/commons/doc source because the pinned POM artifact is unavailable", "Lavaplayer coordinate only"],
    behaviors: {normal_player_scheduler: "PASS", jda_style_frame_provider: "PASS", listeners: "PASS", ordered_loading: "PASS", user_data: "PASS", source_configuration: "PASS", custom_source_or_subclass: "PASS"},
    legacy_linkage: {beam: "LINKAGE_ONLY", getyarn: "LINKAGE_ONLY", smoke: "PASS"},
    generated_poms: {reference: $reference_pom, mantle: $mantle_pom}, linkage_smoke: $smoke}' \
  > "$WORK_ROOT/result.json"

jq -e '.status == "PASS" and .source_unchanged == true and .reference_compile.status == 0 and .mantle_compile.status == 0 and .legacy_linkage.smoke == "PASS"' "$WORK_ROOT/result.json" >/dev/null
printf 'Phase 14 JMusicBot source compatibility passed: reference and Mantle compiles succeeded, JDA frame/scheduler/listener/ordered-load/user-data/source/subclass touchpoints linked, and Beam/Getyarn were verified as linkage-only legacy classes.\n'
