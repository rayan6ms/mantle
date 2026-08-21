#[allow(dead_code)]
#[path = "support/http_replay.rs"]
mod http_replay;
#[path = "support/range_media.rs"]
mod range_media;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use http_replay::{ReplayResponse, ReplayServer};
use mantle_core::{SourceLoad, SourceManager, SourceReference, SourceRegistryError};
use mantle_media::{
    BandcampErrorKind, BandcampPlaybackScheme, BandcampRoute, BandcampSourceItem,
    BandcampSourceManager, BandcampSourceOptions, Codec, Container, HttpNetworkAccess,
    HttpRangeOptions, MediaCancellation, MediaLimits, PcmFrame, RemoteHttpOptions,
    route_bandcamp_identifier,
};
use range_media::RangeMediaServer;
use serde_json::{Value, json};

const TRACK_URL: &str = "https://fixture-artist.bandcamp.com/track/fixture-song";
const ALBUM_URL: &str = "https://fixture-artist.bandcamp.com/album/fixture-album";

#[test]
fn routes_current_track_and_album_pages_strictly() {
    let options = BandcampSourceOptions::default();
    assert_eq!(
        route_bandcamp_identifier(
            "http://fixture-artist.bandcamp.com/track/fixture-song?from=share",
            &options,
        ),
        Some(BandcampRoute::Track(TRACK_URL.to_owned()))
    );
    assert_eq!(
        route_bandcamp_identifier("fixture-artist.bandcamp.com/album/fixture-album", &options),
        Some(BandcampRoute::Album(ALBUM_URL.to_owned()))
    );
    for rejected in [
        "bcsearch:fixture",
        "https://token@fixture-artist.bandcamp.com/track/fixture-song",
        "https://fixture-artist.bandcamp.com:443/track/fixture-song",
        "https://fixture-artist.bandcamp.test/track/fixture-song",
        "https://bandcamp.com.evil.test/track/fixture-song",
        "https://fixture-artist.bandcamp.com/track/fixture/song",
        "https://fixture-artist.bandcamp.com/track/bad%2fslug",
        "https://fixture-artist.bandcamp.com/track/fixture-song#fragment",
    ] {
        assert_eq!(route_bandcamp_identifier(rejected, &options), None);
    }
}

#[test]
fn track_replay_parses_current_attribute_and_refreshes_signed_playback() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/pages/track/fixture-song");
        assert_eq!(
            request.header("accept"),
            Some("text/html,application/xhtml+xml")
        );
        assert_eq!(request.header("user-agent"), Some("Mantle-Bandcamp/1"));
        let token = if count == 0 {
            "first-secret"
        } else {
            "fresh-secret"
        };
        ReplayResponse::json(track_page(&format!(
            "https://t4.bcbits.com/stream/fixture/mp3-128/42?token={token}&expires=1"
        )))
    });
    let manager = manager(&server, false);
    let track = manager
        .load_track_metadata(TRACK_URL, &MediaCancellation::new())
        .unwrap()
        .expect("track page result");
    assert_eq!(track.info.title, "Fixture & Song");
    assert_eq!(track.info.author, "Fixture Artist");
    assert_eq!(track.info.duration, Duration::from_millis(123_456));
    assert_eq!(track.info.identifier, TRACK_URL);
    assert_eq!(track.info.uri.as_deref(), Some(TRACK_URL));
    assert_eq!(track.info.isrc.as_deref(), Some("BCFIXTURE001"));
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://f4.bcbits.com/img/a0000000042_1.png")
    );
    assert!(
        track
            .playback
            .as_ref()
            .unwrap()
            .as_str()
            .contains("first-secret")
    );
    assert!(!format!("{track:?}").contains("first-secret"));

    let fresh = manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .expect("fresh playback URL");
    assert!(fresh.as_str().contains("fresh-secret"));
    assert!(!format!("{fresh:?}").contains("fresh-secret"));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn album_replay_preserves_service_order_and_page_metadata() {
    let server = ReplayServer::start(|request, _| {
        assert_eq!(request.target, "/pages/album/fixture-album");
        ReplayResponse::json(album_page())
    });
    let manager = manager(&server, false);
    let item = manager
        .load_route(
            &BandcampRoute::Album(ALBUM_URL.to_owned()),
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("album page result");
    let BandcampSourceItem::Playlist(album) = item else {
        panic!("expected album playlist");
    };
    assert_eq!(album.name, "Fixture Album");
    assert_eq!(album.author.as_deref(), Some("Fixture Artist"));
    assert_eq!(album.uri.as_deref(), Some(ALBUM_URL));
    assert!(!album.is_search_result);
    assert_eq!(album.selected_track, None);
    assert_eq!(album.tracks.len(), 2);
    assert_eq!(album.tracks[0].info.title, "Opening Track");
    assert_eq!(album.tracks[1].info.title, "Closing Track");
    assert_eq!(
        album.tracks[1].info.identifier,
        "https://fixture-artist.bandcamp.com/track/closing-track"
    );
    assert_eq!(album.tracks[1].info.author, "Guest Artist");
    assert!(album.tracks[1].playback.is_none());
    assert!(album.tracks.iter().all(|track| track.info.isrc.is_none()));
}

#[test]
fn parser_bounds_malformed_pages_and_terminal_states_fail_closed() {
    let malformed = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            b"<html><div data-tralbum=\"{&quot;item_type&quot;:&quot;track&quot;}\"></div></html>"
                .to_vec(),
        )
    });
    assert_eq!(
        manager(&malformed, false)
            .load_track_metadata(TRACK_URL, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        BandcampErrorKind::InvalidResponse
    );

    let album = ReplayServer::start(|_, _| ReplayResponse::json(album_page()));
    let mut bounded = options(&album, false);
    bounded.max_collection_tracks = 1;
    let bounded = BandcampSourceManager::new(bounded).unwrap();
    assert_eq!(
        bounded
            .load_route(
                &BandcampRoute::Album(ALBUM_URL.to_owned()),
                &MediaCancellation::new()
            )
            .unwrap_err()
            .kind(),
        BandcampErrorKind::InvalidResponse
    );

    let duplicate = ReplayServer::start(|_, _| {
        let page = track_page("https://t4.bcbits.com/a.mp3");
        let mut duplicated = page.clone();
        duplicated.extend_from_slice(&page);
        ReplayResponse::json(duplicated)
    });
    assert_eq!(
        manager(&duplicate, false)
            .load_track_metadata(TRACK_URL, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        BandcampErrorKind::InvalidResponse
    );

    let missing = ReplayServer::start(|_, _| ReplayResponse::status(404));
    assert!(
        manager(&missing, false)
            .load_track_metadata(TRACK_URL, &MediaCancellation::new())
            .unwrap()
            .is_none()
    );

    let cancelled = ReplayServer::start(|_, _| ReplayResponse::json(track_page("https://x")));
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert_eq!(
        manager(&cancelled, false)
            .load_track_metadata(TRACK_URL, &cancellation)
            .unwrap_err()
            .kind(),
        BandcampErrorKind::Cancelled
    );
    assert!(cancelled.requests().is_empty());

    let mut invalid = options(&missing, false);
    invalid.max_embedded_json_bytes = 0;
    assert_eq!(
        BandcampSourceManager::new(invalid).unwrap_err().kind(),
        BandcampErrorKind::InvalidOptions
    );
}

#[test]
fn source_details_reconstruct_tracks_and_shutdown_is_terminal() {
    let server = ReplayServer::start(|_, _| {
        ReplayResponse::json(track_page("https://t4.bcbits.com/fixture.mp3"))
    });
    let manager = manager(&server, false);
    let loaded = manager
        .load(&SourceReference::new(Some(TRACK_URL.to_owned()), false))
        .unwrap()
        .expect("source item");
    let SourceLoad::Item(item) = loaded else {
        panic!("expected immediate source item");
    };
    let BandcampSourceItem::Track(track) = &item else {
        panic!("expected track");
    };
    assert_eq!(manager.source_name(), "bandcamp");
    assert!(manager.is_encodable(&item));
    assert!(manager.encode(&item).unwrap().is_empty());
    let decoded = manager.decode_with_info(&track.info, &[]).unwrap();
    let BandcampSourceItem::Track(decoded) = decoded else {
        panic!("expected decoded track");
    };
    assert!(decoded.playback.is_none());
    assert!(manager.decode_with_info(&track.info, &[1]).is_err());
    manager.shutdown();
    assert!(matches!(
        manager.load(&SourceReference::new(Some(TRACK_URL.to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    ));
}

#[test]
fn freshly_resolved_mp3_uses_the_bounded_media_pipeline() {
    let bytes = fs::read(media_fixture("tone-mp3-vbr-id3.mp3")).unwrap();
    let media = RangeMediaServer::start(bytes);
    let media_url = format!(
        "http://{}/fixture.mp3?token=replay-secret",
        media.authority()
    );
    let page = ReplayServer::start(move |_, _| ReplayResponse::json(track_page(&media_url)));
    let manager = manager(&page, true);
    let track = manager
        .load_track_metadata(TRACK_URL, &MediaCancellation::new())
        .unwrap()
        .unwrap();
    let mut session = manager
        .open_track_playback(
            &track,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap()
        .expect("playable MP3");
    assert_eq!(session.info().container, Container::Mp3);
    assert_eq!(session.info().codec, Codec::Mp3);
    let mut pcm = PcmFrame::with_capacity(256 * 1024);
    assert!(session.read_pcm(&mut pcm).unwrap());
    assert!(!pcm.samples().is_empty());
    assert!(!media.requests().is_empty());
    assert_eq!(page.requests().len(), 2);
}

#[test]
#[ignore = "scheduled live-service smoke; not a normal PR gate"]
fn scheduled_live_track_smoke() {
    let url = std::env::var("MANTLE_BANDCAMP_SMOKE_URL")
        .unwrap_or_else(|_| "https://kinggizzard.bandcamp.com/track/motor-spirit".to_owned());
    let manager = BandcampSourceManager::new(BandcampSourceOptions::default()).unwrap();
    let track = manager
        .load_track_metadata(&url, &MediaCancellation::new())
        .unwrap()
        .expect("live public track");
    assert!(!track.info.title.is_empty());
    let playback = manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .expect("live public MP3");
    assert!(playback.as_str().starts_with("https://"));
}

fn options(server: &ReplayServer, allow_http_playback: bool) -> BandcampSourceOptions {
    BandcampSourceOptions {
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            max_retries: 0,
            ..RemoteHttpOptions::default()
        },
        page_origin_override: Some(server.url("pages")),
        playback_scheme: if allow_http_playback {
            BandcampPlaybackScheme::HttpForPrivateNetworks
        } else {
            BandcampPlaybackScheme::Https
        },
        ..BandcampSourceOptions::default()
    }
}

fn manager(server: &ReplayServer, allow_http_playback: bool) -> BandcampSourceManager {
    BandcampSourceManager::new(options(server, allow_http_playback)).unwrap()
}

fn private_range_options() -> HttpRangeOptions {
    HttpRangeOptions {
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        max_retries: 0,
        ..HttpRangeOptions::default()
    }
}

fn track_page(playback_url: &str) -> Vec<u8> {
    fixture_page(&json!({
        "current": {
            "title": "Fixture & Song",
            "type": "track",
            "isrc": "BCFIXTURE001"
        },
        "item_type": "track",
        "artist": "Fixture Artist",
        "art_id": 42,
        "url": TRACK_URL,
        "trackinfo": [{
            "id": 42,
            "track_id": 42,
            "title": "Fixture & Song",
            "artist": null,
            "duration": 123.456,
            "title_link": "/track/fixture-song",
            "file": {"mp3-128": playback_url}
        }]
    }))
}

fn album_page() -> Vec<u8> {
    fixture_page(&json!({
        "current": {"title": "Fixture Album", "type": "album"},
        "item_type": "album",
        "artist": "Fixture Artist",
        "art_id": "1234567890",
        "url": ALBUM_URL,
        "trackinfo": [
            {
                "id": 1,
                "title": "Opening Track",
                "artist": null,
                "duration": 60.25,
                "title_link": "/track/opening-track",
                "file": {"mp3-128": "https://t4.bcbits.com/opening.mp3?token=secret"}
            },
            {
                "id": 2,
                "title": "Closing Track",
                "artist": "Guest Artist",
                "duration": 75.0,
                "title_link": "/track/closing-track",
                "file": null
            }
        ]
    }))
}

fn fixture_page(value: &Value) -> Vec<u8> {
    let encoded = serde_json::to_string(&value)
        .unwrap()
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");
    format!("<html><body><div id=\"pagedata\" data-tralbum=\"{encoded}\"></div></body></html>")
        .into_bytes()
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}
