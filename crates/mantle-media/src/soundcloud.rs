use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{
    Codec, Container, HttpRangeInput, HttpRangeOptions, MediaCancellation, MediaError, MediaInfo,
    MediaLimits, MediaSession, RemoteHttpClient, RemoteHttpErrorKind, RemoteHttpOptions,
    RemoteHttpRequest, SeekResult,
};
use mantle_audio::PcmFrame;
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use serde_json::Value;

const DEFAULT_API_BASE_URL: &str = "https://api-v2.soundcloud.com";
const MAX_API_BASE_URL_BYTES: usize = 4 * 1024;
const MAX_CLIENT_ID_BYTES: usize = 256;
const MAX_OAUTH_TOKEN_BYTES: usize = 16 * 1024;
const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_COLLECTION_TRACKS: usize = 10_000;
const MAX_CONFIGURED_TRANSCODINGS: usize = 128;
const MAX_CONFIGURED_STREAM_URL_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoundCloudRoute {
    Resolve(String),
    Search(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundCloudAccess {
    Playable,
    Preview,
    Blocked,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SoundCloudAuthentication {
    client_id: String,
    oauth_token: Option<String>,
}

impl SoundCloudAuthentication {
    /// Creates a public `SoundCloud` authentication policy from an explicit client ID.
    ///
    /// # Errors
    ///
    /// Returns [`SoundCloudErrorKind::InvalidAuthentication`] for an empty, oversized, or
    /// non-graphic client ID.
    pub fn new(client_id: impl Into<String>) -> Result<Self, SoundCloudError> {
        Self::with_oauth(client_id, None::<String>)
    }

    /// Creates a client-ID policy with an optional bounded OAuth access token.
    ///
    /// # Errors
    ///
    /// Returns [`SoundCloudErrorKind::InvalidAuthentication`] when either credential violates
    /// its size or header-safe character policy.
    pub fn with_oauth(
        client_id: impl Into<String>,
        oauth_token: Option<impl Into<String>>,
    ) -> Result<Self, SoundCloudError> {
        let client_id = client_id.into();
        let oauth_token = oauth_token.map(Into::into);
        if client_id.is_empty()
            || client_id.len() > MAX_CLIENT_ID_BYTES
            || !client_id.bytes().all(|byte| byte.is_ascii_graphic())
            || oauth_token.as_deref().is_some_and(|token| {
                token.is_empty()
                    || token.len() > MAX_OAUTH_TOKEN_BYTES
                    || !token.bytes().all(|byte| byte.is_ascii_graphic())
            })
        {
            return Err(SoundCloudError::new(
                SoundCloudErrorKind::InvalidAuthentication,
            ));
        }
        Ok(Self {
            client_id,
            oauth_token,
        })
    }

    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    #[must_use]
    pub const fn oauth_configured(&self) -> bool {
        self.oauth_token.is_some()
    }
}

impl fmt::Debug for SoundCloudAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SoundCloudAuthentication")
            .field("client_id", &"<redacted>")
            .field("oauth_configured", &self.oauth_token.is_some())
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SoundCloudSourceOptions {
    pub http: RemoteHttpOptions,
    pub api_base_url: String,
    pub allow_search: bool,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_collection_tracks: usize,
    pub max_transcodings: usize,
    pub max_stream_url_bytes: usize,
    pub max_response_bytes: u64,
}

impl Default for SoundCloudSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
            allow_search: true,
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_collection_tracks: 600,
            max_transcodings: 32,
            max_stream_url_bytes: 64 * 1024,
            max_response_bytes: 2 * 1024 * 1024,
        }
    }
}

impl SoundCloudSourceOptions {
    fn validate(&self) -> Result<(), SoundCloudError> {
        if self.api_base_url.is_empty()
            || self.api_base_url.len() > MAX_API_BASE_URL_BYTES
            || self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_collection_tracks == 0
            || self.max_collection_tracks > MAX_CONFIGURED_COLLECTION_TRACKS
            || self.max_transcodings == 0
            || self.max_transcodings > MAX_CONFIGURED_TRANSCODINGS
            || self.max_stream_url_bytes == 0
            || self.max_stream_url_bytes > MAX_CONFIGURED_STREAM_URL_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_bytes > self.http.max_response_bytes
        {
            return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidOptions));
        }
        RemoteHttpRequest::get(format!(
            "{}/resolve",
            self.api_base_url.trim_end_matches('/')
        ))
        .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidOptions))?;
        Ok(())
    }
}

impl fmt::Debug for SoundCloudSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SoundCloudSourceOptions")
            .field("http", &self.http)
            .field("allow_search", &self.allow_search)
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_collection_tracks", &self.max_collection_tracks)
            .field("max_transcodings", &self.max_transcodings)
            .field("max_stream_url_bytes", &self.max_stream_url_bytes)
            .field("max_response_bytes", &self.max_response_bytes)
            .finish_non_exhaustive()
    }
}

#[must_use]
pub fn route_soundcloud_identifier(
    identifier: &str,
    options: &SoundCloudSourceOptions,
) -> Option<SoundCloudRoute> {
    if identifier.is_empty() || identifier.len() > options.max_identifier_bytes {
        return None;
    }
    if let Some(query) = identifier.strip_prefix("scsearch:") {
        let query = query.trim();
        return (options.allow_search
            && !query.is_empty()
            && query.len() <= options.max_metadata_string_bytes)
            .then(|| SoundCloudRoute::Search(query.to_owned()));
    }
    if identifier.contains('#') {
        return None;
    }
    let without_scheme = identifier
        .strip_prefix("https://")
        .or_else(|| identifier.strip_prefix("http://"))
        .unwrap_or(identifier);
    let without_query = without_scheme
        .split_once('?')
        .map_or(without_scheme, |pair| pair.0);
    let (host, path) = without_query.split_once('/')?;
    if host.contains('@')
        || !matches!(
            host,
            "soundcloud.com" | "www.soundcloud.com" | "on.soundcloud.com"
        )
    {
        return None;
    }
    let path = path.trim_matches('/');
    if path.is_empty() || path.len() > options.max_identifier_bytes {
        return None;
    }
    let valid = path
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
    valid.then(|| SoundCloudRoute::Resolve(format!("https://{host}/{path}")))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundCloudPlaybackUrl {
    url: String,
    mime_type: String,
    protocol: String,
}

impl SoundCloudPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }
    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
    #[must_use]
    pub fn protocol(&self) -> &str {
        &self.protocol
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundCloudSourceTrack {
    pub info: TrackInfo,
    pub access: SoundCloudAccess,
    pub playback: Option<SoundCloudPlaybackUrl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoundCloudSourcePlaylist {
    pub name: String,
    pub tracks: Vec<SoundCloudSourceTrack>,
    pub selected_track: Option<usize>,
    pub is_search_result: bool,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub author: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoundCloudSourceItem {
    Track(SoundCloudSourceTrack),
    Playlist(SoundCloudSourcePlaylist),
}

pub struct SoundCloudPlaybackSession {
    session: MediaSession,
}

impl SoundCloudPlaybackSession {
    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        self.session.info()
    }
    /// Decodes one bounded PCM frame.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining credentials.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, SoundCloudPlaybackError> {
        self.session
            .read_pcm(output)
            .map_err(map_playback_media_error)
    }
    /// Seeks the bounded progressive media input.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining credentials.
    pub fn seek(&mut self, requested: Duration) -> Result<SeekResult, SoundCloudPlaybackError> {
        self.session
            .seek(requested)
            .map_err(map_playback_media_error)
    }
}

impl fmt::Debug for SoundCloudPlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SoundCloudPlaybackSession")
            .field("media", self.info())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundCloudPlaybackErrorKind {
    Source(SoundCloudErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoundCloudPlaybackError {
    kind: SoundCloudPlaybackErrorKind,
}

impl SoundCloudPlaybackError {
    const fn new(kind: SoundCloudPlaybackErrorKind) -> Self {
        Self { kind }
    }
    #[must_use]
    pub const fn kind(self) -> SoundCloudPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for SoundCloudPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SoundCloudPlaybackErrorKind::Source(_) => "SoundCloud playback discovery failed",
            SoundCloudPlaybackErrorKind::InvalidOptions => "invalid SoundCloud media policy",
            SoundCloudPlaybackErrorKind::Cancelled => "SoundCloud playback cancelled",
            SoundCloudPlaybackErrorKind::Network => "SoundCloud media request failed",
            SoundCloudPlaybackErrorKind::InvalidMedia => "SoundCloud returned invalid media",
            SoundCloudPlaybackErrorKind::IncompatibleFormat => {
                "SoundCloud media is not a supported audio format"
            }
        })
    }
}
impl std::error::Error for SoundCloudPlaybackError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundCloudErrorKind {
    InvalidOptions,
    InvalidAuthentication,
    Cancelled,
    Shutdown,
    Network,
    RateLimited,
    AuthenticationRequired,
    Unavailable,
    PreviewOnly,
    Blocked,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoundCloudError {
    kind: SoundCloudErrorKind,
}

impl SoundCloudError {
    const fn new(kind: SoundCloudErrorKind) -> Self {
        Self { kind }
    }
    #[must_use]
    pub const fn kind(self) -> SoundCloudErrorKind {
        self.kind
    }
}
impl fmt::Display for SoundCloudError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            SoundCloudErrorKind::InvalidOptions => "invalid SoundCloud source policy",
            SoundCloudErrorKind::InvalidAuthentication => {
                "invalid SoundCloud authentication policy"
            }
            SoundCloudErrorKind::Cancelled => "SoundCloud load cancelled",
            SoundCloudErrorKind::Shutdown => "SoundCloud source is shut down",
            SoundCloudErrorKind::Network => "SoundCloud request failed",
            SoundCloudErrorKind::RateLimited => "SoundCloud rate limit reached",
            SoundCloudErrorKind::AuthenticationRequired => "SoundCloud rejected authentication",
            SoundCloudErrorKind::Unavailable => "SoundCloud content is unavailable",
            SoundCloudErrorKind::PreviewOnly => "SoundCloud content is preview-only",
            SoundCloudErrorKind::Blocked => "SoundCloud content is blocked",
            SoundCloudErrorKind::InvalidResponse => "SoundCloud returned an invalid response",
            SoundCloudErrorKind::UnsupportedRoute => "SoundCloud route is not implemented",
        })
    }
}
impl std::error::Error for SoundCloudError {}

pub struct SoundCloudSourceManager {
    options: SoundCloudSourceOptions,
    authentication: SoundCloudAuthentication,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl SoundCloudSourceManager {
    /// Creates a manager after validating HTTP, parser, and credential limits.
    ///
    /// # Errors
    ///
    /// Returns [`SoundCloudErrorKind::InvalidOptions`] for invalid bounds or HTTP policy.
    pub fn new(
        options: SoundCloudSourceOptions,
        authentication: SoundCloudAuthentication,
    ) -> Result<Self, SoundCloudError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            http,
            shutdown: AtomicBool::new(false),
        })
    }

    #[must_use]
    pub const fn authentication(&self) -> &SoundCloudAuthentication {
        &self.authentication
    }

    /// Loads one validated route into a native `SoundCloud` item.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, rate-limit, or response failures.
    pub fn load_route(
        &self,
        route: &SoundCloudRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<SoundCloudSourceItem>, SoundCloudError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SoundCloudError::new(SoundCloudErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(SoundCloudError::new(SoundCloudErrorKind::Cancelled));
        }
        match route {
            SoundCloudRoute::Resolve(url) => self.resolve(url, cancellation),
            SoundCloudRoute::Search(query) => self.search(query, cancellation),
        }
    }

    /// Resolves one track URL through the bounded control-plane API.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, rate-limit, or response failures.
    pub fn load_track_metadata(
        &self,
        url: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<SoundCloudSourceTrack>, SoundCloudError> {
        match self.resolve(url, cancellation)? {
            Some(SoundCloudSourceItem::Track(track)) => Ok(Some(track)),
            Some(SoundCloudSourceItem::Playlist(_)) => {
                Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))
            }
            None => Ok(None),
        }
    }

    /// Appends the configured client ID to a selected transcoding URL.
    ///
    /// # Errors
    ///
    /// Returns a bounded response failure when the resulting URL exceeds its ceiling.
    pub fn resolve_track_playback(
        &self,
        track: &SoundCloudSourceTrack,
        cancellation: &MediaCancellation,
    ) -> Result<Option<SoundCloudPlaybackUrl>, SoundCloudError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SoundCloudError::new(SoundCloudErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(SoundCloudError::new(SoundCloudErrorKind::Cancelled));
        }
        if track.access == SoundCloudAccess::Blocked {
            return Err(SoundCloudError::new(SoundCloudErrorKind::Blocked));
        }
        if track.access == SoundCloudAccess::Preview {
            return Err(SoundCloudError::new(SoundCloudErrorKind::PreviewOnly));
        }
        let Some(playback) = track.playback.clone() else {
            return Ok(None);
        };
        append_client_id(
            &playback,
            self.authentication.client_id(),
            self.options.max_stream_url_bytes,
        )
        .map(Some)
    }

    /// Opens one progressive `SoundCloud` transcoding through Mantle's bounded media pipeline.
    ///
    /// # Errors
    ///
    /// Returns source, cancellation, network, media, or incompatible-format failures.
    pub fn open_track_playback(
        &self,
        track: &SoundCloudSourceTrack,
        range_options: HttpRangeOptions,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Option<SoundCloudPlaybackSession>, SoundCloudPlaybackError> {
        let Some(playback) = self
            .resolve_track_playback(track, &cancellation)
            .map_err(map_playback_source_error)?
        else {
            return Ok(None);
        };
        if playback.protocol != "progressive" || !playback.mime_type.starts_with("audio/") {
            return Err(SoundCloudPlaybackError::new(
                SoundCloudPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        let input = HttpRangeInput::open_with_cancellation(
            playback.as_str(),
            range_options,
            cancellation.clone(),
        )
        .map_err(map_playback_media_error)?;
        let session =
            MediaSession::open_with_cancellation(Box::new(input), None, media_limits, cancellation)
                .map_err(map_playback_media_error)?;
        if !matches!(
            session.info().container,
            Container::Mp3
                | Container::Mp4
                | Container::Ogg
                | Container::WebM
                | Container::Matroska
        ) || !matches!(
            session.info().codec,
            Codec::Mp3
                | Codec::AacLc
                | Codec::HeAacV1
                | Codec::HeAacV2
                | Codec::Opus
                | Codec::Vorbis
        ) {
            return Err(SoundCloudPlaybackError::new(
                SoundCloudPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(Some(SoundCloudPlaybackSession { session }))
    }

    fn resolve(
        &self,
        url: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<SoundCloudSourceItem>, SoundCloudError> {
        let endpoint = format!(
            "{}/resolve?{}",
            self.options.api_base_url.trim_end_matches('/'),
            form_urlencoded::Serializer::new(String::new())
                .append_pair("url", url)
                .append_pair("client_id", self.authentication.client_id())
                .finish()
        );
        let body = self.get(endpoint, cancellation)?;
        parse_resolved(&body, url, &self.options)
    }

    fn search(
        &self,
        query: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<SoundCloudSourceItem>, SoundCloudError> {
        if !self.options.allow_search
            || query.is_empty()
            || query.len() > self.options.max_metadata_string_bytes
        {
            return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidOptions));
        }
        let query_name = query.to_owned();
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("q", query)
            .append_pair(
                "limit",
                &self.options.max_collection_tracks.min(200).to_string(),
            )
            .append_pair("offset", "0")
            .append_pair("client_id", self.authentication.client_id())
            .finish();
        let body = self.get(
            format!(
                "{}/search/tracks?{encoded}",
                self.options.api_base_url.trim_end_matches('/')
            ),
            cancellation,
        )?;
        parse_search(&body, &query_name, &self.options)
    }

    fn get(
        &self,
        endpoint: String,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, SoundCloudError> {
        let mut request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", "application/json"))
            .and_then(|request| request.header("User-Agent", "Mantle-SoundCloud/1"))
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidOptions))?;
        if let Some(token) = self.authentication.oauth_token.as_deref() {
            request = request
                .header("Authorization", &format!("OAuth {token}"))
                .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidOptions))?;
        }
        self.http
            .execute_with_cancellation(&request, cancellation)
            .map(|response| response.body().to_vec())
            .map_err(map_remote_error)
    }
}

impl fmt::Debug for SoundCloudSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SoundCloudSourceManager")
            .field("options", &self.options)
            .field("authentication", &self.authentication)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<SoundCloudSourceItem> for SoundCloudSourceManager {
    fn source_name(&self) -> &'static str {
        "soundcloud"
    }
    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<SoundCloudSourceItem>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }
    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<SoundCloudSourceItem>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_soundcloud_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = MediaCancellation::linked({
            let cancellation = cancellation.clone();
            move || cancellation.is_cancelled()
        });
        match self.load_route(&route, &linked) {
            Ok(Some(item)) => Ok(Some(SourceLoad::Item(item))),
            Ok(None) => Ok(None),
            Err(error) if error.kind() == SoundCloudErrorKind::Cancelled => Ok(None),
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }
    fn is_encodable(&self, item: &SoundCloudSourceItem) -> bool {
        matches!(item, SoundCloudSourceItem::Track(_))
    }
    fn encode(&self, item: &SoundCloudSourceItem) -> Result<Vec<u8>, SourceRegistryError> {
        matches!(item, SoundCloudSourceItem::Track(_))
            .then(Vec::new)
            .ok_or(SourceRegistryError::NotEncodable)
    }
    fn decode(&self, _payload: &[u8]) -> Result<SoundCloudSourceItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }
    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<SoundCloudSourceItem, SourceRegistryError> {
        if !payload.is_empty()
            || info.identifier.is_empty()
            || info.identifier.len() > self.options.max_identifier_bytes
        {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(SoundCloudSourceItem::Track(SoundCloudSourceTrack {
            info: info.clone(),
            access: SoundCloudAccess::Playable,
            playback: None,
        }))
    }
    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_resolved(
    body: &[u8],
    source_url: &str,
    options: &SoundCloudSourceOptions,
) -> Result<Option<SoundCloudSourceItem>, SoundCloudError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    match value.get("kind").and_then(Value::as_str) {
        Some("track") => {
            Ok(parse_track(&value, source_url, options)?.map(SoundCloudSourceItem::Track))
        }
        Some("playlist" | "system-playlist") => {
            Ok(parse_playlist(&value, false, options)?.map(SoundCloudSourceItem::Playlist))
        }
        Some("error") => Err(SoundCloudError::new(SoundCloudErrorKind::Unavailable)),
        _ => Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse)),
    }
}

fn parse_search(
    body: &[u8],
    query: &str,
    options: &SoundCloudSourceOptions,
) -> Result<Option<SoundCloudSourceItem>, SoundCloudError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    let collection = value
        .get("collection")
        .and_then(Value::as_array)
        .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    if collection.len() > options.max_collection_tracks {
        return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse));
    }
    let mut tracks = Vec::with_capacity(collection.len());
    for value in collection {
        if let Some(track) = parse_track(value, "soundcloud:search", options)?
            && track.access != SoundCloudAccess::Blocked
        {
            tracks.push(track);
        }
    }
    if tracks.is_empty() {
        return Ok(None);
    }
    Ok(Some(SoundCloudSourceItem::Playlist(
        SoundCloudSourcePlaylist {
            name: format!("SoundCloud search: {query}"),
            tracks,
            selected_track: None,
            is_search_result: true,
            uri: None,
            artwork_url: None,
            author: None,
        },
    )))
}

fn parse_playlist(
    value: &Value,
    is_search_result: bool,
    options: &SoundCloudSourceOptions,
) -> Result<Option<SoundCloudSourcePlaylist>, SoundCloudError> {
    let collection = value
        .get("tracks")
        .or_else(|| value.get("collection"))
        .and_then(Value::as_array)
        .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    if collection.len() > options.max_collection_tracks {
        return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse));
    }
    let mut tracks = Vec::with_capacity(collection.len());
    for track in collection {
        if let Some(track) = parse_track(track, "soundcloud:playlist", options)?
            && track.access != SoundCloudAccess::Blocked
        {
            tracks.push(track);
        }
    }
    if tracks.is_empty() {
        return Ok(None);
    }
    let name = bounded_string(
        value.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    Ok(Some(SoundCloudSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result,
        uri: bounded_optional_string(
            value.get("permalink_url").and_then(Value::as_str),
            options.max_metadata_string_bytes,
        )?,
        artwork_url: artwork(value.get("artwork_url").and_then(Value::as_str), options)?,
        author: bounded_optional_string(
            value
                .get("user")
                .and_then(|user| user.get("username"))
                .and_then(Value::as_str),
            options.max_metadata_string_bytes,
        )?,
    }))
}

fn parse_track(
    value: &Value,
    source_url: &str,
    options: &SoundCloudSourceOptions,
) -> Result<Option<SoundCloudSourceTrack>, SoundCloudError> {
    let id = value
        .get("id")
        .and_then(|value| {
            value
                .as_u64()
                .map(|id| id.to_string())
                .or_else(|| value.as_str().map(str::to_owned))
        })
        .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    if id.is_empty() || id.len() > options.max_identifier_bytes {
        return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse));
    }
    let title = bounded_string(
        value.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = bounded_string(
        value
            .get("user")
            .and_then(|user| user.get("username"))
            .and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let duration = value
        .get("duration")
        .and_then(Value::as_u64)
        .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))
        .map(Duration::from_millis)?;
    let permalink = bounded_optional_string(
        value.get("permalink_url").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?
    .or_else(|| Some(source_url.to_owned()));
    let access = match value
        .get("access")
        .and_then(Value::as_str)
        .unwrap_or("playable")
    {
        "playable" => SoundCloudAccess::Playable,
        "preview" => SoundCloudAccess::Preview,
        "blocked" => SoundCloudAccess::Blocked,
        _ => return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse)),
    };
    let playback = parse_playback(value, options)?;
    Ok(Some(SoundCloudSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier: id,
            is_stream: false,
            uri: permalink,
            artwork_url: artwork(value.get("artwork_url").and_then(Value::as_str), options)?,
            isrc: bounded_optional_string(
                value.get("isrc").and_then(Value::as_str),
                options.max_metadata_string_bytes,
            )?,
        },
        access,
        playback,
    }))
}

fn parse_playback(
    value: &Value,
    options: &SoundCloudSourceOptions,
) -> Result<Option<SoundCloudPlaybackUrl>, SoundCloudError> {
    let Some(transcodings) = value
        .get("media")
        .and_then(|media| media.get("transcodings"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    if transcodings.len() > options.max_transcodings {
        return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse));
    }
    let mut fallback = None;
    for value in transcodings {
        let url = bounded_string(
            value.get("url").and_then(Value::as_str),
            options.max_stream_url_bytes,
        )?;
        let format = value
            .get("format")
            .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
        let protocol = bounded_string(format.get("protocol").and_then(Value::as_str), 32)?;
        let mime_type = bounded_string(format.get("mime_type").and_then(Value::as_str), 128)?;
        let candidate = SoundCloudPlaybackUrl {
            url,
            mime_type,
            protocol: protocol.clone(),
        };
        if protocol == "progressive" && candidate.mime_type.starts_with("audio/mpeg") {
            return Ok(Some(candidate));
        }
        fallback.get_or_insert(candidate);
    }
    Ok(fallback)
}

fn artwork(
    value: Option<&str>,
    options: &SoundCloudSourceOptions,
) -> Result<Option<String>, SoundCloudError> {
    let Some(value) = bounded_optional_string(value, options.max_metadata_string_bytes)? else {
        return Ok(None);
    };
    Ok(Some(value.replace("-large", "-t500x500")))
}

fn bounded_string(value: Option<&str>, limit: usize) -> Result<String, SoundCloudError> {
    let value = value.ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))?;
    (!value.is_empty() && value.len() <= limit)
        .then(|| value.to_owned())
        .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))
}
fn bounded_optional_string(
    value: Option<&str>,
    limit: usize,
) -> Result<Option<String>, SoundCloudError> {
    value
        .map(|value| {
            (!value.is_empty() && value.len() <= limit)
                .then(|| value.to_owned())
                .ok_or_else(|| SoundCloudError::new(SoundCloudErrorKind::InvalidResponse))
        })
        .transpose()
}

fn append_client_id(
    playback: &SoundCloudPlaybackUrl,
    client_id: &str,
    limit: usize,
) -> Result<SoundCloudPlaybackUrl, SoundCloudError> {
    let separator = if playback.url.contains('?') { '&' } else { '?' };
    let encoded_client_id: String = form_urlencoded::byte_serialize(client_id.as_bytes()).collect();
    let url = format!("{}{separator}client_id={encoded_client_id}", playback.url);
    if url.len() > limit {
        return Err(SoundCloudError::new(SoundCloudErrorKind::InvalidResponse));
    }
    Ok(SoundCloudPlaybackUrl {
        url,
        mime_type: playback.mime_type.clone(),
        protocol: playback.protocol.clone(),
    })
}

fn map_remote_error(error: crate::RemoteHttpError) -> SoundCloudError {
    SoundCloudError::new(match error.kind() {
        RemoteHttpErrorKind::Cancelled => SoundCloudErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => SoundCloudErrorKind::RateLimited,
        RemoteHttpErrorKind::Unauthorized | RemoteHttpErrorKind::Forbidden => {
            SoundCloudErrorKind::AuthenticationRequired
        }
        RemoteHttpErrorKind::NotFound => SoundCloudErrorKind::Unavailable,
        _ => SoundCloudErrorKind::Network,
    })
}
fn map_playback_source_error(error: SoundCloudError) -> SoundCloudPlaybackError {
    SoundCloudPlaybackError::new(SoundCloudPlaybackErrorKind::Source(error.kind()))
}
fn map_playback_media_error(error: MediaError) -> SoundCloudPlaybackError {
    let kind = match error {
        MediaError::Cancelled => SoundCloudPlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            SoundCloudPlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            SoundCloudPlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => SoundCloudPlaybackErrorKind::Network,
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
        | MediaError::Backend { .. } => SoundCloudPlaybackErrorKind::InvalidMedia,
    };
    SoundCloudPlaybackError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_and_rejects_strict_inputs() {
        let options = SoundCloudSourceOptions::default();
        assert_eq!(
            route_soundcloud_identifier("scsearch: architects", &options),
            Some(SoundCloudRoute::Search("architects".into()))
        );
        assert_eq!(
            route_soundcloud_identifier("https://soundcloud.com/fixture/animals", &options),
            Some(SoundCloudRoute::Resolve(
                "https://soundcloud.com/fixture/animals".into()
            ))
        );
        assert_eq!(
            route_soundcloud_identifier("https://token@soundcloud.com/fixture/animals", &options),
            None
        );
        assert_eq!(
            route_soundcloud_identifier("https://soundcloud.test/fixture/animals", &options),
            None
        );
        assert_eq!(route_soundcloud_identifier("scsearch:   ", &options), None);
    }
}
