#[path = "support/http_replay.rs"]
mod http_replay;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use http_replay::{ReplayResponse, ReplayServer};
use mantle_audio::EncodedFrameSlot;
use mantle_core::{SourceLoad, SourceManager, SourceReference, SourceRegistryError};
use mantle_media::{
    HlsLimits, HlsLiveLimits, HttpNetworkAccess, HttpPlaylistOptions, HttpStreamOptions,
    MediaCancellation, MediaLimits, MpegTsLimits, PlaylistLimits, RemoteHttpOptions,
    TwitchAuthentication, TwitchErrorKind, TwitchLivePlaybackOptions, TwitchLivePlaybackPoll,
    TwitchPlaybackScheme, TwitchRoute, TwitchSourceManager, TwitchSourceOptions,
    route_twitch_identifier,
};
use serde_json::{Value, json};

const CHANNEL: &str = "twitchdev";
const CHANNEL_URL: &str = "https://www.twitch.tv/twitchdev";

#[test]
fn routes_current_live_channel_hosts_strictly() {
    let options = TwitchSourceOptions::default();
    for identifier in [
        "http://twitch.tv/TwitchDev?referrer=raid",
        "https://www.twitch.tv/TwitchDev",
        "https://go.twitch.tv/twitchdev/",
        "m.twitch.tv/TWITCHDEV",
    ] {
        assert_eq!(
            route_twitch_identifier(identifier, &options),
            Some(TwitchRoute {
                channel: CHANNEL.to_owned(),
            })
        );
    }
    for rejected in [
        CHANNEL,
        "https://token@twitch.tv/twitchdev",
        "https://twitch.tv:443/twitchdev",
        "https://twitch.test/twitchdev",
        "https://twitch.tv.evil.test/twitchdev",
        "https://twitch.tv/videos",
        "https://twitch.tv/videos/123",
        "https://twitch.tv/twitch-dev",
        "https://twitch.tv/twitchdev/extra",
        "https://twitch.tv/twitchdev#fragment",
    ] {
        assert_eq!(route_twitch_identifier(rejected, &options), None);
    }
}

#[test]
fn helix_replay_builds_live_metadata_and_keeps_credentials_private() {
    let server = ReplayServer::start(|request, _| {
        assert_eq!(
            request.target,
            "/helix/streams?user_login=twitchdev&first=1"
        );
        assert_eq!(request.header("accept"), Some("application/json"));
        assert_eq!(request.header("client-id"), Some("client-id"));
        assert_eq!(request.header("authorization"), Some("Bearer oauth-secret"));
        ReplayResponse::json(live_stream_json())
    });
    let manager = manager(&server);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap();
    assert_eq!(track.info.title, "Fixture stream");
    assert_eq!(track.info.author, "TwitchDev");
    assert_eq!(track.info.duration, Duration::ZERO);
    assert_eq!(track.info.identifier, CHANNEL_URL);
    assert_eq!(track.info.uri.as_deref(), Some(CHANNEL_URL));
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://static-cdn.jtvnw.net/previews-ttv/live_user_twitchdev-440x248.jpg")
    );
    assert!(track.info.is_stream);
    assert_eq!(track.channel, CHANNEL);
    let diagnostic = format!("{manager:?}");
    for secret in ["client-id", "oauth-secret", "device-secret"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
}

#[test]
fn helix_offline_auth_rate_limit_and_cancellation_are_distinct() {
    for (status, body, expected) in [
        (200, br#"{"data":[]}"#.as_slice(), TwitchErrorKind::Offline),
        (401, b"".as_slice(), TwitchErrorKind::AuthenticationRequired),
        (429, b"".as_slice(), TwitchErrorKind::RateLimited),
    ] {
        let server = ReplayServer::start(move |_, _| {
            if status == 200 {
                ReplayResponse::json(body)
            } else {
                ReplayResponse::status(status)
            }
        });
        let error = manager(&server)
            .load_route(&route(), &MediaCancellation::new())
            .unwrap_err();
        assert_eq!(error.kind(), expected);
    }

    let server = ReplayServer::start(|_, _| ReplayResponse::json(live_stream_json()));
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert_eq!(
        manager(&server)
            .load_route(&route(), &cancellation)
            .unwrap_err()
            .kind(),
        TwitchErrorKind::Cancelled
    );
    assert!(server.requests().is_empty());
}

#[test]
fn compatibility_query_is_bounded_redacted_and_never_receives_oauth() {
    let server = ReplayServer::start(|request, _| {
        assert_eq!(request.target, "/gql");
        assert_eq!(request.header("content-type"), Some("application/json"));
        assert_eq!(request.header("client-id"), Some("client-id"));
        assert_eq!(request.header("x-device-id"), Some("device-secret"));
        assert_eq!(request.header("authorization"), None);
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["operationName"], "PlaybackAccessToken_Template");
        assert_eq!(body["variables"]["login"], CHANNEL);
        assert_eq!(body["variables"]["isLive"], true);
        assert!(
            body["query"]
                .as_str()
                .unwrap()
                .contains("streamPlaybackAccessToken")
        );
        assert!(!String::from_utf8_lossy(&request.body).contains("oauth-secret"));
        ReplayResponse::json(playback_token_json())
    });
    let playback = manager(&server)
        .resolve_playback(CHANNEL, &MediaCancellation::new())
        .unwrap();
    assert!(
        playback
            .as_str()
            .starts_with(&server.url("usher/twitchdev.m3u8?"))
    );
    assert!(playback.as_str().contains("sig=signature-secret"));
    assert!(playback.as_str().contains("player_backend=html5"));
    let diagnostic = format!("{playback:?}");
    assert!(!diagnostic.contains("signature-secret"));
    assert!(!diagnostic.contains("expires"));

    let offline = ReplayServer::start(|_, _| {
        ReplayResponse::json(br#"{"data":{"streamPlaybackAccessToken":null}}"#)
    });
    assert_eq!(
        manager(&offline)
            .resolve_playback(CHANNEL, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        TwitchErrorKind::Offline
    );

    let malformed = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"data":{"streamPlaybackAccessToken":{"value":"{}","signature":"sig"}}}"#,
        )
    });
    assert_eq!(
        manager(&malformed)
            .resolve_playback(CHANNEL, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        TwitchErrorKind::InvalidResponse
    );

    let oversized = ReplayServer::start(|_, _| ReplayResponse::json(playback_token_json()));
    let mut bounded = options(&oversized);
    bounded.max_signature_bytes = 4;
    assert_eq!(
        TwitchSourceManager::new(bounded, authentication())
            .unwrap()
            .resolve_playback(CHANNEL, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        TwitchErrorKind::InvalidResponse
    );
}

#[test]
fn signed_master_selects_lowest_bandwidth_and_reuses_bounded_live_pipeline() {
    let transport: Arc<[u8]> = fs::read(media_fixture("tone-aac-lc.ts")).unwrap().into();
    let response_transport = Arc::clone(&transport);
    let server = ReplayServer::start(move |request, _| {
        if request.target.starts_with("/helix/streams?") {
            assert_eq!(request.header("authorization"), Some("Bearer oauth-secret"));
            return ReplayResponse::json(live_stream_json());
        }
        assert_eq!(request.header("authorization"), None);
        match request.target.split('?').next().unwrap() {
            "/gql" => ReplayResponse::json(playback_token_json()),
            "/usher/twitchdev.m3u8" => ReplayResponse::json(
                b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000000\nhigh.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=128000\naudio.m3u8\n",
            ),
            "/usher/audio.m3u8" => ReplayResponse::json(
                b"#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:10\n#EXTINF:1,Audio\nsegment.ts\n",
            ),
            "/usher/segment.ts" => ReplayResponse::json(response_transport.as_ref()),
            "/usher/high.m3u8" => panic!("higher-bandwidth variant must not be selected"),
            target => panic!("unexpected Twitch replay request: {target}"),
        }
    });
    let manager = manager(&server);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap();
    let mut playback = manager
        .open_live_playback(&track, private_live_options(), MediaCancellation::new())
        .unwrap();
    let mut output = EncodedFrameSlot::new();
    assert_eq!(
        playback.poll_frame(Duration::ZERO, &mut output).unwrap(),
        TwitchLivePlaybackPoll::Frame
    );
    assert_eq!(output.timestamp(), Some(Duration::ZERO));
    assert!(!output.data().is_empty());
    let targets: Vec<_> = server
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert!(targets.iter().any(|target| target == "/usher/audio.m3u8"));
    assert!(targets.iter().any(|target| target == "/usher/segment.ts"));
    assert!(!targets.iter().any(|target| target == "/usher/high.m3u8"));
}

#[test]
fn source_contract_reconstructs_empty_details_and_shutdown_is_terminal() {
    let server = ReplayServer::start(|_, _| ReplayResponse::json(live_stream_json()));
    let manager = manager(&server);
    let reference = SourceReference::new(Some(CHANNEL_URL.to_owned()), false);
    let loaded = manager.load(&reference).unwrap().unwrap();
    let SourceLoad::Item(track) = loaded else {
        panic!("expected one Twitch track")
    };
    assert!(manager.is_encodable(&track));
    assert!(manager.encode(&track).unwrap().is_empty());
    let restored = manager.decode_with_info(&track.info, &[]).unwrap();
    assert_eq!(restored, track);
    assert!(manager.decode(&[]).is_err());
    assert!(manager.decode_with_info(&track.info, &[1]).is_err());
    manager.shutdown();
    assert!(matches!(
        manager.load(&reference),
        Err(SourceRegistryError::Shutdown)
    ));
}

#[test]
fn invalid_authentication_and_option_bounds_fail_before_network_use() {
    assert_eq!(
        TwitchAuthentication::new("", "token").unwrap_err().kind(),
        TwitchErrorKind::InvalidAuthentication
    );
    assert_eq!(
        TwitchAuthentication::new("client", "bad\nvalue")
            .unwrap_err()
            .kind(),
        TwitchErrorKind::InvalidAuthentication
    );
    let server = ReplayServer::start(|_, _| ReplayResponse::json(live_stream_json()));
    let mut invalid = options(&server);
    invalid.max_signature_bytes = 0;
    assert_eq!(
        TwitchSourceManager::new(invalid, authentication())
            .unwrap_err()
            .kind(),
        TwitchErrorKind::InvalidOptions
    );
    assert!(server.requests().is_empty());
}

#[test]
#[ignore = "requires caller-owned Twitch client ID, access token, and known-live channel"]
fn scheduled_live_helix_metadata_smoke() {
    let url = std::env::var("MANTLE_TWITCH_SMOKE_URL").expect("Twitch smoke URL");
    let client_id = std::env::var("MANTLE_TWITCH_CLIENT_ID").expect("Twitch client ID");
    let access_token = std::env::var("MANTLE_TWITCH_ACCESS_TOKEN").expect("Twitch access token");
    let options = TwitchSourceOptions::default();
    let route = route_twitch_identifier(&url, &options).expect("Twitch live-channel route");
    let manager = TwitchSourceManager::new(
        options,
        TwitchAuthentication::new(client_id, access_token).unwrap(),
    )
    .unwrap();
    let track = manager
        .load_route(&route, &MediaCancellation::new())
        .expect("known-live Twitch channel");
    assert!(!track.info.title.is_empty());
    assert_eq!(track.channel, route.channel);
    assert!(track.info.is_stream);
}

fn options(server: &ReplayServer) -> TwitchSourceOptions {
    TwitchSourceOptions {
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            max_retries: 0,
            ..RemoteHttpOptions::default()
        },
        helix_base_url: server.url("helix"),
        gql_url: server.url("gql"),
        usher_base_url: server.url("usher"),
        playback_scheme: TwitchPlaybackScheme::HttpForPrivateNetworks,
        ..TwitchSourceOptions::default()
    }
}

fn authentication() -> TwitchAuthentication {
    TwitchAuthentication::with_device_id("client-id", "oauth-secret", Some("device-secret"))
        .unwrap()
}

fn manager(server: &ReplayServer) -> TwitchSourceManager {
    TwitchSourceManager::new(options(server), authentication()).unwrap()
}

fn route() -> TwitchRoute {
    TwitchRoute {
        channel: CHANNEL.to_owned(),
    }
}

fn private_live_options() -> TwitchLivePlaybackOptions {
    let segment = HttpStreamOptions {
        max_response_bytes: 128 * 1_024,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        max_retries: 0,
        ..HttpStreamOptions::default()
    };
    TwitchLivePlaybackOptions {
        playlist: HttpPlaylistOptions {
            http: HttpStreamOptions {
                max_response_bytes: 4 * 1_024,
                ..segment
            },
            playlist: PlaylistLimits {
                max_playlist_bytes: 4 * 1_024,
                ..PlaylistLimits::default()
            },
            include_plain: false,
        },
        segment,
        hls: HlsLimits::default(),
        live: HlsLiveLimits::default(),
        mpeg_ts: MpegTsLimits::default(),
        media: MediaLimits::default(),
    }
}

fn live_stream_json() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "data": [{
            "id": "fixture-stream",
            "user_login": CHANNEL,
            "user_name": "TwitchDev",
            "type": "live",
            "title": "Fixture stream",
            "thumbnail_url": "https://static-cdn.jtvnw.net/previews-ttv/live_user_twitchdev-{width}x{height}.jpg"
        }]
    }))
    .unwrap()
}

fn playback_token_json() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "data": {
            "streamPlaybackAccessToken": {
                "value": "{\"expires\":1780000000,\"channel\":\"twitchdev\"}",
                "signature": "signature-secret"
            }
        }
    }))
    .unwrap()
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}
