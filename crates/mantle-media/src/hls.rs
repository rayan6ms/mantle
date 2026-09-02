use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;

use crate::playlist::{
    PlaylistError, PlaylistLoadError, load_http_bytes, load_http_bytes_routed,
    validate_line_lengths,
};
use crate::{
    HttpPlaylistOptions, HttpStreamInput, HttpStreamOptions, MediaCancellation, MediaError,
    MediaInput, MpegTsLimits, OutboundRoutePolicy, PlaylistLimits, extract_mpeg_ts_adts,
    resolve_http_reference,
};

/// Resource limits for one parsed HLS master or media playlist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HlsLimits {
    pub max_variants: usize,
    pub max_segments: usize,
    pub max_segment_duration: Duration,
    pub max_playlist_duration: Duration,
}

impl Default for HlsLimits {
    fn default() -> Self {
        Self {
            max_variants: 64,
            max_segments: 10_000,
            max_segment_duration: Duration::from_hours(1),
            max_playlist_duration: Duration::from_hours(168),
        }
    }
}

impl HlsLimits {
    pub(crate) fn validate(self) -> Result<Self, HlsError> {
        if self.max_variants == 0 {
            return Err(HlsError::InvalidLimits("max_variants must be non-zero"));
        }
        if self.max_segments == 0 {
            return Err(HlsError::InvalidLimits("max_segments must be non-zero"));
        }
        if self.max_segment_duration.is_zero() {
            return Err(HlsError::InvalidLimits(
                "max_segment_duration must be non-zero",
            ));
        }
        if self.max_playlist_duration.is_zero() {
            return Err(HlsError::InvalidLimits(
                "max_playlist_duration must be non-zero",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsVariant {
    pub uri: String,
    pub bandwidth: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsMasterPlaylist {
    pub variants: Vec<HlsVariant>,
}

impl HlsMasterPlaylist {
    #[must_use]
    pub fn selected_variant(&self) -> Option<&HlsVariant> {
        self.variants.first()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsSegment {
    pub sequence: u64,
    pub uri: String,
    pub duration: Option<Duration>,
    pub title: Option<String>,
    pub discontinuity: bool,
}

/// Loads one finite HLS segment through the bounded sequential HTTP input.
///
/// # Errors
///
/// Returns an error for invalid HTTP policy, destination rejection, response failure, truncation,
/// or a body that exceeds [`HttpStreamOptions::max_response_bytes`].
pub fn load_http_hls_segment(
    segment: &HlsSegment,
    options: HttpStreamOptions,
) -> Result<Vec<u8>, MediaError> {
    load_http_hls_segment_with_cancellation(segment, options, MediaCancellation::new())
}

/// Loads one finite HLS segment through bounded HTTP with cancellation.
///
/// # Errors
///
/// Returns [`MediaError::Cancelled`] when cancellation is requested before or during the body,
/// in addition to the errors from [`load_http_hls_segment`].
pub fn load_http_hls_segment_with_cancellation(
    segment: &HlsSegment,
    options: HttpStreamOptions,
    cancellation: MediaCancellation,
) -> Result<Vec<u8>, MediaError> {
    let cancellation_state = cancellation.clone();
    let mut input = HttpStreamInput::open_with_cancellation(&segment.uri, options, cancellation)?;
    let capacity = input
        .byte_len()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0)
        .min(usize::try_from(options.max_response_bytes).unwrap_or(usize::MAX));
    let mut bytes = Vec::with_capacity(capacity);
    if let Err(error) = input.read_to_end(&mut bytes) {
        if cancellation_state.is_cancelled() {
            return Err(MediaError::Cancelled);
        }
        return Err(MediaError::Io(error));
    }
    Ok(bytes)
}

pub(crate) fn load_http_hls_segment_routed_with_cancellation(
    segment: &HlsSegment,
    options: HttpStreamOptions,
    cancellation: MediaCancellation,
    route_policy: Arc<dyn OutboundRoutePolicy>,
) -> Result<Vec<u8>, MediaError> {
    let cancellation_state = cancellation.clone();
    let mut input = HttpStreamInput::open_routed_with_cancellation(
        &segment.uri,
        options,
        cancellation,
        route_policy,
    )?;
    let capacity = input
        .byte_len()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0)
        .min(usize::try_from(options.max_response_bytes).unwrap_or(usize::MAX));
    let mut bytes = Vec::with_capacity(capacity);
    if let Err(error) = input.read_to_end(&mut bytes) {
        if cancellation_state.is_cancelled() {
            return Err(MediaError::Cancelled);
        }
        return Err(MediaError::Io(error));
    }
    Ok(bytes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HlsMediaPlaylist {
    pub media_sequence: u64,
    pub target_duration: Option<Duration>,
    pub end_list: bool,
    pub segments: Vec<HlsSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HlsPlaylist {
    Master(HlsMasterPlaylist),
    Media(HlsMediaPlaylist),
}

#[derive(Debug)]
pub struct HlsVodSequence {
    media: HlsMediaPlaylist,
    next_index: usize,
}

impl HlsVodSequence {
    /// Creates a finite cursor over an HLS media playlist.
    ///
    /// # Errors
    ///
    /// Returns [`HlsError::NotVod`] when `#EXT-X-ENDLIST` is absent.
    pub fn new(media: HlsMediaPlaylist) -> Result<Self, HlsError> {
        if !media.end_list {
            return Err(HlsError::NotVod);
        }
        Ok(Self {
            media,
            next_index: 0,
        })
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.media.segments.len().saturating_sub(self.next_index)
    }

    pub fn next_segment(&mut self) -> Option<&HlsSegment> {
        let index = self.next_index;
        let segment = self.media.segments.get(index)?;
        self.next_index += 1;
        Some(segment)
    }
}

/// Bounds for live-playlist polling and retained segment identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HlsLiveLimits {
    pub max_retained_segments: usize,
    pub reload_interval: Duration,
    pub max_segment_wait: Duration,
    pub max_no_progress_reloads: usize,
}

impl Default for HlsLiveLimits {
    fn default() -> Self {
        Self {
            max_retained_segments: 1_024,
            reload_interval: Duration::from_millis(200),
            max_segment_wait: Duration::from_hours(1),
            max_no_progress_reloads: 20_000,
        }
    }
}

impl HlsLiveLimits {
    pub(crate) fn validate(self) -> Result<Self, HlsError> {
        if self.max_retained_segments == 0 {
            return Err(HlsError::InvalidLimits(
                "max_retained_segments must be non-zero",
            ));
        }
        if self.reload_interval.is_zero() {
            return Err(HlsError::InvalidLimits("reload_interval must be non-zero"));
        }
        if self.max_segment_wait.is_zero() {
            return Err(HlsError::InvalidLimits("max_segment_wait must be non-zero"));
        }
        if self.max_no_progress_reloads == 0 {
            return Err(HlsError::InvalidLimits(
                "max_no_progress_reloads must be non-zero",
            ));
        }
        Ok(self)
    }
}

/// Result of applying one fetched media-playlist snapshot to [`HlsLiveSequence`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HlsLivePoll {
    Segment(HlsSegment),
    WaitUntil(Duration),
    Ended,
    Exhausted,
}

/// Bounded deterministic state for selecting segments from live HLS snapshots.
#[derive(Debug)]
pub struct HlsLiveSequence {
    limits: HlsLiveLimits,
    last_uri: Option<String>,
    retained_uris: VecDeque<String>,
    wait_started_at: Option<Duration>,
    next_reload_at: Option<Duration>,
    last_poll_at: Option<Duration>,
    no_progress_reloads: usize,
}

impl HlsLiveSequence {
    /// Creates live selection state with centralized history and polling bounds.
    ///
    /// # Errors
    ///
    /// Returns [`HlsError::InvalidLimits`] when any live bound is zero.
    pub fn new(limits: HlsLiveLimits) -> Result<Self, HlsError> {
        let limits = limits.validate()?;
        Ok(Self {
            limits,
            last_uri: None,
            retained_uris: VecDeque::new(),
            wait_started_at: None,
            next_reload_at: None,
            last_poll_at: None,
            no_progress_reloads: 0,
        })
    }

    /// Applies one already-bounded playlist snapshot at a caller-supplied monotonic time.
    ///
    /// Selection follows the reference's oldest-first behavior. When the last emitted URI is no
    /// longer retained, the oldest unseen URI in the current window is selected. A no-progress
    /// snapshot requests another poll after the configured interval, capped by the first segment's
    /// declared duration and `max_segment_wait`.
    ///
    /// # Errors
    ///
    /// Returns an error when time moves backwards or the no-progress reload ceiling is exceeded.
    pub fn poll(
        &mut self,
        playlist: &HlsMediaPlaylist,
        now: Duration,
    ) -> Result<HlsLivePoll, HlsError> {
        if self.last_poll_at.is_some_and(|previous| now < previous) {
            return Err(HlsError::InvalidPlaylist("live poll time moved backwards"));
        }
        self.last_poll_at = Some(now);
        if let Some(next_reload_at) = self.next_reload_at
            && now < next_reload_at
        {
            return Ok(HlsLivePoll::WaitUntil(next_reload_at));
        }

        let start = self
            .last_uri
            .as_ref()
            .and_then(|last| {
                playlist
                    .segments
                    .iter()
                    .position(|segment| segment.uri == *last)
            })
            .map_or(0, |index| index + 1);
        let selected = playlist.segments[start..]
            .iter()
            .find(|segment| !self.retained_uris.contains(&segment.uri))
            .cloned();
        if let Some(segment) = selected {
            self.record_segment(&segment.uri);
            self.wait_started_at = None;
            self.next_reload_at = None;
            self.no_progress_reloads = 0;
            return Ok(HlsLivePoll::Segment(segment));
        }
        if playlist.end_list {
            self.wait_started_at = None;
            self.next_reload_at = None;
            return Ok(HlsLivePoll::Ended);
        }

        let Some(duration) = playlist
            .segments
            .first()
            .and_then(|segment| segment.duration)
        else {
            self.next_reload_at = None;
            return Ok(HlsLivePoll::Exhausted);
        };
        let wait = duration.min(self.limits.max_segment_wait);
        let started = *self.wait_started_at.get_or_insert(now);
        let deadline = started.checked_add(wait).unwrap_or(Duration::MAX);
        if now >= deadline {
            self.next_reload_at = None;
            return Ok(HlsLivePoll::Exhausted);
        }
        self.no_progress_reloads = self.no_progress_reloads.saturating_add(1);
        if self.no_progress_reloads > self.limits.max_no_progress_reloads {
            return Err(HlsError::LiveReloadLimitExceeded {
                limit: self.limits.max_no_progress_reloads,
            });
        }
        let next_reload_at = now
            .checked_add(self.limits.reload_interval)
            .unwrap_or(Duration::MAX)
            .min(deadline);
        self.next_reload_at = Some(next_reload_at);
        Ok(HlsLivePoll::WaitUntil(next_reload_at))
    }

    /// Fetches and applies one bounded HTTP live-playlist snapshot.
    ///
    /// A call before a previously returned `WaitUntil` deadline returns that same deadline without
    /// making another request.
    ///
    /// # Errors
    ///
    /// Returns HTTP, playlist, HLS parser, live state, or unexpected-master-playlist errors.
    pub fn poll_http(
        &mut self,
        url: impl AsRef<str>,
        options: HttpPlaylistOptions,
        hls_limits: HlsLimits,
        now: Duration,
    ) -> Result<HlsLivePoll, HlsError> {
        self.poll_http_with_cancellation(url, options, hls_limits, now, MediaCancellation::new())
    }

    /// Fetches and applies one bounded HTTP live snapshot with cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`HlsError::Media`] when cancellation or HTTP input fails, in addition to the
    /// errors from [`Self::poll_http`].
    pub fn poll_http_with_cancellation(
        &mut self,
        url: impl AsRef<str>,
        options: HttpPlaylistOptions,
        hls_limits: HlsLimits,
        now: Duration,
        cancellation: MediaCancellation,
    ) -> Result<HlsLivePoll, HlsError> {
        cancellation.check().map_err(HlsError::Media)?;
        if self.last_poll_at.is_some_and(|previous| now < previous) {
            return Err(HlsError::InvalidPlaylist("live poll time moved backwards"));
        }
        if let Some(next_reload_at) = self.next_reload_at
            && now < next_reload_at
        {
            self.last_poll_at = Some(now);
            return Ok(HlsLivePoll::WaitUntil(next_reload_at));
        }
        let playlist =
            load_http_hls_playlist_with_cancellation(url, options, hls_limits, cancellation)?;
        let HlsPlaylist::Media(media) = playlist else {
            return Err(HlsError::InvalidPlaylist(
                "live reload returned a master playlist",
            ));
        };
        self.poll(&media, now)
    }

    pub(crate) fn poll_http_routed_with_cancellation(
        &mut self,
        url: impl AsRef<str>,
        options: HttpPlaylistOptions,
        hls_limits: HlsLimits,
        now: Duration,
        cancellation: MediaCancellation,
        route_policy: Arc<dyn OutboundRoutePolicy>,
    ) -> Result<HlsLivePoll, HlsError> {
        cancellation.check().map_err(HlsError::Media)?;
        if self.last_poll_at.is_some_and(|previous| now < previous) {
            return Err(HlsError::InvalidPlaylist("live poll time moved backwards"));
        }
        if let Some(next_reload_at) = self.next_reload_at
            && now < next_reload_at
        {
            self.last_poll_at = Some(now);
            return Ok(HlsLivePoll::WaitUntil(next_reload_at));
        }
        let playlist = load_http_hls_playlist_routed_with_cancellation(
            url,
            options,
            hls_limits,
            cancellation,
            route_policy,
        )?;
        let HlsPlaylist::Media(media) = playlist else {
            return Err(HlsError::InvalidPlaylist(
                "live reload returned a master playlist",
            ));
        };
        self.poll(&media, now)
    }

    #[must_use]
    pub fn retained_segments(&self) -> usize {
        self.retained_uris.len()
    }

    #[must_use]
    pub fn retained_segment_capacity(&self) -> usize {
        self.retained_uris.capacity()
    }

    #[must_use]
    pub fn retained_identity_capacity_bytes(&self) -> usize {
        self.retained_uris
            .iter()
            .map(String::capacity)
            .sum::<usize>()
            .saturating_add(self.last_uri.as_ref().map_or(0, String::capacity))
    }

    fn record_segment(&mut self, uri: &str) {
        self.last_uri = Some(uri.to_owned());
        self.retained_uris.push_back(uri.to_owned());
        if self.retained_uris.len() > self.limits.max_retained_segments {
            self.retained_uris.pop_front();
        }
    }
}

/// Forward-only input that fetches each finite HLS VOD segment, strips MPEG-TS/PES, and exposes
/// one joined ADTS stream while retaining at most one extracted segment.
pub struct HlsVodAdtsInput {
    sequence: HlsVodSequence,
    http_options: HttpStreamOptions,
    mpeg_ts_limits: MpegTsLimits,
    cancellation: MediaCancellation,
    current: Cursor<Box<[u8]>>,
    eof: bool,
}

impl HlsVodAdtsInput {
    /// Creates a joined input for a finite media playlist.
    ///
    /// # Errors
    ///
    /// Returns [`HlsError::NotVod`] when `#EXT-X-ENDLIST` is absent.
    pub fn new(
        media: HlsMediaPlaylist,
        http_options: HttpStreamOptions,
        mpeg_ts_limits: MpegTsLimits,
    ) -> Result<Self, HlsError> {
        Self::new_with_cancellation(
            media,
            http_options,
            mpeg_ts_limits,
            MediaCancellation::new(),
        )
    }

    /// Creates a joined finite input with a caller-owned cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns [`HlsError::NotVod`] when `#EXT-X-ENDLIST` is absent. HTTP and MPEG-TS failures
    /// surface as ordinary [`io::Error`] values from [`Read`].
    pub fn new_with_cancellation(
        media: HlsMediaPlaylist,
        http_options: HttpStreamOptions,
        mpeg_ts_limits: MpegTsLimits,
        cancellation: MediaCancellation,
    ) -> Result<Self, HlsError> {
        Ok(Self {
            sequence: HlsVodSequence::new(media)?,
            http_options,
            mpeg_ts_limits,
            cancellation,
            current: Cursor::new(Vec::new().into_boxed_slice()),
            eof: false,
        })
    }
}

impl Read for HlsVodAdtsInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check_io()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            let count = self.current.read(buffer)?;
            if count != 0 {
                return Ok(count);
            }
            if self.eof {
                return Ok(0);
            }
            let Some(segment) = self.sequence.next_segment().cloned() else {
                self.eof = true;
                return Ok(0);
            };
            let transport = load_http_hls_segment_with_cancellation(
                &segment,
                self.http_options,
                self.cancellation.clone(),
            )
            .map_err(media_error_to_io)?;
            self.cancellation.check_io()?;
            let extracted = extract_mpeg_ts_adts(&transport, self.mpeg_ts_limits)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            self.current = Cursor::new(extracted.into_adts_bytes());
        }
    }
}

impl Seek for HlsVodAdtsInput {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "joined HLS ADTS input is not seekable",
        ))
    }
}

impl MediaInput for HlsVodAdtsInput {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

fn media_error_to_io(error: MediaError) -> io::Error {
    let kind = if matches!(error, MediaError::Cancelled) {
        io::ErrorKind::Interrupted
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, error)
}

#[derive(Debug)]
pub enum HlsError {
    InvalidLimits(&'static str),
    InvalidPlaylist(&'static str),
    TooManyVariants { limit: usize },
    TooManySegments { limit: usize },
    SegmentDurationExceeded { actual: Duration, limit: Duration },
    PlaylistDurationExceeded { actual: Duration, limit: Duration },
    UnsupportedFeature(&'static str),
    LiveReloadLimitExceeded { limit: usize },
    NotVod,
    Media(MediaError),
    Playlist(PlaylistError),
}

impl fmt::Display for HlsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits(message) => write!(formatter, "invalid HLS limits: {message}"),
            Self::InvalidPlaylist(message) => write!(formatter, "invalid HLS playlist: {message}"),
            Self::TooManyVariants { limit } => {
                write!(
                    formatter,
                    "HLS playlist contains more than {limit} variants"
                )
            }
            Self::TooManySegments { limit } => {
                write!(
                    formatter,
                    "HLS playlist contains more than {limit} segments"
                )
            }
            Self::SegmentDurationExceeded { actual, limit } => write!(
                formatter,
                "HLS segment duration {actual:?} exceeds the {limit:?} limit"
            ),
            Self::PlaylistDurationExceeded { actual, limit } => write!(
                formatter,
                "HLS playlist duration {actual:?} exceeds the {limit:?} limit"
            ),
            Self::UnsupportedFeature(feature) => {
                write!(formatter, "unsupported HLS feature: {feature}")
            }
            Self::LiveReloadLimitExceeded { limit } => write!(
                formatter,
                "HLS live playlist made no progress after {limit} reloads"
            ),
            Self::NotVod => formatter.write_str("HLS media playlist is not finite VOD"),
            Self::Media(error) => error.fmt(formatter),
            Self::Playlist(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for HlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Media(error) => Some(error),
            Self::Playlist(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PlaylistError> for HlsError {
    fn from(error: PlaylistError) -> Self {
        Self::Playlist(error)
    }
}

impl From<PlaylistLoadError> for HlsError {
    fn from(error: PlaylistLoadError) -> Self {
        match error {
            PlaylistLoadError::Media(error) => Self::Media(error),
            PlaylistLoadError::Playlist(error) => Self::Playlist(error),
        }
    }
}

/// Loads and parses one bounded HTTP HLS master or media playlist.
///
/// # Errors
///
/// Returns an error for HTTP/cancellation failures or any error from [`parse_hls_playlist`].
pub fn load_http_hls_playlist(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    hls_limits: HlsLimits,
) -> Result<HlsPlaylist, HlsError> {
    load_http_hls_playlist_with_cancellation(url, options, hls_limits, MediaCancellation::new())
}

/// Loads and parses one bounded HTTP HLS playlist with cancellation.
///
/// # Errors
///
/// Returns [`HlsError::Media`] for HTTP/cancellation failures and the ordinary parser errors for
/// invalid or unsupported HLS content.
pub fn load_http_hls_playlist_with_cancellation(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    hls_limits: HlsLimits,
    cancellation: MediaCancellation,
) -> Result<HlsPlaylist, HlsError> {
    let playlist_limits = options.playlist;
    let (base, bytes) = load_http_bytes(url, options, cancellation)?;
    parse_hls_playlist(&bytes, &base, playlist_limits, hls_limits)
}

pub(crate) fn load_http_hls_playlist_routed_with_cancellation(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    hls_limits: HlsLimits,
    cancellation: MediaCancellation,
    route_policy: Arc<dyn OutboundRoutePolicy>,
) -> Result<HlsPlaylist, HlsError> {
    let playlist_limits = options.playlist;
    let (base, bytes) = load_http_bytes_routed(url, options, cancellation, route_policy)?;
    parse_hls_playlist(&bytes, &base, playlist_limits, hls_limits)
}

/// Parses one bounded HLS master or media playlist and resolves all selected URIs.
///
/// # Errors
///
/// Returns an error for invalid limits, oversized input/lines/collections, malformed required
/// fields, invalid HTTP references, or unsupported encryption/map/byte-range features.
pub fn parse_hls_playlist(
    bytes: &[u8],
    base_uri: &str,
    playlist_limits: PlaylistLimits,
    hls_limits: HlsLimits,
) -> Result<HlsPlaylist, HlsError> {
    let playlist_limits = playlist_limits.validate()?;
    let hls_limits = hls_limits.validate()?;
    if bytes.len() > playlist_limits.max_playlist_bytes {
        return Err(PlaylistError::TooLarge {
            actual: bytes.len(),
            limit: playlist_limits.max_playlist_bytes,
        }
        .into());
    }
    validate_line_lengths(bytes, playlist_limits.max_line_bytes)?;
    let text = String::from_utf8_lossy(bytes);
    if !text.starts_with("#EXTM3U") {
        return Err(HlsError::InvalidPlaylist("missing #EXTM3U header"));
    }
    reject_unsupported_features(&text)?;
    if text
        .lines()
        .any(|line| line.trim().starts_with("#EXT-X-STREAM-INF:"))
    {
        parse_master(&text, base_uri, playlist_limits, hls_limits).map(HlsPlaylist::Master)
    } else {
        parse_media(&text, base_uri, playlist_limits, hls_limits).map(HlsPlaylist::Media)
    }
}

fn parse_master(
    text: &str,
    base_uri: &str,
    playlist_limits: PlaylistLimits,
    hls_limits: HlsLimits,
) -> Result<HlsMasterPlaylist, HlsError> {
    let mut variants = Vec::new();
    let mut pending_variant = None;
    let mut data_lines = 0;
    for raw_line in text.lines().skip(1) {
        let line = raw_line.trim();
        if let Some(attributes) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            pending_variant = Some(
                attribute_value(attributes, "BANDWIDTH")
                    .and_then(|value| value.parse::<u64>().ok()),
            );
        } else if !line.is_empty() && !line.starts_with('#') {
            count_data_line(&mut data_lines, playlist_limits.max_entries)?;
            if let Some(bandwidth) = pending_variant.take() {
                if variants.len() == hls_limits.max_variants {
                    return Err(HlsError::TooManyVariants {
                        limit: hls_limits.max_variants,
                    });
                }
                variants.push(HlsVariant {
                    uri: resolve_http_reference(base_uri, line)?,
                    bandwidth,
                });
            }
        }
    }
    if pending_variant.is_some() {
        return Err(HlsError::InvalidPlaylist("variant URI is missing"));
    }
    if variants.is_empty() {
        return Err(HlsError::InvalidPlaylist("master playlist has no variants"));
    }
    Ok(HlsMasterPlaylist { variants })
}

fn parse_media(
    text: &str,
    base_uri: &str,
    playlist_limits: PlaylistLimits,
    hls_limits: HlsLimits,
) -> Result<HlsMediaPlaylist, HlsError> {
    let mut media_sequence = 0;
    let mut target_duration = None;
    let mut end_list = false;
    let mut segments = Vec::new();
    let mut pending_info: Option<(Option<Duration>, Option<String>)> = None;
    let mut pending_discontinuity = false;
    let mut total_duration = Duration::ZERO;
    let mut data_lines = 0;

    for raw_line in text.lines().skip(1) {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            let duration = Duration::from_secs(parse_u64(value, "invalid target duration")?);
            if duration.is_zero() {
                return Err(HlsError::InvalidPlaylist("target duration is zero"));
            }
            if duration > hls_limits.max_segment_duration {
                return Err(HlsError::SegmentDurationExceeded {
                    actual: duration,
                    limit: hls_limits.max_segment_duration,
                });
            }
            target_duration = Some(duration);
        } else if let Some(value) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            media_sequence = parse_u64(value, "invalid media sequence")?;
        } else if let Some(value) = line.strip_prefix("#EXTINF:") {
            pending_info = Some(parse_segment_info(value, hls_limits)?);
        } else if line == "#EXT-X-DISCONTINUITY" {
            pending_discontinuity = true;
        } else if line == "#EXT-X-ENDLIST" {
            end_list = true;
        } else if !line.is_empty() && !line.starts_with('#') {
            count_data_line(&mut data_lines, playlist_limits.max_entries)?;
            if segments.len() == hls_limits.max_segments {
                return Err(HlsError::TooManySegments {
                    limit: hls_limits.max_segments,
                });
            }
            let sequence_offset = u64::try_from(segments.len())
                .map_err(|_| HlsError::InvalidPlaylist("segment sequence overflow"))?;
            let sequence = media_sequence
                .checked_add(sequence_offset)
                .ok_or(HlsError::InvalidPlaylist("segment sequence overflow"))?;
            let (duration, title) = pending_info.take().unwrap_or((None, None));
            if let Some(duration) = duration {
                total_duration = total_duration.checked_add(duration).ok_or(
                    HlsError::PlaylistDurationExceeded {
                        actual: Duration::MAX,
                        limit: hls_limits.max_playlist_duration,
                    },
                )?;
                if total_duration > hls_limits.max_playlist_duration {
                    return Err(HlsError::PlaylistDurationExceeded {
                        actual: total_duration,
                        limit: hls_limits.max_playlist_duration,
                    });
                }
            }
            segments.push(HlsSegment {
                sequence,
                uri: resolve_http_reference(base_uri, line)?,
                duration,
                title,
                discontinuity: std::mem::take(&mut pending_discontinuity),
            });
        }
    }
    if segments.is_empty() {
        return Err(HlsError::InvalidPlaylist("media playlist has no segments"));
    }
    Ok(HlsMediaPlaylist {
        media_sequence,
        target_duration,
        end_list,
        segments,
    })
}

fn parse_segment_info(
    value: &str,
    limits: HlsLimits,
) -> Result<(Option<Duration>, Option<String>), HlsError> {
    let Some((duration, title)) = value.split_once(',') else {
        return Ok((None, None));
    };
    let duration = parse_decimal_millis(duration)?;
    if duration > limits.max_segment_duration {
        return Err(HlsError::SegmentDurationExceeded {
            actual: duration,
            limit: limits.max_segment_duration,
        });
    }
    Ok((Some(duration), Some(title.to_owned())))
}

fn parse_decimal_millis(value: &str) -> Result<Duration, HlsError> {
    let (seconds, fraction) = value
        .split_once('.')
        .map_or((value, ""), |(seconds, fraction)| (seconds, fraction));
    if seconds.is_empty()
        || !seconds.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(HlsError::InvalidPlaylist("invalid segment duration"));
    }
    let seconds = parse_u64(seconds, "invalid segment duration")?;
    let mut milliseconds = 0_u64;
    let mut scale = 100_u64;
    for digit in fraction.bytes().take(3) {
        milliseconds += u64::from(digit - b'0') * scale;
        scale /= 10;
    }
    seconds
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(milliseconds))
        .map(Duration::from_millis)
        .ok_or(HlsError::InvalidPlaylist("segment duration overflow"))
}

fn attribute_value<'a>(attributes: &'a str, wanted: &str) -> Option<&'a str> {
    let mut remaining = attributes;
    while !remaining.is_empty() {
        let (name, after_name) = remaining.split_once('=')?;
        let (value, rest) = if let Some(quoted) = after_name.strip_prefix('"') {
            let end = quoted.find('"')?;
            (
                &quoted[..end],
                quoted[end + 1..].strip_prefix(',').unwrap_or(""),
            )
        } else {
            after_name
                .split_once(',')
                .map_or((after_name, ""), |(value, rest)| (value, rest))
        };
        if name == wanted {
            return Some(value);
        }
        remaining = rest;
    }
    None
}

fn parse_u64(value: &str, message: &'static str) -> Result<u64, HlsError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(HlsError::InvalidPlaylist(message));
    }
    value
        .parse()
        .map_err(|_| HlsError::InvalidPlaylist(message))
}

fn count_data_line(count: &mut usize, limit: usize) -> Result<(), HlsError> {
    if *count == limit {
        return Err(PlaylistError::TooManyEntries { limit }.into());
    }
    *count += 1;
    Ok(())
}

fn reject_unsupported_features(text: &str) -> Result<(), HlsError> {
    for line in text.lines().map(str::trim) {
        if let Some(attributes) = line.strip_prefix("#EXT-X-KEY:")
            && attribute_value(attributes, "METHOD") != Some("NONE")
        {
            return Err(HlsError::UnsupportedFeature("encryption"));
        }
        if line.starts_with("#EXT-X-MAP:") {
            return Err(HlsError::UnsupportedFeature("initialization maps"));
        }
        if line.starts_with("#EXT-X-BYTERANGE:") {
            return Err(HlsError::UnsupportedFeature("byte ranges"));
        }
    }
    Ok(())
}
