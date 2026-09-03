//! Bounded, backend-independent media probing, decoding, packet extraction, and seeking.

use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration as StdDuration;

use mantle_audio::{AudioFrameError, PcmFormat, opus_packet_duration};
use mantle_xaac::{XaacConfig, XaacDecodeStatus, XaacDecoder, XaacProfile};
use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_FLAC, CODEC_ID_MP3, CODEC_ID_OPUS, CODEC_ID_PCM_S16LE,
    CODEC_ID_PCM_S24LE, CODEC_ID_PCM_S32LE, CODEC_ID_VORBIS,
};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::common::Limit;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{
    MediaSource as SymphoniaMediaSource, MediaSourceStream, MediaSourceStreamOptions,
};
use symphonia::core::meta::{MetadataContainer, MetadataOptions, StandardTag, Tag};
use symphonia::core::units::{Duration as SymphoniaDuration, Time, TimeBase, Timestamp};

const MAX_XAAC_DELAYED_TIMESTAMPS: usize = 16;
const MAX_AAC_PROFILE_PROBE_PACKETS: usize = 64;
const MAX_ADTS_SCAN_DISTANCE: usize = 1_000;
const MAX_EBML_INSPECTION_BYTES: usize = 128 * 1024;

mod bandcamp;
mod beam;
mod getyarn;
mod hls;
mod http_input;
mod mpeg_ts;
mod niconico;
mod playlist;
mod remote_http;
mod soundcloud;
mod source;
mod twitch;
mod vimeo;
mod yandex_music;
mod youtube;
mod youtube_cipher_process;
mod youtube_playback;

pub use bandcamp::{
    BandcampError, BandcampErrorKind, BandcampPlaybackError, BandcampPlaybackErrorKind,
    BandcampPlaybackScheme, BandcampPlaybackSession, BandcampPlaybackUrl, BandcampRoute,
    BandcampSourceItem, BandcampSourceManager, BandcampSourceOptions, BandcampSourcePlaylist,
    BandcampSourceTrack, route_bandcamp_identifier,
};
pub use beam::{
    BeamError, BeamErrorKind, BeamRoute, BeamSourceManager, BeamSourceOptions, BeamSourceTrack,
    route_beam_identifier,
};
pub use getyarn::{
    GetyarnError, GetyarnErrorKind, GetyarnRoute, GetyarnSourceManager, GetyarnSourceOptions,
    GetyarnSourceTrack, route_getyarn_identifier,
};
pub use hls::{
    HlsError, HlsLimits, HlsLiveLimits, HlsLivePoll, HlsLiveSequence, HlsMasterPlaylist,
    HlsMediaPlaylist, HlsPlaylist, HlsSegment, HlsVariant, HlsVodAdtsInput, HlsVodSequence,
    load_http_hls_playlist, load_http_hls_playlist_with_cancellation, load_http_hls_segment,
    load_http_hls_segment_with_cancellation, parse_hls_playlist,
};
pub use http_input::{
    HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, HttpStreamInput, HttpStreamOptions,
    OutboundRoute, OutboundRouteContext, OutboundRouteOutcome, OutboundRoutePolicy,
};
pub use mantle_audio::PcmFrame;
pub use mpeg_ts::{
    MpegTsAdtsSegment, MpegTsError, MpegTsLimits, MpegTsMetadata, extract_mpeg_ts_adts,
};
pub use niconico::{
    NicoNicoAuthentication, NicoNicoError, NicoNicoErrorKind, NicoNicoPlaybackError,
    NicoNicoPlaybackErrorKind, NicoNicoPlaybackScheme, NicoNicoPlaybackSession,
    NicoNicoPlaybackUrl, NicoNicoRoute, NicoNicoSourceManager, NicoNicoSourceOptions,
    NicoNicoSourceTrack, route_niconico_identifier,
};
pub use playlist::{
    HttpPlaylistOptions, PlaylistError, PlaylistFormat, PlaylistLimits, PlaylistLoadError,
    PlaylistMatch, PlaylistReference, load_http_playlist, load_http_playlist_with_cancellation,
    probe_playlist, resolve_http_reference,
};
pub use remote_http::{
    RemoteHttpClient, RemoteHttpError, RemoteHttpErrorKind, RemoteHttpOptions, RemoteHttpRequest,
    RemoteHttpResponse, RemoteRetryMode,
};
pub use soundcloud::{
    SoundCloudAccess, SoundCloudAuthentication, SoundCloudError, SoundCloudErrorKind,
    SoundCloudPlaybackError, SoundCloudPlaybackErrorKind, SoundCloudPlaybackSession,
    SoundCloudPlaybackUrl, SoundCloudRoute, SoundCloudSourceItem, SoundCloudSourceManager,
    SoundCloudSourceOptions, SoundCloudSourcePlaylist, SoundCloudSourceTrack,
    route_soundcloud_identifier,
};
pub use source::{
    HttpMediaSourceManager, HttpMediaSourceOptions, LocalMediaSourceManager, MediaProbe,
    MediaSourceTrack,
};
pub use twitch::{
    TwitchAuthentication, TwitchError, TwitchErrorKind, TwitchLivePlaybackOptions,
    TwitchLivePlaybackPoll, TwitchLivePlaybackSession, TwitchPlaybackError,
    TwitchPlaybackErrorKind, TwitchPlaybackScheme, TwitchPlaybackUrl, TwitchRoute,
    TwitchSourceManager, TwitchSourceOptions, TwitchSourceTrack, route_twitch_identifier,
};
pub use vimeo::{
    VimeoAuthentication, VimeoError, VimeoErrorKind, VimeoPlaybackError, VimeoPlaybackErrorKind,
    VimeoPlaybackKind, VimeoPlaybackScheme, VimeoPlaybackSession, VimeoPlaybackUrl, VimeoRoute,
    VimeoSourceManager, VimeoSourceOptions, VimeoSourceTrack, route_vimeo_identifier,
};
pub use yandex_music::{
    YandexMusicAuthentication, YandexMusicError, YandexMusicErrorKind, YandexMusicPlaybackError,
    YandexMusicPlaybackErrorKind, YandexMusicPlaybackScheme, YandexMusicPlaybackSession,
    YandexMusicPlaybackUrl, YandexMusicPlaylistKind, YandexMusicRoute, YandexMusicSourceItem,
    YandexMusicSourceManager, YandexMusicSourceOptions, YandexMusicSourcePlaylist,
    YandexMusicSourceTrack, route_yandex_music_identifier,
};
pub use youtube::{
    YoutubeAudioSourceManager, YoutubeAuthentication, YoutubeCipherChallenge,
    YoutubeCipherResolver, YoutubeCipherResolverError, YoutubeCipherResolverErrorKind,
    YoutubeCipherSolution, YoutubeClientKind, YoutubeError, YoutubeErrorKind, YoutubeOAuthClock,
    YoutubeOAuthDeviceCode, YoutubeOAuthOptions, YoutubeOAuthTokenStatus, YoutubePlaybackFormat,
    YoutubePlaybackFormatKind, YoutubePlaybackFormats, YoutubePlayerScript,
    YoutubeResolvedPlaybackUrl, YoutubeRoute, YoutubeSourceItem, YoutubeSourceOptions,
    YoutubeSourcePlaylist, YoutubeSourceTrack, route_youtube_identifier,
};
pub use youtube_cipher_process::{YoutubeProcessCipherOptions, YoutubeProcessCipherResolver};
pub use youtube_playback::{
    YoutubeLivePlaybackOptions, YoutubeLivePlaybackPoll, YoutubeLivePlaybackSession,
    YoutubePlaybackError, YoutubePlaybackErrorKind, YoutubePlaybackMode, YoutubePlaybackSession,
};

/// Bounds applied before and around the media backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaLimits {
    pub max_probe_bytes: u64,
    pub input_buffer_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_codec_probe_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_pcm_samples_per_frame: usize,
    pub max_native_decoder_bytes: usize,
    pub max_codec_config_bytes: usize,
    pub max_consecutive_decode_errors: u32,
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_probe_bytes: 1024 * 1024,
            input_buffer_bytes: 64 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_codec_probe_bytes: 1024 * 1024,
            max_packet_bytes: 1024 * 1024,
            max_pcm_samples_per_frame: 256 * 1024,
            max_native_decoder_bytes: 32 * 1024 * 1024,
            max_codec_config_bytes: 4 * 1024,
            max_consecutive_decode_errors: 8,
        }
    }
}

impl MediaLimits {
    fn validate(self) -> Result<Self, MediaError> {
        if self.max_probe_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_probe_bytes must be non-zero",
            ));
        }
        if self.input_buffer_bytes <= 32 * 1024 || !self.input_buffer_bytes.is_power_of_two() {
            return Err(MediaError::InvalidLimits(
                "input_buffer_bytes must be a power of two greater than 32 KiB",
            ));
        }
        if self.max_metadata_string_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_metadata_string_bytes must be non-zero",
            ));
        }
        if self.max_codec_probe_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_codec_probe_bytes must be non-zero",
            ));
        }
        if self.max_packet_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_packet_bytes must be non-zero",
            ));
        }
        if self.max_pcm_samples_per_frame == 0 {
            return Err(MediaError::InvalidLimits(
                "max_pcm_samples_per_frame must be non-zero",
            ));
        }
        if self.max_native_decoder_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_native_decoder_bytes must be non-zero",
            ));
        }
        if self.max_codec_config_bytes == 0 {
            return Err(MediaError::InvalidLimits(
                "max_codec_config_bytes must be non-zero",
            ));
        }
        Ok(self)
    }
}

/// Mantle's backend-independent input boundary.
pub trait MediaInput: Read + Seek + Send + Sync {
    fn is_seekable(&self) -> bool;
    fn byte_len(&self) -> Option<u64>;
}

impl MediaInput for File {
    fn is_seekable(&self) -> bool {
        self.metadata().is_ok_and(|metadata| metadata.is_file())
    }

    fn byte_len(&self) -> Option<u64> {
        self.metadata().ok().map(|metadata| metadata.len())
    }
}

/// An owned in-memory input for deterministic tests and callers with existing bytes.
pub struct MemoryInput {
    inner: Cursor<Box<[u8]>>,
}

impl MemoryInput {
    #[must_use]
    pub fn new(bytes: impl Into<Box<[u8]>>) -> Self {
        Self {
            inner: Cursor::new(bytes.into()),
        }
    }
}

impl Read for MemoryInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buffer)
    }
}

impl Seek for MemoryInput {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

impl MediaInput for MemoryInput {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.inner.get_ref().len() as u64)
    }
}

struct PrefixInput {
    prefix: Cursor<Box<[u8]>>,
    inner: Box<dyn MediaInput>,
    byte_len: Option<u64>,
}

impl Read for PrefixInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let prefix_count = self.prefix.read(buffer)?;
        if prefix_count != 0 || buffer.is_empty() {
            return Ok(prefix_count);
        }
        self.inner.read(buffer)
    }
}

impl Seek for PrefixInput {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "prefixed media input is not seekable",
        ))
    }
}

impl MediaInput for PrefixInput {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        self.byte_len
    }
}

struct OffsetInput {
    inner: Box<dyn MediaInput>,
    physical_offset: u64,
    logical_position: u64,
}

impl Read for OffsetInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.logical_position = self.logical_position.saturating_add(count as u64);
        Ok(count)
    }
}

impl Seek for OffsetInput {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if !self.inner.is_seekable() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "offset media input is not seekable",
            ));
        }
        let logical_length = self.byte_len().ok_or_else(|| {
            io::Error::new(io::ErrorKind::Unsupported, "media input length is unknown")
        })?;
        let target = match position {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(delta) => self.logical_position.checked_add_signed(delta),
            SeekFrom::End(delta) => logical_length.checked_add_signed(delta),
        }
        .filter(|target| *target <= logical_length)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid media seek"))?;
        let physical = self
            .physical_offset
            .checked_add(target)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "media seek overflow"))?;
        self.inner.seek(SeekFrom::Start(physical))?;
        self.logical_position = target;
        Ok(target)
    }
}

impl MediaInput for OffsetInput {
    fn is_seekable(&self) -> bool {
        self.inner.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.inner
            .byte_len()
            .and_then(|length| length.checked_sub(self.physical_offset))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Wave,
    Mp3,
    Mp4,
    WebM,
    Matroska,
    Flac,
    Ogg,
    Adts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    PcmS16Le,
    PcmS24Le,
    PcmS32Le,
    Mp3,
    AacLc,
    HeAacV1,
    HeAacV2,
    Opus,
    Flac,
    Vorbis,
}

/// Bounded metadata fields that participate in Lavaplayer's local-track contract.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub isrc: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaInfo {
    pub container: Container,
    pub codec: Codec,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: Option<StdDuration>,
    pub seekable: bool,
    pub metadata: MediaMetadata,
}

/// A clonable, one-way cancellation signal for media probing and playback.
#[derive(Clone, Default)]
pub struct MediaCancellation {
    cancelled: Arc<AtomicBool>,
    linked: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl fmt::Debug for MediaCancellation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MediaCancellation")
            .field("cancelled", &self.is_cancelled())
            .field("linked", &self.linked.is_some())
            .finish()
    }
}

impl MediaCancellation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a token linked to an external cooperative cancellation signal.
    #[must_use]
    pub fn linked(check: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            linked: Some(Arc::new(check)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire) || self.linked.as_ref().is_some_and(|check| check())
    }

    fn check(&self) -> Result<(), MediaError> {
        if self.is_cancelled() {
            Err(MediaError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn check_io(&self) -> io::Result<()> {
        if self.is_cancelled() {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "media operation cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

/// Caller-owned reusable storage for an encoded packet such as Opus.
#[derive(Debug)]
pub struct EncodedPacket {
    data: Vec<u8>,
    timestamp: Option<StdDuration>,
    duration: Option<StdDuration>,
}

impl EncodedPacket {
    #[must_use]
    pub fn with_capacity(byte_capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(byte_capacity),
            timestamp: None,
            duration: None,
        }
    }

    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    #[must_use]
    pub fn timestamp(&self) -> Option<StdDuration> {
        self.timestamp
    }

    #[must_use]
    pub fn duration(&self) -> Option<StdDuration> {
        self.duration
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeekResult {
    pub requested: StdDuration,
    pub actual: Option<StdDuration>,
}

#[derive(Debug)]
pub enum MediaError {
    Io(io::Error),
    Cancelled,
    InvalidLimits(&'static str),
    InvalidHttpOptions(&'static str),
    ProbeLimitExceeded {
        limit: u64,
    },
    NoAudioTrack,
    UnsupportedContainer(String),
    UnsupportedCodec(String),
    UnsupportedCodecProfile {
        codec: &'static str,
        profile: &'static str,
    },
    PacketTooLarge {
        actual: usize,
        limit: usize,
    },
    CodecConfigTooLarge {
        actual: usize,
        limit: usize,
    },
    CodecProbeLimitExceeded {
        actual: usize,
        limit: usize,
    },
    PcmFrameTooLarge {
        actual: usize,
        limit: usize,
    },
    OutputBufferTooSmall {
        required: usize,
        capacity: usize,
    },
    DecodeErrorLimitExceeded {
        limit: u32,
    },
    DecodeDelayLimitExceeded {
        limit: usize,
    },
    WrongOutputKind {
        codec: Codec,
    },
    Backend {
        operation: &'static str,
        message: String,
    },
}

impl fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "media I/O failed: {error}"),
            Self::Cancelled => formatter.write_str("media operation cancelled"),
            Self::InvalidLimits(message) => write!(formatter, "invalid media limits: {message}"),
            Self::InvalidHttpOptions(message) => {
                write!(formatter, "invalid HTTP range options: {message}")
            }
            Self::ProbeLimitExceeded { limit } => {
                write!(formatter, "media probe exceeded its {limit}-byte limit")
            }
            Self::NoAudioTrack => formatter.write_str("media contains no supported audio track"),
            Self::UnsupportedContainer(container) => {
                write!(formatter, "unsupported media container: {container}")
            }
            Self::UnsupportedCodec(codec) => write!(formatter, "unsupported audio codec: {codec}"),
            Self::UnsupportedCodecProfile { codec, profile } => {
                write!(formatter, "unsupported {codec} profile: {profile}")
            }
            Self::PacketTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "media packet has {actual} bytes; limit is {limit}"
                )
            }
            Self::CodecConfigTooLarge { actual, limit } => write!(
                formatter,
                "codec configuration has {actual} bytes; limit is {limit}"
            ),
            Self::CodecProbeLimitExceeded { actual, limit } => write!(
                formatter,
                "codec profile probe consumed {actual} bytes; limit is {limit}"
            ),
            Self::PcmFrameTooLarge { actual, limit } => {
                write!(
                    formatter,
                    "decoded frame has {actual} samples; limit is {limit}"
                )
            }
            Self::OutputBufferTooSmall { required, capacity } => write!(
                formatter,
                "reusable output capacity is {capacity}; {required} elements are required"
            ),
            Self::DecodeErrorLimitExceeded { limit } => {
                write!(
                    formatter,
                    "more than {limit} consecutive packets failed to decode"
                )
            }
            Self::DecodeDelayLimitExceeded { limit } => {
                write!(
                    formatter,
                    "decoder buffered more than {limit} packet timestamps without output"
                )
            }
            Self::WrongOutputKind { codec } => {
                write!(
                    formatter,
                    "the {codec:?} track does not produce this output kind"
                )
            }
            Self::Backend { operation, message } => {
                write!(
                    formatter,
                    "media backend failed during {operation}: {message}"
                )
            }
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for MediaError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// A probed audio track with a private, replaceable backend implementation.
pub struct MediaSession {
    format: Box<dyn FormatReader>,
    pending_packets: VecDeque<symphonia::core::packet::Packet>,
    decoder: Option<PcmDecoder>,
    info: Box<MediaInfo>,
    cancellation: MediaCancellation,
    track_id: u32,
    time_base: Option<TimeBase>,
    limits: MediaLimits,
    consecutive_decode_errors: u32,
}

impl MediaSession {
    /// Opens and probes a local file.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid limits, I/O or probe failures, unsupported media, and AAC
    /// profiles that the selected decoder cannot reproduce correctly.
    pub fn open_file(path: impl AsRef<Path>, limits: MediaLimits) -> Result<Self, MediaError> {
        Self::open_file_with_cancellation(path, limits, MediaCancellation::new())
    }

    /// Opens and probes a local file with a caller-owned cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Cancelled`] when cancellation is requested before or during probing,
    /// in addition to the errors from [`Self::open_file`].
    pub fn open_file_with_cancellation(
        path: impl AsRef<Path>,
        limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Self, MediaError> {
        cancellation.check()?;
        let path = path.as_ref();
        let file = File::open(path)?;
        let extension = path.extension().and_then(|value| value.to_str());
        Self::open_with_cancellation(Box::new(file), extension, limits, cancellation)
    }

    /// Opens and probes an arbitrary Mantle media input.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid limits, probe failures, unsupported media, and AAC profiles
    /// that the selected decoder cannot reproduce correctly.
    pub fn open(
        input: Box<dyn MediaInput>,
        extension_hint: Option<&str>,
        limits: MediaLimits,
    ) -> Result<Self, MediaError> {
        Self::open_with_cancellation(input, extension_hint, limits, MediaCancellation::new())
    }

    /// Opens and probes an arbitrary Mantle media input with a cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns [`MediaError::Cancelled`] when cancellation is requested before or during probing,
    /// in addition to the errors from [`Self::open`].
    pub fn open_with_cancellation(
        input: Box<dyn MediaInput>,
        extension_hint: Option<&str>,
        limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Self, MediaError> {
        let limits = limits.validate()?;
        cancellation.check()?;
        let ProbedMedia {
            mut format,
            seekable,
            ebml_metadata,
            adts_config,
            id3_metadata,
        } = probe_media_input(input, extension_hint, limits, &cancellation)?;

        let container = map_container(
            format.format_info().short_name,
            ebml_metadata.as_ref().map(|metadata| metadata.container),
        )?;
        let ebml_output_sample_rate = ebml_metadata
            .as_ref()
            .and_then(|metadata| metadata.output_sample_rate);
        let media_time_base = format.media_info().time_base;
        let media_duration = format
            .media_info()
            .duration
            .and_then(|duration| duration_to_std(media_time_base, duration))
            .filter(|duration| !duration.is_zero());
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(MediaError::NoAudioTrack)?;
        let track_id = track.id;
        let time_base = track.time_base.or(media_time_base);
        let mut duration = track
            .duration
            .and_then(|duration| duration_to_std(time_base, duration))
            .or(media_duration);
        let mut params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or(MediaError::NoAudioTrack)?
            .clone();
        if container == Container::Wave {
            validate_wave_track_params(&params)?;
        }
        if container == Container::Adts {
            apply_adts_track_config(&mut params, adts_config)?;
            duration = None;
        }
        let mut sample_rate = ebml_output_sample_rate
            .or(params.sample_rate)
            .ok_or_else(|| {
                MediaError::UnsupportedCodec(format!("{} without a sample rate", params.codec))
            })?;
        let mut codec = map_codec(&params, sample_rate)?;
        let mut pending_packets = VecDeque::new();
        let ambiguous_implicit_aac = codec == Codec::AacLc
            && ((container == Container::Matroska && ebml_output_sample_rate.is_none())
                || container == Container::Adts);
        if ambiguous_implicit_aac {
            let profile_probe = probe_implicit_aac_profile(
                format.as_mut(),
                track_id,
                &params,
                limits,
                &cancellation,
            )?;
            codec = profile_probe.codec;
            sample_rate = profile_probe.sample_rate;
            pending_packets = profile_probe.packets;
        }
        let mut channels = source_channel_count(&params)?;
        if matches!(codec, Codec::HeAacV1 | Codec::HeAacV2) {
            channels = 2;
        }
        let metadata = merged_session_metadata(
            format.as_mut(),
            u64::from(track_id),
            limits.max_metadata_string_bytes,
            ebml_metadata.and_then(|metadata| metadata.segment_title),
            id3_metadata,
        );
        let decoder = create_pcm_decoder(&params, codec, limits)?;
        cancellation.check()?;

        Ok(Self {
            format,
            pending_packets,
            decoder,
            info: Box::new(MediaInfo {
                container,
                codec,
                sample_rate,
                channels,
                duration,
                seekable: seekable && container != Container::Adts,
                metadata,
            }),
            cancellation,
            track_id,
            time_base,
            limits,
            consecutive_decode_errors: 0,
        })
    }

    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        &self.info
    }

    #[must_use]
    pub fn limits(&self) -> MediaLimits {
        self.limits
    }

    /// Decodes the next audio packet into caller-owned reusable PCM storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the track is encoded output, a configured bound is exceeded, the
    /// caller's buffer is too small, or demuxing/decoding fails.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, MediaError> {
        self.cancellation.check()?;
        if self.info.codec == Codec::Opus {
            return Err(MediaError::WrongOutputKind {
                codec: self.info.codec,
            });
        }
        loop {
            let Some(packet) = self.next_audio_packet()? else {
                output.clear();
                return Ok(false);
            };
            let timestamp = timestamp_to_std(self.time_base, packet.pts);
            let Some(decoder) = self.decoder.as_mut() else {
                return Err(MediaError::WrongOutputKind {
                    codec: self.info.codec,
                });
            };
            let PcmDecoder::Symphonia(audio_decoder) = decoder else {
                let PcmDecoder::Xaac(xaac) = decoder else {
                    unreachable!();
                };
                if !xaac.decode_access_unit(&packet.data, timestamp, output, self.limits)? {
                    continue;
                }
                self.consecutive_decode_errors = 0;
                return Ok(true);
            };
            let decoded_audio = match audio_decoder.decode(&packet) {
                Ok(audio) => audio,
                Err(SymphoniaError::DecodeError(_)) => {
                    self.consecutive_decode_errors =
                        self.consecutive_decode_errors.saturating_add(1);
                    if self.consecutive_decode_errors > self.limits.max_consecutive_decode_errors {
                        return Err(MediaError::DecodeErrorLimitExceeded {
                            limit: self.limits.max_consecutive_decode_errors,
                        });
                    }
                    continue;
                }
                Err(error) => return Err(backend_error("decode", &error)),
            };
            self.cancellation.check()?;
            self.consecutive_decode_errors = 0;
            let sample_count = decoded_audio.samples_interleaved();
            if sample_count > self.limits.max_pcm_samples_per_frame {
                return Err(MediaError::PcmFrameTooLarge {
                    actual: sample_count,
                    limit: self.limits.max_pcm_samples_per_frame,
                });
            }
            let channels =
                u16::try_from(decoded_audio.spec().channels().count()).map_err(|_| {
                    MediaError::UnsupportedCodec("decoded channel count exceeds u16".to_owned())
                })?;
            let samples = prepare_pcm_output(
                output,
                sample_count,
                decoded_audio.spec().rate(),
                channels,
                timestamp,
            )?;
            decoded_audio.copy_to_slice_interleaved(samples);
            return Ok(true);
        }
    }

    /// Copies the next encoded packet into caller-owned reusable storage.
    ///
    /// # Errors
    ///
    /// Returns an error when the track produces PCM, a configured bound is exceeded, the caller's
    /// buffer is too small, or demuxing fails.
    pub fn read_encoded(&mut self, output: &mut EncodedPacket) -> Result<bool, MediaError> {
        self.cancellation.check()?;
        if self.info.codec != Codec::Opus {
            return Err(MediaError::WrongOutputKind {
                codec: self.info.codec,
            });
        }
        let Some(packet) = self.next_audio_packet()? else {
            output.data.clear();
            return Ok(false);
        };
        if packet.data.len() > output.data.capacity() {
            return Err(MediaError::OutputBufferTooSmall {
                required: packet.data.len(),
                capacity: output.data.capacity(),
            });
        }
        output.data.clear();
        output.data.extend_from_slice(&packet.data);
        output.timestamp = timestamp_to_std(self.time_base, packet.pts);
        output.duration = duration_to_std(self.time_base, packet.dur)
            .filter(|duration| !duration.is_zero())
            .or_else(|| opus_packet_duration(&packet.data));
        Ok(true)
    }

    /// Seeks the selected track to a packet at or near the requested position and resets decoding.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not seekable or the container cannot perform the seek.
    pub fn seek(&mut self, requested: StdDuration) -> Result<SeekResult, MediaError> {
        self.cancellation.check()?;
        if !self.info.seekable {
            return Err(MediaError::Io(io::Error::new(
                io::ErrorKind::Unsupported,
                "media input is not seekable",
            )));
        }
        let total_nanos =
            u64::try_from(requested.as_nanos().min(i64::MAX as u128)).unwrap_or(u64::MAX);
        let result = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from_nanos_u64(total_nanos),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|error| backend_error("seek", &error));
        self.cancellation.check()?;
        let result = result?;
        self.pending_packets.clear();
        if let Some(decoder) = self.decoder.as_mut() {
            match decoder {
                PcmDecoder::Symphonia(decoder) => decoder.reset(),
                PcmDecoder::Xaac(decoder) => decoder.reset()?,
            }
        }
        self.consecutive_decode_errors = 0;
        Ok(SeekResult {
            requested,
            actual: timestamp_to_std(self.time_base, result.actual_ts),
        })
    }

    pub(crate) fn reset_decoder(&mut self) -> Result<(), MediaError> {
        if let Some(decoder) = self.decoder.as_mut() {
            match decoder {
                PcmDecoder::Symphonia(decoder) => decoder.reset(),
                PcmDecoder::Xaac(decoder) => decoder.reset()?,
            }
        }
        self.pending_packets.clear();
        self.consecutive_decode_errors = 0;
        Ok(())
    }

    fn next_audio_packet(&mut self) -> Result<Option<symphonia::core::packet::Packet>, MediaError> {
        if let Some(packet) = self.pending_packets.pop_front() {
            return Ok(Some(packet));
        }
        loop {
            self.cancellation.check()?;
            let packet = self.format.next_packet();
            self.cancellation.check()?;
            let packet = packet.map_err(|error| backend_error("demux", &error))?;
            let Some(packet) = packet else {
                return Ok(None);
            };
            if packet.track_id != self.track_id {
                continue;
            }
            if packet.data.len() > self.limits.max_packet_bytes {
                return Err(MediaError::PacketTooLarge {
                    actual: packet.data.len(),
                    limit: self.limits.max_packet_bytes,
                });
            }
            return Ok(Some(packet));
        }
    }
}

struct ProbedMedia {
    format: Box<dyn FormatReader>,
    seekable: bool,
    ebml_metadata: Option<EbmlMetadata>,
    adts_config: Option<AdtsConfig>,
    id3_metadata: MediaMetadata,
}

fn inspect_initial_id3v2(
    mut input: Box<dyn MediaInput>,
    limits: MediaLimits,
) -> Result<(Box<dyn MediaInput>, MediaMetadata), MediaError> {
    const HEADER_BYTES: usize = 10;

    let byte_len = input.byte_len();
    let seekable = input.is_seekable();
    let initial_position = if seekable {
        input.stream_position()?
    } else {
        0
    };
    let mut header = [0_u8; HEADER_BYTES];
    let header_len = read_up_to(input.as_mut(), &mut header)?;
    let valid_header = header_len == HEADER_BYTES
        && &header[..3] == b"ID3"
        && (2..=4).contains(&header[3])
        && header[4] != 0xff;
    let Some(body_len) = valid_header
        .then(|| decode_syncsafe(&header[6..10]))
        .flatten()
    else {
        if seekable {
            input.seek(SeekFrom::Start(initial_position))?;
            return Ok((input, MediaMetadata::default()));
        }
        let replay = PrefixInput {
            prefix: Cursor::new(header[..header_len].to_vec().into_boxed_slice()),
            inner: input,
            byte_len,
        };
        return Ok((Box::new(replay), MediaMetadata::default()));
    };

    if body_len > limits.max_metadata_string_bytes {
        return Err(metadata_preflight_error(format!(
            "ID3v2 tag declares {body_len} bytes; limit is {}",
            limits.max_metadata_string_bytes
        )));
    }

    let mut body = vec![0_u8; body_len];
    input.read_exact(&mut body)?;
    let footer_len = usize::from(header[3] == 4 && header[5] & 0x10 != 0) * HEADER_BYTES;
    if footer_len != 0 {
        let mut footer = [0_u8; HEADER_BYTES];
        input.read_exact(&mut footer)?;
    }
    let metadata = parse_id3v2_metadata(header[3], header[5], &mut body, limits)?;
    let stripped_len = HEADER_BYTES
        .checked_add(body_len)
        .and_then(|length| length.checked_add(footer_len))
        .ok_or_else(|| metadata_preflight_error("ID3v2 tag length overflow"))?;
    let physical_offset = initial_position
        .checked_add(stripped_len as u64)
        .ok_or_else(|| metadata_preflight_error("ID3v2 input offset overflow"))?;
    let stripped = OffsetInput {
        inner: input,
        physical_offset,
        logical_position: 0,
    };
    Ok((Box::new(stripped), metadata))
}

fn read_up_to(reader: &mut dyn MediaInput, buffer: &mut [u8]) -> io::Result<usize> {
    let mut offset = 0;
    while offset < buffer.len() {
        let count = reader.read(&mut buffer[offset..])?;
        if count == 0 {
            break;
        }
        offset += count;
    }
    Ok(offset)
}

fn decode_syncsafe(bytes: &[u8]) -> Option<usize> {
    if bytes.len() != 4 || bytes.iter().any(|byte| byte & 0x80 != 0) {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0_usize, |value, byte| (value << 7) | usize::from(*byte)),
    )
}

fn decode_id3_unsynchronisation(bytes: &mut Vec<u8>) {
    let mut source = 0;
    let mut destination = 0;
    while source < bytes.len() {
        bytes[destination] = bytes[source];
        destination += 1;
        source += 1;
        if bytes[source - 1] == 0xff && bytes.get(source) == Some(&0) {
            source += 1;
        }
    }
    bytes.truncate(destination);
}

fn parse_id3v2_metadata(
    version: u8,
    flags: u8,
    body: &mut Vec<u8>,
    limits: MediaLimits,
) -> Result<MediaMetadata, MediaError> {
    if version == 2 && flags & 0x40 != 0 {
        return Err(metadata_preflight_error(
            "compressed ID3v2.2 tags are unsupported",
        ));
    }
    if version < 4 && flags & 0x80 != 0 {
        decode_id3_unsynchronisation(body);
    }

    let mut offset = id3_extended_header_len(version, flags, body)?;
    let header_len = if version == 2 { 6 } else { 10 };
    let id_len = if version == 2 { 3 } else { 4 };
    let mut metadata = MediaMetadata::default();

    while body.len().saturating_sub(offset) >= header_len {
        let id = &body[offset..offset + id_len];
        if !id
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            break;
        }
        let frame_len = match version {
            2 => {
                usize::from(body[offset + 3]) << 16
                    | usize::from(body[offset + 4]) << 8
                    | usize::from(body[offset + 5])
            }
            3 => u32::from_be_bytes(body[offset + 4..offset + 8].try_into().unwrap()) as usize,
            4 => decode_syncsafe(&body[offset + 4..offset + 8])
                .ok_or_else(|| metadata_preflight_error("invalid ID3v2.4 frame size"))?,
            _ => unreachable!(),
        };
        let frame_start = offset + header_len;
        let remaining = body.len() - frame_start;
        if frame_len > remaining {
            return Err(metadata_preflight_error(format!(
                "ID3v2 frame declares {frame_len} bytes with only {remaining} remaining"
            )));
        }
        let frame_end = frame_start + frame_len;
        let format_flags = if version == 2 {
            0
        } else {
            u16::from_be_bytes(body[offset + 8..offset + 10].try_into().unwrap())
        };
        let safely_decodable = match version {
            2 => true,
            3 => format_flags & 0x00e0 == 0,
            4 => format_flags.trailing_zeros() >= 4,
            _ => false,
        };
        if frame_len != 0 && safely_decodable {
            let value = decode_id3_text(
                &body[frame_start..frame_end],
                limits.max_metadata_string_bytes,
            );
            match id {
                b"TT2" | b"TIT2" if metadata.title.is_none() => metadata.title = value,
                b"TP1" | b"TPE1" if metadata.author.is_none() => metadata.author = value,
                b"TRC" | b"TSRC" if metadata.isrc.is_none() => metadata.isrc = value,
                _ => {}
            }
        }
        offset = frame_end;
    }
    Ok(metadata)
}

fn id3_extended_header_len(version: u8, flags: u8, body: &[u8]) -> Result<usize, MediaError> {
    if flags & 0x40 == 0 || version == 2 {
        return Ok(0);
    }
    if body.len() < 4 {
        return Err(metadata_preflight_error("truncated ID3v2 extended header"));
    }
    let length = if version == 3 {
        let payload = u32::from_be_bytes(body[..4].try_into().unwrap()) as usize;
        if !matches!(payload, 6 | 10) {
            return Err(metadata_preflight_error("invalid ID3v2.3 extended header"));
        }
        4 + payload
    } else {
        let total = decode_syncsafe(&body[..4])
            .ok_or_else(|| metadata_preflight_error("invalid ID3v2.4 extended header"))?;
        if total < 6 {
            return Err(metadata_preflight_error("invalid ID3v2.4 extended header"));
        }
        total
    };
    if length > body.len() {
        return Err(metadata_preflight_error("truncated ID3v2 extended header"));
    }
    Ok(length)
}

fn decode_id3_text(frame: &[u8], limit: usize) -> Option<String> {
    let (&encoding, payload) = frame.split_first()?;
    let mut value = match encoding {
        0 => payload.iter().map(|byte| char::from(*byte)).collect(),
        1 | 2 => {
            let (payload, little_endian) = if encoding == 2 {
                (payload, false)
            } else if payload.starts_with(&[0xff, 0xfe]) {
                (&payload[2..], true)
            } else if payload.starts_with(&[0xfe, 0xff]) {
                (&payload[2..], false)
            } else {
                return None;
            };
            if payload.len() % 2 != 0 {
                return None;
            }
            let units = payload.chunks_exact(2).map(|bytes| {
                if little_endian {
                    u16::from_le_bytes([bytes[0], bytes[1]])
                } else {
                    u16::from_be_bytes([bytes[0], bytes[1]])
                }
            });
            String::from_utf16(&units.collect::<Vec<_>>()).ok()?
        }
        3 => String::from_utf8(payload.to_vec()).ok()?,
        _ => return None,
    };
    if let Some(terminator) = value.find('\0') {
        value.truncate(terminator);
    }
    (!value.is_empty() && value.len() <= limit).then_some(value)
}

fn metadata_preflight_error(message: impl Into<String>) -> MediaError {
    MediaError::Backend {
        operation: "metadata preflight",
        message: message.into(),
    }
}

fn container_preflight_error(message: impl Into<String>) -> MediaError {
    MediaError::Backend {
        operation: "container preflight",
        message: message.into(),
    }
}

#[allow(clippy::too_many_lines)]
fn inspect_mp4_packet_sizes(
    input: &mut dyn MediaInput,
    max_probe_bytes: u64,
    max_packet_bytes: usize,
    cancellation: &MediaCancellation,
) -> Result<(), MediaError> {
    const MAX_DEPTH: usize = 16;

    fn read_at(
        input: &mut dyn MediaInput,
        position: u64,
        buffer: &mut [u8],
        inspected: &mut u64,
        limit: u64,
        cancellation: &MediaCancellation,
    ) -> Result<(), MediaError> {
        cancellation.check()?;
        let requested = buffer.len() as u64;
        *inspected = inspected
            .checked_add(requested)
            .ok_or(MediaError::ProbeLimitExceeded { limit })?;
        if *inspected > limit {
            return Err(MediaError::ProbeLimitExceeded { limit });
        }
        input.seek(SeekFrom::Start(position))?;
        input.read_exact(buffer)?;
        cancellation.check()
    }

    fn reject_oversized_packet(size: u32, limit: usize) -> Result<(), MediaError> {
        if u64::from(size) > limit as u64 {
            return Err(container_preflight_error(format!(
                "MP4 sample declares {size} bytes; limit is {limit}"
            )));
        }
        Ok(())
    }

    fn require_bytes(
        position: u64,
        length: u64,
        end: u64,
        message: &'static str,
    ) -> Result<(), MediaError> {
        if position
            .checked_add(length)
            .is_none_or(|required_end| required_end > end)
        {
            return Err(container_preflight_error(message));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn inspect_range(
        input: &mut dyn MediaInput,
        start: u64,
        end: u64,
        depth: usize,
        inspected: &mut u64,
        probe_limit: u64,
        packet_limit: usize,
        cancellation: &MediaCancellation,
    ) -> Result<(), MediaError> {
        if depth > MAX_DEPTH {
            return Err(container_preflight_error(
                "MP4 box nesting exceeds the preflight limit",
            ));
        }

        let mut position = start;
        while end.saturating_sub(position) >= 8 {
            let mut header = [0_u8; 16];
            read_at(
                input,
                position,
                &mut header[..8],
                inspected,
                probe_limit,
                cancellation,
            )?;
            let short_size = u32::from_be_bytes(header[..4].try_into().expect("fixed slice"));
            let kind: [u8; 4] = header[4..8].try_into().expect("fixed slice");
            let (box_size, header_size) = match short_size {
                0 => (end - position, 8_u64),
                1 => {
                    read_at(
                        input,
                        position + 8,
                        &mut header[8..16],
                        inspected,
                        probe_limit,
                        cancellation,
                    )?;
                    (
                        u64::from_be_bytes(header[8..16].try_into().expect("fixed slice")),
                        16,
                    )
                }
                size => (u64::from(size), 8),
            };
            if box_size < header_size {
                return Err(container_preflight_error(
                    "MP4 box is smaller than its header",
                ));
            }
            let box_end = position
                .checked_add(box_size)
                .filter(|box_end| *box_end <= end)
                .ok_or_else(|| container_preflight_error("MP4 box exceeds its parent"))?;
            let payload = position + header_size;

            match &kind {
                b"stsz" => {
                    require_bytes(payload, 12, box_end, "MP4 sample-size box is truncated")?;
                    let mut fields = [0_u8; 12];
                    read_at(
                        input,
                        payload,
                        &mut fields,
                        inspected,
                        probe_limit,
                        cancellation,
                    )?;
                    let default_size =
                        u32::from_be_bytes(fields[4..8].try_into().expect("fixed slice"));
                    let sample_count =
                        u32::from_be_bytes(fields[8..12].try_into().expect("fixed slice"));
                    if default_size != 0 {
                        reject_oversized_packet(default_size, packet_limit)?;
                    } else {
                        let table_bytes =
                            u64::from(sample_count).checked_mul(4).ok_or_else(|| {
                                container_preflight_error("MP4 sample-size table overflows")
                            })?;
                        require_bytes(
                            payload + 12,
                            table_bytes,
                            box_end,
                            "MP4 sample-size table is truncated",
                        )?;
                        let mut cursor = payload + 12;
                        let mut remaining = sample_count;
                        let mut chunk = [0_u8; 1024];
                        while remaining != 0 {
                            let entries = remaining.min(256);
                            let bytes = entries as usize * 4;
                            read_at(
                                input,
                                cursor,
                                &mut chunk[..bytes],
                                inspected,
                                probe_limit,
                                cancellation,
                            )?;
                            for value in chunk[..bytes].chunks_exact(4) {
                                reject_oversized_packet(
                                    u32::from_be_bytes(value.try_into().expect("fixed slice")),
                                    packet_limit,
                                )?;
                            }
                            cursor += bytes as u64;
                            remaining -= entries;
                        }
                    }
                }
                b"tfhd" => {
                    require_bytes(
                        payload,
                        8,
                        box_end,
                        "MP4 track-fragment header is truncated",
                    )?;
                    let mut fields = [0_u8; 8];
                    read_at(
                        input,
                        payload,
                        &mut fields,
                        inspected,
                        probe_limit,
                        cancellation,
                    )?;
                    let flags = u32::from_be_bytes(fields[..4].try_into().expect("fixed slice"))
                        & 0x00ff_ffff;
                    let mut cursor = payload + 8;
                    if flags & 0x0000_0001 != 0 {
                        cursor += 8;
                    }
                    if flags & 0x0000_0002 != 0 {
                        cursor += 4;
                    }
                    if flags & 0x0000_0008 != 0 {
                        cursor += 4;
                    }
                    if flags & 0x0000_0010 != 0 {
                        let mut size = [0_u8; 4];
                        require_bytes(
                            cursor,
                            4,
                            box_end,
                            "MP4 track-fragment header is truncated",
                        )?;
                        read_at(
                            input,
                            cursor,
                            &mut size,
                            inspected,
                            probe_limit,
                            cancellation,
                        )?;
                        reject_oversized_packet(u32::from_be_bytes(size), packet_limit)?;
                    }
                }
                b"trex" => {
                    require_bytes(payload, 20, box_end, "MP4 track-extension box is truncated")?;
                    let mut fields = [0_u8; 20];
                    read_at(
                        input,
                        payload,
                        &mut fields,
                        inspected,
                        probe_limit,
                        cancellation,
                    )?;
                    reject_oversized_packet(
                        u32::from_be_bytes(fields[16..20].try_into().expect("fixed slice")),
                        packet_limit,
                    )?;
                }
                b"trun" => {
                    require_bytes(payload, 8, box_end, "MP4 fragment table is truncated")?;
                    let mut fields = [0_u8; 8];
                    read_at(
                        input,
                        payload,
                        &mut fields,
                        inspected,
                        probe_limit,
                        cancellation,
                    )?;
                    let flags = u32::from_be_bytes(fields[..4].try_into().expect("fixed slice"))
                        & 0x00ff_ffff;
                    let sample_count =
                        u32::from_be_bytes(fields[4..8].try_into().expect("fixed slice"));
                    let mut cursor = payload + 8;
                    if flags & 0x0000_0001 != 0 {
                        cursor += 4;
                    }
                    if flags & 0x0000_0004 != 0 {
                        cursor += 4;
                    }
                    let duration = usize::from(flags & 0x0000_0100 != 0) * 4;
                    let size_offset = duration;
                    let stride = duration
                        + usize::from(flags & 0x0000_0200 != 0) * 4
                        + usize::from(flags & 0x0000_0400 != 0) * 4
                        + usize::from(flags & 0x0000_0800 != 0) * 4;
                    let table_bytes = u64::from(sample_count)
                        .checked_mul(stride as u64)
                        .ok_or_else(|| container_preflight_error("MP4 fragment table overflows"))?;
                    require_bytes(
                        cursor,
                        table_bytes,
                        box_end,
                        "MP4 fragment table is truncated",
                    )?;
                    if flags & 0x0000_0200 != 0 {
                        let mut remaining = sample_count;
                        let mut chunk = [0_u8; 4096];
                        while remaining != 0 {
                            let entries = remaining.min(
                                u32::try_from(chunk.len() / stride)
                                    .expect("fixed buffer and MP4 field width fit u32"),
                            );
                            let bytes = entries as usize * stride;
                            read_at(
                                input,
                                cursor,
                                &mut chunk[..bytes],
                                inspected,
                                probe_limit,
                                cancellation,
                            )?;
                            for entry in chunk[..bytes].chunks_exact(stride) {
                                reject_oversized_packet(
                                    u32::from_be_bytes(
                                        entry[size_offset..size_offset + 4]
                                            .try_into()
                                            .expect("fixed slice"),
                                    ),
                                    packet_limit,
                                )?;
                            }
                            cursor += bytes as u64;
                            remaining -= entries;
                        }
                    }
                }
                b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"edts" | b"udta" | b"moof"
                | b"traf" | b"mvex" => inspect_range(
                    input,
                    payload,
                    box_end,
                    depth + 1,
                    inspected,
                    probe_limit,
                    packet_limit,
                    cancellation,
                )?,
                b"meta" => {
                    if payload + 4 > box_end {
                        return Err(container_preflight_error("MP4 metadata box is truncated"));
                    }
                    inspect_range(
                        input,
                        payload + 4,
                        box_end,
                        depth + 1,
                        inspected,
                        probe_limit,
                        packet_limit,
                        cancellation,
                    )?;
                }
                _ => {}
            }

            position = box_end;
            if short_size == 0 {
                break;
            }
        }
        Ok(())
    }

    let Some(input_len) = input.byte_len() else {
        return Err(container_preflight_error(
            "MP4 input length is unavailable for bounded preflight",
        ));
    };
    let initial_position = input.stream_position()?;
    let mut inspected = 0;
    let result = inspect_range(
        input,
        initial_position,
        input_len,
        0,
        &mut inspected,
        max_probe_bytes,
        max_packet_bytes,
        cancellation,
    );
    let restored = input.seek(SeekFrom::Start(initial_position));
    result?;
    restored?;
    Ok(())
}

fn probe_media_input(
    input: Box<dyn MediaInput>,
    extension_hint: Option<&str>,
    limits: MediaLimits,
    cancellation: &MediaCancellation,
) -> Result<ProbedMedia, MediaError> {
    let (mut input, id3_metadata) = inspect_initial_id3v2(input, limits)?;
    let seekable = input.is_seekable();
    if seekable
        && extension_hint.is_some_and(|extension| {
            extension.eq_ignore_ascii_case("m4a")
                || extension.eq_ignore_ascii_case("mp4")
                || extension.eq_ignore_ascii_case("mov")
        })
    {
        inspect_mp4_packet_sizes(
            input.as_mut(),
            limits.max_probe_bytes,
            limits.max_packet_bytes,
            cancellation,
        )?;
    }
    let ebml_metadata = if seekable {
        inspect_ebml_metadata(
            input.as_mut(),
            limits.max_probe_bytes,
            limits.max_metadata_string_bytes,
            cancellation,
        )?
    } else {
        match extension_hint {
            Some(extension) if extension.eq_ignore_ascii_case("webm") => Some(EbmlMetadata {
                container: Container::WebM,
                segment_title: None,
                output_sample_rate: None,
            }),
            Some(extension) if extension.eq_ignore_ascii_case("mkv") => Some(EbmlMetadata {
                container: Container::Matroska,
                segment_title: None,
                output_sample_rate: None,
            }),
            _ => None,
        }
    };
    let adts_config = if seekable
        && ebml_metadata.is_none()
        && extension_hint.is_some_and(|extension| {
            extension.eq_ignore_ascii_case("aac") || extension.eq_ignore_ascii_case("adts")
        }) {
        inspect_adts_config(input.as_mut(), limits.max_probe_bytes, cancellation)?
    } else {
        None
    };
    let probe_state = Arc::new(ProbeState::new(limits.max_probe_bytes));
    let source = Box::new(InputAdapter {
        input,
        probe_state: Arc::clone(&probe_state),
        cancellation: cancellation.clone(),
    });
    let stream = MediaSourceStream::new(
        source,
        MediaSourceStreamOptions {
            buffer_len: limits.input_buffer_bytes,
        },
    );
    let mut hint = Hint::new();
    if adts_config.is_some() {
        hint.with_extension("aac");
    } else if let Some(extension) = extension_hint {
        hint.with_extension(extension);
    }
    let metadata_options = MetadataOptions::default()
        .limit_tag_bytes(Limit::Maximum(limits.max_metadata_string_bytes))
        .limit_visual_bytes(Limit::Maximum(0));
    let probed = symphonia::default::get_probe().probe(
        &hint,
        stream,
        FormatOptions::default(),
        metadata_options,
    );
    probe_state.active.store(false, Ordering::Release);
    cancellation.check()?;
    match probed {
        Ok(format) => Ok(ProbedMedia {
            format,
            seekable,
            ebml_metadata,
            adts_config,
            id3_metadata,
        }),
        Err(_) if probe_state.exceeded.load(Ordering::Acquire) => {
            Err(MediaError::ProbeLimitExceeded {
                limit: limits.max_probe_bytes,
            })
        }
        Err(error) => Err(backend_error("probe", &error)),
    }
}

#[derive(Clone, Copy)]
struct AdtsConfig {
    audio_object_type: u8,
    sample_rate_index: u8,
    sample_rate: u32,
    channel_configuration: u8,
}

impl AdtsConfig {
    fn parse(header: &[u8]) -> Option<Self> {
        const SAMPLE_RATES: [u32; 13] = [
            96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
            8_000, 7_350,
        ];
        if header.len() < 7 || header[0] != 0xff || header[1] & 0xf6 != 0xf0 || header[6] & 3 != 0 {
            return None;
        }
        let protection_absent = header[1] & 1 != 0;
        let header_bytes = if protection_absent { 7 } else { 9 };
        let frame_bytes = (usize::from(header[3] & 3) << 11)
            | (usize::from(header[4]) << 3)
            | (usize::from(header[5]) >> 5);
        if frame_bytes < header_bytes {
            return None;
        }
        let sample_rate_index = (header[2] >> 2) & 15;
        let sample_rate = *SAMPLE_RATES.get(usize::from(sample_rate_index))?;
        let channel_configuration = ((header[2] & 1) << 2) | (header[3] >> 6);
        if !(1..=2).contains(&channel_configuration) {
            return None;
        }
        Some(Self {
            audio_object_type: ((header[2] >> 6) & 3) + 1,
            sample_rate_index,
            sample_rate,
            channel_configuration,
        })
    }

    fn from_codec_params(
        params: &symphonia::core::codecs::audio::AudioCodecParameters,
    ) -> Option<Self> {
        const SAMPLE_RATES: [u32; 13] = [
            96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
            8_000, 7_350,
        ];
        let sample_rate = params.sample_rate?;
        let sample_rate_index = u8::try_from(
            SAMPLE_RATES
                .iter()
                .position(|candidate| *candidate == sample_rate)?,
        )
        .ok()?;
        let channel_configuration = u8::try_from(params.channels.as_ref()?.count()).ok()?;
        if !(1..=2).contains(&channel_configuration) {
            return None;
        }
        Some(Self {
            audio_object_type: 2,
            sample_rate_index,
            sample_rate,
            channel_configuration,
        })
    }

    fn audio_specific_config(self) -> [u8; 2] {
        let bits = (u16::from(self.audio_object_type) << 11)
            | (u16::from(self.sample_rate_index) << 7)
            | (u16::from(self.channel_configuration) << 3);
        bits.to_be_bytes()
    }
}

fn apply_adts_track_config(
    params: &mut symphonia::core::codecs::audio::AudioCodecParameters,
    inspected: Option<AdtsConfig>,
) -> Result<(), MediaError> {
    let config = inspected
        .or_else(|| AdtsConfig::from_codec_params(params))
        .ok_or(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "ADTS header does not declare bounded AAC-LC parameters",
        })?;
    if config.audio_object_type != 2 {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "ADTS audio object type is not AAC-LC",
        });
    }
    params.with_sample_rate(config.sample_rate);
    params.with_extra_data(config.audio_specific_config().into());
    Ok(())
}

fn source_channel_count(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
) -> Result<u16, MediaError> {
    params
        .channels
        .as_ref()
        .map(symphonia::core::audio::Channels::count)
        .and_then(|channels| u16::try_from(channels).ok())
        .ok_or_else(|| {
            MediaError::UnsupportedCodec(format!(
                "{} without a supported channel count",
                params.codec
            ))
        })
}

fn validate_wave_track_params(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
) -> Result<(), MediaError> {
    let sample_rate = params
        .sample_rate
        .ok_or_else(|| MediaError::UnsupportedCodec("WAVE PCM without a sample rate".to_owned()))?;
    if !(100..=384_000).contains(&sample_rate) {
        return Err(MediaError::UnsupportedCodec(format!(
            "WAVE PCM sample rate {sample_rate} is outside 100..=384000 Hz"
        )));
    }
    let channels = source_channel_count(params)?;
    if !(1..=2).contains(&channels) {
        return Err(MediaError::UnsupportedCodec(format!(
            "WAVE PCM channel count {channels} is outside Mantle's mono/stereo scope"
        )));
    }
    Ok(())
}

fn inspect_adts_config(
    input: &mut dyn MediaInput,
    max_probe_bytes: u64,
    cancellation: &MediaCancellation,
) -> Result<Option<AdtsConfig>, MediaError> {
    cancellation.check()?;
    let original = input.stream_position()?;
    let scan_distance = usize::try_from(max_probe_bytes)
        .unwrap_or(usize::MAX)
        .min(MAX_ADTS_SCAN_DISTANCE);
    let available = usize::try_from(input.byte_len().unwrap_or(scan_distance as u64))
        .unwrap_or(usize::MAX)
        .min(scan_distance);
    let mut bytes = vec![0_u8; available];
    let mut read = 0;
    while read < bytes.len() {
        cancellation.check()?;
        let count = input.read(&mut bytes[read..])?;
        cancellation.check()?;
        if count == 0 {
            break;
        }
        read += count;
    }
    let found = bytes[..read]
        .windows(7)
        .take(scan_distance)
        .enumerate()
        .find_map(|(offset, header)| AdtsConfig::parse(header).map(|config| (offset, config)));
    let Some((offset, config)) = found else {
        input.seek(SeekFrom::Start(original))?;
        return Ok(None);
    };
    input.seek(SeekFrom::Start(original.saturating_add(offset as u64)))?;
    cancellation.check()?;
    Ok(Some(config))
}

struct EbmlMetadata {
    container: Container,
    segment_title: Option<String>,
    output_sample_rate: Option<u32>,
}

fn inspect_ebml_metadata(
    input: &mut dyn MediaInput,
    max_probe_bytes: u64,
    max_metadata_string_bytes: usize,
    cancellation: &MediaCancellation,
) -> Result<Option<EbmlMetadata>, MediaError> {
    cancellation.check()?;
    let original = input.stream_position()?;
    let inspection_bytes = usize::try_from(input.byte_len().unwrap_or(u64::MAX))
        .unwrap_or(usize::MAX)
        .min(usize::try_from(max_probe_bytes).unwrap_or(usize::MAX))
        .min(MAX_EBML_INSPECTION_BYTES);
    let mut header = vec![0_u8; inspection_bytes];
    let mut read = 0;
    while read < header.len() {
        cancellation.check()?;
        let count = input.read(&mut header[read..])?;
        cancellation.check()?;
        if count == 0 {
            break;
        }
        read += count;
    }
    input.seek(SeekFrom::Start(original))?;
    cancellation.check()?;
    let header = &header[..read];
    let Some(ebml_header) = find_ebml_element(header, 0x1a45_dfa3) else {
        return Ok(None);
    };
    let container = match find_ebml_element(ebml_header.data, 0x4282)
        .filter(|element| element.complete)
        .and_then(|element| std::str::from_utf8(element.data).ok())
    {
        Some("webm") => Container::WebM,
        Some("matroska") => Container::Matroska,
        _ => return Ok(None),
    };
    let segment_title = find_ebml_element(header, 0x1853_8067)
        .and_then(|segment| find_ebml_element(segment.data, 0x1549_a966))
        .and_then(|info| find_ebml_element(info.data, 0x7ba9))
        .filter(|title| title.complete && title.data.len() <= max_metadata_string_bytes)
        .and_then(|title| std::str::from_utf8(title.data).ok())
        .filter(|title| !title.is_empty())
        .map(str::to_owned);
    let output_sample_rate = find_ebml_element(header, 0x1853_8067)
        .and_then(|segment| find_ebml_element(segment.data, 0x1654_ae6b))
        .and_then(|tracks| default_ebml_audio_output_sample_rate(tracks.data));
    Ok(Some(EbmlMetadata {
        container,
        segment_title,
        output_sample_rate,
    }))
}

struct AacProfileProbe {
    codec: Codec,
    sample_rate: u32,
    packets: VecDeque<symphonia::core::packet::Packet>,
}

fn probe_implicit_aac_profile(
    format: &mut dyn FormatReader,
    track_id: u32,
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
    limits: MediaLimits,
    cancellation: &MediaCancellation,
) -> Result<AacProfileProbe, MediaError> {
    let Some(config_bytes) = params.extra_data.as_deref() else {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "implicit profile probe requires AudioSpecificConfig",
        });
    };
    let Some(config) = parse_audio_specific_config(config_bytes) else {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "implicit profile probe received invalid AudioSpecificConfig",
        });
    };
    if config.audio_object_type != 2 || config.explicit_he {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "implicit profile probe requires AAC-LC core signaling",
        });
    }
    let candidate = if config.channel_configuration == 1 {
        Codec::HeAacV2
    } else {
        Codec::HeAacV1
    };
    let mut decoder = create_xaac_decoder(params, candidate, limits)?;

    let mut detected = None;
    let mut packets = VecDeque::with_capacity(MAX_AAC_PROFILE_PROBE_PACKETS);
    let mut probe_bytes = 0_usize;
    for _ in 0..MAX_AAC_PROFILE_PROBE_PACKETS {
        cancellation.check()?;
        let packet = format
            .next_packet()
            .map_err(|error| backend_error("AAC profile probe", &error))?;
        let Some(packet) = packet else { break };
        if packet.track_id != track_id {
            continue;
        }
        if packet.data.len() > limits.max_packet_bytes {
            return Err(MediaError::PacketTooLarge {
                actual: packet.data.len(),
                limit: limits.max_packet_bytes,
            });
        }
        probe_bytes = probe_bytes.checked_add(packet.data.len()).ok_or(
            MediaError::CodecProbeLimitExceeded {
                actual: usize::MAX,
                limit: limits.max_codec_probe_bytes,
            },
        )?;
        if probe_bytes > limits.max_codec_probe_bytes {
            return Err(MediaError::CodecProbeLimitExceeded {
                actual: probe_bytes,
                limit: limits.max_codec_probe_bytes,
            });
        }
        let decode_status = decoder.decode_access_unit(&packet.data);
        packets.push_back(packet);
        match decode_status {
            Ok(XaacDecodeStatus::NeedMoreInput) => {}
            Ok(XaacDecodeStatus::Frame(frame)) => {
                detected = Some(if frame.sample_rate() > config.core_sample_rate {
                    (candidate, frame.sample_rate())
                } else {
                    (Codec::AacLc, frame.sample_rate())
                });
                break;
            }
            Err(error) => return Err(native_backend_error("AAC profile probe", &error)),
        }
    }
    cancellation.check()?;
    let Some((codec, sample_rate)) = detected else {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "implicit profile was not resolved within configured probe bounds",
        });
    };
    Ok(AacProfileProbe {
        codec,
        sample_rate,
        packets,
    })
}

struct EbmlElement<'a> {
    data: &'a [u8],
    complete: bool,
}

fn find_ebml_element(mut bytes: &[u8], target_id: u64) -> Option<EbmlElement<'_>> {
    while !bytes.is_empty() {
        let (id, element, consumed) = read_ebml_element(bytes)?;
        if id == target_id {
            return Some(element);
        }
        bytes = bytes.get(consumed..)?;
    }
    None
}

fn read_ebml_element(bytes: &[u8]) -> Option<(u64, EbmlElement<'_>, usize)> {
    let (id, id_bytes) = read_ebml_vint(bytes, true)?;
    let (size, size_bytes) = read_ebml_vint(&bytes[id_bytes..], false)?;
    let header_bytes = id_bytes.checked_add(size_bytes)?;
    let payload = bytes.get(header_bytes..)?;
    let declared_size = usize::try_from(size).ok();
    let available_size = declared_size.unwrap_or(payload.len()).min(payload.len());
    let complete = declared_size.is_some_and(|size| size <= payload.len());
    let consumed = header_bytes.checked_add(declared_size.unwrap_or(payload.len()))?;
    Some((
        id,
        EbmlElement {
            data: &payload[..available_size],
            complete,
        },
        consumed,
    ))
}

fn default_ebml_audio_output_sample_rate(mut tracks: &[u8]) -> Option<u32> {
    let mut first_audio = None;
    while !tracks.is_empty() {
        let (id, track, consumed) = read_ebml_element(tracks)?;
        if id == 0xae {
            let track_type = find_ebml_element(track.data, 0x83)
                .and_then(|element| read_ebml_uint(element.data));
            if track_type == Some(2) {
                let output_sample_rate = find_ebml_element(track.data, 0xe1)
                    .and_then(|audio| find_ebml_element(audio.data, 0x78b5))
                    .and_then(|frequency| read_ebml_float(frequency.data))
                    .and_then(ebml_sample_rate);
                let is_default = find_ebml_element(track.data, 0x88)
                    .and_then(|element| read_ebml_uint(element.data))
                    .is_none_or(|flag| flag != 0);
                if is_default {
                    return output_sample_rate;
                }
                if first_audio.is_none() {
                    first_audio = Some(output_sample_rate);
                }
            }
        }
        tracks = tracks.get(consumed..)?;
    }
    first_audio.flatten()
}

fn read_ebml_uint(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || bytes.len() > 8 {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte)),
    )
}

fn read_ebml_float(bytes: &[u8]) -> Option<f64> {
    match bytes.len() {
        4 => Some(f64::from(f32::from_bits(u32::from_be_bytes(
            bytes.try_into().ok()?,
        )))),
        8 => Some(f64::from_bits(u64::from_be_bytes(bytes.try_into().ok()?))),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn ebml_sample_rate(frequency: f64) -> Option<u32> {
    if !frequency.is_finite() || !(1.0..=f64::from(u32::MAX)).contains(&frequency) {
        return None;
    }
    // The finite positive range check above makes the rounded conversion lossless for the
    // integer-valued Matroska sampling-frequency contract.
    Some(frequency.round() as u32)
}

fn read_ebml_vint(bytes: &[u8], retain_marker: bool) -> Option<(u64, usize)> {
    let first = *bytes.first()?;
    let width = first.leading_zeros() as usize + 1;
    if first == 0 || width > 8 || bytes.len() < width {
        return None;
    }
    let marker = 1_u8 << (8 - width);
    let mut value = if retain_marker {
        u64::from(first)
    } else {
        u64::from(first & !marker)
    };
    for byte in &bytes[1..width] {
        value = (value << 8) | u64::from(*byte);
    }
    if !retain_marker {
        let unknown = (1_u64 << (7 * width)) - 1;
        if value == unknown {
            return Some((u64::MAX, width));
        }
    }
    Some((value, width))
}

enum PcmDecoder {
    Symphonia(Box<dyn AudioDecoder>),
    Xaac(Box<XaacPcmDecoder>),
}

struct XaacPcmDecoder {
    decoder: XaacDecoder,
    pending_timestamps: VecDeque<Option<StdDuration>>,
}

impl XaacPcmDecoder {
    fn new(decoder: XaacDecoder) -> Self {
        Self {
            decoder,
            pending_timestamps: VecDeque::with_capacity(MAX_XAAC_DELAYED_TIMESTAMPS),
        }
    }

    fn decode_access_unit(
        &mut self,
        data: &[u8],
        timestamp: Option<StdDuration>,
        output: &mut PcmFrame,
        limits: MediaLimits,
    ) -> Result<bool, MediaError> {
        if self.pending_timestamps.len() >= MAX_XAAC_DELAYED_TIMESTAMPS {
            return Err(MediaError::DecodeDelayLimitExceeded {
                limit: MAX_XAAC_DELAYED_TIMESTAMPS,
            });
        }
        self.pending_timestamps.push_back(timestamp);
        let native_status = self
            .decoder
            .decode_access_unit(data)
            .map_err(xaac_decode_error)?;
        let XaacDecodeStatus::Frame(pcm) = native_status else {
            return Ok(false);
        };
        let timestamp = self
            .pending_timestamps
            .pop_front()
            .ok_or_else(|| MediaError::Backend {
                operation: "decode",
                message: "native decoder produced output without a queued timestamp".to_owned(),
            })?;
        let sample_count = pcm.bytes().len() / 2;
        if sample_count > limits.max_pcm_samples_per_frame {
            return Err(MediaError::PcmFrameTooLarge {
                actual: sample_count,
                limit: limits.max_pcm_samples_per_frame,
            });
        }
        let samples = prepare_pcm_output(
            output,
            sample_count,
            pcm.sample_rate(),
            pcm.channels(),
            timestamp,
        )?;
        for (sample, encoded) in samples.iter_mut().zip(pcm.bytes().chunks_exact(2)) {
            *sample = f32::from(i16::from_ne_bytes([encoded[0], encoded[1]])) / 32_768.0;
        }
        Ok(true)
    }

    fn reset(&mut self) -> Result<(), MediaError> {
        self.decoder
            .reset()
            .map_err(|error| native_backend_error("seek reset", &error))?;
        self.pending_timestamps.clear();
        Ok(())
    }
}

fn create_pcm_decoder(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
    codec: Codec,
    limits: MediaLimits,
) -> Result<Option<PcmDecoder>, MediaError> {
    match codec {
        Codec::Opus => Ok(None),
        Codec::HeAacV1 | Codec::HeAacV2 => create_xaac_decoder(params, codec, limits)
            .map(XaacPcmDecoder::new)
            .map(Box::new)
            .map(PcmDecoder::Xaac)
            .map(Some),
        _ => symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map(PcmDecoder::Symphonia)
            .map(Some)
            .map_err(|error| backend_error("decoder creation", &error)),
    }
}

struct ProbeState {
    active: AtomicBool,
    exceeded: AtomicBool,
    bytes_read: AtomicU64,
    limit: u64,
}

impl ProbeState {
    fn new(limit: u64) -> Self {
        Self {
            active: AtomicBool::new(true),
            exceeded: AtomicBool::new(false),
            bytes_read: AtomicU64::new(0),
            limit,
        }
    }
}

struct InputAdapter {
    input: Box<dyn MediaInput>,
    probe_state: Arc<ProbeState>,
    cancellation: MediaCancellation,
}

impl Read for InputAdapter {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if !self.probe_state.active.load(Ordering::Acquire) {
            let result = self.input.read(buffer);
            self.cancellation.check_io()?;
            return result;
        }
        let read = self.probe_state.bytes_read.load(Ordering::Relaxed);
        let remaining = self.probe_state.limit.saturating_sub(read);
        if remaining == 0 {
            self.probe_state.exceeded.store(true, Ordering::Release);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "probe byte limit exceeded",
            ));
        }
        let allowed = usize::try_from(remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        let count = self.input.read(&mut buffer[..allowed])?;
        self.cancellation.check_io()?;
        self.probe_state
            .bytes_read
            .fetch_add(count as u64, Ordering::Relaxed);
        Ok(count)
    }
}

impl Seek for InputAdapter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.cancellation.check_io()?;
        let result = self.input.seek(position);
        self.cancellation.check_io()?;
        result
    }
}

impl SymphoniaMediaSource for InputAdapter {
    fn is_seekable(&self) -> bool {
        self.input.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.input.byte_len()
    }
}

fn merged_session_metadata(
    format: &mut dyn FormatReader,
    track_id: u64,
    max_string_bytes: usize,
    ebml_title: Option<String>,
    id3: MediaMetadata,
) -> MediaMetadata {
    let mut metadata = extract_metadata(format, track_id, max_string_bytes);
    if metadata.title.is_none() {
        metadata.title = ebml_title.or(id3.title);
    }
    if metadata.author.is_none() {
        metadata.author = id3.author;
    }
    if metadata.isrc.is_none() {
        metadata.isrc = id3.isrc;
    }
    metadata
}

fn extract_metadata(
    format: &mut dyn FormatReader,
    track_id: u64,
    max_string_bytes: usize,
) -> MediaMetadata {
    let mut result = MediaMetadata::default();
    let mut revisions = format.metadata();
    loop {
        if let Some(revision) = revisions.current() {
            apply_metadata_container(&mut result, &revision.media, max_string_bytes, false);
            if let Some(track) = revision
                .per_track
                .iter()
                .find(|track| track.track_id == track_id)
            {
                apply_metadata_container(&mut result, &track.metadata, max_string_bytes, true);
            }
        }
        if revisions.pop().is_none() {
            break;
        }
    }
    result
}

fn apply_metadata_container(
    result: &mut MediaMetadata,
    container: &MetadataContainer,
    max_string_bytes: usize,
    overwrite: bool,
) {
    for tag in &container.tags {
        let Some((field, value)) = standard_metadata_value(tag) else {
            continue;
        };
        if value.is_empty() || value.len() > max_string_bytes {
            continue;
        }
        let target = match field {
            MetadataField::Title => &mut result.title,
            MetadataField::Author => &mut result.author,
            MetadataField::Isrc => &mut result.isrc,
        };
        if overwrite || target.is_none() {
            *target = Some(value.to_owned());
        }
    }
}

#[derive(Clone, Copy)]
enum MetadataField {
    Title,
    Author,
    Isrc,
}

fn standard_metadata_value(tag: &Tag) -> Option<(MetadataField, &str)> {
    match tag.std.as_ref()? {
        StandardTag::TrackTitle(value)
        | StandardTag::Album(value)
        | StandardTag::MovieTitle(value) => Some((MetadataField::Title, value.as_str())),
        StandardTag::Artist(value)
        | StandardTag::AlbumArtist(value)
        | StandardTag::Author(value) => Some((MetadataField::Author, value.as_str())),
        StandardTag::IdentIsrc(value) => Some((MetadataField::Isrc, value.as_str())),
        _ => None,
    }
}

fn map_container(name: &str, ebml_container: Option<Container>) -> Result<Container, MediaError> {
    match name {
        "wave" => Ok(Container::Wave),
        "mp3" => Ok(Container::Mp3),
        "isomp4" => Ok(Container::Mp4),
        "matroska" => ebml_container.ok_or_else(|| {
            MediaError::UnsupportedContainer("EBML without a WebM or Matroska document type".into())
        }),
        "flac" => Ok(Container::Flac),
        "ogg" => Ok(Container::Ogg),
        "aac" => Ok(Container::Adts),
        _ => Err(MediaError::UnsupportedContainer(name.to_owned())),
    }
}

fn map_codec(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
    declared_sample_rate: u32,
) -> Result<Codec, MediaError> {
    match params.codec {
        CODEC_ID_PCM_S16LE => Ok(Codec::PcmS16Le),
        CODEC_ID_PCM_S24LE => Ok(Codec::PcmS24Le),
        CODEC_ID_PCM_S32LE => Ok(Codec::PcmS32Le),
        CODEC_ID_MP3 => Ok(Codec::Mp3),
        CODEC_ID_OPUS => Ok(Codec::Opus),
        CODEC_ID_FLAC => Ok(Codec::Flac),
        CODEC_ID_VORBIS => Ok(Codec::Vorbis),
        CODEC_ID_AAC => classify_aac(params, declared_sample_rate),
        codec => Err(MediaError::UnsupportedCodec(codec.to_string())),
    }
}

fn classify_aac(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
    declared_sample_rate: u32,
) -> Result<Codec, MediaError> {
    let Some(extra_data) = params.extra_data.as_deref() else {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "missing AudioSpecificConfig",
        });
    };
    let config =
        parse_audio_specific_config(extra_data).ok_or(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "invalid AudioSpecificConfig",
        })?;
    if config.audio_object_type != 2 {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "unsupported audio object type",
        });
    }
    if !config.explicit_he && config.core_sample_rate == declared_sample_rate {
        return Ok(Codec::AacLc);
    }
    if config.initial_audio_object_type == 29 || config.channel_configuration == 1 {
        Ok(Codec::HeAacV2)
    } else {
        Ok(Codec::HeAacV1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioSpecificConfig {
    initial_audio_object_type: u8,
    audio_object_type: u8,
    core_sample_rate: u32,
    channel_configuration: u8,
    explicit_he: bool,
}

fn parse_audio_specific_config(bytes: &[u8]) -> Option<AudioSpecificConfig> {
    let mut bits = BitReader::new(bytes);
    let initial_object_type = read_audio_object_type(&mut bits)?;
    let initial_rate = read_sample_rate(&mut bits)?;
    let channel_configuration = u8::try_from(bits.read(4)?).ok()?;
    let explicit_he = matches!(initial_object_type, 5 | 29);
    let (audio_object_type, core_sample_rate) = if explicit_he {
        read_sample_rate(&mut bits)?;
        (read_audio_object_type(&mut bits)?, initial_rate)
    } else {
        (initial_object_type, initial_rate)
    };
    Some(AudioSpecificConfig {
        initial_audio_object_type: initial_object_type,
        audio_object_type,
        core_sample_rate,
        channel_configuration,
        explicit_he,
    })
}

fn create_xaac_decoder(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
    codec: Codec,
    limits: MediaLimits,
) -> Result<XaacDecoder, MediaError> {
    let config_bytes = params
        .extra_data
        .as_deref()
        .ok_or(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "missing AudioSpecificConfig",
        })?;
    if config_bytes.len() > limits.max_codec_config_bytes {
        return Err(MediaError::CodecConfigTooLarge {
            actual: config_bytes.len(),
            limit: limits.max_codec_config_bytes,
        });
    }
    let parsed =
        parse_audio_specific_config(config_bytes).ok_or(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "invalid AudioSpecificConfig",
        })?;
    let profile = match codec {
        Codec::AacLc => XaacProfile::AacLc,
        Codec::HeAacV1 => XaacProfile::HeAacV1,
        Codec::HeAacV2 => XaacProfile::HeAacV2,
        _ => {
            return Err(MediaError::UnsupportedCodec(
                "libxaac selected for a non-AAC codec".to_owned(),
            ));
        }
    };
    let max_pcm_bytes_per_frame =
        limits
            .max_pcm_samples_per_frame
            .checked_mul(2)
            .ok_or(MediaError::InvalidLimits(
                "max_pcm_samples_per_frame is too large",
            ))?;
    XaacDecoder::new(XaacConfig {
        audio_specific_config: config_bytes.into(),
        core_sample_rate: parsed.core_sample_rate,
        profile,
        max_access_unit_bytes: limits.max_packet_bytes,
        max_pcm_bytes_per_frame,
        max_native_memory_bytes: limits.max_native_decoder_bytes,
    })
    .map_err(|error| native_backend_error("decoder creation", &error))
}

fn read_audio_object_type(bits: &mut BitReader<'_>) -> Option<u8> {
    let value = u8::try_from(bits.read(5)?).ok()?;
    if value == 31 {
        Some(32_u8.checked_add(u8::try_from(bits.read(6)?).ok()?)?)
    } else {
        Some(value)
    }
}

fn read_sample_rate(bits: &mut BitReader<'_>) -> Option<u32> {
    const RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    let index = bits.read(4)? as usize;
    if index == 15 {
        bits.read(24)
    } else {
        RATES.get(index).copied()
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read(&mut self, count: usize) -> Option<u32> {
        if count > 32 || self.position.checked_add(count)? > self.bytes.len().checked_mul(8)? {
            return None;
        }
        let mut value = 0_u32;
        for _ in 0..count {
            let byte = self.bytes[self.position / 8];
            let bit = (byte >> (7 - self.position % 8)) & 1;
            value = (value << 1) | u32::from(bit);
            self.position += 1;
        }
        Some(value)
    }
}

fn timestamp_to_std(time_base: Option<TimeBase>, timestamp: Timestamp) -> Option<StdDuration> {
    let nanos = time_base?.calc_time(timestamp)?.as_nanos();
    u64::try_from(nanos.max(0))
        .ok()
        .map(StdDuration::from_nanos)
}

fn duration_to_std(
    time_base: Option<TimeBase>,
    duration: SymphoniaDuration,
) -> Option<StdDuration> {
    let timestamp = duration.timestamp_from(Timestamp::ZERO)?;
    timestamp_to_std(time_base, timestamp)
}

fn backend_error(operation: &'static str, error: &SymphoniaError) -> MediaError {
    MediaError::Backend {
        operation,
        message: error.to_string(),
    }
}

fn native_backend_error(operation: &'static str, error: &mantle_xaac::XaacError) -> MediaError {
    MediaError::Backend {
        operation,
        message: error.to_string(),
    }
}

fn xaac_decode_error(error: mantle_xaac::XaacError) -> MediaError {
    match error {
        mantle_xaac::XaacError::AccessUnitTooLarge { actual, limit } => {
            MediaError::PacketTooLarge { actual, limit }
        }
        mantle_xaac::XaacError::OutputTooLarge { actual, limit } => MediaError::PcmFrameTooLarge {
            actual: actual / 2,
            limit: limit / 2,
        },
        error => native_backend_error("decode", &error),
    }
}

fn prepare_pcm_output(
    output: &mut PcmFrame,
    sample_count: usize,
    sample_rate: u32,
    channels: u16,
    timestamp: Option<StdDuration>,
) -> Result<&mut [f32], MediaError> {
    let format = PcmFormat::new(sample_rate, channels).map_err(map_audio_frame_error)?;
    output
        .prepare(sample_count, format, timestamp)
        .map_err(map_audio_frame_error)
}

fn map_audio_frame_error(error: AudioFrameError) -> MediaError {
    match error {
        AudioFrameError::PcmCapacityExceeded { required, capacity } => {
            MediaError::OutputBufferTooSmall { required, capacity }
        }
        AudioFrameError::InvalidSampleRate { .. } | AudioFrameError::UnsupportedChannels { .. } => {
            MediaError::UnsupportedCodec(error.to_string())
        }
        AudioFrameError::MissingPcmFormat
        | AudioFrameError::MisalignedPcmSamples { .. }
        | AudioFrameError::SampleBufferTooSmall { .. }
        | AudioFrameError::FilterLimitExceeded { .. }
        | AudioFrameError::StreamingProcessorLimitExceeded { .. }
        | AudioFrameError::StreamingProcessorFormatUnsupported { .. }
        | AudioFrameError::StreamingProcessorRequiresPull
        | AudioFrameError::StreamingInputPending { .. }
        | AudioFrameError::StreamingInputAlreadyFinished
        | AudioFrameError::InvalidStreamingProcessorProgress { .. }
        | AudioFrameError::StreamingProcessorStalled
        | AudioFrameError::StreamingProcessorCapacityExceeded { .. }
        | AudioFrameError::PcmFormatMismatch { .. }
        | AudioFrameError::InvalidResamplerConfiguration(_)
        | AudioFrameError::UnsupportedResampleRatio { .. }
        | AudioFrameError::ResamplerInputLimitExceeded { .. }
        | AudioFrameError::ResamplerAlreadyFinished
        | AudioFrameError::ResamplerFailure
        | AudioFrameError::InvalidOpusPcmSamples { .. }
        | AudioFrameError::OpusEncodingFailure
        | AudioFrameError::OpusDecodingFailure
        | AudioFrameError::EncodedFrameTooLarge { .. } => MediaError::Backend {
            operation: "decode",
            message: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AudioSpecificConfig, parse_audio_specific_config};
    use mantle_audio::opus_packet_duration;

    #[test]
    fn parses_lc_and_explicit_he_audio_specific_configs() {
        assert_eq!(
            parse_audio_specific_config(&[0x11, 0x90]),
            Some(AudioSpecificConfig {
                initial_audio_object_type: 2,
                audio_object_type: 2,
                core_sample_rate: 48_000,
                channel_configuration: 2,
                explicit_he: false,
            })
        );
        assert_eq!(
            parse_audio_specific_config(&[0x2b, 0x92, 0x08]),
            Some(AudioSpecificConfig {
                initial_audio_object_type: 5,
                audio_object_type: 2,
                core_sample_rate: 22_050,
                channel_configuration: 2,
                explicit_he: true,
            })
        );
        assert_eq!(parse_audio_specific_config(&[0x11]), None);
    }

    #[test]
    fn derives_bounded_opus_packet_durations_from_the_toc() {
        assert_eq!(
            opus_packet_duration(&[16 << 3]),
            Some(Duration::from_micros(2_500))
        );
        assert_eq!(
            opus_packet_duration(&[(19 << 3) | 1]),
            Some(Duration::from_millis(40))
        );
        assert_eq!(
            opus_packet_duration(&[(19 << 3) | 3, 6]),
            Some(Duration::from_millis(120))
        );
        assert_eq!(opus_packet_duration(&[(19 << 3) | 3, 7]), None);
        assert_eq!(opus_packet_duration(&[]), None);
    }
}
