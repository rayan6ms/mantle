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
    Codec, Container, HttpNetworkAccess, HttpRangeOptions, MediaCancellation, MediaLimits,
    PcmFrame, RemoteHttpOptions, VimeoAuthentication, VimeoErrorKind, VimeoPlaybackErrorKind,
    VimeoPlaybackKind, VimeoPlaybackScheme, VimeoRoute, VimeoSourceManager, VimeoSourceOptions,
    route_vimeo_identifier,
};
use range_media::RangeMediaServer;
use serde_json::{Value, json};

const VIDEO_ID: &str = "76979871";
const VIDEO_URL: &str = "https://vimeo.com/76979871";

#[test]
fn routes_current_public_and_player_urls_strictly() {
    let options = VimeoSourceOptions::default();
    assert_eq!(
        route_vimeo_identifier("http://vimeo.com/76979871?from=share", &options),
        Some(VimeoRoute {
            video_id: VIDEO_ID.to_owned(),
            unlisted_hash: None,
        })
    );
    assert_eq!(
        route_vimeo_identifier("vimeo.com/76979871/abcDEF123", &options),
        Some(VimeoRoute {
            video_id: VIDEO_ID.to_owned(),
            unlisted_hash: Some("abcDEF123".to_owned()),
        })
    );
    assert_eq!(
        route_vimeo_identifier(
            "https://player.vimeo.com/video/76979871?h=abcDEF123",
            &options,
        ),
        Some(VimeoRoute {
            video_id: VIDEO_ID.to_owned(),
            unlisted_hash: Some("abcDEF123".to_owned()),
        })
    );
    for rejected in [
        VIDEO_ID,
        "https://token@vimeo.com/76979871",
        "https://vimeo.com:443/76979871",
        "https://vimeo.test/76979871",
        "https://vimeo.com.evil.test/76979871",
        "https://vimeo.com/channels/staffpicks/76979871",
        "https://player.vimeo.com/76979871",
        "https://vimeo.com/not-a-number",
        "https://vimeo.com/76979871/hash/extra",
        "https://vimeo.com/76979871#fragment",
    ] {
        assert_eq!(route_vimeo_identifier(rejected, &options), None);
    }
}

#[test]
fn public_config_replay_builds_metadata_and_refreshes_lowest_mp4() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/player/76979871/config");
        assert_eq!(request.header("authorization"), None);
        assert_eq!(request.header("accept"), Some("application/json"));
        let token = if count == 0 {
            "first-secret"
        } else {
            "fresh-secret"
        };
        ReplayResponse::json(public_progressive_config(token))
    });
    let manager = public_manager(&server, false);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .expect("public video");
    assert_eq!(track.info.title, "Fixture Video");
    assert_eq!(track.info.author, "Fixture Creator");
    assert_eq!(track.info.duration, Duration::from_secs(62));
    assert_eq!(track.info.identifier, VIDEO_ID);
    assert_eq!(track.info.uri.as_deref(), Some(VIDEO_URL));
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://i.vimeocdn.com/video/fixture_640")
    );
    let initial = track.playback.as_ref().unwrap();
    assert_eq!(initial.kind(), VimeoPlaybackKind::ProgressiveMp4);
    assert_eq!(initial.mime_type(), "video/mp4");
    assert!(initial.as_str().contains("360p"));
    assert!(initial.as_str().contains("first-secret"));
    assert!(!format!("{track:?}").contains("first-secret"));

    let fresh = manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .expect("fresh progressive URL");
    assert!(fresh.as_str().contains("360p"));
    assert!(fresh.as_str().contains("fresh-secret"));
    assert!(!format!("{fresh:?}").contains("fresh-secret"));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn hls_only_public_config_is_observable_but_not_misparsed_as_progressive_mp4() {
    let server = ReplayServer::start(|_, _| ReplayResponse::json(public_hls_config()));
    let manager = public_manager(&server, false);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .unwrap();
    let playback = track.playback.as_ref().unwrap();
    assert_eq!(playback.kind(), VimeoPlaybackKind::Hls);
    assert_eq!(playback.mime_type(), "application/x-mpegURL");
    assert!(!format!("{playback:?}").contains("hls-secret"));

    let error = manager
        .open_track_playback(
            &track,
            HttpRangeOptions::default(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), VimeoPlaybackErrorKind::IncompatibleFormat);
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn authenticated_api_replay_uses_official_headers_and_redacts_the_token() {
    let server = ReplayServer::start(|request, count| {
        assert!(request.target.starts_with("/api/videos/76979871?fields="));
        assert!(request.target.contains("play"));
        assert_eq!(request.header("authorization"), Some("Bearer token-secret"));
        assert_eq!(
            request.header("accept"),
            Some("application/vnd.vimeo.*+json;version=3.4")
        );
        let token = if count == 0 { "initial" } else { "refreshed" };
        ReplayResponse::json(api_video(&format!(
            "https://player.vimeo.com/progressive_redirect/360p.mp4?token={token}"
        )))
    });
    let manager = authenticated_manager(&server, false);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .unwrap();
    assert!(manager.authentication_configured());
    assert_eq!(track.info.title, "Authenticated Fixture");
    assert_eq!(track.info.author, "API Creator");
    assert_eq!(track.info.identifier, VIDEO_ID);
    assert_eq!(
        track.playback.as_ref().unwrap().kind(),
        VimeoPlaybackKind::ProgressiveMp4
    );
    let fresh = manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .unwrap();
    assert!(fresh.as_str().contains("refreshed"));
    let diagnostic = format!("{manager:?} {fresh:?}");
    assert!(!diagnostic.contains("token-secret"));
    assert!(!diagnostic.contains("refreshed"));
}

#[test]
fn authenticated_audio_mp4_uses_the_bounded_media_pipeline() {
    let bytes = fs::read(media_fixture("tone-aac-lc.m4a")).unwrap();
    let media = RangeMediaServer::start(bytes);
    let media_url = format!(
        "http://{}/fixture.m4a?token=media-secret",
        media.authority()
    );
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(api_audio_video(&media_url)));
    let manager = authenticated_manager(&api, true);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .unwrap();
    assert_eq!(track.playback.as_ref().unwrap().mime_type(), "audio/mp4");
    let mut session = manager
        .open_track_playback(
            &track,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap()
        .expect("playable MP4");
    assert_eq!(session.info().container, Container::Mp4);
    assert_eq!(session.info().codec, Codec::AacLc);
    let mut pcm = PcmFrame::with_capacity(256 * 1024);
    assert!(session.read_pcm(&mut pcm).unwrap());
    assert!(!pcm.samples().is_empty());
    assert!(!media.requests().is_empty());
    assert_eq!(api.requests().len(), 2);
}

#[test]
fn bounds_errors_cancellation_source_details_and_shutdown_fail_closed() {
    assert_eq!(
        VimeoAuthentication::new("").unwrap_err().kind(),
        VimeoErrorKind::InvalidAuthentication
    );
    let server = ReplayServer::start(|_, _| ReplayResponse::json(public_progressive_config("x")));
    let mut invalid = options(&server, false);
    invalid.max_playback_candidates = 0;
    assert_eq!(
        VimeoSourceManager::new(invalid).unwrap_err().kind(),
        VimeoErrorKind::InvalidOptions
    );

    let oversized = ReplayServer::start(|_, _| {
        let mut value: Value = serde_json::from_slice(&public_progressive_config("x")).unwrap();
        value["request"]["files"]["progressive"] = json!([
            {"url":"https://player.vimeo.com/a.mp4","mime":"video/mp4","height":360},
            {"url":"https://player.vimeo.com/b.mp4","mime":"video/mp4","height":720}
        ]);
        ReplayResponse::json(serde_json::to_vec(&value).unwrap())
    });
    let mut bounded = options(&oversized, false);
    bounded.max_playback_candidates = 1;
    assert_eq!(
        VimeoSourceManager::new(bounded)
            .unwrap()
            .load_route(&route(), &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        VimeoErrorKind::InvalidResponse
    );

    let mismatched = ReplayServer::start(|_, _| {
        let mut value: Value = serde_json::from_slice(&public_progressive_config("x")).unwrap();
        value["video"]["id"] = json!(1);
        ReplayResponse::json(serde_json::to_vec(&value).unwrap())
    });
    assert_eq!(
        public_manager(&mismatched, false)
            .load_route(&route(), &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        VimeoErrorKind::InvalidResponse
    );

    let missing = ReplayServer::start(|_, _| ReplayResponse::status(404));
    assert!(
        public_manager(&missing, false)
            .load_route(&route(), &MediaCancellation::new())
            .unwrap()
            .is_none()
    );

    let cancelled = MediaCancellation::new();
    cancelled.cancel();
    assert_eq!(
        public_manager(&server, false)
            .load_route(&route(), &cancelled)
            .unwrap_err()
            .kind(),
        VimeoErrorKind::Cancelled
    );

    let manager = public_manager(&server, false);
    let loaded = manager
        .load(&SourceReference::new(Some(VIDEO_URL.to_owned()), false))
        .unwrap()
        .unwrap();
    let SourceLoad::Item(track) = loaded else {
        panic!("expected immediate track");
    };
    assert_eq!(manager.source_name(), "vimeo");
    assert!(manager.is_encodable(&track));
    assert!(manager.encode(&track).unwrap().is_empty());
    let decoded = manager.decode_with_info(&track.info, &[]).unwrap();
    assert!(decoded.playback.is_none());
    assert!(manager.decode_with_info(&track.info, &[1]).is_err());
    manager.shutdown();
    assert!(matches!(
        manager.load(&SourceReference::new(Some(VIDEO_URL.to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    ));
}

#[test]
#[ignore = "scheduled live-service smoke; not a normal PR gate"]
fn scheduled_live_public_config_smoke() {
    let url = std::env::var("MANTLE_VIMEO_SMOKE_URL").unwrap_or_else(|_| VIDEO_URL.to_owned());
    let options = VimeoSourceOptions::default();
    let route = route_vimeo_identifier(&url, &options).expect("live route");
    let manager = VimeoSourceManager::new(options).unwrap();
    let track = manager
        .load_route(&route, &MediaCancellation::new())
        .unwrap()
        .expect("live public video");
    assert!(!track.info.title.is_empty());
    assert!(track.playback.is_some());
}

fn options(server: &ReplayServer, allow_http_playback: bool) -> VimeoSourceOptions {
    VimeoSourceOptions {
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            max_retries: 0,
            ..RemoteHttpOptions::default()
        },
        player_base_url: server.url("player"),
        api_base_url: server.url("api"),
        playback_scheme: if allow_http_playback {
            VimeoPlaybackScheme::HttpForPrivateNetworks
        } else {
            VimeoPlaybackScheme::Https
        },
        ..VimeoSourceOptions::default()
    }
}

fn public_manager(server: &ReplayServer, allow_http_playback: bool) -> VimeoSourceManager {
    VimeoSourceManager::new(options(server, allow_http_playback)).unwrap()
}

fn authenticated_manager(server: &ReplayServer, allow_http_playback: bool) -> VimeoSourceManager {
    VimeoSourceManager::with_authentication(
        options(server, allow_http_playback),
        VimeoAuthentication::new("token-secret").unwrap(),
    )
    .unwrap()
}

fn route() -> VimeoRoute {
    VimeoRoute {
        video_id: VIDEO_ID.to_owned(),
        unlisted_hash: None,
    }
}

fn private_range_options() -> HttpRangeOptions {
    HttpRangeOptions {
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        max_retries: 0,
        ..HttpRangeOptions::default()
    }
}

fn public_progressive_config(token: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "video": {
            "id": 76_979_871,
            "title": "Fixture Video",
            "duration": 62,
            "url": VIDEO_URL,
            "thumbnail_url": "https://i.vimeocdn.com/video/fixture_640",
            "owner": {"name": "Fixture Creator"}
        },
        "request": {"files": {
            "progressive": [
                {
                    "url": format!("https://player.vimeo.com/progressive/1080p.mp4?token={token}"),
                    "mime": "video/mp4",
                    "height": 1080
                },
                {
                    "url": format!("https://player.vimeo.com/progressive/360p.mp4?token={token}"),
                    "mime": "video/mp4",
                    "height": 360
                }
            ]
        }}
    }))
    .unwrap()
}

fn public_hls_config() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "video": {
            "id": 76_979_871,
            "title": "HLS Fixture",
            "duration": 62,
            "thumbnail_url": "https://i.vimeocdn.com/video/hls_640",
            "owner": {"name": "Fixture Creator"}
        },
        "request": {"files": {
            "progressive": [],
            "hls": {
                "default_cdn": "skyfire",
                "cdns": {
                    "skyfire": {
                        "url": "https://skyfire.vimeocdn.com/playlist.m3u8?token=hls-secret"
                    }
                }
            }
        }}
    }))
    .unwrap()
}

fn api_video(playback_url: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "uri": "/videos/76979871",
        "name": "Authenticated Fixture",
        "user": {"name": "API Creator"},
        "duration": 62,
        "pictures": {"base_link": "https://i.vimeocdn.com/video/api_640"},
        "link": VIDEO_URL,
        "play": {"progressive": [
            {"type": "video/mp4", "codec": "H264", "height": 360, "link": playback_url}
        ]}
    }))
    .unwrap()
}

fn api_audio_video(playback_url: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "uri": "/videos/76979871",
        "name": "Audio Fixture",
        "user": {"name": "API Creator"},
        "duration": 1,
        "pictures": {"base_link": "https://i.vimeocdn.com/video/api_640"},
        "link": VIDEO_URL,
        "play": {"progressive": [
            {"type": "audio/mp4", "codec": "aac", "height": 0, "link": playback_url}
        ]}
    }))
    .unwrap()
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}
