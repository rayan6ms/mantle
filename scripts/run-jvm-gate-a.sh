#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly REFERENCE_JAR="${MANTLE_REFERENCE_JAR:-$ROOT/.cache/reference/lavaplayer-2.2.6/lavaplayer-2.2.6.jar}"
readonly WORK="$ROOT/target/gate-a"
readonly CLASSES="$WORK/consumer-classes"
readonly FLAC_CLASSES="$WORK/flac-consumer-classes"
readonly MP3_CLASSES="$WORK/mp3-consumer-classes"
readonly FLAC_LOADER_CLASSES="$WORK/flac-loader-consumer-classes"
readonly FLAC_METADATA_READER_CLASSES="$WORK/flac-metadata-reader-consumer-classes"
readonly MATROSKA_CLASSES="$WORK/matroska-consumer-classes"
readonly MPEG_CLASSES="$WORK/mpeg-consumer-classes"
readonly OGG_CODEC_CLASSES="$WORK/ogg-codec-consumer-classes"
readonly OGG_FLAC_CLASSES="$WORK/ogg-flac-consumer-classes"
readonly OGG_OPUS_CLASSES="$WORK/ogg-opus-consumer-classes"
readonly OGG_VORBIS_CLASSES="$WORK/ogg-vorbis-consumer-classes"
readonly OGG_VORBIS_TRACK_CLASSES="$WORK/ogg-vorbis-track-consumer-classes"
readonly OGG_VORBIS_TRACK_SOURCES="$WORK/ogg-vorbis-track-sources"
readonly OGG_PROBE_CLASSES="$WORK/ogg-probe-consumer-classes"
readonly MPEG_FILE_LOADER_CLASSES="$WORK/mpeg-file-loader-consumer-classes"
readonly JAR="$WORK/mantle-gate-a.jar"
readonly MISMATCH_JAR="$WORK/mantle-gate-a-mismatch.jar"

if [[ ! -f "$REFERENCE_JAR" ]]; then
  printf 'Gate A reference JAR not found: %s\n' "$REFERENCE_JAR" >&2
  exit 1
fi

rm -rf -- "$CLASSES" "$FLAC_CLASSES" "$MP3_CLASSES" "$FLAC_LOADER_CLASSES" \
  "$FLAC_METADATA_READER_CLASSES" "$MATROSKA_CLASSES" "$MPEG_CLASSES" \
  "$MPEG_FILE_LOADER_CLASSES" "$OGG_CODEC_CLASSES" "$OGG_FLAC_CLASSES" \
  "$OGG_OPUS_CLASSES" "$OGG_VORBIS_CLASSES" "$OGG_VORBIS_TRACK_CLASSES" \
  "$OGG_VORBIS_TRACK_SOURCES" "$OGG_PROBE_CLASSES"
mkdir -p "$CLASSES" "$FLAC_CLASSES" "$MP3_CLASSES" "$FLAC_LOADER_CLASSES" \
  "$FLAC_METADATA_READER_CLASSES" "$MATROSKA_CLASSES" "$MPEG_CLASSES" \
  "$MPEG_FILE_LOADER_CLASSES" "$OGG_CODEC_CLASSES" "$OGG_FLAC_CLASSES" \
  "$OGG_OPUS_CLASSES" "$OGG_VORBIS_CLASSES" "$OGG_VORBIS_TRACK_CLASSES" \
  "$OGG_VORBIS_TRACK_SOURCES" "$OGG_PROBE_CLASSES"
cargo build --locked -p mantle-jvm --features gate-a-direct-attachment
cargo run --locked -q -p mantle-jvm-gate -- emit \
  --reference-jar "$REFERENCE_JAR" --output "$JAR" --expected-abi 1 \
  --manifest-output "$WORK/emission-manifest.json"
cargo run --locked -q -p mantle-jvm-gate -- verify-structure \
  --reference-jar "$REFERENCE_JAR" --candidate-jar "$JAR"

for consumer in smoke probe integration classloader event track-value track-enum track-contract audio-frame audio-configuration frame-buffer-factory audio-frame-buffer audio-frame-rebuilder terminator-audio-frame reference-mutable-audio-frame audio-frame-provider-tools audio-processing-context audio-player-options decoded-track-holder track-state-listener audio-output-hook audio-load-result-handler functional-result-handler audio-player-lifecycle-manager audio-player-interface default-audio-player default-audio-player-manager internal-audio-track audio-track-executor local-audio-track-executor-callback local-audio-track-executor track-marker-tracker base-audio-track primordial-audio-track-executor delegated-audio-track audio-track-info-builder abstract-audio-frame-buffer allocating-audio-frame-buffer non-allocating-audio-frame-buffer audio-filter-interface float-pcm-audio-filter short-pcm-audio-filter universal-pcm-audio-filter user-provided-audio-filters converter-audio-filter to-float-audio-filter to-short-audio-filter to-split-short-audio-filter equalizer volume audio-data-format audio-data-format-tools pcm-filter-factory pcm-format resampling-pcm-audio-filter audio-post-processor buffering-post-processor channel-count-pcm-audio-filter composite-audio-filter filter-chain-builder final-pcm-audio-filter audio-filter-chain audio-pipeline audio-pipeline-factory audio-source-manager-interface audio-source-managers probing-audio-source-manager local-audio-source-manager local-audio-track local-seekable-input-stream heartbeating-http-stream nico-audio-source-manager nico-audio-track default-sound-cloud-data-loader default-sound-cloud-data-reader default-sound-cloud-format-handler default-sound-cloud-playlist-loader default-sound-cloud-track-format sound-cloud-audio-source-manager sound-cloud-audio-source-manager-builder sound-cloud-audio-track sound-cloud-client-id-tracker sound-cloud-data-loader sound-cloud-data-reader sound-cloud-format-handler sound-cloud-helper sound-cloud-http-context-filter sound-cloud-m3u-audio-track sound-cloud-m3u-info sound-cloud-mp3-segment-decoder sound-cloud-opus-segment-decoder sound-cloud-playlist-loader sound-cloud-segment-decoder sound-cloud-segment-decoder-factory sound-cloud-track-format m3u-stream-audio-track m3u-stream-segment-url-provider mpeg-ts-m3u-stream-audio-track twitch-constants twitch-stream-audio-source-manager twitch-stream-audio-track twitch-stream-segment-url-provider bandcamp-audio-source-manager bandcamp-audio-track beam-audio-source-manager beam-audio-track beam-segment-url-provider getyarn-audio-source-manager getyarn-audio-track http-audio-source-manager http-audio-track vimeo-audio-source-manager vimeo-playback-format vimeo-audio-track abstract-yandex-music-api-loader yandex-music-api-extractor default-yandex-music-direct-url-loader default-yandex-music-playlist-loader default-yandex-music-track-loader default-yandex-search-provider yandex-http-context-filter yandex-music-api-loader yandex-music-audio-source-manager yandex-music-audio-track yandex-music-direct-url-loader yandex-music-playlist-loader yandex-music-search-result-loader yandex-music-track-loader yandex-music-utils default-youtube-link-router default-youtube-playlist-loader default-youtube-track-details default-youtube-track-details-loader youtube-cached-player-script youtube-info-status youtube-access-token-tracker youtube-cached-auth-script youtube-audio-source-manager youtube-audio-track youtube-cipher-operation youtube-client-config youtube-constants youtube-format-info youtube-http-context-filter youtube-link-router youtube-mix-loader youtube-mix-provider youtube-mpeg-stream-audio-track youtube-payload-helper youtube-persistent-http-stream youtube-playlist-loader youtube-search-music-provider youtube-search-music-result-loader youtube-search-provider youtube-search-result-loader youtube-signature-cipher youtube-signature-cipher-manager youtube-signature-resolver youtube-track-details youtube-track-details-loader; do
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
    audio-filter-interface) consumer_class='AudioFilterInterface' ;;
    float-pcm-audio-filter) consumer_class='FloatPcmAudioFilter' ;;
    short-pcm-audio-filter) consumer_class='ShortPcmAudioFilter' ;;
    universal-pcm-audio-filter) consumer_class='UniversalPcmAudioFilter' ;;
    user-provided-audio-filters) consumer_class='UserProvidedAudioFilters' ;;
    converter-audio-filter) consumer_class='ConverterAudioFilter' ;;
    to-float-audio-filter) consumer_class='ToFloatAudioFilter' ;;
    to-short-audio-filter) consumer_class='ToShortAudioFilter' ;;
    to-split-short-audio-filter) consumer_class='ToSplitShortAudioFilter' ;;
    equalizer) consumer_class='Equalizer' ;;
    volume) consumer_class='Volume' ;;
    audio-data-format) consumer_class='AudioDataFormat' ;;
    audio-data-format-tools) consumer_class='AudioDataFormatTools' ;;
    pcm-filter-factory) consumer_class='PcmFilterFactory' ;;
    pcm-format) consumer_class='PcmFormat' ;;
    resampling-pcm-audio-filter) consumer_class='ResamplingPcmAudioFilter' ;;
    audio-post-processor) consumer_class='AudioPostProcessor' ;;
    buffering-post-processor) consumer_class='BufferingPostProcessor' ;;
    channel-count-pcm-audio-filter) consumer_class='ChannelCountPcmAudioFilter' ;;
    composite-audio-filter) consumer_class='CompositeAudioFilter' ;;
    filter-chain-builder) consumer_class='FilterChainBuilder' ;;
    final-pcm-audio-filter) consumer_class='FinalPcmAudioFilter' ;;
    audio-filter-chain) consumer_class='AudioFilterChain' ;;
    audio-pipeline) consumer_class='AudioPipeline' ;;
    audio-pipeline-factory) consumer_class='AudioPipelineFactory' ;;
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
    sound-cloud-http-context-filter) consumer_class='SoundCloudHttpContextFilter' ;;
    sound-cloud-m3u-audio-track) consumer_class='SoundCloudM3uAudioTrack' ;;
    sound-cloud-m3u-info) consumer_class='SoundCloudM3uInfo' ;;
    sound-cloud-mp3-segment-decoder) consumer_class='SoundCloudMp3SegmentDecoder' ;;
    sound-cloud-opus-segment-decoder) consumer_class='SoundCloudOpusSegmentDecoder' ;;
    sound-cloud-playlist-loader) consumer_class='SoundCloudPlaylistLoader' ;;
    sound-cloud-segment-decoder) consumer_class='SoundCloudSegmentDecoder' ;;
    sound-cloud-segment-decoder-factory) consumer_class='SoundCloudSegmentDecoderFactory' ;;
    sound-cloud-track-format) consumer_class='SoundCloudTrackFormat' ;;
    m3u-stream-audio-track) consumer_class='M3uStreamAudioTrack' ;;
    m3u-stream-segment-url-provider) consumer_class='M3uStreamSegmentUrlProvider' ;;
    mpeg-ts-m3u-stream-audio-track) consumer_class='MpegTsM3uStreamAudioTrack' ;;
    twitch-constants) consumer_class='TwitchConstants' ;;
    twitch-stream-audio-source-manager) consumer_class='TwitchStreamAudioSourceManager' ;;
    twitch-stream-audio-track) consumer_class='TwitchStreamAudioTrack' ;;
    twitch-stream-segment-url-provider) consumer_class='TwitchStreamSegmentUrlProvider' ;;
    bandcamp-audio-source-manager) consumer_class='BandcampAudioSourceManager' ;;
    bandcamp-audio-track) consumer_class='BandcampAudioTrack' ;;
    beam-audio-source-manager) consumer_class='BeamAudioSourceManager' ;;
    beam-audio-track) consumer_class='BeamAudioTrack' ;;
    beam-segment-url-provider) consumer_class='BeamSegmentUrlProvider' ;;
    getyarn-audio-source-manager) consumer_class='GetyarnAudioSourceManager' ;;
    getyarn-audio-track) consumer_class='GetyarnAudioTrack' ;;
    http-audio-source-manager) consumer_class='HttpAudioSourceManager' ;;
    http-audio-track) consumer_class='HttpAudioTrack' ;;
    vimeo-audio-source-manager) consumer_class='VimeoAudioSourceManager' ;;
    vimeo-playback-format) consumer_class='VimeoPlaybackFormat' ;;
    vimeo-audio-track) consumer_class='VimeoAudioTrack' ;;
    abstract-yandex-music-api-loader) consumer_class='AbstractYandexMusicApiLoader' ;;
    yandex-music-api-extractor) consumer_class='YandexMusicApiExtractor' ;;
    default-yandex-music-direct-url-loader) consumer_class='DefaultYandexMusicDirectUrlLoader' ;;
    default-yandex-music-playlist-loader) consumer_class='DefaultYandexMusicPlaylistLoader' ;;
    default-yandex-music-track-loader) consumer_class='DefaultYandexMusicTrackLoader' ;;
    default-yandex-search-provider) consumer_class='DefaultYandexSearchProvider' ;;
    yandex-http-context-filter) consumer_class='YandexHttpContextFilter' ;;
    yandex-music-api-loader) consumer_class='YandexMusicApiLoader' ;;
    yandex-music-audio-source-manager) consumer_class='YandexMusicAudioSourceManager' ;;
    yandex-music-audio-track) consumer_class='YandexMusicAudioTrack' ;;
    yandex-music-direct-url-loader) consumer_class='YandexMusicDirectUrlLoader' ;;
    yandex-music-playlist-loader) consumer_class='YandexMusicPlaylistLoader' ;;
    yandex-music-search-result-loader) consumer_class='YandexMusicSearchResultLoader' ;;
    yandex-music-track-loader) consumer_class='YandexMusicTrackLoader' ;;
    yandex-music-utils) consumer_class='YandexMusicUtils' ;;
    default-youtube-link-router) consumer_class='DefaultYoutubeLinkRouter' ;;
    default-youtube-playlist-loader) consumer_class='DefaultYoutubePlaylistLoader' ;;
    default-youtube-track-details) consumer_class='DefaultYoutubeTrackDetails' ;;
    default-youtube-track-details-loader) consumer_class='DefaultYoutubeTrackDetailsLoader' ;;
    youtube-cached-player-script) consumer_class='YoutubeCachedPlayerScript' ;;
    youtube-info-status) consumer_class='YoutubeInfoStatus' ;;
    youtube-access-token-tracker) consumer_class='YoutubeAccessTokenTracker' ;;
    youtube-cached-auth-script) consumer_class='YoutubeCachedAuthScript' ;;
    youtube-audio-source-manager) consumer_class='YoutubeAudioSourceManager' ;;
    youtube-audio-track) consumer_class='YoutubeAudioTrack' ;;
    youtube-cipher-operation) consumer_class='YoutubeCipherOperation' ;;
    youtube-client-config) consumer_class='YoutubeClientConfig' ;;
    youtube-constants) consumer_class='YoutubeConstants' ;;
    youtube-format-info) consumer_class='YoutubeFormatInfo' ;;
    youtube-http-context-filter) consumer_class='YoutubeHttpContextFilter' ;;
    youtube-link-router) consumer_class='YoutubeLinkRouter' ;;
    youtube-mix-loader) consumer_class='YoutubeMixLoader' ;;
    youtube-mix-provider) consumer_class='YoutubeMixProvider' ;;
    youtube-mpeg-stream-audio-track) consumer_class='YoutubeMpegStreamAudioTrack' ;;
    youtube-payload-helper) consumer_class='YoutubePayloadHelper' ;;
    youtube-persistent-http-stream) consumer_class='YoutubePersistentHttpStream' ;;
    youtube-playlist-loader) consumer_class='YoutubePlaylistLoader' ;;
    youtube-search-music-provider) consumer_class='YoutubeSearchMusicProvider' ;;
    youtube-search-music-result-loader) consumer_class='YoutubeSearchMusicResultLoader' ;;
    youtube-search-provider) consumer_class='YoutubeSearchProvider' ;;
    youtube-search-result-loader) consumer_class='YoutubeSearchResultLoader' ;;
    youtube-signature-cipher) consumer_class='YoutubeSignatureCipher' ;;
    youtube-signature-cipher-manager) consumer_class='YoutubeSignatureCipherManager' ;;
    youtube-signature-resolver) consumer_class='YoutubeSignatureResolver' ;;
    youtube-track-details) consumer_class='YoutubeTrackDetails' ;;
    youtube-track-details-loader) consumer_class='YoutubeTrackDetailsLoader' ;;
  esac
  cargo run --locked -q -p mantle-jvm-gate -- "write-$consumer-consumer" \
    --output "$WORK/Gate${consumer_class}.java"
done
cargo run --locked -q -p mantle-jvm-gate -- write-audio-player-input-stream-consumer \
  --output "$WORK/GateAudioPlayerInputStream.java"
cargo run --locked -q -p mantle-jvm-gate -- write-opus-audio-data-format-consumer \
  --output "$WORK/GateOpusAudioDataFormat.java"
cargo run --locked -q -p mantle-jvm-gate -- write-pcm16-audio-data-format-consumer \
  --output "$WORK/GatePcm16AudioDataFormat.java"
cargo run --locked -q -p mantle-jvm-gate -- write-standard-audio-data-formats-consumer \
  --output "$WORK/GateStandardAudioDataFormats.java"
cargo run --locked -q -p mantle-jvm-gate -- write-audio-chunk-decoder-consumer \
  --output "$WORK/GateAudioChunkDecoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-audio-chunk-encoder-consumer \
  --output "$WORK/GateAudioChunkEncoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-opus-chunk-decoder-consumer \
  --output "$WORK/GateOpusChunkDecoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-opus-chunk-encoder-consumer \
  --output "$WORK/GateOpusChunkEncoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-pcm-chunk-decoder-consumer \
  --output "$WORK/GatePcmChunkDecoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-pcm-chunk-encoder-consumer \
  --output "$WORK/GatePcmChunkEncoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-formats-consumer \
  --output "$WORK/GateFormats.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-consumer \
  --output "$WORK/GateMediaContainer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-descriptor-consumer \
  --output "$WORK/GateMediaContainerDescriptor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-detection-consumer \
  --output "$WORK/GateMediaContainerDetection.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-detection-result-consumer \
  --output "$WORK/GateMediaContainerDetectionResult.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-hints-consumer \
  --output "$WORK/GateMediaContainerHints.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-probe-consumer \
  --output "$WORK/GateMediaContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-media-container-registry-consumer \
  --output "$WORK/GateMediaContainerRegistry.java"
cargo run --locked -q -p mantle-jvm-gate -- write-adts-audio-track-consumer \
  --output "$WORK/GateAdtsAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-audio-track-consumer \
  --output "$WORK/GateMp3AudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-audio-track-support-consumer \
  --output "$WORK/Mp3GateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-constant-rate-seeker-consumer \
  --output "$WORK/GateMp3ConstantRateSeeker.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-container-probe-consumer \
  --output "$WORK/GateMp3ContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-frame-reader-consumer \
  --output "$WORK/GateMp3FrameReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-seeker-consumer \
  --output "$WORK/GateMp3Seeker.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-stream-seeker-consumer \
  --output "$WORK/GateMp3StreamSeeker.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-track-provider-consumer \
  --output "$WORK/GateMp3TrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mp3-xing-seeker-consumer \
  --output "$WORK/GateMp3XingSeeker.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-aac-track-consumer \
  --output "$WORK/GateMpegAacTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-audio-track-consumer \
  --output "$WORK/GateMpegAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-audio-track-support-consumer \
  --output "$WORK/MpegGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-container-probe-consumer \
  --output "$WORK/GateMpegContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-adts-container-probe-consumer \
  --output "$WORK/GateMpegAdtsContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-ts-elementary-input-stream-consumer \
  --output "$WORK/GateMpegTsElementaryInputStream.java"
cargo run --locked -q -p mantle-jvm-gate -- write-pes-packet-input-stream-consumer \
  --output "$WORK/GatePesPacketInputStream.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-audio-track-consumer \
  --output "$WORK/GateOggAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-audio-track-support-consumer \
  --output "$WORK/OggGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-codec-handler-consumer \
  --output "$WORK/GateOggCodecHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-container-probe-consumer \
  --output "$WORK/GateOggContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-metadata-consumer \
  --output "$WORK/GateOggMetadata.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-packet-input-stream-consumer \
  --output "$WORK/GateOggPacketInputStream.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-page-header-consumer \
  --output "$WORK/GateOggPageHeader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-page-scanner-consumer \
  --output "$WORK/GateOggPageScanner.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-seek-point-consumer \
  --output "$WORK/GateOggSeekPoint.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-stream-size-info-consumer \
  --output "$WORK/GateOggStreamSizeInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-track-blueprint-consumer \
  --output "$WORK/GateOggTrackBlueprint.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-track-handler-consumer \
  --output "$WORK/GateOggTrackHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-track-loader-consumer \
  --output "$WORK/GateOggTrackLoader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-codec-handler-consumer \
  --output "$WORK/GateOggFlacCodecHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-opus-codec-handler-consumer \
  --output "$WORK/GateOggOpusCodecHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-opus-track-handler-consumer \
  --output "$WORK/GateOggOpusTrackHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-opus-router-support-consumer \
  --output "$WORK/OpusPacketRouter.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-codec-handler-consumer \
  --output "$WORK/GateOggVorbisCodecHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-extended-m3u-parser-consumer \
  --output "$WORK/GateExtendedM3uParser.java"
cargo run --locked -q -p mantle-jvm-gate -- write-hls-stream-segment-consumer \
  --output "$WORK/GateHlsStreamSegment.java"
cargo run --locked -q -p mantle-jvm-gate -- write-hls-stream-segment-parser-consumer \
  --output "$WORK/GateHlsStreamSegmentParser.java"
cargo run --locked -q -p mantle-jvm-gate -- write-hls-stream-segment-url-provider-consumer \
  --output "$WORK/GateHlsStreamSegmentUrlProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-hls-stream-track-consumer \
  --output "$WORK/GateHlsStreamTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-m3u-playlist-container-probe-consumer \
  --output "$WORK/GateM3uPlaylistContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-plain-playlist-container-probe-consumer \
  --output "$WORK/GatePlainPlaylistContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-pls-playlist-container-probe-consumer \
  --output "$WORK/GatePlsPlaylistContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-vorbis-comment-parser-consumer \
  --output "$WORK/GateVorbisCommentParser.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-track-handler-support-consumer \
  --output "$WORK/OggVorbisTrackHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-track-consumer \
  --output "$OGG_VORBIS_TRACK_SOURCES/GateOggVorbisTrackHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-decoder-support-consumer \
  --output "$OGG_VORBIS_TRACK_SOURCES/VorbisDecoder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-pipeline-support-consumer \
  --output "$OGG_VORBIS_TRACK_SOURCES/AudioPipeline.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-pipeline-factory-support-consumer \
  --output "$OGG_VORBIS_TRACK_SOURCES/AudioPipelineFactory.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-vorbis-pcm-format-support-consumer \
  --output "$OGG_VORBIS_TRACK_SOURCES/PcmFormat.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-track-handler-consumer \
  --output "$WORK/GateOggFlacTrackHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-track-handler-support-consumer \
  --output "$WORK/OggPacketInputStream.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-pipeline-support-consumer \
  --output "$WORK/AudioPipeline.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-pipeline-factory-support-consumer \
  --output "$WORK/AudioPipelineFactory.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-pcm-format-support-consumer \
  --output "$WORK/PcmFormat.java"
cargo run --locked -q -p mantle-jvm-gate -- write-ogg-flac-frame-reader-support-consumer \
  --output "$WORK/FlacFrameReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-file-loader-consumer \
  --output "$WORK/GateMpegFileLoader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-track-info-consumer \
  --output "$WORK/GateMpegTrackInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-track-info-builder-consumer \
  --output "$WORK/GateMpegTrackInfoBuilder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-file-track-provider-consumer \
  --output "$WORK/GateMpegFileTrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-parse-stop-checker-consumer \
  --output "$WORK/GateMpegParseStopChecker.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-reader-consumer \
  --output "$WORK/GateMpegReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-reader-chain-consumer \
  --output "$WORK/GateMpegReaderChain.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-section-handler-consumer \
  --output "$WORK/GateMpegSectionHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-section-info-consumer \
  --output "$WORK/GateMpegSectionInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-versioned-section-handler-consumer \
  --output "$WORK/GateMpegVersionedSectionHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-versioned-section-info-consumer \
  --output "$WORK/GateMpegVersionedSectionInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-fragmented-file-track-provider-consumer \
  --output "$WORK/GateMpegFragmentedFileTrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-global-seek-info-consumer \
  --output "$WORK/GateMpegGlobalSeekInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-segment-entry-consumer \
  --output "$WORK/GateMpegSegmentEntry.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-track-fragment-header-consumer \
  --output "$WORK/GateMpegTrackFragmentHeader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-standard-file-track-provider-consumer \
  --output "$WORK/GateMpegStandardFileTrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-noop-track-consumer-consumer \
  --output "$WORK/GateMpegNoopTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-mpeg-track-consumer-consumer \
  --output "$WORK/GateMpegTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-adts-container-probe-consumer \
  --output "$WORK/GateAdtsContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-adts-packet-header-consumer \
  --output "$WORK/GateAdtsPacketHeader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-adts-stream-provider-consumer \
  --output "$WORK/GateAdtsStreamProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-adts-stream-reader-consumer \
  --output "$WORK/GateAdtsStreamReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-aac-packet-router-consumer \
  --output "$WORK/GateAacPacketRouter.java"
cargo run --locked -q -p mantle-jvm-gate -- write-opus-packet-router-consumer \
  --output "$WORK/GateOpusPacketRouter.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-audio-track-consumer \
  --output "$WORK/GateFlacAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-audio-track-support-consumer \
  --output "$WORK/FlacGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-audio-track-consumer \
  --output "$WORK/GateWavAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-audio-track-support-consumer \
  --output "$WORK/WavGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-container-probe-consumer \
  --output "$WORK/GateWavContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-file-info-consumer \
  --output "$WORK/GateWavFileInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-file-loader-consumer \
  --output "$WORK/GateWavFileLoader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wav-track-provider-consumer \
  --output "$WORK/GateWavTrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-wave-format-type-consumer \
  --output "$WORK/GateWaveFormatType.java"
cargo run --locked -q -p mantle-jvm-gate -- write-copy-on-update-identity-list-consumer \
  --output "$WORK/GateCopyOnUpdateIdentityList.java"
cargo run --locked -q -p mantle-jvm-gate -- write-data-format-tools-consumer \
  --output "$WORK/GateDataFormatTools.java"
cargo run --locked -q -p mantle-jvm-gate -- write-data-format-tools-text-range-consumer \
  --output "$WORK/GateDataFormatToolsTextRange.java"
cargo run --locked -q -p mantle-jvm-gate -- write-decoded-exception-consumer \
  --output "$WORK/GateDecodedException.java"
cargo run --locked -q -p mantle-jvm-gate -- write-exception-tools-consumer \
  --output "$WORK/GateExceptionTools.java"
cargo run --locked -q -p mantle-jvm-gate -- write-exception-tools-default-error-debug-info-handler-consumer \
  --output "$WORK/GateExceptionToolsDefaultErrorDebugInfoHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-exception-tools-error-debug-info-consumer \
  --output "$WORK/GateExceptionToolsErrorDebugInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-exception-tools-error-debug-info-handler-consumer \
  --output "$WORK/GateExceptionToolsErrorDebugInfoHandler.java"
cargo run --locked -q -p mantle-jvm-gate -- write-friendly-exception-consumer \
  --output "$WORK/GateFriendlyException.java"
cargo run --locked -q -p mantle-jvm-gate -- write-friendly-exception-severity-consumer \
  --output "$WORK/GateFriendlyExceptionSeverity.java"
cargo run --locked -q -p mantle-jvm-gate -- write-future-tools-consumer \
  --output "$WORK/GateFutureTools.java"
cargo run --locked -q -p mantle-jvm-gate -- write-garbage-collection-monitor-consumer \
  --output "$WORK/GateGarbageCollectionMonitor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-container-probe-consumer \
  --output "$WORK/GateFlacContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-file-loader-consumer \
  --output "$WORK/GateFlacFileLoader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-file-loader-support-consumer \
  --output "$WORK/FlacLoaderGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-metadata-header-consumer \
  --output "$WORK/GateFlacMetadataHeader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-metadata-reader-consumer \
  --output "$WORK/GateFlacMetadataReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-metadata-reader-support-consumer \
  --output "$WORK/FlacMetadataReaderGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-seek-point-consumer \
  --output "$WORK/GateFlacSeekPoint.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-stream-info-consumer \
  --output "$WORK/GateFlacStreamInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-track-info-consumer \
  --output "$WORK/GateFlacTrackInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-track-info-builder-consumer \
  --output "$WORK/GateFlacTrackInfoBuilder.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-track-provider-consumer \
  --output "$WORK/GateFlacTrackProvider.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-frame-header-reader-consumer \
  --output "$WORK/GateFlacFrameHeaderReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-frame-info-consumer \
  --output "$WORK/GateFlacFrameInfo.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-frame-reader-consumer \
  --output "$WORK/GateFlacFrameReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-flac-sub-frame-reader-consumer \
  --output "$WORK/GateFlacSubFrameReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-aac-track-consumer-consumer \
  --output "$WORK/GateMatroskaAacTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-opus-track-consumer-consumer \
  --output "$WORK/GateMatroskaOpusTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-track-consumer-consumer \
  --output "$WORK/GateMatroskaTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-vorbis-track-consumer-consumer \
  --output "$WORK/GateMatroskaVorbisTrackConsumer.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-block-consumer \
  --output "$WORK/GateMatroskaBlock.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-cue-point-consumer \
  --output "$WORK/GateMatroskaCuePoint.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-ebml-reader-consumer \
  --output "$WORK/GateMatroskaEbmlReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-element-consumer \
  --output "$WORK/GateMatroskaElement.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-element-type-consumer \
  --output "$WORK/GateMatroskaElementType.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-file-reader-consumer \
  --output "$WORK/GateMatroskaFileReader.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-file-track-consumer \
  --output "$WORK/GateMatroskaFileTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-mutable-element-consumer \
  --output "$WORK/GateMatroskaMutableElement.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-streaming-file-consumer \
  --output "$WORK/GateMatroskaStreamingFile.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-audio-track-consumer \
  --output "$WORK/GateMatroskaAudioTrack.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-audio-track-support-consumer \
  --output "$WORK/MatroskaGateSupport.java"
cargo run --locked -q -p mantle-jvm-gate -- write-matroska-container-probe-consumer \
  --output "$WORK/GateMatroskaContainerProbe.java"
cargo run --locked -q -p mantle-jvm-gate -- write-youtube-track-format-consumer \
  --output "$WORK/GateYoutubeTrackFormat.java"
cargo run --locked -q -p mantle-jvm-gate -- write-youtube-track-json-data-consumer \
  --output "$WORK/GateYoutubeTrackJsonData.java"
cargo run --locked -q -p mantle-jvm-gate -- write-legacy-adaptive-formats-extractor-consumer \
  --output "$WORK/GateLegacyAdaptiveFormatsExtractor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-legacy-dash-mpd-formats-extractor-consumer \
  --output "$WORK/GateLegacyDashMpdFormatsExtractor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-legacy-stream-map-formats-extractor-consumer \
  --output "$WORK/GateLegacyStreamMapFormatsExtractor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-offline-youtube-track-format-extractor-consumer \
  --output "$WORK/GateOfflineYoutubeTrackFormatExtractor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-streaming-data-formats-extractor-consumer \
  --output "$WORK/GateStreamingDataFormatsExtractor.java"
cargo run --locked -q -p mantle-jvm-gate -- write-youtube-track-format-extractor-consumer \
  --output "$WORK/GateYoutubeTrackFormatExtractor.java"

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
  "$WORK/GateCopyOnUpdateIdentityList.java" \
  "$WORK/GateDataFormatTools.java" \
  "$WORK/GateDataFormatToolsTextRange.java" \
  "$WORK/GateAudioFilterInterface.java" \
  "$WORK/GateFloatPcmAudioFilter.java" \
  "$WORK/GateShortPcmAudioFilter.java" \
  "$WORK/GateUniversalPcmAudioFilter.java" \
  "$WORK/GateUserProvidedAudioFilters.java" \
  "$WORK/GateConverterAudioFilter.java" \
  "$WORK/GateToFloatAudioFilter.java" \
  "$WORK/GateToShortAudioFilter.java" \
  "$WORK/GateToSplitShortAudioFilter.java" \
  "$WORK/GateEqualizer.java" \
  "$WORK/GateVolume.java" \
  "$WORK/GateAudioDataFormat.java" \
  "$WORK/GateAudioDataFormatTools.java" \
  "$WORK/GateAudioPlayerInputStream.java" \
  "$WORK/GateOpusAudioDataFormat.java" \
  "$WORK/GatePcm16AudioDataFormat.java" \
  "$WORK/GateStandardAudioDataFormats.java" \
  "$WORK/GateAudioChunkDecoder.java" \
  "$WORK/GateAudioChunkEncoder.java" \
  "$WORK/GateOpusChunkDecoder.java" \
  "$WORK/GateOpusChunkEncoder.java" \
  "$WORK/GatePcmChunkDecoder.java" \
  "$WORK/GatePcmChunkEncoder.java" \
  "$WORK/GateFormats.java" \
  "$WORK/GateMediaContainer.java" \
  "$WORK/GateMediaContainerDescriptor.java" \
  "$WORK/GateMediaContainerDetection.java" \
  "$WORK/GateMediaContainerDetectionResult.java" \
  "$WORK/GateMediaContainerHints.java" \
  "$WORK/GateMediaContainerProbe.java" \
  "$WORK/GateMediaContainerRegistry.java" \
  "$WORK/GateAdtsAudioTrack.java" \
  "$WORK/GateFlacMetadataHeader.java" \
  "$WORK/GateFlacSeekPoint.java" \
  "$WORK/GateFlacStreamInfo.java" \
  "$WORK/GateFlacTrackInfo.java" \
  "$WORK/GateFlacTrackInfoBuilder.java" \
  "$WORK/GateFlacTrackProvider.java" \
  "$WORK/GateFlacFrameHeaderReader.java" \
  "$WORK/GateFlacFrameInfo.java" \
  "$WORK/GateFlacFrameReader.java" \
  "$WORK/GateFlacSubFrameReader.java" \
  "$WORK/GateMp3ContainerProbe.java" \
  "$WORK/GateMp3FrameReader.java" \
  "$WORK/GateMp3Seeker.java" \
  "$WORK/GateMp3StreamSeeker.java" \
  "$WORK/GateMp3TrackProvider.java" \
  "$WORK/GateMp3XingSeeker.java" \
  "$WORK/GateMpegNoopTrackConsumer.java" \
  "$WORK/GateMpegTrackInfo.java" \
  "$WORK/GateMpegTrackInfoBuilder.java" \
  "$WORK/GateMpegTrackConsumer.java" \
  "$WORK/GateMpegFileTrackProvider.java" \
  "$WORK/GateMpegParseStopChecker.java" \
  "$WORK/GateMpegReader.java" \
  "$WORK/GateMpegReaderChain.java" \
  "$WORK/GateMpegSectionHandler.java" \
  "$WORK/GateMpegSectionInfo.java" \
  "$WORK/GateMpegVersionedSectionHandler.java" \
  "$WORK/GateMpegVersionedSectionInfo.java" \
  "$WORK/GateMpegFragmentedFileTrackProvider.java" \
  "$WORK/GateMpegGlobalSeekInfo.java" \
  "$WORK/GateMpegSegmentEntry.java" \
  "$WORK/GateMpegTrackFragmentHeader.java" \
  "$WORK/GateMpegStandardFileTrackProvider.java" \
  "$WORK/GateAdtsContainerProbe.java" \
  "$WORK/GateAdtsPacketHeader.java" \
  "$WORK/GateAdtsStreamReader.java" \
  "$WORK/GatePcmFilterFactory.java" \
  "$WORK/GatePcmFormat.java" \
  "$WORK/GateResamplingPcmAudioFilter.java" \
  "$WORK/GateAudioPostProcessor.java" \
  "$WORK/GateBufferingPostProcessor.java" \
  "$WORK/GateChannelCountPcmAudioFilter.java" \
  "$WORK/GateCompositeAudioFilter.java" \
  "$WORK/GateFilterChainBuilder.java" \
  "$WORK/GateFinalPcmAudioFilter.java" \
  "$WORK/GateAudioFilterChain.java" \
  "$WORK/GateAudioPipeline.java" \
  "$WORK/GateAudioPipelineFactory.java" \
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
  flac_classes_argument="$(cygpath -w "$FLAC_CLASSES")"
  mp3_classes_argument="$(cygpath -w "$MP3_CLASSES")"
  flac_loader_classes_argument="$(cygpath -w "$FLAC_LOADER_CLASSES")"
  flac_metadata_reader_classes_argument="$(cygpath -w "$FLAC_METADATA_READER_CLASSES")"
  matroska_classes_argument="$(cygpath -w "$MATROSKA_CLASSES")"
  mpeg_file_loader_classes_argument="$(cygpath -w "$MPEG_FILE_LOADER_CLASSES")"
  ogg_flac_classes_argument="$(cygpath -w "$OGG_FLAC_CLASSES")"
  ogg_opus_classes_argument="$(cygpath -w "$OGG_OPUS_CLASSES")"
  ogg_vorbis_classes_argument="$(cygpath -w "$OGG_VORBIS_CLASSES")"
  ogg_vorbis_track_classes_argument="$(cygpath -w "$OGG_VORBIS_TRACK_CLASSES")"
  jar_argument="$(cygpath -w "$JAR")"
  reference_argument="$(cygpath -w "$REFERENCE_JAR")"
else
  native="$(cd "$(dirname "$native")" && pwd)/$(basename "$native")"
  classes_argument="$CLASSES"
  flac_classes_argument="$FLAC_CLASSES"
  mp3_classes_argument="$MP3_CLASSES"
  flac_loader_classes_argument="$FLAC_LOADER_CLASSES"
  flac_metadata_reader_classes_argument="$FLAC_METADATA_READER_CLASSES"
  matroska_classes_argument="$MATROSKA_CLASSES"
  mpeg_file_loader_classes_argument="$MPEG_FILE_LOADER_CLASSES"
  ogg_flac_classes_argument="$OGG_FLAC_CLASSES"
  ogg_opus_classes_argument="$OGG_OPUS_CLASSES"
  ogg_vorbis_classes_argument="$OGG_VORBIS_CLASSES"
  ogg_vorbis_track_classes_argument="$OGG_VORBIS_TRACK_CLASSES"
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

javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$FLAC_CLASSES" \
  "$WORK/GateFlacAudioTrack.java" \
  "$WORK/GateFlacContainerProbe.java" \
  "$WORK/FlacGateSupport.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$MP3_CLASSES" \
  "$WORK/GateMp3AudioTrack.java" \
  "$WORK/Mp3GateSupport.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$MP3_CLASSES" \
  "$WORK/GateMp3ConstantRateSeeker.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$MPEG_CLASSES" \
  "$WORK/GateMpegAudioTrack.java" \
  "$WORK/GateMpegContainerProbe.java" \
  "$WORK/GateMpegAdtsContainerProbe.java" \
  "$WORK/GateMpegTsElementaryInputStream.java" \
  "$WORK/GatePesPacketInputStream.java" \
  "$WORK/GateOggAudioTrack.java" \
  "$WORK/OggGateSupport.java" \
  "$WORK/MpegGateSupport.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_CODEC_CLASSES" \
  "$WORK/GateOggCodecHandler.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_PROBE_CLASSES" \
  "$WORK/GateOggContainerProbe.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_CODEC_CLASSES" \
  "$WORK/GateOggMetadata.java" \
  "$WORK/GateOggPacketInputStream.java" \
  "$WORK/GateOggPageHeader.java" \
  "$WORK/GateOggPageScanner.java" \
  "$WORK/GateOggSeekPoint.java" \
  "$WORK/GateOggStreamSizeInfo.java" \
  "$WORK/GateOggTrackBlueprint.java" \
  "$WORK/GateOggTrackHandler.java" \
  "$WORK/GateOggTrackLoader.java" \
  "$WORK/GateOggFlacCodecHandler.java" \
  "$WORK/GateOggOpusCodecHandler.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_FLAC_CLASSES" \
  "$WORK/GateOggFlacTrackHandler.java" \
  "$WORK/OggPacketInputStream.java" \
  "$WORK/AudioPipeline.java" \
  "$WORK/AudioPipelineFactory.java" \
  "$WORK/PcmFormat.java" \
  "$WORK/FlacFrameReader.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_OPUS_CLASSES" \
  "$WORK/GateOggOpusTrackHandler.java" \
  "$WORK/OpusPacketRouter.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_VORBIS_CLASSES" \
  "$WORK/GateOggVorbisCodecHandler.java" \
  "$WORK/GateExtendedM3uParser.java" \
  "$WORK/GateHlsStreamSegment.java" \
  "$WORK/GateHlsStreamSegmentParser.java" \
  "$WORK/GateHlsStreamSegmentUrlProvider.java" \
  "$WORK/GateHlsStreamTrack.java" \
  "$WORK/GateM3uPlaylistContainerProbe.java" \
  "$WORK/GatePlainPlaylistContainerProbe.java" \
  "$WORK/GatePlsPlaylistContainerProbe.java" \
  "$WORK/GateWavAudioTrack.java" \
  "$WORK/WavGateSupport.java" \
  "$WORK/GateWavContainerProbe.java" \
  "$WORK/GateWavFileInfo.java" \
  "$WORK/GateWavFileLoader.java" \
  "$WORK/GateWavTrackProvider.java" \
  "$WORK/GateWaveFormatType.java" \
  "$WORK/GateVorbisCommentParser.java" \
  "$WORK/OggVorbisTrackHandler.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$OGG_VORBIS_TRACK_CLASSES" \
  "$OGG_VORBIS_TRACK_SOURCES/GateOggVorbisTrackHandler.java" \
  "$OGG_VORBIS_TRACK_SOURCES/VorbisDecoder.java" \
  "$OGG_VORBIS_TRACK_SOURCES/AudioPipeline.java" \
  "$OGG_VORBIS_TRACK_SOURCES/AudioPipelineFactory.java" \
  "$OGG_VORBIS_TRACK_SOURCES/PcmFormat.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  -d "$MPEG_FILE_LOADER_CLASSES" "$WORK/GateMpegFileLoader.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  -d "$FLAC_LOADER_CLASSES" \
  "$WORK/GateFlacFileLoader.java" \
  "$WORK/FlacLoaderGateSupport.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  -d "$FLAC_METADATA_READER_CLASSES" \
  "$WORK/GateFlacMetadataReader.java" \
  "$WORK/FlacMetadataReaderGateSupport.java"
javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  -d "$MATROSKA_CLASSES" \
  "$WORK/GateMatroskaAudioTrack.java" \
  "$WORK/GateMatroskaContainerProbe.java" \
  "$WORK/MatroskaGateSupport.java"

javac --release 11 -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" -d "$CLASSES" \
  "$WORK/GateDecodedException.java" \
  "$WORK/GateExceptionTools.java" \
  "$WORK/GateExceptionToolsDefaultErrorDebugInfoHandler.java" \
  "$WORK/GateExceptionToolsErrorDebugInfo.java" \
  "$WORK/GateExceptionToolsErrorDebugInfoHandler.java" \
  "$WORK/GateFriendlyException.java" \
  "$WORK/GateFriendlyExceptionSeverity.java" \
  "$WORK/GateFutureTools.java" \
  "$WORK/GateGarbageCollectionMonitor.java" \
  "$WORK/GateAdtsStreamProvider.java" \
  "$WORK/GateAacPacketRouter.java" \
  "$WORK/GateMpegAacTrackConsumer.java" \
  "$WORK/GateMatroskaAacTrackConsumer.java" \
  "$WORK/GateMatroskaOpusTrackConsumer.java" \
  "$WORK/GateMatroskaTrackConsumer.java" \
  "$WORK/GateMatroskaVorbisTrackConsumer.java" \
  "$WORK/GateMatroskaBlock.java" \
  "$WORK/GateMatroskaCuePoint.java" \
  "$WORK/GateMatroskaEbmlReader.java" \
  "$WORK/GateMatroskaElement.java" \
  "$WORK/GateMatroskaElementType.java" \
  "$WORK/GateMatroskaFileReader.java" \
  "$WORK/GateMatroskaFileTrack.java" \
  "$WORK/GateMatroskaMutableElement.java" \
  "$WORK/GateMatroskaStreamingFile.java" \
  "$WORK/GateOpusPacketRouter.java" \
  "$WORK/GateDefaultSoundCloudDataLoader.java" \
  "$WORK/GateDefaultSoundCloudPlaylistLoader.java" \
  "$WORK/GateSoundCloudAudioSourceManager.java" \
  "$WORK/GateSoundCloudAudioSourceManagerBuilder.java" \
  "$WORK/GateSoundCloudAudioTrack.java" \
  "$WORK/GateSoundCloudClientIdTracker.java" \
  "$WORK/GateSoundCloudDataLoader.java" \
  "$WORK/GateSoundCloudDataReader.java" \
  "$WORK/GateSoundCloudFormatHandler.java" \
  "$WORK/GateSoundCloudHelper.java" \
  "$WORK/GateSoundCloudHttpContextFilter.java" \
  "$WORK/GateSoundCloudM3uAudioTrack.java" \
  "$WORK/GateSoundCloudM3uInfo.java" \
  "$WORK/GateSoundCloudMp3SegmentDecoder.java" \
  "$WORK/GateSoundCloudOpusSegmentDecoder.java" \
  "$WORK/GateSoundCloudPlaylistLoader.java" \
  "$WORK/GateSoundCloudSegmentDecoder.java" \
  "$WORK/GateSoundCloudSegmentDecoderFactory.java" \
  "$WORK/GateSoundCloudTrackFormat.java" \
  "$WORK/GateM3uStreamAudioTrack.java" \
  "$WORK/GateM3uStreamSegmentUrlProvider.java" \
  "$WORK/GateMpegTsM3uStreamAudioTrack.java" \
  "$WORK/GateTwitchConstants.java" \
  "$WORK/GateTwitchStreamAudioSourceManager.java" \
  "$WORK/GateTwitchStreamAudioTrack.java" \
  "$WORK/GateTwitchStreamSegmentUrlProvider.java" \
  "$WORK/GateBandcampAudioSourceManager.java" \
  "$WORK/GateBandcampAudioTrack.java" \
  "$WORK/GateBeamAudioSourceManager.java" \
  "$WORK/GateBeamAudioTrack.java" \
  "$WORK/GateBeamSegmentUrlProvider.java" \
  "$WORK/GateGetyarnAudioSourceManager.java" \
  "$WORK/GateGetyarnAudioTrack.java" \
  "$WORK/GateHttpAudioSourceManager.java" \
  "$WORK/GateHttpAudioTrack.java" \
  "$WORK/GateVimeoAudioSourceManager.java" \
  "$WORK/GateVimeoPlaybackFormat.java" \
  "$WORK/GateVimeoAudioTrack.java" \
  "$WORK/GateAbstractYandexMusicApiLoader.java" \
  "$WORK/GateYandexMusicApiExtractor.java" \
  "$WORK/GateDefaultYandexMusicDirectUrlLoader.java" \
  "$WORK/GateDefaultYandexMusicPlaylistLoader.java" \
  "$WORK/GateDefaultYandexMusicTrackLoader.java" \
  "$WORK/GateDefaultYandexSearchProvider.java" \
  "$WORK/GateYandexHttpContextFilter.java" \
  "$WORK/GateYandexMusicApiLoader.java" \
  "$WORK/GateYandexMusicAudioSourceManager.java" \
  "$WORK/GateYandexMusicAudioTrack.java" \
  "$WORK/GateYandexMusicDirectUrlLoader.java" \
  "$WORK/GateYandexMusicPlaylistLoader.java" \
  "$WORK/GateYandexMusicSearchResultLoader.java" \
  "$WORK/GateYandexMusicTrackLoader.java" \
  "$WORK/GateYandexMusicUtils.java" \
  "$WORK/GateDefaultYoutubeLinkRouter.java" \
  "$WORK/GateDefaultYoutubePlaylistLoader.java" \
  "$WORK/GateDefaultYoutubeTrackDetails.java" \
  "$WORK/GateDefaultYoutubeTrackDetailsLoader.java" \
  "$WORK/GateYoutubeCachedPlayerScript.java" \
  "$WORK/GateYoutubeInfoStatus.java" \
  "$WORK/GateYoutubeAccessTokenTracker.java" \
  "$WORK/GateYoutubeCachedAuthScript.java" \
  "$WORK/GateYoutubeAudioSourceManager.java" \
  "$WORK/GateYoutubeAudioTrack.java" \
  "$WORK/GateYoutubeCipherOperation.java" \
  "$WORK/GateYoutubeClientConfig.java" \
  "$WORK/GateYoutubeConstants.java" \
  "$WORK/GateYoutubeFormatInfo.java" \
  "$WORK/GateYoutubeHttpContextFilter.java" \
  "$WORK/GateYoutubeLinkRouter.java" \
  "$WORK/GateYoutubeMixLoader.java" \
  "$WORK/GateYoutubeMixProvider.java" \
  "$WORK/GateYoutubeMpegStreamAudioTrack.java" \
  "$WORK/GateYoutubePayloadHelper.java" \
  "$WORK/GateYoutubePersistentHttpStream.java" \
  "$WORK/GateYoutubePlaylistLoader.java" \
  "$WORK/GateYoutubeSearchMusicProvider.java" \
  "$WORK/GateYoutubeSearchMusicResultLoader.java" \
  "$WORK/GateYoutubeSearchProvider.java" \
  "$WORK/GateYoutubeSearchResultLoader.java" \
  "$WORK/GateYoutubeSignatureCipher.java" \
  "$WORK/GateYoutubeSignatureCipherManager.java" \
  "$WORK/GateYoutubeSignatureResolver.java" \
  "$WORK/GateYoutubeTrackDetails.java" \
  "$WORK/GateYoutubeTrackDetailsLoader.java" \
  "$WORK/GateYoutubeTrackFormat.java" \
  "$WORK/GateYoutubeTrackJsonData.java" \
  "$WORK/GateLegacyAdaptiveFormatsExtractor.java" \
  "$WORK/GateLegacyDashMpdFormatsExtractor.java" \
  "$WORK/GateLegacyStreamMapFormatsExtractor.java" \
  "$WORK/GateOfflineYoutubeTrackFormatExtractor.java" \
  "$WORK/GateStreamingDataFormatsExtractor.java" \
  "$WORK/GateYoutubeTrackFormatExtractor.java"

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
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioFilterInterface \
  >"$WORK/audio-filter-interface-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioFilterInterface >"$WORK/audio-filter-interface-candidate.txt"
cmp "$WORK/audio-filter-interface-reference.txt" \
  "$WORK/audio-filter-interface-candidate.txt"
grep --fixed-strings \
  'implementation=seek,flush,close,identity;exceptions=flush-interrupted;' \
  "$WORK/audio-filter-interface-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFloatPcmAudioFilter \
  >"$WORK/float-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFloatPcmAudioFilter >"$WORK/float-pcm-audio-filter-candidate.txt"
cmp "$WORK/float-pcm-audio-filter-reference.txt" \
  "$WORK/float-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'implementation=process,input-identity,offset,length,state;exceptions=process-interrupted,null-receiver;reflection=public-abstract-interface,0-fields,0-constructors,1-method,1-parent,throws' \
  "$WORK/float-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateShortPcmAudioFilter \
  >"$WORK/short-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateShortPcmAudioFilter >"$WORK/short-pcm-audio-filter-candidate.txt"
cmp "$WORK/short-pcm-audio-filter-reference.txt" \
  "$WORK/short-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'short=array,buffer,identity,state,offset,length;split=jagged-identity,state,offset,length;failures=array-interrupted,buffer-interrupted,split-interrupted,null-receivers;reflection=2-public-abstract-interfaces,0-fields,0-constructors,3-methods,audio-filter-parent,throws' \
  "$WORK/short-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateUniversalPcmAudioFilter \
  >"$WORK/universal-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateUniversalPcmAudioFilter >"$WORK/universal-pcm-audio-filter-candidate.txt"
cmp "$WORK/universal-pcm-audio-filter-reference.txt" \
  "$WORK/universal-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'intersection=short-array,short-buffer,split-short,float,lifecycle,identity,state;failures=interrupted-identity,prefix,null-receiver;reflection=public-abstract-marker,0-fields,0-constructors,0-methods,3-ordered-parents,declaring-interfaces' \
  "$WORK/universal-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateUserProvidedAudioFilters \
  >"$WORK/user-provided-audio-filters-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateUserProvidedAudioFilters >"$WORK/user-provided-audio-filters-candidate.txt"
cmp "$WORK/user-provided-audio-filters-reference.txt" \
  "$WORK/user-provided-audio-filters-candidate.txt"
grep --fixed-strings \
  'construction=context,next,snapshot,null-factory,empty-factory,copy,reverse,subclass;dispatch=float,short-array,buffer,split,identity,values,interrupted;lifecycle=reverse-order,continue-after-failure,same-factory,no-swap,swap,empty-repeat,null-target;failures=null-context,null-list,runtime-identity,rebuild-rollback;reflection=public-concrete-composite,4-private-fields,1-constructor,7-methods,generic-list,throws' \
  "$WORK/user-provided-audio-filters-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateConverterAudioFilter \
  >"$WORK/converter-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateConverterAudioFilter >"$WORK/converter-audio-filter-candidate.txt"
cmp "$WORK/converter-audio-filter-reference.txt" \
  "$WORK/converter-audio-filter-candidate.txt"
grep --fixed-strings \
  'conversion=zero,signed-zero,subnormal,halves,unit-wrap,multiple-wrap,nan,infinities,extremes;lifecycle=seek,flush,close,repeated,stateless,subclass;reflection=public-abstract-object,universal-parent,1-protected-constant,1-public-constructor,4-methods,throws' \
  "$WORK/converter-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateToFloatAudioFilter \
  >"$WORK/to-float-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToFloatAudioFilter >"$WORK/to-float-audio-filter-candidate.txt"
cmp "$WORK/to-float-audio-filter-reference.txt" \
  "$WORK/to-float-audio-filter-candidate.txt"
grep --fixed-strings \
  'conversion=extremes,interleaved,split,buffer,float-identity;chunking=4096-plus-1,reuse,complete-frames,tail-preserved;failures=interrupted,null,zero-channel,negative-channel,bounds;reflection=public-concrete-converter,3-private-final-fields,1-public-constructor,5-methods,throws' \
  "$WORK/to-float-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateToShortAudioFilter \
  >"$WORK/to-short-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToShortAudioFilter >"$WORK/to-short-audio-filter-candidate.txt"
cmp "$WORK/to-short-audio-filter-reference.txt" \
  "$WORK/to-short-audio-filter-candidate.txt"
grep --fixed-strings \
  'conversion=float-extremes,planar-offset-quirk,split-offset-quirk,short-identity,buffer-identity;chunking=4096-plus-1,reuse,repeated-source-prefix,interleaved-output;failures=interrupted,null,zero-channel,negative-channel,overflow,bounds,end-overflow;reflection=public-concrete-converter,3-private-final-fields,1-public-constructor,4-methods,throws' \
  "$WORK/to-short-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateToSplitShortAudioFilter exact \
  >"$WORK/to-split-short-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToSplitShortAudioFilter exact >"$WORK/to-split-short-audio-filter-candidate.txt"
cmp "$WORK/to-split-short-audio-filter-reference.txt" \
  "$WORK/to-split-short-audio-filter-candidate.txt"
grep --fixed-strings \
  'exact=constructor,one-channel-array,one-channel-buffer,split-identity,negative-noop,interrupted,null,reflection' \
  "$WORK/to-split-short-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateToSplitShortAudioFilter liveness \
  >"$WORK/to-split-short-audio-filter-liveness-reference.txt"
grep --fixed-strings \
  'liveness=float-stalls-true,zero-array-stalls-true,zero-buffer-stalls-true' \
  "$WORK/to-split-short-audio-filter-liveness-reference.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToSplitShortAudioFilter reference-defects \
  >"$WORK/to-split-short-audio-filter-defects-reference.txt"
grep --fixed-strings \
  'reference-defects=array-overrun,buffer-overread' \
  "$WORK/to-split-short-audio-filter-defects-reference.txt" >/dev/null
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToSplitShortAudioFilter liveness \
  >"$WORK/to-split-short-audio-filter-liveness-candidate.txt"
grep --fixed-strings \
  'liveness=float-stalls-false,zero-array-stalls-false,zero-buffer-stalls-false' \
  "$WORK/to-split-short-audio-filter-liveness-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateToSplitShortAudioFilter safety >"$WORK/to-split-short-audio-filter-safety.txt"
grep --fixed-strings \
  'safety=float-progress,4096-plus-1,reuse,stereo-array,stereo-buffer,tail-preserved,zero-channel,bounds,interrupted' \
  "$WORK/to-split-short-audio-filter-safety.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateEqualizer \
  >"$WORK/equalizer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateEqualizer >"$WORK/equalizer-candidate.txt"
cmp "$WORK/equalizer-reference.txt" "$WORK/equalizer-candidate.txt"
grep --fixed-strings \
  ';contracts=configuration,clamping,live-array,15-band-dsp,channel-history,seek-reset,identity,offset,length,interrupted,lifecycle,compatibility,factory,failures,reflection' \
  "$WORK/equalizer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateVolume \
  >"$WORK/volume-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVolume >"$WORK/volume-candidate.txt"
cmp "$WORK/volume-reference.txt" "$WORK/volume-candidate.txt"
grep --fixed-strings \
  ';contracts=arithmetic,saturation,slices,state,frame-interpolation,identity,codec-lifecycle,finally,interrupt,post-processing,failures,reflection' \
  "$WORK/volume-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioDataFormat \
  >"$WORK/audio-data-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioDataFormat >"$WORK/audio-data-format-candidate.txt"
cmp "$WORK/audio-data-format-reference.txt" "$WORK/audio-data-format-candidate.txt"
grep --fixed-strings \
  'contracts=geometry,overflow,duration,abstract-dispatch,exact-class,short-circuit,nullable-codec,hash-overflow,failures,reflection' \
  "$WORK/audio-data-format-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioDataFormatTools \
  >"$WORK/audio-data-format-tools-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioDataFormatTools >"$WORK/audio-data-format-tools-candidate.txt"
cmp "$WORK/audio-data-format-tools-reference.txt" \
  "$WORK/audio-data-format-tools-candidate.txt"
grep --fixed-strings \
  'contracts=pcm-signed,geometry,float-conversion,overflow,codec-dispatch,unsupported,null-codec,failure-identity,reflection' \
  "$WORK/audio-data-format-tools-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioPlayerInputStream \
  >"$WORK/audio-player-input-stream-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioPlayerInputStream >"$WORK/audio-player-input-stream-candidate.txt"
cmp "$WORK/audio-player-input-stream-reference.txt" \
  "$WORK/audio-player-input-stream-candidate.txt"
grep --fixed-strings \
  'contracts=signed-read,availability,bulk-offset,frame-crossing,null-retry,silence,timeout-listener,interrupt,format-check,close,factory,private-state,reflection' \
  "$WORK/audio-player-input-stream-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOpusAudioDataFormat \
  >"$WORK/opus-audio-data-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOpusAudioDataFormat >"$WORK/opus-audio-data-format-candidate.txt"
cmp "$WORK/opus-audio-data-format-reference.txt" \
  "$WORK/opus-audio-data-format-candidate.txt"
grep --fixed-strings \
  'contracts=codec,geometry,overflow,silence-alias,equality,hash,factories,failure-order,private-state,reflection' \
  "$WORK/opus-audio-data-format-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePcm16AudioDataFormat \
  >"$WORK/pcm16-audio-data-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePcm16AudioDataFormat >"$WORK/pcm16-audio-data-format-candidate.txt"
cmp "$WORK/pcm16-audio-data-format-reference.txt" \
  "$WORK/pcm16-audio-data-format-candidate.txt"
grep --fixed-strings \
  'contracts=codec,geometry,overflow,silence-instance,equality,endian-ignored,hash,factories,endian-transcoding,null-configuration,private-state,reflection' \
  "$WORK/pcm16-audio-data-format-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateStandardAudioDataFormats \
  >"$WORK/standard-audio-data-formats-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateStandardAudioDataFormats >"$WORK/standard-audio-data-formats-candidate.txt"
cmp "$WORK/standard-audio-data-formats-reference.txt" \
  "$WORK/standard-audio-data-formats-candidate.txt"
grep --fixed-strings \
  'contracts=singletons,initialization-order,runtime-types,discord-geometry,common-geometry,endian-assignments,value-pairs,constructor,reflection' \
  "$WORK/standard-audio-data-formats-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioChunkDecoder \
  >"$WORK/audio-chunk-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioChunkDecoder >"$WORK/audio-chunk-decoder-candidate.txt"
cmp "$WORK/audio-chunk-decoder-reference.txt" \
  "$WORK/audio-chunk-decoder-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,no-fields,no-constructors,decode-signature,close-signature,identity-dispatch,caller-buffer,null-forwarding,failure-identity,reflection' \
  "$WORK/audio-chunk-decoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioChunkEncoder \
  >"$WORK/audio-chunk-encoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioChunkEncoder >"$WORK/audio-chunk-encoder-candidate.txt"
cmp "$WORK/audio-chunk-encoder-reference.txt" \
  "$WORK/audio-chunk-encoder-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,no-fields,no-constructors,overload-signatures,close-signature,identity-dispatch,input-consumption,returned-array,caller-output,null-forwarding,failure-identity,reflection' \
  "$WORK/audio-chunk-encoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOpusChunkDecoder \
  >"$WORK/opus-chunk-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOpusChunkDecoder >"$WORK/opus-chunk-decoder-candidate.txt"
cmp "$WORK/opus-chunk-decoder-reference.txt" \
  "$WORK/opus-chunk-decoder-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-geometry,direct-encoded-buffer,capacity-4096,buffer-reuse,silence-decode,output-clear-flip,oversize-order,null-order,heap-output,close-idempotence,closed-failure,private-state,reflection' \
  "$WORK/opus-chunk-decoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOpusChunkEncoder \
  >"$WORK/opus-chunk-encoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOpusChunkEncoder >"$WORK/opus-chunk-encoder-candidate.txt"
cmp "$WORK/opus-chunk-encoder-reference.txt" \
  "$WORK/opus-chunk-encoder-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-order,configuration-quality,format-identity,direct-staging-capacity,returning-array,exact-allocation,staging-consumption,direct-output,heap-output,array-offset-zero,input-preservation,null-order,heap-input,small-output,readonly-output,close-idempotence,closed-failure,private-state,reflection' \
  "$WORK/opus-chunk-encoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePcmChunkDecoder \
  >"$WORK/pcm-chunk-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePcmChunkDecoder >"$WORK/pcm-chunk-decoder-candidate.txt"
cmp "$WORK/pcm-chunk-decoder-reference.txt" \
  "$WORK/pcm-chunk-decoder-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-capacity,heap-byte-buffer,byte-order,shared-short-view,big-endian,little-endian,odd-tail,output-clear-rewind,buffer-reuse,oversize-order,null-order,small-output,readonly-output,close-noop,private-state,reflection' \
  "$WORK/pcm-chunk-decoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePcmChunkEncoder \
  >"$WORK/pcm-chunk-encoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePcmChunkEncoder >"$WORK/pcm-chunk-encoder-candidate.txt"
cmp "$WORK/pcm-chunk-encoder-reference.txt" \
  "$WORK/pcm-chunk-encoder-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-capacity,heap-byte-buffer,byte-order,shared-short-view,big-endian,little-endian,returning-array,exact-allocation,input-mark-reset,buffered-append-flip,byte-cursor-bypass,buffer-reuse,null-order,oversize-order,small-output,readonly-output,close-noop,private-state,reflection' \
  "$WORK/pcm-chunk-encoder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFormats \
  >"$WORK/formats-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFormats >"$WORK/formats-candidate.txt"
cmp "$WORK/formats-reference.txt" "$WORK/formats-candidate.txt"
grep --fixed-strings \
  'contracts=constant-values,constant-identity,public-static-final,constant-value-attributes,public-constructor,subclassable,no-instance-state,no-class-initializer,reflection' \
  "$WORK/formats-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainer \
  >"$WORK/media-container-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainer >"$WORK/media-container-candidate.txt"
cmp "$WORK/media-container-reference.txt" "$WORK/media-container-candidate.txt"
grep --fixed-strings \
  'contracts=enum-order,identity,name-ordinal,defensive-values,value-of,lookup-failures,probe-types,probe-identity,fresh-mutable-array-list,enum-collections,private-enum-state,generic-signatures,reflection' \
  "$WORK/media-container-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerDescriptor \
  >"$WORK/media-container-descriptor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerDescriptor >"$WORK/media-container-descriptor-candidate.txt"
cmp "$WORK/media-container-descriptor-reference.txt" \
  "$WORK/media-container-descriptor-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-identity,null-construction,argument-order,return-identity,repeated-delegation,null-forwarding,exception-identity,null-probe-failure,subclassable,no-private-state,reflection' \
  "$WORK/media-container-descriptor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerDetection \
  >"$WORK/media-container-detection-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerDetection >"$WORK/media-container-detection-candidate.txt"
cmp "$WORK/media-container-detection-reference.txt" \
  "$WORK/media-container-detection-candidate.txt"
grep --fixed-strings \
  'contracts=constant-values,constant-identity,constructor-identity,byte-match,wildcard,eof,rewind,no-rewind,read-seek-failures,greedy-regex,partial-read,charset,regex-rewind,hint-first,fallback-pass,probe-order,probe-seek-zero,probe-failure-suppression,result-identity,unknown-singleton,saved-head,outer-friendly-wrap,private-state,reflection' \
  "$WORK/media-container-detection-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerDetectionResult \
  >"$WORK/media-container-detection-result-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerDetectionResult >"$WORK/media-container-detection-result-candidate.txt"
cmp "$WORK/media-container-detection-result-reference.txt" \
  "$WORK/media-container-detection-result-candidate.txt"
grep --fixed-strings \
  'contracts=unknown-singleton,unknown-null-state,fresh-factories,argument-identity,null-acceptance,container-detected,descriptor-freshness,descriptor-state,supported-derivation,unsupported-reason,track-info,reference-derivation,private-constructor-state,reflection' \
  "$WORK/media-container-detection-result-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerHints \
  >"$WORK/media-container-hints-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerHints >"$WORK/media-container-hints-candidate.txt"
cmp "$WORK/media-container-hints-reference.txt" "$WORK/media-container-hints-candidate.txt"
grep --fixed-strings \
  'contracts=eager-empty-singleton,singleton-identity,fresh-non-null,factory-identity,null-acceptance,empty-string-presence,derived-presence,private-constructor-state,non-final-class,reflection' \
  "$WORK/media-container-hints-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerProbe \
  >"$WORK/media-container-probe-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerProbe >"$WORK/media-container-probe-candidate.txt"
cmp "$WORK/media-container-probe-reference.txt" "$WORK/media-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=implementation-dispatch,name-identity,hints-identity,boolean-result,probe-identity,result-identity,checked-exception-identity,create-track-identity,nulls,abstract-interface,reflection' \
  "$WORK/media-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMediaContainerRegistry \
  >"$WORK/media-container-registry-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMediaContainerRegistry >"$WORK/media-container-registry-candidate.txt"
cmp "$WORK/media-container-registry-reference.txt" \
  "$WORK/media-container-registry-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-alias,list-identity,live-mutation,linear-find,first-match,short-circuit,null-list,null-name,null-probe,failure-identity,eager-default,default-order,default-mutability,fresh-extension,additional-order,array-copy,null-additional,null-varargs,subclassable,generic-signatures,varargs,reflection' \
  "$WORK/media-container-registry-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAdtsAudioTrack \
  >"$WORK/adts-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAdtsAudioTrack >"$WORK/adts-audio-track-candidate.txt"
cmp "$WORK/adts-audio-track-reference.txt" "$WORK/adts-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,processing-context,read-callback,non-seekable,empty-stream,input-ownership,context-order,null-executor,null-input,io-wrapping,failure-identity,identifier-dispatch,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/adts-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$mp3_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3AudioTrack \
  >"$WORK/mp3-audio-track-reference.txt"
java -Xverify:all \
  -cp "$mp3_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3AudioTrack >"$WORK/mp3-audio-track-candidate.txt"
cmp "$WORK/mp3-audio-track-reference.txt" "$WORK/mp3-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,processing-context,header-parse,read-callback,seek-callback,full-timecode,executor-control,input-ownership,context-failure,null-executor,identifier-dispatch,loop-failure,callback-failure,parse-failure,close-finally,close-replacement,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/mp3-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegAudioTrack \
  >"$WORK/mpeg-audio-track-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegAudioTrack >"$WORK/mpeg-audio-track-candidate.txt"
cmp "$WORK/mpeg-audio-track-reference.txt" "$WORK/mpeg-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,track-selection,context,initialise,reader,duration,read-callback,seek-callback,full-width-timecode,executor-control,input-ownership,unsupported,parse-failure,context-failure,initialise-failure,reader-failure,loop-failure,callback-failure,close-finally,close-replacement,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/mpeg-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegContainerProbe \
  >"$WORK/mpeg-container-probe-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegContainerProbe >"$WORK/mpeg-container-probe-candidate.txt"
cmp "$WORK/mpeg-container-probe-reference.txt" "$WORK/mpeg-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,constant-hints,always-no-hints,iso-tag,wildcard,rewind,scan-miss,short-input,unsupported-audio,unsupported-reader,metadata,duration,probe-identity,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/mpeg-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegAdtsContainerProbe \
  >"$WORK/mpeg-adts-container-probe-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegAdtsContainerProbe >"$WORK/mpeg-adts-container-probe-candidate.txt"
cmp "$WORK/mpeg-adts-container-probe-reference.txt" "$WORK/mpeg-adts-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,hint-extension,case-insensitive,wrong-hints,empty-miss,non-ts-miss,no-rewind,null-reference,null-input,io-identity,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/mpeg-adts-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegTsElementaryInputStream \
  >"$WORK/mpeg-ts-elementary-input-stream-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTsElementaryInputStream >"$WORK/mpeg-ts-elementary-input-stream-candidate.txt"
cmp "$WORK/mpeg-ts-elementary-input-stream-reference.txt" "$WORK/mpeg-ts-elementary-input-stream-candidate.txt"
grep --fixed-strings \
  'contracts=constant,constructor,wrapper,no-eager-read,metadata-freshness,metadata-nulls,empty-eof,short-input,invalid-packet,null-buffer,failure-identity,public-shape,private-state,reflection' \
  "$WORK/mpeg-ts-elementary-input-stream-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePesPacketInputStream \
  >"$WORK/pes-packet-input-stream-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePesPacketInputStream >"$WORK/pes-packet-input-stream-candidate.txt"
cmp "$WORK/pes-packet-input-stream-reference.txt" "$WORK/pes-packet-input-stream-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,greedy-wrapper,no-eager-read,buffers,private-state,sync-scan,header-skip,single-read,bulk-read,packet-boundaries,multiple-packets,available,zero-length,signed-length,truncated-headers,premature-payload-eof,failure-identity,subclassable,throws,reflection' \
  "$WORK/pes-packet-input-stream-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOggAudioTrack \
  >"$WORK/ogg-audio-track-reference.txt"
java -Xverify:all \
  -cp "$MPEG_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggAudioTrack >"$WORK/ogg-audio-track-candidate.txt"
cmp "$WORK/ogg-audio-track-reference.txt" "$WORK/ogg-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,packet-wrapper,non-closing,blueprint-load,handler-load,processing-context,initialise-zeroes,read-callback,seek-callback,full-width-timecode,chained-blueprints,handler-reuse,wait-on-end,executor-control,empty-stream,io-identity,io-wrapping,interruption-identity,runtime-identity,null-handler,null-executor,identifier-dispatch,input-ownership,subclassable,eager-logger,private-state,synthetic-callback,throws,reflection' \
  "$WORK/ogg-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOggCodecHandler \
  >"$WORK/ogg-codec-handler-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggCodecHandler >"$WORK/ogg-codec-handler-candidate.txt"
cmp "$WORK/ogg-codec-handler-reference.txt" "$WORK/ogg-codec-handler-candidate.txt"
grep --fixed-strings \
  'contracts=public-interface,no-fields,no-constructors,four-abstract-methods,identifier-int,maximum-length-int,packet-identity,broker-identity,blueprint-identity,metadata-identity,null-arguments,null-returns,checked-failure-identity,runtime-failure-identity,implementation-dispatch,throws,reflection' \
  "$WORK/ogg-codec-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_PROBE_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggContainerProbe "$ROOT/tests/media/fixtures/tone-opus-tags.ogg" \
  >"$WORK/ogg-container-probe-reference.txt"
java -Xverify:all \
  -cp "$OGG_PROBE_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggContainerProbe "$ROOT/tests/media/fixtures/tone-opus-tags.ogg" \
  >"$WORK/ogg-container-probe-candidate.txt"
cmp "$WORK/ogg-container-probe-reference.txt" "$WORK/ogg-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,no-hints,null-hints,empty-miss,non-ogg-miss,current-position,rewind,null-reference-miss,matched-null-reference,null-input,read-failure-identity,seek-failure-identity,runtime-failure-identity,provider-failure-identity,truncated-supported,metadata-failure-swallowed,provider-overlay,tagged-opus,title,artist,isrc,duration,descriptor-identity,stream-ownership,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,private-helper,private-state,throws,reflection' \
  "$WORK/ogg-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggMetadata >"$WORK/ogg-metadata-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggMetadata >"$WORK/ogg-metadata-candidate.txt"
cmp "$WORK/ogg-metadata-reference.txt" "$WORK/ogg-metadata-candidate.txt"
grep --fixed-strings \
  'contracts=empty-singleton,empty-values,empty-map-immutable,direct-map-identity,direct-length-identity,uppercase-title,uppercase-artist,uppercase-isrc,case-sensitive,live-map-mutation,nullable-tags,nullable-length,wrong-value-cast,failure-identity,identifier-null,uri-null,artwork-null,provider-interface,identity-semantics,subclassable,public-static-field,private-constants,private-final-state,generic-map-signature,throws,reflection' \
  "$WORK/ogg-metadata-candidate.txt" >/dev/null
# Preserve Ogg packet/page semantics while bounding the reference's whole-content seek-table allocation.
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPacketInputStream reference >"$WORK/ogg-packet-input-stream-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPacketInputStream candidate >"$WORK/ogg-packet-input-stream-candidate.txt"
grep --fixed-strings \
  'common=public-input-stream,13-fields,1-constructor,17-methods,private-state-enum,constructor-capture,track-boundaries,packet-boundaries,single-read,bulk-read,zero-length-read,available,multiple-packets,page-continuation,empty-pages,chained-tracks,last-page,physical-eof,invalid-header,invalid-version,truncated-header,premature-packet-eof,checked-failure-identity,seek-point-identity,ceiling-selection,track-seeking,seek-table,position-restore,size-info,hard-seek-gates,delegated-close,subclassable,generics,throws,reflection;scan=legacy-content-length-allocation' \
  "$WORK/ogg-packet-input-stream-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-input-stream,13-fields,1-constructor,17-methods,private-state-enum,constructor-capture,track-boundaries,packet-boundaries,single-read,bulk-read,zero-length-read,available,multiple-packets,page-continuation,empty-pages,chained-tracks,last-page,physical-eof,invalid-header,invalid-version,truncated-header,premature-packet-eof,checked-failure-identity,seek-point-identity,ceiling-selection,track-seeking,seek-table,position-restore,size-info,hard-seek-gates,delegated-close,subclassable,generics,throws,reflection;scan=bounded-64-mib' \
  "$WORK/ogg-packet-input-stream-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPageHeader >"$WORK/ogg-page-header-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPageHeader >"$WORK/ogg-page-header-candidate.txt"
cmp "$WORK/ogg-page-header-reference.txt" "$WORK/ogg-page-header-candidate.txt"
grep --fixed-strings \
  'contracts=constants,individual-flags,combined-flags,unrelated-flags,negative-flags,full-width-fields,negative-segment-count,no-validation,immutable-public-state,identity-semantics,subclassable,field-order,constructor,throws,reflection' \
  "$WORK/ogg-page-header-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPageScanner >"$WORK/ogg-page-scanner-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggPageScanner >"$WORK/ogg-page-scanner-candidate.txt"
cmp "$WORK/ogg-page-scanner-reference.txt" "$WORK/ogg-page-scanner-candidate.txt"
grep --fixed-strings \
  'contracts=public-scanner,9-fields,1-constructor,3-methods,direct-state,live-data,data-length-window,signature-search,version-gate,lacing-capacity,payload-capacity,checksum-ignored,stream-fields-ignored,last-page,contiguous-pages,absolute-offset,end-index-size-arithmetic,granule-endianness,sample-rate,timecode-truncation,seek-order,persistent-page-sequence,fresh-mutable-list,strict-tail-boundary,short-input,null-input,invalid-length,identity-semantics,subclassable,generics,throws,reflection' \
  "$WORK/ogg-page-scanner-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggSeekPoint >"$WORK/ogg-seek-point-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggSeekPoint >"$WORK/ogg-seek-point-candidate.txt"
cmp "$WORK/ogg-seek-point-reference.txt" "$WORK/ogg-seek-point-candidate.txt"
grep --fixed-strings \
  'contracts=public-value,4-fields,1-constructor,4-getters,direct-assignment,full-width-values,negative-values,independent-values,stable-getters,immutable-private-state,identity-semantics,subclassable,override-dispatch,field-order,throws,reflection' \
  "$WORK/ogg-seek-point-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggStreamSizeInfo >"$WORK/ogg-stream-size-info-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggStreamSizeInfo >"$WORK/ogg-stream-size-info-candidate.txt"
cmp "$WORK/ogg-stream-size-info-reference.txt" "$WORK/ogg-stream-size-info-candidate.txt"
grep --fixed-strings \
  'contracts=public-value,5-fields,1-constructor,1-method,direct-assignment,full-width-values,negative-values,no-validation,duration-multiply-first,integer-truncation,signed-division,long-overflow,zero-rate-failure,unrelated-fields,immutable-public-state,identity-semantics,subclassable,override-dispatch,field-order,throws,reflection' \
  "$WORK/ogg-stream-size-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackBlueprint >"$WORK/ogg-track-blueprint-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackBlueprint >"$WORK/ogg-track-blueprint-candidate.txt"
cmp "$WORK/ogg-track-blueprint-reference.txt" "$WORK/ogg-track-blueprint-candidate.txt"
grep --fixed-strings \
  'contracts=public-interface,no-fields,no-constructors,2-abstract-methods,stream-identity,null-stream,handler-identity,null-return,full-width-sample-rate,implementation-dispatch,runtime-failure-identity,no-defaults,no-generics,throws,reflection' \
  "$WORK/ogg-track-blueprint-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackHandler >"$WORK/ogg-track-handler-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackHandler >"$WORK/ogg-track-handler-candidate.txt"
cmp "$WORK/ogg-track-handler-reference.txt" "$WORK/ogg-track-handler-candidate.txt"
grep --fixed-strings \
  'contracts=public-interface,closeable-parent,no-fields,no-constructors,3-declared-methods,context-identity,null-context,full-width-initial-timecodes,provide-dispatch,full-width-seek,inherited-close,ordered-dispatch,checked-failure-identity,runtime-failure-identity,no-defaults,no-generics,throws,reflection' \
  "$WORK/ogg-track-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackLoader >"$WORK/ogg-track-loader-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggTrackLoader >"$WORK/ogg-track-loader-candidate.txt"
cmp "$WORK/ogg-track-loader-reference.txt" "$WORK/ogg-track-loader-candidate.txt"
grep --fixed-strings \
  'contracts=public-loader,2-fields,1-constructor,2-public-static-methods,private-detection,provider-order,maximum-first-packet,track-boundary,packet-boundary,identifier-read,selection-order,stream-identity,broker-identity,blueprint-identity,metadata-identity,null-provider-returns,long-first-packet,unknown-codec,short-header,null-stream,checked-failure-identity,runtime-failure-identity,subclassable,private-nested-detection,throws,reflection' \
  "$WORK/ogg-track-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggFlacCodecHandler >"$WORK/ogg-flac-codec-handler-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggFlacCodecHandler >"$WORK/ogg-flac-codec-handler-candidate.txt"
cmp "$WORK/ogg-flac-codec-handler-reference.txt" "$WORK/ogg-flac-codec-handler-candidate.txt"
grep --fixed-strings \
  'contracts=identifier,maximum-length,public-construction,stream-info,metadata-blocks,tag-parse,metadata-duration,seek-table,seek-point-forwarding,blueprint-sample-rate,handler-info-identity,handler-stream-identity,empty-tags,unknown-identifier,short-native-header,wrong-native-header,missing-metadata,metadata-io-identity,blueprint-io-identity,metadata-size-lookup,subclassable,private-blueprint,private-methods,throws,reflection' \
  "$WORK/ogg-flac-codec-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggOpusCodecHandler >"$WORK/ogg-opus-codec-handler-reference.txt"
java -Xverify:all \
  -cp "$OGG_CODEC_CLASSES$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggOpusCodecHandler >"$WORK/ogg-opus-codec-handler-candidate.txt"
cmp "$WORK/ogg-opus-codec-handler-reference.txt" "$WORK/ogg-opus-codec-handler-candidate.txt"
grep --fixed-strings \
  'contracts=identifier,maximum-length,public-construction,opus-head,little-endian-rate,unsigned-channels,comment-skip-bound,comment-save-bound,comment-read-bound,tag-parse,empty-tags,metadata-duration,unknown-duration,size-rate,seek-table,seek-point-identity,blueprint-state,blueprint-sample-rate,broker-clear,handler-stream-identity,handler-broker-identity,handler-channel-rate,missing-comments,oversized-comments,complete-long-comments,checked-failure-identity,runtime-failure-identity,subclassable,private-blueprint,private-methods,throws,reflection' \
  "$WORK/ogg-opus-codec-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_opus_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggOpusTrackHandler >"$WORK/ogg-opus-track-handler-reference.txt"
java -Xverify:all \
  -cp "$ogg_opus_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggOpusTrackHandler >"$WORK/ogg-opus-track-handler-candidate.txt"
cmp "$WORK/ogg-opus-track-handler-reference.txt" "$WORK/ogg-opus-track-handler-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,nullable-identities,signed-channel-rate,router-default,initialise-context,router-rate-channel,full-width-timecodes,initialise-order,reinitialise,packet-loop,integer-max-bounds,consume-result-ignored,buffer-identity,empty-packet-skip,no-flush,interruption-identity,io-wrapping,runtime-identity,seek-order,seek-result,preinit-seek-order,close-before-init,close-dispatch,close-repeat,public-shape,private-state,throws,reflection' \
  "$WORK/ogg-opus-track-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggVorbisCodecHandler >"$WORK/ogg-vorbis-codec-handler-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggVorbisCodecHandler >"$WORK/ogg-vorbis-codec-handler-candidate.txt"
cmp "$WORK/ogg-vorbis-codec-handler-reference.txt" \
  "$WORK/ogg-vorbis-codec-handler-candidate.txt"
grep --fixed-strings \
  'contracts=identifier,maximum-length,public-construction,unvalidated-info-prefix,little-endian-rate,info-array-identity,comment-skip-bound,comment-save-bound,comment-read-bound,tag-parse,empty-singleton,unknown-duration,metadata-duration,size-rate,seek-table,nullable-seek-table,blueprint-state,blueprint-sample-rate,handler-info-identity,handler-stream-identity,handler-broker-identity,missing-comments,oversized-comments,complete-long-comments,short-comment-prefix,short-info-order,checked-failure-identity,runtime-failure-identity,subclassable,private-blueprint,private-methods,throws,reflection' \
  "$WORK/ogg-vorbis-codec-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVorbisCommentParser >"$WORK/vorbis-comment-parser-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVorbisCommentParser >"$WORK/vorbis-comment-parser-candidate.txt"
cmp "$WORK/vorbis-comment-parser-reference.txt" \
  "$WORK/vorbis-comment-parser-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,subclassable,fresh-hash-map,mutable-result,little-endian,vendor-skip,item-count,buffer-position,trailing-bytes,utf8,locale-root,first-equals,empty-key,empty-value,no-equals,case-folded-duplicate,last-value,negative-item-count,truncated-size,truncated-payload,strict-size,strict-payload,negative-vendor,negative-item-length,short-header,oversized-vendor,null-buffer,reflection' \
  "$WORK/vorbis-comment-parser-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExtendedM3uParser >"$WORK/extended-m3u-parser-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExtendedM3uParser >"$WORK/extended-m3u-parser-candidate.txt"
cmp "$WORK/extended-m3u-parser-reference.txt" \
  "$WORK/extended-m3u-parser-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,subclassable,trim,shared-empty-line,null-empty-fields,empty-predicates,data-line,data-map-immutable,bare-directive,hash-directive,trailing-colon,first-colon,raw-extra-data,fresh-hash-map,mutable-arguments,quoted-comma,unquoted-value,empty-value,uppercase-hyphen-keys,case-sensitive-keys,duplicate-last,malformed-permissive,argument-map-identity,null-line,line-predicates,outer-reflection,line-reflection,generic-map' \
  "$WORK/extended-m3u-parser-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegment >"$WORK/hls-stream-segment-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegment >"$WORK/hls-stream-segment-candidate.txt"
cmp "$WORK/hls-stream-segment-reference.txt" \
  "$WORK/hls-stream-segment-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,subclassable,reference-identity,nullable-duration,boxed-duration,extreme-duration,final-fields,field-order,constructor-order,no-validation,reflection' \
  "$WORK/hls-stream-segment-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegmentParser >"$WORK/hls-stream-segment-parser-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegmentParser >"$WORK/hls-stream-segment-parser-candidate.txt"
cmp "$WORK/hls-stream-segment-parser-reference.txt" \
  "$WORK/hls-stream-segment-parser-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,subclassable,array-list-result,fresh-result,trimmed-data,extinf-order,metadata-name,decimal-milliseconds,invalid-duration-null,nan-zero,infinity-saturation,negative-infinity-saturation,stale-metadata,non-extinf-ignored,no-comma-null-metadata,empty-lines,mutable-result,null-array,null-url,reflection,generic-signatures,private-duration-helper' \
  "$WORK/hls-stream-segment-parser-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegmentUrlProvider >"$WORK/hls-stream-segment-url-provider-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamSegmentUrlProvider >"$WORK/hls-stream-segment-url-provider-candidate.txt"
cmp "$WORK/hls-stream-segment-url-provider-reference.txt" \
  "$WORK/hls-stream-segment-url-provider-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,subclassable,superclass-state,constructor-identities,quality-default,segment-playlist-detection,case-sensitive-detection,leading-space-detection,entry-selection,relative-entry,empty-entry,null-lines,cached-fetch-identity,request-type,request-method,request-uri,uncached-null-failure,reflection,volatile-cache,synthetic-lambda' \
  "$WORK/hls-stream-segment-url-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamTrack >"$WORK/hls-stream-track-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHlsStreamTrack >"$WORK/hls-stream-track-candidate.txt"
cmp "$WORK/hls-stream-track-reference.txt" \
  "$WORK/hls-stream-track-candidate.txt"
grep --fixed-strings \
  'contracts=public-concrete,subclassable,mpeg-ts-m3u-super,constructor-info-identity,inner-url-routing,outer-url-routing,provider-freshness,string-identity,http-manager-identity,http-per-call-delegation,null-url,null-manager-deferred-failure,private-final-fields,protected-methods,reflection' \
  "$WORK/hls-stream-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateM3uPlaylistContainerProbe >"$WORK/m3u-playlist-container-probe-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateM3uPlaylistContainerProbe >"$WORK/m3u-playlist-container-probe-candidate.txt"
cmp "$WORK/m3u-playlist-container-probe-reference.txt" \
  "$WORK/m3u-playlist-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,public-probe,name-identity,hints-false,non-m3u-null,unsupported-create-track,interface-implementation,private-constants,private-manager,checked-probe-throws,reflection' \
  "$WORK/m3u-playlist-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePlainPlaylistContainerProbe >"$WORK/plain-playlist-container-probe-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePlainPlaylistContainerProbe >"$WORK/plain-playlist-container-probe-candidate.txt"
cmp "$WORK/plain-playlist-container-probe-reference.txt" \
  "$WORK/plain-playlist-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,public-probe,name-identity,hints-false,non-plain-null,plain-reference,reference-fields,unsupported-create-track,interface-implementation,private-pattern,checked-probe-throws,reflection' \
  "$WORK/plain-playlist-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePlsPlaylistContainerProbe >"$WORK/pls-playlist-container-probe-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePlsPlaylistContainerProbe >"$WORK/pls-playlist-container-probe-candidate.txt"
cmp "$WORK/pls-playlist-container-probe-reference.txt" \
  "$WORK/pls-playlist-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=public-construction,public-probe,name-identity,hints-false,non-pls-null,wildcard-header,case-sensitive-header,unsupported-empty,unsupported-reason,indexed-file-title-pairing,unknown-title,http-https-icy-links,scheme-filtering,duplicate-last-value,whitespace-patterns,unsupported-create-track,interface-implementation,private-header,mutable-private-patterns,checked-probe-throws,map-entry-inner-metadata,reflection' \
  "$WORK/pls-playlist-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavAudioTrack >"$WORK/wav-audio-track-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavAudioTrack >"$WORK/wav-audio-track-candidate.txt"
cmp "$WORK/wav-audio-track-reference.txt" \
  "$WORK/wav-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,loader-order,processing-context,read-callback,seek-callback,full-timecode,executor-control,input-ownership,context-failure,load-failure,null-executor,identifier-dispatch,loop-failure,callback-failure,close-finally,close-replacement,null-provider,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/wav-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavContainerProbe >"$WORK/wav-container-probe-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavContainerProbe >"$WORK/wav-container-probe-candidate.txt"
cmp "$WORK/wav-container-probe-reference.txt" \
  "$WORK/wav-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,ignored-hints,null-hints,riff-wildcard,case-sensitive,rewind-match,rewind-miss,initial-position,logging-order,loader-parse,metadata,duration,metadata-fallback,supported-result,self-probe,null-reference,read-failure,parse-failure,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/wav-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavFileInfo >"$WORK/wav-file-info-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavFileInfo >"$WORK/wav-file-info-candidate.txt"
cmp "$WORK/wav-file-info-reference.txt" \
  "$WORK/wav-file-info-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,field-identity,negative-values,duration,duration-overflow,negative-duration,zero-rate-failure,padding,odd-bit-padding,public-shape,final-fields,throws,reflection' \
  "$WORK/wav-file-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavFileLoader >"$WORK/wav-file-loader-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavFileLoader >"$WORK/wav-file-loader-candidate.txt"
cmp "$WORK/wav-file-loader-reference.txt" \
  "$WORK/wav-file-loader-candidate.txt"
grep --fixed-strings \
  'contracts=constants,constructor,input-identity,pcm-parse,position,duration,unknown-format,bad-alignment,non-wav,load-track-order,load-track-failure,private-builder,reflection' \
  "$WORK/wav-file-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavTrackProvider >"$WORK/wav-track-provider-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWavTrackProvider >"$WORK/wav-track-provider-candidate.txt"
cmp "$WORK/wav-track-provider-reference.txt" \
  "$WORK/wav-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-failure,field-layout,seek-full-width,seek-order,seek-io-wrapping,close-dispatch,provide-empty,provide-io-wrapping,private-state,throws,reflection' \
  "$WORK/wav-track-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWaveFormatType >"$WORK/wave-format-type-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateWaveFormatType >"$WORK/wave-format-type-candidate.txt"
cmp "$WORK/wave-format-type-reference.txt" \
  "$WORK/wave-format-type-candidate.txt"
grep --fixed-strings \
  'contracts=order,names,ordinals,codes,values-clone,value-of,value-of-failures,code-lookup,unknown-fallback,private-state,synthetic-state,throws,reflection' \
  "$WORK/wave-format-type-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateCopyOnUpdateIdentityList >"$WORK/copy-on-update-identity-list-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateCopyOnUpdateIdentityList >"$WORK/copy-on-update-identity-list-candidate.txt"
cmp "$WORK/copy-on-update-identity-list-reference.txt" \
  "$WORK/copy-on-update-identity-list-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-empty,public-items,identity-add,equals-distinct,duplicate-no-publish,null-add,copy-publication,snapshot-stability,remove-identity,remove-all,remove-missing-publishes,external-state,null-state-failures,subclassable,generics,throws,reflection' \
  "$WORK/copy-on-update-identity-list-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDataFormatTools >"$WORK/data-format-tools-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDataFormatTools >"$WORK/data-format-tools-candidate.txt"
cmp "$WORK/data-format-tools-reference.txt" \
  "$WORK/data-format-tools-candidate.txt"
grep --fixed-strings \
  'contracts=extract-between,extract-ranges,extract-after,extract-candidates,null-empty,map-string,map-pairs,url-decode,default-null,stream-lines,duration,nullable-text,array-range,reflection' \
  "$WORK/data-format-tools-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDataFormatToolsTextRange >"$WORK/data-format-tools-text-range-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDataFormatToolsTextRange >"$WORK/data-format-tools-text-range-candidate.txt"
cmp "$WORK/data-format-tools-text-range-reference.txt" \
  "$WORK/data-format-tools-text-range-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,field-identity,null-values,subclassable,identity,reflection' \
  "$WORK/data-format-tools-text-range-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDecodedException >"$WORK/decoded-exception-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDecodedException >"$WORK/decoded-exception-candidate.txt"
cmp "$WORK/decoded-exception-reference.txt" \
  "$WORK/decoded-exception-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,fields,message,cause,null-values,stack,suppression,reflection' \
  "$WORK/decoded-exception-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionTools >"$WORK/exception-tools-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionTools >"$WORK/exception-tools-candidate.txt"
cmp "$WORK/exception-tools-reference.txt" \
  "$WORK/exception-tools-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,rethrow,wrap-friendly,wrap-runtime,to-runtime,find-deep,interrupt,log,debug-info,serialization,close-warnings,reflection' \
  "$WORK/exception-tools-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsDefaultErrorDebugInfoHandler \
  >"$WORK/exception-tools-default-error-debug-info-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsDefaultErrorDebugInfoHandler \
  >"$WORK/exception-tools-default-error-debug-info-handler-candidate.txt"
cmp "$WORK/exception-tools-default-error-debug-info-handler-reference.txt" \
  "$WORK/exception-tools-default-error-debug-info-handler-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,interface,warning-dispatch,payload-logger-ignored,null-payload,subclassable,nested-linkage,reflection' \
  "$WORK/exception-tools-default-error-debug-info-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsErrorDebugInfo >"$WORK/exception-tools-error-debug-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsErrorDebugInfo >"$WORK/exception-tools-error-debug-info-candidate.txt"
cmp "$WORK/exception-tools-error-debug-info-reference.txt" \
  "$WORK/exception-tools-error-debug-info-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,field-identity,null-values,subclassable,nested-linkage,reflection' \
  "$WORK/exception-tools-error-debug-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsErrorDebugInfoHandler \
  >"$WORK/exception-tools-error-debug-info-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateExceptionToolsErrorDebugInfoHandler \
  >"$WORK/exception-tools-error-debug-info-handler-candidate.txt"
cmp "$WORK/exception-tools-error-debug-info-handler-reference.txt" \
  "$WORK/exception-tools-error-debug-info-handler-candidate.txt"
grep --fixed-strings \
  'contracts=caller-implementation,payload-identity,null-payload,proxy-dispatch,nested-linkage,reflection' \
  "$WORK/exception-tools-error-debug-info-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFriendlyException >"$WORK/friendly-exception-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFriendlyException >"$WORK/friendly-exception-candidate.txt"
cmp "$WORK/friendly-exception-reference.txt" \
  "$WORK/friendly-exception-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,message,severity,cause,null-values,stack,suppression,subclassable,nested-linkage,reflection' \
  "$WORK/friendly-exception-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFriendlyExceptionSeverity >"$WORK/friendly-exception-severity-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFriendlyExceptionSeverity >"$WORK/friendly-exception-severity-candidate.txt"
cmp "$WORK/friendly-exception-severity-reference.txt" \
  "$WORK/friendly-exception-severity-candidate.txt"
grep --fixed-strings \
  'contracts=order,names,ordinals,to-string,values-clone,value-of,value-of-failures,nested-linkage,synthetic-state,reflection' \
  "$WORK/friendly-exception-severity-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFutureTools >"$WORK/future-tools-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFutureTools >"$WORK/future-tools-candidate.txt"
cmp "$WORK/future-tools-reference.txt" "$WORK/future-tools-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,subclassable,take-count,input-order,done-filter,null-filter,get-failure,interruption-stop,runtime-stop,null-service,stream-fallback,null-futures,mutable-result,generics,private-state,synthetic-lambda,reflection' \
  "$WORK/future-tools-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateGarbageCollectionMonitor >"$WORK/garbage-collection-monitor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateGarbageCollectionMonitor >"$WORK/garbage-collection-monitor-candidate.txt"
cmp "$WORK/garbage-collection-monitor-reference.txt" \
  "$WORK/garbage-collection-monitor-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,subclassable,state,enable-disable-idempotence,scheduling,frequency,cancel-mode,reschedule,schedule-failure-state,bucket-thresholds,negative-duration,warning-reset,debug-reset,type-filter,null-notification,invalid-payload,no-gc-filter,gc-duration,interfaces,private-state,generics,reflection' \
  "$WORK/garbage-collection-monitor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_vorbis_track_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggVorbisTrackHandler >"$WORK/ogg-vorbis-track-handler-reference.txt"
java -Xverify:all \
  -cp "$ogg_vorbis_track_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggVorbisTrackHandler >"$WORK/ogg-vorbis-track-handler-candidate.txt"
cmp "$WORK/ogg-vorbis-track-handler-reference.txt" \
  "$WORK/ogg-vorbis-track-handler-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,info-identity,nullable-stream-broker,little-endian-rate,unsigned-channels,pcm-buffer-shapes,decoder-construction,deferred-short-info,setup-packet,direct-info-copy,setup-buffer-identity,integer-max-bounds,consume-result-ignored,decoder-initialise,broker-reset,pipeline-context,pcm-format,full-width-timecodes,reinitialise,packet-loop,decoder-input-identity,output-buffer-identity,full-buffer-drain,partial-output,zero-output-skip,interruption-identity,io-wrapping,runtime-identity,seek-order,seek-result,preinit-seek-order,close-before-init,close-order,close-repeat,public-shape,private-state,throws,reflection' \
  "$WORK/ogg-vorbis-track-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$ogg_flac_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggFlacTrackHandler >"$WORK/ogg-flac-track-handler-reference.txt"
java -Xverify:all \
  -cp "$ogg_flac_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOggFlacTrackHandler >"$WORK/ogg-flac-track-handler-candidate.txt"
cmp "$WORK/ogg-flac-track-handler-reference.txt" "$WORK/ogg-flac-track-handler-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,info-identity,stream-identity,buffer-shapes,nullable-input,initialise-context,pcm-format,full-width-timecodes,pipeline-seek,packet-loop,frame-reader-identity,buffer-identity,frame-count,zero-frame-failure,io-wrapping,runtime-identity,seek-order,seek-result,seek-io-wrapping,close-before-init,close-dispatch,close-repeat,public-shape,private-state,throws,reflection' \
  "$WORK/ogg-flac-track-handler-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$mpeg_file_loader_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFileLoader "$ROOT/tests/media/fixtures/tone-aac-lc-metadata.m4a" \
  "$ROOT/tests/media/fixtures/tone-aac-lc-fragmented.m4a" \
  >"$WORK/mpeg-file-loader-reference.txt"
java -Xverify:all \
  -cp "$mpeg_file_loader_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFileLoader "$ROOT/tests/media/fixtures/tone-aac-lc-metadata.m4a" \
  "$ROOT/tests/media/fixtures/tone-aac-lc-fragmented.m4a" \
  >"$WORK/mpeg-file-loader-candidate.txt"
cmp "$WORK/mpeg-file-loader-reference.txt" "$WORK/mpeg-file-loader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,input-identity,root-bounds,private-state,mutable-track-list,metadata-types,standard-headers,fragmented-headers,track-fields,decoder-config,standard-reader,fragmented-reader,consumer-identity,duration,event-message,last-event,io-wrapping,cause-identity,empty-file,null-input,subclassable,generics,throws,reflection' \
  "$WORK/mpeg-file-loader-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegNoopTrackConsumer >"$WORK/mpeg-noop-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegNoopTrackConsumer >"$WORK/mpeg-noop-track-consumer-candidate.txt"
cmp "$WORK/mpeg-noop-track-consumer-reference.txt" \
  "$WORK/mpeg-noop-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,track-identity,null-track,initialise,seek-extremes,flush,consume,null-channel,length-extremes,channel-untouched,close,idempotent,interrupt-preserved,interface-dispatch,subclassable,private-final-state,checked-throws,reflection' \
  "$WORK/mpeg-noop-track-consumer-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackConsumer >"$WORK/mpeg-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackConsumer >"$WORK/mpeg-track-consumer-candidate.txt"
cmp "$WORK/mpeg-track-consumer-reference.txt" "$WORK/mpeg-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,no-fields,no-constructors,six-methods,track-return-identity,initialise-dispatch,seek-argument-order,full-width-timecodes,flush-dispatch,consume-channel-identity,null-channel,length-extremes,close-dispatch,checked-failure-identity,implementation-compatibility,checked-throws,reflection' \
  "$WORK/mpeg-track-consumer-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackInfo >"$WORK/mpeg-track-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackInfo >"$WORK/mpeg-track-info-candidate.txt"
cmp "$WORK/mpeg-track-info-reference.txt" "$WORK/mpeg-track-info-candidate.txt"
grep --fixed-strings \
  'contracts=field-order,scalar-storage,string-identity,decoder-config-identity,null-members,mutation-visibility,no-validation,identity-equality,subclassable,public-final-fields,nested-builder,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/mpeg-track-info-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackInfoBuilder >"$WORK/mpeg-track-info-builder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackInfoBuilder >"$WORK/mpeg-track-info-builder-candidate.txt"
cmp "$WORK/mpeg-track-info-builder-reference.txt" \
  "$WORK/mpeg-track-info-builder-candidate.txt"
grep --fixed-strings \
  'contracts=defaults,setter-storage,getter-storage,scalar-edges,reference-identity,build-order,build-freshness,build-shared-array,post-build-mutation,null-members,no-validation,identity-semantics,subclassable,private-fields,private-field-order,public-methods,method-descriptors,no-throws,member-counts,reflection' \
  "$WORK/mpeg-track-info-builder-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFileTrackProvider >"$WORK/mpeg-file-track-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFileTrackProvider >"$WORK/mpeg-file-track-provider-candidate.txt"
cmp "$WORK/mpeg-file-track-provider-reference.txt" \
  "$WORK/mpeg-file-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,no-fields,no-constructors,four-methods,consumer-identity,null-consumer,boolean-result,duration-result,full-width-duration,provide-dispatch,seek-dispatch,full-width-timecode,checked-failure-identity,implementation-compatibility,checked-throws,exception-order,reflection' \
  "$WORK/mpeg-file-track-provider-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegParseStopChecker >"$WORK/mpeg-parse-stop-checker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegParseStopChecker >"$WORK/mpeg-parse-stop-checker-candidate.txt"
cmp "$WORK/mpeg-parse-stop-checker-reference.txt" \
  "$WORK/mpeg-parse-stop-checker-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,functional-interface,no-fields,no-constructors,one-method,section-identity,null-section,start-phase,end-phase,boolean-result,unchecked-failure-identity,lambda-compatibility,loader-implementation,root-stop-rules,no-checked-throws,reflection' \
  "$WORK/mpeg-parse-stop-checker-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegReader >"$WORK/mpeg-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegReader >"$WORK/mpeg-reader-candidate.txt"
cmp "$WORK/mpeg-reader-reference.txt" "$WORK/mpeg-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,input-identity,data-wrapper,private-buffers,null-input,standard-child,unsigned-length,extended-length,zero-length,fourcc-bytes,parent-boundary,eof,truncated-header,io-identity,skip-target,skip-overflow,skip-wrapping,cause-identity,utf8,fourcc-charset,terminated-string,empty-string,malformed-replacement,negative-size,compressed-int,compressed-four-byte-limit,parse-flags,version-flags,section-copy,chain-freshness,chain-state,null-parent,subclassable,private-state,checked-throws,nested-metadata,reflection' \
  "$WORK/mpeg-reader-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegReaderChain >"$WORK/mpeg-reader-chain-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegReaderChain >"$WORK/mpeg-reader-chain-candidate.txt"
cmp "$WORK/mpeg-reader-chain-reference.txt" "$WORK/mpeg-reader-chain-candidate.txt"
grep --fixed-strings \
  'contracts=fluent-registration,handler-order,handler-state,nullable-registration,stop-replacement,ordinary-dispatch,exact-type-match,multiple-handlers,terminator-metadata,terminator-inert,skip-all-sections,parent-identity,versioned-dispatch,per-handler-flags,section-copy,pre-stop,post-stop,stop-phase-order,skip-after-stop,next-failure,checker-failure,handler-failure,skip-failure,failure-identity,null-type,null-handler,private-constructor,private-handler,checked-throws,member-counts,reflection' \
  "$WORK/mpeg-reader-chain-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSectionHandler >"$WORK/mpeg-section-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSectionHandler >"$WORK/mpeg-section-handler-candidate.txt"
cmp "$WORK/mpeg-section-handler-reference.txt" "$WORK/mpeg-section-handler-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,functional-interface,no-fields,no-constructors,one-method,section-identity,null-section,anonymous-implementation,lambda-compatibility,checked-failure-identity,unchecked-failure-identity,chain-dispatch,parent-identity,skip-after-handler,checked-throws,reflection' \
  "$WORK/mpeg-section-handler-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSectionInfo >"$WORK/mpeg-section-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSectionInfo >"$WORK/mpeg-section-info-candidate.txt"
cmp "$WORK/mpeg-section-info-reference.txt" "$WORK/mpeg-section-info-candidate.txt"
grep --fixed-strings \
  'contracts=field-order,offset-storage,length-storage,type-identity,null-type,full-width-longs,empty-type,no-validation,identity-equality,object-hash,object-string,subclassable,public-final-fields,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/mpeg-section-info-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegVersionedSectionHandler >"$WORK/mpeg-versioned-section-handler-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegVersionedSectionHandler >"$WORK/mpeg-versioned-section-handler-candidate.txt"
cmp "$WORK/mpeg-versioned-section-handler-reference.txt" \
  "$WORK/mpeg-versioned-section-handler-candidate.txt"
grep --fixed-strings \
  'contracts=public-abstract-interface,functional-interface,no-fields,no-constructors,one-method,section-identity,null-section,anonymous-implementation,lambda-compatibility,checked-failure-identity,unchecked-failure-identity,chain-dispatch,version-flags,section-copy,parent-identity,skip-after-handler,checked-throws,reflection' \
  "$WORK/mpeg-versioned-section-handler-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegVersionedSectionInfo >"$WORK/mpeg-versioned-section-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegVersionedSectionInfo >"$WORK/mpeg-versioned-section-info-candidate.txt"
cmp "$WORK/mpeg-versioned-section-info-reference.txt" \
  "$WORK/mpeg-versioned-section-info-candidate.txt"
grep --fixed-strings \
  'contracts=field-order,superclass-copy,offset-storage,length-storage,type-identity,version-storage,flags-storage,full-width-longs,full-width-ints,null-type,no-validation,identity-equality,object-hash,object-string,subclassable,public-final-fields,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/mpeg-versioned-section-info-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFragmentedFileTrackProvider >"$WORK/mpeg-fragmented-file-track-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegFragmentedFileTrackProvider >"$WORK/mpeg-fragmented-file-track-provider-candidate.txt"
cmp "$WORK/mpeg-fragmented-file-track-provider-reference.txt" \
  "$WORK/mpeg-fragmented-file-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,reader-identity,root-identity,initial-fragment-state,nullable-consumer,pre-fragment-initialise,pre-fragment-duration,pre-fragment-seek,subclassable,private-state,field-order,method-descriptors,checked-throws,reflection' \
  "$WORK/mpeg-fragmented-file-track-provider-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegGlobalSeekInfo >"$WORK/mpeg-global-seek-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegGlobalSeekInfo >"$WORK/mpeg-global-seek-info-candidate.txt"
cmp "$WORK/mpeg-global-seek-info-reference.txt" "$WORK/mpeg-global-seek-info-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,timescale-storage,entries-identity,offset-allocation,cumulative-time-offsets,cumulative-file-offsets,full-width-base-offset,overflow,nullable-rejection,empty-rejection,identity-equality,subclassable,public-final-fields,field-order,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/mpeg-global-seek-info-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSegmentEntry >"$WORK/mpeg-segment-entry-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegSegmentEntry >"$WORK/mpeg-segment-entry-candidate.txt"
cmp "$WORK/mpeg-segment-entry-reference.txt" "$WORK/mpeg-segment-entry-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,type-storage,size-storage,duration-storage,signed-extrema,no-validation,identity-equality,subclassable,public-final-fields,field-order,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/mpeg-segment-entry-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackFragmentHeader >"$WORK/mpeg-track-fragment-header-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTrackFragmentHeader >"$WORK/mpeg-track-fragment-header-candidate.txt"
cmp "$WORK/mpeg-track-fragment-header-reference.txt" \
  "$WORK/mpeg-track-fragment-header-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,scalar-storage,array-identity,signed-extrema,identity-equality,subclassable,builder-default,builder-setters,array-allocation,array-identity-through-build,default-size-fill,zero-default-null,flag-retention,unchecked-failures,negative-size-failure,public-final-fields,private-builder-state,field-order,method-descriptors,constructor-descriptors,no-throws,member-counts,reflection' \
  "$WORK/mpeg-track-fragment-header-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegStandardFileTrackProvider >"$WORK/mpeg-standard-file-track-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegStandardFileTrackProvider >"$WORK/mpeg-standard-file-track-provider-candidate.txt"
cmp "$WORK/mpeg-standard-file-track-provider-reference.txt" \
  "$WORK/mpeg-standard-file-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,reader-identity,collection-defaults,scalar-defaults,nullable-reader,consumer-identity,initialise-miss,initialise-cleanup,media-header-v0,media-header-v1,full-width-timescale,parser-attachment,pre-init-duration,pre-init-seek,pre-init-frames,public-interface,private-state,field-order,method-descriptors,checked-throws,inner-metadata,reflection' \
  "$WORK/mpeg-standard-file-track-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$mp3_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3ConstantRateSeeker \
  >"$WORK/mp3-constant-rate-seeker-reference.txt"
java -Xverify:all \
  -cp "$mp3_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3ConstantRateSeeker >"$WORK/mp3-constant-rate-seeker-candidate.txt"
cmp "$WORK/mp3-constant-rate-seeker-reference.txt" "$WORK/mp3-constant-rate-seeker-candidate.txt"
grep --fixed-strings \
  'contracts=meta-tags,offset,ordinary-frame,factory,interface,seekable,duration,frame-index,seek-delegation,full-width-timecode,clamping,reflection' \
  "$WORK/mp3-constant-rate-seeker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3ContainerProbe \
  "$ROOT/tests/media/fixtures/tone-mp3-vbr-id3.mp3" \
  >"$WORK/mp3-container-probe-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3ContainerProbe "$ROOT/tests/media/fixtures/tone-mp3-vbr-id3.mp3" \
  >"$WORK/mp3-container-probe-candidate.txt"
cmp "$WORK/mp3-container-probe-reference.txt" "$WORK/mp3-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,hint-presence,mime,extension,case-insensitive,combined-hints,null-hints,scan-miss,scan-boundary,reference-null,stream-null,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,id3-tag-state,private-state,throws,reflection' \
  "$WORK/mp3-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3FrameReader \
  "$ROOT/tests/media/fixtures/tone-mp3-vbr-id3.mp3" \
  >"$WORK/mp3-frame-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3FrameReader "$ROOT/tests/media/fixtures/tone-mp3-vbr-id3.mp3" \
  >"$WORK/mp3-frame-reader-candidate.txt"
cmp "$WORK/mp3-frame-reader-reference.txt" "$WORK/mp3-frame-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,buffer-identity,scan-success,frame-header,frame-size,frame-start,fill-buffer,next-frame,second-frame,scan-miss,scan-limit,append-scan-buffer,io-identity,private-state,reflection' \
  "$WORK/mp3-frame-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3Seeker \
  >"$WORK/mp3-seeker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3Seeker >"$WORK/mp3-seeker-candidate.txt"
cmp "$WORK/mp3-seeker-reference.txt" "$WORK/mp3-seeker-candidate.txt"
grep --fixed-strings \
  'contracts=public-interface,constant-rate-implementation,duration-dispatch,seekable-dispatch,frame-index-dispatch,full-width-timecode,checked-io,reflection' \
  "$WORK/mp3-seeker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3StreamSeeker >"$WORK/mp3-stream-seeker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3StreamSeeker >"$WORK/mp3-stream-seeker-candidate.txt"
cmp "$WORK/mp3-stream-seeker-reference.txt" "$WORK/mp3-stream-seeker-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,interface,duration-unknown,non-seekable,unsupported-seek,exception-message,input-untouched,null-input,full-width-timecode,checked-io,reflection' \
  "$WORK/mp3-stream-seeker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3TrackProvider \
  >"$WORK/mp3-track-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3TrackProvider >"$WORK/mp3-track-provider-candidate.txt"
cmp "$WORK/mp3-track-provider-reference.txt" "$WORK/mp3-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-state,metadata-defaults,id3-tags,provider-interface,seeker-dispatch,unknown-duration,non-seekable,parse-io,close,private-state,checked-io,reflection' \
  "$WORK/mp3-track-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMp3XingSeeker \
  >"$WORK/mp3-xing-seeker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMp3XingSeeker >"$WORK/mp3-xing-seeker-candidate.txt"
cmp "$WORK/mp3-xing-seeker-reference.txt" "$WORK/mp3-xing-seeker-candidate.txt"
grep --fixed-strings \
  'contracts=invalid-tag,missing-flags,xing-tag,required-flags,factory,interface,duration,seekable,percentile-mapping,position-clamp,frame-index,long-timecode,checked-io,reflection' \
  "$WORK/mp3-xing-seeker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegAacTrackConsumer \
  >"$WORK/mpeg-aac-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegAacTrackConsumer >"$WORK/mpeg-aac-track-consumer-candidate.txt"
cmp "$WORK/mpeg-aac-track-consumer-reference.txt" "$WORK/mpeg-aac-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,track-identity,direct-buffer,buffer-capacity,router-construction,decoder-config,default-config,initialise,get-track,chunking,remainder-chunk,input-position,reused-buffer,direct-chunks,empty-input,io-wrap,interruption,seek-forwarding,flush-forwarding,close-forwarding,private-fields,logger-owner,interface,throws,reflection' \
  "$WORK/mpeg-aac-track-consumer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAdtsContainerProbe \
  >"$WORK/adts-container-probe-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAdtsContainerProbe >"$WORK/adts-container-probe-candidate.txt"
cmp "$WORK/adts-container-probe-reference.txt" "$WORK/adts-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,hint-presence,mime,extension,case-insensitive,combined-hints,null-hints,header-detection,crc-header,scan-boundary,no-rewind,metadata-overlay,provider-order,supported-result,self-probe,null-settings,miss,null-reference,io-identity,runtime-identity,provider-failure,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/adts-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAdtsPacketHeader \
  >"$WORK/adts-packet-header-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAdtsPacketHeader >"$WORK/adts-packet-header-candidate.txt"
cmp "$WORK/adts-packet-header-reference.txt" "$WORK/adts-packet-header-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,protection,profile,sample-rate,channels,payload-length,raw-values,null-comparison,decoder-key,ignored-protection,ignored-payload,self-comparison,identity-semantics,subclassable,public-final-fields,reflection' \
  "$WORK/adts-packet-header-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAdtsStreamProvider \
  >"$WORK/adts-stream-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAdtsStreamProvider >"$WORK/adts-stream-provider-candidate.txt"
cmp "$WORK/adts-stream-provider-reference.txt" "$WORK/adts-stream-provider-candidate.txt"
grep --fixed-strings \
  'contracts=construction,input-identity,context-identity,private-state,initial-seek,seek-overwrite,empty-stream,io-wrapping,runtime-identity,packet-bounds,truncated-packet,complete-packet,decoder-configuration,decoder-reuse,reconfiguration,downstream-reset,decode-fill,decode-loop,interruption-identity,pipeline-creation,delayed-seek,native-output-buffer,close-order,close-finally,repeated-close,subclassable,throws,reflection' \
  "$WORK/adts-stream-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAdtsStreamReader \
  >"$WORK/adts-stream-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAdtsStreamReader >"$WORK/adts-stream-reader-candidate.txt"
cmp "$WORK/adts-stream-reader-reference.txt" "$WORK/adts-stream-reader-candidate.txt"
grep --fixed-strings \
  'contracts=construction,input-identity,scan-buffer,static-state,distance-bounds,unbounded-delegation,header-cache,next-packet,sticky-eof,rollover,sequential-packets,syncword,mpeg-id,layer,protection,crc-consumption,crc-eof,profiles,sample-rates,channels,payload-length,private-bit,ignored-flags,single-frame,io-identity,null-input,private-helpers,subclassable,throws,reflection' \
  "$WORK/adts-stream-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAacPacketRouter \
  >"$WORK/aac-packet-router-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAacPacketRouter >"$WORK/aac-packet-router-candidate.txt"
cmp "$WORK/aac-packet-router-reference.txt" "$WORK/aac-packet-router-candidate.txt"
grep --fixed-strings \
  'contracts=construction,context-identity,configurer-identity,null-construction,eager-logger,private-state,lazy-decoder,configurer-order,configurer-failure,null-configurer,decoder-reuse,input-identity,stream-info-lazy,pipeline-creation,pcm-format,native-output-buffer,delayed-seek,retained-seek,decode-loop,non-flush-mode,buffer-clear,interruption-identity,seek-forwarding,seek-overwrite,decoder-reset,decoder-close-failure,flush-noop,flush-mode,flush-loop,close-order,close-finally,close-failure,repeated-close,public-decoder,subclassable,generic-signatures,throws,reflection' \
  "$WORK/aac-packet-router-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateOpusPacketRouter \
  >"$WORK/opus-packet-router-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOpusPacketRouter >"$WORK/opus-packet-router-candidate.txt"
cmp "$WORK/opus-packet-router-reference.txt" "$WORK/opus-packet-router-candidate.txt"
grep --fixed-strings \
  'contracts=construction,context-identity,input-geometry,header-state,offered-frame,output-format,volume,private-state,eager-logger,heap-header,direct-header,position-preservation,direct-underflow,zero-frame,frame-size,format-rebuild,format-reuse,duration,timecode,seek-state,seek-forwarding,seek-failure-prefix,strict-seek-threshold,passthrough,input-window,frame-reuse,heap-staging,staging-growth,direct-identity,native-output,decode-limit,decode-order,interruption-identity,reencode-mode,passthrough-mode,mode-cleanup,volume-application,pipeline-creation,initial-seek,initialisation-cleanup,flush-noop,flush-forwarding,close-order,close-failure-prefix,buffer-cleanup,repeated-close,subclassable,private-helpers,throws,reflection' \
  "$WORK/opus-packet-router-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$flac_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacAudioTrack \
  >"$WORK/flac-audio-track-reference.txt"
java -Xverify:all \
  -cp "$flac_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacAudioTrack >"$WORK/flac-audio-track-candidate.txt"
cmp "$WORK/flac-audio-track-reference.txt" "$WORK/flac-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,loader-order,processing-context,read-callback,seek-callback,full-timecode,executor-control,input-ownership,load-failure,context-failure,null-executor,null-provider,identifier-dispatch,loop-failure,callback-failure,close-finally,close-replacement,subclassable,eager-logger,private-state,throws,reflection' \
  "$WORK/flac-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$flac_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacContainerProbe \
  >"$WORK/flac-container-probe-reference.txt"
java -Xverify:all \
  -cp "$flac_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacContainerProbe >"$WORK/flac-container-probe-candidate.txt"
cmp "$WORK/flac-container-probe-reference.txt" \
  "$WORK/flac-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,ignored-hints,null-hints,fourcc,case-sensitive,rewind-match,rewind-miss,initial-position,logging-order,loader-input,parse-order,provider-order,tag-overlay,exact-tag-keys,duration,metadata-fallback,supported-result,self-probe,null-settings,miss,null-reference,read-failure,seek-failure,parse-failure,provider-failure,null-tags,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,constant-fields,private-state,throws,reflection' \
  "$WORK/flac-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$flac_loader_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  com.sedmelluq.discord.lavaplayer.container.flac.GateFlacFileLoader \
  >"$WORK/flac-file-loader-reference.txt"
java -Xverify:all \
  -cp "$flac_loader_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  com.sedmelluq.discord.lavaplayer.container.flac.GateFlacFileLoader \
  >"$WORK/flac-file-loader-candidate.txt"
cmp "$WORK/flac-file-loader-reference.txt" \
  "$WORK/flac-file-loader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-input,data-input-wrapper,null-construction,static-fourcc,header-consumption,no-rewind,initial-position,invalid-header,short-header,stream-info,builder-order,metadata-flag,metadata-loop,block-results,data-input-identity,input-identity,builder-identity,first-frame-position,build-identity,load-order,context-identity,provider-info,provider-input,null-context,read-failure,stream-info-failure,builder-failure,get-stream-failure,block-failure,position-failure,set-position-failure,build-failure,provider-failure,subclassable,private-helper,private-state,throws,reflection' \
  "$WORK/flac-file-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacMetadataHeader \
  >"$WORK/flac-metadata-header-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacMetadataHeader >"$WORK/flac-metadata-header-candidate.txt"
cmp "$WORK/flac-metadata-header-reference.txt" \
  "$WORK/flac-metadata-header-candidate.txt"
grep --fixed-strings \
  'contracts=constants,last-flag,block-type,unsigned-length,big-endian,all-first-bytes,length-edges,minimum-input,trailing-input,input-snapshot,null-input,short-input,identity-semantics,subclassable,public-final-fields,constant-values,reflection' \
  "$WORK/flac-metadata-header-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$flac_metadata_reader_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  com.sedmelluq.discord.lavaplayer.container.flac.GateFlacMetadataReader \
  >"$WORK/flac-metadata-reader-reference.txt"
java -Xverify:all \
  -cp "$flac_metadata_reader_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  com.sedmelluq.discord.lavaplayer.container.flac.GateFlacMetadataReader \
  >"$WORK/flac-metadata-reader-candidate.txt"
cmp "$WORK/flac-metadata-reader-reference.txt" \
  "$WORK/flac-metadata-reader-candidate.txt"
grep --fixed-strings \
  'contracts=construction,charset,stream-header,stream-type,stream-size,stream-payload,last-inversion,read-order,unknown-skip,zero-skip,short-skip,block-continuation,seek-division,seek-remainder,seek-values,placeholder-count,seek-array-identity,comment-vendor-skip,little-endian,comment-count,utf8,split-limit,uppercase-locale,ignored-comments,declared-length-ignored,nulls,header-failure,payload-failure,stream-info-failure,seek-failure,builder-failure,tag-failure,private-helpers,static-methods,throws,subclassable,reflection' \
  "$WORK/flac-metadata-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacSeekPoint \
  >"$WORK/flac-seek-point-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacSeekPoint >"$WORK/flac-seek-point-candidate.txt"
cmp "$WORK/flac-seek-point-reference.txt" \
  "$WORK/flac-seek-point-candidate.txt"
grep --fixed-strings \
  'contracts=constant,zero-values,edge-values,raw-values,assignment-order,identity-semantics,subclassable,public-final-fields,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/flac-seek-point-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacStreamInfo \
  >"$WORK/flac-stream-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacStreamInfo >"$WORK/flac-stream-info-candidate.txt"
cmp "$WORK/flac-stream-info-reference.txt" \
  "$WORK/flac-stream-info-candidate.txt"
grep --fixed-strings \
  'contracts=constant,zero-vector,maximum-vector,bit-widths,big-endian,field-order,channel-offset,bits-offset,sample-count,md5-copy,input-snapshot,public-mutation,metadata-flag,trailing-input,short-input,null-input,identity-semantics,subclassable,public-final-fields,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/flac-stream-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacTrackInfo \
  >"$WORK/flac-track-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacTrackInfo >"$WORK/flac-track-info-candidate.txt"
cmp "$WORK/flac-track-info-reference.txt" \
  "$WORK/flac-track-info-candidate.txt"
grep --fixed-strings \
  'contracts=stream-identity,seek-array-identity,seek-count,tags-identity,first-frame-position,duration-formula,duration-truncation,duration-maximum,null-members,null-stream,zero-rate,mutation-visibility,identity-semantics,subclassable,public-final-fields,generic-tags,constructor-descriptor,no-throws,member-counts,reflection' \
  "$WORK/flac-track-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacTrackInfoBuilder \
  >"$WORK/flac-track-info-builder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacTrackInfoBuilder >"$WORK/flac-track-info-builder-candidate.txt"
cmp "$WORK/flac-track-info-builder-reference.txt" \
  "$WORK/flac-track-info-builder-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-identity,hash-map-default,get-stream-info,default-seek-points,default-count,default-position,set-seek-points,set-count,seek-array-identity,add-tag,tag-replacement,null-tags,set-first-frame-position,build-order,build-freshness,build-shared-state,build-duration,post-build-mutation,null-stream,zero-rate,identity-semantics,subclassable,private-fields,private-generic-field,public-methods,method-descriptors,no-throws,member-counts,reflection' \
  "$WORK/flac-track-info-builder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacTrackProvider \
  >"$WORK/flac-track-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacTrackProvider >"$WORK/flac-track-provider-candidate.txt"
cmp "$WORK/flac-track-provider-reference.txt" \
  "$WORK/flac-track-provider-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-state,pcm-format,reader-state,buffer-shapes,seek-binary-search,seek-position,seek-time,seek-default,frame-io-wrap,close,private-fields,private-methods,throws,identity-semantics,subclassable,reflection' \
  "$WORK/flac-track-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacFrameHeaderReader \
  >"$WORK/flac-frame-header-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacFrameHeaderReader >"$WORK/flac-frame-header-reader-candidate.txt"
cmp "$WORK/flac-frame-header-reader-reference.txt" \
  "$WORK/flac-frame-header-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,block-mapping,rate-mapping,channel-mapping,size-mapping,standard,explicit,inherited,left-side,mid-side,utf8-variable,invalid-block,rate-mismatch,channel-mismatch,size-mismatch,io-propagation,private-fields,private-methods,throws,identity-semantics,subclassable,reflection' \
  "$WORK/flac-frame-header-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacFrameInfo \
  >"$WORK/flac-frame-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacFrameInfo >"$WORK/flac-frame-info-candidate.txt"
cmp "$WORK/flac-frame-info-reference.txt" \
  "$WORK/flac-frame-info-candidate.txt"
grep --fixed-strings \
  'contracts=constructor-values,constructor-edges,null-channel,public-final-fields,identity-semantics,subclassable,enum-order,delta-channels,name-ordinal,defensive-values,value-of,lookup-failures,enum-collections,nested-enum,private-enum-state,generic-signature,reflection' \
  "$WORK/flac-frame-info-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacFrameReader \
  >"$WORK/flac-frame-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacFrameReader >"$WORK/flac-frame-reader-candidate.txt"
cmp "$WORK/flac-frame-reader-reference.txt" \
  "$WORK/flac-frame-reader-candidate.txt"
grep --fixed-strings \
  'contracts=temporary-buffer-constant,constructor,sync-scan,fixed-blocking,variable-blocking,eof,subframe-loop,crc-consumption,8-bit-increase,16-bit-copy,24-bit-decrease,left-side,right-side,mid-side,none-delta,sample-prefix,io-propagation,header-failure,subframe-failure,private-methods,throws,identity-semantics,subclassable,reflection' \
  "$WORK/flac-frame-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFlacSubFrameReader \
  >"$WORK/flac-sub-frame-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFlacSubFrameReader >"$WORK/flac-sub-frame-reader-candidate.txt"
cmp "$WORK/flac-sub-frame-reader-reference.txt" \
  "$WORK/flac-sub-frame-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,constant,verbatim,signed-samples,wasted-bits,delta-width,fixed-orders,rice-signed,rice-partitions,rice2-escape,lpc-orders,lpc-coefficients,lpc-shift,temporary-buffer,invalid-header,invalid-descriptor,invalid-residual,io-propagation,private-methods,throws,identity-semantics,subclassable,reflection' \
  "$WORK/flac-sub-frame-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMatroskaAacTrackConsumer \
  >"$WORK/matroska-aac-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaAacTrackConsumer >"$WORK/matroska-aac-track-consumer-candidate.txt"
cmp "$WORK/matroska-aac-track-consumer-reference.txt" \
  "$WORK/matroska-aac-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,track-identity,direct-buffer,buffer-capacity,router-construction,bound-configurer,codec-private-identity,configure-result-ignored,initialise,get-track,chunking,remainder-chunk,input-position,reused-buffer,direct-chunks,empty-input,null-input,interruption-position,seek-forwarding,flush-forwarding,close-forwarding,failure-identity,private-fields,private-method,logger-owner,interface,throws,identity-semantics,subclassable,reflection' \
  "$WORK/matroska-aac-track-consumer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMatroskaOpusTrackConsumer \
  >"$WORK/matroska-opus-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaOpusTrackConsumer >"$WORK/matroska-opus-track-consumer-candidate.txt"
cmp "$WORK/matroska-opus-track-consumer-reference.txt" \
  "$WORK/matroska-opus-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,track-identity,router-construction,input-frequency,float-to-int,input-channels,initialise,get-track,seek-forwarding,flush-forwarding,consume-forwarding,close-forwarding,null-input,failure-identity,private-fields,interface,throws,identity-semantics,subclassable,reflection' \
  "$WORK/matroska-opus-track-consumer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaTrackConsumer >"$WORK/matroska-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaTrackConsumer >"$WORK/matroska-track-consumer-candidate.txt"
cmp "$WORK/matroska-track-consumer-reference.txt" \
  "$WORK/matroska-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=dispatch,track-identity,long-identity,buffer-identity,checked-failures,interface,auto-closeable,reflection' \
  "$WORK/matroska-track-consumer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaVorbisTrackConsumer >"$WORK/matroska-vorbis-track-consumer-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaVorbisTrackConsumer >"$WORK/matroska-vorbis-track-consumer-candidate.txt"
cmp "$WORK/matroska-vorbis-track-consumer-reference.txt" \
  "$WORK/matroska-vorbis-track-consumer-candidate.txt"
grep --fixed-strings \
  'contracts=public-constructor,track-consumer-interface,method-signatures,checked-throws,subclassable,reflection' \
  "$WORK/matroska-vorbis-track-consumer-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaBlock >"$WORK/matroska-block-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaBlock >"$WORK/matroska-block-candidate.txt"
cmp "$WORK/matroska-block-reference.txt" \
  "$WORK/matroska-block-candidate.txt"
grep --fixed-strings \
  'contracts=interface,constructor,defaults,track-filter,single-frame,fixed-lacing,xiph-lacing,buffer-reuse,bounds,reflection' \
  "$WORK/matroska-block-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaCuePoint >"$WORK/matroska-cue-point-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaCuePoint >"$WORK/matroska-cue-point-candidate.txt"
cmp "$WORK/matroska-cue-point-reference.txt" \
  "$WORK/matroska-cue-point-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,primitive-identity,array-identity,null-array,field-finality,reflection' \
  "$WORK/matroska-cue-point-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaEbmlReader >"$WORK/matroska-ebml-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaEbmlReader >"$WORK/matroska-ebml-reader-candidate.txt"
cmp "$WORK/matroska-ebml-reader-reference.txt" \
  "$WORK/matroska-ebml-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,type-enum,fixed-size,variable-size,unsigned-null,signed,lacing,lengths-1-to-8,consumption,truncation,invalid-prefix,failures,throws,reflection' \
  "$WORK/matroska-ebml-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaElement >"$WORK/matroska-element-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaElement >"$WORK/matroska-element-candidate.txt"
cmp "$WORK/matroska-element-reference.txt" \
  "$WORK/matroska-element-candidate.txt"
grep --fixed-strings \
  'contracts=protected-constructor,protected-fields,defaults,getters,id-matching,data-type-identity,null-failures,unchecked-arithmetic,frozen-snapshot,base-class-copy,type-identity,subclassable,reflection' \
  "$WORK/matroska-element-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaElementType >"$WORK/matroska-element-type-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaElementType >"$WORK/matroska-element-type-candidate.txt"
cmp "$WORK/matroska-element-type-reference.txt" \
  "$WORK/matroska-element-type-candidate.txt"
grep --fixed-strings \
  'contracts=element-enum,data-type-enum,ordering,names,ordinals,byte-encodings,decoded-ids,data-types,find-known,find-unknown,values-clones,value-of,mutable-bytes,stable-id,reflection' \
  "$WORK/matroska-element-type-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaFileReader >"$WORK/matroska-file-reader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaFileReader >"$WORK/matroska-file-reader-candidate.txt"
cmp "$WORK/matroska-file-reader-reference.txt" \
  "$WORK/matroska-file-reader-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,element-reading,parent-bounds,typed-integers,typed-floats,typed-strings,bytes,skip,seek,block-filter,data-input,failures,throws,reflection' \
  "$WORK/matroska-file-reader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaFileTrack >"$WORK/matroska-file-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaFileTrack >"$WORK/matroska-file-track-candidate.txt"
cmp "$WORK/matroska-file-track-reference.txt" \
  "$WORK/matroska-file-track-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,field-identity,audio-details,enum-order,enum-lookup,parse,audio-parse,unknown-fields,defaults,malformed,throws,reflection' \
  "$WORK/matroska-file-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaMutableElement >"$WORK/matroska-mutable-element-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaMutableElement >"$WORK/matroska-mutable-element-candidate.txt"
cmp "$WORK/matroska-mutable-element-reference.txt" \
  "$WORK/matroska-mutable-element-candidate.txt"
grep --fixed-strings \
  'contracts=protected-constructor,subclassable,mutators,inherited-getters,full-width-state,null-state,reflection' \
  "$WORK/matroska-mutable-element-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaStreamingFile >"$WORK/matroska-streaming-file-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaStreamingFile >"$WORK/matroska-streaming-file-candidate.txt"
cmp "$WORK/matroska-streaming-file-reference.txt" \
  "$WORK/matroska-streaming-file-candidate.txt"
grep --fixed-strings \
  'contracts=constructor,defaults,metadata,track-list,track-identity,array-independence,seek-state,public-methods,subclassable,reflection' \
  "$WORK/matroska-streaming-file-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$matroska_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaAudioTrack >"$WORK/matroska-audio-track-reference.txt"
java -Xverify:all \
  -cp "$matroska_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaAudioTrack >"$WORK/matroska-audio-track-candidate.txt"
cmp "$WORK/matroska-audio-track-reference.txt" \
  "$WORK/matroska-audio-track-candidate.txt"
grep --fixed-strings \
  'contracts=track-info,input-identity,null-construction,file-read,duration-cast,processing-context,audio-only,codec-selection,opus-priority,last-fallback,initialise,read-callback,seek-callback,seek-track-index,full-timecode,executor-control,input-ownership,io-wrap,runtime-identity,context-failure,null-executor,no-supported-track,construction-failure,get-track-failure,initialise-cleanup,loop-cleanup,callback-cleanup,warning-close,subclassable,eager-logger,private-state,private-helpers,throws,reflection' \
  "$WORK/matroska-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$matroska_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaAudioTrack safety >"$WORK/matroska-audio-track-safety.txt"
grep --fixed-strings 'safety=single-selected-consumer' \
  "$WORK/matroska-audio-track-safety.txt" >/dev/null
java -Xverify:all \
  -cp "$matroska_classes_argument$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaContainerProbe >"$WORK/matroska-container-probe-reference.txt"
java -Xverify:all \
  -cp "$matroska_classes_argument$classpath_separator$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMatroskaContainerProbe >"$WORK/matroska-container-probe-candidate.txt"
cmp "$WORK/matroska-container-probe-reference.txt" \
  "$WORK/matroska-container-probe-candidate.txt"
grep --fixed-strings \
  'contracts=name,ignored-hints,null-hints,ebml,case-sensitive,rewind-match,rewind-miss,initial-position,logging-order,read-order,supported-codecs,unsupported-result,metadata-fallback,duration-truncation,result-shape,self-probe,null-settings,failures,track-factory,ignored-parameters,null-track-arguments,subclassable,eager-logger,constant-fields,private-state,throws,reflection' \
  "$WORK/matroska-container-probe-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePcmFilterFactory \
  >"$WORK/pcm-filter-factory-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePcmFilterFactory >"$WORK/pcm-filter-factory-candidate.txt"
cmp "$WORK/pcm-filter-factory-reference.txt" \
  "$WORK/pcm-filter-factory-candidate.txt"
grep --fixed-strings \
  'implementation=track,format,output,list-identity,mutable,repeated,nulls;failures=runtime-identity,null-receiver;reflection=public-abstract-interface,0-fields,0-constructors,1-method,generic-list,no-throws' \
  "$WORK/pcm-filter-factory-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GatePcmFormat \
  >"$WORK/pcm-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GatePcmFormat >"$WORK/pcm-format-candidate.txt"
cmp "$WORK/pcm-format-reference.txt" \
  "$WORK/pcm-format-candidate.txt"
grep --fixed-strings \
  'construction=order,positive,zero,negative,min,max,independent,subclass;reflection=public-concrete-object,2-public-final-fields,1-constructor,0-methods,no-throws' \
  "$WORK/pcm-format-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateResamplingPcmAudioFilter \
  >"$WORK/resampling-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateResamplingPcmAudioFilter >"$WORK/resampling-pcm-audio-filter-candidate.txt"
cmp "$WORK/resampling-pcm-audio-filter-reference.txt" \
  "$WORK/resampling-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'construction=configuration,channels,zero,null-downstream;streaming=upsample,downsample,offset,channel-isolation,4096-blocks,seek-reset;quality=low,medium,high,finite,bounded;lifecycle=flush-noop,close-idempotent,closed-rejection;failures=null-config,negative-channels,null-quality,downstream-interruption;reflection=public-concrete-object,float-filter,0-public-fields,1-constructor,4-public-methods,throws' \
  "$WORK/resampling-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateResamplingPcmAudioFilter --bounded-policy \
  >"$WORK/resampling-pcm-audio-filter-bounded.txt"
grep --fixed-strings \
  'bounded-policy=32768-output-frames,pre-dispatch-rejection' \
  "$WORK/resampling-pcm-audio-filter-bounded.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioPostProcessor \
  >"$WORK/audio-post-processor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioPostProcessor >"$WORK/audio-post-processor-candidate.txt"
cmp "$WORK/audio-post-processor-reference.txt" \
  "$WORK/audio-post-processor-candidate.txt"
grep --fixed-strings \
  'implementation=process,close,timecode,buffer-identity,state;exceptions=process-interrupted,null-receiver;reflection=public-abstract-interface,0-fields,0-constructors,2-methods,throws' \
  "$WORK/audio-post-processor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBufferingPostProcessor \
  >"$WORK/buffering-post-processor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBufferingPostProcessor >"$WORK/buffering-post-processor-candidate.txt"
cmp "$WORK/buffering-post-processor-reference.txt" \
  "$WORK/buffering-post-processor-candidate.txt"
grep --fixed-strings \
  'construction=identity,direct,capacity,format,nulls,ordering;process=clear,encode,input-identity,reuse,timecode,volume,format,data,consume;failures=encode,options,frame-buffer,interrupted,prefix,identity;close=repeat,failure,null-encoder;reflection=public-concrete-object,4-private-final-fields,1-interface,1-constructor,2-declared-methods,throws' \
  "$WORK/buffering-post-processor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateChannelCountPcmAudioFilter \
  >"$WORK/channel-count-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateChannelCountPcmAudioFilter >"$WORK/channel-count-pcm-audio-filter-candidate.txt"
cmp "$WORK/channel-count-pcm-audio-filter-reference.txt" \
  "$WORK/channel-count-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'construction=layout,capacity,derived-state,null,invalid;passthrough=array,buffer,identity,state,complete-frames;interleaved=mono-stereo,downmix,upmix,partial,reuse;split=float,short,downmix,upmix,reuse,offset,length;lifecycle=seek-clears-output,preserves-input,flush-close-noop;failures=downstream-identity,prefix,null,overflow;reflection=public-concrete-object,10-fields,1-interface,1-constructor,10-methods,3-private,throws' \
  "$WORK/channel-count-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateCompositeAudioFilter \
  >"$WORK/composite-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateCompositeAudioFilter >"$WORK/composite-audio-filter-candidate.txt"
cmp "$WORK/composite-audio-filter-reference.txt" \
  "$WORK/composite-audio-filter-candidate.txt"
grep --fixed-strings \
  'lifecycle=seek,flush,close,order,arguments,get-filters-once;callback-failures=runtime,checked,logged,continue,error-stops;boundaries=null-list,null-filter,iterator,get-filters,identity;reflection=public-abstract-object,1-private-static-final-field,1-interface,1-constructor,4-methods,protected-generic,throws' \
  "$WORK/composite-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFilterChainBuilder \
  >"$WORK/filter-chain-builder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFilterChainBuilder >"$WORK/filter-chain-builder-candidate.txt"
cmp "$WORK/filter-chain-builder-reference.txt" \
  "$WORK/filter-chain-builder-candidate.txt"
grep --fixed-strings \
  'construction=mutable-array-list,append,last,null,empty;adapters=float-identity,universal-identity,split,float,short,precedence,channel-count,zero;build=context,list,input,reuse,null;failures=unsupported,null-head,empty,negative,identity;reflection=public-concrete-object,1-private-final-generic-field,0-interfaces,1-constructor,6-methods,1-private,no-throws' \
  "$WORK/filter-chain-builder-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateFinalPcmAudioFilter \
  >"$WORK/final-pcm-audio-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateFinalPcmAudioFilter >"$WORK/final-pcm-audio-filter-candidate.txt"
cmp "$WORK/final-pcm-audio-filter-reference.txt" \
  "$WORK/final-pcm-audio-filter-candidate.txt"
grep --fixed-strings \
  'construction=format,direct-capacity,processor-identity,initial-state,nulls;process=short-array,split-short,short-buffer,float,offset,length,mono,clamp,chunking,timecodes,shared-buffer;seek=clear,skip,base,reset,path-units;flush=padding,empty;lifecycle=close-order,repeat;failures=process-interrupted,close-prefix,identity;reflection=public-concrete-object,8-private-fields,1-interface,1-constructor,10-methods,3-private,throws' \
  "$WORK/final-pcm-audio-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioFilterChain \
  >"$WORK/audio-filter-chain-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioFilterChain >"$WORK/audio-filter-chain-candidate.txt"
cmp "$WORK/audio-filter-chain-reference.txt" \
  "$WORK/audio-filter-chain-candidate.txt"
grep --fixed-strings \
  'construction=identity,nulls,no-copy;reflection=public-concrete-object,3-public-final-fields,1-constructor,0-methods,generics' \
  "$WORK/audio-filter-chain-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioPipeline \
  >"$WORK/audio-pipeline-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioPipeline >"$WORK/audio-pipeline-candidate.txt"
cmp "$WORK/audio-pipeline-reference.txt" \
  "$WORK/audio-pipeline-candidate.txt"
grep --fixed-strings \
  'construction=identity,no-copy,null-chain;process=float,short,buffer,split,identity,offset,length,interrupted,null-input;lifecycle=seek,flush,close,order,continue-on-failure;reflection=public-concrete-composite,2-private-final-fields,1-constructor,5-methods,generics,throws' \
  "$WORK/audio-pipeline-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAudioPipelineFactory \
  >"$WORK/audio-pipeline-factory-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAudioPipelineFactory >"$WORK/audio-pipeline-factory-candidate.txt"
cmp "$WORK/audio-pipeline-factory-reference.txt" \
  "$WORK/audio-pipeline-factory-candidate.txt"
grep --fixed-strings \
  'required=format,volume,factory,short-circuit,nulls;post=volume,buffering,encoder,identity,fixed-list;create=base,hot-swap,user,channel,resample,combined,first,order;failures=context,input,output,options,encoder-order;reflection=public-concrete-object,0-fields,1-constructor,3-methods,static,private-generic' \
  "$WORK/audio-pipeline-factory-candidate.txt" >/dev/null
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
# Retain inert/filter shell behavior while preventing legacy credential injection and refresh.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudHttpContextFilter reference \
  >"$WORK/sound-cloud-http-context-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudHttpContextFilter candidate \
  >"$WORK/sound-cloud-http-context-filter-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,2-fields,1-constructor,5-callbacks,no-op-lifecycle,false-exception,user-agent,retry-counter,cdn-pass-through,reflection;service=legacy-global-client-id-injection,substring-cdn-bypass,401-refresh' \
  "$WORK/sound-cloud-http-context-filter-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,2-fields,1-constructor,5-callbacks,no-op-lifecycle,false-exception,user-agent,retry-counter,cdn-pass-through,reflection;service=bounded-native-control-plane,strict-cdn-pass-through,no-client-id-injection,no-refresh' \
  "$WORK/sound-cloud-http-context-filter-candidate.txt" >/dev/null
# Retain the exact track shell while keeping legacy HLS segment playback out of scope.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudM3uAudioTrack reference \
  >"$WORK/sound-cloud-m3u-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudM3uAudioTrack candidate \
  >"$WORK/sound-cloud-m3u-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,4-fields,1-constructor,1-exported-method,capture,static-state,checked-exception,reflection;service=legacy-hls-playback-get' \
  "$WORK/sound-cloud-m3u-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,4-fields,1-constructor,1-exported-method,capture,static-state,checked-exception,reflection;service=bounded-progressive-only,no-http,hls-explicitly-unsupported' \
  "$WORK/sound-cloud-m3u-audio-track-candidate.txt" >/dev/null
# Preserve the exact immutable M3U descriptor and caller-supplied decoder factory identity.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudM3uInfo \
  >"$WORK/sound-cloud-m3u-info-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudM3uInfo \
  >"$WORK/sound-cloud-m3u-info-candidate.txt"
cmp "$WORK/sound-cloud-m3u-info-reference.txt" \
  "$WORK/sound-cloud-m3u-info-candidate.txt"
grep --fixed-strings \
  'public-concrete,2-fields,1-constructor,0-methods;identity,nulls,reflection' \
  "$WORK/sound-cloud-m3u-info-candidate.txt" >/dev/null
# Retain the exact decoder shell and lifecycle while rejecting legacy HLS MP3 playback.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudMp3SegmentDecoder reference \
  >"$WORK/sound-cloud-mp3-segment-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudMp3SegmentDecoder candidate \
  >"$WORK/sound-cloud-mp3-segment-decoder-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,1-field,1-constructor,4-methods,capture,no-op-lifecycle,generic-supplier,checked-signatures,reflection;service=legacy-mp3-segment-supplier' \
  "$WORK/sound-cloud-mp3-segment-decoder-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,1-field,1-constructor,4-methods,capture,no-op-lifecycle,generic-supplier,checked-signatures,reflection;service=bounded-progressive-only,no-supplier,hls-explicitly-unsupported' \
  "$WORK/sound-cloud-mp3-segment-decoder-candidate.txt" >/dev/null
# Retain exact stateful cleanup while rejecting legacy HLS Opus preparation and playback.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudOpusSegmentDecoder reference \
  >"$WORK/sound-cloud-opus-segment-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudOpusSegmentDecoder candidate \
  >"$WORK/sound-cloud-opus-segment-decoder-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,3-fields,1-constructor,4-exported-methods,capture,stateful-reset-close,generic-supplier,checked-signatures,reflection;service=legacy-opus-segment-supplier' \
  "$WORK/sound-cloud-opus-segment-decoder-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,3-fields,1-constructor,4-exported-methods,capture,stateful-reset-close,generic-supplier,checked-signatures,reflection;service=bounded-progressive-only,no-supplier,hls-explicitly-unsupported' \
  "$WORK/sound-cloud-opus-segment-decoder-candidate.txt" >/dev/null
# Retain the exact caller-defined playlist-loader SPI and generic track factory.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudPlaylistLoader \
  >"$WORK/sound-cloud-playlist-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudPlaylistLoader \
  >"$WORK/sound-cloud-playlist-loader-candidate.txt"
cmp "$WORK/sound-cloud-playlist-loader-reference.txt" \
  "$WORK/sound-cloud-playlist-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,1-method;dispatch,argument-identity,return-identity,nulls,unchecked,generic-function-parameter,reflection' \
  "$WORK/sound-cloud-playlist-loader-candidate.txt" >/dev/null
# Retain the exact caller-defined segment-decoder SPI and checked-failure contract.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudSegmentDecoder \
  >"$WORK/sound-cloud-segment-decoder-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudSegmentDecoder \
  >"$WORK/sound-cloud-segment-decoder-candidate.txt"
cmp "$WORK/sound-cloud-segment-decoder-reference.txt" \
  "$WORK/sound-cloud-segment-decoder-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,autocloseable,0-fields,0-constructors,3-methods;ordered-dispatch,boolean,longs,context-identity,nulls,checked-failures,reflection' \
  "$WORK/sound-cloud-segment-decoder-candidate.txt" >/dev/null
# Retain the exact nested caller-defined decoder-factory SPI and generic stream supplier.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudSegmentDecoderFactory \
  >"$WORK/sound-cloud-segment-decoder-factory-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudSegmentDecoderFactory \
  >"$WORK/sound-cloud-segment-decoder-factory-candidate.txt"
cmp "$WORK/sound-cloud-segment-decoder-factory-reference.txt" \
  "$WORK/sound-cloud-segment-decoder-factory-candidate.txt"
grep --fixed-strings \
  'public-static-abstract-interface,0-fields,0-constructors,1-method;dispatch,argument-identity,return-identity,nulls,unchecked,no-supplier-invocation,generic-supplier,reflection' \
  "$WORK/sound-cloud-segment-decoder-factory-candidate.txt" >/dev/null
# Retain all four exact caller-defined SoundCloud format getters.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateSoundCloudTrackFormat \
  >"$WORK/sound-cloud-track-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateSoundCloudTrackFormat \
  >"$WORK/sound-cloud-track-format-candidate.txt"
cmp "$WORK/sound-cloud-track-format-reference.txt" \
  "$WORK/sound-cloud-track-format-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,4-methods;ordered-dispatch,return-identity,nulls,unchecked,reflection' \
  "$WORK/sound-cloud-track-format-candidate.txt" >/dev/null
# Retain the exact generic M3U stream bridge, including lazy chaining and nested cleanup.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateM3uStreamAudioTrack \
  >"$WORK/m3u-stream-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateM3uStreamAudioTrack \
  >"$WORK/m3u-stream-audio-track-candidate.txt"
cmp "$WORK/m3u-stream-audio-track-reference.txt" \
  "$WORK/m3u-stream-audio-track-candidate.txt"
grep --fixed-strings \
  'public-abstract,delegated,0-fields,1-constructor,4-exported-methods;construction,hooks,lazy-chain,segment-order,identity,cleanup,suppression,failures,reflection' \
  "$WORK/m3u-stream-audio-track-candidate.txt" >/dev/null
# Retain the exact generic M3U provider family with bounded fake-response coverage.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateM3uStreamSegmentUrlProvider \
  >"$WORK/m3u-stream-segment-url-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateM3uStreamSegmentUrlProvider \
  >"$WORK/m3u-stream-segment-url-provider-candidate.txt"
cmp "$WORK/m3u-stream-segment-url-provider-reference.txt" \
  "$WORK/m3u-stream-segment-url-provider-candidate.txt"
grep --fixed-strings \
  'provider=public-abstract,4-fields,2-constructors,13-methods;nested=protected-static-values,5-fields,1-constructor;behavior=base-url,uri-resolution,channels,segments,generics,selection,lazy-wait,timeouts,response-ownership,identity,failures,reflection' \
  "$WORK/m3u-stream-segment-url-provider-candidate.txt" >/dev/null
# Retain the exact MPEG-TS/PES/ADTS joined-stream delegation chain.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateMpegTsM3uStreamAudioTrack \
  >"$WORK/mpeg-ts-m3u-stream-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateMpegTsM3uStreamAudioTrack \
  >"$WORK/mpeg-ts-m3u-stream-audio-track-candidate.txt"
cmp "$WORK/mpeg-ts-m3u-stream-audio-track-reference.txt" \
  "$WORK/mpeg-ts-m3u-stream-audio-track-candidate.txt"
grep --fixed-strings \
  'public-abstract,m3u-super,0-fields,1-constructor,1-method;construction,track-info,executor,ts-adts,pes,elementary-type,raw-identity,no-eager-read,nulls,failure-identity,reflection' \
  "$WORK/mpeg-ts-m3u-stream-audio-track-candidate.txt" >/dev/null
# Retain the Twitch constant holder shell and all five package-visible literal values.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateTwitchConstants \
  >"$WORK/twitch-constants-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTwitchConstants \
  >"$WORK/twitch-constants-candidate.txt"
cmp "$WORK/twitch-constants-reference.txt" "$WORK/twitch-constants-candidate.txt"
grep --fixed-strings \
  'public-object-shell,5-package-constants,1-constructor,0-methods;construction,urls,image-template,metadata-payload,access-token-payload,format-substitution,constant-identity,reflection' \
  "$WORK/twitch-constants-candidate.txt" >/dev/null
# Preserve the stable manager/SPI shell while replacing homepage scraping and undocumented
# GraphQL metadata with caller-credentialed, bounded native Helix resolution.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateTwitchStreamAudioSourceManager reference \
  >"$WORK/twitch-stream-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTwitchStreamAudioSourceManager candidate "$native" \
  >"$WORK/twitch-stream-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,15-exported-methods,source-name,legacy-route,empty-details,decode,requests,headers,http-config,shutdown;service=legacy-homepage-graphql' \
  "$WORK/twitch-stream-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,6-fields,1-constructor,15-exported-methods,source-name,legacy-route,empty-details,decode,requests,headers,http-config,shutdown;service=current-helix,explicit-credentials,no-homepage-scrape,bounded-native' \
  "$WORK/twitch-stream-audio-source-manager-candidate.txt" >/dev/null
# Preserve track construction, identity, and protected access while current live playback bypasses
# the legacy provider path in favor of the bounded native Twitch pipeline.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateTwitchStreamAudioTrack reference \
  >"$WORK/twitch-stream-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTwitchStreamAudioTrack candidate "$native" \
  >"$WORK/twitch-stream-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,mpeg-super,3-fields,1-constructor,6-exported-methods;construction,channel,provider,http,source-identity,shallow-clone,reflection;service=legacy-provider-mpeg' \
  "$WORK/twitch-stream-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,mpeg-super,3-fields,1-constructor,6-exported-methods;construction,channel,provider,http,source-identity,shallow-clone,reflection;service=current-native-bounded-hls,no-legacy-provider-playback' \
  "$WORK/twitch-stream-audio-track-candidate.txt" >/dev/null
# Retain construction, Twitch VIDEO quality selection, and manager-owned segment request headers;
# the legacy GraphQL-token/Usher playlist path is retired in favor of bounded native playback.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTwitchStreamSegmentUrlProvider reference \
  >"$WORK/twitch-stream-segment-url-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateTwitchStreamSegmentUrlProvider candidate \
  >"$WORK/twitch-stream-segment-url-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,m3u-provider-super,6-fields,1-constructor,3-exported-methods;construction,video-quality,manager-get-request,reflection;service=legacy-graphql-token-and-usher-playlist' \
  "$WORK/twitch-stream-segment-url-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,m3u-provider-super,6-fields,1-constructor,3-exported-methods;construction,video-quality,manager-get-request,reflection;service=deterministic-retired-provider,bounded-native-playback' \
  "$WORK/twitch-stream-segment-url-provider-candidate.txt" >/dev/null
# Preserve Bandcamp manager construction, empty serialization details, reconstruction, HTTP
# configuration, and lifecycle while routing current track/album metadata through bounded native
# public-page parsing and retiring the legacy search scraper.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBandcampAudioSourceManager reference \
  >"$WORK/bandcamp-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBandcampAudioSourceManager candidate "$native" \
  >"$WORK/bandcamp-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,6-fields,2-constructors,9-exported-methods;construction,allow-search,source-name,empty-details,decode,http-config,shutdown,reflection;service=legacy-search-and-unbounded-html' \
  "$WORK/bandcamp-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,6-fields,2-constructors,9-exported-methods;construction,allow-search,source-name,empty-details,decode,http-config,shutdown,reflection;service=current-track-album-only,bounded-native,no-search-scrape' \
  "$WORK/bandcamp-audio-source-manager-candidate.txt" >/dev/null
# Preserve Bandcamp track construction, manager identity, and cloning while replacing the legacy
# direct HTML/MP3 path with bounded native public-page discovery and media processing.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBandcampAudioTrack reference \
  >"$WORK/bandcamp-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBandcampAudioTrack candidate "$native" \
  >"$WORK/bandcamp-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,3-exported-methods;construction,source-identity,shallow-clone,reflection;service=legacy-page-html-and-direct-mp3' \
  "$WORK/bandcamp-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,3-exported-methods;construction,source-identity,shallow-clone,reflection;service=current-native-bounded-page-and-mp3' \
  "$WORK/bandcamp-audio-track-candidate.txt" >/dev/null
# Preserve the Beam manager shell and empty details while recognizing strict historical routes as
# terminal no-track results without contacting the discontinued Mixer service.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBeamAudioSourceManager reference \
  >"$WORK/beam-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBeamAudioSourceManager candidate "$native" \
  >"$WORK/beam-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,3-fields,1-constructor,9-exported-methods;construction,source-name,regex,empty-details,decode,http-config,no-op-shutdown,reflection;service=legacy-mixer-api' \
  "$WORK/beam-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,3-fields,1-constructor,9-exported-methods;construction,source-name,regex,empty-details,decode,http-config,no-op-shutdown,reflection;service=retired-terminal-no-track,no-network' \
  "$WORK/beam-audio-source-manager-candidate.txt" >/dev/null
# Preserve Beam track construction, private composite parsing, provider/HTTP/source identity, and
# cloning while making playback fail locally before touching the retired Mixer service.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBeamAudioTrack reference \
  >"$WORK/beam-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBeamAudioTrack candidate "$native" \
  >"$WORK/beam-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,mpeg-ts-m3u-super,3-fields,1-constructor,5-exported-methods;construction,source-identity,segment-provider,private-parsing,http-delegation,shallow-clone,reflection;service=legacy-mixer-hls' \
  "$WORK/beam-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,mpeg-ts-m3u-super,3-fields,1-constructor,5-exported-methods;construction,source-identity,segment-provider,private-parsing,http-delegation,shallow-clone,reflection;service=retired-stable-failure,no-network' \
  "$WORK/beam-audio-track-candidate.txt" >/dev/null
# Preserve local Beam segment provider contracts while replacing retired Mixer manifest discovery
# with a stable failure before HTTP use.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateBeamSegmentUrlProvider reference \
  >"$WORK/beam-segment-url-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateBeamSegmentUrlProvider candidate \
  >"$WORK/beam-segment-url-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,m3u-provider-super,3-fields,1-constructor,3-exported-methods;construction,name-quality,cached-playlist,http-get,reflection;service=legacy-mixer-manifest-fetch' \
  "$WORK/beam-segment-url-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,m3u-provider-super,3-fields,1-constructor,3-exported-methods;construction,name-quality,cached-playlist,http-get,reflection;service=retired-stable-failure,no-network' \
  "$WORK/beam-segment-url-provider-candidate.txt" >/dev/null
# Preserve Getyarn manager serialization and HTTP façade contracts while routing recognized bounded
# clip pages to a terminal no-track result before the retired page scraper can perform network I/O.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateGetyarnAudioSourceManager reference \
  >"$WORK/getyarn-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateGetyarnAudioSourceManager candidate "$native" \
  >"$WORK/getyarn-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,http-configurable-audio-source,2-fields,1-constructor,9-exported-methods;construction,source-name,regex,empty-details,decode,http-config,no-op-shutdown,reflection;service=legacy-page-scraper' \
  "$WORK/getyarn-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,http-configurable-audio-source,2-fields,1-constructor,9-exported-methods;construction,source-name,regex,empty-details,decode,http-config,no-op-shutdown,reflection;service=compatibility-terminal-no-track,no-network' \
  "$WORK/getyarn-audio-source-manager-candidate.txt" >/dev/null
# Preserve track construction, source identity, clone, and reflection contracts while routing
# direct legacy media playback to a stable failure before source, executor, or network access.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateGetyarnAudioTrack reference \
  >"$WORK/getyarn-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateGetyarnAudioTrack candidate "$native" \
  >"$WORK/getyarn-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-audio-track-super,2-fields,1-constructor,2-exported-methods;construction,source-identity,shallow-clone,reflection;service=legacy-direct-media-http' \
  "$WORK/getyarn-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-audio-track-super,2-fields,1-constructor,2-exported-methods;construction,source-identity,shallow-clone,reflection;service=retired-stable-failure,no-network' \
  "$WORK/getyarn-audio-track-candidate.txt" >/dev/null
# Preserve HTTP manager construction, normalization, known-container, serialization, façade, and
# lifecycle contracts while routing unknown-container probing through bounded native HTTP policy.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateHttpAudioSourceManager reference \
  >"$WORK/http-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHttpAudioSourceManager candidate "$native" \
  >"$WORK/http-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,probing-super,http-configurable,1-field,2-constructors,11-exported-methods;construction,normalization,known-container,track-creation,http-config,serialization,no-op-shutdown,reflection;service=legacy-direct-http-probe' \
  "$WORK/http-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,probing-super,http-configurable,1-field,2-constructors,11-exported-methods;construction,normalization,known-container,track-creation,http-config,serialization,no-op-shutdown,reflection;service=current-bounded-native-http,ssrf-guarded' \
  "$WORK/http-audio-source-manager-candidate.txt" >/dev/null
# Preserve HTTP track construction, descriptor/source identities, shallow cloning, and reflection
# while routing playback through Mantle's bounded native HTTP and media pipeline.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateHttpAudioTrack reference \
  >"$WORK/http-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateHttpAudioTrack candidate "$native" \
  >"$WORK/http-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-audio-track-super,3-fields,1-constructor,4-exported-methods;construction,descriptor-identity,source-identity,shallow-clone,reflection;service=legacy-direct-http-delegate,resource-closing' \
  "$WORK/http-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-audio-track-super,3-fields,1-constructor,4-exported-methods;construction,descriptor-identity,source-identity,shallow-clone,reflection;service=current-bounded-native-http,ssrf-guarded' \
  "$WORK/http-audio-track-candidate.txt" >/dev/null
# Preserve manager identity, serialization, reconstruction, HTTP configuration, and lifecycle while
# current metadata resolution bypasses hidden viewer JWTs for bounded public config or caller auth.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateVimeoAudioSourceManager reference \
  >"$WORK/vimeo-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVimeoAudioSourceManager candidate "$native" \
  >"$WORK/vimeo-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,3-fields,1-constructor,11-exported-methods;construction,source-name,empty-details,decode,http-config,shutdown,reflection;service=legacy-viewer-jwt-api-and-config' \
  "$WORK/vimeo-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,3-fields,1-constructor,11-exported-methods;construction,source-name,empty-details,decode,http-config,shutdown,reflection;service=current-public-config-or-caller-token,bounded-native,no-viewer-jwt' \
  "$WORK/vimeo-audio-source-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateVimeoPlaybackFormat reference \
  >"$WORK/vimeo-playback-format-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVimeoPlaybackFormat candidate \
  >"$WORK/vimeo-playback-format-candidate.txt"
cmp "$WORK/vimeo-playback-format-reference.txt" "$WORK/vimeo-playback-format-candidate.txt"
grep --fixed-strings \
  'public-static,object-super,2-public-final-fields,1-public-constructor;url-and-hls-value-identity,null-preserved,reflection' \
  "$WORK/vimeo-playback-format-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateVimeoAudioTrack reference \
  >"$WORK/vimeo-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateVimeoAudioTrack candidate "$native" \
  >"$WORK/vimeo-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,4-exported-methods;capture,source-identity,relative-url,shallow-clone,reflection;service=legacy-viewer-jwt-config,hls-or-mpeg' \
  "$WORK/vimeo-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,4-exported-methods;capture,source-identity,relative-url,shallow-clone,reflection;service=current-native-bounded-progressive-mp4,no-viewer-jwt-or-legacy-hls' \
  "$WORK/vimeo-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateAbstractYandexMusicApiLoader reference \
  >"$WORK/abstract-yandex-music-api-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateAbstractYandexMusicApiLoader candidate \
  >"$WORK/abstract-yandex-music-api-loader-candidate.txt"
grep --fixed-strings \
  'common=public-abstract,object-super,yandex-api-loader,1-protected-field,1-package-constructor,3-exported-methods;generic-extractor,http-config-identity,mutable-manager,repeatable-warning-close,reflection;service=legacy-arbitrary-url-get-unbounded-json' \
  "$WORK/abstract-yandex-music-api-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-abstract,object-super,yandex-api-loader,1-protected-field,1-package-constructor,3-exported-methods;generic-extractor,http-config-identity,mutable-manager,repeatable-warning-close,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/abstract-yandex-music-api-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicApiExtractor \
  >"$WORK/yandex-music-api-extractor-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicApiExtractor \
  >"$WORK/yandex-music-api-extractor-candidate.txt"
cmp "$WORK/yandex-music-api-extractor-reference.txt" \
  "$WORK/yandex-music-api-extractor-candidate.txt"
grep --fixed-strings \
  'protected-static-generic-interface,no-reflection-super,1-type-variable,1-public-abstract-method;erased-object-return,generic-T-return,http-json-parameters,checked-exception,proxy-invocation,reflection' \
  "$WORK/yandex-music-api-extractor-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYandexMusicDirectUrlLoader reference \
  >"$WORK/default-yandex-music-direct-url-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYandexMusicDirectUrlLoader candidate \
  >"$WORK/default-yandex-music-direct-url-loader-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,direct-url-interface,3-private-constants,1-constructor,1-exported-method;construction,http-config,private-helper-signatures,reflection;service=legacy-api-json-and-storage-xml,md5-signed-direct-url' \
  "$WORK/default-yandex-music-direct-url-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,direct-url-interface,3-private-constants,1-constructor,1-exported-method;construction,http-config,private-helper-signatures,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-yandex-music-direct-url-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYandexMusicPlaylistLoader reference \
  >"$WORK/default-yandex-music-playlist-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYandexMusicPlaylistLoader candidate \
  >"$WORK/default-yandex-music-playlist-loader-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,track-loader-super,playlist-interface,4-fields,1-constructor,4-exported-methods;constants,construction,http-config,pure-error-parser,executor-shutdown,private-signatures,reflection;service=legacy-api-json,unbounded-cached-track-fanout' \
  "$WORK/default-yandex-music-playlist-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,track-loader-super,playlist-interface,4-fields,1-constructor,4-exported-methods;constants,construction,http-config,pure-error-parser,executor-shutdown,private-signatures,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-yandex-music-playlist-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYandexMusicTrackLoader reference \
  >"$WORK/default-yandex-music-track-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYandexMusicTrackLoader candidate \
  >"$WORK/default-yandex-music-track-loader-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,track-loader-interface,1-private-constant,1-constructor,1-exported-method;construction,http-config,generic-factory,synthetic-lambda,reflection;service=legacy-query-api-json' \
  "$WORK/default-yandex-music-track-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,track-loader-interface,1-private-constant,1-constructor,1-exported-method;construction,http-config,generic-factory,synthetic-lambda,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-yandex-music-track-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYandexSearchProvider reference \
  >"$WORK/default-yandex-search-provider-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYandexSearchProvider candidate \
  >"$WORK/default-yandex-search-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,search-loader-interface,4-private-constants,1-constructor,1-exported-method;construction,http-config,compiled-pattern,generic-factory,private-helper-and-lambda-signatures,invalid-query-fallthrough,reflection;service=legacy-query-api-json' \
  "$WORK/default-yandex-search-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,abstract-api-super,search-loader-interface,4-private-constants,1-constructor,1-exported-method;construction,http-config,compiled-pattern,generic-factory,private-helper-and-lambda-signatures,invalid-query-fallthrough,reflection;service=deterministic-no-network,current-bounded-native-search' \
  "$WORK/default-yandex-search-provider-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexHttpContextFilter reference \
  >"$WORK/yandex-http-context-filter-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexHttpContextFilter candidate \
  >"$WORK/yandex-http-context-filter-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-super,http-context-filter-interface,1-private-static-field,1-constructor,6-exported-methods;construction,static-setter,cookie-reset,context-close,non-secret-request-headers,repetition,false-retry-policy,reflection;service=legacy-global-oauth-forwarding' \
  "$WORK/yandex-http-context-filter-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-super,http-context-filter-interface,1-private-static-field,1-constructor,6-exported-methods;construction,static-setter,cookie-reset,context-close,non-secret-request-headers,repetition,false-retry-policy,reflection;service=global-oauth-rejected,current-origin-bounded-manager-auth' \
  "$WORK/yandex-http-context-filter-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicApiLoader \
  >"$WORK/yandex-music-api-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicApiLoader >"$WORK/yandex-music-api-loader-candidate.txt"
cmp "$WORK/yandex-music-api-loader-reference.txt" \
  "$WORK/yandex-music-api-loader-candidate.txt"
grep --fixed-strings \
  'interface=public-abstract,object-root,0-superinterfaces,0-fields,0-constructors,2-abstract-methods;implementation=configuration-identity,null-identity,repeatable-shutdown;reflection=exact' \
  "$WORK/yandex-music-api-loader-reference.txt" >/dev/null
# Preserve the manager, loader, serialization, HTTP configuration, and lifecycle shell while
# routing current service work through Mantle's bounded caller-token Yandex implementation.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicAudioSourceManager reference \
  >"$WORK/yandex-music-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicAudioSourceManager candidate "$native" \
  >"$WORK/yandex-music-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,19-fields,3-constructors,17-methods;construction,source-name,empty-details,track-factory,loader-identity,http-config,shutdown,reflection;service=legacy-loader-routing' \
  "$WORK/yandex-music-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,audio-source-http-configurable,19-fields,3-constructors,17-methods;construction,source-name,empty-details,track-factory,loader-identity,http-config,shutdown,reflection;service=current-native-bounded-authenticated-routing,no-global-oauth' \
  "$WORK/yandex-music-audio-source-manager-candidate.txt" >/dev/null
# Preserve the track shell and identity contracts while making playback use the bounded native MP3
# bridge with explicit caller credentials instead of the legacy direct-URL loader.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicAudioTrack reference \
  >"$WORK/yandex-music-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicAudioTrack candidate "$native" \
  >"$WORK/yandex-music-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,3-methods;construction,track-info,source-identity,fresh-clone,reflection;service=legacy-direct-url-http-mp3' \
  "$WORK/yandex-music-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,3-methods;construction,track-info,source-identity,fresh-clone,reflection;service=current-native-bounded-mp3,explicit-token,no-legacy-direct-loader' \
  "$WORK/yandex-music-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicDirectUrlLoader \
  >"$WORK/yandex-music-direct-url-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicDirectUrlLoader \
  >"$WORK/yandex-music-direct-url-loader-candidate.txt"
cmp "$WORK/yandex-music-direct-url-loader-reference.txt" \
  "$WORK/yandex-music-direct-url-loader-candidate.txt"
grep --fixed-strings \
  'interface=public-abstract,object-root,yandex-api-loader-superinterface,0-fields,0-constructors,1-declared-method;implementation=argument-result-identity,null-identity,inherited-configuration-shutdown;reflection=exact' \
  "$WORK/yandex-music-direct-url-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicPlaylistLoader \
  >"$WORK/yandex-music-playlist-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicPlaylistLoader \
  >"$WORK/yandex-music-playlist-loader-candidate.txt"
cmp "$WORK/yandex-music-playlist-loader-reference.txt" \
  "$WORK/yandex-music-playlist-loader-candidate.txt"
grep --fixed-strings \
  'interface=public-abstract,object-root,yandex-api-loader-superinterface,0-fields,0-constructors,2-overloaded-methods;generic-factory=track-info-to-track;implementation=overload-argument-result-identity,null-identity,inherited-configuration-shutdown;reflection=exact' \
  "$WORK/yandex-music-playlist-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicSearchResultLoader \
  >"$WORK/yandex-music-search-result-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicSearchResultLoader \
  >"$WORK/yandex-music-search-result-loader-candidate.txt"
cmp "$WORK/yandex-music-search-result-loader-reference.txt" \
  "$WORK/yandex-music-search-result-loader-candidate.txt"
grep --fixed-strings \
  'interface=public-abstract,object-root,yandex-api-loader-superinterface,0-fields,0-constructors,1-declared-method;generic-factory=track-info-to-track;implementation=query-playlist-factory-result-identity,null-identity,inherited-configuration-shutdown;reflection=exact' \
  "$WORK/yandex-music-search-result-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicTrackLoader \
  >"$WORK/yandex-music-track-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicTrackLoader \
  >"$WORK/yandex-music-track-loader-candidate.txt"
cmp "$WORK/yandex-music-track-loader-reference.txt" \
  "$WORK/yandex-music-track-loader-candidate.txt"
grep --fixed-strings \
  'interface=public-abstract,object-root,yandex-api-loader-superinterface,0-fields,0-constructors,1-declared-method;generic-factory=track-info-to-track;implementation=track-album-factory-result-identity,null-identity,inherited-configuration-shutdown;reflection=exact' \
  "$WORK/yandex-music-track-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYandexMusicUtils \
  >"$WORK/yandex-music-utils-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYandexMusicUtils \
  >"$WORK/yandex-music-utils-candidate.txt"
cmp "$WORK/yandex-music-utils-reference.txt" \
  "$WORK/yandex-music-utils-candidate.txt"
grep --fixed-strings \
  'class=public-concrete,object-root,0-interfaces,1-private-constant,1-constructor,2-declared-methods;generic-factory=track-info-to-track;extraction=wrapper,direct,artist-order,metadata,url,cover-priority-og-album-null,factory-result-identity;reflection=exact' \
  "$WORK/yandex-music-utils-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYoutubeLinkRouter \
  >"$WORK/default-youtube-link-router-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYoutubeLinkRouter \
  >"$WORK/default-youtube-link-router-candidate.txt"
cmp "$WORK/default-youtube-link-router-reference.txt" \
  "$WORK/default-youtube-link-router-candidate.txt"
grep --fixed-strings \
  'class=public-concrete,object-root,youtube-router-interface,9-private-fields,1-constructor,11-declared-methods;routes=search,music,direct-video,direct-playlist,main-watch-playlist-mix-anonymous,short,embed,shorts,live,none,unsupported-null,truncate,duplicate-first,null-result;protected=7,generic-T;reflection=exact' \
  "$WORK/default-youtube-link-router-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYoutubePlaylistLoader reference \
  >"$WORK/default-youtube-playlist-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYoutubePlaylistLoader candidate \
  >"$WORK/default-youtube-playlist-loader-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,playlist-loader-interface,1-private-volatile-field,1-constructor,2-exported-methods;default-page-count-6,unrestricted-mutation,generic-factory,private-signatures,synthetic-helper,reflection;service=legacy-innertube-browse,mutable-page-count' \
  "$WORK/default-youtube-playlist-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,playlist-loader-interface,1-private-volatile-field,1-constructor,2-exported-methods;default-page-count-6,unrestricted-mutation,generic-factory,private-signatures,synthetic-helper,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-youtube-playlist-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYoutubeTrackDetails reference \
  >"$WORK/default-youtube-track-details-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYoutubeTrackDetails candidate \
  >"$WORK/default-youtube-track-details-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,track-details-interface,4-private-fields,1-constructor,7-declared-methods,temporal-nest;constructor-identity,player-script,generic-formats,modern-vod-live-error,legacy-vod-live-error,thumbnail-duration-uri,reflection;service=legacy-four-extractor-chain,streaming-format' \
  "$WORK/default-youtube-track-details-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,track-details-interface,4-private-fields,1-constructor,7-declared-methods,temporal-nest;constructor-identity,player-script,generic-formats,modern-vod-live-error,legacy-vod-live-error,thumbnail-duration-uri,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-youtube-track-details-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateDefaultYoutubeTrackDetailsLoader reference \
  >"$WORK/default-youtube-track-details-loader-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateDefaultYoutubeTrackDetailsLoader candidate \
  >"$WORK/default-youtube-track-details-loader-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,details-loader-interface,2-private-fields,1-constructor,9-declared-methods,2-nested-declarations;constructor-empty-cache,playability-matrix,reason-fallback-simple-runs,synthetic-helper,exceptions,reflection;service=legacy-innertube-embed-player-script-cache' \
  "$WORK/default-youtube-track-details-loader-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,details-loader-interface,2-private-fields,1-constructor,9-declared-methods,2-nested-declarations;constructor-empty-cache,playability-matrix,reason-fallback-simple-runs,synthetic-helper,exceptions,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/default-youtube-track-details-loader-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeCachedPlayerScript \
  >"$WORK/youtube-cached-player-script-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeCachedPlayerScript \
  >"$WORK/youtube-cached-player-script-candidate.txt"
cmp "$WORK/youtube-cached-player-script-reference.txt" \
  "$WORK/youtube-cached-player-script-candidate.txt"
grep --fixed-strings \
  'shape=protected-static-member,object-root,2-public-final-fields,1-public-constructor,0-methods;capture=reference,null,long-extremes;identity=object' \
  "$WORK/youtube-cached-player-script-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeInfoStatus \
  >"$WORK/youtube-info-status-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeInfoStatus \
  >"$WORK/youtube-info-status-candidate.txt"
cmp "$WORK/youtube-info-status-reference.txt" \
  "$WORK/youtube-info-status-candidate.txt"
grep --fixed-strings \
  'identity=name,ordinal,field,lookup;copy=true;lookup-errors=iae,npe' \
  "$WORK/youtube-info-status-candidate.txt" >/dev/null
# C_SEMANTIC retains the tracker shell and cached values while disabling legacy credential traffic.
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeAccessTokenTracker reference \
  >"$WORK/youtube-access-token-tracker-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeAccessTokenTracker candidate \
  >"$WORK/youtube-access-token-tracker-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,23-fields,1-constructor,8-exported-methods,21-declared-methods,dependency-credential-capture,defaults,context-marker,cached-identity,private-shell;service=legacy-email-password-android-tv-visitor-http' \
  "$WORK/youtube-access-token-tracker-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,23-fields,1-constructor,8-exported-methods,21-declared-methods,dependency-credential-capture,defaults,context-marker,cached-identity,private-shell;service=deterministic-no-network,native-auth-owner,cached-only' \
  "$WORK/youtube-access-token-tracker-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeCachedAuthScript \
  >"$WORK/youtube-cached-auth-script-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeCachedAuthScript \
  >"$WORK/youtube-cached-auth-script-candidate.txt"
cmp "$WORK/youtube-cached-auth-script-reference.txt" \
  "$WORK/youtube-cached-auth-script-candidate.txt"
grep --fixed-strings \
  'shape=protected-static-member,object-root,2-public-final-fields,1-public-constructor,0-methods;capture=client-id,client-secret,null,reference;identity=object' \
  "$WORK/youtube-cached-auth-script-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeAudioSourceManager reference \
  >"$WORK/youtube-audio-source-manager-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeAudioSourceManager candidate "$native" \
  >"$WORK/youtube-audio-source-manager-candidate.txt"
grep --fixed-strings \
  'common=public-deprecated,audio-source-http-configurable,13-fields,3-constructors,20-methods,1-nested;construction,collaborator-identity,source-name,empty-details,track-reconstruction,playlist-pages,http-config,access-tracker,shutdown,reflection;service=legacy-link-router,retry,credential-bootstrap,track-details-http' \
  "$WORK/youtube-audio-source-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-deprecated,audio-source-http-configurable,13-fields,3-constructors,20-methods,1-nested;construction,collaborator-identity,source-name,empty-details,track-reconstruction,playlist-pages,http-config,access-tracker,shutdown,reflection;service=current-native-bounded-routing,no-legacy-credential-bootstrap' \
  "$WORK/youtube-audio-source-manager-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" GateYoutubeAudioTrack reference \
  >"$WORK/youtube-audio-track-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeAudioTrack candidate "$native" \
  >"$WORK/youtube-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,10-methods,1-nested;construction,track-info,source-identity,seekable,fresh-shallow-clone,reflection;service=legacy-details-format-signature-http,webm-or-mpeg-delegate' \
  "$WORK/youtube-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,delegated-super,2-fields,1-constructor,10-methods,1-nested;construction,track-info,source-identity,seekable,fresh-shallow-clone,reflection;service=current-native-bounded-discovery,finite-or-live-playback,no-legacy-java-decoder' \
  "$WORK/youtube-audio-track-candidate.txt" >/dev/null
java -Xverify:all \
  -cp "$classes_argument$classpath_separator$reference_argument" \
  GateYoutubeCipherOperation >"$WORK/youtube-cipher-operation-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH" \
  GateYoutubeCipherOperation >"$WORK/youtube-cipher-operation-candidate.txt"
cmp "$WORK/youtube-cipher-operation-reference.txt" \
  "$WORK/youtube-cipher-operation-candidate.txt"
grep --fixed-strings \
  'operation=public-concrete,object-root,2-public-final-fields,1-public-constructor,0-methods;capture=type-reference-null,full-int;identity=object;type=public-final-enum,4-public-constants,2-public-static-methods,1-private-constructor;order=SWAP,REVERSE,SLICE,SPLICE;identity=name,ordinal,field,lookup;copy=true;lookup-errors=iae,npe' \
  "$WORK/youtube-cipher-operation-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeClientConfig >"$WORK/youtube-client-config-reference.txt"
java -Xverify:all \
  -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeClientConfig >"$WORK/youtube-client-config-candidate.txt"
cmp "$WORK/youtube-client-config-reference.txt" "$WORK/youtube-client-config-candidate.txt"
grep --fixed-strings \
  'config=public-json-object,9-fields,2-constructors,15-methods,1-nested;fresh-null-empty,fluent-root-client-user-screen-embed-playback,deep-copy,name-user-agent-identity,api-key-reset-on-copy,http-context-attribute,four-mutable-presets;android-version=ANDROID_11,os-11,sdk-30,enum-copy-lookup' \
  "$WORK/youtube-client-config-candidate.txt" >/dev/null
java -Xverify:all -cp "$classes_argument$classpath_separator$reference_argument" \
  GateYoutubeConstants >"$WORK/youtube-constants-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH" \
  GateYoutubeConstants >"$WORK/youtube-constants-candidate.txt"
cmp "$WORK/youtube-constants-reference.txt" "$WORK/youtube-constants-candidate.txt"
grep --fixed-strings \
  'public-object-shell,46-package-constants,1-public-constructor,0-methods;fresh-construction,object-identity,youtube-api,music-api,tv-auth,legacy-auth,payload-composition,constant-identity,reflection' \
  "$WORK/youtube-constants-candidate.txt" >/dev/null
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeFormatInfo >"$WORK/youtube-format-info-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeFormatInfo >"$WORK/youtube-format-info-candidate.txt"
cmp "$WORK/youtube-format-info-reference.txt" "$WORK/youtube-format-info-candidate.txt"
grep --fixed-strings \
  'public-final-enum,5-constants,2-public-fields,3-public-methods;mime-codec-pairs,values-copy,lookup-identity,exact-before-substring,unknown-null,lookup-errors,reflection' \
  "$WORK/youtube-format-info-candidate.txt" >/dev/null
# Preserve the HTTP filter shell and anonymous mechanics while fencing legacy token forwarding.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeHttpContextFilter reference \
  >"$WORK/youtube-http-context-filter-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeHttpContextFilter candidate \
  >"$WORK/youtube-http-context-filter-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,5-fields,1-constructor,6-callbacks,constant,tracker-setter,cookie-reset,context-close,raw-bypass,api-key,retry-counter,429-block,connection-reset,reflection;service=legacy-authorization,visitor-forwarding,401-refresh' \
  "$WORK/youtube-http-context-filter-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,5-fields,1-constructor,6-callbacks,constant,tracker-setter,cookie-reset,context-close,raw-bypass,api-key,retry-counter,429-block,connection-reset,reflection;service=bounded-native-auth,no-authorization,no-visitor-forwarding,no-401-refresh' \
  "$WORK/youtube-http-context-filter-candidate.txt" >/dev/null
# A_EXACT preserves the generic router and nested callback SPI without implementation policy.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeLinkRouter >"$WORK/youtube-link-router-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeLinkRouter >"$WORK/youtube-link-router-candidate.txt"
cmp "$WORK/youtube-link-router-reference.txt" "$WORK/youtube-link-router-candidate.txt"
grep --fixed-strings \
  'router=public-abstract-generic-interface,0-fields,0-constructors,1-method,1-public-static-nested;route=method-T,string,routes-T,erased-object;routes=public-static-generic-interface,0-fields,0-constructors,7-methods;callbacks=track,playlist,mix,search,searchMusic,anonymous,none;dispatch=ordered,identity,nulls,unchecked;reflection=exact' \
  "$WORK/youtube-link-router-candidate.txt" >/dev/null
# A_EXACT preserves the generic mix-loader SPI and leaves track creation under caller control.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMixLoader >"$WORK/youtube-mix-loader-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMixLoader >"$WORK/youtube-mix-loader-candidate.txt"
cmp "$WORK/youtube-mix-loader-reference.txt" "$WORK/youtube-mix-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,1-method;load=http,mix-id,selected-id,generic-track-factory,audio-playlist;identity=arguments,return,nulls,unchecked;factory=lazy;reflection=exact' \
  "$WORK/youtube-mix-loader-candidate.txt" >/dev/null
# Preserve the provider shell while fencing its legacy single-client network implementation.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMixProvider reference >"$WORK/youtube-mix-provider-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMixProvider candidate >"$WORK/youtube-mix-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,mix-loader-interface,0-fields,1-constructor,2-exported-methods,2-private-methods;fresh-construction,generic-factory,private-signatures,reflection;service=legacy-single-android-next,unbounded-tracks' \
  "$WORK/youtube-mix-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,mix-loader-interface,0-fields,1-constructor,2-exported-methods,2-private-methods;fresh-construction,generic-factory,private-signatures,reflection;service=deterministic-no-network,current-bounded-native-source' \
  "$WORK/youtube-mix-provider-candidate.txt" >/dev/null
# Preserve the segmented-MPEG shell while routing current playback through bounded native media.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMpegStreamAudioTrack reference \
  >"$WORK/youtube-mpeg-stream-audio-track-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeMpegStreamAudioTrack candidate \
  >"$WORK/youtube-mpeg-stream-audio-track-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,mpeg-audio-track-super,0-exported-fields,1-constructor,5-exported-methods,7-private-fields,9-private-methods,2-private-static-nested;retry-400-50,rewind-43200,signatures,reflection;service=legacy-persistent-mpeg-segments,wall-clock-retry,43200-second-rewind' \
  "$WORK/youtube-mpeg-stream-audio-track-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,mpeg-audio-track-super,0-exported-fields,1-constructor,5-exported-methods,7-private-fields,9-private-methods,2-private-static-nested;retry-400-50,rewind-43200,signatures,reflection;service=deterministic-no-http,bounded-native-hls-and-finite-media' \
  "$WORK/youtube-mpeg-stream-audio-track-candidate.txt" >/dev/null
# Preserve the local JSON join helper exactly, including identity and collision behavior.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePayloadHelper >"$WORK/youtube-payload-helper-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePayloadHelper >"$WORK/youtube-payload-helper-candidate.txt"
cmp "$WORK/youtube-payload-helper-reference.txt" \
  "$WORK/youtube-payload-helper-candidate.txt"
grep --fixed-strings \
  'public-concrete,object-root,0-fields,1-public-constructor,1-public-static-method;absent-inserts-fresh-child,existing-child-identity,no-overwrite,empty-key,mutation-visible;errors=null-root-npe,null-key-json,non-object-json,json-null-json;reflection' \
  "$WORK/youtube-payload-helper-candidate.txt" >/dev/null
# Preserve the stream shell and capability flags while fencing legacy query-range HTTP.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePersistentHttpStream reference \
  >"$WORK/youtube-persistent-http-stream-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePersistentHttpStream candidate \
  >"$WORK/youtube-persistent-http-stream-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,persistent-http-stream-super,3-private-fields,1-constructor,5-exported-methods,2-private-methods;buffer-11862014,constructor-capture,no-http-on-construction,headers-false,hard-seek-true,signatures,reflection;service=legacy-query-range-url,network-backed-read-skip' \
  "$WORK/youtube-persistent-http-stream-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,persistent-http-stream-super,3-private-fields,1-constructor,5-exported-methods,2-private-methods;buffer-11862014,constructor-capture,no-http-on-construction,headers-false,hard-seek-true,signatures,reflection;service=deterministic-no-http,bounded-native-http-and-media' \
  "$WORK/youtube-persistent-http-stream-candidate.txt" >/dev/null
# A_EXACT preserves the generic playlist-loader SPI without implementation policy.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePlaylistLoader >"$WORK/youtube-playlist-loader-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubePlaylistLoader >"$WORK/youtube-playlist-loader-candidate.txt"
cmp "$WORK/youtube-playlist-loader-reference.txt" \
  "$WORK/youtube-playlist-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,2-methods;set-page-count=int;load=http,playlist-id,selected-id,generic-track-factory,audio-playlist;identity=arguments,return,nulls,unchecked;factory=lazy;reflection=exact' \
  "$WORK/youtube-playlist-loader-candidate.txt" >/dev/null
# Preserve the provider shell while fencing the frozen YouTube Music endpoint.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchMusicProvider reference \
  >"$WORK/youtube-search-music-provider-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchMusicProvider candidate \
  >"$WORK/youtube-search-music-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,search-music-result-loader,2-private-fields,1-constructor,2-exported-methods,4-private-methods;fresh-cookieless-manager,configuration-identity,generic-track-factory,logger,signatures,reflection;service=legacy-music-endpoint,manager-access-before-factory' \
  "$WORK/youtube-search-music-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,search-music-result-loader,2-private-fields,1-constructor,2-exported-methods,4-private-methods;fresh-cookieless-manager,configuration-identity,generic-track-factory,logger,signatures,reflection;service=deterministic-no-manager-access,bounded-native-current-client-search' \
  "$WORK/youtube-search-music-provider-candidate.txt" >/dev/null
# A_EXACT preserves the generic music-search loader SPI without implementation policy.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchMusicResultLoader \
  >"$WORK/youtube-search-music-result-loader-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchMusicResultLoader \
  >"$WORK/youtube-search-music-result-loader-candidate.txt"
cmp "$WORK/youtube-search-music-result-loader-reference.txt" \
  "$WORK/youtube-search-music-result-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,2-methods;load=query,generic-track-factory,audio-item;configuration=extended-http-configurable;identity=arguments,returns,nulls,unchecked;factory=lazy;reflection=exact' \
  "$WORK/youtube-search-music-result-loader-candidate.txt" >/dev/null
# Preserve the provider shell while fencing the frozen Android search endpoint.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchProvider reference >"$WORK/youtube-search-provider-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchProvider candidate >"$WORK/youtube-search-provider-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,search-result-loader,2-private-fields,1-constructor,2-exported-methods,5-private-methods;fresh-cookieless-manager,configuration-identity,generic-track-factory,logger,signatures,reflection;service=legacy-android-search-endpoint,manager-access-before-factory' \
  "$WORK/youtube-search-provider-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,search-result-loader,2-private-fields,1-constructor,2-exported-methods,5-private-methods;fresh-cookieless-manager,configuration-identity,generic-track-factory,logger,signatures,reflection;service=deterministic-no-manager-access,bounded-native-current-client-search' \
  "$WORK/youtube-search-provider-candidate.txt" >/dev/null
# A_EXACT preserves the generic ordinary-search loader SPI without implementation policy.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchResultLoader >"$WORK/youtube-search-result-loader-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSearchResultLoader >"$WORK/youtube-search-result-loader-candidate.txt"
cmp "$WORK/youtube-search-result-loader-reference.txt" \
  "$WORK/youtube-search-result-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,2-methods;load=query,generic-track-factory,audio-item;configuration=extended-http-configurable;identity=arguments,returns,nulls,unchecked;factory=lazy;reflection=exact' \
  "$WORK/youtube-search-result-loader-candidate.txt" >/dev/null
# Preserve the local cipher pipeline while fencing arbitrary legacy JavaScript execution.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureCipher reference >"$WORK/youtube-signature-cipher-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureCipher candidate >"$WORK/youtube-signature-cipher-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,4-fields,1-constructor,7-methods;fresh-array-list,empty-script-state,setter-identity,ordered-operations,swap-reverse-slice-splice,edge-errors,signatures,reflection;transform=legacy-script-engine,eval-before-invoke' \
  "$WORK/youtube-signature-cipher-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,4-fields,1-constructor,7-methods;fresh-array-list,empty-script-state,setter-identity,ordered-operations,swap-reverse-slice-splice,edge-errors,signatures,reflection;transform=deterministic-no-script-engine,bounded-native-signature-and-n' \
  "$WORK/youtube-signature-cipher-candidate.txt" >/dev/null
# Preserve the manager shell while fencing Rhino, frozen script fetching, and regex extraction.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureCipherManager reference \
  >"$WORK/youtube-signature-cipher-manager-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureCipherManager candidate \
  >"$WORK/youtube-signature-cipher-manager-candidate.txt"
grep --fixed-strings \
  'common=public-concrete,object-root,signature-resolver,24-private-fields,1-constructor,3-exported-methods,6-private-methods;fresh-concurrent-cache,fresh-dump-set,fresh-lock,static-constants,compiled-patterns,logger,signatures,reflection;service=legacy-rhino,pass-through-unsigned-dash' \
  "$WORK/youtube-signature-cipher-manager-reference.txt" >/dev/null
grep --fixed-strings \
  'common=public-concrete,object-root,signature-resolver,24-private-fields,1-constructor,3-exported-methods,6-private-methods;fresh-concurrent-cache,fresh-dump-set,fresh-lock,static-constants,compiled-patterns,logger,signatures,reflection;service=deterministic-no-engine,no-http-or-script-access,bounded-native-signature-and-n' \
  "$WORK/youtube-signature-cipher-manager-candidate.txt" >/dev/null
# A_EXACT preserves the signature resolver SPI without implementation policy.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureResolver >"$WORK/youtube-signature-resolver-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeSignatureResolver >"$WORK/youtube-signature-resolver-candidate.txt"
cmp "$WORK/youtube-signature-resolver-reference.txt" \
  "$WORK/youtube-signature-resolver-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,3-methods;script=io-exception,cipher;format=exception,uri;dash=exception,string;identity=arguments,returns,nulls,checked;reflection=exact' \
  "$WORK/youtube-signature-resolver-candidate.txt" >/dev/null
# A_EXACT preserves the track-details SPI and its generic format-list contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackDetails >"$WORK/youtube-track-details-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackDetails >"$WORK/youtube-track-details-candidate.txt"
cmp "$WORK/youtube-track-details-reference.txt" "$WORK/youtube-track-details-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,3-methods;info=audio-track-info;formats=list-of-youtube-track-format;script=string;identity=arguments,returns,nulls,unchecked;reflection=exact' \
  "$WORK/youtube-track-details-candidate.txt" >/dev/null
# A_EXACT preserves the track-details-loader SPI and its primitive flag contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackDetailsLoader >"$WORK/youtube-track-details-loader-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackDetailsLoader >"$WORK/youtube-track-details-loader-candidate.txt"
cmp "$WORK/youtube-track-details-loader-reference.txt" \
  "$WORK/youtube-track-details-loader-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,0-fields,0-constructors,1-method;details=http,id,boolean,source-manager-to-track-details;identity=arguments,primitive,return,nulls,unchecked;reflection=exact' \
  "$WORK/youtube-track-details-loader-candidate.txt" >/dev/null
# A_EXACT preserves the immutable YouTube track-format value contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackFormat >"$WORK/youtube-track-format-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackFormat >"$WORK/youtube-track-format-candidate.txt"
cmp "$WORK/youtube-track-format-reference.txt" "$WORK/youtube-track-format-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,10-private-final-fields,1-constructor,10-getters;format-info=constructor-derived;capture=type,longs,strings,boolean;identity=type,n,signature,key;url=fresh-uri,syntax-wrapper,null-error;nullable=info,n,signature,key;reflection=exact' \
  "$WORK/youtube-track-format-candidate.txt" >/dev/null
# A_EXACT preserves the immutable YouTube track-JSON parsing contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackJsonData >"$WORK/youtube-track-json-data-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackJsonData >"$WORK/youtube-track-json-data-candidate.txt"
cmp "$WORK/youtube-track-json-data-reference.txt" \
  "$WORK/youtube-track-json-data-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,1-private-static-final-log,3-public-final-fields,1-constructor,2-public-methods,2-private-methods;capture=identity,nulls;with-script=fresh,retains-browsers;main-result=direct,nested,polymer,embedded,fallback,first-non-null;errors=wrapped,redacted,cause-chain;reflection=exact' \
  "$WORK/youtube-track-json-data-candidate.txt" >/dev/null
# A_EXACT preserves the legacy adaptive-format extraction contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyAdaptiveFormatsExtractor >"$WORK/legacy-adaptive-formats-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyAdaptiveFormatsExtractor >"$WORK/legacy-adaptive-formats-candidate.txt"
cmp "$WORK/legacy-adaptive-formats-reference.txt" \
  "$WORK/legacy-adaptive-formats-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,offline-extractor,0-fields,1-constructor,1-public-method,1-private-helper;absent=shared-empty;adaptive=ordered-array-list,url-decoded;format=type,longs,fixed-channels,url,empty-n,signature,key,default-audio;defaults=signature-key;errors=unchecked;reflection=exact' \
  "$WORK/legacy-adaptive-formats-candidate.txt" >/dev/null
# A_EXACT preserves legacy DASH resolution, HTTP/XML parsing, and response ownership.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyDashMpdFormatsExtractor >"$WORK/legacy-dash-mpd-formats-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyDashMpdFormatsExtractor >"$WORK/legacy-dash-mpd-formats-candidate.txt"
cmp "$WORK/legacy-dash-mpd-formats-reference.txt" \
  "$WORK/legacy-dash-mpd-formats-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,youtube-format-extractor,1-private-static-final-log,1-constructor,1-public-method,2-private-helpers;absent=shared-empty,no-io;resolution=argument-identity,http-get;response=status,utf8-xml,closed,suppressed;document=ordered,skip-missing-clen;format=type,longs,fixed-channels,url,empty-n,null-signature,default-key,default-audio;errors=url-context,cause-identity;reflection=exact' \
  "$WORK/legacy-dash-mpd-formats-candidate.txt" >/dev/null
# A_EXACT preserves the fault-isolating legacy stream-map extraction contract.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyStreamMapFormatsExtractor >"$WORK/legacy-stream-map-formats-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateLegacyStreamMapFormatsExtractor >"$WORK/legacy-stream-map-formats-candidate.txt"
cmp "$WORK/legacy-stream-map-formats-reference.txt" \
  "$WORK/legacy-stream-map-formats-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,offline-extractor,1-private-static-final-log,1-constructor,1-public-method,2-private-helpers;absent=shared-empty;stream-map=ordered,array-list,url-decoded,per-entry-isolation,skip-missing-fields;quality=small,medium,hd720,default-negative;format=type,length,fixed-channels,url,empty-n,signature,key,default-audio;errors=swallowed;reflection=exact' \
  "$WORK/legacy-stream-map-formats-candidate.txt" >/dev/null
# A_EXACT preserves the offline extractor SPI and its default delegation bridge.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOfflineYoutubeTrackFormatExtractor >"$WORK/offline-youtube-track-format-extractor-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateOfflineYoutubeTrackFormatExtractor >"$WORK/offline-youtube-track-format-extractor-candidate.txt"
cmp "$WORK/offline-youtube-track-format-extractor-reference.txt" \
  "$WORK/offline-youtube-track-format-extractor-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,youtube-format-extractor,0-fields,0-constructors,2-methods;offline=abstract-data-to-list;default=delegates-data,ignores-http-resolver;identity=argument,return,nulls,unchecked;generics=list-of-youtube-track-format;reflection=exact' \
  "$WORK/offline-youtube-track-format-extractor-candidate.txt" >/dev/null
# A_EXACT preserves modern streaming-data extraction and its live-format rules.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateStreamingDataFormatsExtractor >"$WORK/streaming-data-formats-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateStreamingDataFormatsExtractor >"$WORK/streaming-data-formats-candidate.txt"
cmp "$WORK/streaming-data-formats-reference.txt" \
  "$WORK/streaming-data-formats-candidate.txt"
grep --fixed-strings \
  'public-concrete-object,offline-extractor,1-private-static-final-log,1-constructor,1-public-method,1-private-helper;absence=shared-empty;present=fresh-mutable,formats-before-adaptive;sources=direct-url,cipher,signature-cipher;format=type,bitrate,length,channels,url,n,signature,key,default-audio;live=video-details,ended-reason,missing-length;errors=skip-nonlive-missing-length,per-entry-isolation,outer-unchecked;reflection=exact' \
  "$WORK/streaming-data-formats-candidate.txt" >/dev/null
# A_EXACT preserves the generic YouTube format extractor SPI and signature constant.
java -Xverify:all -cp "$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackFormatExtractor >"$WORK/youtube-track-format-extractor-reference.txt"
java -Xverify:all -cp "$GATE_CLASSPATH$classpath_separator$REFERENCE_PROVIDER_TOOLS_CLASSPATH" \
  GateYoutubeTrackFormatExtractor >"$WORK/youtube-track-format-extractor-candidate.txt"
cmp "$WORK/youtube-track-format-extractor-reference.txt" \
  "$WORK/youtube-track-format-extractor-candidate.txt"
grep --fixed-strings \
  'public-abstract-interface,object-super,0-parents,1-field,1-method;constant=public-static-final,string,signature;extract=abstract,data-http-resolver-to-generic-list;identity=arguments,return,nulls,unchecked;reflection=exact' \
  "$WORK/youtube-track-format-extractor-candidate.txt" >/dev/null
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
