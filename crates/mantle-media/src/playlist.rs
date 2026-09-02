use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::sync::Arc;

use ureq::http::Uri;

use crate::{
    HttpStreamInput, HttpStreamOptions, MediaCancellation, MediaError, MediaInput,
    OutboundRoutePolicy,
};

const PLAIN_PROBE_BYTES: usize = 1_000;

/// Resource limits for an in-memory playlist probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaylistLimits {
    pub max_playlist_bytes: usize,
    pub max_line_bytes: usize,
    pub max_entries: usize,
}

impl Default for PlaylistLimits {
    fn default() -> Self {
        Self {
            max_playlist_bytes: 1024 * 1024,
            max_line_bytes: 16 * 1024,
            max_entries: 1_000,
        }
    }
}

impl PlaylistLimits {
    pub(crate) fn validate(self) -> Result<Self, PlaylistError> {
        if self.max_playlist_bytes == 0 {
            return Err(PlaylistError::InvalidLimits(
                "max_playlist_bytes must be non-zero",
            ));
        }
        if self.max_line_bytes == 0 {
            return Err(PlaylistError::InvalidLimits(
                "max_line_bytes must be non-zero",
            ));
        }
        if self.max_entries == 0 {
            return Err(PlaylistError::InvalidLimits("max_entries must be non-zero"));
        }
        Ok(self)
    }
}

/// Combined HTTP and parser policy for loading one playlist response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpPlaylistOptions {
    pub http: HttpStreamOptions,
    pub playlist: PlaylistLimits,
    pub include_plain: bool,
}

impl Default for HttpPlaylistOptions {
    fn default() -> Self {
        let playlist = PlaylistLimits::default();
        let http = HttpStreamOptions {
            max_response_bytes: u64::try_from(playlist.max_playlist_bytes).unwrap_or(u64::MAX),
            ..HttpStreamOptions::default()
        };
        Self {
            http,
            playlist,
            include_plain: false,
        }
    }
}

/// The playlist boundary selected by [`probe_playlist`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaylistFormat {
    M3u,
    Hls,
    Pls,
    Plain,
}

/// A source reference selected from a playlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistReference {
    pub identifier: String,
    pub title: Option<String>,
}

/// A detected playlist and its first compatible source reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistMatch {
    pub format: PlaylistFormat,
    pub reference: PlaylistReference,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaylistError {
    InvalidLimits(&'static str),
    TooLarge { actual: usize, limit: usize },
    LineTooLong { actual: usize, limit: usize },
    TooManyEntries { limit: usize },
    InvalidReference(&'static str),
}

impl fmt::Display for PlaylistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits(message) => write!(formatter, "invalid playlist limits: {message}"),
            Self::TooLarge { actual, limit } => {
                write!(formatter, "playlist has {actual} bytes; limit is {limit}")
            }
            Self::LineTooLong { actual, limit } => {
                write!(
                    formatter,
                    "playlist line has {actual} bytes; limit is {limit}"
                )
            }
            Self::TooManyEntries { limit } => {
                write!(formatter, "playlist contains more than {limit} entries")
            }
            Self::InvalidReference(message) => {
                write!(formatter, "invalid playlist reference: {message}")
            }
        }
    }
}

impl std::error::Error for PlaylistError {}

#[derive(Debug)]
pub enum PlaylistLoadError {
    Media(MediaError),
    Playlist(PlaylistError),
}

impl fmt::Display for PlaylistLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Media(error) => error.fmt(formatter),
            Self::Playlist(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PlaylistLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Media(error) => Some(error),
            Self::Playlist(error) => Some(error),
        }
    }
}

impl From<MediaError> for PlaylistLoadError {
    fn from(error: MediaError) -> Self {
        Self::Media(error)
    }
}

impl From<PlaylistError> for PlaylistLoadError {
    fn from(error: PlaylistError) -> Self {
        Self::Playlist(error)
    }
}

/// Loads, bounds, probes, and resolves one HTTP playlist response.
///
/// # Errors
///
/// Returns an error for HTTP policy/fetch failures, parser limits, or a selected reference that
/// is not a valid HTTP(S) URI after resolution.
pub fn load_http_playlist(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
) -> Result<Option<PlaylistMatch>, PlaylistLoadError> {
    load_http_playlist_with_cancellation(url, options, MediaCancellation::new())
}

/// Loads, bounds, probes, and resolves one HTTP playlist response with cancellation.
///
/// # Errors
///
/// Returns [`PlaylistLoadError::Media`] when cancellation or HTTP input fails and
/// [`PlaylistLoadError::Playlist`] for parser or reference failures.
pub fn load_http_playlist_with_cancellation(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    cancellation: MediaCancellation,
) -> Result<Option<PlaylistMatch>, PlaylistLoadError> {
    let (base, bytes) = load_http_bytes(url, options, cancellation)?;
    let Some(mut matched) = probe_playlist(&bytes, options.include_plain, options.playlist)? else {
        return Ok(None);
    };
    matched.reference.identifier = resolve_http_reference(&base, &matched.reference.identifier)?;
    Ok(Some(matched))
}

pub(crate) fn load_http_bytes(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    cancellation: MediaCancellation,
) -> Result<(String, Vec<u8>), PlaylistLoadError> {
    load_http_bytes_inner(url, options, cancellation, None)
}

pub(crate) fn load_http_bytes_routed(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    cancellation: MediaCancellation,
    route_policy: Arc<dyn OutboundRoutePolicy>,
) -> Result<(String, Vec<u8>), PlaylistLoadError> {
    load_http_bytes_inner(url, options, cancellation, Some(route_policy))
}

fn load_http_bytes_inner(
    url: impl AsRef<str>,
    options: HttpPlaylistOptions,
    cancellation: MediaCancellation,
    route_policy: Option<Arc<dyn OutboundRoutePolicy>>,
) -> Result<(String, Vec<u8>), PlaylistLoadError> {
    let playlist_limits = options.playlist.validate()?;
    let mut http_options = options.http;
    http_options.max_response_bytes = http_options
        .max_response_bytes
        .min(u64::try_from(playlist_limits.max_playlist_bytes).unwrap_or(u64::MAX));
    let cancellation_state = cancellation.clone();
    let mut input = if let Some(route_policy) = route_policy {
        HttpStreamInput::open_routed_with_cancellation(
            url,
            http_options,
            cancellation,
            route_policy,
        )?
    } else {
        HttpStreamInput::open_with_cancellation(url, http_options, cancellation)?
    };
    let base = input.final_uri().to_string();
    let capacity = input
        .byte_len()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0)
        .min(playlist_limits.max_playlist_bytes);
    let mut bytes = Vec::with_capacity(capacity);
    if let Err(error) = input.read_to_end(&mut bytes) {
        if cancellation_state.is_cancelled() {
            return Err(PlaylistLoadError::Media(MediaError::Cancelled));
        }
        return Err(PlaylistLoadError::Media(MediaError::Io(error)));
    }
    Ok((base, bytes))
}

/// Resolves a playlist reference against an absolute HTTP(S) base URI.
///
/// Fragments are removed because they are not part of an HTTP request target. Dot segments are
/// removed without decoding percent escapes. Credentials and non-HTTP schemes are rejected.
///
/// # Errors
///
/// Returns an error when the base or resolved reference is not an absolute HTTP(S) URI without
/// user information.
pub fn resolve_http_reference(base: &str, reference: &str) -> Result<String, PlaylistError> {
    let base = parse_absolute_http_uri(base)?;
    resolve_http_uri(&base, reference)
}

/// Probes bounded bytes for the Phase 10 M3U, PLS, and plain-list boundaries.
///
/// Plain-list detection is opt-in because its only signature is an HTTP-compatible identifier at
/// byte zero. M3U and PLS retain their reference probe order. Invalid UTF-8 outside an identifier
/// is replaced, matching Java's ordinary UTF-8 reader behavior without permitting unbounded text.
///
/// # Errors
///
/// Returns an error when limits are invalid or a matching playlist exceeds a byte, line, or entry
/// limit. A bounded input that does not match a playlist returns `Ok(None)`.
pub fn probe_playlist(
    bytes: &[u8],
    include_plain: bool,
    limits: PlaylistLimits,
) -> Result<Option<PlaylistMatch>, PlaylistError> {
    let limits = limits.validate()?;
    if bytes.len() > limits.max_playlist_bytes {
        return Err(PlaylistError::TooLarge {
            actual: bytes.len(),
            limit: limits.max_playlist_bytes,
        });
    }

    let detected = if bytes.starts_with(b"#EXTM3U") || bytes.starts_with(b"#EXTINF") {
        Some(PlaylistFormat::M3u)
    } else if has_pls_header(bytes) {
        Some(PlaylistFormat::Pls)
    } else if include_plain
        && starts_with_supported_identifier(&bytes[..bytes.len().min(PLAIN_PROBE_BYTES)])
    {
        Some(PlaylistFormat::Plain)
    } else {
        None
    };
    let Some(detected) = detected else {
        return Ok(None);
    };

    validate_line_lengths(bytes, limits.max_line_bytes)?;
    let text = String::from_utf8_lossy(bytes);
    match detected {
        PlaylistFormat::M3u => parse_m3u(&text, limits),
        PlaylistFormat::Pls => parse_pls(&text, limits),
        PlaylistFormat::Plain => parse_plain(&text, limits),
        PlaylistFormat::Hls => unreachable!("HLS is classified while parsing M3U"),
    }
}

fn has_pls_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"[playlist]") || bytes.starts_with(b"[Playlist]")
}

pub(crate) fn validate_line_lengths(bytes: &[u8], limit: usize) -> Result<(), PlaylistError> {
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.len() > limit {
            return Err(PlaylistError::LineTooLong {
                actual: line.len(),
                limit,
            });
        }
    }
    Ok(())
}

fn parse_m3u(text: &str, limits: PlaylistLimits) -> Result<Option<PlaylistMatch>, PlaylistError> {
    let mut first_referral = None;
    let mut first_hls = None;
    let mut pending_stream = false;
    let mut entries = 0;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with("#EXTINF") {
            pending_stream = true;
            continue;
        }
        if line.starts_with("#EXT-X-STREAM-INF") {
            pending_stream = true;
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        increment_entries(&mut entries, limits.max_entries)?;
        if pending_stream {
            pending_stream = false;
            if first_hls.is_none() {
                first_hls = Some(PlaylistReference {
                    identifier: normalize_identifier(line),
                    title: None,
                });
            }
        } else if first_referral.is_none() && starts_with_supported_identifier(line.as_bytes()) {
            first_referral = Some(PlaylistReference {
                identifier: normalize_identifier(line),
                title: None,
            });
        }
    }

    Ok(first_hls
        .map(|reference| PlaylistMatch {
            format: PlaylistFormat::Hls,
            reference,
        })
        .or_else(|| {
            first_referral.map(|reference| PlaylistMatch {
                format: PlaylistFormat::M3u,
                reference,
            })
        }))
}

#[derive(Default)]
struct PlsEntry {
    identifier: Option<String>,
    title: Option<String>,
}

fn parse_pls(text: &str, limits: PlaylistLimits) -> Result<Option<PlaylistMatch>, PlaylistError> {
    let mut entries = BTreeMap::<usize, PlsEntry>::new();
    for raw_line in text.lines().skip(1) {
        let line = raw_line.trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some((index, is_file)) = parse_pls_key(key) else {
            continue;
        };
        if !entries.contains_key(&index) && entries.len() == limits.max_entries {
            return Err(PlaylistError::TooManyEntries {
                limit: limits.max_entries,
            });
        }
        let entry = entries.entry(index).or_default();
        if is_file {
            if starts_with_supported_identifier(value.as_bytes()) {
                entry.identifier = Some(normalize_identifier(value));
            }
        } else {
            entry.title = Some(value.to_owned());
        }
    }

    Ok(entries.into_values().find_map(|entry| {
        entry.identifier.map(|identifier| PlaylistMatch {
            format: PlaylistFormat::Pls,
            reference: PlaylistReference {
                identifier,
                title: entry.title,
            },
        })
    }))
}

fn parse_pls_key(key: &str) -> Option<(usize, bool)> {
    let (digits, is_file) = key
        .strip_prefix("File")
        .map(|digits| (digits, true))
        .or_else(|| key.strip_prefix("Title").map(|digits| (digits, false)))?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().map(|index| (index, is_file))
}

fn parse_plain(text: &str, limits: PlaylistLimits) -> Result<Option<PlaylistMatch>, PlaylistError> {
    let mut first = None;
    let mut entries = 0;
    for raw_line in text.lines() {
        let line = raw_line.trim_end_matches('\r');
        if starts_with_supported_identifier(line.as_bytes()) {
            increment_entries(&mut entries, limits.max_entries)?;
            if first.is_none() {
                first = Some(PlaylistReference {
                    identifier: normalize_identifier(line),
                    title: None,
                });
            }
        }
    }
    Ok(first.map(|reference| PlaylistMatch {
        format: PlaylistFormat::Plain,
        reference,
    }))
}

fn resolve_http_uri(base: &Uri, reference: &str) -> Result<String, PlaylistError> {
    let reference = reference.split('#').next().unwrap_or("");
    if reference.starts_with("http://") || reference.starts_with("https://") {
        return parse_absolute_http_uri(reference).map(|uri| uri.to_string());
    }

    let scheme = base
        .scheme_str()
        .ok_or(PlaylistError::InvalidReference("base URI has no scheme"))?;
    let authority = base
        .authority()
        .ok_or(PlaylistError::InvalidReference("base URI has no authority"))?;
    if reference.starts_with("//") {
        return parse_absolute_http_uri(&format!("{scheme}:{reference}"))
            .map(|uri| uri.to_string());
    }
    if reference
        .split(['/', '?'])
        .next()
        .is_some_and(|prefix| prefix.contains(':'))
    {
        return Err(PlaylistError::InvalidReference(
            "reference uses an unsupported scheme",
        ));
    }

    let (reference_path, reference_query) = reference
        .split_once('?')
        .map_or((reference, None), |(path, query)| (path, Some(query)));
    let path = if reference_path.is_empty() {
        base.path().to_owned()
    } else if reference_path.starts_with('/') {
        remove_dot_segments(reference_path)
    } else {
        let base_path = base.path();
        let prefix_end = base_path.rfind('/').map_or(0, |index| index + 1);
        let mut merged = String::with_capacity(prefix_end + reference_path.len());
        if prefix_end == 0 {
            merged.push('/');
        } else {
            merged.push_str(&base_path[..prefix_end]);
        }
        merged.push_str(reference_path);
        remove_dot_segments(&merged)
    };
    let query = if reference_path.is_empty() && reference_query.is_none() {
        base.query()
    } else {
        reference_query
    };

    let mut resolved = format!("{scheme}://{authority}{path}");
    if let Some(query) = query {
        resolved.push('?');
        resolved.push_str(query);
    }
    parse_absolute_http_uri(&resolved).map(|uri| uri.to_string())
}

fn parse_absolute_http_uri(value: &str) -> Result<Uri, PlaylistError> {
    let uri: Uri = value
        .parse()
        .map_err(|_| PlaylistError::InvalidReference("reference is not a valid URI"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
        return Err(PlaylistError::InvalidReference(
            "reference must be an absolute HTTP(S) URI",
        ));
    }
    if uri
        .authority()
        .is_some_and(|authority| authority.as_str().contains('@'))
    {
        return Err(PlaylistError::InvalidReference(
            "reference must not contain user information",
        ));
    }
    Ok(uri)
}

fn remove_dot_segments(path: &str) -> String {
    let mut input = path.to_owned();
    let mut output = String::with_capacity(path.len());
    while !input.is_empty() {
        if input.starts_with("../") {
            input.drain(..3);
        } else if input.starts_with("./") || input.starts_with("/./") {
            input.drain(..2);
        } else if input == "/." {
            input.truncate(1);
        } else if input.starts_with("/../") {
            input.drain(..3);
            remove_last_path_segment(&mut output);
        } else if input == "/.." {
            input.truncate(1);
            remove_last_path_segment(&mut output);
        } else if input == "." || input == ".." {
            input.clear();
        } else {
            let end = if let Some(rest) = input.strip_prefix('/') {
                rest.find('/').map_or(input.len(), |index| index + 1)
            } else {
                input.find('/').unwrap_or(input.len())
            };
            output.push_str(&input[..end]);
            input.drain(..end);
        }
    }
    if output.is_empty() {
        output.push('/');
    }
    output
}

fn remove_last_path_segment(path: &mut String) {
    if let Some(index) = path.rfind('/') {
        path.truncate(index);
    } else {
        path.clear();
    }
}

fn increment_entries(entries: &mut usize, limit: usize) -> Result<(), PlaylistError> {
    if *entries == limit {
        return Err(PlaylistError::TooManyEntries { limit });
    }
    *entries += 1;
    Ok(())
}

fn starts_with_supported_identifier(bytes: &[u8]) -> bool {
    bytes.starts_with(b"http://") || bytes.starts_with(b"https://") || bytes.starts_with(b"icy://")
}

fn normalize_identifier(identifier: &str) -> String {
    identifier.strip_prefix("icy://").map_or_else(
        || identifier.to_owned(),
        |rest| {
            let mut normalized = String::with_capacity("http://".len() + rest.len());
            normalized.push_str("http://");
            normalized.push_str(rest);
            normalized
        },
    )
}
