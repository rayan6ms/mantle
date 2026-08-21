use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mantle_audio::PcmFrame;
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use serde_json::Value;
use ureq::http::Uri;

use crate::{
    Codec, Container, HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, MediaCancellation,
    MediaError, MediaInfo, MediaLimits, MediaSession, RemoteHttpClient, RemoteHttpErrorKind,
    RemoteHttpOptions, RemoteHttpRequest, SeekResult,
};

const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_COLLECTION_TRACKS: usize = 10_000;
const MAX_CONFIGURED_PAGE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_EMBEDDED_JSON_BYTES: usize = 32 * 1024 * 1024;
const MAX_CONFIGURED_PLAYBACK_URL_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_TRACK_DURATION: Duration = Duration::from_hours(31 * 24);
const TRALBUM_ATTRIBUTE: &[u8] = b"data-tralbum=\"";

/// Current public Bandcamp page shapes supported by the bounded web adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BandcampRoute {
    Track(String),
    Album(String),
}

impl BandcampRoute {
    fn url(&self) -> &str {
        match self {
            Self::Track(url) | Self::Album(url) => url,
        }
    }
}

/// Scheme policy for ephemeral Bandcamp media URLs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BandcampPlaybackScheme {
    #[default]
    Https,
    /// Permit HTTP only alongside the explicit private-network source policy.
    ///
    /// This exists for trusted loopback replay and must not be used for public service traffic.
    HttpForPrivateNetworks,
}

/// Bounded routing, public-page parsing, and media-discovery policy for Bandcamp.
#[derive(Clone, Eq, PartialEq)]
pub struct BandcampSourceOptions {
    pub http: RemoteHttpOptions,
    /// Optional page origin used by deterministic replay or an explicitly trusted mirror.
    ///
    /// The validated Bandcamp path is appended to this value. User identifiers still have to be
    /// strict public Bandcamp track or album URLs.
    pub page_origin_override: Option<String>,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_collection_tracks: usize,
    pub max_page_bytes: u64,
    pub max_embedded_json_bytes: usize,
    pub max_playback_url_bytes: usize,
    pub max_track_duration: Duration,
    pub playback_scheme: BandcampPlaybackScheme,
}

impl Default for BandcampSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            page_origin_override: None,
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_collection_tracks: 600,
            max_page_bytes: 4 * 1024 * 1024,
            max_embedded_json_bytes: 2 * 1024 * 1024,
            max_playback_url_bytes: 64 * 1024,
            max_track_duration: Duration::from_hours(24),
            playback_scheme: BandcampPlaybackScheme::Https,
        }
    }
}

impl BandcampSourceOptions {
    fn validate(&self) -> Result<(), BandcampError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_collection_tracks == 0
            || self.max_collection_tracks > MAX_CONFIGURED_COLLECTION_TRACKS
            || self.max_page_bytes == 0
            || self.max_page_bytes > MAX_CONFIGURED_PAGE_BYTES
            || self.max_page_bytes > self.http.max_response_bytes
            || self.max_embedded_json_bytes == 0
            || self.max_embedded_json_bytes > MAX_CONFIGURED_EMBEDDED_JSON_BYTES
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_PLAYBACK_URL_BYTES
            || self.max_track_duration.is_zero()
            || self.max_track_duration > MAX_CONFIGURED_TRACK_DURATION
            || (self.playback_scheme == BandcampPlaybackScheme::HttpForPrivateNetworks
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks)
        {
            return Err(BandcampError::new(BandcampErrorKind::InvalidOptions));
        }
        if let Some(origin) = self.page_origin_override.as_deref() {
            validate_page_origin(origin)?;
            if origin.starts_with("http://")
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks
            {
                return Err(BandcampError::new(BandcampErrorKind::InvalidOptions));
            }
        }
        Ok(())
    }
}

impl fmt::Debug for BandcampSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BandcampSourceOptions")
            .field("http", &self.http)
            .field(
                "page_origin_override_configured",
                &self.page_origin_override.is_some(),
            )
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_collection_tracks", &self.max_collection_tracks)
            .field("max_page_bytes", &self.max_page_bytes)
            .field("max_embedded_json_bytes", &self.max_embedded_json_bytes)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field("max_track_duration", &self.max_track_duration)
            .field("playback_scheme", &self.playback_scheme)
            .finish()
    }
}

/// Routes a bounded identifier without network access.
#[must_use]
pub fn route_bandcamp_identifier(
    identifier: &str,
    options: &BandcampSourceOptions,
) -> Option<BandcampRoute> {
    if identifier.is_empty()
        || identifier.len() > options.max_identifier_bytes
        || identifier.contains('#')
    {
        return None;
    }
    let without_query = identifier.split_once('?').map_or(identifier, |pair| pair.0);
    let normalized =
        if without_query.starts_with("http://") || without_query.starts_with("https://") {
            without_query.to_owned()
        } else {
            format!("https://{without_query}")
        };
    let uri: Uri = normalized.parse().ok()?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) {
        return None;
    }
    let authority = uri.authority()?;
    if authority.as_str().contains('@') || authority.as_str() != authority.host() {
        return None;
    }
    let host = authority.host().to_ascii_lowercase();
    if !valid_bandcamp_host(&host) {
        return None;
    }
    let segments: Vec<_> = uri.path().trim_matches('/').split('/').collect();
    if segments.len() != 2 || !valid_slug(segments[1]) {
        return None;
    }
    let canonical = format!("https://{host}/{}/{}", segments[0], segments[1]);
    match segments[0] {
        "track" => Some(BandcampRoute::Track(canonical)),
        "album" => Some(BandcampRoute::Album(canonical)),
        _ => None,
    }
}

fn valid_bandcamp_host(host: &str) -> bool {
    if host != "bandcamp.com" && !host.ends_with(".bandcamp.com") {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// An ephemeral Bandcamp MP3 URL. Its diagnostics always redact the value.
#[derive(Clone, Eq, PartialEq)]
pub struct BandcampPlaybackUrl {
    url: String,
}

impl BandcampPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }
}

impl fmt::Debug for BandcampPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BandcampPlaybackUrl(<redacted>)")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BandcampSourceTrack {
    pub info: TrackInfo,
    pub playback: Option<BandcampPlaybackUrl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BandcampSourcePlaylist {
    pub name: String,
    pub tracks: Vec<BandcampSourceTrack>,
    pub selected_track: Option<usize>,
    pub is_search_result: bool,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub author: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BandcampSourceItem {
    Track(BandcampSourceTrack),
    Playlist(BandcampSourcePlaylist),
}

pub struct BandcampPlaybackSession {
    session: MediaSession,
}

impl BandcampPlaybackSession {
    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        self.session.info()
    }

    /// Decodes one bounded PCM frame.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining the signed URL.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, BandcampPlaybackError> {
        self.session
            .read_pcm(output)
            .map_err(map_playback_media_error)
    }

    /// Seeks the bounded progressive media input.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining the signed URL.
    pub fn seek(&mut self, requested: Duration) -> Result<SeekResult, BandcampPlaybackError> {
        self.session
            .seek(requested)
            .map_err(map_playback_media_error)
    }
}

impl fmt::Debug for BandcampPlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BandcampPlaybackSession")
            .field("media", self.info())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BandcampPlaybackErrorKind {
    Source(BandcampErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BandcampPlaybackError {
    kind: BandcampPlaybackErrorKind,
}

impl BandcampPlaybackError {
    const fn new(kind: BandcampPlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> BandcampPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for BandcampPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            BandcampPlaybackErrorKind::Source(_) => "Bandcamp playback discovery failed",
            BandcampPlaybackErrorKind::InvalidOptions => "invalid Bandcamp media policy",
            BandcampPlaybackErrorKind::Cancelled => "Bandcamp playback cancelled",
            BandcampPlaybackErrorKind::Network => "Bandcamp media request failed",
            BandcampPlaybackErrorKind::InvalidMedia => "Bandcamp returned invalid media",
            BandcampPlaybackErrorKind::IncompatibleFormat => {
                "Bandcamp media is not a supported MP3 stream"
            }
        })
    }
}

impl std::error::Error for BandcampPlaybackError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BandcampErrorKind {
    InvalidOptions,
    Cancelled,
    Shutdown,
    Network,
    RateLimited,
    Unavailable,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BandcampError {
    kind: BandcampErrorKind,
}

impl BandcampError {
    const fn new(kind: BandcampErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> BandcampErrorKind {
        self.kind
    }
}

impl fmt::Display for BandcampError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            BandcampErrorKind::InvalidOptions => "invalid Bandcamp source policy",
            BandcampErrorKind::Cancelled => "Bandcamp load cancelled",
            BandcampErrorKind::Shutdown => "Bandcamp source is shut down",
            BandcampErrorKind::Network => "Bandcamp request failed",
            BandcampErrorKind::RateLimited => "Bandcamp rate limit reached",
            BandcampErrorKind::Unavailable => "Bandcamp content is unavailable",
            BandcampErrorKind::InvalidResponse => "Bandcamp returned an invalid page",
            BandcampErrorKind::UnsupportedRoute => "Bandcamp route is not implemented",
        })
    }
}

impl std::error::Error for BandcampError {}

pub struct BandcampSourceManager {
    options: BandcampSourceOptions,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl BandcampSourceManager {
    /// Creates a manager after validating all HTTP, page, parser, and media URL bounds.
    ///
    /// # Errors
    ///
    /// Returns [`BandcampErrorKind::InvalidOptions`] for an invalid bound or HTTP policy.
    pub fn new(options: BandcampSourceOptions) -> Result<Self, BandcampError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            http,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Loads one validated public track or album page.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, network, rate-limit, unavailable, or bounded parser failures.
    pub fn load_route(
        &self,
        route: &BandcampRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<BandcampSourceItem>, BandcampError> {
        self.ensure_active(cancellation)?;
        let Some(body) = self.get_page(route.url(), cancellation)? else {
            return Ok(None);
        };
        parse_page(&body, route, &self.options).map(Some)
    }

    /// Loads one public Bandcamp track page.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, network, unavailable, or bounded parser failures.
    pub fn load_track_metadata(
        &self,
        url: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<BandcampSourceTrack>, BandcampError> {
        let Some(route) = route_bandcamp_identifier(url, &self.options) else {
            return Err(BandcampError::new(BandcampErrorKind::UnsupportedRoute));
        };
        if !matches!(route, BandcampRoute::Track(_)) {
            return Err(BandcampError::new(BandcampErrorKind::UnsupportedRoute));
        }
        match self.load_route(&route, cancellation)? {
            Some(BandcampSourceItem::Track(track)) => Ok(Some(track)),
            Some(BandcampSourceItem::Playlist(_)) => {
                Err(BandcampError::new(BandcampErrorKind::InvalidResponse))
            }
            None => Ok(None),
        }
    }

    /// Re-fetches a track page and selects its current ephemeral `mp3-128` URL.
    ///
    /// # Errors
    ///
    /// Returns route, cancellation, network, unavailable, or bounded parser failures.
    pub fn resolve_track_playback(
        &self,
        track: &BandcampSourceTrack,
        cancellation: &MediaCancellation,
    ) -> Result<Option<BandcampPlaybackUrl>, BandcampError> {
        let Some(route) = route_bandcamp_identifier(&track.info.identifier, &self.options) else {
            return Err(BandcampError::new(BandcampErrorKind::InvalidResponse));
        };
        let BandcampRoute::Track(_) = route else {
            return Err(BandcampError::new(BandcampErrorKind::InvalidResponse));
        };
        match self.load_route(&route, cancellation)? {
            Some(BandcampSourceItem::Track(fresh)) => Ok(fresh.playback),
            Some(BandcampSourceItem::Playlist(_)) => {
                Err(BandcampError::new(BandcampErrorKind::InvalidResponse))
            }
            None => Ok(None),
        }
    }

    /// Opens the freshly discovered MP3 through Mantle's bounded seekable media pipeline.
    ///
    /// # Errors
    ///
    /// Returns source, cancellation, network, media, or incompatible-format failures.
    pub fn open_track_playback(
        &self,
        track: &BandcampSourceTrack,
        range_options: HttpRangeOptions,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Option<BandcampPlaybackSession>, BandcampPlaybackError> {
        let Some(playback) = self
            .resolve_track_playback(track, &cancellation)
            .map_err(map_playback_source_error)?
        else {
            return Ok(None);
        };
        let input = HttpRangeInput::open_with_cancellation(
            playback.as_str(),
            range_options,
            cancellation.clone(),
        )
        .map_err(map_playback_media_error)?;
        let session = MediaSession::open_with_cancellation(
            Box::new(input),
            Some("mp3"),
            media_limits,
            cancellation,
        )
        .map_err(map_playback_media_error)?;
        if session.info().container != Container::Mp3 || session.info().codec != Codec::Mp3 {
            return Err(BandcampPlaybackError::new(
                BandcampPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(Some(BandcampPlaybackSession { session }))
    }

    fn ensure_active(&self, cancellation: &MediaCancellation) -> Result<(), BandcampError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(BandcampError::new(BandcampErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(BandcampError::new(BandcampErrorKind::Cancelled));
        }
        Ok(())
    }

    fn get_page(
        &self,
        canonical_url: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<Vec<u8>>, BandcampError> {
        let endpoint = self.page_endpoint(canonical_url)?;
        let request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", "text/html,application/xhtml+xml"))
            .and_then(|request| request.header("User-Agent", "Mantle-Bandcamp/1"))
            .and_then(|request| request.max_response_bytes(self.options.max_page_bytes))
            .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidOptions))?;
        match self.http.execute_with_cancellation(&request, cancellation) {
            Ok(response) => Ok(Some(response.body().to_vec())),
            Err(error) if error.kind() == RemoteHttpErrorKind::NotFound => Ok(None),
            Err(error) => Err(map_remote_error(error)),
        }
    }

    fn page_endpoint(&self, canonical_url: &str) -> Result<String, BandcampError> {
        let Some(origin) = self.options.page_origin_override.as_deref() else {
            return Ok(canonical_url.to_owned());
        };
        let uri: Uri = canonical_url
            .parse()
            .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidResponse))?;
        Ok(format!("{}{}", origin.trim_end_matches('/'), uri.path()))
    }
}

impl fmt::Debug for BandcampSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BandcampSourceManager")
            .field("options", &self.options)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<BandcampSourceItem> for BandcampSourceManager {
    fn source_name(&self) -> &'static str {
        "bandcamp"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BandcampSourceItem>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<BandcampSourceItem>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_bandcamp_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = MediaCancellation::linked({
            let cancellation = cancellation.clone();
            move || cancellation.is_cancelled()
        });
        match self.load_route(&route, &linked) {
            Ok(Some(item)) => Ok(Some(SourceLoad::Item(item))),
            Ok(None) => Ok(None),
            Err(error) if error.kind() == BandcampErrorKind::Cancelled => Ok(None),
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, item: &BandcampSourceItem) -> bool {
        matches!(item, BandcampSourceItem::Track(_))
    }

    fn encode(&self, item: &BandcampSourceItem) -> Result<Vec<u8>, SourceRegistryError> {
        matches!(item, BandcampSourceItem::Track(_))
            .then(Vec::new)
            .ok_or(SourceRegistryError::NotEncodable)
    }

    fn decode(&self, _payload: &[u8]) -> Result<BandcampSourceItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<BandcampSourceItem, SourceRegistryError> {
        if !payload.is_empty()
            || !matches!(
                route_bandcamp_identifier(&info.identifier, &self.options),
                Some(BandcampRoute::Track(_))
            )
        {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(BandcampSourceItem::Track(BandcampSourceTrack {
            info: info.clone(),
            playback: None,
        }))
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_page(
    body: &[u8],
    route: &BandcampRoute,
    options: &BandcampSourceOptions,
) -> Result<BandcampSourceItem, BandcampError> {
    let encoded = extract_tralbum_attribute(body, options.max_embedded_json_bytes)?;
    let decoded = decode_html_attribute(encoded, options.max_embedded_json_bytes)?;
    let root: Value = serde_json::from_slice(&decoded)
        .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidResponse))?;
    let expected_type = match route {
        BandcampRoute::Track(_) => "track",
        BandcampRoute::Album(_) => "album",
    };
    let item_type = root
        .get("item_type")
        .and_then(Value::as_str)
        .or_else(|| {
            root.get("current")
                .and_then(|current| current.get("type"))
                .and_then(Value::as_str)
        })
        .ok_or_else(invalid_response)?;
    if item_type != expected_type {
        return Err(invalid_response());
    }
    let artist = bounded_string(
        root.get("artist").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let artwork_url = parse_artwork(&root, options)?;
    let track_values = root
        .get("trackinfo")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    if track_values.is_empty()
        || track_values.len() > options.max_collection_tracks
        || (matches!(route, BandcampRoute::Track(_)) && track_values.len() != 1)
    {
        return Err(invalid_response());
    }
    let page_base = page_base(route.url())?;
    let isrc = if matches!(route, BandcampRoute::Track(_)) {
        bounded_optional_string(
            root.get("current")
                .and_then(|current| current.get("isrc"))
                .and_then(Value::as_str),
            options.max_metadata_string_bytes,
        )?
    } else {
        None
    };
    let mut tracks = Vec::with_capacity(track_values.len());
    for value in track_values {
        tracks.push(parse_track(
            value,
            &page_base,
            &artist,
            artwork_url.as_deref(),
            isrc.as_deref(),
            options,
        )?);
    }
    match route {
        BandcampRoute::Track(_) => Ok(BandcampSourceItem::Track(tracks.remove(0))),
        BandcampRoute::Album(url) => {
            let name = bounded_string(
                root.get("current")
                    .and_then(|current| current.get("title"))
                    .and_then(Value::as_str),
                options.max_metadata_string_bytes,
            )?;
            Ok(BandcampSourceItem::Playlist(BandcampSourcePlaylist {
                name,
                tracks,
                selected_track: None,
                is_search_result: false,
                uri: Some(url.clone()),
                artwork_url,
                author: Some(artist),
            }))
        }
    }
}

fn parse_track(
    value: &Value,
    page_base: &str,
    page_artist: &str,
    artwork_url: Option<&str>,
    page_isrc: Option<&str>,
    options: &BandcampSourceOptions,
) -> Result<BandcampSourceTrack, BandcampError> {
    let title = bounded_string(
        value.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = match value.get("artist").and_then(Value::as_str) {
        Some(artist) => bounded_string(Some(artist), options.max_metadata_string_bytes)?,
        None => page_artist.to_owned(),
    };
    let seconds = value
        .get("duration")
        .and_then(Value::as_f64)
        .ok_or_else(invalid_response)?;
    if !seconds.is_finite() || seconds < 0.0 || seconds > options.max_track_duration.as_secs_f64() {
        return Err(invalid_response());
    }
    let title_link = bounded_string(
        value.get("title_link").and_then(Value::as_str),
        options.max_identifier_bytes,
    )?;
    let Some(slug) = title_link.strip_prefix("/track/") else {
        return Err(invalid_response());
    };
    if !valid_slug(slug) || slug.contains('/') {
        return Err(invalid_response());
    }
    let uri = format!("{page_base}{title_link}");
    if uri.len() > options.max_identifier_bytes {
        return Err(invalid_response());
    }
    let playback = parse_playback(value, options)?;
    Ok(BandcampSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration: Duration::from_secs_f64(seconds),
            identifier: uri.clone(),
            is_stream: false,
            uri: Some(uri),
            artwork_url: artwork_url.map(str::to_owned),
            isrc: page_isrc.map(str::to_owned),
        },
        playback,
    })
}

fn parse_playback(
    value: &Value,
    options: &BandcampSourceOptions,
) -> Result<Option<BandcampPlaybackUrl>, BandcampError> {
    let Some(url) = value
        .get("file")
        .and_then(|file| file.get("mp3-128"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let url = bounded_string(Some(url), options.max_playback_url_bytes)?;
    let uri: Uri = url.parse().map_err(|_| invalid_response())?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    let valid_scheme = match options.playback_scheme {
        BandcampPlaybackScheme::Https => {
            uri.scheme_str() == Some("https") && valid_bcbits_host(authority.host())
        }
        BandcampPlaybackScheme::HttpForPrivateNetworks => {
            matches!(uri.scheme_str(), Some("http" | "https"))
        }
    };
    if !valid_scheme || authority.as_str().contains('@') || url.contains('#') {
        return Err(invalid_response());
    }
    Ok(Some(BandcampPlaybackUrl { url }))
}

fn valid_bcbits_host(host: &str) -> bool {
    host == "bcbits.com" || host.ends_with(".bcbits.com")
}

fn parse_artwork(
    root: &Value,
    options: &BandcampSourceOptions,
) -> Result<Option<String>, BandcampError> {
    let Some(value) = root.get("art_id") else {
        return Ok(None);
    };
    let id = value
        .as_u64()
        .map(|id| id.to_string())
        .or_else(|| value.as_str().map(str::to_owned))
        .ok_or_else(invalid_response)?;
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_response());
    }
    let padded = format!("{id:0>10}");
    let artwork = format!("https://f4.bcbits.com/img/a{padded}_1.png");
    if artwork.len() > options.max_metadata_string_bytes {
        return Err(invalid_response());
    }
    Ok(Some(artwork))
}

fn extract_tralbum_attribute(body: &[u8], decoded_limit: usize) -> Result<&[u8], BandcampError> {
    let start =
        find_bytes(body, TRALBUM_ATTRIBUTE).ok_or_else(invalid_response)? + TRALBUM_ATTRIBUTE.len();
    let tail = &body[start..];
    let end = tail
        .iter()
        .position(|byte| *byte == b'"')
        .ok_or_else(invalid_response)?;
    let encoded = &tail[..end];
    if encoded.is_empty()
        || encoded.len() > decoded_limit.saturating_mul(8)
        || find_bytes(&tail[end + 1..], TRALBUM_ATTRIBUTE).is_some()
    {
        return Err(invalid_response());
    }
    Ok(encoded)
}

fn decode_html_attribute(encoded: &[u8], limit: usize) -> Result<Vec<u8>, BandcampError> {
    let mut decoded = Vec::with_capacity(encoded.len().min(limit));
    let mut offset = 0;
    while offset < encoded.len() {
        if encoded[offset] == b'&'
            && let Some(relative_end) = encoded[offset + 1..]
                .iter()
                .take(16)
                .position(|byte| *byte == b';')
        {
            let entity_end = offset + 1 + relative_end;
            if let Some(character) = decode_entity(&encoded[offset + 1..entity_end]) {
                let mut buffer = [0_u8; 4];
                append_bounded(
                    &mut decoded,
                    character.encode_utf8(&mut buffer).as_bytes(),
                    limit,
                )?;
                offset = entity_end + 1;
                continue;
            }
        }
        append_bounded(&mut decoded, &encoded[offset..=offset], limit)?;
        offset += 1;
    }
    Ok(decoded)
}

fn decode_entity(entity: &[u8]) -> Option<char> {
    match entity {
        b"quot" => Some('"'),
        b"amp" => Some('&'),
        b"apos" | b"#39" => Some('\''),
        b"lt" => Some('<'),
        b"gt" => Some('>'),
        _ => {
            let text = std::str::from_utf8(entity).ok()?;
            let codepoint =
                if let Some(hex) = text.strip_prefix("#x").or_else(|| text.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    text.strip_prefix('#')?.parse().ok()?
                };
            char::from_u32(codepoint).filter(|character| *character != '\0')
        }
    }
}

fn append_bounded(output: &mut Vec<u8>, bytes: &[u8], limit: usize) -> Result<(), BandcampError> {
    if output.len().saturating_add(bytes.len()) > limit {
        return Err(invalid_response());
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|candidate| candidate == needle)
}

fn page_base(url: &str) -> Result<String, BandcampError> {
    let uri: Uri = url.parse().map_err(|_| invalid_response())?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    Ok(format!("https://{}", authority.host()))
}

fn bounded_string(value: Option<&str>, limit: usize) -> Result<String, BandcampError> {
    let value = value.ok_or_else(invalid_response)?;
    (!value.is_empty() && value.len() <= limit)
        .then(|| value.to_owned())
        .ok_or_else(invalid_response)
}

fn bounded_optional_string(
    value: Option<&str>,
    limit: usize,
) -> Result<Option<String>, BandcampError> {
    value
        .map(|value| bounded_string(Some(value), limit))
        .transpose()
}

fn validate_page_origin(origin: &str) -> Result<(), BandcampError> {
    if origin.is_empty()
        || origin.len() > MAX_CONFIGURED_IDENTIFIER_BYTES
        || origin.contains(['?', '#', '@'])
    {
        return Err(BandcampError::new(BandcampErrorKind::InvalidOptions));
    }
    let uri: Uri = origin
        .parse()
        .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidOptions))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err(BandcampError::new(BandcampErrorKind::InvalidOptions));
    }
    RemoteHttpRequest::get(origin)
        .map(|_| ())
        .map_err(|_| BandcampError::new(BandcampErrorKind::InvalidOptions))
}

fn invalid_response() -> BandcampError {
    BandcampError::new(BandcampErrorKind::InvalidResponse)
}

fn map_remote_error(error: crate::RemoteHttpError) -> BandcampError {
    BandcampError::new(match error.kind() {
        RemoteHttpErrorKind::Cancelled => BandcampErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => BandcampErrorKind::RateLimited,
        RemoteHttpErrorKind::NotFound => BandcampErrorKind::Unavailable,
        _ => BandcampErrorKind::Network,
    })
}

fn map_playback_source_error(error: BandcampError) -> BandcampPlaybackError {
    BandcampPlaybackError::new(BandcampPlaybackErrorKind::Source(error.kind()))
}

fn map_playback_media_error(error: MediaError) -> BandcampPlaybackError {
    let kind = match error {
        MediaError::Cancelled => BandcampPlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            BandcampPlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            BandcampPlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => BandcampPlaybackErrorKind::Network,
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
        | MediaError::Backend { .. } => BandcampPlaybackErrorKind::InvalidMedia,
    };
    BandcampPlaybackError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_handles_named_and_numeric_entities_once() {
        let decoded = decode_html_attribute(
            br"{&quot;x&quot;:&quot;A &amp; B &#39; &#x2665; &amp;quot;&quot;}",
            1024,
        )
        .unwrap();
        assert_eq!(
            std::str::from_utf8(&decoded).unwrap(),
            r#"{"x":"A & B ' ♥ &quot;"}"#
        );
    }
}
