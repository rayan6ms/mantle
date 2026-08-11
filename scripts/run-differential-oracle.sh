#!/usr/bin/env bash
set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly WORK="$ROOT/target/oracle"
readonly SCENARIO="$ROOT/tests/oracle/scenarios/foundation.json"
readonly REFERENCE="$ROOT/.cache/reference/lavaplayer-2.2.6"
readonly REFERENCE_JAR="$REFERENCE/lavaplayer-2.2.6.jar"
readonly MANTLE_JAR="$WORK/mantle-gate-a.jar"

if [[ ! -f "$REFERENCE_JAR" ]] || ! find "$REFERENCE/dependencies" -maxdepth 1 -type f -name '*.jar' -print -quit | grep -q .; then
  printf 'Differential oracle requires the frozen reference JAR and runtime dependencies under %s\n' "$REFERENCE" >&2
  exit 1
fi

if [[ -n "${JAVA_HOME:-}" ]] && [[ -x "$JAVA_HOME/bin/java" ]]; then
  java="$JAVA_HOME/bin/java"
  javac="$JAVA_HOME/bin/javac"
elif command -v java >/dev/null 2>&1 && command -v javac >/dev/null 2>&1; then
  java="$(command -v java)"
  javac="$(command -v javac)"
elif [[ -x "$ROOT/.cache/toolchains/jdk-11.0.32+9/bin/java" ]]; then
  java="$ROOT/.cache/toolchains/jdk-11.0.32+9/bin/java"
  javac="$ROOT/.cache/toolchains/jdk-11.0.32+9/bin/javac"
else
  printf 'Differential oracle requires a JDK with java and javac.\n' >&2
  exit 1
fi

mkdir -p "$WORK/reference-classes" "$WORK/mantle-classes"

oracle() {
  cargo run --locked -q -p mantle-oracle -- "$@"
}

oracle validate --scenario "$SCENARIO"
oracle protocol --scenario "$SCENARIO" --output "$WORK/scenario.protocol"
oracle write-runner --backend reference --output "$WORK/OracleReference.java"
oracle write-runner --backend mantle --output "$WORK/OracleMantle.java"

cargo build --locked -p mantle-jvm --features gate-a-direct-attachment
cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$MANTLE_JAR" --expected-abi 1

reference_classpath="$REFERENCE_JAR"
while IFS= read -r dependency; do
  reference_classpath="$reference_classpath:$dependency"
done < <(find "$REFERENCE/dependencies" -maxdepth 1 -type f -name '*.jar' | sort)

"$javac" --release 11 -cp "$reference_classpath" \
  -d "$WORK/reference-classes" "$WORK/OracleReference.java"
"$javac" --release 11 -cp "$reference_classpath" \
  -d "$WORK/mantle-classes" "$WORK/OracleMantle.java"

case "$(uname -s)" in
  Darwin) native="$ROOT/target/debug/libmantle_jvm.dylib" ;;
  *) native="$ROOT/target/debug/libmantle_jvm.so" ;;
esac

for run in 1 2; do
  "$java" -Xverify:all -cp "$WORK/reference-classes:$reference_classpath" OracleReference \
    <"$WORK/scenario.protocol" >"$WORK/reference-$run.raw.jsonl" \
    2>"$WORK/reference-$run.stderr"
  "$java" -Xverify:all -cp "$WORK/mantle-classes:$MANTLE_JAR" OracleMantle "$native" \
    <"$WORK/scenario.protocol" >"$WORK/mantle-$run.raw.jsonl" \
    2>"$WORK/mantle-$run.stderr"
  oracle normalize --backend reference --scenario "$SCENARIO" \
    --input "$WORK/reference-$run.raw.jsonl" --output "$WORK/reference-$run.trace.json"
  oracle normalize --backend mantle --scenario "$SCENARIO" \
    --input "$WORK/mantle-$run.raw.jsonl" --output "$WORK/mantle-$run.trace.json"
done

oracle assert-deterministic \
  --first "$WORK/reference-1.trace.json" --second "$WORK/reference-2.trace.json"
oracle assert-deterministic \
  --first "$WORK/mantle-1.trace.json" --second "$WORK/mantle-2.trace.json"
oracle assert-deterministic \
  --first "$ROOT/tests/oracle/expected/reference-foundation.json" \
  --second "$WORK/reference-1.trace.json"
oracle assert-deterministic \
  --first "$ROOT/tests/oracle/expected/mantle-foundation.json" \
  --second "$WORK/mantle-1.trace.json"
oracle compare --reference "$WORK/reference-1.trace.json" \
  --mantle "$WORK/mantle-1.trace.json" --output "$WORK/comparison.json"

jq -r '"Differential oracle deterministic: \(.equal_records) equal records, \(.differences | length) expected differences."' \
  "$WORK/comparison.json"
