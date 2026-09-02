#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly RESULT_ROOT="${PHASE15_RESULTS_ROOT:-$ROOT/target/phase15/concurrency-lifecycle}"
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly EXPLICIT_COUNT="${MANTLE_LIFETIME_EXPLICIT_COUNT:-2048}"
readonly GC_COUNT="${MANTLE_LIFETIME_GC_COUNT:-1024}"
readonly TIMEOUT_SECONDS="${MANTLE_LIFETIME_TIMEOUT_SECONDS:-30}"
readonly WORK="$RESULT_ROOT/jvm"
readonly CLASSES="$WORK/classes"
readonly JAR="$WORK/mantle-gate-a.jar"

unset APPIMAGE APPDIR
test -f "$REFERENCE_JAR"
mkdir -p "$CLASSES"

env -u APPIMAGE -u APPDIR cargo test --locked -p mantle-jvm \
  ordering_key::tests::phase15_ -- --nocapture >"$RESULT_ROOT/loom-queue.log" 2>&1
env -u APPIMAGE -u APPDIR cargo test --locked -p mantle-core \
  --test phase11_async_loading -- --nocapture >"$RESULT_ROOT/core-lifecycle.log" 2>&1

env -u APPIMAGE -u APPDIR cargo build --locked -p mantle-jvm \
  --features gate-a-direct-attachment >"$RESULT_ROOT/jvm-build.log" 2>&1
env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$JAR" --expected-abi 1 \
  >"$RESULT_ROOT/jar-emission.log" 2>&1
env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- write-probe-consumer \
  --output "$WORK/GateProbe.java" >>"$RESULT_ROOT/jar-emission.log" 2>&1
env -u APPIMAGE -u APPDIR cargo run --locked -q -p mantle-jvm-gate -- write-classloader-consumer \
  --output "$WORK/GateClassloader.java" >>"$RESULT_ROOT/jar-emission.log" 2>&1
javac --release 11 -cp "$REFERENCE_JAR" -d "$CLASSES" "$WORK/GateProbe.java"
javac --release 11 -d "$CLASSES" "$WORK/GateClassloader.java"

NATIVE="$ROOT/target/debug/libmantle_jvm.so"
case "$(uname -s)" in
  Darwin) NATIVE="$ROOT/target/debug/libmantle_jvm.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) NATIVE="$ROOT/target/debug/mantle_jvm.dll" ;;
esac
NATIVE="$(cd "$(dirname "$NATIVE")" && pwd)/$(basename "$NATIVE")"
readonly NATIVE

{
  printf 'bounds explicit=%s gc=%s timeout_seconds=%s\n' \
    "$EXPLICIT_COUNT" "$GC_COUNT" "$TIMEOUT_SECONDS"
  java -Xverify:all -Xmx256m \
    -Dmantle.lifetime.explicit="$EXPLICIT_COUNT" \
    -Dmantle.lifetime.gc="$GC_COUNT" \
    -Dmantle.lifetime.timeout-seconds="$TIMEOUT_SECONDS" \
    -cp "$CLASSES:$JAR" GateProbe "$NATIVE" lifetime
  java -Xverify:all -cp "$CLASSES:$JAR" GateClassloader "$JAR" "$NATIVE"
  java -Xverify:all -cp "$CLASSES:$JAR" GateProbe "$NATIVE" leak-manager
  java -Xverify:all -cp "$CLASSES:$JAR" GateProbe "$NATIVE" dispatcher-exit
  printf 'JNI lifecycle probes passed.\n'
} >"$RESULT_ROOT/jni-lifecycle.log" 2>&1

printf 'Phase 15 concurrency/lifecycle probes passed.\n'
