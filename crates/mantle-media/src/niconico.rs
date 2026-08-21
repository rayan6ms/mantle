use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aes::Aes128;
use cbc::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use mantle_audio::PcmFrame;
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use serde_json::{Value, json};
use ureq::http::Uri;

use crate::{
    Codec, Container, HttpNetworkAccess, MediaCancellation, MediaError, MediaInfo, MediaLimits,
    MediaSession, MemoryInput, RemoteHttpClient, RemoteHttpError, RemoteHttpErrorKind,
    RemoteHttpOptions, RemoteHttpRequest, RemoteHttpResponse, SeekResult, resolve_http_reference,
};

const DEFAULT_WATCH_API_BASE_URL: &str = "https://www.nicovideo.jp/api/watch";
const DEFAULT_ACCESS_API_BASE_URL: &str = "https://nvapi.nicovideo.jp/v1/watch";
const MAX_SESSION_BYTES: usize = 16 * 1024;
const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_FORMATS: usize = 10_000;
const MAX_CONFIGURED_PLAYLIST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CONFIGURED_PLAYLIST_LINE_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_PLAYLIST_ENTRIES: usize = 100_000;
const MAX_CONFIGURED_MEDIA_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_TOTAL_MEDIA_BYTES: u64 = 512 * 1024 * 1024;
const MAX_CONFIGURED_TRACK_DURATION: Duration = Duration::from_hours(31 * 24);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NicoNicoRoute {
    pub video_id: String,
}

impl NicoNicoRoute {
    #[must_use]
    pub fn canonical_url(&self) -> String {
        format!("https://www.nicovideo.jp/watch/{}", self.video_id)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NicoNicoPlaybackScheme {
    #[default]
    Https,
    /// Permit HTTP only alongside explicit private-network access for deterministic replay.
    HttpForPrivateNetworks,
}

#[derive(Clone, Eq, PartialEq)]
pub struct NicoNicoAuthentication {
    user_session: String,
}

impl NicoNicoAuthentication {
    /// Creates an explicit `user_session` cookie value. Mantle does not automate password or MFA
    /// login and never includes this value in diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`NicoNicoErrorKind::InvalidAuthentication`] for an empty, oversized, or
    /// header-unsafe value.
    pub fn new_user_session(user_session: impl Into<String>) -> Result<Self, NicoNicoError> {
        let user_session = user_session.into();
        if user_session.is_empty()
            || user_session.len() > MAX_SESSION_BYTES
            || !user_session
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b';' | b','))
        {
            return Err(NicoNicoError::new(NicoNicoErrorKind::InvalidAuthentication));
        }
        Ok(Self { user_session })
    }
}

impl fmt::Debug for NicoNicoAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NicoNicoAuthentication")
            .field("configured", &true)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct NicoNicoSourceOptions {
    pub http: RemoteHttpOptions,
    pub watch_api_base_url: String,
    pub access_api_base_url: String,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_response_bytes: u64,
    pub max_formats: usize,
    pub max_playback_url_bytes: usize,
    pub max_playlist_bytes: u64,
    pub max_playlist_line_bytes: usize,
    pub max_playlist_entries: usize,
    pub max_media_resource_bytes: u64,
    pub max_total_media_bytes: u64,
    pub max_track_duration: Duration,
    pub playback_scheme: NicoNicoPlaybackScheme,
}

impl Default for NicoNicoSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            watch_api_base_url: DEFAULT_WATCH_API_BASE_URL.to_owned(),
            access_api_base_url: DEFAULT_ACCESS_API_BASE_URL.to_owned(),
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_response_bytes: 8 * 1024 * 1024,
            max_formats: 128,
            max_playback_url_bytes: 64 * 1024,
            max_playlist_bytes: 2 * 1024 * 1024,
            max_playlist_line_bytes: 64 * 1024,
            max_playlist_entries: 20_000,
            max_media_resource_bytes: 8 * 1024 * 1024,
            max_total_media_bytes: 128 * 1024 * 1024,
            max_track_duration: Duration::from_hours(24),
            playback_scheme: NicoNicoPlaybackScheme::Https,
        }
    }
}

impl NicoNicoSourceOptions {
    fn validate(&self) -> Result<(), NicoNicoError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_bytes > self.http.max_response_bytes
            || self.max_formats == 0
            || self.max_formats > MAX_CONFIGURED_FORMATS
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.watch_api_base_url.len() > self.max_playback_url_bytes
            || self.access_api_base_url.len() > self.max_playback_url_bytes
            || self.max_playlist_bytes == 0
            || self.max_playlist_bytes > MAX_CONFIGURED_PLAYLIST_BYTES
            || self.max_playlist_bytes > self.http.max_response_bytes
            || self.max_playlist_line_bytes == 0
            || self.max_playlist_line_bytes > MAX_CONFIGURED_PLAYLIST_LINE_BYTES
            || self.max_playlist_entries == 0
            || self.max_playlist_entries > MAX_CONFIGURED_PLAYLIST_ENTRIES
            || self.max_media_resource_bytes == 0
            || self.max_media_resource_bytes > MAX_CONFIGURED_MEDIA_RESOURCE_BYTES
            || self.max_media_resource_bytes > self.http.max_response_bytes
            || self.max_total_media_bytes == 0
            || self.max_total_media_bytes > MAX_CONFIGURED_TOTAL_MEDIA_BYTES
            || self.max_track_duration.is_zero()
            || self.max_track_duration > MAX_CONFIGURED_TRACK_DURATION
            || (self.playback_scheme == NicoNicoPlaybackScheme::HttpForPrivateNetworks
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks)
        {
            return Err(NicoNicoError::new(NicoNicoErrorKind::InvalidOptions));
        }
        validate_control_base(
            &self.watch_api_base_url,
            "www.nicovideo.jp",
            self.playback_scheme,
        )?;
        validate_control_base(
            &self.access_api_base_url,
            "nvapi.nicovideo.jp",
            self.playback_scheme,
        )
    }
}

impl fmt::Debug for NicoNicoSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NicoNicoSourceOptions")
            .field("http", &self.http)
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_formats", &self.max_formats)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field("max_playlist_bytes", &self.max_playlist_bytes)
            .field("max_playlist_line_bytes", &self.max_playlist_line_bytes)
            .field("max_playlist_entries", &self.max_playlist_entries)
            .field("max_media_resource_bytes", &self.max_media_resource_bytes)
            .field("max_total_media_bytes", &self.max_total_media_bytes)
            .field("max_track_duration", &self.max_track_duration)
            .field("playback_scheme", &self.playback_scheme)
            .finish_non_exhaustive()
    }
}

#[must_use]
pub fn route_niconico_identifier(
    identifier: &str,
    options: &NicoNicoSourceOptions,
) -> Option<NicoNicoRoute> {
    if identifier.is_empty()
        || identifier.len() > options.max_identifier_bytes
        || identifier.contains('#')
    {
        return None;
    }
    let normalized = if identifier.starts_with("http://") || identifier.starts_with("https://") {
        identifier.to_owned()
    } else {
        format!("https://{identifier}")
    };
    let uri: Uri = normalized.parse().ok()?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) {
        return None;
    }
    let authority = uri.authority()?;
    if authority.as_str().contains('@') || authority.as_str() != authority.host() {
        return None;
    }
    if !matches!(
        authority.host().to_ascii_lowercase().as_str(),
        "nicovideo.jp" | "www.nicovideo.jp" | "sp.nicovideo.jp" | "embed.nicovideo.jp"
    ) {
        return None;
    }
    let segments: Vec<_> = uri.path().trim_matches('/').split('/').collect();
    if segments.len() != 2 || !matches!(segments[0], "watch" | "shorts") {
        return None;
    }
    valid_video_id(segments[1]).then(|| NicoNicoRoute {
        video_id: segments[1].to_owned(),
    })
}

fn valid_video_id(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }
    value.len() >= 3
        && value.as_bytes()[..2].iter().all(u8::is_ascii_lowercase)
        && value.as_bytes()[2..].iter().all(u8::is_ascii_digit)
}

#[derive(Clone, Eq, PartialEq)]
pub struct NicoNicoPlaybackUrl {
    url: String,
    audio_id: String,
}

impl NicoNicoPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }
}

impl fmt::Debug for NicoNicoPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NicoNicoPlaybackUrl")
            .field("url", &"<redacted>")
            .field("audio_selected", &true)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NicoNicoSourceTrack {
    pub info: TrackInfo,
    pub playback_available: bool,
}

pub struct NicoNicoPlaybackSession {
    session: MediaSession,
}

impl NicoNicoPlaybackSession {
    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        self.session.info()
    }

    /// Decodes one bounded PCM frame.
    ///
    /// # Errors
    ///
    /// Returns a stable playback failure without retaining signed URLs or keys.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, NicoNicoPlaybackError> {
        self.session
            .read_pcm(output)
            .map_err(map_playback_media_error)
    }

    /// Seeks the assembled bounded CMAF input.
    ///
    /// # Errors
    ///
    /// Returns a stable playback failure without retaining signed URLs or keys.
    pub fn seek(&mut self, requested: Duration) -> Result<SeekResult, NicoNicoPlaybackError> {
        self.session
            .seek(requested)
            .map_err(map_playback_media_error)
    }
}

impl fmt::Debug for NicoNicoPlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NicoNicoPlaybackSession")
            .field("media", self.info())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NicoNicoPlaybackErrorKind {
    Source(NicoNicoErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    GeoRestricted,
    InvalidPlaylist,
    InvalidMedia,
    IncompatibleFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NicoNicoPlaybackError {
    kind: NicoNicoPlaybackErrorKind,
}

impl NicoNicoPlaybackError {
    const fn new(kind: NicoNicoPlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> NicoNicoPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for NicoNicoPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            NicoNicoPlaybackErrorKind::Source(_) => "NicoNico playback discovery failed",
            NicoNicoPlaybackErrorKind::InvalidOptions => "invalid NicoNico media policy",
            NicoNicoPlaybackErrorKind::Cancelled => "NicoNico playback cancelled",
            NicoNicoPlaybackErrorKind::Network => "NicoNico media request failed",
            NicoNicoPlaybackErrorKind::GeoRestricted => {
                "NicoNico media is unavailable in this region"
            }
            NicoNicoPlaybackErrorKind::InvalidPlaylist => "NicoNico returned an invalid playlist",
            NicoNicoPlaybackErrorKind::InvalidMedia => "NicoNico returned invalid media",
            NicoNicoPlaybackErrorKind::IncompatibleFormat => {
                "NicoNico playback is not supported AAC CMAF"
            }
        })
    }
}

impl std::error::Error for NicoNicoPlaybackError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NicoNicoErrorKind {
    InvalidOptions,
    InvalidAuthentication,
    Cancelled,
    Shutdown,
    Network,
    RateLimited,
    AuthenticationRequired,
    GeoRestricted,
    Unavailable,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NicoNicoError {
    kind: NicoNicoErrorKind,
}

impl NicoNicoError {
    const fn new(kind: NicoNicoErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> NicoNicoErrorKind {
        self.kind
    }
}

impl fmt::Display for NicoNicoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            NicoNicoErrorKind::InvalidOptions => "invalid NicoNico source policy",
            NicoNicoErrorKind::InvalidAuthentication => "invalid NicoNico authentication policy",
            NicoNicoErrorKind::Cancelled => "NicoNico load cancelled",
            NicoNicoErrorKind::Shutdown => "NicoNico source is shut down",
            NicoNicoErrorKind::Network => "NicoNico request failed",
            NicoNicoErrorKind::RateLimited => "NicoNico rate limit reached",
            NicoNicoErrorKind::AuthenticationRequired => "NicoNico rejected authentication",
            NicoNicoErrorKind::GeoRestricted => "NicoNico media is unavailable in this region",
            NicoNicoErrorKind::Unavailable => "NicoNico content is unavailable",
            NicoNicoErrorKind::InvalidResponse => "NicoNico returned an invalid response",
            NicoNicoErrorKind::UnsupportedRoute => "NicoNico route is not implemented",
        })
    }
}

impl std::error::Error for NicoNicoError {}

struct PlaybackContext {
    watch_track_id: String,
    access_right_key: String,
    video_id: String,
    audio_id: String,
}

pub struct NicoNicoSourceManager {
    options: NicoNicoSourceOptions,
    authentication: Option<NicoNicoAuthentication>,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl NicoNicoSourceManager {
    /// Creates an anonymous manager after validating all HTTP and resource bounds.
    ///
    /// # Errors
    ///
    /// Returns [`NicoNicoErrorKind::InvalidOptions`] for an invalid policy.
    pub fn new(options: NicoNicoSourceOptions) -> Result<Self, NicoNicoError> {
        Self::build(options, None)
    }

    /// Creates a manager with an explicit `user_session` cookie.
    ///
    /// # Errors
    ///
    /// Returns [`NicoNicoErrorKind::InvalidOptions`] for an invalid policy.
    pub fn with_authentication(
        options: NicoNicoSourceOptions,
        authentication: NicoNicoAuthentication,
    ) -> Result<Self, NicoNicoError> {
        Self::build(options, Some(authentication))
    }

    fn build(
        options: NicoNicoSourceOptions,
        authentication: Option<NicoNicoAuthentication>,
    ) -> Result<Self, NicoNicoError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| NicoNicoError::new(NicoNicoErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            http,
            shutdown: AtomicBool::new(false),
        })
    }

    #[must_use]
    pub const fn authentication_configured(&self) -> bool {
        self.authentication.is_some()
    }

    /// Loads current watch metadata and availability.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, unavailable, or parser failures.
    pub fn load_route(
        &self,
        route: &NicoNicoRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<NicoNicoSourceTrack>, NicoNicoError> {
        self.load_route_at(route, unix_millis(), cancellation)
    }

    /// Deterministic-clock form of [`Self::load_route`] used by protocol replays.
    ///
    /// # Errors
    ///
    /// Returns the same stable failures as [`Self::load_route`].
    pub fn load_route_at(
        &self,
        route: &NicoNicoRoute,
        unix_millis: u64,
        cancellation: &MediaCancellation,
    ) -> Result<Option<NicoNicoSourceTrack>, NicoNicoError> {
        self.fetch_watch(route, unix_millis, cancellation)
            .map(|watch| watch.map(|(track, _)| track))
    }

    /// Refreshes watch data and exchanges the current access-right key for a signed HLS URL.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, unavailable, or parser failures.
    pub fn resolve_track_playback(
        &self,
        track: &NicoNicoSourceTrack,
        cancellation: &MediaCancellation,
    ) -> Result<Option<NicoNicoPlaybackUrl>, NicoNicoError> {
        self.resolve_track_playback_at(track, unix_millis(), cancellation)
    }

    /// Deterministic-clock form of [`Self::resolve_track_playback`].
    ///
    /// # Errors
    ///
    /// Returns the same stable failures as [`Self::resolve_track_playback`].
    pub fn resolve_track_playback_at(
        &self,
        track: &NicoNicoSourceTrack,
        unix_millis: u64,
        cancellation: &MediaCancellation,
    ) -> Result<Option<NicoNicoPlaybackUrl>, NicoNicoError> {
        if !valid_video_id(&track.info.identifier) {
            return Err(invalid_response());
        }
        let route = NicoNicoRoute {
            video_id: track.info.identifier.clone(),
        };
        let Some((_, context)) = self.fetch_watch(&route, unix_millis, cancellation)? else {
            return Ok(None);
        };
        let Some(context) = context else {
            return Ok(None);
        };
        self.exchange_access_right(&route, &context, cancellation)
            .map(Some)
    }

    /// Opens freshly authorized, bounded AES-128-CBC CMAF audio through Mantle's media pipeline.
    ///
    /// # Errors
    ///
    /// Returns source, cancellation, network, playlist, media, or incompatible-format failures.
    pub fn open_track_playback(
        &self,
        track: &NicoNicoSourceTrack,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Option<NicoNicoPlaybackSession>, NicoNicoPlaybackError> {
        let Some(playback) = self
            .resolve_track_playback(track, &cancellation)
            .map_err(map_playback_source_error)?
        else {
            return Ok(None);
        };
        let bytes = self.load_cmaf_audio(&playback, &cancellation)?;
        let session = MediaSession::open_with_cancellation(
            Box::new(MemoryInput::new(bytes)),
            Some("m4a"),
            media_limits,
            cancellation,
        )
        .map_err(map_playback_media_error)?;
        if session.info().container != Container::Mp4
            || !matches!(
                session.info().codec,
                Codec::AacLc | Codec::HeAacV1 | Codec::HeAacV2
            )
        {
            return Err(NicoNicoPlaybackError::new(
                NicoNicoPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(Some(NicoNicoPlaybackSession { session }))
    }

    fn fetch_watch(
        &self,
        route: &NicoNicoRoute,
        unix_millis: u64,
        cancellation: &MediaCancellation,
    ) -> Result<Option<(NicoNicoSourceTrack, Option<PlaybackContext>)>, NicoNicoError> {
        self.ensure_active(cancellation)?;
        if !valid_video_id(&route.video_id) {
            return Err(NicoNicoError::new(NicoNicoErrorKind::UnsupportedRoute));
        }
        let version = if self.authentication.is_some() {
            "v3"
        } else {
            "v3_guest"
        };
        let endpoint = format!(
            "{}/{version}/{}?actionTrackId=AAAAAAAAAA_{unix_millis}",
            self.options.watch_api_base_url.trim_end_matches('/'),
            route.video_id
        );
        // The watch endpoint uses Apache content negotiation and currently returns 406 for an
        // explicit `Accept: application/json`, despite returning JSON without that header.
        let request = self.control_request(RemoteHttpRequest::get(endpoint), None)?;
        let response = match self.http.execute_with_cancellation(&request, cancellation) {
            Ok(response) => response,
            Err(error) if error.kind() == RemoteHttpErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(map_remote_error(error)),
        };
        let root: Value =
            serde_json::from_slice(response.body()).map_err(|_| invalid_response())?;
        parse_watch(&root, route, &self.options).map(Some)
    }

    fn exchange_access_right(
        &self,
        route: &NicoNicoRoute,
        context: &PlaybackContext,
        cancellation: &MediaCancellation,
    ) -> Result<NicoNicoPlaybackUrl, NicoNicoError> {
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("actionTrackId", &context.watch_track_id)
            .finish();
        let endpoint = format!(
            "{}/{}/access-rights/hls?{query}",
            self.options.access_api_base_url.trim_end_matches('/'),
            route.video_id
        );
        let body = serde_json::to_vec(&json!({
            "outputs": [[context.video_id, context.audio_id]]
        }))
        .map_err(|_| invalid_response())?;
        let request = self
            .control_request(
                RemoteHttpRequest::post(endpoint, body),
                Some("application/json"),
            )?
            .header("Content-Type", "application/json")
            .and_then(|request| request.header("X-Access-Right-Key", &context.access_right_key))
            .and_then(|request| request.header("X-Request-With", "https://www.nicovideo.jp"))
            .map_err(|_| invalid_response())?;
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?;
        let root: Value =
            serde_json::from_slice(response.body()).map_err(|_| invalid_response())?;
        if root
            .get("meta")
            .and_then(|meta| meta.get("status"))
            .and_then(Value::as_u64)
            != Some(201)
        {
            return Err(invalid_response());
        }
        let url = bounded_string(
            root.get("data")
                .and_then(|data| data.get("contentUrl"))
                .and_then(Value::as_str),
            self.options.max_playback_url_bytes,
        )?;
        validate_playback_url(&url, self.options.playback_scheme)?;
        Ok(NicoNicoPlaybackUrl {
            url,
            audio_id: context.audio_id.clone(),
        })
    }

    fn control_request(
        &self,
        request: Result<RemoteHttpRequest, RemoteHttpError>,
        accept: Option<&str>,
    ) -> Result<RemoteHttpRequest, NicoNicoError> {
        let mut request = request
            .and_then(|request| request.header("User-Agent", "Mantle-NicoNico/1"))
            .and_then(|request| request.header("X-Frontend-ID", "6"))
            .and_then(|request| request.header("X-Frontend-Version", "0"))
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| NicoNicoError::new(NicoNicoErrorKind::InvalidOptions))?;
        if let Some(accept) = accept {
            request = request
                .header("Accept", accept)
                .map_err(|_| NicoNicoError::new(NicoNicoErrorKind::InvalidOptions))?;
        }
        if let Some(authentication) = self.authentication.as_ref() {
            request = request
                .header(
                    "Cookie",
                    &format!("user_session={}", authentication.user_session),
                )
                .map_err(|_| NicoNicoError::new(NicoNicoErrorKind::InvalidAuthentication))?;
        }
        Ok(request)
    }

    fn load_cmaf_audio(
        &self,
        playback: &NicoNicoPlaybackUrl,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, NicoNicoPlaybackError> {
        let master = self.fetch_media(
            playback.as_str(),
            self.options.max_playlist_bytes,
            cancellation,
        )?;
        let audio_url = parse_master_audio_url(
            playback.as_str(),
            &master,
            &playback.audio_id,
            &self.options,
        )?;
        let media = self.fetch_media(&audio_url, self.options.max_playlist_bytes, cancellation)?;
        let playlist = parse_media_playlist(&audio_url, &media, &self.options)?;
        let mut keys = BTreeMap::<String, [u8; 16]>::new();
        let capacity = usize::try_from(self.options.max_total_media_bytes)
            .unwrap_or(usize::MAX)
            .min(8 * 1024 * 1024);
        let mut assembled = Vec::with_capacity(capacity);
        self.append_resource(&mut assembled, &playlist.init, &mut keys, cancellation)?;
        for segment in &playlist.segments {
            self.append_resource(&mut assembled, segment, &mut keys, cancellation)?;
        }
        Ok(assembled)
    }

    fn append_resource(
        &self,
        assembled: &mut Vec<u8>,
        resource: &CmafResource,
        keys: &mut BTreeMap<String, [u8; 16]>,
        cancellation: &MediaCancellation,
    ) -> Result<(), NicoNicoPlaybackError> {
        let bytes = self.fetch_media(
            &resource.url,
            self.options.max_media_resource_bytes,
            cancellation,
        )?;
        let decoded = if let Some(encryption) = resource.encryption.as_ref() {
            let key = if let Some(key) = keys.get(&encryption.key_url) {
                *key
            } else {
                if keys.len() >= self.options.max_playlist_entries {
                    return Err(invalid_playlist());
                }
                let key_bytes = self.fetch_media(&encryption.key_url, 16, cancellation)?;
                let key: [u8; 16] = key_bytes.try_into().map_err(|_| invalid_playlist())?;
                keys.insert(encryption.key_url.clone(), key);
                key
            };
            decrypt_aes128_cbc(bytes, key, encryption.iv)?
        } else {
            bytes
        };
        let new_len = assembled
            .len()
            .checked_add(decoded.len())
            .ok_or_else(invalid_playlist)?;
        if u64::try_from(new_len).unwrap_or(u64::MAX) > self.options.max_total_media_bytes {
            return Err(invalid_playlist());
        }
        assembled.extend_from_slice(&decoded);
        Ok(())
    }

    fn fetch_media(
        &self,
        url: &str,
        max_response_bytes: u64,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, NicoNicoPlaybackError> {
        validate_playback_url(url, self.options.playback_scheme).map_err(|_| invalid_playlist())?;
        let request = RemoteHttpRequest::get(url)
            .and_then(|request| request.header("User-Agent", "Mantle-NicoNico/1"))
            .and_then(|request| request.max_response_bytes(max_response_bytes))
            .map_err(|_| NicoNicoPlaybackError::new(NicoNicoPlaybackErrorKind::InvalidOptions))?;
        self.http
            .execute_with_cancellation(&request, cancellation)
            .map(RemoteHttpResponse::into_body)
            .map_err(map_playback_remote_error)
    }

    fn ensure_active(&self, cancellation: &MediaCancellation) -> Result<(), NicoNicoError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(NicoNicoError::new(NicoNicoErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(NicoNicoError::new(NicoNicoErrorKind::Cancelled));
        }
        Ok(())
    }
}

impl fmt::Debug for NicoNicoSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NicoNicoSourceManager")
            .field("options", &self.options)
            .field("authentication_configured", &self.authentication.is_some())
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<NicoNicoSourceTrack> for NicoNicoSourceManager {
    fn source_name(&self) -> &'static str {
        "niconico"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<NicoNicoSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<NicoNicoSourceTrack>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_niconico_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = MediaCancellation::linked({
            let cancellation = cancellation.clone();
            move || cancellation.is_cancelled()
        });
        match self.load_route(&route, &linked) {
            Ok(Some(track)) => Ok(Some(SourceLoad::Item(track))),
            Ok(None) => Ok(None),
            Err(error) if error.kind() == NicoNicoErrorKind::Cancelled => Ok(None),
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, _item: &NicoNicoSourceTrack) -> bool {
        true
    }

    fn encode(&self, _item: &NicoNicoSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(Vec::new())
    }

    fn decode(&self, _payload: &[u8]) -> Result<NicoNicoSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<NicoNicoSourceTrack, SourceRegistryError> {
        let route_matches = info
            .uri
            .as_deref()
            .and_then(|uri| route_niconico_identifier(uri, &self.options))
            .is_some_and(|route| route.video_id == info.identifier);
        if !payload.is_empty() || !valid_video_id(&info.identifier) || !route_matches {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(NicoNicoSourceTrack {
            info: info.clone(),
            playback_available: false,
        })
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_watch(
    root: &Value,
    route: &NicoNicoRoute,
    options: &NicoNicoSourceOptions,
) -> Result<(NicoNicoSourceTrack, Option<PlaybackContext>), NicoNicoError> {
    if root
        .get("meta")
        .and_then(|meta| meta.get("status"))
        .and_then(Value::as_u64)
        != Some(200)
    {
        return Err(invalid_response());
    }
    let data = root.get("data").ok_or_else(invalid_response)?;
    let video = data.get("video").ok_or_else(invalid_response)?;
    let id = bounded_string(
        video.get("id").and_then(Value::as_str),
        options.max_identifier_bytes,
    )?;
    if id != route.video_id || !valid_video_id(&id) {
        return Err(invalid_response());
    }
    let title = bounded_string(
        video.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = bounded_optional_string(
        data.get("owner")
            .and_then(|owner| owner.get("nickname").or_else(|| owner.get("name")))
            .and_then(Value::as_str)
            .or_else(|| {
                data.get("channel")
                    .and_then(|channel| channel.get("name"))
                    .and_then(Value::as_str)
            }),
        options.max_metadata_string_bytes,
    )?
    .unwrap_or_else(|| "Unknown artist".to_owned());
    let duration_seconds = video
        .get("duration")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_response)?;
    let duration = Duration::from_secs(duration_seconds);
    if duration > options.max_track_duration {
        return Err(invalid_response());
    }
    let thumbnail = video.get("thumbnail");
    let artwork_url = bounded_optional_string(
        thumbnail.and_then(|thumbnail| {
            ["largeUrl", "player", "middleUrl", "url"]
                .iter()
                .find_map(|key| thumbnail.get(*key).and_then(Value::as_str))
        }),
        options.max_metadata_string_bytes,
    )?;
    let context = parse_playback_context(data, options)?;
    Ok((
        NicoNicoSourceTrack {
            info: TrackInfo {
                title,
                author,
                duration,
                identifier: id,
                is_stream: false,
                uri: Some(route.canonical_url()),
                artwork_url,
                isrc: None,
            },
            playback_available: context.is_some(),
        },
        context,
    ))
}

fn parse_playback_context(
    data: &Value,
    options: &NicoNicoSourceOptions,
) -> Result<Option<PlaybackContext>, NicoNicoError> {
    let Some(domand) = data.get("media").and_then(|media| media.get("domand")) else {
        return Ok(None);
    };
    let videos = domand
        .get("videos")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    let audios = domand
        .get("audios")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    if videos.len() > options.max_formats || audios.len() > options.max_formats {
        return Err(invalid_response());
    }
    let Some(video_id) = select_format(videos, true, options)? else {
        return Ok(None);
    };
    let Some(audio_id) = select_format(audios, false, options)? else {
        return Ok(None);
    };
    let watch_track_id = bounded_string(
        data.get("client")
            .and_then(|client| client.get("watchTrackId"))
            .and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let access_right_key = bounded_string(
        domand.get("accessRightKey").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    Ok(Some(PlaybackContext {
        watch_track_id,
        access_right_key,
        video_id,
        audio_id,
    }))
}

fn select_format(
    formats: &[Value],
    lowest: bool,
    options: &NicoNicoSourceOptions,
) -> Result<Option<String>, NicoNicoError> {
    let mut selected: Option<(u64, String)> = None;
    for format in formats {
        if !format
            .get("isAvailable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let id = bounded_string(
            format.get("id").and_then(Value::as_str),
            options.max_metadata_string_bytes,
        )?;
        let bitrate = format.get("bitRate").and_then(Value::as_u64).unwrap_or(0);
        let replace = selected.as_ref().is_none_or(|(best, _)| {
            if lowest {
                bitrate < *best
            } else {
                bitrate > *best
            }
        });
        if replace {
            selected = Some((bitrate, id));
        }
    }
    Ok(selected.map(|(_, id)| id))
}

#[derive(Clone)]
struct Encryption {
    key_url: String,
    iv: [u8; 16],
}

#[derive(Clone)]
struct CmafResource {
    url: String,
    encryption: Option<Encryption>,
}

struct CmafPlaylist {
    init: CmafResource,
    segments: Vec<CmafResource>,
}

fn parse_master_audio_url(
    base: &str,
    bytes: &[u8],
    audio_id: &str,
    options: &NicoNicoSourceOptions,
) -> Result<String, NicoNicoPlaybackError> {
    let text = bounded_playlist_text(bytes, options)?;
    if text
        .lines()
        .any(|line| line.trim_start().starts_with("#EXTINF:"))
    {
        return Ok(base.to_owned());
    }
    let mut fallback = None;
    let mut entries = 0;
    for line in text.lines() {
        if !line.starts_with("#EXT-X-MEDIA:") {
            continue;
        }
        entries += 1;
        if entries > options.max_playlist_entries {
            return Err(invalid_playlist());
        }
        let attributes = parse_attributes(line.trim_start_matches("#EXT-X-MEDIA:"), options)?;
        if attributes.get("TYPE").map(String::as_str) != Some("AUDIO") {
            continue;
        }
        let Some(uri) = attributes.get("URI") else {
            continue;
        };
        let resolved = resolve_and_validate(base, uri, options)?;
        if attributes.get("GROUP-ID").map(String::as_str) == Some(audio_id) {
            return Ok(resolved);
        }
        fallback.get_or_insert(resolved);
    }
    fallback.ok_or_else(invalid_playlist)
}

fn parse_media_playlist(
    base: &str,
    bytes: &[u8],
    options: &NicoNicoSourceOptions,
) -> Result<CmafPlaylist, NicoNicoPlaybackError> {
    let text = bounded_playlist_text(bytes, options)?;
    let mut sequence = 0_u64;
    let mut encryption: Option<(String, Option<[u8; 16]>)> = None;
    let mut init = None;
    let mut segments = Vec::new();
    let mut pending_segment = false;
    let mut end_list = false;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            sequence = value.parse().map_err(|_| invalid_playlist())?;
        } else if let Some(value) = line.strip_prefix("#EXT-X-KEY:") {
            let attributes = parse_attributes(value, options)?;
            match attributes.get("METHOD").map(String::as_str) {
                Some("NONE") => encryption = None,
                Some("AES-128") => {
                    let uri = attributes.get("URI").ok_or_else(invalid_playlist)?;
                    let key_url = resolve_and_validate(base, uri, options)?;
                    let iv = attributes.get("IV").map(|iv| parse_iv(iv)).transpose()?;
                    encryption = Some((key_url, iv));
                }
                _ => return Err(invalid_playlist()),
            }
        } else if let Some(value) = line.strip_prefix("#EXT-X-MAP:") {
            if init.is_some() {
                return Err(invalid_playlist());
            }
            let attributes = parse_attributes(value, options)?;
            if attributes.contains_key("BYTERANGE") {
                return Err(invalid_playlist());
            }
            let uri = attributes.get("URI").ok_or_else(invalid_playlist)?;
            let url = resolve_and_validate(base, uri, options)?;
            let resource_encryption = encryption
                .as_ref()
                .map(|(key_url, iv)| {
                    Ok(Encryption {
                        key_url: key_url.clone(),
                        iv: iv.ok_or_else(invalid_playlist)?,
                    })
                })
                .transpose()?;
            init = Some(CmafResource {
                url,
                encryption: resource_encryption,
            });
        } else if line.starts_with("#EXTINF:") {
            if pending_segment {
                return Err(invalid_playlist());
            }
            pending_segment = true;
        } else if line == "#EXT-X-ENDLIST" {
            end_list = true;
        } else if line.starts_with("#EXT-X-BYTERANGE:") || line == "#EXT-X-DISCONTINUITY" {
            return Err(invalid_playlist());
        } else if !line.is_empty() && !line.starts_with('#') {
            if !pending_segment || segments.len() >= options.max_playlist_entries {
                return Err(invalid_playlist());
            }
            let url = resolve_and_validate(base, line, options)?;
            let resource_encryption = encryption.as_ref().map(|(key_url, iv)| Encryption {
                key_url: key_url.clone(),
                iv: iv.unwrap_or_else(|| sequence_iv(sequence)),
            });
            segments.push(CmafResource {
                url,
                encryption: resource_encryption,
            });
            sequence = sequence.checked_add(1).ok_or_else(invalid_playlist)?;
            pending_segment = false;
        }
    }
    if pending_segment || !end_list || segments.is_empty() {
        return Err(invalid_playlist());
    }
    Ok(CmafPlaylist {
        init: init.ok_or_else(invalid_playlist)?,
        segments,
    })
}

fn bounded_playlist_text<'a>(
    bytes: &'a [u8],
    options: &NicoNicoSourceOptions,
) -> Result<&'a str, NicoNicoPlaybackError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > options.max_playlist_bytes
        || bytes
            .split(|byte| *byte == b'\n')
            .any(|line| line.len() > options.max_playlist_line_bytes)
    {
        return Err(invalid_playlist());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_playlist())?;
    if !text.starts_with("#EXTM3U") {
        return Err(invalid_playlist());
    }
    Ok(text)
}

fn parse_attributes(
    input: &str,
    options: &NicoNicoSourceOptions,
) -> Result<BTreeMap<String, String>, NicoNicoPlaybackError> {
    let mut attributes = BTreeMap::new();
    let mut start = 0;
    let mut quoted = false;
    let bytes = input.as_bytes();
    for index in 0..=bytes.len() {
        let at_end = index == bytes.len();
        if !at_end && bytes[index] == b'"' {
            quoted = !quoted;
        }
        if at_end || (bytes[index] == b',' && !quoted) {
            let field = input[start..index].trim();
            let (name, value) = field.split_once('=').ok_or_else(invalid_playlist)?;
            if attributes.len() >= options.max_playlist_entries
                || name.is_empty()
                || value.len() > options.max_playlist_line_bytes
            {
                return Err(invalid_playlist());
            }
            let value = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                &value[1..value.len() - 1]
            } else {
                value
            };
            if attributes
                .insert(name.to_owned(), value.to_owned())
                .is_some()
            {
                return Err(invalid_playlist());
            }
            start = index.saturating_add(1);
        }
    }
    if quoted {
        return Err(invalid_playlist());
    }
    Ok(attributes)
}

fn parse_iv(value: &str) -> Result<[u8; 16], NicoNicoPlaybackError> {
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .ok_or_else(invalid_playlist)?;
    if hex.len() != 32 {
        return Err(invalid_playlist());
    }
    let mut iv = [0_u8; 16];
    for (index, byte) in iv.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|_| invalid_playlist())?;
    }
    Ok(iv)
}

fn sequence_iv(sequence: u64) -> [u8; 16] {
    let mut iv = [0_u8; 16];
    iv[8..].copy_from_slice(&sequence.to_be_bytes());
    iv
}

fn decrypt_aes128_cbc(
    mut bytes: Vec<u8>,
    key: [u8; 16],
    iv: [u8; 16],
) -> Result<Vec<u8>, NicoNicoPlaybackError> {
    let plaintext = cbc::Decryptor::<Aes128>::new((&key).into(), (&iv).into())
        .decrypt_padded::<Pkcs7>(&mut bytes)
        .map_err(|_| invalid_playlist())?;
    let length = plaintext.len();
    bytes.truncate(length);
    Ok(bytes)
}

fn resolve_and_validate(
    base: &str,
    reference: &str,
    options: &NicoNicoSourceOptions,
) -> Result<String, NicoNicoPlaybackError> {
    let resolved = resolve_http_reference(base, reference).map_err(|_| invalid_playlist())?;
    if resolved.len() > options.max_playback_url_bytes {
        return Err(invalid_playlist());
    }
    validate_playback_url(&resolved, options.playback_scheme).map_err(|_| invalid_playlist())?;
    Ok(resolved)
}

fn validate_control_base(
    value: &str,
    expected_host: &str,
    scheme: NicoNicoPlaybackScheme,
) -> Result<(), NicoNicoError> {
    let uri: Uri = value
        .parse()
        .map_err(|_| NicoNicoError::new(NicoNicoErrorKind::InvalidOptions))?;
    let authority = uri
        .authority()
        .ok_or_else(|| NicoNicoError::new(NicoNicoErrorKind::InvalidOptions))?;
    let valid = match scheme {
        NicoNicoPlaybackScheme::Https => {
            uri.scheme_str() == Some("https")
                && authority.host().eq_ignore_ascii_case(expected_host)
                && authority.as_str() == authority.host()
        }
        NicoNicoPlaybackScheme::HttpForPrivateNetworks => {
            matches!(uri.scheme_str(), Some("http" | "https")) && !authority.as_str().contains('@')
        }
    };
    if !valid || value.contains('#') || value.contains('?') {
        return Err(NicoNicoError::new(NicoNicoErrorKind::InvalidOptions));
    }
    Ok(())
}

fn validate_playback_url(value: &str, scheme: NicoNicoPlaybackScheme) -> Result<(), NicoNicoError> {
    let uri: Uri = value.parse().map_err(|_| invalid_response())?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    let host = authority.host().to_ascii_lowercase();
    let valid = match scheme {
        NicoNicoPlaybackScheme::Https => {
            uri.scheme_str() == Some("https")
                && (host == "domand.nicovideo.jp" || host.ends_with(".domand.nicovideo.jp"))
                && authority.as_str() == authority.host()
        }
        NicoNicoPlaybackScheme::HttpForPrivateNetworks => {
            matches!(uri.scheme_str(), Some("http" | "https"))
        }
    };
    if !valid || authority.as_str().contains('@') || value.contains('#') {
        return Err(invalid_response());
    }
    Ok(())
}

fn bounded_string(value: Option<&str>, limit: usize) -> Result<String, NicoNicoError> {
    value
        .filter(|value| !value.is_empty() && value.len() <= limit)
        .map(str::to_owned)
        .ok_or_else(invalid_response)
}

fn bounded_optional_string(
    value: Option<&str>,
    limit: usize,
) -> Result<Option<String>, NicoNicoError> {
    value
        .map(|value| bounded_string(Some(value), limit))
        .transpose()
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn invalid_response() -> NicoNicoError {
    NicoNicoError::new(NicoNicoErrorKind::InvalidResponse)
}

fn invalid_playlist() -> NicoNicoPlaybackError {
    NicoNicoPlaybackError::new(NicoNicoPlaybackErrorKind::InvalidPlaylist)
}

fn map_remote_error(error: RemoteHttpError) -> NicoNicoError {
    let kind = match error.kind() {
        RemoteHttpErrorKind::InvalidOptions | RemoteHttpErrorKind::InvalidRequest => {
            NicoNicoErrorKind::InvalidOptions
        }
        RemoteHttpErrorKind::Cancelled => NicoNicoErrorKind::Cancelled,
        RemoteHttpErrorKind::Unauthorized => NicoNicoErrorKind::AuthenticationRequired,
        RemoteHttpErrorKind::Forbidden => NicoNicoErrorKind::GeoRestricted,
        RemoteHttpErrorKind::NotFound => NicoNicoErrorKind::Unavailable,
        RemoteHttpErrorKind::RateLimited => NicoNicoErrorKind::RateLimited,
        RemoteHttpErrorKind::RequestTooLarge
        | RemoteHttpErrorKind::ResponseTooLarge
        | RemoteHttpErrorKind::InvalidResponse => NicoNicoErrorKind::InvalidResponse,
        RemoteHttpErrorKind::DestinationDenied
        | RemoteHttpErrorKind::Timeout
        | RemoteHttpErrorKind::Transport
        | RemoteHttpErrorKind::ServerUnavailable
        | RemoteHttpErrorKind::HttpStatus => NicoNicoErrorKind::Network,
    };
    NicoNicoError::new(kind)
}

fn map_playback_source_error(error: NicoNicoError) -> NicoNicoPlaybackError {
    let kind = if error.kind() == NicoNicoErrorKind::Cancelled {
        NicoNicoPlaybackErrorKind::Cancelled
    } else {
        NicoNicoPlaybackErrorKind::Source(error.kind())
    };
    NicoNicoPlaybackError::new(kind)
}

fn map_playback_remote_error(error: RemoteHttpError) -> NicoNicoPlaybackError {
    let kind = match error.kind() {
        RemoteHttpErrorKind::InvalidOptions | RemoteHttpErrorKind::InvalidRequest => {
            NicoNicoPlaybackErrorKind::InvalidOptions
        }
        RemoteHttpErrorKind::Cancelled => NicoNicoPlaybackErrorKind::Cancelled,
        RemoteHttpErrorKind::RequestTooLarge
        | RemoteHttpErrorKind::ResponseTooLarge
        | RemoteHttpErrorKind::InvalidResponse => NicoNicoPlaybackErrorKind::InvalidPlaylist,
        RemoteHttpErrorKind::Forbidden => NicoNicoPlaybackErrorKind::GeoRestricted,
        RemoteHttpErrorKind::DestinationDenied
        | RemoteHttpErrorKind::Timeout
        | RemoteHttpErrorKind::Transport
        | RemoteHttpErrorKind::Unauthorized
        | RemoteHttpErrorKind::NotFound
        | RemoteHttpErrorKind::RateLimited
        | RemoteHttpErrorKind::ServerUnavailable
        | RemoteHttpErrorKind::HttpStatus => NicoNicoPlaybackErrorKind::Network,
    };
    NicoNicoPlaybackError::new(kind)
}

fn map_playback_media_error(error: MediaError) -> NicoNicoPlaybackError {
    let kind = match error {
        MediaError::Cancelled => NicoNicoPlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            NicoNicoPlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            NicoNicoPlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => NicoNicoPlaybackErrorKind::Network,
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
        | MediaError::Backend { .. } => NicoNicoPlaybackErrorKind::InvalidMedia,
    };
    NicoNicoPlaybackError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_iv_places_the_media_sequence_in_the_low_64_bits() {
        assert_eq!(
            sequence_iv(0x0102_0304_0506_0708),
            [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn playlist_and_origin_failures_are_bounded_and_explicit() {
        let mut options = NicoNicoSourceOptions {
            http: RemoteHttpOptions {
                network_access: HttpNetworkAccess::AllowPrivateNetworks,
                ..RemoteHttpOptions::default()
            },
            playback_scheme: NicoNicoPlaybackScheme::HttpForPrivateNetworks,
            ..NicoNicoSourceOptions::default()
        };
        assert!(
            parse_media_playlist(
                "http://127.0.0.1/audio.m3u8",
                b"#EXTM3U\n#EXT-X-MAP:URI=\"init.cmfa\"\n#EXTINF:1,\nsegment.cmfa\n",
                &options,
            )
            .is_err()
        );
        assert!(decrypt_aes128_cbc(vec![0; 15], [0; 16], [0; 16]).is_err());
        assert!(
            validate_playback_url(
                "https://delivery.domand.nicovideo.jp.evil.test/master.m3u8",
                NicoNicoPlaybackScheme::Https,
            )
            .is_err()
        );

        options.max_playlist_entries = 0;
        assert_eq!(
            NicoNicoSourceManager::new(options).unwrap_err().kind(),
            NicoNicoErrorKind::InvalidOptions
        );
    }
}
