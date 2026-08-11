#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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

for consumer in smoke probe integration classloader; do
  cargo run --locked -q -p mantle-jvm-gate -- "write-$consumer-consumer" \
    --output "$WORK/Gate${consumer^}.java"
done

javac --release 11 -cp "$REFERENCE_JAR" -d "$CLASSES" \
  "$WORK/GateSmoke.java" "$WORK/GateProbe.java" "$WORK/GateIntegration.java"
javac --release 11 -d "$CLASSES" "$WORK/GateClassloader.java"

case "$(uname -s)" in
  Darwin) native="$ROOT/target/debug/libmantle_jvm.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) native="$ROOT/target/debug/mantle_jvm.dll"; classpath_separator=';' ;;
  *) native="$ROOT/target/debug/libmantle_jvm.so" ;;
esac
classpath_separator="${classpath_separator:-:}"
if command -v cygpath >/dev/null 2>&1; then
  native="$(cygpath -w "$native")"
  jar_argument="$(cygpath -w "$JAR")"
else
  native="$(cd "$(dirname "$native")" && pwd)/$(basename "$native")"
  jar_argument="$JAR"
fi

readonly GATE_CLASSPATH="$CLASSES$classpath_separator$JAR"
java -Xverify:all -cp "$GATE_CLASSPATH" GateSmoke "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateIntegration "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" callbacks
java -Xverify:all -Xmx256m -cp "$GATE_CLASSPATH" GateProbe "$native" lifetime
java -Xverify:all -cp "$CLASSES" GateClassloader "$jar_argument" "$native"
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" leak-manager
java -Xverify:all -cp "$GATE_CLASSPATH" GateProbe "$native" dispatcher-exit

cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$MISMATCH_JAR" --expected-abi 2
if java -Xverify:all -cp "$CLASSES$classpath_separator$MISMATCH_JAR" GateSmoke "$native" \
    >"$WORK/abi-mismatch.stdout" 2>"$WORK/abi-mismatch.stderr"; then
  printf 'ABI mismatch unexpectedly succeeded\n' >&2
  exit 1
fi
if ! grep -q 'Mantle compatibility JAR expects native ABI 2' "$WORK/abi-mismatch.stderr"; then
  printf 'ABI mismatch did not produce the required diagnostic\n' >&2
  exit 1
fi

printf 'Gate A JVM suite passed on %s (%s).\n' "$(java -version 2>&1 | sed -n '1p')" "$(uname -s)"
