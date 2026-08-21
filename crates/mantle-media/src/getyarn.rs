use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use ureq::http::Uri;

const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_CLIP_ID_BYTES: usize = 1024;
const MAX_CONFIGURED_MEDIA_URL_BYTES: usize = 1024 * 1024;

/// A bounded historical Getyarn clip-page route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetyarnRoute {
    pub clip_id: String,
    pub original_url: String,
}

impl GetyarnRoute {
    #[must_use]
    pub fn canonical_url(&self) -> String {
        format!("https://getyarn.io/yarn-clip/{}", self.clip_id)
    }
}

/// Limits for compatibility routing and legacy empty-detail reconstruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetyarnSourceOptions {
    pub max_identifier_bytes: usize,
    pub max_clip_id_bytes: usize,
    pub max_media_url_bytes: usize,
}

impl Default for GetyarnSourceOptions {
    fn default() -> Self {
        Self {
            max_identifier_bytes: 8 * 1024,
            max_clip_id_bytes: 128,
            max_media_url_bytes: 64 * 1024,
        }
    }
}

impl GetyarnSourceOptions {
    fn validate(self) -> Result<Self, GetyarnError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_clip_id_bytes == 0
            || self.max_clip_id_bytes > MAX_CONFIGURED_CLIP_ID_BYTES
            || self.max_media_url_bytes == 0
            || self.max_media_url_bytes > MAX_CONFIGURED_MEDIA_URL_BYTES
        {
            return Err(GetyarnError::new(GetyarnErrorKind::InvalidOptions));
        }
        Ok(self)
    }
}

/// Recognizes the historical Getyarn clip-page shape without network access.
#[must_use]
pub fn route_getyarn_identifier(
    identifier: &str,
    options: &GetyarnSourceOptions,
) -> Option<GetyarnRoute> {
    if identifier.is_empty()
        || identifier.len() > options.max_identifier_bytes
        || identifier.contains('#')
    {
        return None;
    }
    let uri: Uri = identifier.parse().ok()?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.query().is_some() {
        return None;
    }
    let authority = uri.authority()?;
    if authority.as_str().contains('@') || authority.as_str() != authority.host() {
        return None;
    }
    if !matches!(
        authority.host().to_ascii_lowercase().as_str(),
        "getyarn.io" | "www.getyarn.io"
    ) {
        return None;
    }
    let clip_id = uri.path().strip_prefix("/yarn-clip/")?;
    if !valid_clip_id(clip_id, options.max_clip_id_bytes) {
        return None;
    }
    Some(GetyarnRoute {
        clip_id: clip_id.to_owned(),
        original_url: identifier.to_owned(),
    })
}

fn valid_clip_id(clip_id: &str, limit: usize) -> bool {
    !clip_id.is_empty()
        && clip_id.len() <= limit
        && clip_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_legacy_media_url(url: &str, limit: usize) -> bool {
    if url.is_empty() || url.len() > limit || url.contains('#') {
        return false;
    }
    let Ok(uri) = url.parse::<Uri>() else {
        return false;
    };
    let Some(authority) = uri.authority() else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && !authority.as_str().contains('@')
        && authority.as_str() == authority.host()
}

/// A track reconstructed from Lavaplayer's historical empty Getyarn source details.
#[derive(Clone, Eq, PartialEq)]
pub struct GetyarnSourceTrack {
    pub info: TrackInfo,
    pub clip_id: String,
    pub page_url: String,
}

impl fmt::Debug for GetyarnSourceTrack {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GetyarnSourceTrack")
            .field("title", &self.info.title)
            .field("author", &self.info.author)
            .field("duration", &self.info.duration)
            .field("is_stream", &self.info.is_stream)
            .field("media_identifier", &"<redacted>")
            .field("clip_id", &self.clip_id)
            .field("page_url", &self.page_url)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GetyarnErrorKind {
    InvalidOptions,
    UnsupportedRoute,
    InvalidSourceDetails,
    Cancelled,
    Shutdown,
    UnsupportedPlayback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GetyarnError {
    kind: GetyarnErrorKind,
}

impl GetyarnError {
    const fn new(kind: GetyarnErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> GetyarnErrorKind {
        self.kind
    }
}

impl fmt::Display for GetyarnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            GetyarnErrorKind::InvalidOptions => "invalid Getyarn compatibility policy",
            GetyarnErrorKind::UnsupportedRoute => "Getyarn route is not implemented",
            GetyarnErrorKind::InvalidSourceDetails => "invalid legacy Getyarn source details",
            GetyarnErrorKind::Cancelled => "Getyarn compatibility load cancelled",
            GetyarnErrorKind::Shutdown => "Getyarn compatibility source is shut down",
            GetyarnErrorKind::UnsupportedPlayback => {
                "Getyarn playback has no supported current protocol"
            }
        })
    }
}

impl std::error::Error for GetyarnError {}

/// Compatibility-only Getyarn manager. It contains no network client by design.
pub struct GetyarnSourceManager {
    options: GetyarnSourceOptions,
    shutdown: AtomicBool,
}

impl GetyarnSourceManager {
    /// Creates a bounded compatibility manager.
    ///
    /// # Errors
    ///
    /// Returns [`GetyarnErrorKind::InvalidOptions`] for zero or excessive reconstruction limits.
    pub fn new(options: GetyarnSourceOptions) -> Result<Self, GetyarnError> {
        Ok(Self {
            options: options.validate()?,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Reports the deterministic result for one recognized historical clip page.
    ///
    /// # Errors
    ///
    /// Always returns [`GetyarnErrorKind::UnsupportedPlayback`] for a valid route, or a
    /// lifecycle/route error first. No page request or scraper exists in this manager.
    pub fn load_route(
        &self,
        route: &GetyarnRoute,
        cancellation: &SourceCancellation,
    ) -> Result<GetyarnSourceTrack, GetyarnError> {
        self.ensure_active(cancellation)?;
        if route_getyarn_identifier(&route.original_url, &self.options).as_ref() != Some(route) {
            return Err(GetyarnError::new(GetyarnErrorKind::UnsupportedRoute));
        }
        Err(GetyarnError::new(GetyarnErrorKind::UnsupportedPlayback))
    }

    /// Reports the deterministic playback result for a reconstructed legacy track.
    ///
    /// # Errors
    ///
    /// Always returns [`GetyarnErrorKind::UnsupportedPlayback`] for valid legacy state. No direct
    /// media request exists in this manager.
    pub fn open_track_playback(
        &self,
        track: &GetyarnSourceTrack,
        cancellation: &SourceCancellation,
    ) -> Result<(), GetyarnError> {
        self.ensure_active(cancellation)?;
        self.validate_track(track)?;
        Err(GetyarnError::new(GetyarnErrorKind::UnsupportedPlayback))
    }

    fn ensure_active(&self, cancellation: &SourceCancellation) -> Result<(), GetyarnError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(GetyarnError::new(GetyarnErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(GetyarnError::new(GetyarnErrorKind::Cancelled));
        }
        Ok(())
    }

    fn validate_track(&self, track: &GetyarnSourceTrack) -> Result<(), GetyarnError> {
        let route = parse_legacy_track_info(&track.info, &self.options)
            .ok_or_else(|| GetyarnError::new(GetyarnErrorKind::InvalidSourceDetails))?;
        if route.clip_id != track.clip_id || route.original_url != track.page_url {
            return Err(GetyarnError::new(GetyarnErrorKind::InvalidSourceDetails));
        }
        Ok(())
    }
}

impl Default for GetyarnSourceManager {
    fn default() -> Self {
        Self::new(GetyarnSourceOptions::default()).expect("default Getyarn policy must be valid")
    }
}

impl fmt::Debug for GetyarnSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GetyarnSourceManager")
            .field("options", &self.options)
            .field("network_enabled", &false)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish()
    }
}

impl SourceManager<GetyarnSourceTrack> for GetyarnSourceManager {
    fn source_name(&self) -> &'static str {
        "getyarn.io"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<GetyarnSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<GetyarnSourceTrack>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_getyarn_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        match self.load_route(&route, cancellation) {
            Err(error) if error.kind() == GetyarnErrorKind::UnsupportedPlayback => Ok(Some(
                SourceLoad::Referral(SourceReference::new(None, false)),
            )),
            Err(error) if error.kind() == GetyarnErrorKind::Cancelled => Ok(None),
            Err(_) | Ok(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, item: &GetyarnSourceTrack) -> bool {
        self.validate_track(item).is_ok()
    }

    fn encode(&self, item: &GetyarnSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        self.is_encodable(item)
            .then(Vec::new)
            .ok_or(SourceRegistryError::NotEncodable)
    }

    fn decode(&self, _payload: &[u8]) -> Result<GetyarnSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<GetyarnSourceTrack, SourceRegistryError> {
        if !payload.is_empty() {
            return Err(SourceRegistryError::SourceFailure);
        }
        let route = parse_legacy_track_info(info, &self.options)
            .ok_or(SourceRegistryError::SourceFailure)?;
        Ok(GetyarnSourceTrack {
            info: info.clone(),
            clip_id: route.clip_id,
            page_url: route.original_url,
        })
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn parse_legacy_track_info(
    info: &TrackInfo,
    options: &GetyarnSourceOptions,
) -> Option<GetyarnRoute> {
    if info.is_stream || !valid_legacy_media_url(&info.identifier, options.max_media_url_bytes) {
        return None;
    }
    route_getyarn_identifier(info.uri.as_deref()?, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_url_upgrades_historical_http_pages() {
        let route = GetyarnRoute {
            clip_id: "fixture-id".to_owned(),
            original_url: "http://www.getyarn.io/yarn-clip/fixture-id".to_owned(),
        };
        assert_eq!(
            route.canonical_url(),
            "https://getyarn.io/yarn-clip/fixture-id"
        );
    }
}
