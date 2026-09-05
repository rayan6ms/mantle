use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{
    MediaCancellation, RemoteHttpClient, RemoteHttpErrorKind, RemoteHttpOptions, RemoteHttpRequest,
    RemoteRetryMode,
};

const VIDEO_ID_BYTES: usize = 11;
const MAX_CONFIGURED_CLIENTS: usize = 16;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_THUMBNAILS: usize = 256;
const MAX_CONFIGURED_RESULT_TRACKS: usize = 10_000;
const MAX_CONFIGURED_PLAYLIST_PAGES: usize = 64;
const MAX_CONFIGURED_PLAYBACK_FORMATS: usize = 2_048;
const MAX_CONFIGURED_PLAYBACK_URL_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_PLAYER_SCRIPT_URL_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_PLAYER_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_PLAYER_SCRIPT_CACHE_TTL: Duration = Duration::from_hours(168);
const MAX_CONFIGURED_CIPHER_OPERATIONS: usize = 1_024;
const MAX_CONFIGURED_CIPHER_INPUT_BYTES: usize = 1024 * 1024;
const MAX_CREDENTIAL_BYTES: usize = 16 * 1024;
const MAX_API_BASE_URL_BYTES: usize = 4 * 1024;
const MAX_OAUTH_RESPONSE_BYTES: u64 = 1024 * 1024;
const MAX_OAUTH_TOKEN_LIFETIME: Duration = Duration::from_hours(168);
const WATCH_URL_PREFIX: &str = "https://www.youtube.com/watch?v=";
const DEFAULT_API_BASE_URL: &str = "https://youtubei.googleapis.com/youtubei/v1";
const DEFAULT_MUSIC_API_BASE_URL: &str = "https://music.youtube.com/youtubei/v1";
const DEFAULT_PLAYER_EMBED_URL: &str = "https://www.youtube.com/embed/";
const SEARCH_PARAMS: &str = "EgIQAfABAQ==";
const MUSIC_SEARCH_PARAMS: &str = "Eg-KAQwIARAAGAAgACgAMABqChADEAQQCRAFEAo=";
const DEFAULT_OAUTH_DEVICE_CODE_URL: &str = "https://www.youtube.com/o/oauth2/device/code";
const DEFAULT_OAUTH_TOKEN_URL: &str = "https://www.youtube.com/o/oauth2/token";
const DEFAULT_OAUTH_CLIENT_ID: &str =
    "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com";
const DEFAULT_OAUTH_CLIENT_SECRET: &str = "SboVhoG9s0rNafixCSGGKXAT";
const DEFAULT_OAUTH_SCOPES: &str =
    "http://gdata.youtube.com https://www.googleapis.com/auth/youtube";
const OAUTH_DEVICE_GRANT_TYPE: &str = "http://oauth.net/grant_type/device/1.0";

/// Current ordered `InnerTube` client profiles used by the pinned rewritten `YouTube` source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum YoutubeClientKind {
    Music,
    AndroidVr,
    Web,
    WebEmbedded,
    Tv,
}

impl YoutubeClientKind {
    #[must_use]
    pub const fn supports_video_loading(self) -> bool {
        !matches!(self, Self::Music | Self::Tv)
    }

    #[must_use]
    pub const fn supports_search(self) -> bool {
        matches!(self, Self::AndroidVr | Self::Web)
    }

    #[must_use]
    pub const fn supports_music_search(self) -> bool {
        matches!(self, Self::Music)
    }

    #[must_use]
    pub const fn supports_playlist_loading(self) -> bool {
        matches!(self, Self::AndroidVr | Self::Web)
    }

    #[must_use]
    pub const fn supports_playback(self) -> bool {
        !matches!(self, Self::Music)
    }

    #[must_use]
    pub const fn supports_oauth(self) -> bool {
        matches!(self, Self::Tv)
    }

    #[must_use]
    pub const fn requires_player_script(self) -> bool {
        matches!(self, Self::Web | Self::WebEmbedded)
    }

    const fn identifier(self) -> &'static str {
        match self {
            Self::Music => "WEB_REMIX",
            Self::AndroidVr => "ANDROID_VR",
            Self::Web => "WEB",
            Self::WebEmbedded => "WEB_EMBEDDED_PLAYER",
            Self::Tv => "TVHTML5",
        }
    }

    const fn version(self) -> &'static str {
        match self {
            Self::Music => "1.20240724.00.00",
            Self::AndroidVr => "1.60.19",
            Self::Web => "2.20250403.01.00",
            Self::WebEmbedded => "1.20250401.01.00",
            Self::Tv => "7.20250319.10.00",
        }
    }

    const fn user_agent(self) -> Option<&'static str> {
        match self {
            Self::AndroidVr => Some(
                "com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
            ),
            Self::Tv => Some("Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version"),
            _ => None,
        }
    }

    const fn player_params(self) -> Option<&'static str> {
        match self {
            Self::Web | Self::WebEmbedded | Self::Tv => Some("2AMB"),
            Self::Music | Self::AndroidVr => None,
        }
    }

    const fn uses_proof_of_origin(self) -> bool {
        matches!(self, Self::Web | Self::WebEmbedded)
    }
}

/// Bounded OAuth endpoints and public client configuration for the pinned `YouTube` device flow.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubeOAuthOptions {
    pub device_code_url: String,
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub max_response_bytes: u64,
    pub expiry_skew: Duration,
}

impl Default for YoutubeOAuthOptions {
    fn default() -> Self {
        Self {
            device_code_url: DEFAULT_OAUTH_DEVICE_CODE_URL.to_owned(),
            token_url: DEFAULT_OAUTH_TOKEN_URL.to_owned(),
            client_id: DEFAULT_OAUTH_CLIENT_ID.to_owned(),
            client_secret: DEFAULT_OAUTH_CLIENT_SECRET.to_owned(),
            scopes: DEFAULT_OAUTH_SCOPES.to_owned(),
            max_response_bytes: 64 * 1024,
            expiry_skew: Duration::from_mins(1),
        }
    }
}

impl YoutubeOAuthOptions {
    fn validate(&self) -> Result<(), YoutubeError> {
        if self.device_code_url.is_empty()
            || self.device_code_url.len() > MAX_API_BASE_URL_BYTES
            || self.token_url.is_empty()
            || self.token_url.len() > MAX_API_BASE_URL_BYTES
            || self.client_id.is_empty()
            || self.client_id.len() > MAX_CREDENTIAL_BYTES
            || self.client_secret.is_empty()
            || self.client_secret.len() > MAX_CREDENTIAL_BYTES
            || self.scopes.is_empty()
            || self.scopes.len() > MAX_CREDENTIAL_BYTES
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_OAUTH_RESPONSE_BYTES
            || self.expiry_skew > MAX_OAUTH_TOKEN_LIFETIME
        {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        RemoteHttpRequest::post(&self.device_code_url, [])
            .and_then(|request| request.max_response_bytes(self.max_response_bytes))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        RemoteHttpRequest::post(&self.token_url, [])
            .and_then(|request| request.max_response_bytes(self.max_response_bytes))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        Ok(())
    }
}

impl fmt::Debug for YoutubeOAuthOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeOAuthOptions")
            .field("max_response_bytes", &self.max_response_bytes)
            .field("expiry_skew", &self.expiry_skew)
            .finish_non_exhaustive()
    }
}

/// Monotonic caller-injectable time source for deterministic OAuth expiry behavior.
pub trait YoutubeOAuthClock: Send + Sync {
    fn now(&self) -> Duration;
}

struct SystemYoutubeOAuthClock {
    origin: Instant,
}

impl Default for SystemYoutubeOAuthClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl YoutubeOAuthClock for SystemYoutubeOAuthClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Validated source policy, routing switches, client order, and parser bounds.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubeSourceOptions {
    pub http: RemoteHttpOptions,
    pub oauth: YoutubeOAuthOptions,
    pub allow_search: bool,
    pub allow_direct_video_ids: bool,
    pub allow_direct_playlist_ids: bool,
    pub max_clients: usize,
    pub max_metadata_string_bytes: usize,
    pub max_thumbnails: usize,
    pub max_search_results: usize,
    pub max_playlist_tracks: usize,
    pub max_playlist_pages: usize,
    pub max_mix_tracks: usize,
    pub max_playback_formats: usize,
    pub max_playback_url_bytes: usize,
    pub max_player_script_url_bytes: usize,
    pub max_player_embed_bytes: u64,
    pub max_player_script_bytes: u64,
    pub player_script_cache_ttl: Duration,
    pub max_cipher_operations: usize,
    pub max_cipher_input_bytes: usize,
    pub api_base_url: String,
    pub music_api_base_url: String,
    pub player_embed_url: String,
    pub clients: Vec<YoutubeClientKind>,
}

impl fmt::Debug for YoutubeSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeSourceOptions")
            .field("http", &self.http)
            .field("oauth", &self.oauth)
            .field("allow_search", &self.allow_search)
            .field("allow_direct_video_ids", &self.allow_direct_video_ids)
            .field("allow_direct_playlist_ids", &self.allow_direct_playlist_ids)
            .field("max_clients", &self.max_clients)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_thumbnails", &self.max_thumbnails)
            .field("max_search_results", &self.max_search_results)
            .field("max_playlist_tracks", &self.max_playlist_tracks)
            .field("max_playlist_pages", &self.max_playlist_pages)
            .field("max_mix_tracks", &self.max_mix_tracks)
            .field("max_playback_formats", &self.max_playback_formats)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field(
                "max_player_script_url_bytes",
                &self.max_player_script_url_bytes,
            )
            .field("max_player_embed_bytes", &self.max_player_embed_bytes)
            .field("max_player_script_bytes", &self.max_player_script_bytes)
            .field("player_script_cache_ttl", &self.player_script_cache_ttl)
            .field("max_cipher_operations", &self.max_cipher_operations)
            .field("max_cipher_input_bytes", &self.max_cipher_input_bytes)
            .field("client_count", &self.clients.len())
            .finish_non_exhaustive()
    }
}

impl Default for YoutubeSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            oauth: YoutubeOAuthOptions::default(),
            allow_search: true,
            allow_direct_video_ids: true,
            allow_direct_playlist_ids: true,
            max_clients: 8,
            max_metadata_string_bytes: 64 * 1024,
            max_thumbnails: 64,
            max_search_results: 100,
            max_playlist_tracks: 600,
            max_playlist_pages: 6,
            max_mix_tracks: 100,
            max_playback_formats: 256,
            max_playback_url_bytes: 64 * 1024,
            max_player_script_url_bytes: 16 * 1024,
            max_player_embed_bytes: 1024 * 1024,
            max_player_script_bytes: 4 * 1024 * 1024,
            player_script_cache_ttl: Duration::from_hours(24),
            max_cipher_operations: 64,
            max_cipher_input_bytes: 16 * 1024,
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
            music_api_base_url: DEFAULT_MUSIC_API_BASE_URL.to_owned(),
            player_embed_url: DEFAULT_PLAYER_EMBED_URL.to_owned(),
            clients: vec![
                YoutubeClientKind::Music,
                YoutubeClientKind::AndroidVr,
                YoutubeClientKind::Web,
                YoutubeClientKind::WebEmbedded,
            ],
        }
    }
}

impl YoutubeSourceOptions {
    fn validate(&self) -> Result<(), YoutubeError> {
        self.oauth.validate()?;
        if self.max_clients == 0
            || self.max_clients > MAX_CONFIGURED_CLIENTS
            || self.clients.is_empty()
            || self.clients.len() > self.max_clients
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_thumbnails == 0
            || self.max_thumbnails > MAX_CONFIGURED_THUMBNAILS
            || self.max_search_results == 0
            || self.max_search_results > MAX_CONFIGURED_RESULT_TRACKS
            || self.max_playlist_tracks == 0
            || self.max_playlist_tracks > MAX_CONFIGURED_RESULT_TRACKS
            || self.max_playlist_pages == 0
            || self.max_playlist_pages > MAX_CONFIGURED_PLAYLIST_PAGES
            || self.max_mix_tracks == 0
            || self.max_mix_tracks > MAX_CONFIGURED_RESULT_TRACKS
            || self.max_playback_formats == 0
            || self.max_playback_formats > MAX_CONFIGURED_PLAYBACK_FORMATS
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_PLAYBACK_URL_BYTES
            || self.max_player_script_url_bytes == 0
            || self.max_player_script_url_bytes > MAX_CONFIGURED_PLAYER_SCRIPT_URL_BYTES
            || self.max_player_embed_bytes == 0
            || self.max_player_embed_bytes > MAX_CONFIGURED_PLAYER_RESOURCE_BYTES
            || self.max_player_embed_bytes > self.http.max_response_bytes
            || self.max_player_script_bytes == 0
            || self.max_player_script_bytes > MAX_CONFIGURED_PLAYER_RESOURCE_BYTES
            || self.max_player_script_bytes > self.http.max_response_bytes
            || self.player_script_cache_ttl.is_zero()
            || self.player_script_cache_ttl > MAX_CONFIGURED_PLAYER_SCRIPT_CACHE_TTL
            || self.max_cipher_operations == 0
            || self.max_cipher_operations > MAX_CONFIGURED_CIPHER_OPERATIONS
            || self.max_cipher_input_bytes == 0
            || self.max_cipher_input_bytes > MAX_CONFIGURED_CIPHER_INPUT_BYTES
            || self.oauth.max_response_bytes > self.http.max_response_bytes
            || self.api_base_url.is_empty()
            || self.api_base_url.len() > MAX_API_BASE_URL_BYTES
            || self.music_api_base_url.is_empty()
            || self.music_api_base_url.len() > MAX_API_BASE_URL_BYTES
            || self.player_embed_url.is_empty()
            || self.player_embed_url.len() > MAX_API_BASE_URL_BYTES
        {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        let endpoint = format!(
            "{}/player?prettyPrint=false",
            self.api_base_url.trim_end_matches('/')
        );
        RemoteHttpRequest::post(endpoint, [])
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let music_endpoint = format!(
            "{}/search?prettyPrint=false",
            self.music_api_base_url.trim_end_matches('/')
        );
        RemoteHttpRequest::post(music_endpoint, [])
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        RemoteHttpRequest::get(&self.player_embed_url)
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        Ok(())
    }
}

/// Explicit optional authentication and proof-of-origin inputs.
#[derive(Clone, Default)]
pub struct YoutubeAuthentication {
    oauth_access_token: Option<String>,
    oauth_refresh_token: Option<String>,
    po_token: Option<String>,
    visitor_data: Option<String>,
}

impl YoutubeAuthentication {
    /// Creates validated secret material. PO token and visitor data must be supplied together.
    ///
    /// # Errors
    ///
    /// Returns [`YoutubeErrorKind::InvalidAuthentication`] for empty, oversized, or unpaired
    /// values.
    pub fn new(
        oauth_access_token: Option<String>,
        po_token: Option<String>,
        visitor_data: Option<String>,
    ) -> Result<Self, YoutubeError> {
        let authentication = Self {
            oauth_access_token,
            oauth_refresh_token: None,
            po_token,
            visitor_data,
        };
        authentication.validate()?;
        Ok(authentication)
    }

    /// Creates validated refresh-token credentials for an OAuth-capable playback client.
    ///
    /// The access token is fetched lazily on the first eligible player request. PO token and
    /// visitor data must be supplied together.
    ///
    /// # Errors
    ///
    /// Returns [`YoutubeErrorKind::InvalidAuthentication`] for empty, oversized, or unpaired
    /// values.
    pub fn with_refresh_token(
        oauth_refresh_token: String,
        po_token: Option<String>,
        visitor_data: Option<String>,
    ) -> Result<Self, YoutubeError> {
        let authentication = Self {
            oauth_access_token: None,
            oauth_refresh_token: Some(oauth_refresh_token),
            po_token,
            visitor_data,
        };
        authentication.validate()?;
        Ok(authentication)
    }

    fn validate(&self) -> Result<(), YoutubeError> {
        if self.po_token.is_some() != self.visitor_data.is_some()
            || [
                &self.oauth_access_token,
                &self.oauth_refresh_token,
                &self.po_token,
                &self.visitor_data,
            ]
            .into_iter()
            .flatten()
            .any(|value| value.is_empty() || value.len() > MAX_CREDENTIAL_BYTES)
        {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidAuthentication));
        }
        Ok(())
    }
}

impl fmt::Debug for YoutubeAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeAuthentication")
            .field(
                "oauth",
                &(self.oauth_access_token.is_some() || self.oauth_refresh_token.is_some()),
            )
            .field("oauth_refresh", &self.oauth_refresh_token.is_some())
            .field("proof_of_origin", &self.po_token.is_some())
            .field("visitor_data", &self.visitor_data.is_some())
            .finish()
    }
}

/// User-facing values from the bounded OAuth device-code request.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubeOAuthDeviceCode {
    verification_url: String,
    user_code: String,
    device_code: String,
    poll_interval: Duration,
    expires_in: Duration,
}

impl YoutubeOAuthDeviceCode {
    #[must_use]
    pub fn verification_url(&self) -> &str {
        &self.verification_url
    }

    #[must_use]
    pub fn user_code(&self) -> &str {
        &self.user_code
    }

    #[must_use]
    pub fn device_code(&self) -> &str {
        &self.device_code
    }

    #[must_use]
    pub const fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    #[must_use]
    pub const fn expires_in(&self) -> Duration {
        self.expires_in
    }
}

impl fmt::Debug for YoutubeOAuthDeviceCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeOAuthDeviceCode")
            .field("poll_interval", &self.poll_interval)
            .field("expires_in", &self.expires_in)
            .finish_non_exhaustive()
    }
}

/// Non-secret result of an access-token exchange or refresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YoutubeOAuthTokenStatus {
    expires_in: Duration,
    refresh_token_rotated: bool,
}

impl YoutubeOAuthTokenStatus {
    #[must_use]
    pub const fn expires_in(&self) -> Duration {
        self.expires_in
    }

    #[must_use]
    pub const fn refresh_token_rotated(&self) -> bool {
        self.refresh_token_rotated
    }
}

struct YoutubeOAuthState {
    access_token: Option<String>,
    refresh_token: Option<String>,
    token_type: String,
    expires_at: Option<Duration>,
}

impl YoutubeOAuthState {
    fn new(authentication: &YoutubeAuthentication) -> Self {
        Self {
            access_token: authentication.oauth_access_token.clone(),
            refresh_token: authentication.oauth_refresh_token.clone(),
            token_type: "Bearer".to_owned(),
            expires_at: None,
        }
    }
}

#[derive(Deserialize)]
struct OAuthDeviceCodeResponse {
    verification_url: String,
    user_code: String,
    device_code: String,
    #[serde(default = "default_oauth_poll_interval_seconds")]
    interval: u64,
    expires_in: u64,
}

const fn default_oauth_poll_interval_seconds() -> u64 {
    5
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
    error: Option<String>,
}

/// A recognized `YouTube` identifier after deterministic routing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YoutubeRoute {
    Video(String),
    Playlist {
        playlist_id: String,
        selected_video_id: Option<String>,
    },
    Mix {
        playlist_id: String,
        selected_video_id: String,
    },
    Search(String),
    MusicSearch(String),
    NoTrack,
}

/// Routes current `YouTube` URLs, direct IDs, and search prefixes without network access.
#[must_use]
pub fn route_youtube_identifier(
    identifier: &str,
    options: &YoutubeSourceOptions,
) -> Option<YoutubeRoute> {
    if let Some(query) = identifier.strip_prefix("ytsearch:") {
        return options.allow_search.then(|| route_search(query, false));
    }
    if let Some(query) = identifier.strip_prefix("ytmsearch:") {
        return options.allow_search.then(|| route_search(query, true));
    }

    if options.allow_direct_video_ids && valid_video_id(identifier) {
        return Some(YoutubeRoute::Video(identifier.to_owned()));
    }
    if options.allow_direct_playlist_ids && valid_direct_playlist_id(identifier) {
        return Some(YoutubeRoute::Playlist {
            playlist_id: identifier.to_owned(),
            selected_video_id: None,
        });
    }

    let (host, request_target) = split_youtube_url(identifier)?;
    let (path, query) = request_target
        .split_once('?')
        .map_or((request_target, ""), |(path, query)| (path, query));
    let path = path.split('#').next().unwrap_or(path);
    let query = query.split('#').next().unwrap_or(query);

    if matches!(host, "youtu.be" | "www.youtu.be") {
        return Some(route_video_candidate(path.trim_start_matches('/')));
    }
    if !matches!(
        host,
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com"
    ) {
        return None;
    }

    match path {
        "/watch" => {
            let video_id = query_parameter(query, "v")?;
            Some(route_video_with_list(
                video_id,
                query_parameter(query, "list"),
            ))
        }
        "/playlist" => query_parameter(query, "list").map(|playlist_id| {
            if playlist_id.starts_with("RD") {
                let selected = playlist_id.strip_prefix("RD").unwrap_or_default();
                if valid_video_id(selected) {
                    YoutubeRoute::Mix {
                        playlist_id: playlist_id.to_owned(),
                        selected_video_id: selected.to_owned(),
                    }
                } else {
                    YoutubeRoute::NoTrack
                }
            } else {
                YoutubeRoute::Playlist {
                    playlist_id: playlist_id.to_owned(),
                    selected_video_id: None,
                }
            }
        }),
        _ => ["/live/", "/embed/", "/shorts/"]
            .iter()
            .find_map(|prefix| path.strip_prefix(prefix))
            .map(route_video_candidate),
    }
}

fn route_search(query: &str, music: bool) -> YoutubeRoute {
    let query = query.trim();
    if query.is_empty() {
        YoutubeRoute::NoTrack
    } else if music {
        YoutubeRoute::MusicSearch(query.to_owned())
    } else {
        YoutubeRoute::Search(query.to_owned())
    }
}

fn split_youtube_url(identifier: &str) -> Option<(&str, &str)> {
    let without_scheme = identifier
        .strip_prefix("https://")
        .or_else(|| identifier.strip_prefix("http://"))
        .unwrap_or(identifier);
    let slash = without_scheme.find('/').unwrap_or(without_scheme.len());
    let host = &without_scheme[..slash];
    if !matches!(
        host,
        "youtube.com"
            | "www.youtube.com"
            | "m.youtube.com"
            | "music.youtube.com"
            | "youtu.be"
            | "www.youtu.be"
    ) {
        return None;
    }
    let target = without_scheme
        .get(slash..)
        .filter(|target| !target.is_empty())
        .unwrap_or("/");
    Some((host, target))
}

fn query_parameter<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|part| {
        let (candidate, value) = part.split_once('=')?;
        (candidate == name && !value.is_empty()).then_some(value)
    })
}

fn route_video_with_list(video_id: &str, playlist_id: Option<&str>) -> YoutubeRoute {
    let Some(video) = trim_video_id(video_id) else {
        return YoutubeRoute::NoTrack;
    };
    if let Some(playlist_id) = playlist_id {
        if playlist_id.starts_with("RD") {
            return YoutubeRoute::Mix {
                playlist_id: playlist_id.to_owned(),
                selected_video_id: video.to_owned(),
            };
        }
        if !["LL", "WL", "LM"]
            .iter()
            .any(|prefix| playlist_id.starts_with(prefix))
        {
            return YoutubeRoute::Playlist {
                playlist_id: playlist_id.to_owned(),
                selected_video_id: Some(video.to_owned()),
            };
        }
    }
    YoutubeRoute::Video(video.to_owned())
}

fn route_video_candidate(candidate: &str) -> YoutubeRoute {
    trim_video_id(candidate).map_or(YoutubeRoute::NoTrack, |video| {
        YoutubeRoute::Video(video.to_owned())
    })
}

fn trim_video_id(candidate: &str) -> Option<&str> {
    let candidate = candidate.get(..candidate.len().min(VIDEO_ID_BYTES))?;
    valid_video_id(candidate).then_some(candidate)
}

fn valid_video_id(identifier: &str) -> bool {
    identifier.len() == VIDEO_ID_BYTES
        && identifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_direct_playlist_id(identifier: &str) -> bool {
    identifier
        .strip_prefix("PL")
        .or_else(|| identifier.strip_prefix("UU"))
        .is_some_and(|rest| {
            !rest.is_empty()
                && rest
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
}

fn extract_player_script_reference(embed: &[u8]) -> Option<String> {
    let embed = std::str::from_utf8(embed).ok()?;
    let mut search_from = 0;
    while let Some(relative_key) = embed[search_from..].find("\"jsUrl\"") {
        let key_end = search_from + relative_key + "\"jsUrl\"".len();
        let remainder = embed[key_end..].trim_start();
        let Some(remainder) = remainder.strip_prefix(':') else {
            search_from = key_end;
            continue;
        };
        let remainder = remainder.trim_start();
        if !remainder.starts_with('"') {
            search_from = key_end;
            continue;
        }
        let mut escaped = false;
        for (index, byte) in remainder.bytes().enumerate().skip(1) {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                return serde_json::from_str::<String>(&remainder[..=index]).ok();
            }
        }
        return None;
    }
    None
}

fn resolve_player_script_url(
    embed_url: &str,
    script_reference: &str,
    max_url_bytes: usize,
) -> Result<String, YoutubeError> {
    let resolved =
        if script_reference.starts_with("https://") || script_reference.starts_with("http://") {
            script_reference.to_owned()
        } else if script_reference.starts_with("//") {
            format!("https:{script_reference}")
        } else if script_reference.starts_with('/') {
            let (scheme, remainder) = embed_url
                .split_once("://")
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
            let authority = remainder
                .split('/')
                .next()
                .filter(|authority| !authority.is_empty())
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
            format!("{scheme}://{authority}{script_reference}")
        } else {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        };
    if resolved.len() > max_url_bytes || RemoteHttpRequest::get(&resolved).is_err() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(resolved)
}

fn extract_signature_timestamp(source: &[u8]) -> Option<u64> {
    let source = std::str::from_utf8(source).ok()?;
    for marker in ["signatureTimestamp", "sts"] {
        for (position, _) in source.match_indices(marker) {
            let mut remainder = &source[position + marker.len()..];
            remainder = remainder.trim_start();
            if let Some(after_quote) = remainder
                .strip_prefix('"')
                .or_else(|| remainder.strip_prefix('\''))
            {
                remainder = after_quote.trim_start();
            }
            let Some(after_colon) = remainder.strip_prefix(':') else {
                continue;
            };
            remainder = after_colon.trim_start();
            if let Some(after_quote) = remainder
                .strip_prefix('"')
                .or_else(|| remainder.strip_prefix('\''))
            {
                remainder = after_quote;
            }
            let digits = remainder
                .bytes()
                .take_while(u8::is_ascii_digit)
                .take(21)
                .count();
            if digits == 0 || digits > 20 {
                continue;
            }
            if let Ok(timestamp) = remainder[..digits].parse::<u64>() {
                return Some(timestamp);
            }
        }
    }
    None
}

impl YoutubeTransformProgram {
    fn apply(&self, input: &str, max_input_bytes: usize) -> Result<String, YoutubeError> {
        if input.is_empty()
            || input.len() > max_input_bytes
            || !input.bytes().all(|byte| byte.is_ascii())
        {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let mut output = input.as_bytes().to_vec();
        for operation in &self.operations {
            match *operation {
                YoutubeTransformOperation::Reverse => output.reverse(),
                YoutubeTransformOperation::Slice(count) => {
                    let count = count.min(output.len());
                    output.drain(..count);
                }
                YoutubeTransformOperation::Swap(position) => {
                    if output.is_empty() {
                        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
                    }
                    let position = position % output.len();
                    output.swap(0, position);
                }
            }
        }
        if output.is_empty() || output.len() > max_input_bytes {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        String::from_utf8(output).map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))
    }
}

#[derive(Clone, Copy)]
enum YoutubeHelperOperation {
    Reverse,
    Slice,
    Swap,
}

struct YoutubeHelperMethod<'a> {
    object: &'a str,
    method: &'a str,
    operation: YoutubeHelperOperation,
}

struct NamedYoutubeTransform<'a> {
    name: &'a str,
    program: YoutubeTransformProgram,
}

fn parse_youtube_cipher_program(
    source: &[u8],
    max_operations: usize,
) -> Result<YoutubeCipherProgram, YoutubeError> {
    let source = std::str::from_utf8(source)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let helpers = parse_youtube_helper_methods(source, max_operations)?;
    let transforms = parse_youtube_transform_functions(source, &helpers, max_operations)?;
    let signature_name =
        referenced_transform_name(source, &["\"signature\"", "'signature'"], &transforms, None)?;
    let n_name =
        referenced_transform_name(source, &["\"n\"", "'n'"], &transforms, Some(signature_name))?;
    let signature = transforms
        .iter()
        .find(|transform| transform.name == signature_name)
        .map(|transform| transform.program.clone())
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let n_parameter = transforms
        .iter()
        .find(|transform| transform.name == n_name)
        .map(|transform| transform.program.clone())
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    Ok(YoutubeCipherProgram {
        signature,
        n_parameter,
    })
}

fn parse_youtube_helper_methods(
    source: &str,
    max_operations: usize,
) -> Result<Vec<YoutubeHelperMethod<'_>>, YoutubeError> {
    let mut helpers = Vec::new();
    let mut search_from = 0_usize;
    while let Some(relative) = source[search_from..].find("var") {
        let position = search_from + relative;
        search_from = position + 3;
        if position > 0 && is_js_identifier_byte(source.as_bytes()[position - 1]) {
            continue;
        }
        let mut cursor = skip_ascii_whitespace(source, position + 3);
        let Some((object, next)) = parse_js_identifier(source, cursor) else {
            continue;
        };
        cursor = skip_ascii_whitespace(source, next);
        if source.as_bytes().get(cursor) != Some(&b'=') {
            continue;
        }
        cursor = skip_ascii_whitespace(source, cursor + 1);
        if source.as_bytes().get(cursor) != Some(&b'{') {
            continue;
        }
        let object_end = find_matching_javascript(source, cursor, b'{', b'}')?;
        let helper_start = helpers.len();
        let mut valid_helper_object = true;
        let mut member_cursor = cursor + 1;
        while member_cursor < object_end {
            member_cursor = skip_ascii_whitespace_and_commas(source, member_cursor);
            if member_cursor >= object_end {
                break;
            }
            let Some((method, next)) = parse_js_property_name(source, member_cursor) else {
                valid_helper_object = false;
                break;
            };
            member_cursor = skip_ascii_whitespace(source, next);
            if source.as_bytes().get(member_cursor) != Some(&b':') {
                valid_helper_object = false;
                break;
            }
            member_cursor = skip_ascii_whitespace(source, member_cursor + 1);
            let Some(after_function) = source[member_cursor..].strip_prefix("function") else {
                valid_helper_object = false;
                break;
            };
            member_cursor = source.len() - after_function.len();
            member_cursor = skip_ascii_whitespace(source, member_cursor);
            if source.as_bytes().get(member_cursor) != Some(&b'(') {
                valid_helper_object = false;
                break;
            }
            let parameters_end = find_matching_javascript(source, member_cursor, b'(', b')')?;
            let parameters = &source[member_cursor + 1..parameters_end];
            member_cursor = skip_ascii_whitespace(source, parameters_end + 1);
            if source.as_bytes().get(member_cursor) != Some(&b'{') {
                valid_helper_object = false;
                break;
            }
            let body_end = find_matching_javascript(source, member_cursor, b'{', b'}')?;
            if let Some(operation) =
                classify_youtube_helper(parameters, &source[member_cursor + 1..body_end])
            {
                if helpers.len() >= max_operations.saturating_mul(4) {
                    return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
                }
                helpers.push(YoutubeHelperMethod {
                    object,
                    method,
                    operation,
                });
            }
            member_cursor = body_end + 1;
        }
        if !valid_helper_object {
            helpers.truncate(helper_start);
        }
        search_from = object_end + 1;
    }
    if helpers.is_empty() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(helpers)
}

fn classify_youtube_helper(parameters: &str, body: &str) -> Option<YoutubeHelperOperation> {
    let mut parameters = parameters.split(',').map(str::trim);
    let first = parameters.next()?;
    if !valid_js_identifier(first) {
        return None;
    }
    let second = parameters.next();
    if parameters.next().is_some() || second.is_some_and(|value| !valid_js_identifier(value)) {
        return None;
    }
    let body = strip_ascii_whitespace(body)?;
    if body == format!("{first}.reverse()") || body == format!("return{first}.reverse()") {
        return Some(YoutubeHelperOperation::Reverse);
    }
    if let Some(second) = second {
        if body == format!("{first}.splice(0,{second})")
            || body == format!("return{first}.splice(0,{second})")
        {
            return Some(YoutubeHelperOperation::Slice);
        }
        let swap = format!(
            "varc={first}[0];{first}[0]={first}[{second}%{first}.length];{first}[{second}%{first}.length]=c"
        );
        let swap_parenthesized = format!(
            "varc={first}[0];{first}[0]={first}[({second}%{first}.length)];{first}[({second}%{first}.length)]=c"
        );
        if body == swap || body == swap_parenthesized {
            return Some(YoutubeHelperOperation::Swap);
        }
    }
    None
}

fn parse_youtube_transform_functions<'a>(
    source: &'a str,
    helpers: &[YoutubeHelperMethod<'a>],
    max_operations: usize,
) -> Result<Vec<NamedYoutubeTransform<'a>>, YoutubeError> {
    let mut transforms = Vec::new();
    let mut search_from = 0_usize;
    while let Some(relative) = source[search_from..].find("function") {
        let position = search_from + relative;
        search_from = position + "function".len();
        if position > 0 && is_js_identifier_byte(source.as_bytes()[position - 1]) {
            continue;
        }
        let mut cursor = skip_ascii_whitespace(source, search_from);
        let Some((name, next)) = parse_js_identifier(source, cursor) else {
            continue;
        };
        cursor = skip_ascii_whitespace(source, next);
        if source.as_bytes().get(cursor) != Some(&b'(') {
            continue;
        }
        let parameters_end = find_matching_javascript(source, cursor, b'(', b')')?;
        let parameter = source[cursor + 1..parameters_end].trim();
        if !valid_js_identifier(parameter) {
            continue;
        }
        cursor = skip_ascii_whitespace(source, parameters_end + 1);
        if source.as_bytes().get(cursor) != Some(&b'{') {
            continue;
        }
        let body_end = find_matching_javascript(source, cursor, b'{', b'}')?;
        if let Some(program) = parse_youtube_transform_body(
            &source[cursor + 1..body_end],
            parameter,
            helpers,
            max_operations,
        )? {
            transforms.push(NamedYoutubeTransform { name, program });
        }
        search_from = body_end + 1;
    }
    if transforms.len() < 2 {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(transforms)
}

fn parse_youtube_transform_body(
    body: &str,
    parameter: &str,
    helpers: &[YoutubeHelperMethod<'_>],
    max_operations: usize,
) -> Result<Option<YoutubeTransformProgram>, YoutubeError> {
    let Some(body) = strip_ascii_whitespace(body) else {
        return Ok(None);
    };
    let double_split = format!("{parameter}={parameter}.split(\"\");");
    let single_split = format!("{parameter}={parameter}.split('');");
    let body = body
        .strip_prefix(&double_split)
        .or_else(|| body.strip_prefix(&single_split));
    let Some(body) = body else { return Ok(None) };
    let double_join = format!("return{parameter}.join(\"\")");
    let single_join = format!("return{parameter}.join('')");
    let middle = body
        .strip_suffix(&double_join)
        .or_else(|| body.strip_suffix(&format!("{double_join};")))
        .or_else(|| body.strip_suffix(&single_join))
        .or_else(|| body.strip_suffix(&format!("{single_join};")));
    let Some(middle) = middle else {
        return Ok(None);
    };
    let mut operations = Vec::new();
    for statement in middle.split(';').filter(|statement| !statement.is_empty()) {
        if operations.len() >= max_operations {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        operations.push(parse_youtube_transform_call(statement, parameter, helpers)?);
    }
    if operations.is_empty() {
        return Ok(None);
    }
    Ok(Some(YoutubeTransformProgram { operations }))
}

fn parse_youtube_transform_call(
    statement: &str,
    parameter: &str,
    helpers: &[YoutubeHelperMethod<'_>],
) -> Result<YoutubeTransformOperation, YoutubeError> {
    let (qualified, arguments) = statement
        .split_once('(')
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let arguments = arguments
        .strip_suffix(')')
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let (object, method) = qualified
        .split_once('.')
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let helper = helpers
        .iter()
        .find(|helper| helper.object == object && helper.method == method)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let mut arguments = arguments.split(',');
    if arguments.next() != Some(parameter) {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let value = arguments
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    if arguments.next().is_some() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    match (helper.operation, value) {
        (YoutubeHelperOperation::Reverse, None | Some(_)) => Ok(YoutubeTransformOperation::Reverse),
        (YoutubeHelperOperation::Slice, Some(value)) => Ok(YoutubeTransformOperation::Slice(value)),
        (YoutubeHelperOperation::Swap, Some(value)) => Ok(YoutubeTransformOperation::Swap(value)),
        (YoutubeHelperOperation::Slice | YoutubeHelperOperation::Swap, None) => {
            Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse))
        }
    }
}

fn referenced_transform_name<'a>(
    source: &str,
    markers: &[&str],
    transforms: &[NamedYoutubeTransform<'a>],
    excluded: Option<&str>,
) -> Result<&'a str, YoutubeError> {
    for marker in markers {
        let Some(position) = source.find(marker) else {
            continue;
        };
        let end = (position + marker.len() + 256).min(source.len());
        let window = &source[position + marker.len()..end];
        let mut matches = transforms
            .iter()
            .filter(|transform| Some(transform.name) != excluded)
            .filter_map(|transform| {
                find_js_function_call(window, transform.name).map(|index| (index, transform.name))
            })
            .collect::<Vec<_>>();
        matches.sort_unstable_by_key(|(index, _)| *index);
        if let Some((_, name)) = matches.first() {
            return Ok(*name);
        }
    }
    Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse))
}

fn find_js_function_call(source: &str, name: &str) -> Option<usize> {
    let mut search_from = 0_usize;
    while let Some(relative) = source[search_from..].find(name) {
        let position = search_from + relative;
        let before_ok = position == 0 || !is_js_identifier_byte(source.as_bytes()[position - 1]);
        let after_name = position + name.len();
        let cursor = skip_ascii_whitespace(source, after_name);
        if before_ok && source.as_bytes().get(cursor) == Some(&b'(') {
            return Some(position);
        }
        search_from = after_name;
    }
    None
}

fn parse_js_property_name(source: &str, position: usize) -> Option<(&str, usize)> {
    if matches!(source.as_bytes().get(position), Some(b'\'' | b'"')) {
        let quote = source.as_bytes()[position];
        let end = source.as_bytes()[position + 1..]
            .iter()
            .position(|byte| *byte == quote)?
            + position
            + 1;
        let name = &source[position + 1..end];
        valid_js_identifier(name).then_some((name, end + 1))
    } else {
        parse_js_identifier(source, position)
    }
}

fn parse_js_identifier(source: &str, position: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let first = *bytes.get(position)?;
    if !first.is_ascii_alphabetic() && !matches!(first, b'_' | b'$') {
        return None;
    }
    let mut end = position + 1;
    while bytes
        .get(end)
        .is_some_and(|byte| is_js_identifier_byte(*byte))
    {
        end += 1;
    }
    Some((&source[position..end], end))
}

fn valid_js_identifier(value: &str) -> bool {
    parse_js_identifier(value, 0).is_some_and(|(_, end)| end == value.len())
}

fn valid_oauth_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CREDENTIAL_BYTES
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

const fn is_js_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn skip_ascii_whitespace(source: &str, mut position: usize) -> usize {
    while source
        .as_bytes()
        .get(position)
        .is_some_and(u8::is_ascii_whitespace)
    {
        position += 1;
    }
    position
}

fn skip_ascii_whitespace_and_commas(source: &str, mut position: usize) -> usize {
    while source
        .as_bytes()
        .get(position)
        .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b',')
    {
        position += 1;
    }
    position
}

fn strip_ascii_whitespace(value: &str) -> Option<String> {
    value.bytes().all(|byte| byte.is_ascii()).then(|| {
        value
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .collect()
    })
}

fn find_matching_javascript(
    source: &str,
    open_position: usize,
    open: u8,
    close: u8,
) -> Result<usize, YoutubeError> {
    if source.as_bytes().get(open_position) != Some(&open) {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let bytes = source.as_bytes();
    let mut depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    let mut position = open_position;
    while position < bytes.len() {
        let byte = bytes[position];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
        } else if byte == b'/' && bytes.get(position + 1) == Some(&b'/') {
            position += 2;
            while position < bytes.len() && !matches!(bytes[position], b'\n' | b'\r') {
                position += 1;
            }
            continue;
        } else if byte == b'/' && bytes.get(position + 1) == Some(&b'*') {
            let remainder = &source[position + 2..];
            let comment_end = remainder
                .find("*/")
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
            position += comment_end + 4;
            continue;
        } else if byte == open {
            depth = depth
                .checked_add(1)
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        } else if byte == close {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
            if depth == 0 {
                return Ok(position);
            }
        }
        position += 1;
    }
    Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse))
}

/// One reconstructed `YouTube` track. Playback format selection is resolved separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YoutubeSourceTrack {
    pub info: TrackInfo,
}

/// A bounded `YouTube` playlist or search result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YoutubeSourcePlaylist {
    pub name: String,
    pub tracks: Vec<YoutubeSourceTrack>,
    pub selected_track: Option<usize>,
    pub is_search_result: bool,
}

/// Native item returned by the `YouTube` source manager.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YoutubeSourceItem {
    Track(YoutubeSourceTrack),
    Playlist(YoutubeSourcePlaylist),
}

/// One bounded player script acquisition used to seed playback requests.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubePlayerScript {
    url: String,
    signature_timestamp: u64,
    byte_len: usize,
}

impl YoutubePlayerScript {
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn signature_timestamp(&self) -> u64 {
        self.signature_timestamp
    }

    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}

impl fmt::Debug for YoutubePlayerScript {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubePlayerScript")
            .field("url", &"<redacted>")
            .field("signature_timestamp", &self.signature_timestamp)
            .field("byte_len", &self.byte_len)
            .finish()
    }
}

/// A transformed media URL whose diagnostics never expose query credentials.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubeResolvedPlaybackUrl {
    url: String,
}

/// Bounded inputs supplied to an optional current-player cipher provider.
pub struct YoutubeCipherChallenge<'a> {
    player_script_url: &'a str,
    player_script: &'a [u8],
    signature: Option<&'a str>,
    n_parameter: Option<&'a str>,
    max_output_bytes: usize,
    cancellation: &'a MediaCancellation,
}

impl YoutubeCipherChallenge<'_> {
    #[must_use]
    pub fn player_script_url(&self) -> &str {
        self.player_script_url
    }

    #[must_use]
    pub fn player_script(&self) -> &[u8] {
        self.player_script
    }

    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        self.signature
    }

    #[must_use]
    pub fn n_parameter(&self) -> Option<&str> {
        self.n_parameter
    }

    #[must_use]
    pub const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }

    #[must_use]
    pub const fn cancellation(&self) -> &MediaCancellation {
        self.cancellation
    }
}

impl fmt::Debug for YoutubeCipherChallenge<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeCipherChallenge")
            .field("player_script_url", &"<redacted>")
            .field("player_script_bytes", &self.player_script.len())
            .field("signature", &self.signature.is_some())
            .field("n_parameter", &self.n_parameter.is_some())
            .field("max_output_bytes", &self.max_output_bytes)
            .field("cancelled", &self.cancellation.is_cancelled())
            .finish()
    }
}

/// Credential-safe output from an optional current-player cipher provider.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubeCipherSolution {
    signature: Option<String>,
    n_parameter: Option<String>,
}

impl YoutubeCipherSolution {
    #[must_use]
    pub const fn new(signature: Option<String>, n_parameter: Option<String>) -> Self {
        Self {
            signature,
            n_parameter,
        }
    }
}

impl fmt::Debug for YoutubeCipherSolution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeCipherSolution")
            .field("signature", &self.signature.is_some())
            .field("n_parameter", &self.n_parameter.is_some())
            .finish()
    }
}

/// Stable error classes returned by an optional current-player cipher provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YoutubeCipherResolverErrorKind {
    UnsupportedScript,
    ExecutionFailed,
    Cancelled,
}

/// Credential-safe failure from an optional current-player cipher provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct YoutubeCipherResolverError {
    kind: YoutubeCipherResolverErrorKind,
}

impl YoutubeCipherResolverError {
    #[must_use]
    pub const fn new(kind: YoutubeCipherResolverErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> YoutubeCipherResolverErrorKind {
        self.kind
    }
}

impl fmt::Display for YoutubeCipherResolverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            YoutubeCipherResolverErrorKind::UnsupportedScript => {
                "YouTube cipher provider does not support this player script"
            }
            YoutubeCipherResolverErrorKind::ExecutionFailed => {
                "YouTube cipher provider execution failed"
            }
            YoutubeCipherResolverErrorKind::Cancelled => {
                "YouTube cipher provider execution cancelled"
            }
        })
    }
}

impl std::error::Error for YoutubeCipherResolverError {}

/// Optional isolated provider for current scripts outside the native narrow grammar.
pub trait YoutubeCipherResolver: Send + Sync {
    /// Resolves only the supplied bounded challenge fields.
    ///
    /// # Errors
    ///
    /// Returns a stable unsupported, execution, or cancellation class without diagnostic secrets.
    fn resolve(
        &self,
        challenge: &YoutubeCipherChallenge<'_>,
    ) -> Result<YoutubeCipherSolution, YoutubeCipherResolverError>;
}

impl YoutubeResolvedPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.url
    }
}

impl fmt::Debug for YoutubeResolvedPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeResolvedPlaybackUrl")
            .field("url", &"<redacted>")
            .finish()
    }
}

#[derive(Clone)]
struct YoutubeCipherProgram {
    signature: YoutubeTransformProgram,
    n_parameter: YoutubeTransformProgram,
}

#[derive(Clone)]
struct YoutubeTransformProgram {
    operations: Vec<YoutubeTransformOperation>,
}

#[derive(Clone, Copy)]
enum YoutubeTransformOperation {
    Reverse,
    Slice(usize),
    Swap(usize),
}

struct CachedYoutubePlayerScript {
    player_script: YoutubePlayerScript,
    source: Arc<[u8]>,
    cipher: Option<Result<YoutubeCipherProgram, YoutubeErrorKind>>,
    expires_at: Instant,
}

/// A playback container/codec combination understood by Mantle's current media pipeline.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum YoutubePlaybackFormatKind {
    HlsMpegTsAac,
    WebmOpus,
    WebmVorbis,
    Mp4AacLc,
    WebmVideoVorbis,
    Mp4VideoAacLc,
}

impl YoutubePlaybackFormatKind {
    const fn preference(self) -> u8 {
        match self {
            Self::HlsMpegTsAac => 0,
            Self::WebmOpus => 1,
            Self::WebmVorbis => 2,
            Self::Mp4AacLc => 3,
            Self::WebmVideoVorbis => 4,
            Self::Mp4VideoAacLc => 5,
        }
    }
}

/// One bounded playback candidate. Signed URLs and challenge inputs are omitted from diagnostics.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubePlaybackFormat {
    kind: Option<YoutubePlaybackFormatKind>,
    itag: i32,
    bitrate: u64,
    content_length: Option<u64>,
    audio_channels: u16,
    playback_url: String,
    n_parameter: Option<String>,
    signature: Option<String>,
    signature_key: String,
    default_audio_track: bool,
    drc: bool,
}

impl YoutubePlaybackFormat {
    #[must_use]
    pub const fn kind(&self) -> Option<YoutubePlaybackFormatKind> {
        self.kind
    }

    #[must_use]
    pub const fn itag(&self) -> i32 {
        self.itag
    }

    #[must_use]
    pub const fn bitrate(&self) -> u64 {
        self.bitrate
    }

    #[must_use]
    pub const fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    #[must_use]
    pub const fn audio_channels(&self) -> u16 {
        self.audio_channels
    }

    #[must_use]
    pub fn playback_url(&self) -> &str {
        &self.playback_url
    }

    #[must_use]
    pub fn n_parameter(&self) -> Option<&str> {
        self.n_parameter.as_deref()
    }

    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    #[must_use]
    pub fn signature_key(&self) -> &str {
        &self.signature_key
    }

    #[must_use]
    pub const fn is_default_audio_track(&self) -> bool {
        self.default_audio_track
    }

    #[must_use]
    pub const fn is_drc(&self) -> bool {
        self.drc
    }

    #[must_use]
    pub const fn requires_cipher(&self) -> bool {
        self.signature.is_some() || self.n_parameter.is_some()
    }
}

impl fmt::Debug for YoutubePlaybackFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubePlaybackFormat")
            .field("kind", &self.kind)
            .field("itag", &self.itag)
            .field("bitrate", &self.bitrate)
            .field("content_length", &self.content_length)
            .field("audio_channels", &self.audio_channels)
            .field("playback_url", &"<redacted>")
            .field("n_parameter", &self.n_parameter.is_some())
            .field("signature", &self.signature.is_some())
            .field("signature_key", &"<redacted>")
            .field("default_audio_track", &self.default_audio_track)
            .field("drc", &self.drc)
            .finish()
    }
}

/// Ordered candidates and the selected best format from one successful playback client.
#[derive(Clone, Eq, PartialEq)]
pub struct YoutubePlaybackFormats {
    client: YoutubeClientKind,
    formats: Vec<YoutubePlaybackFormat>,
    selected_index: usize,
}

impl YoutubePlaybackFormats {
    #[must_use]
    pub const fn client(&self) -> YoutubeClientKind {
        self.client
    }

    #[must_use]
    pub fn formats(&self) -> &[YoutubePlaybackFormat] {
        &self.formats
    }

    #[must_use]
    pub fn selected(&self) -> &YoutubePlaybackFormat {
        &self.formats[self.selected_index]
    }

    #[must_use]
    pub const fn requires_player_script(&self) -> bool {
        self.client.requires_player_script()
    }
}

impl fmt::Debug for YoutubePlaybackFormats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubePlaybackFormats")
            .field("client", &self.client)
            .field("format_count", &self.formats.len())
            .field("selected_index", &self.selected_index)
            .finish()
    }
}

/// Stable `YouTube`-specific failure classes kept free of identifiers and service response text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YoutubeErrorKind {
    InvalidOptions,
    InvalidAuthentication,
    Cancelled,
    Network,
    RateLimited,
    LoginRequired,
    AuthorizationPending,
    OAuthSlowDown,
    AccessDenied,
    ExpiredDeviceCode,
    Unavailable,
    InvalidResponse,
    UnsupportedRoute,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct YoutubeError {
    kind: YoutubeErrorKind,
    attempts: usize,
}

impl YoutubeError {
    const fn new(kind: YoutubeErrorKind) -> Self {
        Self { kind, attempts: 0 }
    }

    const fn with_attempts(kind: YoutubeErrorKind, attempts: usize) -> Self {
        Self { kind, attempts }
    }

    #[must_use]
    pub const fn kind(&self) -> YoutubeErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn attempts(&self) -> usize {
        self.attempts
    }
}

impl fmt::Debug for YoutubeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeError")
            .field("kind", &self.kind)
            .field("attempts", &self.attempts)
            .finish()
    }
}

impl fmt::Display for YoutubeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            YoutubeErrorKind::InvalidOptions => "invalid YouTube source policy",
            YoutubeErrorKind::InvalidAuthentication => "invalid YouTube authentication policy",
            YoutubeErrorKind::Cancelled => "YouTube load cancelled",
            YoutubeErrorKind::Network => "YouTube request failed",
            YoutubeErrorKind::RateLimited => "YouTube rate limit reached",
            YoutubeErrorKind::LoginRequired => "YouTube content requires authentication",
            YoutubeErrorKind::AuthorizationPending => "YouTube OAuth authorization is pending",
            YoutubeErrorKind::OAuthSlowDown => "YouTube OAuth polling must slow down",
            YoutubeErrorKind::AccessDenied => "YouTube OAuth access was denied",
            YoutubeErrorKind::ExpiredDeviceCode => "YouTube OAuth device code expired",
            YoutubeErrorKind::Unavailable => "YouTube content is unavailable",
            YoutubeErrorKind::InvalidResponse => "YouTube returned an invalid response",
            YoutubeErrorKind::UnsupportedRoute => "YouTube route is not implemented",
        })
    }
}

impl std::error::Error for YoutubeError {}

/// First-class source manager for current `YouTube` identifiers and ordered `InnerTube` clients.
pub struct YoutubeAudioSourceManager {
    options: YoutubeSourceOptions,
    authentication: YoutubeAuthentication,
    oauth: Mutex<YoutubeOAuthState>,
    oauth_clock: Arc<dyn YoutubeOAuthClock>,
    http: RemoteHttpClient,
    cipher_resolver: Option<Arc<dyn YoutubeCipherResolver>>,
    player_script: Mutex<Option<CachedYoutubePlayerScript>>,
    shutdown: AtomicBool,
}

impl YoutubeAudioSourceManager {
    /// Creates the manager after validating all client, parser, HTTP, and authentication bounds.
    ///
    /// # Errors
    ///
    /// Returns a policy error when options or credentials are invalid.
    pub fn new(
        options: YoutubeSourceOptions,
        authentication: YoutubeAuthentication,
    ) -> Result<Self, YoutubeError> {
        Self::new_inner(
            options,
            authentication,
            None,
            Arc::new(SystemYoutubeOAuthClock::default()),
        )
    }

    /// Creates a manager whose control-plane requests select and bind outbound routes.
    ///
    /// Pass the same policy to [`YoutubeAudioSourceManager::open_selected_playback_routed`] so
    /// discovery, cipher/control requests, and media range connections share `RoutePlanner` state.
    ///
    /// # Errors
    ///
    /// Returns a policy error when options or credentials are invalid.
    pub fn with_route_policy(
        options: YoutubeSourceOptions,
        authentication: YoutubeAuthentication,
        route_policy: Arc<dyn crate::OutboundRoutePolicy>,
    ) -> Result<Self, YoutubeError> {
        options.validate()?;
        authentication.validate()?;
        let oauth = Mutex::new(YoutubeOAuthState::new(&authentication));
        let http = RemoteHttpClient::with_route_policy(options.http, route_policy)
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            oauth,
            oauth_clock: Arc::new(SystemYoutubeOAuthClock::default()),
            http,
            cipher_resolver: None,
            player_script: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        })
    }

    /// Creates the manager with a caller-provided monotonic clock for deterministic token expiry.
    ///
    /// # Errors
    ///
    /// Returns a policy error when options or credentials are invalid.
    pub fn with_oauth_clock(
        options: YoutubeSourceOptions,
        authentication: YoutubeAuthentication,
        oauth_clock: Arc<dyn YoutubeOAuthClock>,
    ) -> Result<Self, YoutubeError> {
        Self::new_inner(options, authentication, None, oauth_clock)
    }

    /// Creates the manager with an isolated provider for current scripts outside the native
    /// transformation grammar.
    ///
    /// # Errors
    ///
    /// Returns a policy error when options or credentials are invalid.
    pub fn with_cipher_resolver(
        options: YoutubeSourceOptions,
        authentication: YoutubeAuthentication,
        cipher_resolver: Arc<dyn YoutubeCipherResolver>,
    ) -> Result<Self, YoutubeError> {
        Self::new_inner(
            options,
            authentication,
            Some(cipher_resolver),
            Arc::new(SystemYoutubeOAuthClock::default()),
        )
    }

    fn new_inner(
        options: YoutubeSourceOptions,
        authentication: YoutubeAuthentication,
        cipher_resolver: Option<Arc<dyn YoutubeCipherResolver>>,
        oauth_clock: Arc<dyn YoutubeOAuthClock>,
    ) -> Result<Self, YoutubeError> {
        options.validate()?;
        authentication.validate()?;
        let oauth = Mutex::new(YoutubeOAuthState::new(&authentication));
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            oauth,
            oauth_clock,
            http,
            cipher_resolver,
            player_script: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        })
    }

    /// Requests one bounded OAuth device code. The caller owns presentation and polling cadence.
    ///
    /// # Errors
    ///
    /// Returns a stable policy, cancellation, network, or invalid-response error.
    pub fn request_oauth_device_code(
        &self,
        device_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeOAuthDeviceCode, YoutubeError> {
        self.check_oauth_request(device_id, cancellation)?;
        let body = serde_json::to_vec(&serde_json::json!({
            "client_id": self.options.oauth.client_id,
            "scope": self.options.oauth.scopes,
            "device_id": device_id,
            "device_model": "ytlr::",
        }))
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let response: OAuthDeviceCodeResponse =
            self.execute_oauth_json(&self.options.oauth.device_code_url, body, cancellation)?;
        if !valid_oauth_value(&response.verification_url)
            || !valid_oauth_value(&response.user_code)
            || !valid_oauth_value(&response.device_code)
            || response.expires_in == 0
            || Duration::from_secs(response.expires_in) > MAX_OAUTH_TOKEN_LIFETIME
            || response.interval == 0
            || response.interval > response.expires_in
            || RemoteHttpRequest::get(&response.verification_url).is_err()
        {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        Ok(YoutubeOAuthDeviceCode {
            verification_url: response.verification_url,
            user_code: response.user_code,
            device_code: response.device_code,
            poll_interval: Duration::from_secs(response.interval),
            expires_in: Duration::from_secs(response.expires_in),
        })
    }

    /// Performs exactly one caller-scheduled device-code exchange and stores successful tokens.
    ///
    /// This method never sleeps or starts a polling thread. Pending and slow-down responses are
    /// returned as stable error kinds so the caller can honor the device-code cadence.
    ///
    /// # Errors
    ///
    /// Returns a stable OAuth, cancellation, network, or invalid-response error.
    pub fn exchange_oauth_device_code(
        &self,
        device_code: &str,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeOAuthTokenStatus, YoutubeError> {
        self.check_oauth_request(device_code, cancellation)?;
        let body = serde_json::to_vec(&serde_json::json!({
            "client_id": self.options.oauth.client_id,
            "client_secret": self.options.oauth.client_secret,
            "code": device_code,
            "grant_type": OAUTH_DEVICE_GRANT_TYPE,
        }))
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let response =
            self.execute_oauth_json(&self.options.oauth.token_url, body, cancellation)?;
        let mut state = self
            .oauth
            .lock()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        self.apply_oauth_token_response(&mut state, response)
    }

    /// Refreshes the access token once, or returns the still-valid cached token status.
    ///
    /// # Errors
    ///
    /// Returns invalid-authentication when no refresh token exists, plus stable cancellation,
    /// network, OAuth, and response errors.
    pub fn refresh_oauth_access_token(
        &self,
        force: bool,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeOAuthTokenStatus, YoutubeError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        if cancellation.is_cancelled() {
            return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
        }
        let mut state = self
            .oauth
            .lock()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        if !force
            && state.access_token.is_some()
            && let Some(expires_at) = state.expires_at
            && expires_at > self.oauth_clock.now()
        {
            return Ok(YoutubeOAuthTokenStatus {
                expires_in: expires_at.saturating_sub(self.oauth_clock.now()),
                refresh_token_rotated: false,
            });
        }
        self.refresh_oauth_locked(&mut state, cancellation)
    }

    /// Returns the current refresh token for explicit secure persistence by the caller.
    #[must_use]
    pub fn oauth_refresh_token(&self) -> Option<String> {
        self.oauth
            .lock()
            .ok()
            .and_then(|state| state.refresh_token.clone())
    }

    fn check_oauth_request(
        &self,
        value: &str,
        cancellation: &MediaCancellation,
    ) -> Result<(), YoutubeError> {
        if self.shutdown.load(Ordering::Acquire) || !valid_oauth_value(value) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        if cancellation.is_cancelled() {
            return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
        }
        Ok(())
    }

    fn execute_oauth_json<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: Vec<u8>,
        cancellation: &MediaCancellation,
    ) -> Result<T, YoutubeError> {
        let request = RemoteHttpRequest::post(endpoint, body)
            .and_then(|request| request.header("Content-Type", "application/json"))
            .and_then(|request| request.max_response_bytes(self.options.oauth.max_response_bytes))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?
            .retry_mode(RemoteRetryMode::Never);
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(|error| match error.kind() {
                RemoteHttpErrorKind::InvalidRequest | RemoteHttpErrorKind::RequestTooLarge => {
                    YoutubeError::new(YoutubeErrorKind::InvalidOptions)
                }
                _ => map_remote_error(error),
            })?;
        serde_json::from_slice(response.body())
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))
    }

    fn refresh_oauth_locked(
        &self,
        state: &mut YoutubeOAuthState,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeOAuthTokenStatus, YoutubeError> {
        let refresh_token = state
            .refresh_token
            .as_deref()
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidAuthentication))?;
        let body = serde_json::to_vec(&serde_json::json!({
            "client_id": self.options.oauth.client_id,
            "client_secret": self.options.oauth.client_secret,
            "refresh_token": refresh_token,
            "grant_type": "refresh_token",
        }))
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let response =
            self.execute_oauth_json(&self.options.oauth.token_url, body, cancellation)?;
        self.apply_oauth_token_response(state, response)
    }

    fn apply_oauth_token_response(
        &self,
        state: &mut YoutubeOAuthState,
        response: OAuthTokenResponse,
    ) -> Result<YoutubeOAuthTokenStatus, YoutubeError> {
        if let Some(error) = response.error.as_deref() {
            return Err(YoutubeError::new(match error {
                "authorization_pending" => YoutubeErrorKind::AuthorizationPending,
                "slow_down" => YoutubeErrorKind::OAuthSlowDown,
                "access_denied" => YoutubeErrorKind::AccessDenied,
                "expired_token" => YoutubeErrorKind::ExpiredDeviceCode,
                _ => YoutubeErrorKind::InvalidResponse,
            }));
        }
        let access_token = response
            .access_token
            .filter(|value| valid_oauth_value(value))
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let token_type = response
            .token_type
            .filter(|value| valid_oauth_value(value))
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let expires_in = Duration::from_secs(response.expires_in.unwrap_or(300));
        if expires_in.is_zero() || expires_in > MAX_OAUTH_TOKEN_LIFETIME {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let authorization = format!("{token_type} {access_token}");
        RemoteHttpRequest::get(DEFAULT_PLAYER_EMBED_URL)
            .and_then(|request| request.header("Authorization", &authorization))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let refresh_token_rotated = response.refresh_token.is_some();
        if let Some(refresh_token) = response.refresh_token {
            if !valid_oauth_value(&refresh_token) {
                return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
            }
            state.refresh_token = Some(refresh_token);
        }
        state.access_token = Some(access_token);
        state.token_type = token_type;
        state.expires_at = Some(
            self.oauth_clock
                .now()
                .saturating_add(expires_in.saturating_sub(self.options.oauth.expiry_skew)),
        );
        Ok(YoutubeOAuthTokenStatus {
            expires_in,
            refresh_token_rotated,
        })
    }

    fn oauth_authorization_header(
        &self,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<Option<String>, YoutubeError> {
        if !client.supports_oauth() {
            return Ok(None);
        }
        let mut state = self
            .oauth
            .lock()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let expired = state
            .expires_at
            .is_some_and(|expires_at| expires_at <= self.oauth_clock.now());
        if (state.access_token.is_none() || expired) && state.refresh_token.is_some() {
            self.refresh_oauth_locked(&mut state, cancellation)?;
        }
        let Some(access_token) = state.access_token.as_deref() else {
            return Ok(None);
        };
        Ok(Some(format!("{} {access_token}", state.token_type)))
    }

    /// Acquires and caches the bounded base player script and its signature timestamp.
    ///
    /// The script source is retained privately for the later cipher stage. Diagnostics redact its
    /// URL and never expose source bytes.
    ///
    /// # Errors
    ///
    /// Returns a stable cancellation, network, or invalid-response error when discovery fails.
    pub fn acquire_player_script(
        &self,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubePlayerScript, YoutubeError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        if cancellation.is_cancelled() {
            return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
        }
        let mut cache = self
            .player_script
            .lock()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let now = Instant::now();
        if let Some(cached) = cache.as_ref()
            && cached.expires_at > now
        {
            return Ok(cached.player_script.clone());
        }

        let embed_request = RemoteHttpRequest::get(&self.options.player_embed_url)
            .and_then(|request| request.max_response_bytes(self.options.max_player_embed_bytes))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let embed = self
            .http
            .execute_with_cancellation(&embed_request, cancellation)
            .map_err(map_remote_error)?;
        let script_reference = extract_player_script_reference(embed.body())
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        if script_reference.len() > self.options.max_player_script_url_bytes {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let script_url = resolve_player_script_url(
            &self.options.player_embed_url,
            &script_reference,
            self.options.max_player_script_url_bytes,
        )?;
        let script_request = RemoteHttpRequest::get(&script_url)
            .and_then(|request| request.max_response_bytes(self.options.max_player_script_bytes))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let source = self
            .http
            .execute_with_cancellation(&script_request, cancellation)
            .map_err(map_remote_error)?
            .into_body();
        let signature_timestamp = extract_signature_timestamp(&source)
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let player_script = YoutubePlayerScript {
            url: script_url,
            signature_timestamp,
            byte_len: source.len(),
        };
        *cache = Some(CachedYoutubePlayerScript {
            player_script: player_script.clone(),
            source: source.into(),
            cipher: None,
            expires_at: Instant::now() + self.options.player_script_cache_ttl,
        });
        Ok(player_script)
    }

    /// Resolves the selected format's signature and `n` challenges with either the configured
    /// provider or the cached bounded native player-script program.
    ///
    /// # Errors
    ///
    /// Returns a stable policy, cancellation, network, or invalid-response error. Unsupported
    /// player-script syntax fails closed and is negatively cached for that script version.
    pub fn resolve_selected_playback_url(
        &self,
        formats: &YoutubePlaybackFormats,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeResolvedPlaybackUrl, YoutubeError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        if cancellation.is_cancelled() {
            return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
        }
        let format = formats.selected();
        if !formats.client.requires_player_script() {
            if format.signature.is_some() {
                return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
            }
            return Ok(YoutubeResolvedPlaybackUrl {
                url: format.playback_url.clone(),
            });
        }
        if format.signature.is_none() && format.n_parameter.is_none() {
            return Ok(YoutubeResolvedPlaybackUrl {
                url: format.playback_url.clone(),
            });
        }

        let player_script = self.acquire_player_script(cancellation)?;
        let solution = self.resolve_cipher_solution(&player_script, format, cancellation)?;
        validate_cipher_solution(&solution, format, self.options.max_cipher_input_bytes)?;

        let mut url = format.playback_url.clone();
        if let Some(transformed) = solution.signature.as_deref() {
            url = set_url_query_parameter(
                &url,
                &format.signature_key,
                transformed,
                false,
                self.options.max_playback_url_bytes,
                self.options.max_cipher_input_bytes,
            )?;
        }
        if let Some(transformed) = solution.n_parameter.as_deref() {
            url = set_url_query_parameter(
                &url,
                "n",
                transformed,
                true,
                self.options.max_playback_url_bytes,
                self.options.max_cipher_input_bytes,
            )?;
        }
        Ok(YoutubeResolvedPlaybackUrl { url })
    }

    fn resolve_cipher_solution(
        &self,
        player_script: &YoutubePlayerScript,
        format: &YoutubePlaybackFormat,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeCipherSolution, YoutubeError> {
        if let Some(resolver) = self.cipher_resolver.as_ref() {
            let source = {
                let cache = self
                    .player_script
                    .lock()
                    .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
                cache
                    .as_ref()
                    .filter(|cached| cached.player_script.url == player_script.url)
                    .map(|cached| Arc::clone(&cached.source))
                    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?
            };
            let challenge = YoutubeCipherChallenge {
                player_script_url: &player_script.url,
                player_script: &source,
                signature: format.signature.as_deref(),
                n_parameter: format.n_parameter.as_deref(),
                max_output_bytes: self.options.max_cipher_input_bytes,
                cancellation,
            };
            let solution = resolver
                .resolve(&challenge)
                .map_err(|error| match error.kind() {
                    YoutubeCipherResolverErrorKind::Cancelled => {
                        YoutubeError::new(YoutubeErrorKind::Cancelled)
                    }
                    YoutubeCipherResolverErrorKind::UnsupportedScript
                    | YoutubeCipherResolverErrorKind::ExecutionFailed => {
                        YoutubeError::new(YoutubeErrorKind::InvalidResponse)
                    }
                })?;
            if cancellation.is_cancelled() {
                return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
            }
            Ok(solution)
        } else {
            let program = {
                let mut cache = self
                    .player_script
                    .lock()
                    .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
                let cached = cache
                    .as_mut()
                    .filter(|cached| cached.player_script.url == player_script.url)
                    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
                if cached.cipher.is_none() {
                    cached.cipher = Some(
                        parse_youtube_cipher_program(
                            &cached.source,
                            self.options.max_cipher_operations,
                        )
                        .map_err(|error| error.kind),
                    );
                }
                cached
                    .cipher
                    .as_ref()
                    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?
                    .clone()
                    .map_err(YoutubeError::new)?
            };
            Ok(YoutubeCipherSolution {
                signature: format
                    .signature
                    .as_deref()
                    .map(|signature| {
                        program
                            .signature
                            .apply(signature, self.options.max_cipher_input_bytes)
                    })
                    .transpose()?,
                n_parameter: format
                    .n_parameter
                    .as_deref()
                    .map(|n_parameter| {
                        program
                            .n_parameter
                            .apply(n_parameter, self.options.max_cipher_input_bytes)
                    })
                    .transpose()?,
            })
        }
    }

    /// Discovers and ranks bounded playback candidates using playback-capable clients in order.
    ///
    /// This does not resolve signature or `n` challenges. Those inputs remain redacted on the
    /// returned format for the local cipher stage.
    ///
    /// # Errors
    ///
    /// Returns a stable policy, cancellation, network, playability, or response error when every
    /// eligible client fails.
    pub fn discover_playback_formats(
        &self,
        video_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubePlaybackFormats, YoutubeError> {
        if self.shutdown.load(Ordering::Acquire) || !valid_video_id(video_id) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidOptions));
        }
        let mut attempts = 0_usize;
        let mut final_kind = YoutubeErrorKind::UnsupportedRoute;
        for client in self
            .options
            .clients
            .iter()
            .copied()
            .filter(|client| client.supports_playback())
        {
            if cancellation.is_cancelled() {
                return Err(YoutubeError::with_attempts(
                    YoutubeErrorKind::Cancelled,
                    attempts,
                ));
            }
            attempts += 1;
            let result = (|| {
                let signature_timestamp = if client.requires_player_script() {
                    Some(
                        self.acquire_player_script(cancellation)?
                            .signature_timestamp(),
                    )
                } else {
                    None
                };
                let authorization = self.oauth_authorization_header(client, cancellation)?;
                self.player_request(
                    video_id,
                    client,
                    signature_timestamp,
                    authorization.as_deref(),
                )
                .and_then(|request| {
                    self.http
                        .execute_with_cancellation(&request, cancellation)
                        .map_err(map_remote_error)
                })
                .and_then(|response| {
                    parse_playback_response(
                        response.body(),
                        video_id,
                        client,
                        self.options.max_playback_formats,
                        self.options.max_metadata_string_bytes,
                        self.options.max_playback_url_bytes,
                    )
                })
            })();
            match result {
                Ok(formats) => return Ok(formats),
                Err(error) if error.kind == YoutubeErrorKind::Cancelled => return Err(error),
                Err(error) => final_kind = error.kind,
            }
        }
        Err(YoutubeError::with_attempts(final_kind, attempts))
    }

    fn load_video(
        &self,
        video_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeSourceTrack, YoutubeError> {
        let mut attempts = 0;
        let mut final_kind = YoutubeErrorKind::UnsupportedRoute;
        for client in self
            .options
            .clients
            .iter()
            .copied()
            .filter(|client| client.supports_video_loading())
        {
            if cancellation.is_cancelled() {
                return Err(YoutubeError::with_attempts(
                    YoutubeErrorKind::Cancelled,
                    attempts,
                ));
            }
            attempts += 1;
            match self.load_video_with_client(video_id, client, cancellation) {
                Ok(track) => return Ok(track),
                Err(error) if error.kind == YoutubeErrorKind::Cancelled => return Err(error),
                Err(error) => final_kind = error.kind,
            }
        }
        Err(YoutubeError::with_attempts(final_kind, attempts))
    }

    fn load_video_with_client(
        &self,
        video_id: &str,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<YoutubeSourceTrack, YoutubeError> {
        let request = self.player_request(video_id, client, None, None)?;
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?;
        parse_player_response(
            response.body(),
            video_id,
            self.options.max_metadata_string_bytes,
            self.options.max_thumbnails,
        )
    }

    fn load_search(
        &self,
        query: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let _ = bounded_text(query, self.options.max_metadata_string_bytes)?;
        self.load_collection_with_clients(YoutubeClientKind::supports_search, |client| {
            self.load_search_with_client(query, client, cancellation)
        })
    }

    fn load_search_with_client(
        &self,
        query: &str,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let mut fields = Map::new();
        fields.insert("query".to_owned(), Value::String(query.to_owned()));
        fields.insert("params".to_owned(), Value::String(SEARCH_PARAMS.to_owned()));
        let request =
            self.data_request(&self.options.api_base_url, "search", client, fields, None)?;
        let bytes = self.execute_data_request(&request, cancellation)?;
        parse_search_response(
            &bytes,
            query,
            self.options.max_search_results,
            self.options.max_metadata_string_bytes,
            self.options.max_thumbnails,
        )
    }

    fn load_music_search(
        &self,
        query: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let _ = bounded_text(query, self.options.max_metadata_string_bytes)?;
        self.load_collection_with_clients(YoutubeClientKind::supports_music_search, |client| {
            self.load_music_search_with_client(query, client, cancellation)
        })
    }

    fn load_music_search_with_client(
        &self,
        query: &str,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let mut fields = Map::new();
        fields.insert("query".to_owned(), Value::String(query.to_owned()));
        fields.insert(
            "params".to_owned(),
            Value::String(MUSIC_SEARCH_PARAMS.to_owned()),
        );
        let request = self.data_request(
            &self.options.music_api_base_url,
            "search",
            client,
            fields,
            Some("music.youtube.com"),
        )?;
        let bytes = self.execute_data_request(&request, cancellation)?;
        parse_music_search_response(
            &bytes,
            query,
            self.options.max_search_results,
            self.options.max_metadata_string_bytes,
        )
    }

    fn load_playlist(
        &self,
        playlist_id: &str,
        selected_video_id: Option<&str>,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let _ = bounded_text(playlist_id, self.options.max_metadata_string_bytes)?;
        self.load_collection_with_clients(YoutubeClientKind::supports_playlist_loading, |client| {
            self.load_playlist_with_client(playlist_id, selected_video_id, client, cancellation)
        })
    }

    fn load_playlist_with_client(
        &self,
        playlist_id: &str,
        selected_video_id: Option<&str>,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let mut fields = Map::new();
        fields.insert(
            "browseId".to_owned(),
            Value::String(format!("VL{playlist_id}")),
        );
        let request =
            self.data_request(&self.options.api_base_url, "browse", client, fields, None)?;
        let bytes = self.execute_data_request(&request, cancellation)?;
        let mut parsed = parse_playlist_first_page(
            &bytes,
            self.options.max_playlist_tracks,
            self.options.max_metadata_string_bytes,
            self.options.max_thumbnails,
        )?;

        let mut pages = 1_usize;
        while let Some(continuation) = parsed.continuation.take() {
            if pages >= self.options.max_playlist_pages
                || parsed.tracks.len() >= self.options.max_playlist_tracks
            {
                break;
            }
            if cancellation.is_cancelled() {
                return Err(YoutubeError::new(YoutubeErrorKind::Cancelled));
            }
            let mut fields = Map::new();
            fields.insert("continuation".to_owned(), Value::String(continuation));
            let request =
                self.data_request(&self.options.api_base_url, "browse", client, fields, None)?;
            let bytes = self.execute_data_request(&request, cancellation)?;
            let remaining = self
                .options
                .max_playlist_tracks
                .checked_sub(parsed.tracks.len())
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
            let continuation_page = parse_playlist_continuation_page(
                &bytes,
                remaining,
                self.options.max_metadata_string_bytes,
                self.options.max_thumbnails,
            )?;
            parsed.tracks.extend(continuation_page.tracks);
            parsed.continuation = continuation_page.continuation;
            pages += 1;
        }
        if parsed.tracks.is_empty() {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let selected_track = selected_video_id.and_then(|selected| {
            parsed
                .tracks
                .iter()
                .position(|track| track.info.identifier == selected)
        });
        Ok(Some(YoutubeSourcePlaylist {
            name: parsed
                .name
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?,
            tracks: parsed.tracks,
            selected_track,
            is_search_result: false,
        }))
    }

    fn load_mix(
        &self,
        playlist_id: &str,
        selected_video_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let _ = bounded_text(playlist_id, self.options.max_metadata_string_bytes)?;
        if !valid_video_id(selected_video_id) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        self.load_collection_with_clients(YoutubeClientKind::supports_playlist_loading, |client| {
            self.load_mix_with_client(playlist_id, selected_video_id, client, cancellation)
        })
    }

    fn load_mix_with_client(
        &self,
        playlist_id: &str,
        selected_video_id: &str,
        client: YoutubeClientKind,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let mut fields = Map::new();
        fields.insert(
            "playlistId".to_owned(),
            Value::String(playlist_id.to_owned()),
        );
        fields.insert(
            "videoId".to_owned(),
            Value::String(selected_video_id.to_owned()),
        );
        let request =
            self.data_request(&self.options.api_base_url, "next", client, fields, None)?;
        let bytes = self.execute_data_request(&request, cancellation)?;
        parse_mix_response(
            &bytes,
            selected_video_id,
            self.options.max_mix_tracks,
            self.options.max_metadata_string_bytes,
            self.options.max_thumbnails,
        )
        .map(Some)
    }

    fn load_collection_with_clients(
        &self,
        supports: impl Fn(YoutubeClientKind) -> bool,
        mut load: impl FnMut(YoutubeClientKind) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError>,
    ) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
        let mut attempted = false;
        let mut final_error = YoutubeError::new(YoutubeErrorKind::UnsupportedRoute);
        for client in self
            .options
            .clients
            .iter()
            .copied()
            .filter(|client| supports(*client))
        {
            attempted = true;
            match load(client) {
                Ok(result) => return Ok(result),
                Err(error) if error.kind == YoutubeErrorKind::Cancelled => return Err(error),
                Err(error) => final_error = error,
            }
        }
        if attempted {
            Err(final_error)
        } else {
            Err(YoutubeError::new(YoutubeErrorKind::UnsupportedRoute))
        }
    }

    fn execute_data_request(
        &self,
        request: &RemoteHttpRequest,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, YoutubeError> {
        self.http
            .execute_with_cancellation(request, cancellation)
            .map(|response| response.body().to_vec())
            .map_err(map_remote_error)
    }

    fn data_request(
        &self,
        base_url: &str,
        operation: &str,
        client: YoutubeClientKind,
        mut root: Map<String, Value>,
        referer: Option<&str>,
    ) -> Result<RemoteHttpRequest, YoutubeError> {
        let mut client_fields = Map::new();
        client_fields.insert(
            "clientName".to_owned(),
            Value::String(client.identifier().to_owned()),
        );
        client_fields.insert(
            "clientVersion".to_owned(),
            Value::String(client.version().to_owned()),
        );
        if client == YoutubeClientKind::AndroidVr {
            client_fields.insert("androidSdkVersion".to_owned(), Value::Number(32.into()));
        }
        if client.uses_proof_of_origin()
            && let Some(visitor_data) = &self.authentication.visitor_data
        {
            client_fields.insert(
                "visitorData".to_owned(),
                Value::String(visitor_data.clone()),
            );
        }
        let mut context = Map::new();
        context.insert("client".to_owned(), Value::Object(client_fields));
        if client == YoutubeClientKind::Web {
            let mut user = Map::new();
            user.insert("lockedSafetyMode".to_owned(), Value::Bool(false));
            context.insert("user".to_owned(), Value::Object(user));
        }
        root.insert("context".to_owned(), Value::Object(context));
        if client.uses_proof_of_origin()
            && let Some(po_token) = &self.authentication.po_token
        {
            let mut integrity = Map::new();
            integrity.insert("poToken".to_owned(), Value::String(po_token.clone()));
            root.insert(
                "serviceIntegrityDimensions".to_owned(),
                Value::Object(integrity),
            );
        }

        let body = serde_json::to_vec(&Value::Object(root))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let endpoint = format!(
            "{}/{operation}?prettyPrint=false",
            base_url.trim_end_matches('/')
        );
        let mut request = RemoteHttpRequest::post(endpoint, body)
            .and_then(|request| request.header("Content-Type", "application/json"))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?
            .retry_mode(RemoteRetryMode::Idempotent);
        if let Some(user_agent) = client.user_agent() {
            request = request
                .header("User-Agent", user_agent)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        }
        if client.uses_proof_of_origin()
            && let Some(visitor_data) = &self.authentication.visitor_data
        {
            request = request
                .header("X-Goog-Visitor-Id", visitor_data)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidAuthentication))?;
        }
        if let Some(referer) = referer {
            request = request
                .header("Referer", referer)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        }
        Ok(request)
    }

    fn player_request(
        &self,
        video_id: &str,
        client: YoutubeClientKind,
        signature_timestamp: Option<u64>,
        authorization: Option<&str>,
    ) -> Result<RemoteHttpRequest, YoutubeError> {
        let mut client_fields = Map::new();
        client_fields.insert(
            "clientName".to_owned(),
            Value::String(client.identifier().to_owned()),
        );
        client_fields.insert(
            "clientVersion".to_owned(),
            Value::String(client.version().to_owned()),
        );
        if client != YoutubeClientKind::Tv {
            client_fields.insert("clientScreen".to_owned(), Value::String("EMBED".to_owned()));
        }
        if client == YoutubeClientKind::AndroidVr {
            client_fields.insert("androidSdkVersion".to_owned(), Value::Number(32.into()));
        }
        if client.uses_proof_of_origin()
            && let Some(visitor_data) = &self.authentication.visitor_data
        {
            client_fields.insert(
                "visitorData".to_owned(),
                Value::String(visitor_data.clone()),
            );
        }
        let mut context = Map::new();
        context.insert("client".to_owned(), Value::Object(client_fields));
        if client != YoutubeClientKind::Tv {
            let mut third_party = Map::new();
            third_party.insert(
                "embedUrl".to_owned(),
                Value::String("https://google.com".to_owned()),
            );
            context.insert("thirdParty".to_owned(), Value::Object(third_party));
        }

        let mut root = Map::new();
        root.insert("context".to_owned(), Value::Object(context));
        if let Some(signature_timestamp) = signature_timestamp {
            let mut content_playback_context = Map::new();
            content_playback_context.insert(
                "signatureTimestamp".to_owned(),
                Value::String(signature_timestamp.to_string()),
            );
            let mut playback_context = Map::new();
            playback_context.insert(
                "contentPlaybackContext".to_owned(),
                Value::Object(content_playback_context),
            );
            root.insert(
                "playbackContext".to_owned(),
                Value::Object(playback_context),
            );
        }
        root.insert("videoId".to_owned(), Value::String(video_id.to_owned()));
        root.insert("racyCheckOk".to_owned(), Value::Bool(true));
        root.insert("contentCheckOk".to_owned(), Value::Bool(true));
        if let Some(params) = client.player_params() {
            root.insert("params".to_owned(), Value::String(params.to_owned()));
        }
        if client.uses_proof_of_origin()
            && let Some(po_token) = &self.authentication.po_token
        {
            let mut integrity = Map::new();
            integrity.insert("poToken".to_owned(), Value::String(po_token.clone()));
            root.insert(
                "serviceIntegrityDimensions".to_owned(),
                Value::Object(integrity),
            );
        }
        let body = serde_json::to_vec(&Value::Object(root))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        let endpoint = format!(
            "{}/player?prettyPrint=false",
            self.options.api_base_url.trim_end_matches('/')
        );
        let mut request = RemoteHttpRequest::post(endpoint, body)
            .and_then(|request| request.header("Content-Type", "application/json"))
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?
            .retry_mode(RemoteRetryMode::Idempotent);
        if let Some(user_agent) = client.user_agent() {
            request = request
                .header("User-Agent", user_agent)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidOptions))?;
        }
        if client.uses_proof_of_origin()
            && let Some(visitor_data) = &self.authentication.visitor_data
        {
            request = request
                .header("X-Goog-Visitor-Id", visitor_data)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidAuthentication))?;
        }
        if client.supports_oauth()
            && let Some(authorization) = authorization
        {
            request = request
                .header("Authorization", authorization)
                .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidAuthentication))?;
        }
        Ok(request)
    }
}

impl fmt::Debug for YoutubeAudioSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YoutubeAudioSourceManager")
            .field("client_count", &self.options.clients.len())
            .field("authentication", &self.authentication)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<YoutubeSourceItem> for YoutubeAudioSourceManager {
    fn source_name(&self) -> &'static str {
        "youtube"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<YoutubeSourceItem>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<YoutubeSourceItem>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_youtube_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        match route {
            YoutubeRoute::NoTrack => Ok(Some(SourceLoad::Referral(SourceReference::new(
                None, false,
            )))),
            YoutubeRoute::Video(video_id) => {
                let linked = linked_cancellation(cancellation);
                match self.load_video(&video_id, &linked) {
                    Ok(track) => Ok(Some(SourceLoad::Item(YoutubeSourceItem::Track(track)))),
                    Err(error) if error.kind == YoutubeErrorKind::Cancelled => Ok(None),
                    Err(_) => Err(SourceRegistryError::SourceFailure),
                }
            }
            YoutubeRoute::Playlist {
                playlist_id,
                selected_video_id,
            } => {
                let linked = linked_cancellation(cancellation);
                map_playlist_load(self.load_playlist(
                    &playlist_id,
                    selected_video_id.as_deref(),
                    &linked,
                ))
            }
            YoutubeRoute::Search(query) => {
                let linked = linked_cancellation(cancellation);
                map_playlist_load(self.load_search(&query, &linked))
            }
            YoutubeRoute::MusicSearch(query) => {
                let linked = linked_cancellation(cancellation);
                map_playlist_load(self.load_music_search(&query, &linked))
            }
            YoutubeRoute::Mix {
                playlist_id,
                selected_video_id,
            } => {
                let linked = linked_cancellation(cancellation);
                map_playlist_load(self.load_mix(&playlist_id, &selected_video_id, &linked))
            }
        }
    }

    fn is_encodable(&self, item: &YoutubeSourceItem) -> bool {
        matches!(item, YoutubeSourceItem::Track(_))
    }

    fn encode(&self, item: &YoutubeSourceItem) -> Result<Vec<u8>, SourceRegistryError> {
        if matches!(item, YoutubeSourceItem::Track(_)) {
            Ok(Vec::new())
        } else {
            Err(SourceRegistryError::NotEncodable)
        }
    }

    fn decode(&self, _payload: &[u8]) -> Result<YoutubeSourceItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<YoutubeSourceItem, SourceRegistryError> {
        if !payload.is_empty() || !valid_video_id(&info.identifier) {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(YoutubeSourceItem::Track(YoutubeSourceTrack {
            info: info.clone(),
        }))
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn linked_cancellation(cancellation: &SourceCancellation) -> MediaCancellation {
    let cancellation = cancellation.clone();
    MediaCancellation::linked(move || cancellation.is_cancelled())
}

fn map_playlist_load(
    result: Result<Option<YoutubeSourcePlaylist>, YoutubeError>,
) -> Result<Option<SourceLoad<YoutubeSourceItem>>, SourceRegistryError> {
    match result {
        Ok(Some(playlist)) => Ok(Some(SourceLoad::Item(YoutubeSourceItem::Playlist(
            playlist,
        )))),
        Ok(None) => Ok(Some(SourceLoad::Referral(SourceReference::new(
            None, false,
        )))),
        Err(error) if error.kind == YoutubeErrorKind::Cancelled => Ok(None),
        Err(_) => Err(SourceRegistryError::SourceFailure),
    }
}

fn map_remote_error(error: crate::RemoteHttpError) -> YoutubeError {
    let kind = match error.kind() {
        RemoteHttpErrorKind::Cancelled => YoutubeErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => YoutubeErrorKind::RateLimited,
        RemoteHttpErrorKind::Unauthorized | RemoteHttpErrorKind::Forbidden => {
            YoutubeErrorKind::LoginRequired
        }
        RemoteHttpErrorKind::NotFound => YoutubeErrorKind::Unavailable,
        RemoteHttpErrorKind::InvalidResponse | RemoteHttpErrorKind::ResponseTooLarge => {
            YoutubeErrorKind::InvalidResponse
        }
        _ => YoutubeErrorKind::Network,
    };
    YoutubeError::new(kind)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerResponse {
    playability_status: Option<PlayabilityStatus>,
    video_details: Option<VideoDetails>,
}

#[derive(Deserialize)]
struct PlayabilityStatus {
    status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoDetails {
    video_id: Option<String>,
    title: Option<String>,
    author: Option<String>,
    length_seconds: Option<String>,
    #[serde(default)]
    is_live: bool,
    thumbnail: Option<ThumbnailCollection>,
}

#[derive(Deserialize)]
struct ThumbnailCollection {
    #[serde(default)]
    thumbnails: Vec<Thumbnail>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: Option<String>,
}

fn parse_player_response(
    bytes: &[u8],
    requested_video_id: &str,
    max_metadata_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<YoutubeSourceTrack, YoutubeError> {
    let response: PlayerResponse = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    // Metadata loading is independent of format availability, as in the pinned
    // youtube-source NonMusicClient.loadVideo. A client may expose public details
    // while refusing playback. Format discovery still validates playability.
    if response.video_details.is_none() {
        let status = response
            .playability_status
            .and_then(|status| status.status)
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        match status.as_str() {
            "OK" => {}
            "LOGIN_REQUIRED" | "CONTENT_CHECK_REQUIRED" => {
                return Err(YoutubeError::new(YoutubeErrorKind::LoginRequired));
            }
            "ERROR" | "UNPLAYABLE" | "LIVE_STREAM_OFFLINE" => {
                return Err(YoutubeError::new(YoutubeErrorKind::Unavailable));
            }
            _ => return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse)),
        }
    }
    let details = response
        .video_details
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let video_id = details
        .video_id
        .filter(|video_id| video_id == requested_video_id)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let title = bounded_required_string(details.title, max_metadata_string_bytes)?;
    let author = match details.author {
        Some(author) if !author.is_empty() && author.len() <= max_metadata_string_bytes => author,
        Some(author) if author.len() > max_metadata_string_bytes => {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        _ => "Unknown artist".to_owned(),
    };
    let duration = if details.is_live {
        Duration::ZERO
    } else {
        let seconds = details
            .length_seconds
            .as_deref()
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?
            .parse::<u64>()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        Duration::from_secs(seconds)
    };
    let artwork_url = match details.thumbnail {
        Some(thumbnail) => {
            if thumbnail.thumbnails.len() > max_thumbnails {
                return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
            }
            thumbnail
                .thumbnails
                .into_iter()
                .rev()
                .filter_map(|thumbnail| thumbnail.url)
                .find(|url| {
                    url.len() <= max_metadata_string_bytes
                        && (url.starts_with("https://") || url.starts_with("http://"))
                })
        }
        None => None,
    };
    Ok(YoutubeSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier: video_id.clone(),
            is_stream: details.is_live,
            uri: Some(format!("{WATCH_URL_PREFIX}{video_id}")),
            artwork_url,
            isrc: None,
        },
    })
}

fn bounded_required_string(value: Option<String>, limit: usize) -> Result<String, YoutubeError> {
    value
        .filter(|value| !value.is_empty() && value.len() <= limit)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))
}

fn parse_playback_response(
    bytes: &[u8],
    requested_video_id: &str,
    client: YoutubeClientKind,
    max_formats: usize,
    max_string_bytes: usize,
    max_url_bytes: usize,
) -> Result<YoutubePlaybackFormats, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    validate_playability_value(&json)?;
    let details = json
        .get("videoDetails")
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    if bounded_value_text(details.get("videoId"), max_string_bytes)?.as_deref()
        != Some(requested_video_id)
    {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let is_live = details
        .get("isLive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let streaming_data = json
        .get("streamingData")
        .and_then(Value::as_object)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let merged = optional_value_array(streaming_data.get("formats"))?;
    let adaptive = optional_value_array(streaming_data.get("adaptiveFormats"))?;
    let input_count = merged
        .len()
        .checked_add(adaptive.len())
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let hls_manifest_url = if is_live {
        bounded_value_text(streaming_data.get("hlsManifestUrl"), max_url_bytes)?
    } else {
        None
    };
    if let Some(url) = hls_manifest_url.as_deref()
        && RemoteHttpRequest::get(url).is_err()
    {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let candidate_count = input_count
        .checked_add(usize::from(hls_manifest_url.is_some()))
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    if candidate_count == 0 || candidate_count > max_formats {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }

    let mut formats = Vec::with_capacity(candidate_count);
    if let Some(playback_url) = hls_manifest_url {
        let n_parameter = url_query_parameter(&playback_url, "n", max_string_bytes)?;
        formats.push(YoutubePlaybackFormat {
            kind: Some(YoutubePlaybackFormatKind::HlsMpegTsAac),
            itag: -1,
            bitrate: 0,
            content_length: None,
            audio_channels: 2,
            playback_url,
            n_parameter,
            signature: None,
            signature_key: "signature".to_owned(),
            default_audio_track: true,
            drc: false,
        });
    }
    for format in merged.iter().chain(adaptive) {
        if let Some(format) =
            parse_playback_format(format, is_live, max_string_bytes, max_url_bytes)?
        {
            formats.push(format);
        }
    }
    let selected_index = select_playback_format(&formats)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    Ok(YoutubePlaybackFormats {
        client,
        formats,
        selected_index,
    })
}

fn validate_playability_value(json: &Value) -> Result<(), YoutubeError> {
    let status = json
        .pointer("/playabilityStatus/status")
        .and_then(Value::as_str)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    match status {
        "OK" => Ok(()),
        "LOGIN_REQUIRED" | "CONTENT_CHECK_REQUIRED" => {
            Err(YoutubeError::new(YoutubeErrorKind::LoginRequired))
        }
        "ERROR" | "UNPLAYABLE" | "LIVE_STREAM_OFFLINE" => {
            Err(YoutubeError::new(YoutubeErrorKind::Unavailable))
        }
        _ => Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse)),
    }
}

fn optional_value_array(value: Option<&Value>) -> Result<&[Value], YoutubeError> {
    match value {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(values)) => Ok(values),
        Some(_) => Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse)),
    }
}

fn parse_playback_format(
    json: &Value,
    is_live: bool,
    max_string_bytes: usize,
    max_url_bytes: usize,
) -> Result<Option<YoutubePlaybackFormat>, YoutubeError> {
    let Some(json) = json.as_object() else {
        return Ok(None);
    };
    let Some(mime_type) = bounded_value_text(json.get("mimeType"), max_string_bytes)? else {
        return Ok(None);
    };
    let itag = value_i64(json.get("itag"))
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(-1);
    let bitrate = value_u64(json.get("bitrate")).unwrap_or(0);
    let audio_channels = value_u64(json.get("audioChannels"))
        .unwrap_or(2)
        .try_into()
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let content_length = value_u64(json.get("contentLength"));
    if content_length.is_none() && !is_live && itag != 18 {
        return Ok(None);
    }

    let direct_url = bounded_value_text(json.get("url"), max_url_bytes)?;
    let cipher = bounded_value_text(json.get("signatureCipher"), max_url_bytes)?;
    let cipher = cipher
        .as_deref()
        .map(|cipher| parse_signature_cipher(cipher, max_url_bytes, max_string_bytes))
        .transpose()?;
    let playback_url = cipher
        .as_ref()
        .and_then(|cipher| cipher.url.clone())
        .or(direct_url);
    let Some(playback_url) = playback_url else {
        return Ok(None);
    };
    if playback_url.len() > max_url_bytes || RemoteHttpRequest::get(playback_url.clone()).is_err() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let n_parameter = url_query_parameter(&playback_url, "n", max_string_bytes)?;
    let signature = cipher.as_ref().and_then(|cipher| cipher.signature.clone());
    let signature_key = cipher
        .and_then(|cipher| cipher.signature_key)
        .unwrap_or_else(|| "signature".to_owned());
    let default_audio_track = json
        .get("audioTrack")
        .and_then(|audio_track| audio_track.get("audioIsDefault"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let drc = json.get("isDrc").and_then(Value::as_bool).unwrap_or(false);
    Ok(Some(YoutubePlaybackFormat {
        kind: playback_format_kind(&mime_type),
        itag,
        bitrate,
        content_length,
        audio_channels,
        playback_url,
        n_parameter,
        signature,
        signature_key,
        default_audio_track,
        drc,
    }))
}

struct ParsedSignatureCipher {
    url: Option<String>,
    signature: Option<String>,
    signature_key: Option<String>,
}

fn parse_signature_cipher(
    value: &str,
    max_url_bytes: usize,
    max_string_bytes: usize,
) -> Result<ParsedSignatureCipher, YoutubeError> {
    let mut parsed = ParsedSignatureCipher {
        url: None,
        signature: None,
        signature_key: None,
    };
    let mut count = 0_usize;
    for item in value.split('&') {
        count += 1;
        if count > 16 {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let (key, value) = item.split_once('=').unwrap_or((item, ""));
        let key = percent_decode(key, true, max_string_bytes)?;
        match key.as_str() {
            "url" => set_once(&mut parsed.url, percent_decode(value, true, max_url_bytes)?)?,
            "s" => set_once(
                &mut parsed.signature,
                percent_decode(value, true, max_string_bytes)?,
            )?,
            "sp" => set_once(
                &mut parsed.signature_key,
                percent_decode(value, true, max_string_bytes)?,
            )?,
            _ => {}
        }
    }
    Ok(parsed)
}

fn set_once(target: &mut Option<String>, value: String) -> Result<(), YoutubeError> {
    if target.is_some() || value.is_empty() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    *target = Some(value);
    Ok(())
}

fn percent_decode(
    value: &str,
    plus_as_space: bool,
    max_bytes: usize,
) -> Result<String, YoutubeError> {
    if value.len() > max_bytes.saturating_mul(3) {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len().min(max_bytes));
    let mut index = 0_usize;
    while index < bytes.len() {
        let byte = match bytes[index] {
            b'%' => {
                let high = bytes
                    .get(index + 1)
                    .and_then(|byte| hex_value(*byte))
                    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
                let low = bytes
                    .get(index + 2)
                    .and_then(|byte| hex_value(*byte))
                    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
                index += 3;
                (high << 4) | low
            }
            b'+' if plus_as_space => {
                index += 1;
                b' '
            }
            byte => {
                index += 1;
                byte
            }
        };
        if decoded.len() >= max_bytes {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        decoded.push(byte);
    }
    String::from_utf8(decoded).map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn url_query_parameter(
    url: &str,
    name: &str,
    max_string_bytes: usize,
) -> Result<Option<String>, YoutubeError> {
    let Some((_, query)) = url.split_once('?') else {
        return Ok(None);
    };
    let query = query.split('#').next().unwrap_or(query);
    for (index, item) in query.split('&').enumerate() {
        if index >= 256 {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let (key, value) = item.split_once('=').unwrap_or((item, ""));
        if percent_decode(key, true, max_string_bytes)? == name {
            let value = percent_decode(value, true, max_string_bytes)?;
            return (!value.is_empty())
                .then_some(value)
                .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))
                .map(Some);
        }
    }
    Ok(None)
}

fn validate_cipher_solution(
    solution: &YoutubeCipherSolution,
    format: &YoutubePlaybackFormat,
    max_output_bytes: usize,
) -> Result<(), YoutubeError> {
    if solution.signature.is_some() != format.signature.is_some()
        || solution.n_parameter.is_some() != format.n_parameter.is_some()
        || [&solution.signature, &solution.n_parameter]
            .into_iter()
            .flatten()
            .any(|value| {
                value.is_empty()
                    || value.len() > max_output_bytes
                    || !value.bytes().all(|byte| byte.is_ascii())
            })
        || format
            .n_parameter
            .as_deref()
            .is_some_and(|input| solution.n_parameter.as_deref() == Some(input))
    {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(())
}

fn set_url_query_parameter(
    url: &str,
    name: &str,
    value: &str,
    require_existing: bool,
    max_url_bytes: usize,
    max_value_bytes: usize,
) -> Result<String, YoutubeError> {
    if name.is_empty()
        || name.len() > max_value_bytes
        || value.is_empty()
        || value.len() > max_value_bytes
    {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let (without_fragment, fragment) = url
        .split_once('#')
        .map_or((url, None), |(url, fragment)| (url, Some(fragment)));
    let (base, query) = without_fragment
        .split_once('?')
        .map_or((without_fragment, ""), |(base, query)| (base, query));
    let mut retained = Vec::new();
    let mut replaced = 0_usize;
    if !query.is_empty() {
        for (index, item) in query.split('&').enumerate() {
            if index >= 256 || item.is_empty() {
                return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
            }
            let key = item.split_once('=').map_or(item, |(key, _)| key);
            if percent_decode(key, true, max_value_bytes)? == name {
                replaced += 1;
                if replaced > 1 {
                    return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
                }
            } else {
                retained.push(item);
            }
        }
    }
    if require_existing && replaced != 1 {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let encoded_name = percent_encode_query_component(name);
    let encoded_value = percent_encode_query_component(value);
    let mut resolved = String::with_capacity(url.len().saturating_add(encoded_value.len()));
    resolved.push_str(base);
    resolved.push('?');
    for item in retained {
        resolved.push_str(item);
        resolved.push('&');
    }
    resolved.push_str(&encoded_name);
    resolved.push('=');
    resolved.push_str(&encoded_value);
    if let Some(fragment) = fragment {
        resolved.push('#');
        resolved.push_str(fragment);
    }
    if resolved.len() > max_url_bytes || RemoteHttpRequest::get(&resolved).is_err() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(resolved)
}

fn percent_encode_query_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn playback_format_kind(mime_type: &str) -> Option<YoutubePlaybackFormatKind> {
    let mut fields = mime_type.split(';');
    let mime = fields.next()?.trim().to_ascii_lowercase();
    let codecs = fields.find_map(|field| {
        let (name, value) = field.trim().split_once('=')?;
        name.eq_ignore_ascii_case("codecs")
            .then(|| value.trim().trim_matches('"').to_ascii_lowercase())
    })?;
    match mime.as_str() {
        "audio/webm" if codecs.contains("opus") => Some(YoutubePlaybackFormatKind::WebmOpus),
        "audio/webm" if codecs.contains("vorbis") => Some(YoutubePlaybackFormatKind::WebmVorbis),
        "audio/mp4" if codecs.contains("mp4a.40.2") => Some(YoutubePlaybackFormatKind::Mp4AacLc),
        "video/webm" if codecs.contains("vorbis") => {
            Some(YoutubePlaybackFormatKind::WebmVideoVorbis)
        }
        "video/mp4" if codecs.contains("mp4a.40.2") => {
            Some(YoutubePlaybackFormatKind::Mp4VideoAacLc)
        }
        _ => None,
    }
}

fn value_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn value_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
    })
}

fn select_playback_format(formats: &[YoutubePlaybackFormat]) -> Option<usize> {
    let mut best = None;
    for (index, format) in formats.iter().enumerate() {
        let Some(kind) = format.kind else { continue };
        if !format.default_audio_track
            || (kind == YoutubePlaybackFormatKind::WebmOpus && format.audio_channels > 2)
        {
            continue;
        }
        let replace =
            best.is_none_or(|best_index| is_better_playback_format(format, &formats[best_index]));
        if replace {
            best = Some(index);
        }
    }
    best
}

fn is_better_playback_format(
    candidate: &YoutubePlaybackFormat,
    current: &YoutubePlaybackFormat,
) -> bool {
    let candidate_kind = candidate
        .kind
        .expect("selection only compares supported formats");
    let current_kind = current
        .kind
        .expect("selection only compares supported formats");
    candidate_kind.preference() < current_kind.preference()
        || (candidate_kind == current_kind
            && ((!candidate.drc && current.drc)
                || (candidate.drc == current.drc && candidate.bitrate > current.bitrate)))
}

fn parse_search_response(
    bytes: &[u8],
    query: &str,
    max_tracks: usize,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let sections = json
        .pointer(
            "/contents/twoColumnSearchResultsRenderer/primaryContents/sectionListRenderer/contents",
        )
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"))
        .and_then(Value::as_array)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let mut tracks = Vec::new();
    for section in sections {
        let Some(items) = section
            .pointer("/itemSectionRenderer/contents")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for item in items {
            let renderer = item
                .get("videoRenderer")
                .or_else(|| item.get("compactVideoRenderer"));
            let Some(renderer) = renderer else { continue };
            let Some(track) = parse_search_track(renderer, max_string_bytes, max_thumbnails)?
            else {
                continue;
            };
            push_bounded_track(&mut tracks, track, max_tracks)?;
        }
    }
    if tracks.is_empty() {
        return Ok(None);
    }
    let name = bounded_composed_name("Search results for: ", query, max_string_bytes)?;
    Ok(Some(YoutubeSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: true,
    }))
}

fn parse_search_track(
    renderer: &Value,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<Option<YoutubeSourceTrack>, YoutubeError> {
    if renderer
        .get("unplayableText")
        .is_some_and(|value| !value.is_null())
        || renderer.get("lengthText").is_none_or(Value::is_null)
    {
        return Ok(None);
    }
    let video_id = bounded_value_text(renderer.get("videoId"), max_string_bytes)?
        .filter(|video_id| valid_video_id(video_id))
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let title = renderer_text(
        renderer.get("headline").or_else(|| renderer.get("title")),
        max_string_bytes,
    )?
    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let author = renderer_text(renderer.get("longBylineText"), max_string_bytes)?
        .or(renderer_text(
            renderer.get("shortBylineText"),
            max_string_bytes,
        )?)
        .unwrap_or_else(|| "Unknown artist".to_owned());
    let duration_text = renderer_text(renderer.get("lengthText"), max_string_bytes)?
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let duration = parse_duration_text(&duration_text)?;
    let artwork_url = renderer_artwork(renderer, max_string_bytes, max_thumbnails)?;
    Ok(Some(make_result_track(
        &video_id,
        title,
        author,
        duration,
        false,
        artwork_url,
    )))
}

fn parse_mix_response(
    bytes: &[u8],
    selected_video_id: &str,
    max_tracks: usize,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<YoutubeSourcePlaylist, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let playlist = json
        .pointer("/contents/twoColumnWatchNextResults/playlist/playlist")
        .or_else(|| json.pointer("/contents/singleColumnWatchNextResults/playlist/playlist"))
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let name = bounded_value_text(playlist.get("title"), max_string_bytes)?
        .unwrap_or_else(|| "YouTube mix".to_owned());
    let items = playlist
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let mut tracks = Vec::new();
    for item in items {
        let Some(renderer) = item.get("playlistPanelVideoRenderer") else {
            continue;
        };
        let Some(track) = parse_search_track(renderer, max_string_bytes, max_thumbnails)? else {
            continue;
        };
        push_bounded_track(&mut tracks, track, max_tracks)?;
    }
    if tracks.is_empty() {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let selected_track = tracks
        .iter()
        .position(|track| track.info.identifier == selected_video_id);
    Ok(YoutubeSourcePlaylist {
        name,
        tracks,
        selected_track,
        is_search_result: false,
    })
}

fn parse_music_search_response(
    bytes: &[u8],
    query: &str,
    max_tracks: usize,
    max_string_bytes: usize,
) -> Result<Option<YoutubeSourcePlaylist>, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let sections = json
        .pointer(
            "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
        )
        .and_then(Value::as_array)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let mut tracks = Vec::new();
    for section in sections {
        let Some(items) = section
            .pointer("/musicShelfRenderer/contents")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for item in items {
            let Some(renderer) = item.get("musicResponsiveListItemRenderer") else {
                continue;
            };
            let Some(track) = parse_music_search_track(renderer, max_string_bytes)? else {
                continue;
            };
            push_bounded_track(&mut tracks, track, max_tracks)?;
        }
    }
    if tracks.is_empty() {
        return Ok(None);
    }
    let name = bounded_composed_name("Search music results for: ", query, max_string_bytes)?;
    Ok(Some(YoutubeSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: true,
    }))
}

fn parse_music_search_track(
    renderer: &Value,
    max_string_bytes: usize,
) -> Result<Option<YoutubeSourceTrack>, YoutubeError> {
    let Some(columns) = renderer.get("flexColumns").and_then(Value::as_array) else {
        return Ok(None);
    };
    let Some(metadata) = columns.first().and_then(|column| {
        column.pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs/0")
    }) else {
        return Ok(None);
    };
    let Some(video_id) = bounded_value_text(
        metadata.pointer("/navigationEndpoint/watchEndpoint/videoId"),
        max_string_bytes,
    )?
    else {
        return Ok(None);
    };
    if !valid_video_id(&video_id) {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    let title = bounded_value_text(metadata.get("text"), max_string_bytes)?
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let Some(runs) = columns
        .get(1)
        .and_then(|column| column.pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    let Some(last) = runs.last() else {
        return Ok(None);
    };
    if last
        .get("navigationEndpoint")
        .is_some_and(|value| !value.is_null())
    {
        return Ok(None);
    }
    let author = runs
        .first()
        .map(|run| bounded_value_text(run.get("text"), max_string_bytes))
        .transpose()?
        .flatten()
        .unwrap_or_else(|| "Unknown artist".to_owned());
    let duration_text = bounded_value_text(last.get("text"), max_string_bytes)?
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let duration = parse_duration_text(&duration_text)?;
    Ok(Some(make_result_track(
        &video_id, title, author, duration, false, None,
    )))
}

struct ParsedPlaylistPage {
    name: Option<String>,
    tracks: Vec<YoutubeSourceTrack>,
    continuation: Option<String>,
}

fn parse_playlist_first_page(
    bytes: &[u8],
    max_tracks: usize,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<ParsedPlaylistPage, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    if json
        .get("alerts")
        .and_then(Value::as_array)
        .is_some_and(|alerts| {
            alerts.iter().any(|alert| {
                alert.pointer("/alertRenderer/type").and_then(Value::as_str) == Some("ERROR")
            })
        })
    {
        return Err(YoutubeError::new(YoutubeErrorKind::Unavailable));
    }
    let name = bounded_value_text(
        json.pointer("/metadata/playlistMetadataRenderer/title"),
        max_string_bytes,
    )?
    .or(renderer_text(
        json.pointer("/header/playlistHeaderRenderer/title"),
        max_string_bytes,
    )?)
    .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let list = json
        .pointer(
            "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/itemSectionRenderer/contents/0/playlistVideoListRenderer",
        )
        .or_else(|| {
            json.pointer(
                "/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/playlistVideoListRenderer",
            )
        })
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let items = list
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    Ok(ParsedPlaylistPage {
        name: Some(name),
        tracks: parse_playlist_tracks(items, max_tracks, max_string_bytes, max_thumbnails)?,
        continuation: playlist_continuation(list, max_string_bytes)?,
    })
}

fn parse_playlist_continuation_page(
    bytes: &[u8],
    max_tracks: usize,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<ParsedPlaylistPage, YoutubeError> {
    let json: Value = serde_json::from_slice(bytes)
        .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let list = json
        .pointer("/onResponseReceivedActions/0/appendContinuationItemsAction/continuationItems")
        .or_else(|| json.pointer("/continuationContents/playlistVideoListContinuation"))
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    let items = list
        .as_array()
        .or_else(|| list.get("contents").and_then(Value::as_array))
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    Ok(ParsedPlaylistPage {
        name: None,
        tracks: parse_playlist_tracks(items, max_tracks, max_string_bytes, max_thumbnails)?,
        continuation: playlist_continuation(list, max_string_bytes)?,
    })
}

fn parse_playlist_tracks(
    items: &[Value],
    max_tracks: usize,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<Vec<YoutubeSourceTrack>, YoutubeError> {
    let mut tracks = Vec::new();
    for item in items {
        let Some(renderer) = item.get("playlistVideoRenderer") else {
            continue;
        };
        if renderer.get("isPlayable").is_none_or(Value::is_null)
            || renderer.get("shortBylineText").is_none_or(Value::is_null)
        {
            continue;
        }
        let video_id = bounded_value_text(renderer.get("videoId"), max_string_bytes)?
            .filter(|video_id| valid_video_id(video_id))
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let title = renderer_text(renderer.get("title"), max_string_bytes)?
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        let author = renderer_text(renderer.get("shortBylineText"), max_string_bytes)?
            .unwrap_or_else(|| "Unknown artist".to_owned());
        let duration = renderer
            .get("lengthSeconds")
            .and_then(Value::as_str)
            .map(str::parse::<u64>)
            .transpose()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?
            .map_or(Duration::ZERO, Duration::from_secs);
        let artwork_url = renderer_artwork(renderer, max_string_bytes, max_thumbnails)?;
        push_bounded_track(
            &mut tracks,
            make_result_track(&video_id, title, author, duration, false, artwork_url),
            max_tracks,
        )?;
    }
    Ok(tracks)
}

fn playlist_continuation(
    list: &Value,
    max_string_bytes: usize,
) -> Result<Option<String>, YoutubeError> {
    if let Some(token) = bounded_value_text(
        list.pointer("/continuations/0/nextContinuationData/continuation"),
        max_string_bytes,
    )? {
        return Ok(Some(token));
    }
    let Some(items) = list
        .as_array()
        .or_else(|| list.get("contents").and_then(Value::as_array))
    else {
        return Ok(None);
    };
    for item in items {
        let Some(endpoint) = item.pointer("/continuationItemRenderer/continuationEndpoint") else {
            continue;
        };
        if let Some(token) = bounded_value_text(
            endpoint.pointer("/continuationCommand/token"),
            max_string_bytes,
        )? {
            return Ok(Some(token));
        }
        if let Some(commands) = endpoint
            .pointer("/commandExecutorCommand/commands")
            .and_then(Value::as_array)
        {
            for command in commands {
                if let Some(token) = bounded_value_text(
                    command.pointer("/continuationCommand/token"),
                    max_string_bytes,
                )? {
                    return Ok(Some(token));
                }
            }
        }
    }
    Ok(None)
}

fn renderer_text(
    value: Option<&Value>,
    max_string_bytes: usize,
) -> Result<Option<String>, YoutubeError> {
    let Some(value) = value else { return Ok(None) };
    bounded_value_text(value.get("simpleText"), max_string_bytes)?.map_or_else(
        || bounded_value_text(value.pointer("/runs/0/text"), max_string_bytes),
        |text| Ok(Some(text)),
    )
}

fn bounded_value_text(
    value: Option<&Value>,
    max_string_bytes: usize,
) -> Result<Option<String>, YoutubeError> {
    let Some(value) = value else { return Ok(None) };
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    };
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(bounded_text(value, max_string_bytes)?))
}

fn bounded_text(value: &str, max_string_bytes: usize) -> Result<String, YoutubeError> {
    if value.is_empty() || value.len() > max_string_bytes {
        Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse))
    } else {
        Ok(value.to_owned())
    }
}

fn bounded_composed_name(
    prefix: &str,
    value: &str,
    max_string_bytes: usize,
) -> Result<String, YoutubeError> {
    let length = prefix
        .len()
        .checked_add(value.len())
        .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
    if value.is_empty() || length > max_string_bytes {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(format!("{prefix}{value}"))
}

fn parse_duration_text(value: &str) -> Result<Duration, YoutubeError> {
    let mut seconds = 0_u64;
    let mut fields = 0_usize;
    for field in value.split(':') {
        if field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        let value = field
            .parse::<u64>()
            .map_err(|_| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        if fields > 0 && value > 59 {
            return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
        }
        seconds = seconds
            .checked_mul(60)
            .and_then(|seconds| seconds.checked_add(value))
            .ok_or_else(|| YoutubeError::new(YoutubeErrorKind::InvalidResponse))?;
        fields += 1;
    }
    if !(2..=4).contains(&fields) {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(Duration::from_secs(seconds))
}

fn renderer_artwork(
    renderer: &Value,
    max_string_bytes: usize,
    max_thumbnails: usize,
) -> Result<Option<String>, YoutubeError> {
    let Some(thumbnails) = renderer
        .pointer("/thumbnail/thumbnails")
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    if thumbnails.len() > max_thumbnails {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    Ok(thumbnails.iter().rev().find_map(|thumbnail| {
        thumbnail
            .get("url")
            .and_then(Value::as_str)
            .and_then(|url| {
                (url.len() <= max_string_bytes
                    && (url.starts_with("https://") || url.starts_with("http://")))
                .then(|| url.to_owned())
            })
    }))
}

fn make_result_track(
    video_id: &str,
    title: String,
    author: String,
    duration: Duration,
    is_stream: bool,
    artwork_url: Option<String>,
) -> YoutubeSourceTrack {
    YoutubeSourceTrack {
        info: TrackInfo {
            title,
            author,
            duration,
            identifier: video_id.to_owned(),
            is_stream,
            uri: Some(format!("{WATCH_URL_PREFIX}{video_id}")),
            artwork_url,
            isrc: None,
        },
    }
}

fn push_bounded_track(
    tracks: &mut Vec<YoutubeSourceTrack>,
    track: YoutubeSourceTrack,
    max_tracks: usize,
) -> Result<(), YoutubeError> {
    if tracks.len() >= max_tracks {
        return Err(YoutubeError::new(YoutubeErrorKind::InvalidResponse));
    }
    tracks.push(track);
    Ok(())
}

#[cfg(test)]
mod live_player_script_tests {
    use super::{YoutubeSourceOptions, parse_youtube_cipher_program};

    #[test]
    #[ignore = "requires MANTLE_YOUTUBE_PLAYER_SCRIPT_PATH from the bounded live validator"]
    fn current_player_script_matches_native_cipher_grammar() {
        let path = std::env::var_os("MANTLE_YOUTUBE_PLAYER_SCRIPT_PATH")
            .expect("MANTLE_YOUTUBE_PLAYER_SCRIPT_PATH must name the fetched base.js");
        let source = std::fs::read(path).expect("player script must remain readable");
        let options = YoutubeSourceOptions::default();
        assert!(
            u64::try_from(source.len()).unwrap_or(u64::MAX) <= options.max_player_script_bytes,
            "player script exceeds the configured acquisition bound"
        );
        parse_youtube_cipher_program(&source, options.max_cipher_operations)
            .expect("current player script must match the bounded native cipher grammar");
    }
}
