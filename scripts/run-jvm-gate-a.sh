#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly WORK="$ROOT/target/gate-a"
readonly CLASSES="$WORK/consumer-classes"
readonly JAR="$WORK/mantle-gate-a.jar"
readonly MISMATCH_JAR="$WORK/mantle-gate-a-mismatch.jar"

if [[ ! -f "$REFERENCE_JAR" ]]; then
  printf 'Gate A reference JAR not found: %s\n' "$REFERENCE_JAR" >&2
  exit 1
fi

mkdir -p "$CLASSES"
cargo build --locked -p mantle-jvm --features gate-a-direct-attachment
cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$JAR" --expected-abi 1 \
  --manifest-output "$WORK/emission-manifest.json"
cargo run --locked -q -p mantle-jvm-gate -- verify-structure \
  --reference-jar "$REFERENCE_JAR" --candidate-jar "$JAR"

for consumer in smoke probe integration classloader event track-value track-enum; do
  case "$consumer" in
    smoke) consumer_class='Smoke' ;;
    probe) consumer_class='Probe' ;;
    integration) consumer_class='Integration' ;;
    classloader) consumer_class='Classloader' ;;
    event) consumer_class='Events' ;;
    track-value) consumer_class='TrackValues' ;;
    track-enum) consumer_class='TrackEnums' ;;
  esac
  cargo run --locked -q -p mantle-jvm-gate -- "write-$consumer-consumer" \
    --output "$WORK/Gate${consumer_class}.java"
done

javac --release 11 -cp "$REFERENCE_JAR" -d "$CLASSES" \
  "$WORK/GateSmoke.java" "$WORK/GateProbe.java" "$WORK/GateIntegration.java" \
  "$WORK/GateEvents.java" "$WORK/GateTrackValues.java" "$WORK/GateTrackEnums.java"
javac --release 11 -d "$CLASSES" "$WORK/GateClassloader.java"

case "$(uname -s)" in
  Darwin) native="$ROOT/target/debug/libmantle_jvm.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) native="$ROOT/target/debug/mantle_jvm.dll"; classpath_separator=';' ;;
  *) native="$ROOT/target/debug/libmantle_jvm.so" ;;
esac
classpath_separator="${classpath_separator:-:}"
if command -v cygpath >/dev/null 2>&1; then
  native="$(cygpath -w "$native")"
  classes_argument="$(cygpath -w "$CLASSES")"
  jar_argument="$(cygpath -w "$JAR")"
  reference_argument="$(cygpath -w "$REFERENCE_JAR")"
else
  native="$(cd "$(dirname "$native")" && pwd)/$(basename "$native")"
  classes_argument="$CLASSES"
  jar_argument="$JAR"
  reference_argument="$REFERENCE_JAR"
fi

readonly GATE_CLASSPATH="$classes_argument$classpath_separator$jar_argument"
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateEvents \
  >"$WORK/event-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateEvents \
  >"$WORK/event-candidate.txt"
cmp "$WORK/event-reference.txt" "$WORK/event-candidate.txt"
grep --fixed-strings \
  'pause,resume,start,end,exception,stuck,|legacy-stuck' "$WORK/event-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateTrackValues \
  >"$WORK/track-values-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateTrackValues \
  >"$WORK/track-values-candidate.txt"
cmp "$WORK/track-values-reference.txt" "$WORK/track-values-candidate.txt"
grep --fixed-strings \
  'playlist=identity,mutable,true;marker=987654321,identity' \
  "$WORK/track-values-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateTrackEnums \
  >"$WORK/track-enums-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateTrackEnums \
  >"$WORK/track-enums-candidate.txt"
cmp "$WORK/track-enums-reference.txt" "$WORK/track-enums-candidate.txt"
grep --fixed-strings \
  'copy=true;lookup-errors=iae,npe;reflection=5,6,7' \
  "$WORK/track-enums-candidate.txt" >/dev/null
java -Xverify:all -cp "$GATE_CLASSPATH" GateSmoke "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateIntegration "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" callbacks
java -Xverify:all -Xmx256m -cp "$GATE_CLASSPATH" GateProbe "$native" lifetime
java -Xverify:all -cp "$classes_argument" GateClassloader "$jar_argument" "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" leak-manager
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" dispatcher-exit

cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$MISMATCH_JAR" --expected-abi 2
if command -v cygpath >/dev/null 2>&1; then
  mismatch_argument="$(cygpath -w "$MISMATCH_JAR")"
else
  mismatch_argument="$MISMATCH_JAR"
fi
if java -Xverify:all -cp "$classes_argument$classpath_separator$mismatch_argument" GateSmoke "$native" \
    >"$WORK/abi-mismatch.stdout" 2>"$WORK/abi-mismatch.stderr"; then
  printf 'ABI mismatch unexpectedly succeeded\n' >&2
  exit 1
fi
if ! grep -q 'Mantle compatibility JAR expects native ABI 2' "$WORK/abi-mismatch.stderr"; then
  printf 'ABI mismatch did not produce the required diagnostic\n' >&2
  exit 1
fi

printf 'Gate A JVM suite passed on %s (%s).\n' "$(java -version 2>&1 | sed -n '1p')" "$(uname -s)"
