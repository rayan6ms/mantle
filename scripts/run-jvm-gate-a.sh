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

for consumer in smoke probe integration classloader event track-value track-enum track-contract audio-frame audio-configuration frame-buffer-factory audio-frame-buffer audio-frame-rebuilder terminator-audio-frame reference-mutable-audio-frame audio-frame-provider-tools audio-processing-context audio-player-options decoded-track-holder track-state-listener audio-output-hook audio-load-result-handler functional-result-handler audio-player-lifecycle-manager audio-player-interface default-audio-player default-audio-player-manager internal-audio-track audio-track-executor local-audio-track-executor-callback local-audio-track-executor track-marker-tracker base-audio-track primordial-audio-track-executor delegated-audio-track audio-track-info-builder abstract-audio-frame-buffer allocating-audio-frame-buffer non-allocating-audio-frame-buffer audio-source-manager-interface audio-source-managers probing-audio-source-manager local-audio-source-manager local-audio-track local-seekable-input-stream heartbeating-http-stream nico-audio-source-manager nico-audio-track default-sound-cloud-data-loader default-sound-cloud-data-reader default-sound-cloud-format-handler default-sound-cloud-playlist-loader default-sound-cloud-track-format sound-cloud-audio-source-manager sound-cloud-audio-source-manager-builder sound-cloud-audio-track sound-cloud-client-id-tracker sound-cloud-data-loader sound-cloud-data-reader sound-cloud-format-handler sound-cloud-helper; do
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
    track-state-listener) consumer_class='TrackStateListener' ;;
    audio-output-hook) consumer_class='AudioOutputHook' ;;
    audio-load-result-handler) consumer_class='AudioLoadResultHandler' ;;
    functional-result-handler) consumer_class='FunctionalResultHandler' ;;
    audio-player-lifecycle-manager) consumer_class='AudioPlayerLifecycleManager' ;;
    audio-player-interface) consumer_class='AudioPlayerInterface' ;;
    audio-player-manager-interface) consumer_class='AudioPlayerManagerInterface' ;;
    default-audio-player) consumer_class='DefaultAudioPlayer' ;;
    default-audio-player-manager) consumer_class='DefaultAudioPlayerManager' ;;
    internal-audio-track) consumer_class='InternalAudioTrack' ;;
    audio-track-executor) consumer_class='AudioTrackExecutor' ;;
    local-audio-track-executor-callback) consumer_class='LocalAudioTrackExecutorCallbacks' ;;
    local-audio-track-executor) consumer_class='LocalAudioTrackExecutor' ;;
    track-marker-tracker) consumer_class='TrackMarkerTracker' ;;
    base-audio-track) consumer_class='BaseAudioTrack' ;;
    primordial-audio-track-executor) consumer_class='PrimordialAudioTrackExecutor' ;;
    delegated-audio-track) consumer_class='DelegatedAudioTrack' ;;
    audio-track-info-builder) consumer_class='AudioTrackInfoBuilder' ;;
    abstract-audio-frame-buffer) consumer_class='AbstractAudioFrameBuffer' ;;
    allocating-audio-frame-buffer) consumer_class='AllocatingAudioFrameBuffer' ;;
    non-allocating-audio-frame-buffer) consumer_class='NonAllocatingAudioFrameBuffer' ;;
    audio-source-manager-interface) consumer_class='AudioSourceManagerInterface' ;;
    audio-source-managers) consumer_class='AudioSourceManagers' ;;
    probing-audio-source-manager) consumer_class='ProbingAudioSourceManager' ;;
    local-audio-source-manager) consumer_class='LocalAudioSourceManager' ;;
    local-audio-track) consumer_class='LocalAudioTrack' ;;
    local-seekable-input-stream) consumer_class='LocalSeekableInputStream' ;;
    heartbeating-http-stream) consumer_class='HeartbeatingHttpStream' ;;
    nico-audio-source-manager) consumer_class='NicoAudioSourceManager' ;;
    nico-audio-track) consumer_class='NicoAudioTrack' ;;
    default-sound-cloud-data-loader) consumer_class='DefaultSoundCloudDataLoader' ;;
    default-sound-cloud-data-reader) consumer_class='DefaultSoundCloudDataReader' ;;
    default-sound-cloud-format-handler) consumer_class='DefaultSoundCloudFormatHandler' ;;
    default-sound-cloud-playlist-loader) consumer_class='DefaultSoundCloudPlaylistLoader' ;;
    default-sound-cloud-track-format) consumer_class='DefaultSoundCloudTrackFormat' ;;
    sound-cloud-audio-source-manager) consumer_class='SoundCloudAudioSourceManager' ;;
    sound-cloud-audio-source-manager-builder) consumer_class='SoundCloudAudioSourceManagerBuilder' ;;
    sound-cloud-audio-track) consumer_class='SoundCloudAudioTrack' ;;
    sound-cloud-client-id-tracker) consumer_class='SoundCloudClientIdTracker' ;;
    sound-cloud-data-loader) consumer_class='SoundCloudDataLoader' ;;
    sound-cloud-data-reader) consumer_class='SoundCloudDataReader' ;;
    sound-cloud-format-handler) consumer_class='SoundCloudFormatHandler' ;;
    sound-cloud-helper) consumer_class='SoundCloudHelper' ;;
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
  "$WORK/GateAudioPlayerOptions.java" "$WORK/GateDecodedTrackHolder.java" \
  "$WORK/GateTrackStateListener.java" "$WORK/GateAudioOutputHook.java" \
  "$WORK/GateAudioLoadResultHandler.java" "$WORK/GateFunctionalResultHandler.java" \
  "$WORK/GateAudioPlayerLifecycleManager.java" "$WORK/GateAudioPlayerInterface.java" \
  "$WORK/GateAudioPlayerManagerInterface.java" "$WORK/GateDefaultAudioPlayer.java" \
  "$WORK/GateDefaultAudioPlayerManager.java" "$WORK/GateInternalAudioTrack.java" \
  "$WORK/GateAudioTrackExecutor.java" "$WORK/GateLocalAudioTrackExecutorCallbacks.java" \
  "$WORK/GateLocalAudioTrackExecutor.java" "$WORK/GateTrackMarkerTracker.java" \
  "$WORK/GateBaseAudioTrack.java" "$WORK/GatePrimordialAudioTrackExecutor.java" \
  "$WORK/GateDelegatedAudioTrack.java" "$WORK/GateAudioTrackInfoBuilder.java" \
  "$WORK/GateAbstractAudioFrameBuffer.java" "$WORK/GateAllocatingAudioFrameBuffer.java" \
  "$WORK/GateNonAllocatingAudioFrameBuffer.java" \
  "$WORK/GateAudioSourceManagerInterface.java" "$WORK/GateAudioSourceManagers.java" \
  "$WORK/GateProbingAudioSourceManager.java" "$WORK/GateLocalAudioSourceManager.java" \
  "$WORK/GateLocalAudioTrack.java" "$WORK/GateLocalSeekableInputStream.java" \
  "$WORK/GateHeartbeatingHttpStream.java" "$WORK/GateNicoAudioSourceManager.java" \
  "$WORK/GateNicoAudioTrack.java" "$WORK/GateDefaultSoundCloudDataReader.java" \
  "$WORK/GateDefaultSoundCloudFormatHandler.java" \
  "$WORK/GateDefaultSoundCloudTrackFormat.java"
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

javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$CLASSES" \
  "$WORK/GateDefaultSoundCloudDataLoader.java" \
  "$WORK/GateDefaultSoundCloudPlaylistLoader.java" \
  "$WORK/GateSoundCloudAudioSourceManager.java" \
  "$WORK/GateSoundCloudAudioSourceManagerBuilder.java" \
  "$WORK/GateSoundCloudAudioTrack.java" \
  "$WORK/GateSoundCloudClientIdTracker.java" \
  "$WORK/GateSoundCloudDataLoader.java" \
  "$WORK/GateSoundCloudDataReader.java" \
  "$WORK/GateSoundCloudFormatHandler.java" \
  "$WORK/GateSoundCloudHelper.java"

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
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateTrackStateListener \
  >"$WORK/track-state-listener-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateTrackStateListener \
  >"$WORK/track-state-listener-candidate.txt"
cmp "$WORK/track-state-listener-reference.txt" \
  "$WORK/track-state-listener-candidate.txt"
grep --fixed-strings \
  'dispatch=exception,stuck-min,nullable,stuck-max;' \
  "$WORK/track-state-listener-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioOutputHook \
  >"$WORK/audio-output-hook-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioOutputHook \
  >"$WORK/audio-output-hook-candidate.txt"
cmp "$WORK/audio-output-hook-reference.txt" \
  "$WORK/audio-output-hook-candidate.txt"
grep --fixed-strings \
  'hook=replacement,passthrough,null;factory=identity,null;' \
  "$WORK/audio-output-hook-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioLoadResultHandler \
  >"$WORK/audio-load-result-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioLoadResultHandler \
  >"$WORK/audio-load-result-handler-candidate.txt"
cmp "$WORK/audio-load-result-handler-reference.txt" \
  "$WORK/audio-load-result-handler-candidate.txt"
grep --fixed-strings \
  'dispatch=track,playlist,none,failed,nulls,ordered;' \
  "$WORK/audio-load-result-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateFunctionalResultHandler \
  >"$WORK/functional-result-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateFunctionalResultHandler \
  >"$WORK/functional-result-handler-candidate.txt"
cmp "$WORK/functional-result-handler-reference.txt" \
  "$WORK/functional-result-handler-candidate.txt"
grep --fixed-strings \
  'callbacks=nullable,exceptions-propagated;' \
  "$WORK/functional-result-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioPlayerLifecycleManager \
  >"$WORK/audio-player-lifecycle-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioPlayerLifecycleManager \
  >"$WORK/audio-player-lifecycle-manager-candidate.txt"
cmp "$WORK/audio-player-lifecycle-manager-reference.txt" \
  "$WORK/audio-player-lifecycle-manager-candidate.txt"
grep --fixed-strings \
  'schedule=fixed-rate,duplicate-cancel,restart;' \
  "$WORK/audio-player-lifecycle-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioPlayerInterface \
  >"$WORK/audio-player-interface-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioPlayerInterface \
  >"$WORK/audio-player-interface-candidate.txt"
cmp "$WORK/audio-player-interface-reference.txt" \
  "$WORK/audio-player-interface-candidate.txt"
grep --fixed-strings \
  'dispatch=track,start,volume,filter,buffer,pause,listener,cleanup,inherited-frame;' \
  "$WORK/audio-player-interface-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" GateAudioPlayerManagerInterface \
  >"$WORK/audio-player-manager-interface-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" GateAudioPlayerManagerInterface \
  >"$WORK/audio-player-manager-interface-candidate.txt"
cmp "$WORK/audio-player-manager-interface-reference.txt" \
  "$WORK/audio-player-manager-interface-candidate.txt"
grep --fixed-strings \
  'defaults=register-order,string-reference,identity-return,null-array;' \
  "$WORK/audio-player-manager-interface-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultAudioPlayer \
  >"$WORK/default-audio-player-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultAudioPlayer >"$WORK/default-audio-player-candidate.txt"
cmp "$WORK/default-audio-player-reference.txt" "$WORK/default-audio-player-candidate.txt"
grep --fixed-strings \
  'state=defaults,clamps,pause,replace,stop,destroy,cleanup;' \
  "$WORK/default-audio-player-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultAudioPlayerManager \
  >"$WORK/default-audio-player-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultAudioPlayerManager "$native" \
  >"$WORK/default-audio-player-manager-candidate.txt"
cmp "$WORK/default-audio-player-manager-reference.txt" \
  "$WORK/default-audio-player-manager-candidate.txt"
grep --fixed-strings \
  'state=defaults,identity,clamps,thresholds;source=ordered,http,readonly;' \
  "$WORK/default-audio-player-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateInternalAudioTrack \
  >"$WORK/internal-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateInternalAudioTrack \
  >"$WORK/internal-audio-track-candidate.txt"
cmp "$WORK/internal-audio-track-reference.txt" "$WORK/internal-audio-track-candidate.txt"
grep --fixed-strings \
  'dispatch=assign-true,assign-false,active,process-exception,custom;' \
  "$WORK/internal-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioTrackExecutor \
  >"$WORK/audio-track-executor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioTrackExecutor \
  >"$WORK/audio-track-executor-candidate.txt"
cmp "$WORK/audio-track-executor-reference.txt" "$WORK/audio-track-executor-candidate.txt"
grep --fixed-strings \
  'dispatch=buffer,execute,stop,position,state,markers,failed;' \
  "$WORK/audio-track-executor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateLocalAudioTrackExecutorCallbacks \
  >"$WORK/local-audio-track-executor-callback-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLocalAudioTrackExecutorCallbacks \
  >"$WORK/local-audio-track-executor-callback-candidate.txt"
cmp "$WORK/local-audio-track-executor-callback-reference.txt" \
  "$WORK/local-audio-track-executor-callback-candidate.txt"
grep --fixed-strings \
  'dispatch=read-ok,read-fail,seek-min,seek-max,seek-fail;' \
  "$WORK/local-audio-track-executor-callback-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateLocalAudioTrackExecutor \
  >"$WORK/local-audio-track-executor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLocalAudioTrackExecutor \
  >"$WORK/local-audio-track-executor-candidate.txt"
cmp "$WORK/local-audio-track-executor-reference.txt" \
  "$WORK/local-audio-track-executor-candidate.txt"
grep --fixed-strings \
  'constructor=context,buffer,factory,disposed;position=seekable,clamp,ghosting;' \
  "$WORK/local-audio-track-executor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateTrackMarkerTracker \
  >"$WORK/track-marker-tracker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTrackMarkerTracker >"$WORK/track-marker-tracker-candidate.txt"
cmp "$WORK/track-marker-tracker-reference.txt" \
  "$WORK/track-marker-tracker-candidate.txt"
grep --fixed-strings \
  'empty=remove-null;views=live,distinct,unmodifiable,generic;' \
  "$WORK/track-marker-tracker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBaseAudioTrack \
  >"$WORK/base-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBaseAudioTrack >"$WORK/base-audio-track-candidate.txt"
cmp "$WORK/base-audio-track-reference.txt" "$WORK/base-audio-track-candidate.txt"
grep --fixed-strings \
  'constructor=identity,null,primordial;metadata=identifier,seekable,duration;' \
  "$WORK/base-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePrimordialAudioTrackExecutor \
  >"$WORK/primordial-audio-track-executor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePrimordialAudioTrackExecutor \
  >"$WORK/primordial-audio-track-executor-candidate.txt"
cmp "$WORK/primordial-audio-track-executor-reference.txt" \
  "$WORK/primordial-audio-track-executor-candidate.txt"
grep --fixed-strings \
  'defaults=buffer,state,position,failed,providers,execute;stop=log,null-info;' \
  "$WORK/primordial-audio-track-executor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDelegatedAudioTrack \
  >"$WORK/delegated-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDelegatedAudioTrack >"$WORK/delegated-audio-track-candidate.txt"
cmp "$WORK/delegated-audio-track-reference.txt" \
  "$WORK/delegated-audio-track-candidate.txt"
grep --fixed-strings \
  'constructor=identity,null;fallback=duration,accurate,position;' \
  "$WORK/delegated-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioTrackInfoBuilder \
  >"$WORK/audio-track-info-builder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioTrackInfoBuilder >"$WORK/audio-track-info-builder-candidate.txt"
cmp "$WORK/audio-track-info-builder-reference.txt" \
  "$WORK/audio-track-info-builder-candidate.txt"
grep --fixed-strings \
  'empty=nulls,distinct;setters=fluent,null-retain,stream-reset;' \
  "$WORK/audio-track-info-builder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAbstractAudioFrameBuffer \
  >"$WORK/abstract-audio-frame-buffer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAbstractAudioFrameBuffer >"$WORK/abstract-audio-frame-buffer-candidate.txt"
cmp "$WORK/abstract-audio-frame-buffer-reference.txt" \
  "$WORK/abstract-audio-frame-buffer-candidate.txt"
grep --fixed-strings \
  'constructor=format,null,unique-monitor,zero-flags;' \
  "$WORK/abstract-audio-frame-buffer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAllocatingAudioFrameBuffer \
  >"$WORK/allocating-audio-frame-buffer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAllocatingAudioFrameBuffer >"$WORK/allocating-audio-frame-buffer-candidate.txt"
cmp "$WORK/allocating-audio-frame-buffer-reference.txt" \
  "$WORK/allocating-audio-frame-buffer-candidate.txt"
grep --fixed-strings \
  'constructor=capacity,format,stopping,private-layout;' \
  "$WORK/allocating-audio-frame-buffer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateNonAllocatingAudioFrameBuffer \
  >"$WORK/non-allocating-audio-frame-buffer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateNonAllocatingAudioFrameBuffer >"$WORK/non-allocating-audio-frame-buffer-candidate.txt"
cmp "$WORK/non-allocating-audio-frame-buffer-reference.txt" \
  "$WORK/non-allocating-audio-frame-buffer-candidate.txt"
grep --fixed-strings \
  'constructor=preallocation,capacity,layout;' \
  "$WORK/non-allocating-audio-frame-buffer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioSourceManagerInterface \
  >"$WORK/audio-source-manager-interface-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioSourceManagerInterface >"$WORK/audio-source-manager-interface-candidate.txt"
cmp "$WORK/audio-source-manager-interface-reference.txt" \
  "$WORK/audio-source-manager-interface-candidate.txt"
grep --fixed-strings \
  'implementation=name,load,encodable,encode,decode,shutdown,identity;' \
  "$WORK/audio-source-manager-interface-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioSourceManagers \
  >"$WORK/audio-source-managers-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioSourceManagers >"$WORK/audio-source-managers-candidate.txt"
cmp "$WORK/audio-source-managers-reference.txt" \
  "$WORK/audio-source-managers-candidate.txt"
grep --fixed-strings \
  'remote=order,defaults,custom-registry,constructor-options;' \
  "$WORK/audio-source-managers-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateProbingAudioSourceManager \
  >"$WORK/probing-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateProbingAudioSourceManager >"$WORK/probing-audio-source-manager-candidate.txt"
cmp "$WORK/probing-audio-source-manager-reference.txt" \
  "$WORK/probing-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'load=null,reference,unknown,unsupported,supported,identity;' \
  "$WORK/probing-audio-source-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateLocalAudioSourceManager \
  >"$WORK/local-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLocalAudioSourceManager >"$WORK/local-audio-source-manager-candidate.txt"
cmp "$WORK/local-audio-source-manager-reference.txt" \
  "$WORK/local-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'load=missing,directory,eligible,extension,closed,nulls;' \
  "$WORK/local-audio-source-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateLocalAudioTrack \
  >"$WORK/local-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLocalAudioTrack >"$WORK/local-audio-track-candidate.txt"
cmp "$WORK/local-audio-track-reference.txt" "$WORK/local-audio-track-candidate.txt"
grep --fixed-strings \
  'process=factory,stream,assign,delegate,close;' \
  "$WORK/local-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateLocalSeekableInputStream \
  >"$WORK/local-seekable-input-stream-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLocalSeekableInputStream >"$WORK/local-seekable-input-stream-candidate.txt"
cmp "$WORK/local-seekable-input-stream-reference.txt" \
  "$WORK/local-seekable-input-stream-candidate.txt"
grep --fixed-strings \
  'reads=single,bulk,skip,available,eof-quirk;' \
  "$WORK/local-seekable-input-stream-candidate.txt" >/dev/null
# D_LEGACY intentionally retains the shell while disabling obsolete DMC traffic.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateHeartbeatingHttpStream reference \
  >"$WORK/heartbeating-http-stream-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHeartbeatingHttpStream candidate >"$WORK/heartbeating-http-stream-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,3-exported-methods,capture,setup-dispatch,cancel,close;legacy=reference-scheduler,network-attempt' \
  "$WORK/heartbeating-http-stream-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,3-exported-methods,capture,setup-dispatch,cancel,close;legacy=retained-shell,no-scheduler,unsupported' \
  "$WORK/heartbeating-http-stream-candidate.txt" >/dev/null
# C_SEMANTIC keeps the manager/SPI shell while routing current watch metadata through Rust.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateNicoAudioSourceManager reference \
  >"$WORK/nico-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateNicoAudioSourceManager candidate "$native" \
  >"$WORK/nico-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,4-fields,2-constructors,9-exported-methods,source-name,route-filter,empty-details,decode,shutdown,http-config;service=legacy-xml-login' \
  "$WORK/nico-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,4-fields,2-constructors,9-exported-methods,source-name,route-filter,empty-details,decode,shutdown,http-config;service=current-native,no-legacy-login' \
  "$WORK/nico-audio-source-manager-candidate.txt" >/dev/null
# C_SEMANTIC retains track identity while replacing obsolete DMC/MPEG playback with current CMAF.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateNicoAudioTrack reference \
  >"$WORK/nico-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateNicoAudioTrack candidate "$native" \
  >"$WORK/nico-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,3-exported-methods,capture,source-identity,shallow-clone;service=legacy-dmc-mpeg' \
  "$WORK/nico-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,3-exported-methods,capture,source-identity,shallow-clone;service=current-native-cmaf-opus,no-legacy-dmc' \
  "$WORK/nico-audio-track-candidate.txt" >/dev/null
# A_EXACT preserves the current v2 resolve request, response parsing, and cleanup behavior.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultSoundCloudDataLoader reference \
  >"$WORK/default-sound-cloud-data-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultSoundCloudDataLoader candidate \
  >"$WORK/default-sound-cloud-data-loader-candidate.txt"
cmp "$WORK/default-sound-cloud-data-loader-reference.txt" \
  "$WORK/default-sound-cloud-data-loader-candidate.txt"
grep --fixed-strings \
  'public-concrete,0-fields,1-constructor,1-exported-method;resolve-v2,get,encoded-url,404-null-browser,json,close,status-error,suppressed-close' \
  "$WORK/default-sound-cloud-data-loader-candidate.txt" >/dev/null
# A_EXACT preserves deterministic SoundCloud JSON-to-contract mapping.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultSoundCloudDataReader reference \
  >"$WORK/default-sound-cloud-data-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultSoundCloudDataReader candidate \
  >"$WORK/default-sound-cloud-data-reader-candidate.txt"
cmp "$WORK/default-sound-cloud-data-reader-reference.txt" \
  "$WORK/default-sound-cloud-data-reader-candidate.txt"
grep --fixed-strings \
  'public-concrete,1-field,1-constructor,10-exported-methods;kind-identity,ids,policy,track-info,thumbnail,formats,format-filter-order,playlist-values,missing-quirks,generic-signatures' \
  "$WORK/default-sound-cloud-data-reader-candidate.txt" >/dev/null
# A_EXACT preserves deterministic format priority and identifier routing.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultSoundCloudFormatHandler reference \
  >"$WORK/default-sound-cloud-format-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultSoundCloudFormatHandler candidate \
  >"$WORK/default-sound-cloud-format-handler-candidate.txt"
cmp "$WORK/default-sound-cloud-format-handler-reference.txt" \
  "$WORK/default-sound-cloud-format-handler-candidate.txt"
grep --fixed-strings \
  'public-concrete,1-field,1-constructor,4-exported-methods;opus-hls-priority,mp3-hls,progressive-mp3,exact-mime,stable-order,identifier-prefixes,unknown-fallback,m3u-factories,mp3-lookup,error-quirks' \
  "$WORK/default-sound-cloud-format-handler-candidate.txt" >/dev/null
# A_EXACT preserves current v2 set loading, batching, ordering, and omission behavior.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultSoundCloudPlaylistLoader reference \
  >"$WORK/default-sound-cloud-playlist-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultSoundCloudPlaylistLoader candidate \
  >"$WORK/default-sound-cloud-playlist-loader-candidate.txt"
cmp "$WORK/default-sound-cloud-playlist-loader-reference.txt" \
  "$WORK/default-sound-cloud-playlist-loader-candidate.txt"
grep --fixed-strings \
  'public-concrete,5-exported-fields,1-constructor,5-exported-methods;url-regex,mobile-normalization,dependency-capture,track-url-encoding,stable-sort,v2-batches-of-50,response-close,playlist-order,blocked-omit,bad-track-omit,factory-metadata,http-interface-close,friendly-io-wrap,suppressed-close,generics' \
  "$WORK/default-sound-cloud-playlist-loader-candidate.txt" >/dev/null
# A_EXACT preserves the immutable format tuple without adding value semantics.
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" \
  GateDefaultSoundCloudTrackFormat reference \
  >"$WORK/default-sound-cloud-track-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$reference_argument" \
  GateDefaultSoundCloudTrackFormat candidate \
  >"$WORK/default-sound-cloud-track-format-candidate.txt"
cmp "$WORK/default-sound-cloud-track-format-reference.txt" \
  "$WORK/default-sound-cloud-track-format-candidate.txt"
grep --fixed-strings \
  'public-concrete,4-private-final-fields,1-constructor,4-methods;reference-preserving,null-preserving,no-value-overrides' \
  "$WORK/default-sound-cloud-track-format-candidate.txt" >/dev/null
# A_EXACT preserves routing, serialization, collaborators, HTTP configuration, and filtering.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudAudioSourceManager reference \
  >"$WORK/sound-cloud-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudAudioSourceManager candidate \
  >"$WORK/sound-cloud-audio-source-manager-candidate.txt"
cmp "$WORK/sound-cloud-audio-source-manager-reference.txt" \
  "$WORK/sound-cloud-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'public-concrete,27-fields,2-constructors,16-exported-methods;defaults,builder,dependency-capture,http-config,source-name,always-encodable,empty-encode,decode-owner,track-routing,playlist-fallback,load-pipeline,preview-filter,search-range-cap,liked-tracks,blocked-omit,resource-close,friendly-failures,generics' \
  "$WORK/sound-cloud-audio-source-manager-candidate.txt" >/dev/null
# A_EXACT preserves builder defaults, fluent capture, dependency precedence, and factory fallback.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudAudioSourceManagerBuilder \
  >"$WORK/sound-cloud-audio-source-manager-builder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudAudioSourceManagerBuilder \
  >"$WORK/sound-cloud-audio-source-manager-builder-candidate.txt"
cmp "$WORK/sound-cloud-audio-source-manager-builder-reference.txt" \
  "$WORK/sound-cloud-audio-source-manager-builder-candidate.txt"
grep --fixed-strings \
  'public-static,7-fields,1-constructor,8-methods;defaults,self-return,null-reset,fresh-defaults,explicit-capture,playlist-precedence,factory-order,factory-null-fallback,policy-forwarding' \
  "$WORK/sound-cloud-audio-source-manager-builder-candidate.txt" >/dev/null
# C_SEMANTIC retains track identity while replacing web-client scraping with explicit credentials.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudAudioTrack reference \
  >"$WORK/sound-cloud-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudAudioTrack candidate "$native" \
  >"$WORK/sound-cloud-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,2-fields,1-constructor,3-exported-methods,capture,source-identity,shallow-clone;service=legacy-web-client-http' \
  "$WORK/sound-cloud-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,2-fields,1-constructor,3-exported-methods,capture,source-identity,shallow-clone;service=current-native-explicit-credentials,no-client-scrape' \
  "$WORK/sound-cloud-audio-track-candidate.txt" >/dev/null
# C_SEMANTIC retains the tracker shell while replacing credential scraping with bounded input.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudClientIdTracker reference \
  >"$WORK/sound-cloud-client-id-tracker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudClientIdTracker candidate \
  >"$WORK/sound-cloud-client-id-tracker-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,11-fields,1-constructor,3-exported-methods,dependency-capture,context-marker,private-shell;service=legacy-web-client-scrape' \
  "$WORK/sound-cloud-client-id-tracker-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,11-fields,1-constructor,3-exported-methods,dependency-capture,context-marker,private-shell;service=bounded-explicit-property,no-http,no-client-scrape' \
  "$WORK/sound-cloud-client-id-tracker-candidate.txt" >/dev/null
# A_EXACT preserves the caller-defined data-loader SPI and checked failure contract.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudDataLoader \
  >"$WORK/sound-cloud-data-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudDataLoader \
  >"$WORK/sound-cloud-data-loader-candidate.txt"
cmp "$WORK/sound-cloud-data-loader-reference.txt" \
  "$WORK/sound-cloud-data-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,1-method;dispatch,argument-identity,return-identity,nulls,checked-io,reflection' \
  "$WORK/sound-cloud-data-loader-candidate.txt" >/dev/null
# A_EXACT preserves all caller-defined data-reader SPI methods and generic list contracts.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudDataReader \
  >"$WORK/sound-cloud-data-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudDataReader \
  >"$WORK/sound-cloud-data-reader-candidate.txt"
cmp "$WORK/sound-cloud-data-reader-reference.txt" \
  "$WORK/sound-cloud-data-reader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,9-methods;dispatch,argument-identity,return-identity,boolean,nulls,unchecked,generic-signatures,reflection' \
  "$WORK/sound-cloud-data-reader-candidate.txt" >/dev/null
# A_EXACT preserves all caller-defined format-handler SPI methods and the generic list contract.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudFormatHandler \
  >"$WORK/sound-cloud-format-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudFormatHandler \
  >"$WORK/sound-cloud-format-handler-candidate.txt"
cmp "$WORK/sound-cloud-format-handler-reference.txt" \
  "$WORK/sound-cloud-format-handler-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,4-methods;dispatch,argument-identity,return-identity,nulls,unchecked,generic-list-parameter,reflection' \
  "$WORK/sound-cloud-format-handler-candidate.txt" >/dev/null
# Preserve the exact pure shell while replacing legacy HTTP helpers with bounded native policy.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudHelper reference \
  >"$WORK/sound-cloud-helper-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudHelper candidate \
  >"$WORK/sound-cloud-helper-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,0-fields,1-constructor,4-static-methods,non-mobile,checked-io,reflection;service=legacy-http-playback,mobile-get,short-head' \
  "$WORK/sound-cloud-helper-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,0-fields,1-constructor,4-static-methods,non-mobile,checked-io,reflection;service=bounded-native-source,no-http,legacy-mobile-disabled,short-link-disabled' \
  "$WORK/sound-cloud-helper-candidate.txt" >/dev/null
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
