use crate::{
    Container, HttpPlaylistOptions, HttpRangeInput, HttpRangeOptions, MediaCancellation, MediaInfo,
    MediaLimits, MediaSession, PlaylistFormat, load_http_playlist_with_cancellation,
};
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaProbe {
    Wave,
    MatroskaWebM,
    Mp4,
    Flac,
    Ogg,
    Mp3,
    Adts,
    HlsOuter,
}

impl MediaProbe {
    const fn serialized_name(self) -> &'static str {
        match self {
            Self::Wave => "wav",
            Self::MatroskaWebM => "matroska/webm",
            Self::Mp4 => "mp4",
            Self::Flac => "flac",
            Self::Ogg => "ogg",
            Self::Mp3 => "mp3",
            Self::Adts => "adts",
            Self::HlsOuter => "m3u|hls-outer",
        }
    }

    fn from_serialized_name(name: &str) -> Option<Self> {
        match name {
            "wav" => Some(Self::Wave),
            "matroska/webm" => Some(Self::MatroskaWebM),
            "mp4" => Some(Self::Mp4),
            "flac" => Some(Self::Flac),
            "ogg" => Some(Self::Ogg),
            "mp3" => Some(Self::Mp3),
            "adts" => Some(Self::Adts),
            "m3u|hls-outer" => Some(Self::HlsOuter),
            _ => None,
        }
    }
}

impl From<Container> for MediaProbe {
    fn from(container: Container) -> Self {
        match container {
            Container::Wave => Self::Wave,
            Container::WebM | Container::Matroska => Self::MatroskaWebM,
            Container::Mp4 => Self::Mp4,
            Container::Flac => Self::Flac,
            Container::Ogg => Self::Ogg,
            Container::Mp3 => Self::Mp3,
            Container::Adts => Self::Adts,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaSourceTrack {
    pub info: TrackInfo,
    pub probe: MediaProbe,
}

pub struct LocalMediaSourceManager {
    media_limits: MediaLimits,
}

impl LocalMediaSourceManager {
    #[must_use]
    pub const fn new(media_limits: MediaLimits) -> Self {
        Self { media_limits }
    }
}

impl Default for LocalMediaSourceManager {
    fn default() -> Self {
        Self::new(MediaLimits::default())
    }
}

impl SourceManager<MediaSourceTrack> for LocalMediaSourceManager {
    fn source_name(&self) -> &'static str {
        "local"
    }

    fn is_probing(&self) -> bool {
        true
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<MediaSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<MediaSourceTrack>>, SourceRegistryError> {
        let Some(identifier) = reference.identifier() else {
            return Ok(None);
        };
        let path = Path::new(identifier);
        if !path.is_file() {
            return Ok(None);
        }
        let media_cancellation = linked_cancellation(cancellation);
        let session =
            MediaSession::open_file_with_cancellation(path, self.media_limits, media_cancellation)
                .map_err(|_| SourceRegistryError::SourceFailure)?;
        let track = MediaSourceTrack {
            info: make_track_info(identifier, session.info(), None),
            probe: session.info().container.into(),
        };
        Ok(Some(SourceLoad::Item(track)))
    }

    fn encode(&self, item: &MediaSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        encode_probe(item.probe)
    }

    fn decode(&self, _payload: &[u8]) -> Result<MediaSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<MediaSourceTrack, SourceRegistryError> {
        Ok(MediaSourceTrack {
            info: info.clone(),
            probe: decode_probe(payload)?,
        })
    }

    fn shutdown(&self) {}
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HttpMediaSourceOptions {
    pub media: MediaLimits,
    pub range: HttpRangeOptions,
    pub playlist: HttpPlaylistOptions,
}

pub struct HttpMediaSourceManager {
    options: HttpMediaSourceOptions,
}

impl HttpMediaSourceManager {
    #[must_use]
    pub const fn new(options: HttpMediaSourceOptions) -> Self {
        Self { options }
    }
}

impl Default for HttpMediaSourceManager {
    fn default() -> Self {
        Self::new(HttpMediaSourceOptions::default())
    }
}

impl SourceManager<MediaSourceTrack> for HttpMediaSourceManager {
    fn source_name(&self) -> &'static str {
        "http"
    }

    fn is_probing(&self) -> bool {
        true
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<MediaSourceTrack>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<MediaSourceTrack>>, SourceRegistryError> {
        let Some(identifier) = reference.identifier().and_then(normalize_http_identifier) else {
            return Ok(None);
        };
        let media_cancellation = linked_cancellation(cancellation);
        if is_playlist_identifier(&identifier) {
            let matched = load_http_playlist_with_cancellation(
                &identifier,
                self.options.playlist,
                media_cancellation,
            )
            .map_err(|_| SourceRegistryError::SourceFailure)?;
            if let Some(matched) = matched {
                if matched.format == PlaylistFormat::Hls {
                    return Ok(Some(SourceLoad::Item(MediaSourceTrack {
                        info: TrackInfo {
                            title: matched
                                .reference
                                .title
                                .unwrap_or_else(|| identifier.clone()),
                            author: "Unknown artist".to_owned(),
                            duration: Duration::ZERO,
                            identifier: identifier.clone(),
                            is_stream: true,
                            uri: Some(identifier),
                            artwork_url: None,
                            isrc: None,
                        },
                        probe: MediaProbe::HlsOuter,
                    })));
                }
                return Ok(Some(SourceLoad::Referral(SourceReference::new(
                    Some(matched.reference.identifier),
                    false,
                ))));
            }
        }

        let input = HttpRangeInput::open_with_cancellation(
            &identifier,
            self.options.range,
            linked_cancellation(cancellation),
        )
        .map_err(|_| SourceRegistryError::SourceFailure)?;
        let final_identifier = input.final_uri().to_string();
        if final_identifier != identifier {
            return Ok(Some(SourceLoad::Referral(SourceReference::new(
                Some(final_identifier),
                false,
            ))));
        }
        let extension = extension_hint(&identifier);
        let session = MediaSession::open_with_cancellation(
            Box::new(input),
            extension.as_deref(),
            self.options.media,
            linked_cancellation(cancellation),
        )
        .map_err(|_| SourceRegistryError::SourceFailure)?;
        Ok(Some(SourceLoad::Item(MediaSourceTrack {
            info: make_track_info(&identifier, session.info(), None),
            probe: session.info().container.into(),
        })))
    }

    fn encode(&self, item: &MediaSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        encode_probe(item.probe)
    }

    fn decode(&self, _payload: &[u8]) -> Result<MediaSourceTrack, SourceRegistryError> {
        Err(SourceRegistryError::SourceFailure)
    }

    fn decode_with_info(
        &self,
        info: &TrackInfo,
        payload: &[u8],
    ) -> Result<MediaSourceTrack, SourceRegistryError> {
        Ok(MediaSourceTrack {
            info: info.clone(),
            probe: decode_probe(payload)?,
        })
    }

    fn shutdown(&self) {}
}

fn linked_cancellation(cancellation: &SourceCancellation) -> MediaCancellation {
    let cancellation = cancellation.clone();
    MediaCancellation::linked(move || cancellation.is_cancelled())
}

fn normalize_http_identifier(identifier: &str) -> Option<String> {
    if identifier.starts_with("http://") || identifier.starts_with("https://") {
        Some(identifier.to_owned())
    } else {
        identifier
            .strip_prefix("icy://")
            .map(|remainder| format!("http://{remainder}"))
    }
}

fn is_playlist_identifier(identifier: &str) -> bool {
    let path = identifier
        .split(['?', '#'])
        .next()
        .unwrap_or(identifier)
        .to_ascii_lowercase();
    [".m3u", ".m3u8", ".pls"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
}

fn extension_hint(identifier: &str) -> Option<String> {
    let path = identifier.split(['?', '#']).next().unwrap_or(identifier);
    let name = path.rsplit('/').next()?;
    let (_, extension) = name.rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_owned())
}

fn make_track_info(identifier: &str, media: &MediaInfo, title: Option<String>) -> TrackInfo {
    let fallback_title = Path::new(identifier)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(identifier)
        .to_owned();
    TrackInfo {
        title: title
            .or_else(|| media.metadata.title.clone())
            .unwrap_or(fallback_title),
        author: media
            .metadata
            .author
            .clone()
            .unwrap_or_else(|| "Unknown artist".to_owned()),
        duration: media.duration.unwrap_or(Duration::ZERO),
        identifier: identifier.to_owned(),
        is_stream: media.duration.is_none(),
        uri: Some(identifier.to_owned()),
        artwork_url: None,
        isrc: media.metadata.isrc.clone(),
    }
}

fn encode_probe(probe: MediaProbe) -> Result<Vec<u8>, SourceRegistryError> {
    let name = probe.serialized_name().as_bytes();
    let length = u16::try_from(name.len()).map_err(|_| SourceRegistryError::SourceFailure)?;
    let mut payload = Vec::with_capacity(name.len() + 2);
    payload.extend_from_slice(&length.to_be_bytes());
    payload.extend_from_slice(name);
    Ok(payload)
}

fn decode_probe(payload: &[u8]) -> Result<MediaProbe, SourceRegistryError> {
    let [first, second, rest @ ..] = payload else {
        return Err(SourceRegistryError::SourceFailure);
    };
    let length = usize::from(u16::from_be_bytes([*first, *second]));
    let name = rest
        .get(..length)
        .filter(|_| rest.len() == length)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .ok_or(SourceRegistryError::SourceFailure)?;
    MediaProbe::from_serialized_name(name).ok_or(SourceRegistryError::SourceFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_details_match_lavaplayer_ascii_modified_utf_records() {
        assert_eq!(encode_probe(MediaProbe::Wave).unwrap(), b"\0\x03wav");
        assert_eq!(
            encode_probe(MediaProbe::HlsOuter).unwrap(),
            b"\0\rm3u|hls-outer"
        );
        assert_eq!(decode_probe(b"\0\x03wav"), Ok(MediaProbe::Wave));
        assert_eq!(decode_probe(b"\0\rm3u|hls-outer"), Ok(MediaProbe::HlsOuter));
        assert_eq!(
            decode_probe(b"\0\x07unknown"),
            Err(SourceRegistryError::SourceFailure)
        );
    }

    #[test]
    fn schemes_playlist_hints_and_extensions_are_strict_and_deterministic() {
        assert_eq!(
            normalize_http_identifier("icy://example.test/live"),
            Some("http://example.test/live".to_owned())
        );
        assert_eq!(normalize_http_identifier("HTTP://example.test"), None);
        assert!(is_playlist_identifier("https://example.test/list.M3U8?q=1"));
        assert_eq!(
            extension_hint("https://example.test/audio.ogg?q=1"),
            Some("ogg".to_owned())
        );
    }
}
