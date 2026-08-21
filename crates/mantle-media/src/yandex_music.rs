use std::fmt;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mantle_audio::PcmFrame;
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use md5::{Digest, Md5};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::Deserialize;
use ureq::http::Uri;

use crate::{
    Codec, Container, HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, MediaCancellation,
    MediaError, MediaInfo, MediaLimits, MediaSession, RemoteHttpClient, RemoteHttpErrorKind,
    RemoteHttpOptions, RemoteHttpRequest, SeekResult,
};

const DEFAULT_API_BASE_URL: &str = "https://api.music.yandex.net";
const MAX_API_BASE_URL_BYTES: usize = 4 * 1024;
const MAX_ACCESS_TOKEN_BYTES: usize = 16 * 1024;
const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_METADATA_STRING_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CONFIGURED_ARTISTS: usize = 256;
const MAX_CONFIGURED_COLLECTION_TRACKS: usize = 10_000;
const MAX_CONFIGURED_COLLECTION_PAGES: usize = 64;
const MAX_CONFIGURED_DOWNLOAD_CANDIDATES: usize = 256;
const MAX_CONFIGURED_PLAYBACK_URL_BYTES: usize = 1024 * 1024;
const MAX_CONFIGURED_DOWNLOAD_INFO_BYTES: u64 = 1024 * 1024;
const MAX_NUMERIC_IDENTIFIER_BYTES: usize = 32;
const MAX_PLAYLIST_IDENTIFIER_BYTES: usize = 256;
const MAX_DOWNLOAD_INFO_EVENTS: usize = 256;
const MAX_DOWNLOAD_INFO_DEPTH: usize = 16;
const MAX_BITRATE_KBPS: u32 = 10_000;
const DOWNLOAD_SIGN_SALT: &str = "XGRlBW9FXlekgbPrRHuSiA";
const ALBUM_TRACKS_PER_PAGE: usize = 50;
const ARTIST_TRACKS_PER_PAGE: usize = 10;
const PLAYLIST_TRACKS_PER_PAGE: usize = 100;

/// Current Yandex Music identifier shapes supported by the pinned Phase 12 protocol evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YandexMusicRoute {
    Track {
        track_id: String,
        album_id: Option<String>,
        domain: String,
    },
    Album {
        album_id: String,
        domain: String,
    },
    Artist {
        artist_id: String,
        domain: String,
    },
    Playlist {
        owner: Option<String>,
        playlist_id: String,
        domain: String,
    },
    Search(String),
    Recommendations(String),
}

/// Scheme policy for the signed media URL produced from Yandex download information.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum YandexMusicPlaybackScheme {
    #[default]
    Https,
    /// Permit HTTP only alongside the explicit private-network source policy.
    ///
    /// This exists for trusted loopback replay and must not be used for public service traffic.
    HttpForPrivateNetworks,
}

/// Validated caller-provided OAuth token. Its diagnostics never expose the token value.
#[derive(Clone, Eq, PartialEq)]
pub struct YandexMusicAuthentication {
    access_token: String,
}

impl YandexMusicAuthentication {
    /// Creates an authentication value after validating its header-safe resource bound.
    ///
    /// # Errors
    ///
    /// Returns [`YandexMusicErrorKind::InvalidAuthentication`] for an empty, oversized, or
    /// non-ASCII-graphic token.
    pub fn new(access_token: impl Into<String>) -> Result<Self, YandexMusicError> {
        let access_token = access_token.into();
        if access_token.is_empty()
            || access_token.len() > MAX_ACCESS_TOKEN_BYTES
            || !access_token.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(YandexMusicError::new(
                YandexMusicErrorKind::InvalidAuthentication,
            ));
        }
        Ok(Self { access_token })
    }
}

impl fmt::Debug for YandexMusicAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicAuthentication")
            .field("configured", &true)
            .finish_non_exhaustive()
    }
}

/// Bounded routing, HTTP, and metadata policy for the current Yandex Music adapter.
#[derive(Clone, Eq, PartialEq)]
pub struct YandexMusicSourceOptions {
    pub http: RemoteHttpOptions,
    pub api_base_url: String,
    pub allow_search: bool,
    pub allow_recommendations: bool,
    pub max_identifier_bytes: usize,
    pub max_metadata_string_bytes: usize,
    pub max_artists: usize,
    pub max_collection_tracks: usize,
    pub max_collection_pages: usize,
    pub max_download_candidates: usize,
    pub max_playback_url_bytes: usize,
    pub max_download_info_bytes: u64,
    pub playback_scheme: YandexMusicPlaybackScheme,
    pub max_response_bytes: u64,
}

impl Default for YandexMusicSourceOptions {
    fn default() -> Self {
        Self {
            http: RemoteHttpOptions::default(),
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
            allow_search: true,
            allow_recommendations: true,
            max_identifier_bytes: 8 * 1024,
            max_metadata_string_bytes: 64 * 1024,
            max_artists: 64,
            max_collection_tracks: 600,
            max_collection_pages: 6,
            max_download_candidates: 64,
            max_playback_url_bytes: 64 * 1024,
            max_download_info_bytes: 64 * 1024,
            playback_scheme: YandexMusicPlaybackScheme::Https,
            max_response_bytes: 1024 * 1024,
        }
    }
}

impl YandexMusicSourceOptions {
    fn validate(&self) -> Result<(), YandexMusicError> {
        if self.api_base_url.is_empty()
            || self.api_base_url.len() > MAX_API_BASE_URL_BYTES
            || self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_metadata_string_bytes == 0
            || self.max_metadata_string_bytes > MAX_CONFIGURED_METADATA_STRING_BYTES
            || self.max_artists == 0
            || self.max_artists > MAX_CONFIGURED_ARTISTS
            || self.max_collection_tracks == 0
            || self.max_collection_tracks > MAX_CONFIGURED_COLLECTION_TRACKS
            || self.max_collection_pages == 0
            || self.max_collection_pages > MAX_CONFIGURED_COLLECTION_PAGES
            || self.max_download_candidates == 0
            || self.max_download_candidates > MAX_CONFIGURED_DOWNLOAD_CANDIDATES
            || self.max_playback_url_bytes == 0
            || self.max_playback_url_bytes > MAX_CONFIGURED_PLAYBACK_URL_BYTES
            || self.max_download_info_bytes == 0
            || self.max_download_info_bytes > MAX_CONFIGURED_DOWNLOAD_INFO_BYTES
            || self.max_download_info_bytes > self.http.max_response_bytes
            || (self.playback_scheme == YandexMusicPlaybackScheme::HttpForPrivateNetworks
                && self.http.network_access != HttpNetworkAccess::AllowPrivateNetworks)
            || self.max_response_bytes == 0
            || self.max_response_bytes > MAX_CONFIGURED_RESPONSE_BYTES
            || self.max_response_bytes > self.http.max_response_bytes
        {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let endpoint = format!("{}/tracks/1", self.api_base_url.trim_end_matches('/'));
        RemoteHttpRequest::get(endpoint)
            .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
        Ok(())
    }
}

impl fmt::Debug for YandexMusicSourceOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicSourceOptions")
            .field("http", &self.http)
            .field("allow_search", &self.allow_search)
            .field("allow_recommendations", &self.allow_recommendations)
            .field("max_identifier_bytes", &self.max_identifier_bytes)
            .field("max_metadata_string_bytes", &self.max_metadata_string_bytes)
            .field("max_artists", &self.max_artists)
            .field("max_collection_tracks", &self.max_collection_tracks)
            .field("max_collection_pages", &self.max_collection_pages)
            .field("max_download_candidates", &self.max_download_candidates)
            .field("max_playback_url_bytes", &self.max_playback_url_bytes)
            .field("max_download_info_bytes", &self.max_download_info_bytes)
            .field("playback_scheme", &self.playback_scheme)
            .field("max_response_bytes", &self.max_response_bytes)
            .finish_non_exhaustive()
    }
}

/// Routes one input without network access, using strict current Yandex Music URL shapes.
#[must_use]
pub fn route_yandex_music_identifier(
    identifier: &str,
    options: &YandexMusicSourceOptions,
) -> Option<YandexMusicRoute> {
    if identifier.is_empty() || identifier.len() > options.max_identifier_bytes {
        return None;
    }
    if let Some(query) = identifier.strip_prefix("ymsearch:") {
        let query = query.trim();
        return (options.allow_search
            && !query.is_empty()
            && query.len() <= options.max_metadata_string_bytes)
            .then(|| YandexMusicRoute::Search(query.to_owned()));
    }
    if let Some(track_id) = identifier.strip_prefix("ymrec:") {
        return (options.allow_recommendations && valid_numeric_identifier(track_id))
            .then(|| YandexMusicRoute::Recommendations(track_id.to_owned()));
    }
    route_yandex_music_url(identifier)
}

fn route_yandex_music_url(identifier: &str) -> Option<YandexMusicRoute> {
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
    if host.contains('@') {
        return None;
    }
    let domain = host.strip_prefix("music.yandex.")?;
    if !matches!(domain, "ru" | "com" | "kz" | "by") {
        return None;
    }
    let path = path.strip_suffix('/').unwrap_or(path);
    let parts: Vec<_> = path.split('/').collect();
    match parts.as_slice() {
        ["track", track_id] if valid_numeric_identifier(track_id) => {
            Some(YandexMusicRoute::Track {
                track_id: (*track_id).to_owned(),
                album_id: None,
                domain: domain.to_owned(),
            })
        }
        ["album", album_id, "track", track_id]
            if valid_numeric_identifier(album_id) && valid_numeric_identifier(track_id) =>
        {
            Some(YandexMusicRoute::Track {
                track_id: (*track_id).to_owned(),
                album_id: Some((*album_id).to_owned()),
                domain: domain.to_owned(),
            })
        }
        ["album", album_id] if valid_numeric_identifier(album_id) => {
            Some(YandexMusicRoute::Album {
                album_id: (*album_id).to_owned(),
                domain: domain.to_owned(),
            })
        }
        ["artist", artist_id] | ["artist", artist_id, "tracks"]
            if valid_numeric_identifier(artist_id) =>
        {
            Some(YandexMusicRoute::Artist {
                artist_id: (*artist_id).to_owned(),
                domain: domain.to_owned(),
            })
        }
        ["users", owner, "playlists", playlist_id]
            if valid_playlist_component(owner) && valid_numeric_identifier(playlist_id) =>
        {
            Some(YandexMusicRoute::Playlist {
                owner: Some((*owner).to_owned()),
                playlist_id: (*playlist_id).to_owned(),
                domain: domain.to_owned(),
            })
        }
        ["playlists", playlist_id] if valid_playlist_component(playlist_id) => {
            Some(YandexMusicRoute::Playlist {
                owner: None,
                playlist_id: (*playlist_id).to_owned(),
                domain: domain.to_owned(),
            })
        }
        _ => None,
    }
}

fn valid_numeric_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_NUMERIC_IDENTIFIER_BYTES
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_playlist_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PLAYLIST_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'@' | b'.' | b'-'))
}

/// Native Yandex Music track metadata used by source-neutral and JVM adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YandexMusicSourceTrack {
    pub info: TrackInfo,
}

/// Current collection classification retained for later JVM/plugin metadata adaptation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YandexMusicPlaylistKind {
    Album,
    Artist,
    Playlist,
    Search,
    Recommendations,
}

/// Bounded native playlist/search result returned by Yandex Music collection routes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YandexMusicSourcePlaylist {
    pub name: String,
    pub tracks: Vec<YandexMusicSourceTrack>,
    pub selected_track: Option<usize>,
    pub is_search_result: bool,
    pub kind: YandexMusicPlaylistKind,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub author: Option<String>,
}

/// Result model for the current Yandex Music source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum YandexMusicSourceItem {
    Track(YandexMusicSourceTrack),
    Playlist(YandexMusicSourcePlaylist),
}

/// One signed MP3 media URL. URL and signing inputs are always redacted from diagnostics.
#[derive(Clone, Eq, PartialEq)]
pub struct YandexMusicPlaybackUrl {
    url: String,
    bitrate_kbps: u32,
}

impl YandexMusicPlaybackUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn bitrate_kbps(&self) -> u32 {
        self.bitrate_kbps
    }
}

impl fmt::Debug for YandexMusicPlaybackUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicPlaybackUrl")
            .field("url", &"<redacted>")
            .field("bitrate_kbps", &self.bitrate_kbps)
            .finish()
    }
}

/// A finite Yandex MP3 connected to Mantle's bounded seekable media pipeline.
pub struct YandexMusicPlaybackSession {
    session: MediaSession,
}

impl YandexMusicPlaybackSession {
    #[must_use]
    pub fn info(&self) -> &MediaInfo {
        self.session.info()
    }

    /// Decodes one PCM frame into caller-owned bounded storage.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe cancellation, network, media, or compatibility error.
    pub fn read_pcm(&mut self, output: &mut PcmFrame) -> Result<bool, YandexMusicPlaybackError> {
        self.session
            .read_pcm(output)
            .map_err(map_playback_media_error)
    }

    /// Seeks the bounded MP3 input to the requested time.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe cancellation, network, or media error.
    pub fn seek(&mut self, requested: Duration) -> Result<SeekResult, YandexMusicPlaybackError> {
        self.session
            .seek(requested)
            .map_err(map_playback_media_error)
    }
}

impl fmt::Debug for YandexMusicPlaybackSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicPlaybackSession")
            .field("media", self.info())
            .finish()
    }
}

/// Stable, credential-safe failure classes for Yandex finite-media handoff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YandexMusicPlaybackErrorKind {
    Source(YandexMusicErrorKind),
    InvalidOptions,
    Cancelled,
    Network,
    InvalidMedia,
    IncompatibleFormat,
}

/// A playback failure that never retains a signed URL, OAuth token, or service body.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct YandexMusicPlaybackError {
    kind: YandexMusicPlaybackErrorKind,
}

impl YandexMusicPlaybackError {
    const fn new(kind: YandexMusicPlaybackErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> YandexMusicPlaybackErrorKind {
        self.kind
    }
}

impl fmt::Debug for YandexMusicPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicPlaybackError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for YandexMusicPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            YandexMusicPlaybackErrorKind::Source(_) => "Yandex Music playback discovery failed",
            YandexMusicPlaybackErrorKind::InvalidOptions => "invalid Yandex Music media policy",
            YandexMusicPlaybackErrorKind::Cancelled => "Yandex Music playback cancelled",
            YandexMusicPlaybackErrorKind::Network => "Yandex Music media request failed",
            YandexMusicPlaybackErrorKind::InvalidMedia => "Yandex Music returned invalid media",
            YandexMusicPlaybackErrorKind::IncompatibleFormat => {
                "Yandex Music media does not match the selected MP3 format"
            }
        })
    }
}

impl std::error::Error for YandexMusicPlaybackError {}

/// Stable Yandex Music failure classes whose diagnostics contain no identifiers or credentials.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YandexMusicErrorKind {
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

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct YandexMusicError {
    kind: YandexMusicErrorKind,
}

impl YandexMusicError {
    const fn new(kind: YandexMusicErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(&self) -> YandexMusicErrorKind {
        self.kind
    }
}

impl fmt::Debug for YandexMusicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for YandexMusicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            YandexMusicErrorKind::InvalidOptions => "invalid Yandex Music source policy",
            YandexMusicErrorKind::InvalidAuthentication => {
                "invalid Yandex Music authentication policy"
            }
            YandexMusicErrorKind::Cancelled => "Yandex Music load cancelled",
            YandexMusicErrorKind::Shutdown => "Yandex Music source is shut down",
            YandexMusicErrorKind::Network => "Yandex Music request failed",
            YandexMusicErrorKind::RateLimited => "Yandex Music rate limit reached",
            YandexMusicErrorKind::AuthenticationRequired => "Yandex Music rejected authentication",
            YandexMusicErrorKind::Unavailable => "Yandex Music content is unavailable",
            YandexMusicErrorKind::InvalidResponse => "Yandex Music returned an invalid response",
            YandexMusicErrorKind::UnsupportedRoute => "Yandex Music route is not implemented",
        })
    }
}

impl std::error::Error for YandexMusicError {}

/// First-class bounded source manager for the current Yandex Music API.
pub struct YandexMusicSourceManager {
    options: YandexMusicSourceOptions,
    authentication: YandexMusicAuthentication,
    http: RemoteHttpClient,
    shutdown: AtomicBool,
}

impl YandexMusicSourceManager {
    /// Creates a manager after validating the HTTP, parsing, and authentication policy.
    ///
    /// # Errors
    ///
    /// Returns a stable policy error without exposing the API endpoint or token.
    pub fn new(
        options: YandexMusicSourceOptions,
        authentication: YandexMusicAuthentication,
    ) -> Result<Self, YandexMusicError> {
        options.validate()?;
        let http = RemoteHttpClient::new(options.http)
            .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
        Ok(Self {
            options,
            authentication,
            http,
            shutdown: AtomicBool::new(false),
        })
    }

    #[must_use]
    pub const fn authentication(&self) -> &YandexMusicAuthentication {
        &self.authentication
    }

    /// Loads one current track metadata response through the bounded shared remote client.
    ///
    /// # Errors
    ///
    /// Returns stable cancellation, authentication, rate-limit, network, or response failures.
    pub fn load_track_metadata(
        &self,
        track_id: &str,
        domain: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourceTrack>, YandexMusicError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Cancelled));
        }
        if !valid_numeric_identifier(track_id) || !matches!(domain, "ru" | "com" | "kz" | "by") {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let endpoint = format!(
            "{}/tracks/{track_id}",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        parse_track_response(&body, track_id, domain, &self.options)
    }

    /// Loads one validated route into a bounded native track or playlist result.
    ///
    /// # Errors
    ///
    /// Returns stable policy, cancellation, authentication, network, or response failures.
    pub fn load_route(
        &self,
        route: &YandexMusicRoute,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourceItem>, YandexMusicError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Cancelled));
        }
        match route {
            YandexMusicRoute::Track {
                track_id, domain, ..
            } => self
                .load_track_metadata(track_id, domain, cancellation)
                .map(|track| track.map(YandexMusicSourceItem::Track)),
            YandexMusicRoute::Album { album_id, domain } => self
                .load_album(album_id, domain, cancellation)
                .map(|playlist| playlist.map(YandexMusicSourceItem::Playlist)),
            YandexMusicRoute::Artist { artist_id, domain } => self
                .load_artist(artist_id, domain, cancellation)
                .map(|playlist| playlist.map(YandexMusicSourceItem::Playlist)),
            YandexMusicRoute::Playlist {
                owner,
                playlist_id,
                domain,
            } => self
                .load_playlist(owner.as_deref(), playlist_id, domain, cancellation)
                .map(|playlist| playlist.map(YandexMusicSourceItem::Playlist)),
            YandexMusicRoute::Search(query) => self
                .load_search(query, cancellation)
                .map(|playlist| playlist.map(YandexMusicSourceItem::Playlist)),
            YandexMusicRoute::Recommendations(track_id) => self
                .load_recommendations(track_id, cancellation)
                .map(|playlist| playlist.map(YandexMusicSourceItem::Playlist)),
        }
    }

    /// Resolves the current highest-bitrate MP3 candidate into a signed, redacted media URL.
    ///
    /// # Errors
    ///
    /// Returns stable policy, cancellation, authentication, network, or bounded-response errors.
    pub fn resolve_track_playback(
        &self,
        track_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicPlaybackUrl>, YandexMusicError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(YandexMusicError::new(YandexMusicErrorKind::Cancelled));
        }
        if !valid_numeric_identifier(track_id) {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let endpoint = format!(
            "{}/tracks/{track_id}/download-info",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        let Some(candidate) = parse_download_candidates(&body, &self.options)? else {
            return Ok(None);
        };
        validate_download_info_origin(&candidate.download_info_url, &self.options)?;
        let xml =
            self.authenticated_download_info_get(candidate.download_info_url, cancellation)?;
        let download = parse_download_info_xml(&xml, &self.options)?;
        build_signed_playback_url(&download, candidate.bitrate_in_kbps, &self.options).map(Some)
    }

    /// Resolves and opens one finite Yandex MP3 through bounded HTTP range input and media probing.
    ///
    /// # Errors
    ///
    /// Returns a credential-safe source, policy, cancellation, network, media, or format error.
    pub fn open_track_playback(
        &self,
        track_id: &str,
        range_options: HttpRangeOptions,
        media_limits: MediaLimits,
        cancellation: MediaCancellation,
    ) -> Result<Option<YandexMusicPlaybackSession>, YandexMusicPlaybackError> {
        if self.options.playback_scheme == YandexMusicPlaybackScheme::HttpForPrivateNetworks
            && range_options.network_access != HttpNetworkAccess::AllowPrivateNetworks
        {
            return Err(YandexMusicPlaybackError::new(
                YandexMusicPlaybackErrorKind::InvalidOptions,
            ));
        }
        let Some(resolved) = self
            .resolve_track_playback(track_id, &cancellation)
            .map_err(map_playback_source_error)?
        else {
            return Ok(None);
        };
        let input = HttpRangeInput::open_with_cancellation(
            resolved.as_str(),
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
            return Err(YandexMusicPlaybackError::new(
                YandexMusicPlaybackErrorKind::IncompatibleFormat,
            ));
        }
        Ok(Some(YandexMusicPlaybackSession { session }))
    }

    fn authenticated_get(
        &self,
        endpoint: String,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, YandexMusicError> {
        let authorization = format!("OAuth {}", self.authentication.access_token);
        let request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", "application/json"))
            .and_then(|request| request.header("Authorization", &authorization))
            .and_then(|request| request.header("User-Agent", "Yandex-Music-API"))
            .and_then(|request| {
                request.header("X-Yandex-Music-Client", "YandexMusicAndroid/24023621")
            })
            .and_then(|request| request.max_response_bytes(self.options.max_response_bytes))
            .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?;
        Ok(response.body().to_vec())
    }

    fn authenticated_download_info_get(
        &self,
        endpoint: String,
        cancellation: &MediaCancellation,
    ) -> Result<Vec<u8>, YandexMusicError> {
        let authorization = format!("OAuth {}", self.authentication.access_token);
        let request = RemoteHttpRequest::get(endpoint)
            .and_then(|request| request.header("Accept", "application/json"))
            .and_then(|request| request.header("Authorization", &authorization))
            .and_then(|request| request.max_response_bytes(self.options.max_download_info_bytes))
            .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
        let response = self
            .http
            .execute_with_cancellation(&request, cancellation)
            .map_err(map_remote_error)?;
        Ok(response.body().to_vec())
    }

    fn load_album(
        &self,
        album_id: &str,
        domain: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
        validate_numeric_route(album_id, domain)?;
        let page_size = self.collection_page_size(ALBUM_TRACKS_PER_PAGE)?;
        let endpoint = format!(
            "{}/albums/{album_id}/with-tracks?page-size={page_size}",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        parse_album_response(&body, album_id, domain, &self.options)
    }

    fn load_artist(
        &self,
        artist_id: &str,
        domain: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
        validate_numeric_route(artist_id, domain)?;
        let page_size = self.collection_page_size(ARTIST_TRACKS_PER_PAGE)?;
        let tracks_endpoint = format!(
            "{}/artists/{artist_id}/tracks?page-size={page_size}",
            self.options.api_base_url.trim_end_matches('/')
        );
        let tracks_body = self.authenticated_get(tracks_endpoint, cancellation)?;
        let tracks = parse_artist_tracks_response(&tracks_body, domain, &self.options)?;
        if tracks.is_empty() {
            return Ok(None);
        }
        let artist_endpoint = format!(
            "{}/artists/{artist_id}",
            self.options.api_base_url.trim_end_matches('/')
        );
        let artist_body = self.authenticated_get(artist_endpoint, cancellation)?;
        parse_artist_response(&artist_body, artist_id, domain, tracks, &self.options).map(Some)
    }

    fn load_playlist(
        &self,
        owner: Option<&str>,
        playlist_id: &str,
        domain: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
        if !valid_playlist_component(playlist_id)
            || owner.is_some_and(|value| !valid_playlist_component(value))
            || !valid_domain(domain)
        {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let page_size = self.collection_page_size(PLAYLIST_TRACKS_PER_PAGE)?;
        let path = owner.map_or_else(
            || format!("playlist/{playlist_id}"),
            |owner| format!("users/{owner}/playlists/{playlist_id}"),
        );
        let endpoint = format!(
            "{}/{path}?page-size={page_size}&rich-tracks=true",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        parse_playlist_response(&body, owner, playlist_id, domain, &self.options)
    }

    fn load_search(
        &self,
        query: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
        if !self.options.allow_search
            || !bounded_nonempty(query, self.options.max_metadata_string_bytes)
        {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let query_string = form_urlencoded::Serializer::new(String::new())
            .append_pair("text", query)
            .append_pair("type", "track")
            .append_pair("page", "0")
            .finish();
        let endpoint = format!(
            "{}/search?{query_string}",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        parse_search_response(&body, query, &self.options)
    }

    fn load_recommendations(
        &self,
        track_id: &str,
        cancellation: &MediaCancellation,
    ) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
        if !self.options.allow_recommendations || !valid_numeric_identifier(track_id) {
            return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions));
        }
        let endpoint = format!(
            "{}/tracks/{track_id}/similar",
            self.options.api_base_url.trim_end_matches('/')
        );
        let body = self.authenticated_get(endpoint, cancellation)?;
        parse_recommendations_response(&body, &self.options)
    }

    fn collection_page_size(&self, tracks_per_page: usize) -> Result<usize, YandexMusicError> {
        tracks_per_page
            .checked_mul(self.options.max_collection_pages)
            .map(|size| size.min(self.options.max_collection_tracks))
            .ok_or_else(|| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))
    }
}

impl fmt::Debug for YandexMusicSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("YandexMusicSourceManager")
            .field("options", &self.options)
            .field("authentication", &self.authentication)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

impl SourceManager<YandexMusicSourceItem> for YandexMusicSourceManager {
    fn source_name(&self) -> &'static str {
        "yandex-music"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<YandexMusicSourceItem>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<YandexMusicSourceItem>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_yandex_music_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        let linked = linked_cancellation(cancellation);
        match self.load_route(&route, &linked) {
            Ok(Some(item)) => Ok(Some(SourceLoad::Item(item))),
            Ok(None) => Ok(Some(SourceLoad::Referral(SourceReference::new(
                None, false,
            )))),
            Err(error) if error.kind == YandexMusicErrorKind::Cancelled => Ok(None),
            Err(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, item: &YandexMusicSourceItem) -> bool {
        matches!(item, YandexMusicSourceItem::Track(_))
    }

    fn encode(&self, item: &YandexMusicSourceItem) -> Result<Vec<u8>, SourceRegistryError> {
        if matches!(item, YandexMusicSourceItem::Track(_)) {
            Ok(Vec::new())
        } else {
            Err(SourceRegistryError::NotEncodable)
        }
    }

    fn decode(&self, _payload: &[u8]) -> Result<YandexMusicSourceItem, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<YandexMusicSourceItem, SourceRegistryError> {
        if !payload.is_empty() || !valid_numeric_identifier(&info.identifier) {
            return Err(SourceRegistryError::SourceFailure);
        }
        Ok(YandexMusicSourceItem::Track(YandexMusicSourceTrack {
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

fn map_remote_error(error: crate::RemoteHttpError) -> YandexMusicError {
    let kind = match error.kind() {
        RemoteHttpErrorKind::Cancelled => YandexMusicErrorKind::Cancelled,
        RemoteHttpErrorKind::RateLimited => YandexMusicErrorKind::RateLimited,
        RemoteHttpErrorKind::Unauthorized | RemoteHttpErrorKind::Forbidden => {
            YandexMusicErrorKind::AuthenticationRequired
        }
        RemoteHttpErrorKind::NotFound => YandexMusicErrorKind::Unavailable,
        RemoteHttpErrorKind::InvalidResponse | RemoteHttpErrorKind::ResponseTooLarge => {
            YandexMusicErrorKind::InvalidResponse
        }
        _ => YandexMusicErrorKind::Network,
    };
    YandexMusicError::new(kind)
}

#[derive(Deserialize)]
struct TrackResponse {
    result: Vec<TrackResponseItem>,
}

#[derive(Deserialize)]
struct ResultEnvelope<T> {
    result: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackResponseItem {
    #[serde(default)]
    available: bool,
    id: String,
    title: String,
    duration_ms: u64,
    #[serde(default)]
    artists: Vec<ArtistResponse>,
    cover_uri: Option<String>,
    og_image: Option<String>,
}

#[derive(Deserialize)]
struct ArtistResponse {
    name: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TrackEntry {
    Wrapped { track: TrackResponseItem },
    Direct(TrackResponseItem),
}

impl TrackEntry {
    fn into_track(self) -> TrackResponseItem {
        match self {
            Self::Wrapped { track } | Self::Direct(track) => track,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlbumResult {
    title: String,
    #[serde(default)]
    artists: Vec<ArtistResponse>,
    #[serde(default)]
    volumes: Vec<Vec<TrackEntry>>,
    cover_uri: Option<String>,
    og_image: Option<String>,
}

#[derive(Deserialize)]
struct ArtistTracksResult {
    #[serde(default)]
    tracks: Vec<TrackEntry>,
}

#[derive(Deserialize)]
struct ArtistDetailsResult {
    artist: ArtistDetails,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtistDetails {
    name: String,
    cover_uri: Option<String>,
    og_image: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistResult {
    kind: serde_json::Value,
    title: Option<String>,
    owner: Option<PlaylistOwner>,
    #[serde(default)]
    tracks: Vec<TrackEntry>,
    cover_uri: Option<String>,
    og_image: Option<String>,
}

#[derive(Deserialize)]
struct PlaylistOwner {
    name: Option<String>,
    login: Option<String>,
}

#[derive(Deserialize)]
struct SearchResult {
    tracks: Option<SearchTracks>,
}

#[derive(Deserialize)]
struct SearchTracks {
    #[serde(default)]
    results: Vec<TrackEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecommendationsResult {
    #[serde(default)]
    similar_tracks: Vec<TrackEntry>,
}

#[derive(Deserialize)]
struct DownloadCandidatesResponse {
    result: Option<Vec<DownloadCandidate>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadCandidate {
    codec: String,
    bitrate_in_kbps: u32,
    download_info_url: String,
}

struct DownloadInfoDocument {
    host: String,
    path: String,
    timestamp: String,
    secret: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum DownloadInfoField {
    Host,
    Path,
    Timestamp,
    Secret,
}

impl DownloadInfoField {
    const fn index(self) -> usize {
        match self {
            Self::Host => 0,
            Self::Path => 1,
            Self::Timestamp => 2,
            Self::Secret => 3,
        }
    }
}

fn parse_track_response(
    bytes: &[u8],
    requested_id: &str,
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourceTrack>, YandexMusicError> {
    let mut response: TrackResponse = serde_json::from_slice(bytes)
        .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidResponse))?;
    if response.result.is_empty() {
        return Ok(None);
    }
    if response.result.len() != 1 {
        return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidResponse));
    }
    let item = response
        .result
        .pop()
        .ok_or_else(|| YandexMusicError::new(YandexMusicErrorKind::InvalidResponse))?;
    parse_track_item(item, Some(requested_id), domain, options)
}

fn parse_track_item(
    item: TrackResponseItem,
    requested_id: Option<&str>,
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourceTrack>, YandexMusicError> {
    if !item.available {
        return Ok(None);
    }
    if requested_id.is_some_and(|requested| item.id != requested)
        || !valid_numeric_identifier(&item.id)
        || !bounded_nonempty(&item.title, options.max_metadata_string_bytes)
        || item.artists.len() > options.max_artists
        || !valid_domain(domain)
    {
        return Err(invalid_response());
    }
    let author = parse_artists(item.artists, options.max_metadata_string_bytes)?;
    let artwork_url = item
        .og_image
        .or(item.cover_uri)
        .map(|cover| parse_cover_uri(&cover, options.max_metadata_string_bytes))
        .transpose()?;
    Ok(Some(YandexMusicSourceTrack {
        info: TrackInfo {
            title: item.title,
            author,
            duration: Duration::from_millis(item.duration_ms),
            identifier: item.id.clone(),
            is_stream: false,
            uri: Some(format!("https://music.yandex.{domain}/track/{}", item.id)),
            artwork_url,
            isrc: None,
        },
    }))
}

fn parse_album_response(
    bytes: &[u8],
    album_id: &str,
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
    let response: ResultEnvelope<AlbumResult> = parse_json(bytes)?;
    let Some(album) = response.result else {
        return Ok(None);
    };
    if !bounded_nonempty(&album.title, options.max_metadata_string_bytes)
        || album.artists.len() > options.max_artists
    {
        return Err(invalid_response());
    }
    let entry_count = album.volumes.iter().try_fold(0_usize, |count, volume| {
        count
            .checked_add(volume.len())
            .filter(|count| *count <= options.max_collection_tracks)
    });
    if entry_count.is_none() {
        return Err(invalid_response());
    }
    let tracks = parse_collection_entries(
        album.volumes.into_iter().flatten().collect(),
        domain,
        options,
    )?;
    if tracks.is_empty() {
        return Ok(None);
    }
    let author = parse_optional_artists(album.artists, options.max_metadata_string_bytes)?;
    let artwork_url = parse_optional_cover(
        album.og_image.or(album.cover_uri),
        options.max_metadata_string_bytes,
    )?;
    Ok(Some(YandexMusicSourcePlaylist {
        name: album.title,
        tracks,
        selected_track: None,
        is_search_result: false,
        kind: YandexMusicPlaylistKind::Album,
        uri: Some(format!("https://music.yandex.{domain}/album/{album_id}")),
        artwork_url,
        author,
    }))
}

fn parse_artist_tracks_response(
    bytes: &[u8],
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Vec<YandexMusicSourceTrack>, YandexMusicError> {
    let response: ResultEnvelope<ArtistTracksResult> = parse_json(bytes)?;
    let Some(result) = response.result else {
        return Ok(Vec::new());
    };
    parse_collection_entries(result.tracks, domain, options)
}

fn parse_artist_response(
    bytes: &[u8],
    artist_id: &str,
    domain: &str,
    tracks: Vec<YandexMusicSourceTrack>,
    options: &YandexMusicSourceOptions,
) -> Result<YandexMusicSourcePlaylist, YandexMusicError> {
    let response: ResultEnvelope<ArtistDetailsResult> = parse_json(bytes)?;
    let details = response.result.ok_or_else(invalid_response)?.artist;
    if !bounded_nonempty(&details.name, options.max_metadata_string_bytes) {
        return Err(invalid_response());
    }
    let name = format!("{}'s Top Tracks", details.name);
    if name.len() > options.max_metadata_string_bytes {
        return Err(invalid_response());
    }
    let artwork_url = parse_optional_cover(
        details.og_image.or(details.cover_uri),
        options.max_metadata_string_bytes,
    )?;
    Ok(YandexMusicSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: false,
        kind: YandexMusicPlaylistKind::Artist,
        uri: Some(format!("https://music.yandex.{domain}/artist/{artist_id}")),
        artwork_url,
        author: Some(details.name),
    })
}

fn parse_playlist_response(
    bytes: &[u8],
    requested_owner: Option<&str>,
    playlist_id: &str,
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
    let response: ResultEnvelope<PlaylistResult> = parse_json(bytes)?;
    let Some(playlist) = response.result else {
        return Ok(None);
    };
    let tracks = parse_collection_entries(playlist.tracks, domain, options)?;
    if tracks.is_empty() {
        return Ok(None);
    }
    let author = parse_playlist_owner(playlist.owner, options.max_metadata_string_bytes)?;
    let name = if playlist_kind_is_liked(&playlist.kind) {
        format!(
            "{}'s liked songs",
            author.as_deref().ok_or_else(invalid_response)?
        )
    } else {
        playlist.title.ok_or_else(invalid_response)?
    };
    if !bounded_nonempty(&name, options.max_metadata_string_bytes) {
        return Err(invalid_response());
    }
    let artwork_url = parse_optional_cover(
        playlist.og_image.or(playlist.cover_uri),
        options.max_metadata_string_bytes,
    )?;
    let uri = requested_owner.map_or_else(
        || format!("https://music.yandex.{domain}/playlists/{playlist_id}"),
        |owner| format!("https://music.yandex.{domain}/users/{owner}/playlists/{playlist_id}"),
    );
    Ok(Some(YandexMusicSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: false,
        kind: YandexMusicPlaylistKind::Playlist,
        uri: Some(uri),
        artwork_url,
        author,
    }))
}

fn parse_search_response(
    bytes: &[u8],
    query: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
    let response: ResultEnvelope<SearchResult> = parse_json(bytes)?;
    let entries = response
        .result
        .and_then(|result| result.tracks)
        .map_or_else(Vec::new, |tracks| tracks.results);
    let tracks = parse_collection_entries(entries, "ru", options)?;
    if tracks.is_empty() {
        return Ok(None);
    }
    let name = format!("Yandex Music Search: {query}");
    if name.len() > options.max_metadata_string_bytes {
        return Err(invalid_response());
    }
    Ok(Some(YandexMusicSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: true,
        kind: YandexMusicPlaylistKind::Search,
        uri: None,
        artwork_url: None,
        author: None,
    }))
}

fn parse_recommendations_response(
    bytes: &[u8],
    options: &YandexMusicSourceOptions,
) -> Result<Option<YandexMusicSourcePlaylist>, YandexMusicError> {
    let response: ResultEnvelope<RecommendationsResult> = parse_json(bytes)?;
    let entries = response
        .result
        .map_or_else(Vec::new, |result| result.similar_tracks);
    let tracks = parse_collection_entries(entries, "ru", options)?;
    if tracks.is_empty() {
        return Ok(None);
    }
    let name = "Yandex Music Recommendations".to_owned();
    if name.len() > options.max_metadata_string_bytes {
        return Err(invalid_response());
    }
    Ok(Some(YandexMusicSourcePlaylist {
        name,
        tracks,
        selected_track: None,
        is_search_result: false,
        kind: YandexMusicPlaylistKind::Recommendations,
        uri: None,
        artwork_url: None,
        author: None,
    }))
}

fn parse_download_candidates(
    bytes: &[u8],
    options: &YandexMusicSourceOptions,
) -> Result<Option<DownloadCandidate>, YandexMusicError> {
    let response: DownloadCandidatesResponse = parse_json(bytes)?;
    let candidates = response.result.unwrap_or_default();
    if candidates.len() > options.max_download_candidates {
        return Err(invalid_response());
    }
    let mut best = None;
    for candidate in candidates {
        if !bounded_nonempty(&candidate.codec, 16) {
            return Err(invalid_response());
        }
        if candidate.codec != "mp3" {
            continue;
        }
        if candidate.bitrate_in_kbps == 0
            || candidate.bitrate_in_kbps > MAX_BITRATE_KBPS
            || !bounded_nonempty(&candidate.download_info_url, options.max_playback_url_bytes)
            || RemoteHttpRequest::get(&candidate.download_info_url).is_err()
        {
            return Err(invalid_response());
        }
        if best.as_ref().is_none_or(|current: &DownloadCandidate| {
            candidate.bitrate_in_kbps >= current.bitrate_in_kbps
        }) {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn validate_download_info_origin(
    url: &str,
    options: &YandexMusicSourceOptions,
) -> Result<(), YandexMusicError> {
    let uri: Uri = url.parse().map_err(|_| invalid_response())?;
    let api_uri: Uri = options
        .api_base_url
        .parse()
        .map_err(|_| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
    let authority = uri.authority().ok_or_else(invalid_response)?;
    let host = authority.host();
    let api_authority = api_uri
        .authority()
        .ok_or_else(|| YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))?;
    let api_host = api_authority.host();
    let same_api_host = host.eq_ignore_ascii_case(api_host);
    let approved_yandex_host = is_yandex_download_host(host);
    if !same_api_host && !approved_yandex_host {
        return Err(invalid_response());
    }
    match uri.scheme_str() {
        Some("https") => Ok(()),
        Some("http")
            if same_api_host
                && options.http.network_access == HttpNetworkAccess::AllowPrivateNetworks =>
        {
            Ok(())
        }
        _ => Err(invalid_response()),
    }
}

fn is_yandex_download_host(host: &str) -> bool {
    ["yandex.net", "yandex.ru", "yandex.com"]
        .into_iter()
        .any(|suffix| {
            host.eq_ignore_ascii_case(suffix)
                || host
                    .strip_suffix(suffix)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
}

fn parse_download_info_xml(
    bytes: &[u8],
    options: &YandexMusicSourceOptions,
) -> Result<DownloadInfoDocument, YandexMusicError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut events = 0_usize;
    let mut depth = 0_usize;
    let mut root_seen = false;
    let mut active = None;
    let mut seen_fields = [false; 4];
    let mut host = None;
    let mut path = None;
    let mut timestamp = None;
    let mut secret = None;
    loop {
        events = events.checked_add(1).ok_or_else(invalid_response)?;
        if events > MAX_DOWNLOAD_INFO_EVENTS {
            return Err(invalid_response());
        }
        match reader.read_event().map_err(|_| invalid_response())? {
            Event::Start(start) => {
                depth = depth.checked_add(1).ok_or_else(invalid_response)?;
                if depth > MAX_DOWNLOAD_INFO_DEPTH || active.is_some() {
                    return Err(invalid_response());
                }
                let name = start.name();
                if depth == 1 {
                    if root_seen
                        || name.as_ref() != b"download-info"
                        || start.attributes().next().is_some()
                    {
                        return Err(invalid_response());
                    }
                    root_seen = true;
                } else if depth == 2 {
                    let field = download_info_field(name.as_ref());
                    if field.is_some() && start.attributes().next().is_some() {
                        return Err(invalid_response());
                    }
                    if let Some(field) = field {
                        if seen_fields[field.index()] {
                            return Err(invalid_response());
                        }
                        seen_fields[field.index()] = true;
                    }
                    active = field;
                }
            }
            Event::Text(text) => {
                if let Some(field) = active {
                    let value = text.decode().map_err(|_| invalid_response())?.into_owned();
                    if !bounded_nonempty(&value, options.max_playback_url_bytes) {
                        return Err(invalid_response());
                    }
                    let slot = match field {
                        DownloadInfoField::Host => &mut host,
                        DownloadInfoField::Path => &mut path,
                        DownloadInfoField::Timestamp => &mut timestamp,
                        DownloadInfoField::Secret => &mut secret,
                    };
                    if slot.replace(value).is_some() {
                        return Err(invalid_response());
                    }
                }
            }
            Event::End(end) => {
                if depth == 0 {
                    return Err(invalid_response());
                }
                if depth == 2 {
                    let ending_field = download_info_field(end.name().as_ref());
                    if ending_field.is_some() && ending_field != active {
                        return Err(invalid_response());
                    }
                    active = None;
                }
                depth -= 1;
            }
            Event::Eof => {
                if depth != 0 || !root_seen || active.is_some() {
                    return Err(invalid_response());
                }
                break;
            }
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Empty(_)
            | Event::CData(_)
            | Event::PI(_)
            | Event::DocType(_)
            | Event::GeneralRef(_) => return Err(invalid_response()),
        }
    }
    Ok(DownloadInfoDocument {
        host: host.ok_or_else(invalid_response)?,
        path: path.ok_or_else(invalid_response)?,
        timestamp: timestamp.ok_or_else(invalid_response)?,
        secret: secret.ok_or_else(invalid_response)?,
    })
}

fn download_info_field(name: &[u8]) -> Option<DownloadInfoField> {
    match name {
        b"host" => Some(DownloadInfoField::Host),
        b"path" => Some(DownloadInfoField::Path),
        b"ts" => Some(DownloadInfoField::Timestamp),
        b"s" => Some(DownloadInfoField::Secret),
        _ => None,
    }
}

fn build_signed_playback_url(
    download: &DownloadInfoDocument,
    bitrate_kbps: u32,
    options: &YandexMusicSourceOptions,
) -> Result<YandexMusicPlaybackUrl, YandexMusicError> {
    if !bounded_nonempty(&download.host, options.max_playback_url_bytes)
        || download
            .host
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'@' | b'?' | b'#'))
        || !download.path.starts_with('/')
        || download
            .path
            .bytes()
            .any(|byte| !byte.is_ascii_graphic() || matches!(byte, b'?' | b'#' | b'\\'))
        || download.timestamp.is_empty()
        || download.timestamp.len() > MAX_NUMERIC_IDENTIFIER_BYTES
        || !download.timestamp.bytes().all(|byte| byte.is_ascii_digit())
        || !bounded_nonempty(&download.secret, options.max_playback_url_bytes)
        || !download.secret.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(invalid_response());
    }
    let signing_input = format!("{DOWNLOAD_SIGN_SALT}{}{}", download.path, download.secret);
    if signing_input.len() > options.max_playback_url_bytes {
        return Err(invalid_response());
    }
    let digest = Md5::digest(signing_input.as_bytes());
    let mut checksum = String::with_capacity(32);
    for byte in digest {
        write!(&mut checksum, "{byte:02x}").map_err(|_| invalid_response())?;
    }
    let scheme = match options.playback_scheme {
        YandexMusicPlaybackScheme::Https => "https",
        YandexMusicPlaybackScheme::HttpForPrivateNetworks => "http",
    };
    let url = format!(
        "{scheme}://{}/get-mp3/{checksum}/{}{}",
        download.host, download.timestamp, download.path
    );
    if url.len() > options.max_playback_url_bytes || RemoteHttpRequest::get(&url).is_err() {
        return Err(invalid_response());
    }
    Ok(YandexMusicPlaybackUrl { url, bitrate_kbps })
}

fn parse_collection_entries(
    entries: Vec<TrackEntry>,
    domain: &str,
    options: &YandexMusicSourceOptions,
) -> Result<Vec<YandexMusicSourceTrack>, YandexMusicError> {
    if entries.len() > options.max_collection_tracks {
        return Err(invalid_response());
    }
    entries
        .into_iter()
        .filter_map(|entry| parse_track_item(entry.into_track(), None, domain, options).transpose())
        .collect()
}

fn parse_json<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, YandexMusicError> {
    serde_json::from_slice(bytes).map_err(|_| invalid_response())
}

fn parse_optional_artists(
    artists: Vec<ArtistResponse>,
    max_string_bytes: usize,
) -> Result<Option<String>, YandexMusicError> {
    if artists.is_empty() {
        Ok(None)
    } else {
        parse_artists(artists, max_string_bytes).map(Some)
    }
}

fn parse_playlist_owner(
    owner: Option<PlaylistOwner>,
    max_string_bytes: usize,
) -> Result<Option<String>, YandexMusicError> {
    let Some(owner) = owner else {
        return Ok(None);
    };
    let value = owner.name.or(owner.login);
    if value
        .as_deref()
        .is_some_and(|value| !bounded_nonempty(value, max_string_bytes))
    {
        return Err(invalid_response());
    }
    Ok(value)
}

fn playlist_kind_is_liked(kind: &serde_json::Value) -> bool {
    kind.as_u64() == Some(3) || kind.as_str() == Some("3")
}

fn parse_optional_cover(
    cover: Option<String>,
    max_string_bytes: usize,
) -> Result<Option<String>, YandexMusicError> {
    cover
        .map(|cover| parse_cover_uri(&cover, max_string_bytes))
        .transpose()
}

fn parse_artists(
    artists: Vec<ArtistResponse>,
    max_string_bytes: usize,
) -> Result<String, YandexMusicError> {
    if artists.is_empty() {
        return Ok("Unknown".to_owned());
    }
    if artists
        .iter()
        .any(|artist| !bounded_nonempty(&artist.name, max_string_bytes))
    {
        return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidResponse));
    }
    let joined = artists
        .into_iter()
        .map(|artist| artist.name)
        .collect::<Vec<_>>()
        .join(", ");
    if joined.len() > max_string_bytes {
        return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidResponse));
    }
    Ok(joined)
}

fn parse_cover_uri(cover: &str, max_string_bytes: usize) -> Result<String, YandexMusicError> {
    if !bounded_nonempty(cover, max_string_bytes)
        || cover.contains("://")
        || cover.contains('@')
        || cover.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidResponse));
    }
    let url = format!("https://{}", cover.replace("%%", "400x400"));
    if url.len() > max_string_bytes || RemoteHttpRequest::get(&url).is_err() {
        return Err(YandexMusicError::new(YandexMusicErrorKind::InvalidResponse));
    }
    Ok(url)
}

fn bounded_nonempty(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes
}

fn valid_domain(domain: &str) -> bool {
    matches!(domain, "ru" | "com" | "kz" | "by")
}

fn validate_numeric_route(identifier: &str, domain: &str) -> Result<(), YandexMusicError> {
    if valid_numeric_identifier(identifier) && valid_domain(domain) {
        Ok(())
    } else {
        Err(YandexMusicError::new(YandexMusicErrorKind::InvalidOptions))
    }
}

const fn map_playback_source_error(error: YandexMusicError) -> YandexMusicPlaybackError {
    if matches!(error.kind(), YandexMusicErrorKind::Cancelled) {
        YandexMusicPlaybackError::new(YandexMusicPlaybackErrorKind::Cancelled)
    } else {
        YandexMusicPlaybackError::new(YandexMusicPlaybackErrorKind::Source(error.kind()))
    }
}

fn map_playback_media_error(error: MediaError) -> YandexMusicPlaybackError {
    let kind = match error {
        MediaError::Cancelled => YandexMusicPlaybackErrorKind::Cancelled,
        MediaError::InvalidLimits(_) | MediaError::InvalidHttpOptions(_) => {
            YandexMusicPlaybackErrorKind::InvalidOptions
        }
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted => {
            YandexMusicPlaybackErrorKind::Cancelled
        }
        MediaError::Io(_) => YandexMusicPlaybackErrorKind::Network,
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
        | MediaError::Backend { .. } => YandexMusicPlaybackErrorKind::InvalidMedia,
    };
    YandexMusicPlaybackError::new(kind)
}

fn invalid_response() -> YandexMusicError {
    YandexMusicError::new(YandexMusicErrorKind::InvalidResponse)
}
