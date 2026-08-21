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

for consumer in smoke probe integration classloader event track-value track-enum track-contract audio-frame audio-configuration frame-buffer-factory audio-frame-buffer audio-frame-rebuilder terminator-audio-frame reference-mutable-audio-frame audio-frame-provider-tools audio-processing-context audio-player-options decoded-track-holder; do
  case "$consumer" in
    smoke) consumer_class='Smoke' ;;
    probe) consumer_class='Probe' ;;
    integration) consumer_class='Integration' ;;
    classloader) consumer_class='Classloader' ;;
    event) consumer_class='Events' ;;
    track-value) consumer_class='TrackValues' ;;
    track-enum) consumer_class='TrackEnums' ;;
    track-contract) consumer_class='TrackContracts' ;;
    audio-frame) consumer_class='AudioFrames' ;;
    audio-configuration) consumer_class='AudioConfiguration' ;;
    frame-buffer-factory) consumer_class='FrameBufferFactory' ;;
    audio-frame-buffer) consumer_class='AudioFrameBuffer' ;;
    audio-frame-rebuilder) consumer_class='AudioFrameRebuilder' ;;
    terminator-audio-frame) consumer_class='TerminatorAudioFrame' ;;
    reference-mutable-audio-frame) consumer_class='ReferenceMutableAudioFrame' ;;
    audio-frame-provider-tools) consumer_class='AudioFrameProviderTools' ;;
    audio-processing-context) consumer_class='AudioProcessingContext' ;;
    audio-player-options) consumer_class='AudioPlayerOptions' ;;
    decoded-track-holder) consumer_class='DecodedTrackHolder' ;;
  esac
  cargo run --locked -q -p mantle-jvm-gate -- "write-$consumer-consumer" \
    --output "$WORK/Gate${consumer_class}.java"
done

javac --release 11 -cp "$REFERENCE_JAR" -d "$CLASSES" \
  "$WORK/GateSmoke.java" "$WORK/GateProbe.java" "$WORK/GateIntegration.java" \
  "$WORK/GateEvents.java" "$WORK/GateTrackValues.java" "$WORK/GateTrackEnums.java" \
  "$WORK/GateTrackContracts.java" "$WORK/GateAudioFrames.java" \
  "$WORK/GateAudioConfiguration.java" "$WORK/GateFrameBufferFactory.java" \
  "$WORK/GateAudioFrameBuffer.java" "$WORK/GateAudioFrameRebuilder.java" \
  "$WORK/GateTerminatorAudioFrame.java" "$WORK/GateReferenceMutableAudioFrame.java" \
  "$WORK/GateAudioFrameProviderTools.java" "$WORK/GateAudioProcessingContext.java" \
  "$WORK/GateAudioPlayerOptions.java" "$WORK/GateDecodedTrackHolder.java"
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

reference_provider_tools_classpath="$classes_argument$classpath_separator$reference_argument"
while IFS= read -r dependency; do
  if command -v cygpath >/dev/null 2>&1; then
    dependency_argument="$(cygpath -w "$dependency")"
  else
    dependency_argument="$dependency"
  fi
  reference_provider_tools_classpath+="$classpath_separator$dependency_argument"
done < <(find "$(dirname "$REFERENCE_JAR")/dependencies" -maxdepth 1 -type f -name '*.jar' -print | sort)
readonly REFERENCE_PROVIDER_TOOLS_CLASSPATH="$reference_provider_tools_classpath"

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
  'marker-handler=BYPASSED,public-abstract,void(MarkerState),nested-static' \
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
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateTrackContracts \
  >"$WORK/track-contracts-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateTrackContracts \
  >"$WORK/track-contracts-candidate.txt"
cmp "$WORK/track-contracts-reference.txt" "$WORK/track-contracts-candidate.txt"
grep --fixed-strings \
  'provider=title,author,123,provider-id,uri,art,isrc;reflection=0,16,7,T,java.lang.Class<T>' \
  "$WORK/track-contracts-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioFrames \
  >"$WORK/audio-frames-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioFrames \
  >"$WORK/audio-frames-candidate.txt"
cmp "$WORK/audio-frames-reference.txt" "$WORK/audio-frames-candidate.txt"
grep --fixed-strings \
  'provider=immediate,timed,mutable,timed-mutable,exceptions;reflection=7,4,9+1,4+7+1,5+2' \
  "$WORK/audio-frames-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioConfiguration \
  >"$WORK/audio-configuration-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioConfiguration \
  >"$WORK/audio-configuration-candidate.txt"
cmp "$WORK/audio-configuration-reference.txt" "$WORK/audio-configuration-candidate.txt"
grep --fixed-strings \
  'mutation=null,clamp,format,hot-swap,factory;copy=independent;' \
  "$WORK/audio-configuration-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateFrameBufferFactory \
  >"$WORK/frame-buffer-factory-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateFrameBufferFactory \
  >"$WORK/frame-buffer-factory-candidate.txt"
cmp "$WORK/frame-buffer-factory-reference.txt" "$WORK/frame-buffer-factory-candidate.txt"
grep --fixed-strings \
  'reflection=public-abstract-interface,0-fields,1-method,0-exceptions' \
  "$WORK/frame-buffer-factory-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioFrameBuffer \
  >"$WORK/audio-frame-buffer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioFrameBuffer \
  >"$WORK/audio-frame-buffer-candidate.txt"
cmp "$WORK/audio-frame-buffer-reference.txt" "$WORK/audio-frame-buffer-candidate.txt"
grep --fixed-strings \
  'reflection=consumer-2,buffer-10,inherited-16,exceptions' \
  "$WORK/audio-frame-buffer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioFrameRebuilder \
  >"$WORK/audio-frame-rebuilder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioFrameRebuilder \
  >"$WORK/audio-frame-rebuilder-candidate.txt"
cmp "$WORK/audio-frame-rebuilder-reference.txt" "$WORK/audio-frame-rebuilder-candidate.txt"
grep --fixed-strings \
  'dispatch=frame-identity,null-identity,return-identity;' \
  "$WORK/audio-frame-rebuilder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateTerminatorAudioFrame \
  >"$WORK/terminator-audio-frame-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateTerminatorAudioFrame \
  >"$WORK/terminator-audio-frame-candidate.txt"
cmp "$WORK/terminator-audio-frame-reference.txt" \
  "$WORK/terminator-audio-frame-candidate.txt"
grep --fixed-strings \
  'singleton=stable,fresh-public;accessors=6-unsupported-null-message;' \
  "$WORK/terminator-audio-frame-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateReferenceMutableAudioFrame \
  >"$WORK/reference-mutable-audio-frame-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateReferenceMutableAudioFrame \
  >"$WORK/reference-mutable-audio-frame-candidate.txt"
cmp "$WORK/reference-mutable-audio-frame-reference.txt" \
  "$WORK/reference-mutable-audio-frame-candidate.txt"
grep --fixed-strings \
  'reference=identity,window,copy,mutation,freeze;invalid=deferred,negative,range,overflow;' \
  "$WORK/reference-mutable-audio-frame-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioFrameProviderTools \
  >"$WORK/audio-frame-provider-tools-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioFrameProviderTools \
  >"$WORK/audio-frame-provider-tools-candidate.txt"
cmp "$WORK/audio-frame-provider-tools-reference.txt" \
  "$WORK/audio-frame-provider-tools-candidate.txt"
grep --fixed-strings \
  'failures=timeout-wrap,interrupt-wrap-restore,unchecked-identity;' \
  "$WORK/audio-frame-provider-tools-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioProcessingContext \
  >"$WORK/audio-processing-context-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioProcessingContext \
  >"$WORK/audio-processing-context-candidate.txt"
cmp "$WORK/audio-processing-context-reference.txt" \
  "$WORK/audio-processing-context-candidate.txt"
grep --fixed-strings \
  'filter=snapshot,true,false;nulls=optional,configuration-npe;' \
  "$WORK/audio-processing-context-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioPlayerOptions \
  >"$WORK/audio-player-options-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioPlayerOptions \
  >"$WORK/audio-player-options-candidate.txt"
cmp "$WORK/audio-player-options-reference.txt" \
  "$WORK/audio-player-options-candidate.txt"
grep --fixed-strings \
  'defaults=100,null,null;holders=distinct,per-instance;' \
  "$WORK/audio-player-options-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateDecodedTrackHolder \
  >"$WORK/decoded-track-holder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateDecodedTrackHolder \
  >"$WORK/decoded-track-holder-candidate.txt"
cmp "$WORK/decoded-track-holder-reference.txt" \
  "$WORK/decoded-track-holder-candidate.txt"
grep --fixed-strings \
  'holder=track-identity,null;reflection=1-field,0-methods,1-constructor' \
  "$WORK/decoded-track-holder-candidate.txt" >/dev/null
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
