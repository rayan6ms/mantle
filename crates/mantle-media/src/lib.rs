//! Bounded, backend-independent media probing, decoding, packet extraction, and seeking.

use std::fmt;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration as StdDuration;

use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_MP3, CODEC_ID_OPUS, CODEC_ID_PCM_S16LE,
};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{
    MediaSource as SymphoniaMediaSource, MediaSourceStream, MediaSourceStreamOptions,
};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Duration as SymphoniaDuration, Time, TimeBase, Timestamp};

/// Bounds applied before and around the media backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaLimits {
    pub max_probe_bytes: u64,
    pub input_buffer_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_pcm_samples_per_frame: usize,
    pub max_consecutive_decode_errors: u32,
}

impl Default for MediaLimits {
    fn default() -> Self {
        Self {
            max_probe_bytes: 1024 * 1024,
            input_buffer_bytes: 64 * 1024,
            max_packet_bytes: 1024 * 1024,
            max_pcm_samples_per_frame: 256 * 1024,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Wave,
    Mp3,
    Mp4,
    WebM,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    PcmS16Le,
    Mp3,
    AacLc,
    Opus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaInfo {
    pub container: Container,
    pub codec: Codec,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: Option<StdDuration>,
    pub seekable: bool,
}

/// Caller-owned reusable decoded PCM storage.
#[derive(Debug)]
pub struct PcmFrame {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
    timestamp: Option<StdDuration>,
}

impl PcmFrame {
    #[must_use]
    pub fn with_capacity(sample_capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(sample_capacity),
            sample_rate: 0,
            channels: 0,
            timestamp: None,
        }
    }

    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    #[must_use]
    pub fn timestamp(&self) -> Option<StdDuration> {
        self.timestamp
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
    InvalidLimits(&'static str),
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
            Self::InvalidLimits(message) => write!(formatter, "invalid media limits: {message}"),
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
    decoder: Option<Box<dyn AudioDecoder>>,
    info: MediaInfo,
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
        let path = path.as_ref();
        let file = File::open(path)?;
        let extension = path.extension().and_then(|value| value.to_str());
        Self::open(Box::new(file), extension, limits)
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
        let limits = limits.validate()?;
        let seekable = input.is_seekable();
        let probe_state = Arc::new(ProbeState::new(limits.max_probe_bytes));
        let source = Box::new(InputAdapter {
            input,
            probe_state: Arc::clone(&probe_state),
        });
        let stream = MediaSourceStream::new(
            source,
            MediaSourceStreamOptions {
                buffer_len: limits.input_buffer_bytes,
            },
        );
        let mut hint = Hint::new();
        if let Some(extension) = extension_hint {
            hint.with_extension(extension);
        }
        let probed = symphonia::default::get_probe().probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        );
        probe_state.active.store(false, Ordering::Release);
        let format = match probed {
            Ok(format) => format,
            Err(_) if probe_state.exceeded.load(Ordering::Acquire) => {
                return Err(MediaError::ProbeLimitExceeded {
                    limit: limits.max_probe_bytes,
                });
            }
            Err(error) => return Err(backend_error("probe", &error)),
        };

        let container = map_container(format.format_info().short_name)?;
        let media_duration = duration_to_std(
            format.media_info().time_base,
            format
                .media_info()
                .duration
                .unwrap_or(SymphoniaDuration::ZERO),
        )
        .filter(|duration| !duration.is_zero());
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(MediaError::NoAudioTrack)?;
        let track_id = track.id;
        let time_base = track.time_base;
        let duration = track
            .duration
            .and_then(|duration| duration_to_std(time_base, duration))
            .or(media_duration);
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or(MediaError::NoAudioTrack)?
            .clone();
        let codec = map_codec(&params)?;
        let sample_rate = params.sample_rate.ok_or_else(|| {
            MediaError::UnsupportedCodec(format!("{} without a sample rate", params.codec))
        })?;
        let channels = params
            .channels
            .as_ref()
            .map(symphonia::core::audio::Channels::count)
            .and_then(|channels| u16::try_from(channels).ok())
            .ok_or_else(|| {
                MediaError::UnsupportedCodec(format!(
                    "{} without a supported channel count",
                    params.codec
                ))
            })?;
        let decoder = if codec == Codec::Opus {
            None
        } else {
            Some(
                symphonia::default::get_codecs()
                    .make_audio_decoder(&params, &AudioDecoderOptions::default())
                    .map_err(|error| backend_error("decoder creation", &error))?,
            )
        };

        Ok(Self {
            format,
            decoder,
            info: MediaInfo {
                container,
                codec,
                sample_rate,
                channels,
                duration,
                seekable,
            },
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
        if self.info.codec == Codec::Opus {
            return Err(MediaError::WrongOutputKind {
                codec: self.info.codec,
            });
        }
        loop {
            let Some(packet) = self.next_audio_packet()? else {
                output.samples.clear();
                return Ok(false);
            };
            let timestamp = timestamp_to_std(self.time_base, packet.pts);
            let Some(audio_decoder) = self.decoder.as_mut() else {
                return Err(MediaError::WrongOutputKind {
                    codec: self.info.codec,
                });
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
            self.consecutive_decode_errors = 0;
            let sample_count = decoded_audio.samples_interleaved();
            if sample_count > self.limits.max_pcm_samples_per_frame {
                return Err(MediaError::PcmFrameTooLarge {
                    actual: sample_count,
                    limit: self.limits.max_pcm_samples_per_frame,
                });
            }
            if sample_count > output.samples.capacity() {
                return Err(MediaError::OutputBufferTooSmall {
                    required: sample_count,
                    capacity: output.samples.capacity(),
                });
            }
            let channels =
                u16::try_from(decoded_audio.spec().channels().count()).map_err(|_| {
                    MediaError::UnsupportedCodec("decoded channel count exceeds u16".to_owned())
                })?;
            output.samples.resize(sample_count, 0.0);
            decoded_audio.copy_to_slice_interleaved(&mut output.samples);
            output.sample_rate = decoded_audio.spec().rate();
            output.channels = channels;
            output.timestamp = timestamp;
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
            .map_err(|error| backend_error("seek", &error))?;
        if let Some(decoder) = self.decoder.as_mut() {
            decoder.reset();
        }
        self.consecutive_decode_errors = 0;
        Ok(SeekResult {
            requested,
            actual: timestamp_to_std(self.time_base, result.actual_ts),
        })
    }

    fn next_audio_packet(&mut self) -> Result<Option<symphonia::core::packet::Packet>, MediaError> {
        loop {
            let packet = self
                .format
                .next_packet()
                .map_err(|error| backend_error("demux", &error))?;
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
}

impl Read for InputAdapter {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.probe_state.active.load(Ordering::Acquire) {
            return self.input.read(buffer);
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
        self.probe_state
            .bytes_read
            .fetch_add(count as u64, Ordering::Relaxed);
        Ok(count)
    }
}

impl Seek for InputAdapter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.input.seek(position)
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

fn map_container(name: &str) -> Result<Container, MediaError> {
    match name {
        "wave" => Ok(Container::Wave),
        "mp3" => Ok(Container::Mp3),
        "isomp4" => Ok(Container::Mp4),
        "matroska" => Ok(Container::WebM),
        _ => Err(MediaError::UnsupportedContainer(name.to_owned())),
    }
}

fn map_codec(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
) -> Result<Codec, MediaError> {
    match params.codec {
        CODEC_ID_PCM_S16LE => Ok(Codec::PcmS16Le),
        CODEC_ID_MP3 => Ok(Codec::Mp3),
        CODEC_ID_OPUS => Ok(Codec::Opus),
        CODEC_ID_AAC => {
            validate_aac_lc(params)?;
            Ok(Codec::AacLc)
        }
        codec => Err(MediaError::UnsupportedCodec(codec.to_string())),
    }
}

fn validate_aac_lc(
    params: &symphonia::core::codecs::audio::AudioCodecParameters,
) -> Result<(), MediaError> {
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
    let declared_rate = params.sample_rate.unwrap_or(config.sample_rate);
    if config.audio_object_type != 2 || config.explicit_he || config.sample_rate != declared_rate {
        return Err(MediaError::UnsupportedCodecProfile {
            codec: "AAC",
            profile: "HE-AAC/SBR/PS",
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioSpecificConfig {
    audio_object_type: u8,
    sample_rate: u32,
    explicit_he: bool,
}

fn parse_audio_specific_config(bytes: &[u8]) -> Option<AudioSpecificConfig> {
    let mut bits = BitReader::new(bytes);
    let initial_object_type = read_audio_object_type(&mut bits)?;
    let initial_rate = read_sample_rate(&mut bits)?;
    bits.read(4)?;
    let explicit_he = matches!(initial_object_type, 5 | 29);
    let (audio_object_type, sample_rate) = if explicit_he {
        let extension_rate = read_sample_rate(&mut bits)?;
        (read_audio_object_type(&mut bits)?, extension_rate)
    } else {
        (initial_object_type, initial_rate)
    };
    Some(AudioSpecificConfig {
        audio_object_type,
        sample_rate,
        explicit_he,
    })
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
    u64::try_from(nanos).ok().map(StdDuration::from_nanos)
}

fn duration_to_std(
    time_base: Option<TimeBase>,
    duration: SymphoniaDuration,
) -> Option<StdDuration> {
    let timestamp = duration.timestamp_from(Timestamp::ZERO)?;
    timestamp_to_std(time_base, timestamp)
}

fn opus_packet_duration(packet: &[u8]) -> Option<StdDuration> {
    let toc = *packet.first()?;
    let config = toc >> 3;
    let frame_count = match toc & 0b11 {
        0 => 1_u64,
        1 | 2 => 2,
        3 => u64::from(*packet.get(1)? & 0x3f),
        _ => unreachable!(),
    };
    if frame_count == 0 {
        return None;
    }
    let frame_micros = match config {
        0..=11 => [10_000_u64, 20_000, 40_000, 60_000][usize::from(config & 0b11)],
        12..=15 => [10_000_u64, 20_000][usize::from(config & 0b1)],
        16..=31 => [2_500_u64, 5_000, 10_000, 20_000][usize::from(config & 0b11)],
        _ => unreachable!(),
    };
    let total_micros = frame_micros.checked_mul(frame_count)?;
    (total_micros <= 120_000).then(|| StdDuration::from_micros(total_micros))
}

fn backend_error(operation: &'static str, error: &SymphoniaError) -> MediaError {
    MediaError::Backend {
        operation,
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AudioSpecificConfig, opus_packet_duration, parse_audio_specific_config};

    #[test]
    fn parses_lc_and_explicit_he_audio_specific_configs() {
        assert_eq!(
            parse_audio_specific_config(&[0x11, 0x90]),
            Some(AudioSpecificConfig {
                audio_object_type: 2,
                sample_rate: 48_000,
                explicit_he: false,
            })
        );
        assert_eq!(
            parse_audio_specific_config(&[0x2b, 0x92, 0x08]),
            Some(AudioSpecificConfig {
                audio_object_type: 2,
                sample_rate: 44_100,
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
