use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use ureq::http::Uri;

const MAX_CONFIGURED_IDENTIFIER_BYTES: usize = 64 * 1024;
const MAX_CONFIGURED_CHANNEL_BYTES: usize = 1024;
const MAX_CONFIGURED_STREAM_ID_BYTES: usize = 64 * 1024;

/// A bounded historical Beam/Mixer live-channel route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeamRoute {
    pub channel: String,
    pub original_url: String,
}

impl BeamRoute {
    #[must_use]
    pub fn canonical_url(&self) -> String {
        format!("https://beam.pro/{}", self.channel)
    }
}

/// Limits for compatibility routing and legacy source-detail reconstruction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BeamSourceOptions {
    pub max_identifier_bytes: usize,
    pub max_channel_bytes: usize,
    pub max_stream_id_bytes: usize,
}

impl Default for BeamSourceOptions {
    fn default() -> Self {
        Self {
            max_identifier_bytes: 8 * 1024,
            max_channel_bytes: 128,
            max_stream_id_bytes: 1024,
        }
    }
}

impl BeamSourceOptions {
    fn validate(self) -> Result<Self, BeamError> {
        if self.max_identifier_bytes == 0
            || self.max_identifier_bytes > MAX_CONFIGURED_IDENTIFIER_BYTES
            || self.max_channel_bytes == 0
            || self.max_channel_bytes > MAX_CONFIGURED_CHANNEL_BYTES
            || self.max_stream_id_bytes == 0
            || self.max_stream_id_bytes > MAX_CONFIGURED_STREAM_ID_BYTES
        {
            return Err(BeamError::new(BeamErrorKind::InvalidOptions));
        }
        Ok(self)
    }
}

/// Recognizes the historical HTTPS Beam and Mixer channel URL shapes without network access.
#[must_use]
pub fn route_beam_identifier(identifier: &str, options: &BeamSourceOptions) -> Option<BeamRoute> {
    if identifier.is_empty()
        || identifier.len() > options.max_identifier_bytes
        || identifier.contains('#')
    {
        return None;
    }
    let uri: Uri = identifier.parse().ok()?;
    if uri.scheme_str() != Some("https") || uri.query().is_some() {
        return None;
    }
    let authority = uri.authority()?;
    if authority.as_str().contains('@') || authority.as_str() != authority.host() {
        return None;
    }
    if !matches!(
        authority.host().to_ascii_lowercase().as_str(),
        "beam.pro" | "www.beam.pro" | "mixer.com" | "www.mixer.com"
    ) {
        return None;
    }
    let channel = uri.path().strip_prefix('/')?;
    if !valid_channel(channel, options.max_channel_bytes) {
        return None;
    }
    Some(BeamRoute {
        channel: channel.to_owned(),
        original_url: identifier.to_owned(),
    })
}

fn valid_channel(channel: &str, limit: usize) -> bool {
    !channel.is_empty()
        && channel.len() <= limit
        && channel
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_stream_id(stream_id: &str, limit: usize) -> bool {
    !stream_id.is_empty()
        && stream_id.len() <= limit
        && stream_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

/// A track reconstructed from Lavaplayer's historical empty Beam source details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeamSourceTrack {
    pub info: TrackInfo,
    pub stream_id: String,
    pub channel: String,
    pub original_url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamErrorKind {
    InvalidOptions,
    UnsupportedRoute,
    InvalidSourceDetails,
    Cancelled,
    Shutdown,
    ServiceClosed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BeamError {
    kind: BeamErrorKind,
}

impl BeamError {
    const fn new(kind: BeamErrorKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub const fn kind(self) -> BeamErrorKind {
        self.kind
    }
}

impl fmt::Display for BeamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            BeamErrorKind::InvalidOptions => "invalid Beam compatibility policy",
            BeamErrorKind::UnsupportedRoute => "Beam route is not implemented",
            BeamErrorKind::InvalidSourceDetails => "invalid legacy Beam source details",
            BeamErrorKind::Cancelled => "Beam compatibility load cancelled",
            BeamErrorKind::Shutdown => "Beam compatibility source is shut down",
            BeamErrorKind::ServiceClosed => "Beam/Mixer service is closed",
        })
    }
}

impl std::error::Error for BeamError {}

/// Compatibility-only Beam manager. It contains no network client by design.
pub struct BeamSourceManager {
    options: BeamSourceOptions,
    shutdown: AtomicBool,
}

impl BeamSourceManager {
    /// Creates a bounded compatibility manager.
    ///
    /// # Errors
    ///
    /// Returns [`BeamErrorKind::InvalidOptions`] for zero or excessive reconstruction limits.
    pub fn new(options: BeamSourceOptions) -> Result<Self, BeamError> {
        Ok(Self {
            options: options.validate()?,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Reports the deterministic outcome for a historical recognized channel route.
    ///
    /// # Errors
    ///
    /// Always returns [`BeamErrorKind::ServiceClosed`] for a valid route, or a lifecycle/route
    /// error before that. No network operation is possible from this manager.
    pub fn load_route(
        &self,
        route: &BeamRoute,
        cancellation: &SourceCancellation,
    ) -> Result<BeamSourceTrack, BeamError> {
        self.ensure_active(cancellation)?;
        if route_beam_identifier(&route.original_url, &self.options).as_ref() != Some(route) {
            return Err(BeamError::new(BeamErrorKind::UnsupportedRoute));
        }
        Err(BeamError::new(BeamErrorKind::ServiceClosed))
    }

    /// Reports the deterministic playback outcome for a reconstructed legacy track.
    ///
    /// # Errors
    ///
    /// Always returns [`BeamErrorKind::ServiceClosed`] for valid legacy track state. No network
    /// operation is possible from this manager.
    pub fn open_track_playback(
        &self,
        track: &BeamSourceTrack,
        cancellation: &SourceCancellation,
    ) -> Result<(), BeamError> {
        self.ensure_active(cancellation)?;
        self.validate_track(track)?;
        Err(BeamError::new(BeamErrorKind::ServiceClosed))
    }

    fn ensure_active(&self, cancellation: &SourceCancellation) -> Result<(), BeamError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(BeamError::new(BeamErrorKind::Shutdown));
        }
        if cancellation.is_cancelled() {
            return Err(BeamError::new(BeamErrorKind::Cancelled));
        }
        Ok(())
    }

    fn validate_track(&self, track: &BeamSourceTrack) -> Result<(), BeamError> {
        let parsed = parse_legacy_identifier(&track.info.identifier, &self.options)
            .ok_or_else(|| BeamError::new(BeamErrorKind::InvalidSourceDetails))?;
        if !track.info.is_stream
            || parsed.stream_id != track.stream_id
            || parsed.channel != track.channel
            || parsed.original_url != track.original_url
        {
            return Err(BeamError::new(BeamErrorKind::InvalidSourceDetails));
        }
        Ok(())
    }
}

impl Default for BeamSourceManager {
    fn default() -> Self {
        Self::new(BeamSourceOptions::default()).expect("default Beam policy must be valid")
    }
}

impl fmt::Debug for BeamSourceManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BeamSourceManager")
            .field("options", &self.options)
            .field("network_enabled", &false)
            .field("shutdown", &self.shutdown.load(Ordering::Acquire))
            .finish()
    }
}

impl SourceManager<BeamSourceTrack> for BeamSourceManager {
    fn source_name(&self) -> &'static str {
        "beam.pro"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BeamSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<BeamSourceTrack>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let Some(route) = route_beam_identifier(identifier, &self.options) else {
            return Ok(None);
        };
        match self.load_route(&route, cancellation) {
            Err(error) if error.kind() == BeamErrorKind::ServiceClosed => Ok(Some(
                SourceLoad::Referral(SourceReference::new(None, false)),
            )),
            Err(error) if error.kind() == BeamErrorKind::Cancelled => Ok(None),
            Err(_) | Ok(_) => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn is_encodable(&self, item: &BeamSourceTrack) -> bool {
        self.validate_track(item).is_ok()
    }

    fn encode(&self, item: &BeamSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        self.is_encodable(item)
            .then(Vec::new)
            .ok_or(SourceRegistryError::NotEncodable)
    }

    fn decode(&self, _payload: &[u8]) -> Result<BeamSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<BeamSourceTrack, SourceRegistryError> {
        if !payload.is_empty() || !info.is_stream {
            return Err(SourceRegistryError::SourceFailure);
        }
        let parsed = parse_legacy_identifier(&info.identifier, &self.options)
            .ok_or(SourceRegistryError::SourceFailure)?;
        Ok(BeamSourceTrack {
            info: info.clone(),
            stream_id: parsed.stream_id,
            channel: parsed.channel,
            original_url: parsed.original_url,
        })
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

struct BeamLegacyIdentifier {
    stream_id: String,
    channel: String,
    original_url: String,
}

fn parse_legacy_identifier(
    identifier: &str,
    options: &BeamSourceOptions,
) -> Option<BeamLegacyIdentifier> {
    if identifier.is_empty() || identifier.len() > options.max_identifier_bytes {
        return None;
    }
    let mut parts = identifier.splitn(3, '|');
    let stream_id = parts.next()?;
    let channel = parts.next()?;
    let original_url = parts.next()?;
    if !valid_stream_id(stream_id, options.max_stream_id_bytes)
        || !valid_channel(channel, options.max_channel_bytes)
    {
        return None;
    }
    let route = route_beam_identifier(original_url, options)?;
    if route.channel != channel {
        return None;
    }
    Some(BeamLegacyIdentifier {
        stream_id: stream_id.to_owned(),
        channel: channel.to_owned(),
        original_url: original_url.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_url_uses_the_legacy_source_host() {
        let route = BeamRoute {
            channel: "Fixture".to_owned(),
            original_url: "https://mixer.com/Fixture".to_owned(),
        };
        assert_eq!(route.canonical_url(), "https://beam.pro/Fixture");
    }
}
