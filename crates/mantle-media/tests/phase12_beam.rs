use std::time::Duration;

use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistry,
    SourceRegistryError, SourceRegistryLimits, TrackInfo,
};
use mantle_media::{
    BeamErrorKind, BeamRoute, BeamSourceManager, BeamSourceOptions, BeamSourceTrack,
    route_beam_identifier,
};

const BEAM_URL: &str = "https://beam.pro/Fixture_Channel";
const MIXER_URL: &str = "https://mixer.com/Fixture_Channel";
const LEGACY_IDENTIFIER: &str = "424242|Fixture_Channel|https://mixer.com/Fixture_Channel";

#[test]
fn routes_only_the_historical_https_beam_and_mixer_shapes() {
    let options = BeamSourceOptions::default();
    for identifier in [
        BEAM_URL,
        "https://www.beam.pro/Fixture_Channel",
        MIXER_URL,
        "https://www.mixer.com/Fixture_Channel",
    ] {
        let route = route_beam_identifier(identifier, &options).unwrap();
        assert_eq!(route.channel, "Fixture_Channel");
        assert_eq!(route.original_url, identifier);
        assert_eq!(route.canonical_url(), BEAM_URL);
    }
    for rejected in [
        "Fixture_Channel",
        "http://beam.pro/Fixture_Channel",
        "https://token@beam.pro/Fixture_Channel",
        "https://beam.pro:443/Fixture_Channel",
        "https://beam.test/Fixture_Channel",
        "https://beam.pro.evil.test/Fixture_Channel",
        "https://beam.pro/Fixture_Channel/",
        "https://beam.pro/Fixture_Channel/extra",
        "https://beam.pro/Fixture.Channel",
        "https://beam.pro/Fixture_Channel?referrer=test",
        "https://beam.pro/Fixture_Channel#fragment",
    ] {
        assert_eq!(route_beam_identifier(rejected, &options), None);
    }
}

#[test]
fn recognized_routes_return_terminal_no_track_without_falling_through() {
    let manager = BeamSourceManager::default();
    let route = route_beam_identifier(MIXER_URL, &BeamSourceOptions::default()).unwrap();
    assert_eq!(
        manager
            .load_route(&route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        BeamErrorKind::ServiceClosed
    );
    assert_eq!(manager.source_name(), "beam.pro");
    assert!(format!("{manager:?}").contains("network_enabled: false"));

    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry.register(Box::new(manager)).unwrap();
    registry.register(Box::new(PanicFallback)).unwrap();
    assert_eq!(
        registry
            .load(&SourceReference::new(Some(MIXER_URL.to_owned()), false))
            .unwrap(),
        None
    );
}

#[test]
fn legacy_empty_details_reconstruct_the_composite_identifier_exactly() {
    let manager = BeamSourceManager::default();
    let info = legacy_info();
    let track = manager.decode_with_info(&info, &[]).unwrap();
    assert_eq!(track.info, info);
    assert_eq!(track.stream_id, "424242");
    assert_eq!(track.channel, "Fixture_Channel");
    assert_eq!(track.original_url, MIXER_URL);
    assert!(manager.is_encodable(&track));
    assert!(manager.encode(&track).unwrap().is_empty());
    assert_eq!(
        manager
            .open_track_playback(&track, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        BeamErrorKind::ServiceClosed
    );
    assert!(manager.decode(&[]).is_err());
    assert!(manager.decode_with_info(&info, &[1]).is_err());

    let mut not_stream = info.clone();
    not_stream.is_stream = false;
    assert!(manager.decode_with_info(&not_stream, &[]).is_err());
    let mut mismatched = info;
    mismatched.identifier = "424242|Other_Channel|https://mixer.com/Fixture_Channel".to_owned();
    assert!(manager.decode_with_info(&mismatched, &[]).is_err());
}

#[test]
fn cancellation_invalid_routes_bounds_and_shutdown_are_explicit() {
    let manager = BeamSourceManager::default();
    let route = route_beam_identifier(BEAM_URL, &BeamSourceOptions::default()).unwrap();
    let cancelled = SourceCancellation::new();
    cancelled.cancel();
    assert_eq!(
        manager.load_route(&route, &cancelled).unwrap_err().kind(),
        BeamErrorKind::Cancelled
    );
    assert_eq!(
        manager
            .load_with_cancellation(
                &SourceReference::new(Some(BEAM_URL.to_owned()), false),
                &cancelled,
            )
            .unwrap(),
        None
    );

    let invalid_route = BeamRoute {
        channel: "Other_Channel".to_owned(),
        original_url: BEAM_URL.to_owned(),
    };
    assert_eq!(
        manager
            .load_route(&invalid_route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        BeamErrorKind::UnsupportedRoute
    );

    for invalid in [
        BeamSourceOptions {
            max_identifier_bytes: 0,
            ..BeamSourceOptions::default()
        },
        BeamSourceOptions {
            max_channel_bytes: 0,
            ..BeamSourceOptions::default()
        },
        BeamSourceOptions {
            max_stream_id_bytes: 0,
            ..BeamSourceOptions::default()
        },
    ] {
        assert_eq!(
            BeamSourceManager::new(invalid).unwrap_err().kind(),
            BeamErrorKind::InvalidOptions
        );
    }

    manager.shutdown();
    assert_eq!(
        manager
            .load_route(&route, &SourceCancellation::new())
            .unwrap_err()
            .kind(),
        BeamErrorKind::Shutdown
    );
    assert!(matches!(
        manager.load(&SourceReference::new(Some(BEAM_URL.to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    ));
}

fn legacy_info() -> TrackInfo {
    TrackInfo {
        title: "Fixture stream".to_owned(),
        author: "Fixture_Channel".to_owned(),
        duration: Duration::from_secs(123),
        identifier: LEGACY_IDENTIFIER.to_owned(),
        is_stream: true,
        uri: Some(BEAM_URL.to_owned()),
        artwork_url: Some("https://example.invalid/legacy-thumbnail.jpg".to_owned()),
        isrc: None,
    }
}

struct PanicFallback;

impl SourceManager<BeamSourceTrack> for PanicFallback {
    fn source_name(&self) -> &'static str {
        "panic-fallback"
    }

    fn load(
        &self,
        _reference: &SourceReference,
    ) -> Result<Option<SourceLoad<BeamSourceTrack>>, SourceRegistryError> {
        panic!("recognized closed-service route must terminate source selection")
    }

    fn encode(&self, _item: &BeamSourceTrack) -> Result<Vec<u8>, SourceRegistryError> {
        unreachable!()
    }

    fn decode(&self, _payload: &[u8]) -> Result<BeamSourceTrack, SourceRegistryError> {
        unreachable!()
    }

    fn shutdown(&self) {}
}
