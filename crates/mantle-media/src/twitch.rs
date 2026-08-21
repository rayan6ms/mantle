use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mantle_audio::EncodedFrameSlot;
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use serde_json::{Value, json};
use ureq::http::Uri;

use crate::{
    HlsError, HlsPlaylist, HttpNetworkAccess, MediaCancellation, MediaError, RemoteHttpClient,
    RemoteHttpErrorKind, RemoteHttpOptions, RemoteHttpRequest, RemoteRetryMode,
    YoutubeLivePlaybackOptions, YoutubeLivePlaybackPoll, YoutubeLivePlaybackSession,
    YoutubePlaybackError, YoutubePlaybackErrorKind, load_http_hls_playlist_with_cancellation,
};

const DEFAULT_HELIX_BASE_URL: &str = "https://api.twitch.tv/helix";
const DEFAULT_GQL_URL: &str = "https://gql.twitch.tv/gql";
const DEFAULT_USHER_BASE_URL: &str = "https://usher.ttvnw.net/api/channel/hls";
const MAX_CLIENT_ID_BYTES: usize = 256;
const MAX_ACCESS_TOKEN_BYTES: usize = 16 * 1024;
const MAX_DEVICE_ID_BYTES: usize = 1024;
const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_PLAYBACK_TOKEN_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_SIGNATURE_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_PLAYBACK_URL_BYTES: usize = 2 * 1024 * 1024;
const MAX_CHANNEL_BYTES: usize = 25;
const PLAYBACK_ACCESS_TOKEN_QUERY: &str = "query PlaybackAccessToken_Template($login:String!,$isLive:Boolean!,$vodID:ID!,$isVod:Boolean!,$playerType:String!){streamPlaybackAccessToken(channelName:$login,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:$playerType})@include(if:$isLive){value signature __typename}videoPlaybackAccessToken(id:$vodID,params:{platform:\"web\",playerBackend:\"mediaplayer\",playerType:$playerType})@include(if:$isVod){value signature __typename}}";

/// A strict current Twitch live-channel route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwitchRoute {
    pub channel: String,
}

impl TwitchRoute {
    #[must_use]
    pub fn canonical_url(&self) -> String {
        format!("https://www.twitch.tv/{}", self.channel)
    }
}

/// Scheme policy for expiring Twitch Usher and HLS URLs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TwitchPlaybackScheme {
    #[default]
    Https,
    /// Permit HTTP only alongside the explicit private-network source policy.
    ///
    /// This exists for trusted loopback replay and must not be used for public service traffic.
    HttpForPrivateNetworks,
}

/// Explicit Twitch API and playback client credentials.
#[derive(Clone, Eq, PartialEq)]
pub struct TwitchAuthentication {
    client_id: String,
    access_token: String,
    device_id: Option<String>,
}

impl TwitchAuthentication {
    /// Creates a client-ID and OAuth access-token policy.
    ///
    /// # Errors
    ///
    /// Returns [`TwitchErrorKind::InvalidAuthentication`] when either value is empty, oversized,
    /// or unsafe for an HTTP header.
    pub fn new(
        client_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Result<Self, TwitchError> {
        Self::with_device_id(client_id, access_token, None::<String>)
    }

    /// Creates a credential policy with an optional caller-owned playback device ID.
    ///
    /// # Errors
    ///
    /// Returns [`TwitchErrorKind::InvalidAuthentication`] when any configured value violates its
    /// size or header-safe character policy.
    pub fn with_device_id(
        client_id: impl Into<String>,
        access_token: impl Into<String>,
        device_id: Option<impl Into<String>>,
    ) -> Result<Self, TwitchError> {
        let client_id = client_id.into();
        let access_token = access_token.into();
        let device_id = device_id.map(Into::into);
        if !valid_header_credential(&client_id, MAX_CLIENT_ID_BYTES)
            || !valid_header_credential(&access_token, MAX_ACCESS_TOKEN_BYTES)
            || device_id
                .as_deref()
                .is_some_and(|value| !valid_header_credential(value, MAX_DEVICE_ID_BYTES))
        {
            return Err(TwitchError::new(TwitchErrorKind::InvalidAuthentication));
        }
        Ok(Self {
            client_id,
            access_token,
            device_id,
        })
    }

    #[must_use]
    pub const fn device_id_configured(&self) -> bool {
        self.device_id.is_some()
    }
}

fn valid_header_credential(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && value.bytes().all(|byte| byte.is_ascii_graphic())
}

impl fmt::Debug for TwitchAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TwitchAuthentication")
            .field("client_id", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("device_id_configured", &self.device_id.is_some())
            .finish()
    }
}

/// Bounded Twitch metadata, compatibility-query, and signed-URL policy.
#[derive(Clone, Eq, PartialEq)]
pub struct TwitchSourceOptions {
    pub http: RemoteHttpOptions,
    pub helix_base_url: String,
    pub gql_url: String,
    pub usher_base_url: String,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_response_bytes: u64,
    pub max_gql_request_bytes: usize,
    pub max_playback_token_bytes: usize,
    pub max_signature_bytes: usize,
    pub max_playback_url_bytes: usize,
    pub playback_scheme: TwitchPlaybackScheme,
}

impl Default for TwitchSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            helix_base_url: DEFAULT_HELIX_BASE_URL.to_owned(),
            gql_url: DEFAULT_GQL_URL.to_owned(),
            usher_base_url: DEFAULT_USHER_BASE_URL.to_owned(),
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_response_bytes: 2 * 1024 * 1024,
            max_gql_request_bytes: 64 * 1024,
            max_playback_token_bytes: 256 * 1024,
            max_signature_bytes: 16 * 1024,
            max_playback_url_bytes: 512 * 1024,
            playback_scheme: TwitchPlaybackScheme::Https,
        }
    }
}

impl TwitchSourceOptions {
    fn validate(&self) -> Result<(), TwitchError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_bytes > self.http.max_response_bytes
            || self.max_gql_request_bytes == 0
            || self.max_gql_request_bytes > MAX_CONFIGURED_REQUEST_BYTES
            || self.max_playback_token_bytes == 0
            || self.max_playback_token_bytes > MAX_CONFIGURED_PLAYBACK_TOKEN_BYTES
            || self.max_signature_bytes == 0
            || self.max_signature_bytes > MAX_CONFIGURED_SIGNATURE_BYTES
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_PLAYBACK_URL_BYTES
            || (self.playback_scheme == TwitchPlaybackScheme::HttpForPrivateNetworks
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks)
        {
            return Err(TwitchError::new(TwitchErrorKind::InvalidOptions));
        }
        validate_base_url(&self.helix_base_url, &self.http)?;
        validate_base_url(&self.gql_url, &self.http)?;
        validate_base_url(&self.usher_base_url, &self.http)
    }
}

impl fmt::Debug for TwitchSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TwitchSourceOptions")
            .field("http", &self.http)
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_gql_request_bytes", &self.max_gql_request_bytes)
            .field("max_playback_token_bytes", &self.max_playback_token_bytes)
            .field("max_signature_bytes", &self.max_signature_bytes)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field("playback_scheme", &self.playback_scheme)
            .finish_non_exhaustive()
    }
}

/// Routes one strict live-channel URL without network access.
#[must_use]
pub fn route_twitch_identifier(
    identifier: &str,
    options: &TwitchSourceOptions,
) -> Option<TwitchRoute> {
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
    if !matches!(
        authority.host().to_ascii_lowercase().as_str(),
        "twitch.tv" | "www.twitch.tv" | "go.twitch.tv" | "m.twitch.tv"
    ) {
        return None;
    }
    let channel = uri.path().trim_matches('/').to_ascii_lowercase();
    if !valid_channel(&channel) || reserved_path(&channel) {
        return None;
    }
    Some(TwitchRoute { channel })
}

fn valid_channel(channel: &str) -> bool {
    !channel.is_empty()
        && channel.len() <= MAX_CHANNEL_BYTES
        && channel
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn reserved_path(channel: &str) -> bool {
    matches!(
        channel,
        "directory"
            | "downloads"
            | "inventory"
            | "jobs"
            | "login"
            | "p"
            | "products"
            | "search"
            | "settings"
            | "subscriptions"
            | "turbo"
            | "videos"
            | "wallet"
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwitchSourceTrack {
    pub info: TrackInfo,
    pub channel: String,
}

/// A signed Twitch Usher master-playlist URL. Its diagnostics always redact the value.
#[derive(Clone, Eq, PartialEq)]
pub struct TwitchPlaybackUrl {
    url: String,
}

impl TwitchPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }
}

impl fmt::Debug for TwitchPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TwitchPlaybackUrl(<redacted>)")
    }
}

/// Twitch live playback uses the same bounded MPEG-TS/ADTS policy as `YouTube` live playback.
pub type TwitchLivePlaybackOptions = YoutubeLivePlaybackOptions;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwitchLivePlaybackPoll {
    Frame,
    WaitUntil(Duration),
    Ended,
    Exhausted,
}

pub struct TwitchLivePlaybackSession {
    inner: YoutubeLivePlaybackSession,
}

impl TwitchLivePlaybackSession {
    /// Produces one frame or deterministic reload/terminal outcome at monotonic `now`.
    ///
    /// # Errors
    ///
    /// Returns credential-safe cancellation, network, HLS, MPEG-TS, media, or audio failures.
    pub fn poll_frame(
        &mut self,
        now: Duration,
        output: &mut EncodedFrameSlot,
    ) -> Result<TwitchLivePlaybackPoll, TwitchPlaybackError> {
        self.inner
            .poll_frame(now, output)
            .map(map_live_poll)
            .map_err(map_youtube_playback_error)
    }
}

impl fmt::Debug for TwitchLivePlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TwitchLivePlaybackSession")
            .field("manifest", &"<redacted>")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwitchPlaybackErrorKind {
    Source(TwitchErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
    AudioPipeline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TwitchPlaybackError {
    kind: TwitchPlaybackErrorKind,
}

impl TwitchPlaybackError {
    const fn new(kind: TwitchPlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> TwitchPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for TwitchPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            TwitchPlaybackErrorKind::Source(_) => "Twitch playback discovery failed",
            TwitchPlaybackErrorKind::InvalidOptions => "invalid Twitch media policy",
            TwitchPlaybackErrorKind::Cancelled => "Twitch playback cancelled",
            TwitchPlaybackErrorKind::Network => "Twitch media request failed",
            TwitchPlaybackErrorKind::InvalidMedia => "Twitch returned invalid HLS media",
            TwitchPlaybackErrorKind::IncompatibleFormat => {
                "Twitch media is not compatible MPEG-TS/AAC HLS"
            }
            TwitchPlaybackErrorKind::AudioPipeline => "Twitch audio processing failed",
        })
    }
}

impl std::error::Error for TwitchPlaybackError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TwitchErrorKind {
    InvalidOptions,
    InvalidAuthentication,
    Cancelled,
    Shutdown,
    Network,
    RateLimited,
    AuthenticationRequired,
    Offline,
    Unavailable,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TwitchError {
    kind: TwitchErrorKind,
}

impl TwitchError {
    const fn new(kind: TwitchErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> TwitchErrorKind {
        self.kind
    }
}

impl fmt::Display for TwitchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            TwitchErrorKind::InvalidOptions => "invalid Twitch source policy",
            TwitchErrorKind::InvalidAuthentication => "invalid Twitch authentication policy",
            TwitchErrorKind::Cancelled => "Twitch load cancelled",
            TwitchErrorKind::Shutdown => "Twitch source is shut down",
            TwitchErrorKind::Network => "Twitch request failed",
            TwitchErrorKind::RateLimited => "Twitch rate limit reached",
            TwitchErrorKind::AuthenticationRequired => "Twitch rejected authentication",
            TwitchErrorKind::Offline => "Twitch channel is offline",
            TwitchErrorKind::Unavailable => "Twitch channel is unavailable",
            TwitchErrorKind::InvalidResponse => "Twitch returned an invalid response",
            TwitchErrorKind::UnsupportedRoute => "Twitch route is not implemented",
        })
    }
}

impl std::error::Error for TwitchError {}

pub struct TwitchSourceManager {
    options: TwitchSourceOptions,
    authentication: TwitchAuthentication,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl TwitchSourceManager {
    /// Creates a manager after validating HTTP, credentials, parser, and signed-URL limits.
    ///
    /// # Errors
    ///
    /// Returns [`TwitchErrorKind::InvalidOptions`] for invalid bounds or HTTP policy.
    pub fn new(
        options: TwitchSourceOptions,
        authentication: TwitchAuthentication,
    ) -> Result<Self, TwitchError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            http,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Loads supported Helix metadata for one live channel.
    ///
    /// # Errors
    ///
    /// Returns [`TwitchErrorKind::Offline`] for an empty stream result, plus stable
    /// cancellation, authentication, rate-limit, network, and parser failures.
    pub fn load_route(
        &self,
        route: &TwitchRoute,
        cancellation: &MediaCancellation,
    ) -> Result<TwitchSourceTrack, TwitchError> {
        self.ensure_active(cancellation)?;
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("user_login", &route.channel)
            .append_pair("first", "1")
            .finish();
        let endpoint = format!(
            "{}/streams?{query}",
            self.options.helix_base_url.trim_end_matches('/')
        );
        let request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", "application/json"))
            .and_then(|request| request.header("Client-Id", &self.authentication.client_id))
            .and_then(|request| {
                request.header(
                    "Authorization",
                    &format!("Bearer {}", self.authentication.access_token),
                )
            })
            .and_then(|request| request.header("User-Agent", "Mantle-Twitch/1"))
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidAuthentication))?;
        let body = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?
            .body()
            .to_vec();
        parse_helix_stream(&body, route, &self.options)
    }

    /// Resolves a fresh signed Twitch Usher master-playlist URL.
    ///
    /// # Errors
    ///
    /// Returns offline, cancellation, rate-limit, network, or bounded compatibility-query errors.
    pub fn resolve_playback(
        &self,
        channel: &str,
        cancellation: &MediaCancellation,
    ) -> Result<TwitchPlaybackUrl, TwitchError> {
        self.ensure_active(cancellation)?;
        if !valid_channel(channel) {
            return Err(TwitchError::new(TwitchErrorKind::UnsupportedRoute));
        }
        let body = serde_json::to_vec(&json!({
            "operationName": "PlaybackAccessToken_Template",
            "query": PLAYBACK_ACCESS_TOKEN_QUERY,
            "variables": {
                "isLive": true,
                "login": channel,
                "isVod": false,
                "vodID": "",
                "playerType": "site"
            }
        }))
        .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidOptions))?;
        if body.len() > self.options.max_gql_request_bytes {
            return Err(TwitchError::new(TwitchErrorKind::InvalidOptions));
        }
        let mut request = RemoteHttpRequest::post(&self.options.gql_url, body)
            .and_then(|request| request.header("Accept", "application/json"))
            .and_then(|request| request.header("Content-Type", "application/json"))
            .and_then(|request| request.header("Client-Id", &self.authentication.client_id))
            .and_then(|request| request.header("User-Agent", "Mantle-Twitch/1"))
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidAuthentication))?
            .retry_mode(RemoteRetryMode::Idempotent);
        if let Some(device_id) = self.authentication.device_id.as_deref() {
            request = request
                .header("X-Device-Id", device_id)
                .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidAuthentication))?;
        }
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?;
        parse_playback_token(response.body(), channel, &self.options)
    }

    /// Resolves the signed master, selects the lowest-bandwidth variant, and opens bounded live
    /// MPEG-TS/AAC playback.
    ///
    /// # Errors
    ///
    /// Returns source, cancellation, network, HLS, MPEG-TS, media, or audio-pipeline failures.
    pub fn open_live_playback(
        &self,
        track: &TwitchSourceTrack,
        options: TwitchLivePlaybackOptions,
        cancellation: MediaCancellation,
    ) -> Result<TwitchLivePlaybackSession, TwitchPlaybackError> {
        let playback = self
            .resolve_playback(&track.channel, &cancellation)
            .map_err(map_playback_source_error)?;
        let playlist = load_http_hls_playlist_with_cancellation(
            playback.as_str(),
            options.playlist,
            options.hls,
            cancellation.clone(),
        )
        .map_err(|error| map_hls_error(&error))?;
        let media_url = match playlist {
            HlsPlaylist::Master(master) => master
                .variants
                .iter()
                .min_by_key(|variant| variant.bandwidth.unwrap_or(u64::MAX))
                .map(|variant| variant.uri.clone())
                .ok_or_else(|| TwitchPlaybackError::new(TwitchPlaybackErrorKind::InvalidMedia))?,
            HlsPlaylist::Media(_) => playback.as_str().to_owned(),
        };
        let inner = YoutubeLivePlaybackSession::open_hls_manifest(media_url, options, cancellation)
            .map_err(map_youtube_playback_error)?;
        Ok(TwitchLivePlaybackSession { inner })
    }

    fn ensure_active(&self, cancellation: &MediaCancellation) -> Result<(), TwitchError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(TwitchError::new(TwitchErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(TwitchError::new(TwitchErrorKind::Cancelled));
        }
        Ok(())
    }
}

impl fmt::Debug for TwitchSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TwitchSourceManager")
            .field("options", &self.options)
            .field("authentication", &self.authentication)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<TwitchSourceTrack> for TwitchSourceManager {
    fn source_name(&self) -> &'static str {
        "twitch"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<TwitchSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<TwitchSourceTrack>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_twitch_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = MediaCancellation::linked({
            let cancellation = cancellation.clone();
            move || cancellation.is_cancelled()
        });
        match self.load_route(&route, &linked) {
            Ok(track) => Ok(Some(SourceLoad::Item(track))),
            Err(error)
                if matches!(
                    error.kind(),
                    TwitchErrorKind::Cancelled | TwitchErrorKind::Offline
                ) =>
            {
                Ok(None)
            }
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, _item: &TwitchSourceTrack) -> bool {
        true
    }

    fn encode(&self, _item: &TwitchSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(Vec::new())
    }

    fn decode(&self, _payload: &[u8]) -> Result<TwitchSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<TwitchSourceTrack, SourceRegistryError> {
        let Some(route) = route_twitch_identifier(&info.identifier, &self.options) else {
            return Err(SourceRegistryError::SourceFailure);
        };
        if !payload.is_empty() || !info.is_stream {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(TwitchSourceTrack {
            info: info.clone(),
            channel: route.channel,
        })
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_helix_stream(
    body: &[u8],
    route: &TwitchRoute,
    options: &TwitchSourceOptions,
) -> Result<TwitchSourceTrack, TwitchError> {
    let root: Value = serde_json::from_slice(body).map_err(|_| invalid_response())?;
    let data = root
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(invalid_response)?;
    if data.is_empty() {
        return Err(TwitchError::new(TwitchErrorKind::Offline));
    }
    if data.len() != 1 {
        return Err(invalid_response());
    }
    let stream = &data[0];
    if stream.get("type").and_then(Value::as_str) != Some("live") {
        return Err(TwitchError::new(TwitchErrorKind::Offline));
    }
    let login = bounded_string(
        stream.get("user_login").and_then(Value::as_str),
        MAX_CHANNEL_BYTES,
    )?
    .to_ascii_lowercase();
    if login != route.channel || !valid_channel(&login) {
        return Err(invalid_response());
    }
    let title = bounded_string(
        stream.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = bounded_optional_string(
        stream.get("user_name").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?
    .unwrap_or_else(|| route.channel.clone());
    let artwork_url = bounded_optional_string(
        stream.get("thumbnail_url").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?
    .map(|template| {
        template
            .replace("{width}", "440")
            .replace("{height}", "248")
    });
    if artwork_url
        .as_deref()
        .is_some_and(|url| url.len() > options.max_metadata_string_bytes)
    {
        return Err(invalid_response());
    }
    let canonical = route.canonical_url();
    Ok(TwitchSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration: Duration::ZERO,
            identifier: canonical.clone(),
            is_stream: true,
            uri: Some(canonical),
            artwork_url,
            isrc: None,
        },
        channel: route.channel.clone(),
    })
}

fn parse_playback_token(
    body: &[u8],
    channel: &str,
    options: &TwitchSourceOptions,
) -> Result<TwitchPlaybackUrl, TwitchError> {
    let root: Value = serde_json::from_slice(body).map_err(|_| invalid_response())?;
    if root
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
    {
        return Err(invalid_response());
    }
    let token = root
        .get("data")
        .and_then(|data| data.get("streamPlaybackAccessToken"));
    let Some(token) = token.filter(|token| !token.is_null()) else {
        return Err(TwitchError::new(TwitchErrorKind::Offline));
    };
    let value = bounded_string(
        token.get("value").and_then(Value::as_str),
        options.max_playback_token_bytes,
    )?;
    let signature = bounded_string(
        token.get("signature").and_then(Value::as_str),
        options.max_signature_bytes,
    )?;
    let token_value: Value = serde_json::from_str(&value).map_err(|_| invalid_response())?;
    if token_value.get("expires").and_then(Value::as_u64).is_none() {
        return Err(invalid_response());
    }
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("token", &value)
        .append_pair("sig", &signature)
        .append_pair("allow_source", "true")
        .append_pair("allow_spectre", "true")
        .append_pair("allow_audio_only", "true")
        .append_pair("player_backend", "html5")
        .append_pair("expgroup", "regular")
        .finish();
    let url = format!(
        "{}/{channel}.m3u8?{query}",
        options.usher_base_url.trim_end_matches('/')
    );
    validate_playback_url(url, options)
}

fn validate_playback_url(
    url: String,
    options: &TwitchSourceOptions,
) -> Result<TwitchPlaybackUrl, TwitchError> {
    if url.len() > options.max_playback_url_bytes {
        return Err(invalid_response());
    }
    let uri: Uri = url.parse().map_err(|_| invalid_response())?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    let valid_scheme = match options.playback_scheme {
        TwitchPlaybackScheme::Https => {
            uri.scheme_str() == Some("https")
                && (authority.host() == "ttvnw.net" || authority.host().ends_with(".ttvnw.net"))
        }
        TwitchPlaybackScheme::HttpForPrivateNetworks => {
            matches!(uri.scheme_str(), Some("http" | "https"))
        }
    };
    if !valid_scheme || authority.as_str().contains('@') || url.contains('#') {
        return Err(invalid_response());
    }
    Ok(TwitchPlaybackUrl { url })
}

fn bounded_string(value: Option<&str>, limit: usize) -> Result<String, TwitchError> {
    let value = value.ok_or_else(invalid_response)?;
    (!value.is_empty() && value.len() <= limit)
        .then(|| value.to_owned())
        .ok_or_else(invalid_response)
}

fn bounded_optional_string(
    value: Option<&str>,
    limit: usize,
) -> Result<Option<String>, TwitchError> {
    value
        .map(|value| bounded_string(Some(value), limit))
        .transpose()
}

fn validate_base_url(base: &str, http: &RemoteHttpOptions) -> Result<(), TwitchError> {
    if base.is_empty()
        || base.len() > MAX_CONFIGURED_IDENTIFIER_BYTES
        || base.contains(['?', '#', '@'])
    {
        return Err(TwitchError::new(TwitchErrorKind::InvalidOptions));
    }
    let uri: Uri = base
        .parse()
        .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidOptions))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err(TwitchError::new(TwitchErrorKind::InvalidOptions));
    }
    if uri.scheme_str() == Some("http")
        && http.network_access != HttpNetworkAccess::AllowPrivateNetworks
    {
        return Err(TwitchError::new(TwitchErrorKind::InvalidOptions));
    }
    RemoteHttpRequest::get(base)
        .map(|_| ())
        .map_err(|_| TwitchError::new(TwitchErrorKind::InvalidOptions))
}

fn invalid_response() -> TwitchError {
    TwitchError::new(TwitchErrorKind::InvalidResponse)
}

fn map_remote_error(error: crate::RemoteHttpError) -> TwitchError {
    TwitchError::new(match error.kind() {
        RemoteHttpErrorKind::Cancelled => TwitchErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => TwitchErrorKind::RateLimited,
        RemoteHttpErrorKind::Unauthorized | RemoteHttpErrorKind::Forbidden => {
            TwitchErrorKind::AuthenticationRequired
        }
        RemoteHttpErrorKind::NotFound => TwitchErrorKind::Unavailable,
        _ => TwitchErrorKind::Network,
    })
}

fn map_playback_source_error(error: TwitchError) -> TwitchPlaybackError {
    TwitchPlaybackError::new(TwitchPlaybackErrorKind::Source(error.kind()))
}

fn map_live_poll(poll: YoutubeLivePlaybackPoll) -> TwitchLivePlaybackPoll {
    match poll {
        YoutubeLivePlaybackPoll::Frame => TwitchLivePlaybackPoll::Frame,
        YoutubeLivePlaybackPoll::WaitUntil(deadline) => TwitchLivePlaybackPoll::WaitUntil(deadline),
        YoutubeLivePlaybackPoll::Ended => TwitchLivePlaybackPoll::Ended,
        YoutubeLivePlaybackPoll::Exhausted => TwitchLivePlaybackPoll::Exhausted,
    }
}

fn map_youtube_playback_error(error: YoutubePlaybackError) -> TwitchPlaybackError {
    TwitchPlaybackError::new(match error.kind() {
        YoutubePlaybackErrorKind::InvalidOptions => TwitchPlaybackErrorKind::InvalidOptions,
        YoutubePlaybackErrorKind::Cancelled => TwitchPlaybackErrorKind::Cancelled,
        YoutubePlaybackErrorKind::Source(_) | YoutubePlaybackErrorKind::Network => {
            TwitchPlaybackErrorKind::Network
        }
        YoutubePlaybackErrorKind::InvalidMedia => TwitchPlaybackErrorKind::InvalidMedia,
        YoutubePlaybackErrorKind::IncompatibleFormat => TwitchPlaybackErrorKind::IncompatibleFormat,
        YoutubePlaybackErrorKind::AudioPipeline => TwitchPlaybackErrorKind::AudioPipeline,
    })
}

fn map_hls_error(error: &HlsError) -> TwitchPlaybackError {
    let kind = match error {
        HlsError::InvalidLimits(_) => TwitchPlaybackErrorKind::InvalidOptions,
        HlsError::Media(MediaError::Cancelled) => TwitchPlaybackErrorKind::Cancelled,
        HlsError::Media(MediaError::Io(_)) | HlsError::Playlist(_) => {
            TwitchPlaybackErrorKind::Network
        }
        HlsError::Media(_)
        | HlsError::InvalidPlaylist(_)
        | HlsError::TooManyVariants { .. }
        | HlsError::TooManySegments { .. }
        | HlsError::SegmentDurationExceeded { .. }
        | HlsError::PlaylistDurationExceeded { .. }
        | HlsError::UnsupportedFeature(_)
        | HlsError::LiveReloadLimitExceeded { .. }
        | HlsError::NotVod => TwitchPlaybackErrorKind::InvalidMedia,
    };
    TwitchPlaybackError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_normalizes_current_mobile_and_go_hosts() {
        let options = TwitchSourceOptions::default();
        assert_eq!(
            route_twitch_identifier("https://m.twitch.tv/TwitchDev", &options),
            Some(TwitchRoute {
                channel: "twitchdev".to_owned(),
            })
        );
    }
}
