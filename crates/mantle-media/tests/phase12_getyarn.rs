use std::time::Duration;

use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistry,
    SourceRegistryError, SourceRegistryLimits, TrackInfo,
};
use mantle_media::{
    GetyarnErrorKind, GetyarnRoute, GetyarnSourceManager, GetyarnSourceOptions, GetyarnSourceTrack,
    route_getyarn_identifier,
};

const CLIP_ID: &str = "bcd18bd3-15cc-4710-b9fa-30c50e5f4330";
const CLIP_URL: &str = "https://getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330";
const MEDIA_URL: &str = "https://cdn.example.invalid/fixture.mp4?token=legacy-media-secret";

#[test]
fn routes_only_bounded_historical_getyarn_clip_pages() {
    let options = GetyarnSourceOptions::default();
    for identifier in [
        CLIP_URL,
        "http://getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://www.getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "http://www.getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
    ] {
        let route = route_getyarn_identifier(identifier, &options).unwrap();
        assert_eq!(route.clip_id, CLIP_ID);
        assert_eq!(route.original_url, identifier);
        assert_eq!(route.canonical_url(), CLIP_URL);
    }
    for rejected in [
        CLIP_ID,
        "https://token@getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://getyarn.io:443/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://getyarn.test/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://getyarn.io.evil.test/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://tv.getyarn.io/yarn-clip/bcd18bd3-15cc-4710-b9fa-30c50e5f4330",
        "https://getyarn.io/yarn-clip/",
        "https://getyarn.io/yarn-clip/fixture.id",
        "https://getyarn.io/yarn-clip/fixture/extra",
        "https://getyarn.io/yarn-clip/fixture?referrer=test",
        "https://getyarn.io/yarn-clip/fixture#fragment",
    ] {
        assert_eq!(route_getyarn_identifier(rejected, &options), None);
    }
}

#[test]
fn recognized_pages_return_terminal_no_track_without_falling_through() {
    let manager = GetyarnSourceManager::default();
    let route = route_getyarn_identifier(CLIP_URL, &GetyarnSourceOptions::default()).unwrap();
    assert_eq!(
        manager
            .load_route(&route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        GetyarnErrorKind::UnsupportedPlayback
    );
    assert_eq!(manager.source_name(), "getyarn.io");
    assert!(format!("{manager:?}").contains("network_enabled: false"));

    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry.register(Box::new(manager)).unwrap();
    registry.register(Box::new(PanicFallback)).unwrap();
    assert_eq!(
        registry
            .load(&SourceReference::new(Some(CLIP_URL.to_owned()), false))
            .unwrap(),
        None
    );
}

#[test]
fn legacy_empty_details_retain_page_and_media_roles_without_logging_media_url() {
    let manager = GetyarnSourceManager::default();
    let info = legacy_info();
    let track = manager.decode_with_info(&info, &[]).unwrap();
    assert_eq!(track.info, info);
    assert_eq!(track.clip_id, CLIP_ID);
    assert_eq!(track.page_url, CLIP_URL);
    assert!(manager.is_encodable(&track));
    assert!(manager.encode(&track).unwrap().is_empty());
    assert_eq!(
        manager
            .open_track_playback(&track, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        GetyarnErrorKind::UnsupportedPlayback
    );
    let diagnostic = format!("{track:?}");
    assert!(!diagnostic.contains("legacy-media-secret"));
    assert!(!diagnostic.contains("cdn.example.invalid"));

    assert!(manager.decode(&[]).is_err());
    assert!(manager.decode_with_info(&info, &[1]).is_err());
    let mut streaming = info.clone();
    streaming.is_stream = true;
    assert!(manager.decode_with_info(&streaming, &[]).is_err());
    let mut missing_page = info.clone();
    missing_page.uri = None;
    assert!(manager.decode_with_info(&missing_page, &[]).is_err());
    let mut insecure_media = info;
    insecure_media.identifier = "http://cdn.example.invalid/fixture.mp4".to_owned();
    assert!(manager.decode_with_info(&insecure_media, &[]).is_err());
}

#[test]
fn cancellation_invalid_routes_bounds_and_shutdown_are_explicit() {
    let manager = GetyarnSourceManager::default();
    let route = route_getyarn_identifier(CLIP_URL, &GetyarnSourceOptions::default()).unwrap();
    let cancelled = SourceCancellation::new();
    cancelled.cancel();
    assert_eq!(
        manager.load_route(&route, &cancelled).unwrap_err().kind(),
        GetyarnErrorKind::Cancelled
    );
    assert_eq!(
        manager
            .load_with_cancellation(
                &SourceReference::new(Some(CLIP_URL.to_owned()), false),
                &cancelled,
            )
            .unwrap(),
        None
    );

    let invalid_route = GetyarnRoute {
        clip_id: "other".to_owned(),
        original_url: CLIP_URL.to_owned(),
    };
    assert_eq!(
        manager
            .load_route(&invalid_route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        GetyarnErrorKind::UnsupportedRoute
    );

    for invalid in [
        GetyarnSourceOptions {
            max_identifier_bytes: 0,
            ..GetyarnSourceOptions::default()
        },
        GetyarnSourceOptions {
            max_clip_id_bytes: 0,
            ..GetyarnSourceOptions::default()
        },
        GetyarnSourceOptions {
            max_media_url_bytes: 0,
            ..GetyarnSourceOptions::default()
        },
    ] {
        assert_eq!(
            GetyarnSourceManager::new(invalid).unwrap_err().kind(),
            GetyarnErrorKind::InvalidOptions
        );
    }

    manager.shutdown();
    assert_eq!(
        manager
            .load_route(&route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        GetyarnErrorKind::Shutdown
    );
    assert!(matches!(
        manager.load(&SourceReference::new(Some(CLIP_URL.to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    ));
}

fn legacy_info() -> TrackInfo {
    TrackInfo {
        title: "Fixture quote".to_owned(),
        author: "Unknown".to_owned(),
        duration: Duration::from_secs(4),
        identifier: MEDIA_URL.to_owned(),
        is_stream: false,
        uri: Some(CLIP_URL.to_owned()),
        artwork_url: None,
        isrc: None,
    }
}

struct PanicFallback;

impl SourceManager<GetyarnSourceTrack> for PanicFallback {
    fn source_name(&self) -> &'static str {
        "panic-fallback"
    }

    fn load(
        &self,
        _reference: &SourceReference,
    ) -> Result<Option<SourceLoad<GetyarnSourceTrack>>, SourceRegistryError> {
        panic!("recognized compatibility route must terminate source selection")
    }

    fn encode(&self, _item: &GetyarnSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        unreachable!()
    }

    fn decode(&self, _payload: &[u8]) -> Result<GetyarnSourceTrack, SourceRegistryError> {
        unreachable!()
    }

    fn shutdown(&self) {}
}
