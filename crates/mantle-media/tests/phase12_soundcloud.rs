#[allow(dead_code)]
#[path = "support/http_replay.rs"]
mod http_replay;

use http_replay::{ReplayResponse, ReplayServer};
use mantle_core::{SourceLoad, SourceManager, SourceReference, SourceRegistryError};
use mantle_media::{
    HttpNetworkAccess, MediaCancellation, RemoteHttpOptions, SoundCloudAccess,
    SoundCloudAuthentication, SoundCloudErrorKind, SoundCloudRoute, SoundCloudSourceItem,
    SoundCloudSourceManager, SoundCloudSourceOptions, route_soundcloud_identifier,
};
use serde_json::json;

fn options(server: &ReplayServer) -> SoundCloudSourceOptions {
    SoundCloudSourceOptions {
        api_base_url: server.url("api"),
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            ..RemoteHttpOptions::default()
        },
        ..SoundCloudSourceOptions::default()
    }
}

fn manager(server: &ReplayServer) -> SoundCloudSourceManager {
    SoundCloudSourceManager::new(
        options(server),
        SoundCloudAuthentication::with_oauth("client-id", Some("oauth-secret")).unwrap(),
    )
    .unwrap()
}

fn track_json(id: u64, access: &str) -> serde_json::Value {
    json!({
        "kind": "track",
        "id": id,
        "title": "Animals",
        "duration": 244_321,
        "access": access,
        "isrc": "SCFIXTURE001",
        "user": {"username": "Architects"},
        "permalink_url": "https://soundcloud.com/architects/animals",
        "artwork_url": "https://i1.sndcdn.com/artworks-fixture-large.jpg",
        "media": {"transcodings": [
            {"url": "https://cf-media.example/stream", "format": {"protocol": "progressive", "mime_type": "audio/mpeg"}},
            {"url": "https://cf-media.example/hls", "format": {"protocol": "hls", "mime_type": "audio/aac"}}
        ]}
    })
}

#[test]
fn routes_current_inputs_and_rejects_unsafe_shapes() {
    let options = SoundCloudSourceOptions::default();
    assert_eq!(
        route_soundcloud_identifier("scsearch: animals architects", &options),
        Some(SoundCloudRoute::Search("animals architects".to_owned()))
    );
    assert_eq!(
        route_soundcloud_identifier("soundcloud.com/architects/animals", &options),
        Some(SoundCloudRoute::Resolve(
            "https://soundcloud.com/architects/animals".to_owned()
        ))
    );
    assert_eq!(
        route_soundcloud_identifier("https://soundcloud.com/architects/sets/live", &options),
        Some(SoundCloudRoute::Resolve(
            "https://soundcloud.com/architects/sets/live".to_owned()
        ))
    );
    for rejected in [
        "https://token@soundcloud.com/architects/animals",
        "https://soundcloud.test/architects/animals",
        "https://soundcloud.com/architects/animals#fragment",
        "scsearch:   ",
        "https://soundcloud.com/architects/animals?x=1#fragment",
    ] {
        assert_eq!(route_soundcloud_identifier(rejected, &options), None);
    }
}

#[test]
fn resolve_replay_builds_bounded_track_and_preserves_auth_policy() {
    let response = serde_json::to_vec(&track_json(12_345, "playable")).unwrap();
    let server = ReplayServer::start(move |request, _| {
        assert!(request.target.starts_with("/api/resolve?"));
        assert!(request.target.contains("client_id=client-id"));
        assert_eq!(request.header("authorization"), Some("OAuth oauth-secret"));
        assert_eq!(request.header("accept"), Some("application/json"));
        ReplayResponse::json(response.clone())
    });
    let source_manager = manager(&server);
    let track = source_manager
        .load_track_metadata(
            "https://soundcloud.com/architects/animals",
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("track result");
    assert_eq!(track.info.identifier, "12345");
    assert_eq!(track.info.title, "Animals");
    assert_eq!(track.info.author, "Architects");
    assert_eq!(
        track.info.duration,
        std::time::Duration::from_millis(244_321)
    );
    assert_eq!(track.access, SoundCloudAccess::Playable);
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://i1.sndcdn.com/artworks-fixture-t500x500.jpg")
    );
    assert_eq!(track.playback.as_ref().unwrap().protocol(), "progressive");
    assert!(
        track
            .playback
            .as_ref()
            .unwrap()
            .as_str()
            .contains("cf-media.example")
    );
    let playback = source_manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .unwrap();
    assert!(playback.as_str().contains("client_id=client-id"));
    assert!(!playback.as_str().contains("oauth-secret"));
    let diagnostic = format!("{source_manager:?}");
    assert!(!diagnostic.contains("oauth-secret"));
    assert!(!diagnostic.contains("client-id"));
}

#[test]
fn search_and_playlist_replays_preserve_order_and_access_states() {
    let playlist = json!({
        "kind": "playlist",
        "title": "Fixture set",
        "permalink_url": "https://soundcloud.com/architects/sets/fixture",
        "user": {"username": "Architects"},
        "tracks": [track_json(1, "playable"), track_json(2, "blocked")]
    });
    let search = json!({"collection": [track_json(3, "preview"), track_json(4, "playable")]});
    let playlist_body = serde_json::to_vec(&playlist).unwrap();
    let search_body = serde_json::to_vec(&search).unwrap();
    let server = ReplayServer::start(move |request, _| {
        if request.target.starts_with("/api/search/tracks?") {
            ReplayResponse::json(search_body.clone())
        } else {
            ReplayResponse::json(playlist_body.clone())
        }
    });
    let source_manager = manager(&server);
    let playlist = source_manager
        .load_route(
            &SoundCloudRoute::Resolve("https://soundcloud.com/architects/sets/fixture".to_owned()),
            &MediaCancellation::new(),
        )
        .unwrap()
        .unwrap();
    let SoundCloudSourceItem::Playlist(playlist) = playlist else {
        panic!("expected playlist");
    };
    assert_eq!(playlist.name, "Fixture set");
    assert_eq!(playlist.tracks.len(), 1);
    assert_eq!(playlist.tracks[0].info.identifier, "1");
    let search = source_manager
        .load_route(
            &SoundCloudRoute::Search("animals".to_owned()),
            &MediaCancellation::new(),
        )
        .unwrap()
        .unwrap();
    let SoundCloudSourceItem::Playlist(search) = search else {
        panic!("expected search playlist");
    };
    assert!(search.is_search_result);
    assert_eq!(search.tracks.len(), 2);
    assert_eq!(search.tracks[0].access, SoundCloudAccess::Preview);
    assert_eq!(search.tracks[1].info.identifier, "4");
}

#[test]
fn bounds_errors_cancellation_and_empty_source_details_fail_closed() {
    let server = ReplayServer::start(|_, _| ReplayResponse::json(b"{}".to_vec()));
    assert_eq!(
        SoundCloudAuthentication::new("").unwrap_err().kind(),
        SoundCloudErrorKind::InvalidAuthentication
    );
    let mut bounded = options(&server);
    bounded.max_transcodings = 0;
    assert_eq!(
        SoundCloudSourceManager::new(bounded, SoundCloudAuthentication::new("client").unwrap())
            .unwrap_err()
            .kind(),
        SoundCloudErrorKind::InvalidOptions
    );
    let source_manager = manager(&server);
    let cancelled = MediaCancellation::new();
    cancelled.cancel();
    assert_eq!(
        source_manager
            .load_track_metadata("https://soundcloud.com/architects/animals", &cancelled)
            .unwrap_err()
            .kind(),
        SoundCloudErrorKind::Cancelled
    );
    let malformed =
        ReplayServer::start(|_, _| ReplayResponse::json(br#"{"kind":"track"}"#.to_vec()));
    let malformed_manager = manager(&malformed);
    assert_eq!(
        malformed_manager
            .load_track_metadata(
                "https://soundcloud.com/architects/animals",
                &MediaCancellation::new()
            )
            .unwrap_err()
            .kind(),
        SoundCloudErrorKind::InvalidResponse
    );
    let source = source_manager;
    let item = SoundCloudSourceItem::Track(mantle_media::SoundCloudSourceTrack {
        info: mantle_core::TrackInfo {
            title: "Fixture".into(),
            author: "Artist".into(),
            duration: std::time::Duration::from_secs(1),
            identifier: "123".into(),
            is_stream: false,
            uri: None,
            artwork_url: None,
            isrc: None,
        },
        access: SoundCloudAccess::Playable,
        playback: None,
    });
    assert!(source.is_encodable(&item));
    assert!(source.encode(&item).unwrap().is_empty());
    assert!(matches!(
        source.decode_with_info(&item_info(&item), &[]),
        Ok(SoundCloudSourceItem::Track(_))
    ));
}

fn item_info(item: &SoundCloudSourceItem) -> mantle_core::TrackInfo {
    let SoundCloudSourceItem::Track(track) = item else {
        unreachable!()
    };
    track.info.clone()
}

#[test]
fn source_manager_maps_route_and_shutdown_without_leaking_details() {
    let server = ReplayServer::start(|_, _| {
        ReplayResponse::json(serde_json::to_vec(&track_json(99, "playable")).unwrap())
    });
    let manager = manager(&server);
    let loaded = manager
        .load(&SourceReference::new(
            Some("https://soundcloud.com/a/b".into()),
            false,
        ))
        .unwrap()
        .unwrap();
    assert!(matches!(
        loaded,
        SourceLoad::Item(SoundCloudSourceItem::Track(_))
    ));
    manager.shutdown();
    assert!(matches!(
        manager.load(&SourceReference::new(
            Some("https://soundcloud.com/a/b".into()),
            false
        )),
        Err(SourceRegistryError::Shutdown)
    ));
}
