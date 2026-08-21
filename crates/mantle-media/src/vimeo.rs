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

const DEFAULT_PLAYER_BASE_URL: &str = "https://player.vimeo.com/video";
const DEFAULT_API_BASE_URL: &str = "https://api.vimeo.com";
const MAX_ACCESS_TOKEN_BYTES: usize = 16 * 1024;
const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_PLAYBACK_CANDIDATES: usize = 10_000;
const MAX_CONFIGURED_PLAYBACK_URL_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_TRACK_DURATION: Duration = Duration::from_hours(31 * 24);
const MAX_UNLISTED_HASH_BYTES: usize = 128;

/// Current Vimeo public URL shape supported by the source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VimeoRoute {
    pub video_id: String,
    pub unlisted_hash: Option<String>,
}

impl VimeoRoute {
    #[must_use]
    pub fn canonical_url(&self) -> String {
        self.unlisted_hash.as_ref().map_or_else(
            || format!("https://vimeo.com/{}", self.video_id),
            |hash| format!("https://vimeo.com/{}/{hash}", self.video_id),
        )
    }
}

/// Scheme policy for expiring Vimeo media URLs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VimeoPlaybackScheme {
    #[default]
    Https,
    /// Permit HTTP only alongside the explicit private-network source policy.
    ///
    /// This exists for trusted loopback replay and must not be used for public service traffic.
    HttpForPrivateNetworks,
}

/// Validated caller-provided Vimeo API access token.
#[derive(Clone, Eq, PartialEq)]
pub struct VimeoAuthentication {
    access_token: String,
}

impl VimeoAuthentication {
    /// Creates an authentication policy after checking its header-safe resource bound.
    ///
    /// # Errors
    ///
    /// Returns [`VimeoErrorKind::InvalidAuthentication`] for an empty, oversized, or
    /// non-ASCII-graphic token.
    pub fn new(access_token: impl Into<String>) -> Result<Self, VimeoError> {
        let access_token = access_token.into();
        if access_token.is_empty()
            || access_token.len() > MAX_ACCESS_TOKEN_BYTES
            || !access_token.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(VimeoError::new(VimeoErrorKind::InvalidAuthentication));
        }
        Ok(Self { access_token })
    }
}

impl fmt::Debug for VimeoAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VimeoAuthentication")
            .field("configured", &true)
            .finish_non_exhaustive()
    }
}

/// Bounded routing, response, and playback-discovery policy for Vimeo.
#[derive(Clone, Eq, PartialEq)]
pub struct VimeoSourceOptions {
    pub http: RemoteHttpOptions,
    pub player_base_url: String,
    pub api_base_url: String,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_playback_candidates: usize,
    pub max_playback_url_bytes: usize,
    pub max_response_bytes: u64,
    pub max_track_duration: Duration,
    pub playback_scheme: VimeoPlaybackScheme,
}

impl Default for VimeoSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            player_base_url: DEFAULT_PLAYER_BASE_URL.to_owned(),
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_playback_candidates: 128,
            max_playback_url_bytes: 64 * 1024,
            max_response_bytes: 4 * 1024 * 1024,
            max_track_duration: Duration::from_hours(24),
            playback_scheme: VimeoPlaybackScheme::Https,
        }
    }
}

impl VimeoSourceOptions {
    fn validate(&self) -> Result<(), VimeoError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_playback_candidates == 0
            || self.max_playback_candidates > MAX_CONFIGURED_PLAYBACK_CANDIDATES
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_PLAYBACK_URL_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_bytes > self.http.max_response_bytes
            || self.max_track_duration.is_zero()
            || self.max_track_duration > MAX_CONFIGURED_TRACK_DURATION
            || (self.playback_scheme == VimeoPlaybackScheme::HttpForPrivateNetworks
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks)
        {
            return Err(VimeoError::new(VimeoErrorKind::InvalidOptions));
        }
        validate_base_url(&self.player_base_url, &self.http)?;
        validate_base_url(&self.api_base_url, &self.http)
    }
}

impl fmt::Debug for VimeoSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VimeoSourceOptions")
            .field("http", &self.http)
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_playback_candidates", &self.max_playback_candidates)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("max_track_duration", &self.max_track_duration)
            .field("playback_scheme", &self.playback_scheme)
            .finish_non_exhaustive()
    }
}

/// Routes a bounded public Vimeo identifier without network access.
#[must_use]
pub fn route_vimeo_identifier(
    identifier: &str,
    options: &VimeoSourceOptions,
) -> Option<VimeoRoute> {
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
    let host = authority.host().to_ascii_lowercase();
    let segments: Vec<_> = uri.path().trim_matches('/').split('/').collect();
    let (video_id, path_hash) = match host.as_str() {
        "vimeo.com" | "www.vimeo.com" if matches!(segments.len(), 1 | 2) => {
            (segments[0], segments.get(1).copied())
        }
        "player.vimeo.com" if segments.len() == 2 && segments[0] == "video" => (segments[1], None),
        _ => return None,
    };
    if !valid_video_id(video_id) {
        return None;
    }
    let query_hash = uri.query().and_then(|query| {
        form_urlencoded::parse(query.as_bytes())
            .find_map(|(name, value)| (name == "h").then(|| value.into_owned()))
    });
    let hash = path_hash.map(str::to_owned).or(query_hash);
    if hash
        .as_deref()
        .is_some_and(|hash| !valid_unlisted_hash(hash))
    {
        return None;
    }
    Some(VimeoRoute {
        video_id: video_id.to_owned(),
        unlisted_hash: hash,
    })
}

fn valid_video_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 32 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_unlisted_hash(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_UNLISTED_HASH_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VimeoPlaybackKind {
    ProgressiveMp4,
    Hls,
}

/// An expiring Vimeo media URL. Its diagnostics always redact the value.
#[derive(Clone, Eq, PartialEq)]
pub struct VimeoPlaybackUrl {
    url: String,
    kind: VimeoPlaybackKind,
    mime_type: String,
}

impl VimeoPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn kind(&self) -> VimeoPlaybackKind {
        self.kind
    }

    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }
}

impl fmt::Debug for VimeoPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VimeoPlaybackUrl")
            .field("url", &"<redacted>")
            .field("kind", &self.kind)
            .field("mime_type", &self.mime_type)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VimeoSourceTrack {
    pub info: TrackInfo,
    pub playback: Option<VimeoPlaybackUrl>,
}

pub struct VimeoPlaybackSession {
    session: MediaSession,
}

impl VimeoPlaybackSession {
    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        self.session.info()
    }

    /// Decodes one bounded PCM frame.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining the signed URL.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, VimeoPlaybackError> {
        self.session
            .read_pcm(output)
            .map_err(map_playback_media_error)
    }

    /// Seeks the bounded progressive media input.
    ///
    /// # Errors
    ///
    /// Returns cancellation, media, or bounded-input failures without retaining the signed URL.
    pub fn seek(&mut self, requested: Duration) -> Result<SeekResult, VimeoPlaybackError> {
        self.session
            .seek(requested)
            .map_err(map_playback_media_error)
    }
}

impl fmt::Debug for VimeoPlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VimeoPlaybackSession")
            .field("media", self.info())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VimeoPlaybackErrorKind {
    Source(VimeoErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VimeoPlaybackError {
    kind: VimeoPlaybackErrorKind,
}

impl VimeoPlaybackError {
    const fn new(kind: VimeoPlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> VimeoPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Display for VimeoPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            VimeoPlaybackErrorKind::Source(_) => "Vimeo playback discovery failed",
            VimeoPlaybackErrorKind::InvalidOptions => "invalid Vimeo media policy",
            VimeoPlaybackErrorKind::Cancelled => "Vimeo playback cancelled",
            VimeoPlaybackErrorKind::Network => "Vimeo media request failed",
            VimeoPlaybackErrorKind::InvalidMedia => "Vimeo returned invalid media",
            VimeoPlaybackErrorKind::IncompatibleFormat => {
                "Vimeo playback is not a supported progressive MP4"
            }
        })
    }
}

impl std::error::Error for VimeoPlaybackError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VimeoErrorKind {
    InvalidOptions,
    InvalidAuthentication,
    Cancelled,
    Shutdown,
    Network,
    RateLimited,
    AuthenticationRequired,
    Unavailable,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VimeoError {
    kind: VimeoErrorKind,
}

impl VimeoError {
    const fn new(kind: VimeoErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> VimeoErrorKind {
        self.kind
    }
}

impl fmt::Display for VimeoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            VimeoErrorKind::InvalidOptions => "invalid Vimeo source policy",
            VimeoErrorKind::InvalidAuthentication => "invalid Vimeo authentication policy",
            VimeoErrorKind::Cancelled => "Vimeo load cancelled",
            VimeoErrorKind::Shutdown => "Vimeo source is shut down",
            VimeoErrorKind::Network => "Vimeo request failed",
            VimeoErrorKind::RateLimited => "Vimeo rate limit reached",
            VimeoErrorKind::AuthenticationRequired => "Vimeo rejected authentication",
            VimeoErrorKind::Unavailable => "Vimeo content is unavailable",
            VimeoErrorKind::InvalidResponse => "Vimeo returned an invalid response",
            VimeoErrorKind::UnsupportedRoute => "Vimeo route is not implemented",
        })
    }
}

impl std::error::Error for VimeoError {}

pub struct VimeoSourceManager {
    options: VimeoSourceOptions,
    authentication: Option<VimeoAuthentication>,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl VimeoSourceManager {
    /// Creates a public-config manager after validating all HTTP and parser bounds.
    ///
    /// # Errors
    ///
    /// Returns [`VimeoErrorKind::InvalidOptions`] for an invalid bound or HTTP policy.
    pub fn new(options: VimeoSourceOptions) -> Result<Self, VimeoError> {
        Self::build(options, None)
    }

    /// Creates a manager that uses the official Vimeo API for metadata and expiring file links.
    ///
    /// # Errors
    ///
    /// Returns [`VimeoErrorKind::InvalidOptions`] for an invalid bound or HTTP policy.
    pub fn with_authentication(
        options: VimeoSourceOptions,
        authentication: VimeoAuthentication,
    ) -> Result<Self, VimeoError> {
        Self::build(options, Some(authentication))
    }

    fn build(
        options: VimeoSourceOptions,
        authentication: Option<VimeoAuthentication>,
    ) -> Result<Self, VimeoError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| VimeoError::new(VimeoErrorKind::InvalidOptions))?;
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

    /// Loads one validated video through public player config or the configured official API.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, unavailable, or parser failures.
    pub fn load_route(
        &self,
        route: &VimeoRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<VimeoSourceTrack>, VimeoError> {
        self.ensure_active(cancellation)?;
        let value = if self.authentication.is_some() {
            let Some(body) = self.get_api_video(&route.video_id, cancellation)? else {
                return Ok(None);
            };
            parse_json(&body)?
        } else {
            let Some(body) = self.get_public_config(route, cancellation)? else {
                return Ok(None);
            };
            parse_json(&body)?
        };
        if self.authentication.is_some() {
            parse_api_track(&value, route, &self.options).map(Some)
        } else {
            parse_public_track(&value, route, &self.options).map(Some)
        }
    }

    /// Re-fetches the selected control-plane response and returns a fresh expiring media URL.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, network, unavailable, or parser failures.
    pub fn resolve_track_playback(
        &self,
        track: &VimeoSourceTrack,
        cancellation: &MediaCancellation,
    ) -> Result<Option<VimeoPlaybackUrl>, VimeoError> {
        if !valid_video_id(&track.info.identifier) {
            return Err(VimeoError::new(VimeoErrorKind::InvalidResponse));
        }
        let route = track
            .info
            .uri
            .as_deref()
            .and_then(|uri| route_vimeo_identifier(uri, &self.options))
            .filter(|route| route.video_id == track.info.identifier)
            .unwrap_or_else(|| VimeoRoute {
                video_id: track.info.identifier.clone(),
                unlisted_hash: None,
            });
        self.load_route(&route, cancellation)
            .map(|track| track.and_then(|track| track.playback))
    }

    /// Opens a freshly discovered progressive MP4 through Mantle's bounded media pipeline.
    ///
    /// # Errors
    ///
    /// Returns source, cancellation, network, media, or incompatible-format failures.
    pub fn open_track_playback(
        &self,
        track: &VimeoSourceTrack,
        range_options: HttpRangeOptions,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Option<VimeoPlaybackSession>, VimeoPlaybackError> {
        let Some(playback) = self
            .resolve_track_playback(track, &cancellation)
            .map_err(map_playback_source_error)?
        else {
            return Ok(None);
        };
        if playback.kind != VimeoPlaybackKind::ProgressiveMp4 {
            return Err(VimeoPlaybackError::new(
                VimeoPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        let input = HttpRangeInput::open_with_cancellation(
            playback.as_str(),
            range_options,
            cancellation.clone(),
        )
        .map_err(map_playback_media_error)?;
        let session = MediaSession::open_with_cancellation(
            Box::new(input),
            Some("mp4"),
            media_limits,
            cancellation,
        )
        .map_err(map_playback_media_error)?;
        if session.info().container != Container::Mp4
            || !matches!(
                session.info().codec,
                Codec::AacLc | Codec::HeAacV1 | Codec::HeAacV2 | Codec::Opus
            )
        {
            return Err(VimeoPlaybackError::new(
                VimeoPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(Some(VimeoPlaybackSession { session }))
    }

    fn ensure_active(&self, cancellation: &MediaCancellation) -> Result<(), VimeoError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(VimeoError::new(VimeoErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(VimeoError::new(VimeoErrorKind::Cancelled));
        }
        Ok(())
    }

    fn get_public_config(
        &self,
        route: &VimeoRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<Vec<u8>>, VimeoError> {
        let mut endpoint = format!(
            "{}/{}/config",
            self.options.player_base_url.trim_end_matches('/'),
            route.video_id
        );
        if let Some(hash) = route.unlisted_hash.as_deref() {
            endpoint.push_str("?h=");
            endpoint.extend(form_urlencoded::byte_serialize(hash.as_bytes()));
        }
        self.get_json(endpoint, None, "application/json", cancellation)
    }

    fn get_api_video(
        &self,
        video_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<Vec<u8>>, VimeoError> {
        let token = self
            .authentication
            .as_ref()
            .ok_or_else(|| VimeoError::new(VimeoErrorKind::InvalidAuthentication))?;
        let fields = "uri,name,user.name,duration,pictures.base_link,link,play";
        let endpoint = format!(
            "{}/videos/{video_id}?{}",
            self.options.api_base_url.trim_end_matches('/'),
            form_urlencoded::Serializer::new(String::new())
                .append_pair("fields", fields)
                .finish()
        );
        self.get_json(
            endpoint,
            Some(&format!("Bearer {}", token.access_token)),
            "application/vnd.vimeo.*+json;version=3.4",
            cancellation,
        )
    }

    fn get_json(
        &self,
        endpoint: String,
        authorization: Option<&str>,
        accept: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<Vec<u8>>, VimeoError> {
        let mut request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", accept))
            .and_then(|request| request.header("User-Agent", "Mantle-Vimeo/1"))
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| VimeoError::new(VimeoErrorKind::InvalidOptions))?;
        if let Some(authorization) = authorization {
            request = request
                .header("Authorization", authorization)
                .map_err(|_| VimeoError::new(VimeoErrorKind::InvalidAuthentication))?;
        }
        match self.http.execute_with_cancellation(&request, cancellation) {
            Ok(response) => Ok(Some(response.body().to_vec())),
            Err(error) if error.kind() == RemoteHttpErrorKind::NotFound => Ok(None),
            Err(error) => Err(map_remote_error(error)),
        }
    }
}

impl fmt::Debug for VimeoSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VimeoSourceManager")
            .field("options", &self.options)
            .field("authentication_configured", &self.authentication.is_some())
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<VimeoSourceTrack> for VimeoSourceManager {
    fn source_name(&self) -> &'static str {
        "vimeo"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<VimeoSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<VimeoSourceTrack>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_vimeo_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = MediaCancellation::linked({
            let cancellation = cancellation.clone();
            move || cancellation.is_cancelled()
        });
        match self.load_route(&route, &linked) {
            Ok(Some(track)) => Ok(Some(SourceLoad::Item(track))),
            Ok(None) => Ok(None),
            Err(error) if error.kind() == VimeoErrorKind::Cancelled => Ok(None),
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, _item: &VimeoSourceTrack) -> bool {
        true
    }

    fn encode(&self, _item: &VimeoSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(Vec::new())
    }

    fn decode(&self, _payload: &[u8]) -> Result<VimeoSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<VimeoSourceTrack, SourceRegistryError> {
        if !payload.is_empty() || !valid_video_id(&info.identifier) {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(VimeoSourceTrack {
            info: info.clone(),
            playback: None,
        })
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_public_track(
    root: &Value,
    route: &VimeoRoute,
    options: &VimeoSourceOptions,
) -> Result<VimeoSourceTrack, VimeoError> {
    let video = root.get("video").ok_or_else(invalid_response)?;
    let id = parse_id(video.get("id"))?;
    if id != route.video_id {
        return Err(invalid_response());
    }
    let title = bounded_string(
        video.get("title").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = bounded_optional_string(
        video
            .get("owner")
            .and_then(|owner| owner.get("name"))
            .and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?
    .unwrap_or_else(|| "Unknown artist".to_owned());
    let duration = parse_duration(video.get("duration"), options)?;
    let artwork_url = bounded_optional_string(
        video.get("thumbnail_url").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let playback = parse_public_playback(root, options)?;
    Ok(VimeoSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier: route.video_id.clone(),
            is_stream: false,
            uri: Some(route.canonical_url()),
            artwork_url,
            isrc: None,
        },
        playback,
    })
}

fn parse_api_track(
    root: &Value,
    route: &VimeoRoute,
    options: &VimeoSourceOptions,
) -> Result<VimeoSourceTrack, VimeoError> {
    let id = root
        .get("uri")
        .and_then(Value::as_str)
        .and_then(|uri| uri.strip_prefix("/videos/"))
        .ok_or_else(invalid_response)?;
    if id != route.video_id || !valid_video_id(id) {
        return Err(invalid_response());
    }
    let title = bounded_string(
        root.get("name").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let author = bounded_optional_string(
        root.get("user")
            .and_then(|user| user.get("name"))
            .and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?
    .unwrap_or_else(|| "Unknown artist".to_owned());
    let duration = parse_duration(root.get("duration"), options)?;
    let uri = bounded_optional_string(
        root.get("link").and_then(Value::as_str),
        options.max_identifier_bytes,
    )?
    .unwrap_or_else(|| route.canonical_url());
    let artwork_url = bounded_optional_string(
        root.get("pictures")
            .and_then(|pictures| pictures.get("base_link"))
            .and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let playback = parse_api_playback(root, options)?;
    Ok(VimeoSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier: route.video_id.clone(),
            is_stream: false,
            uri: Some(uri),
            artwork_url,
            isrc: None,
        },
        playback,
    })
}

fn parse_public_playback(
    root: &Value,
    options: &VimeoSourceOptions,
) -> Result<Option<VimeoPlaybackUrl>, VimeoError> {
    let files = root
        .get("request")
        .and_then(|request| request.get("files"))
        .ok_or_else(invalid_response)?;
    if let Some(progressive) = files.get("progressive").and_then(Value::as_array) {
        if progressive.len() > options.max_playback_candidates {
            return Err(invalid_response());
        }
        if let Some(candidate) = select_progressive(progressive, "url", "mime", options)? {
            return Ok(Some(candidate));
        }
    }
    let Some(hls) = files.get("hls") else {
        return Ok(None);
    };
    let default_cdn = bounded_string(
        hls.get("default_cdn").and_then(Value::as_str),
        options.max_metadata_string_bytes,
    )?;
    let cdns = hls
        .get("cdns")
        .and_then(Value::as_object)
        .ok_or_else(invalid_response)?;
    if cdns.len() > options.max_playback_candidates {
        return Err(invalid_response());
    }
    let url = cdns
        .get(&default_cdn)
        .and_then(|cdn| cdn.get("url"))
        .and_then(Value::as_str)
        .ok_or_else(invalid_response)?;
    parse_playback_url(
        url,
        VimeoPlaybackKind::Hls,
        "application/x-mpegURL",
        options,
    )
    .map(Some)
}

fn parse_api_playback(
    root: &Value,
    options: &VimeoSourceOptions,
) -> Result<Option<VimeoPlaybackUrl>, VimeoError> {
    let Some(play) = root.get("play") else {
        return Ok(None);
    };
    if let Some(progressive) = play.get("progressive").and_then(Value::as_array) {
        if progressive.len() > options.max_playback_candidates {
            return Err(invalid_response());
        }
        if let Some(candidate) = select_progressive(progressive, "link", "type", options)? {
            return Ok(Some(candidate));
        }
    }
    let Some(url) = play
        .get("hls")
        .and_then(|hls| hls.get("link"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    parse_playback_url(
        url,
        VimeoPlaybackKind::Hls,
        "application/x-mpegURL",
        options,
    )
    .map(Some)
}

fn select_progressive(
    values: &[Value],
    url_field: &str,
    mime_field: &str,
    options: &VimeoSourceOptions,
) -> Result<Option<VimeoPlaybackUrl>, VimeoError> {
    let mut selected: Option<(u64, VimeoPlaybackUrl)> = None;
    for value in values {
        let mime = value
            .get(mime_field)
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(mime, "audio/mp4" | "video/mp4") {
            continue;
        }
        let url = value
            .get(url_field)
            .and_then(Value::as_str)
            .ok_or_else(invalid_response)?;
        let candidate = parse_playback_url(url, VimeoPlaybackKind::ProgressiveMp4, mime, options)?;
        let height = value.get("height").and_then(Value::as_u64).unwrap_or(0);
        let score = if mime == "audio/mp4" {
            0
        } else {
            height.max(1)
        };
        if selected.as_ref().is_none_or(|(best, _)| score < *best) {
            selected = Some((score, candidate));
        }
    }
    Ok(selected.map(|(_, candidate)| candidate))
}

fn parse_playback_url(
    url: &str,
    kind: VimeoPlaybackKind,
    mime_type: &str,
    options: &VimeoSourceOptions,
) -> Result<VimeoPlaybackUrl, VimeoError> {
    let url = bounded_string(Some(url), options.max_playback_url_bytes)?;
    let uri: Uri = url.parse().map_err(|_| invalid_response())?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    let valid_scheme = match options.playback_scheme {
        VimeoPlaybackScheme::Https => {
            uri.scheme_str() == Some("https") && valid_vimeo_media_host(authority.host())
        }
        VimeoPlaybackScheme::HttpForPrivateNetworks => {
            matches!(uri.scheme_str(), Some("http" | "https"))
        }
    };
    if !valid_scheme || authority.as_str().contains('@') || url.contains('#') {
        return Err(invalid_response());
    }
    let mime_type = bounded_string(Some(mime_type), 128)?;
    Ok(VimeoPlaybackUrl {
        url,
        kind,
        mime_type,
    })
}

fn valid_vimeo_media_host(host: &str) -> bool {
    host == "vimeo.com"
        || host.ends_with(".vimeo.com")
        || host == "vimeocdn.com"
        || host.ends_with(".vimeocdn.com")
        || host == "akamaized.net"
        || host.ends_with(".akamaized.net")
}

fn parse_json(body: &[u8]) -> Result<Value, VimeoError> {
    serde_json::from_slice(body).map_err(|_| invalid_response())
}

fn parse_id(value: Option<&Value>) -> Result<String, VimeoError> {
    let id = value
        .and_then(|value| {
            value
                .as_u64()
                .map(|id| id.to_string())
                .or_else(|| value.as_str().map(str::to_owned))
        })
        .ok_or_else(invalid_response)?;
    valid_video_id(&id)
        .then_some(id)
        .ok_or_else(invalid_response)
}

fn parse_duration(
    value: Option<&Value>,
    options: &VimeoSourceOptions,
) -> Result<Duration, VimeoError> {
    let seconds = value.and_then(Value::as_f64).ok_or_else(invalid_response)?;
    if !seconds.is_finite() || seconds < 0.0 || seconds > options.max_track_duration.as_secs_f64() {
        return Err(invalid_response());
    }
    Ok(Duration::from_secs_f64(seconds))
}

fn bounded_string(value: Option<&str>, limit: usize) -> Result<String, VimeoError> {
    let value = value.ok_or_else(invalid_response)?;
    (!value.is_empty() && value.len() <= limit)
        .then(|| value.to_owned())
        .ok_or_else(invalid_response)
}

fn bounded_optional_string(
    value: Option<&str>,
    limit: usize,
) -> Result<Option<String>, VimeoError> {
    value
        .map(|value| bounded_string(Some(value), limit))
        .transpose()
}

fn validate_base_url(base: &str, http: &RemoteHttpOptions) -> Result<(), VimeoError> {
    if base.is_empty()
        || base.len() > MAX_CONFIGURED_IDENTIFIER_BYTES
        || base.contains(['?', '#', '@'])
    {
        return Err(VimeoError::new(VimeoErrorKind::InvalidOptions));
    }
    let uri: Uri = base
        .parse()
        .map_err(|_| VimeoError::new(VimeoErrorKind::InvalidOptions))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err(VimeoError::new(VimeoErrorKind::InvalidOptions));
    }
    if uri.scheme_str() == Some("http")
        && http.network_access != HttpNetworkAccess::AllowPrivateNetworks
    {
        return Err(VimeoError::new(VimeoErrorKind::InvalidOptions));
    }
    RemoteHttpRequest::get(base)
        .map(|_| ())
        .map_err(|_| VimeoError::new(VimeoErrorKind::InvalidOptions))
}

fn invalid_response() -> VimeoError {
    VimeoError::new(VimeoErrorKind::InvalidResponse)
}

fn map_remote_error(error: crate::RemoteHttpError) -> VimeoError {
    VimeoError::new(match error.kind() {
        RemoteHttpErrorKind::Cancelled => VimeoErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => VimeoErrorKind::RateLimited,
        RemoteHttpErrorKind::Unauthorized | RemoteHttpErrorKind::Forbidden => {
            VimeoErrorKind::AuthenticationRequired
        }
        RemoteHttpErrorKind::NotFound => VimeoErrorKind::Unavailable,
        _ => VimeoErrorKind::Network,
    })
}

fn map_playback_source_error(error: VimeoError) -> VimeoPlaybackError {
    VimeoPlaybackError::new(VimeoPlaybackErrorKind::Source(error.kind()))
}

fn map_playback_media_error(error: MediaError) -> VimeoPlaybackError {
    let kind = match error {
        MediaError::Cancelled => VimeoPlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            VimeoPlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            VimeoPlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => VimeoPlaybackErrorKind::Network,
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
        | MediaError::Backend { .. } => VimeoPlaybackErrorKind::InvalidMedia,
    };
    VimeoPlaybackError::new(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_keeps_unlisted_hashes_but_discards_tracking_queries() {
        let options = VimeoSourceOptions::default();
        assert_eq!(
            route_vimeo_identifier("https://player.vimeo.com/video/123?h=abc123", &options),
            Some(VimeoRoute {
                video_id: "123".to_owned(),
                unlisted_hash: Some("abc123".to_owned()),
            })
        );
    }
}
