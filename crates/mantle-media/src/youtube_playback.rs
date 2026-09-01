use std::fmt;
use std::time::Duration;

use mantle_audio::{
    AudioFrameError, COMPATIBLE_CHANNELS, COMPATIBLE_FRAME_DURATION, COMPATIBLE_PCM_SAMPLES,
    COMPATIBLE_SAMPLE_RATE, EncodedFrameSlot, MAX_RESAMPLER_BUFFERED_FRAMES, OpusEncodingQuality,
    OpusPassthrough, PcmFormat, PcmFrame, PcmOpusEncoder, PcmResampler, ResamplingQuality,
    VolumeLevel,
};

use crate::{
    Codec, Container, EncodedPacket, HlsError, HlsLimits, HlsLiveLimits, HlsLivePoll,
    HlsLiveSequence, HlsPlaylist, HttpPlaylistOptions, HttpRangeInput, HttpRangeOptions,
    HttpStreamOptions, MediaCancellation, MediaError, MediaInfo, MediaInput, MediaLimits,
    MediaSession, MemoryInput, MpegTsError, MpegTsLimits, PlaylistError, YoutubeAudioSourceManager,
    YoutubeError, YoutubeErrorKind, YoutubePlaybackFormatKind, YoutubePlaybackFormats,
    extract_mpeg_ts_adts, load_http_hls_playlist_with_cancellation,
    load_http_hls_segment_with_cancellation,
};

const TRANSCODE_INPUT_CHUNK_FRAMES: usize = 1_024;

/// The active output path for one finite `YouTube` media object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YoutubePlaybackMode {
    OpusPassthrough,
    Transcode,
}

/// Bounded HTTP, parser, segment, and decoder policy for one live HLS session.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct YoutubeLivePlaybackOptions {
    pub playlist: HttpPlaylistOptions,
    pub segment: HttpStreamOptions,
    pub hls: HlsLimits,
    pub live: HlsLiveLimits,
    pub mpeg_ts: MpegTsLimits,
    pub media: MediaLimits,
}

/// One deterministic result from polling a live `YouTube` HLS session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YoutubeLivePlaybackPoll {
    Frame,
    WaitUntil(Duration),
    Ended,
    Exhausted,
}

/// Stable, credential-safe failure classes for `YouTube` media handoff and frame production.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YoutubePlaybackErrorKind {
    Source(YoutubeErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
    AudioPipeline,
}

/// A `YouTube` playback failure that never retains a signed media URL or response body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YoutubePlaybackError {
    kind: YoutubePlaybackErrorKind,
}

impl YoutubePlaybackError {
    const fn new(kind: YoutubePlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> YoutubePlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for YoutubePlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            YoutubePlaybackErrorKind::Source(_) => "YouTube playback discovery failed",
            YoutubePlaybackErrorKind::InvalidOptions => "invalid YouTube media policy",
            YoutubePlaybackErrorKind::Cancelled => "YouTube media playback cancelled",
            YoutubePlaybackErrorKind::Network => "YouTube media request failed",
            YoutubePlaybackErrorKind::InvalidMedia => "YouTube returned invalid media",
            YoutubePlaybackErrorKind::IncompatibleFormat => {
                "YouTube media does not match the selected format"
            }
            YoutubePlaybackErrorKind::AudioPipeline => "YouTube audio processing failed",
        })
    }
}

impl std::error::Error for YoutubePlaybackError {}

/// A finite selected `YouTube` media object connected to Mantle's fixed Opus frame contract.
pub struct YoutubePlaybackSession {
    inner: YoutubePlaybackInner,
}

/// A bounded live HLS session driven by caller-supplied monotonic time.
pub struct YoutubeLivePlaybackSession {
    manifest_url: String,
    media_playlist_url: Option<String>,
    options: YoutubeLivePlaybackOptions,
    cancellation: MediaCancellation,
    sequence: HlsLiveSequence,
    transcoder: Option<PcmTranscoder>,
    terminal_after_drain: Option<YoutubeLivePlaybackPoll>,
    terminal: Option<YoutubeLivePlaybackPoll>,
}

impl fmt::Debug for YoutubeLivePlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeLivePlaybackSession")
            .field("manifest_url", &"<redacted>")
            .field("media_playlist", &self.media_playlist_url.is_some())
            .field("mode", &self.mode())
            .field("terminal", &self.terminal)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for YoutubePlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubePlaybackSession")
            .field("mode", &self.mode())
            .field("media", self.info())
            .finish()
    }
}

enum YoutubePlaybackInner {
    Opus(Box<OpusPlayback>),
    Transcode(Box<PcmTranscoder>),
}

struct OpusPlayback {
    session: MediaSession,
    packet: EncodedPacket,
    passthrough: OpusPassthrough,
}

struct PcmTranscoder {
    session: Option<MediaSession>,
    info: MediaInfo,
    source_format: PcmFormat,
    decoded: PcmFrame,
    decoded_offset: usize,
    resampler: Option<PcmResampler>,
    resampled: PcmFrame,
    resampled_offset: usize,
    assembled: [f32; COMPATIBLE_PCM_SAMPLES],
    assembled_len: usize,
    encoder_input: PcmFrame,
    encoder: PcmOpusEncoder,
    input_eof: bool,
    timestamp_initialized: bool,
    base_timestamp: Option<Duration>,
    frames_encoded: u64,
}

enum PcmTranscodePoll {
    Frame,
    NeedInput,
    Ended,
}

impl YoutubeAudioSourceManager {
    /// Resolves and opens the selected finite media object through bounded HTTP range input.
    ///
    /// Live formats without a content length are rejected here and belong to the separate HLS
    /// playback path. The advertised content length and container/codec must match the fetched
    /// object before any output is produced.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe source, policy, cancellation, network, or media error.
    pub fn open_selected_playback(
        &self,
        formats: &YoutubePlaybackFormats,
        range_options: HttpRangeOptions,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<YoutubePlaybackSession, YoutubePlaybackError> {
        let selected = formats.selected();
        let kind = selected.kind().ok_or_else(|| {
            YoutubePlaybackError::new(YoutubePlaybackErrorKind::IncompatibleFormat)
        })?;
        let content_length = selected.content_length().ok_or_else(|| {
            YoutubePlaybackError::new(YoutubePlaybackErrorKind::IncompatibleFormat)
        })?;
        if content_length == 0 || content_length > range_options.max_source_bytes {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::InvalidOptions,
            ));
        }
        let resolved = self
            .resolve_selected_playback_url(formats, &cancellation)
            .map_err(map_source_error)?;
        let input = HttpRangeInput::open_with_cancellation(
            resolved.as_str(),
            range_options,
            cancellation.clone(),
        )
        .map_err(map_media_error)?;
        if input.byte_len() != Some(content_length) {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::InvalidMedia,
            ));
        }
        let session = MediaSession::open_with_cancellation(
            Box::new(input),
            Some(extension_hint(kind)),
            media_limits,
            cancellation,
        )
        .map_err(map_media_error)?;
        YoutubePlaybackSession::new(session, kind)
    }

    /// Resolves and opens the selected live HLS manifest without performing an eager request.
    ///
    /// Playlist reloads remain caller-clock-driven through
    /// [`YoutubeLivePlaybackSession::poll_frame`]. Every playlist and segment request inherits the
    /// supplied HTTP ceilings, destination policy, timeouts, retries, and cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe policy, source, cancellation, or incompatible-format error.
    pub fn open_selected_live_playback(
        &self,
        formats: &YoutubePlaybackFormats,
        options: YoutubeLivePlaybackOptions,
        cancellation: MediaCancellation,
    ) -> Result<YoutubeLivePlaybackSession, YoutubePlaybackError> {
        cancellation.check().map_err(map_media_error)?;
        let selected = formats.selected();
        if selected.kind() != Some(YoutubePlaybackFormatKind::HlsMpegTsAac)
            || selected.content_length().is_some()
        {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::IncompatibleFormat,
            ));
        }
        let resolved = self
            .resolve_selected_playback_url(formats, &cancellation)
            .map_err(map_source_error)?;
        YoutubeLivePlaybackSession::open_hls_manifest(
            resolved.as_str().to_owned(),
            options,
            cancellation,
        )
    }
}

impl YoutubeLivePlaybackSession {
    #[allow(clippy::large_types_passed_by_value)]
    pub(crate) fn open_hls_manifest(
        manifest_url: String,
        options: YoutubeLivePlaybackOptions,
        cancellation: MediaCancellation,
    ) -> Result<Self, YoutubePlaybackError> {
        cancellation.check().map_err(map_media_error)?;
        validate_live_options(&options)?;
        let sequence = HlsLiveSequence::new(options.live).map_err(map_hls_error)?;
        Ok(Self {
            manifest_url,
            media_playlist_url: None,
            options,
            cancellation,
            sequence,
            transcoder: None,
            terminal_after_drain: None,
            terminal: None,
        })
    }

    #[must_use]
    pub const fn mode(&self) -> YoutubePlaybackMode {
        YoutubePlaybackMode::Transcode
    }

    /// Produces one frame or a deterministic reload/terminal outcome at monotonic `now`.
    ///
    /// Calls made before a returned [`YoutubeLivePlaybackPoll::WaitUntil`] deadline do not perform
    /// an HTTP request. Decoded PCM and resampler state are retained across MPEG-TS segments, so
    /// output timestamps remain continuous and a partial PCM block is padded only at terminal EOF.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe cancellation, network, playlist, MPEG-TS, media, or audio error.
    pub fn poll_frame(
        &mut self,
        now: Duration,
        output: &mut EncodedFrameSlot,
    ) -> Result<YoutubeLivePlaybackPoll, YoutubePlaybackError> {
        self.cancellation.check().map_err(map_media_error)?;
        if let Some(terminal) = self.terminal {
            output.clear();
            return Ok(terminal);
        }
        loop {
            if let Some(terminal) = self.terminal {
                output.clear();
                return Ok(terminal);
            }
            if let Some(transcoder) = self.transcoder.as_mut() {
                match transcoder.poll_continuous_frame(output)? {
                    PcmTranscodePoll::Frame => return Ok(YoutubeLivePlaybackPoll::Frame),
                    PcmTranscodePoll::NeedInput => {}
                    PcmTranscodePoll::Ended => {
                        let terminal = self
                            .terminal_after_drain
                            .take()
                            .unwrap_or(YoutubeLivePlaybackPoll::Exhausted);
                        self.terminal = Some(terminal);
                        output.clear();
                        return Ok(terminal);
                    }
                }
            }

            match self.poll_next_segment(now)? {
                HlsLivePoll::Segment(segment) => {
                    let transport = load_http_hls_segment_with_cancellation(
                        &segment,
                        self.options.segment,
                        self.cancellation.clone(),
                    )
                    .map_err(map_media_error)?;
                    self.cancellation.check().map_err(map_media_error)?;
                    let elementary = extract_mpeg_ts_adts(&transport, self.options.mpeg_ts)
                        .map_err(|error| map_mpeg_ts_error(&error))?;
                    let session = MediaSession::open_with_cancellation(
                        Box::new(MemoryInput::new(elementary.into_adts_bytes())),
                        Some("aac"),
                        self.options.media,
                        self.cancellation.clone(),
                    )
                    .map_err(map_media_error)?;
                    if let Some(transcoder) = self.transcoder.as_mut() {
                        transcoder.replace_input(session)?;
                    } else {
                        self.transcoder = Some(PcmTranscoder::new_continuous(session)?);
                    }
                }
                HlsLivePoll::WaitUntil(deadline) => {
                    output.clear();
                    return Ok(YoutubeLivePlaybackPoll::WaitUntil(deadline));
                }
                HlsLivePoll::Ended => {
                    self.begin_terminal_drain(YoutubeLivePlaybackPoll::Ended)?;
                }
                HlsLivePoll::Exhausted => {
                    self.begin_terminal_drain(YoutubeLivePlaybackPoll::Exhausted)?;
                }
            }
        }
    }

    fn poll_next_segment(&mut self, now: Duration) -> Result<HlsLivePoll, YoutubePlaybackError> {
        if let Some(url) = self.media_playlist_url.as_deref() {
            return self
                .sequence
                .poll_http_with_cancellation(
                    url,
                    self.options.playlist,
                    self.options.hls,
                    now,
                    self.cancellation.clone(),
                )
                .map_err(map_hls_error);
        }

        let playlist = load_http_hls_playlist_with_cancellation(
            &self.manifest_url,
            self.options.playlist,
            self.options.hls,
            self.cancellation.clone(),
        )
        .map_err(map_hls_error)?;
        match playlist {
            HlsPlaylist::Master(master) => {
                let url = master
                    .selected_variant()
                    .ok_or_else(|| {
                        YoutubePlaybackError::new(YoutubePlaybackErrorKind::InvalidMedia)
                    })?
                    .uri
                    .clone();
                self.media_playlist_url = Some(url);
                self.poll_next_segment(now)
            }
            HlsPlaylist::Media(media) => {
                self.media_playlist_url = Some(self.manifest_url.clone());
                self.sequence.poll(&media, now).map_err(map_hls_error)
            }
        }
    }

    fn begin_terminal_drain(
        &mut self,
        terminal: YoutubeLivePlaybackPoll,
    ) -> Result<(), YoutubePlaybackError> {
        if let Some(transcoder) = self.transcoder.as_mut() {
            transcoder.finish_input()?;
            self.terminal_after_drain = Some(terminal);
        } else {
            self.terminal = Some(terminal);
        }
        Ok(())
    }
}

impl YoutubePlaybackSession {
    fn new(
        session: MediaSession,
        expected: YoutubePlaybackFormatKind,
    ) -> Result<Self, YoutubePlaybackError> {
        validate_media_kind(session.info(), expected)?;
        let inner = if session.info().codec == Codec::Opus {
            if session.info().sample_rate != COMPATIBLE_SAMPLE_RATE
                || session.info().channels != COMPATIBLE_CHANNELS
            {
                return Err(YoutubePlaybackError::new(
                    YoutubePlaybackErrorKind::IncompatibleFormat,
                ));
            }
            let format = PcmFormat::new(session.info().sample_rate, session.info().channels)
                .map_err(map_audio_error)?;
            let packet = EncodedPacket::with_capacity(session.limits().max_packet_bytes);
            YoutubePlaybackInner::Opus(Box::new(OpusPlayback {
                session,
                packet,
                passthrough: OpusPassthrough::new(format),
            }))
        } else {
            YoutubePlaybackInner::Transcode(Box::new(PcmTranscoder::new(session)?))
        };
        Ok(Self { inner })
    }

    #[must_use]
    pub const fn mode(&self) -> YoutubePlaybackMode {
        match self.inner {
            YoutubePlaybackInner::Opus(_) => YoutubePlaybackMode::OpusPassthrough,
            YoutubePlaybackInner::Transcode(_) => YoutubePlaybackMode::Transcode,
        }
    }

    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        match &self.inner {
            YoutubePlaybackInner::Opus(playback) => playback.session.info(),
            YoutubePlaybackInner::Transcode(transcoder) => &transcoder.info,
        }
    }

    /// Produces the next Discord-compatible 20 ms Opus frame in caller-owned inline storage.
    ///
    /// Compatible stereo 48 kHz Opus packets are copied directly. Other selected codecs are
    /// decoded, channel-mapped, resampled when needed, assembled into 960-sample blocks, and
    /// encoded with the normal Mantle Opus encoder. A final partial PCM block is zero-padded once.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe cancellation, media, compatibility, or audio-pipeline error.
    pub fn read_frame(
        &mut self,
        output: &mut EncodedFrameSlot,
    ) -> Result<bool, YoutubePlaybackError> {
        match &mut self.inner {
            YoutubePlaybackInner::Opus(playback) => {
                if !playback
                    .session
                    .read_encoded(&mut playback.packet)
                    .map_err(map_media_error)?
                {
                    output.clear();
                    return Ok(false);
                }
                let route = playback
                    .passthrough
                    .route_packet(playback.packet.data(), playback.packet.timestamp(), output)
                    .map_err(map_audio_error)?;
                if !route.delivered() {
                    return Err(YoutubePlaybackError::new(
                        YoutubePlaybackErrorKind::IncompatibleFormat,
                    ));
                }
                Ok(true)
            }
            YoutubePlaybackInner::Transcode(transcoder) => transcoder.read_frame(output),
        }
    }
}

impl PcmTranscoder {
    fn new(session: MediaSession) -> Result<Self, YoutubePlaybackError> {
        Self::new_with_base_timestamp(session, None)
    }

    fn new_continuous(session: MediaSession) -> Result<Self, YoutubePlaybackError> {
        Self::new_with_base_timestamp(session, Some(Duration::ZERO))
    }

    fn new_with_base_timestamp(
        session: MediaSession,
        base_timestamp: Option<Duration>,
    ) -> Result<Self, YoutubePlaybackError> {
        let source_format = PcmFormat::new(session.info().sample_rate, session.info().channels)
            .map_err(map_audio_error)?;
        let resampler = (source_format.sample_rate() != COMPATIBLE_SAMPLE_RATE)
            .then(|| {
                PcmResampler::new(
                    source_format,
                    COMPATIBLE_SAMPLE_RATE,
                    ResamplingQuality::Medium,
                    TRANSCODE_INPUT_CHUNK_FRAMES,
                    mantle_audio::COMPATIBLE_SAMPLES_PER_CHANNEL,
                    MAX_RESAMPLER_BUFFERED_FRAMES,
                )
            })
            .transpose()
            .map_err(map_audio_error)?;
        let decoded_capacity = session.limits().max_pcm_samples_per_frame;
        Ok(Self {
            info: session.info().clone(),
            session: Some(session),
            source_format,
            decoded: PcmFrame::with_capacity(decoded_capacity),
            decoded_offset: 0,
            resampler,
            resampled: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            resampled_offset: 0,
            assembled: [0.0; COMPATIBLE_PCM_SAMPLES],
            assembled_len: 0,
            encoder_input: PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES),
            encoder: PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).map_err(map_audio_error)?,
            input_eof: false,
            timestamp_initialized: base_timestamp.is_some(),
            base_timestamp,
            frames_encoded: 0,
        })
    }

    fn read_frame(&mut self, output: &mut EncodedFrameSlot) -> Result<bool, YoutubePlaybackError> {
        match self.poll_frame(output, true)? {
            PcmTranscodePoll::Frame => Ok(true),
            PcmTranscodePoll::Ended => Ok(false),
            PcmTranscodePoll::NeedInput => unreachable!("finite input finalizes at EOF"),
        }
    }

    fn poll_continuous_frame(
        &mut self,
        output: &mut EncodedFrameSlot,
    ) -> Result<PcmTranscodePoll, YoutubePlaybackError> {
        self.poll_frame(output, false)
    }

    fn poll_frame(
        &mut self,
        output: &mut EncodedFrameSlot,
        finish_at_eof: bool,
    ) -> Result<PcmTranscodePoll, YoutubePlaybackError> {
        while self.assembled_len < COMPATIBLE_PCM_SAMPLES {
            if self.resampler.is_some() {
                if self.append_resampled()? {
                    continue;
                }
                if self.input_eof {
                    break;
                }
                let Some(session) = self.session.as_mut() else {
                    output.clear();
                    return Ok(PcmTranscodePoll::NeedInput);
                };
                if !session
                    .read_pcm(&mut self.decoded)
                    .map_err(map_media_error)?
                {
                    self.session = None;
                    if finish_at_eof {
                        self.finish_input()?;
                    } else {
                        output.clear();
                        return Ok(PcmTranscodePoll::NeedInput);
                    }
                    continue;
                }
                self.validate_decoded_format()?;
                self.resampler
                    .as_mut()
                    .expect("resampler path")
                    .push(&self.decoded)
                    .map_err(map_audio_error)?;
            } else if !self.append_decoded()? {
                if self.input_eof {
                    break;
                }
                let Some(session) = self.session.as_mut() else {
                    output.clear();
                    return Ok(PcmTranscodePoll::NeedInput);
                };
                if session
                    .read_pcm(&mut self.decoded)
                    .map_err(map_media_error)?
                {
                    self.validate_decoded_format()?;
                    self.decoded_offset = 0;
                } else {
                    self.session = None;
                    if finish_at_eof {
                        self.finish_input()?;
                    } else {
                        output.clear();
                        return Ok(PcmTranscodePoll::NeedInput);
                    }
                }
            }
        }

        if self.assembled_len == 0 {
            output.clear();
            return Ok(PcmTranscodePoll::Ended);
        }
        self.assembled[self.assembled_len..].fill(0.0);
        let timestamp = self.base_timestamp.map(|base| {
            let nanos = self.frames_encoded.saturating_mul(
                u64::try_from(COMPATIBLE_FRAME_DURATION.as_nanos()).unwrap_or(u64::MAX),
            );
            base.saturating_add(Duration::from_nanos(nanos))
        });
        let output_format =
            PcmFormat::new(COMPATIBLE_SAMPLE_RATE, COMPATIBLE_CHANNELS).map_err(map_audio_error)?;
        self.encoder_input
            .copy_from_interleaved(&self.assembled, output_format, timestamp)
            .map_err(map_audio_error)?;
        self.encoder
            .encode(&self.encoder_input, output, VolumeLevel::NORMAL)
            .map_err(map_audio_error)?;
        self.assembled_len = 0;
        self.frames_encoded = self.frames_encoded.saturating_add(1);
        Ok(PcmTranscodePoll::Frame)
    }

    fn replace_input(&mut self, session: MediaSession) -> Result<(), YoutubePlaybackError> {
        if self.input_eof || self.session.is_some() {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::AudioPipeline,
            ));
        }
        let format = PcmFormat::new(session.info().sample_rate, session.info().channels)
            .map_err(map_audio_error)?;
        if format != self.source_format {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::IncompatibleFormat,
            ));
        }
        self.session = Some(session);
        Ok(())
    }

    fn finish_input(&mut self) -> Result<(), YoutubePlaybackError> {
        if self.input_eof {
            return Ok(());
        }
        self.session = None;
        self.input_eof = true;
        if let Some(resampler) = self.resampler.as_mut() {
            resampler.finish().map_err(map_audio_error)?;
        }
        Ok(())
    }

    fn append_decoded(&mut self) -> Result<bool, YoutubePlaybackError> {
        if self.decoded_offset >= self.decoded.samples().len() {
            return Ok(false);
        }
        if !self.timestamp_initialized {
            self.base_timestamp = Some(self.decoded.timestamp().unwrap_or_default());
            self.timestamp_initialized = true;
        }
        append_interleaved(
            self.decoded.samples(),
            self.source_format.channels(),
            &mut self.decoded_offset,
            &mut self.assembled,
            &mut self.assembled_len,
        )?;
        Ok(true)
    }

    fn append_resampled(&mut self) -> Result<bool, YoutubePlaybackError> {
        if self.resampled_offset >= self.resampled.samples().len() {
            if !self
                .resampler
                .as_mut()
                .expect("resampler path")
                .read(&mut self.resampled)
                .map_err(map_audio_error)?
            {
                return Ok(false);
            }
            self.resampled_offset = 0;
        }
        if !self.timestamp_initialized {
            self.base_timestamp = Some(self.resampled.timestamp().unwrap_or_default());
            self.timestamp_initialized = true;
        }
        append_interleaved(
            self.resampled.samples(),
            self.source_format.channels(),
            &mut self.resampled_offset,
            &mut self.assembled,
            &mut self.assembled_len,
        )?;
        Ok(true)
    }

    fn validate_decoded_format(&self) -> Result<(), YoutubePlaybackError> {
        if self.decoded.format() != Some(self.source_format) {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(())
    }
}

fn append_interleaved(
    input: &[f32],
    channels: u16,
    input_offset: &mut usize,
    output: &mut [f32; COMPATIBLE_PCM_SAMPLES],
    output_len: &mut usize,
) -> Result<(), YoutubePlaybackError> {
    let channels = usize::from(channels);
    if channels == 0
        || *input_offset > input.len()
        || !input.len().is_multiple_of(channels)
        || !input_offset.is_multiple_of(channels)
    {
        return Err(YoutubePlaybackError::new(
            YoutubePlaybackErrorKind::IncompatibleFormat,
        ));
    }
    let available_frames = (input.len() - *input_offset) / channels;
    let output_frames = (COMPATIBLE_PCM_SAMPLES - *output_len) / usize::from(COMPATIBLE_CHANNELS);
    let frames = available_frames.min(output_frames);
    match channels {
        1 => {
            for sample in &input[*input_offset..*input_offset + frames] {
                output[*output_len] = *sample;
                output[*output_len + 1] = *sample;
                *output_len += 2;
            }
            *input_offset += frames;
        }
        2 => {
            let samples = frames * 2;
            output[*output_len..*output_len + samples]
                .copy_from_slice(&input[*input_offset..*input_offset + samples]);
            *output_len += samples;
            *input_offset += samples;
        }
        _ => {
            return Err(YoutubePlaybackError::new(
                YoutubePlaybackErrorKind::IncompatibleFormat,
            ));
        }
    }
    Ok(())
}

const fn extension_hint(kind: YoutubePlaybackFormatKind) -> &'static str {
    match kind {
        YoutubePlaybackFormatKind::HlsMpegTsAac => "aac",
        YoutubePlaybackFormatKind::WebmOpus
        | YoutubePlaybackFormatKind::WebmVorbis
        | YoutubePlaybackFormatKind::WebmVideoVorbis => "webm",
        YoutubePlaybackFormatKind::Mp4AacLc | YoutubePlaybackFormatKind::Mp4VideoAacLc => "m4a",
    }
}

fn validate_media_kind(
    info: &MediaInfo,
    expected: YoutubePlaybackFormatKind,
) -> Result<(), YoutubePlaybackError> {
    let matches = match expected {
        YoutubePlaybackFormatKind::HlsMpegTsAac => false,
        YoutubePlaybackFormatKind::WebmOpus => {
            info.container == Container::WebM && info.codec == Codec::Opus
        }
        YoutubePlaybackFormatKind::WebmVorbis | YoutubePlaybackFormatKind::WebmVideoVorbis => {
            info.container == Container::WebM && info.codec == Codec::Vorbis
        }
        YoutubePlaybackFormatKind::Mp4AacLc | YoutubePlaybackFormatKind::Mp4VideoAacLc => {
            info.container == Container::Mp4 && info.codec == Codec::AacLc
        }
    };
    if !matches {
        return Err(YoutubePlaybackError::new(
            YoutubePlaybackErrorKind::IncompatibleFormat,
        ));
    }
    Ok(())
}

fn validate_live_options(options: &YoutubeLivePlaybackOptions) -> Result<(), YoutubePlaybackError> {
    options.playlist.http.validate().map_err(map_media_error)?;
    options
        .playlist
        .playlist
        .validate()
        .map_err(|error| map_playlist_error(&error))?;
    options.segment.validate().map_err(map_media_error)?;
    options.hls.validate().map_err(map_hls_error)?;
    options.live.validate().map_err(map_hls_error)?;
    options
        .mpeg_ts
        .validate()
        .map_err(|error| map_mpeg_ts_error(&error))?;
    options.media.validate().map_err(map_media_error)?;
    Ok(())
}

const fn map_playlist_error(error: &PlaylistError) -> YoutubePlaybackError {
    let kind = match error {
        PlaylistError::InvalidLimits(_) => YoutubePlaybackErrorKind::InvalidOptions,
        PlaylistError::TooLarge { .. }
        | PlaylistError::LineTooLong { .. }
        | PlaylistError::TooManyEntries { .. }
        | PlaylistError::InvalidReference(_) => YoutubePlaybackErrorKind::InvalidMedia,
    };
    YoutubePlaybackError::new(kind)
}

fn map_hls_error(error: HlsError) -> YoutubePlaybackError {
    match error {
        HlsError::InvalidLimits(_) => {
            YoutubePlaybackError::new(YoutubePlaybackErrorKind::InvalidOptions)
        }
        HlsError::Media(error) => map_media_error(error),
        HlsError::Playlist(error) => map_playlist_error(&error),
        HlsError::InvalidPlaylist(_)
        | HlsError::TooManyVariants { .. }
        | HlsError::TooManySegments { .. }
        | HlsError::SegmentDurationExceeded { .. }
        | HlsError::PlaylistDurationExceeded { .. }
        | HlsError::UnsupportedFeature(_)
        | HlsError::LiveReloadLimitExceeded { .. }
        | HlsError::NotVod => YoutubePlaybackError::new(YoutubePlaybackErrorKind::InvalidMedia),
    }
}

const fn map_mpeg_ts_error(error: &MpegTsError) -> YoutubePlaybackError {
    let kind = match error {
        MpegTsError::InvalidLimits(_) => YoutubePlaybackErrorKind::InvalidOptions,
        MpegTsError::TruncatedPacket { .. }
        | MpegTsError::TooManyPackets { .. }
        | MpegTsError::InvalidPacket { .. }
        | MpegTsError::Continuity { .. }
        | MpegTsError::PsiSectionTooLarge { .. }
        | MpegTsError::InvalidPsi(_)
        | MpegTsError::TruncatedPsi
        | MpegTsError::MissingProgramMap
        | MpegTsError::MissingAdtsStream
        | MpegTsError::InvalidPes(_)
        | MpegTsError::TruncatedPes
        | MpegTsError::PesPayloadTooLarge { .. }
        | MpegTsError::MissingAdtsPayload
        | MpegTsError::MetadataTooLarge { .. } => YoutubePlaybackErrorKind::InvalidMedia,
    };
    YoutubePlaybackError::new(kind)
}

const fn map_source_error(error: YoutubeError) -> YoutubePlaybackError {
    if matches!(error.kind(), YoutubeErrorKind::Cancelled) {
        YoutubePlaybackError::new(YoutubePlaybackErrorKind::Cancelled)
    } else {
        YoutubePlaybackError::new(YoutubePlaybackErrorKind::Source(error.kind()))
    }
}

fn map_media_error(error: MediaError) -> YoutubePlaybackError {
    let kind = match error {
        MediaError::Cancelled => YoutubePlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            YoutubePlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            YoutubePlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => YoutubePlaybackErrorKind::Network,
        MediaError::UnsupportedContainer(_)
        | MediaError::UnsupportedCodec(_)
        | MediaError::UnsupportedCodecProfile { .. }
        | MediaError::NoAudioTrack
        | MediaError::ProbeLimitExceeded { .. }
        | MediaError::PacketTooLarge { .. }
        | MediaError::CodecConfigTooLarge { .. }
        | MediaError::CodecProbeLimitExceeded { .. }
        | MediaError::PcmFrameTooLarge { .. }
        | MediaError::OutputBufferTooSmall { .. }
        | MediaError::DecodeErrorLimitExceeded { .. }
        | MediaError::DecodeDelayLimitExceeded { .. }
        | MediaError::WrongOutputKind { .. }
        | MediaError::Backend { .. } => YoutubePlaybackErrorKind::InvalidMedia,
    };
    YoutubePlaybackError::new(kind)
}

const fn map_audio_error(_: AudioFrameError) -> YoutubePlaybackError {
    YoutubePlaybackError::new(YoutubePlaybackErrorKind::AudioPipeline)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use mantle_audio::EncodedFrameSlot;

    use super::PcmTranscoder;
    use crate::{MediaLimits, MediaSession};

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/media/fixtures")
            .join(name)
    }

    #[test]
    fn fallback_resamples_low_rate_pcm_into_monotonic_fixed_opus_frames() {
        let session =
            MediaSession::open_file(fixture("tone-aac-lc-24k.mkv"), MediaLimits::default())
                .unwrap();
        let mut transcoder = PcmTranscoder::new(session).unwrap();
        let mut output = EncodedFrameSlot::new();
        let mut previous = None;
        let mut frames = 0_usize;
        while transcoder.read_frame(&mut output).unwrap() {
            assert!(!output.data().is_empty());
            let timestamp = output.timestamp().unwrap();
            if let Some(previous) = previous {
                assert_eq!(timestamp, previous + output.duration());
            }
            previous = Some(timestamp);
            frames += 1;
        }
        assert!((100..=104).contains(&frames), "frames: {frames}");
    }

    #[test]
    fn fallback_duplicates_mono_and_emits_one_padded_final_frame() {
        let session = MediaSession::open_file(
            fixture("tone-pcm-s16le-mono-8k.wav"),
            MediaLimits::default(),
        )
        .unwrap();
        let mut transcoder = PcmTranscoder::new(session).unwrap();
        let mut output = EncodedFrameSlot::new();
        let mut frames = 0_usize;
        while transcoder.read_frame(&mut output).unwrap() {
            assert!(!output.data().is_empty());
            frames += 1;
        }
        assert_eq!(frames, 50);
        assert!(!transcoder.read_frame(&mut output).unwrap());
        assert!(output.data().is_empty());
    }
}
