#[allow(dead_code)]
#[path = "support/http_replay.rs"]
mod http_replay;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use aes::Aes128;
use cbc::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
use http_replay::{ReplayResponse, ReplayServer};
use mantle_core::{SourceLoad, SourceManager, SourceReference, SourceRegistryError};
use mantle_media::{
    Codec, Container, HttpNetworkAccess, MediaCancellation, MediaLimits, NicoNicoAuthentication,
    NicoNicoErrorKind, NicoNicoPlaybackScheme, NicoNicoRoute, NicoNicoSourceManager,
    NicoNicoSourceOptions, PcmFrame, RemoteHttpOptions, route_niconico_identifier,
};
use serde_json::json;

const VIDEO_ID: &str = "sm9";
const VIDEO_URL: &str = "https://www.nicovideo.jp/watch/sm9";
const SESSION: &str = "fixture_user_session_secret";

#[test]
fn routes_current_watch_and_shorts_urls_strictly() {
    let options = NicoNicoSourceOptions::default();
    for accepted in [
        "https://www.nicovideo.jp/watch/sm9",
        "http://nicovideo.jp/watch/sm9?from=share",
        "https://sp.nicovideo.jp/watch/sm9",
        "https://embed.nicovideo.jp/watch/sm9",
        "https://www.nicovideo.jp/shorts/12345",
        "nicovideo.jp/watch/nm123",
    ] {
        assert!(
            route_niconico_identifier(accepted, &options).is_some(),
            "{accepted}"
        );
    }
    assert_eq!(
        route_niconico_identifier(VIDEO_URL, &options),
        Some(NicoNicoRoute {
            video_id: VIDEO_ID.to_owned(),
        })
    );
    for rejected in [
        VIDEO_ID,
        "https://user@www.nicovideo.jp/watch/sm9",
        "https://www.nicovideo.jp:443/watch/sm9",
        "https://nicovideo.jp.evil.test/watch/sm9",
        "https://www.nicovideo.jp/watch/SM9",
        "https://www.nicovideo.jp/watch/sm",
        "https://www.nicovideo.jp/watch/abc123",
        "https://www.nicovideo.jp/watch/sm9/extra",
        "https://www.nicovideo.jp/watch/sm9#fragment",
    ] {
        assert_eq!(
            route_niconico_identifier(rejected, &options),
            None,
            "{rejected}"
        );
    }
}

#[test]
fn watch_and_access_rights_replay_builds_metadata_without_leaking_secrets() {
    let delivery = ReplayServer::start(|request, _| {
        assert_eq!(
            request.target,
            "/delivery/master.m3u8?signature=media-secret"
        );
        ReplayResponse::json(b"#EXTM3U\n".to_vec())
    });
    let content_url = delivery.url("delivery/master.m3u8?signature=media-secret");
    let control = ReplayServer::start(move |request, _| {
        if request.target.starts_with("/watch/v3/") {
            assert_eq!(
                request.header("cookie"),
                Some("user_session=fixture_user_session_secret")
            );
            assert_eq!(request.header("x-frontend-id"), Some("6"));
            assert_eq!(request.header("accept"), Some("*/*"));
            ReplayResponse::json(watch_response())
        } else {
            assert!(
                request
                    .target
                    .starts_with("/access/sm9/access-rights/hls?actionTrackId=")
            );
            assert_eq!(
                request.header("x-access-right-key"),
                Some("access-right-secret")
            );
            assert_eq!(
                request.header("x-request-with"),
                Some("https://www.nicovideo.jp")
            );
            assert_eq!(
                request.header("cookie"),
                Some("user_session=fixture_user_session_secret")
            );
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
                json!({"outputs": [["video-h264-360p", "audio-aac-128kbps"]]})
            );
            ReplayResponse::json_status(
                201,
                serde_json::to_vec(&json!({
                    "meta": {"status": 201},
                    "data": {
                        "contentUrl": content_url,
                        "createTime": "2026-08-21T00:00:00Z",
                        "expireTime": "2026-08-21T00:05:00Z"
                    }
                }))
                .unwrap(),
            )
        }
    });
    let manager = authenticated_manager(&control);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .expect("public fixture");
    assert_eq!(track.info.title, "Fixture Nico Video");
    assert_eq!(track.info.author, "Fixture Uploader");
    assert_eq!(track.info.duration, Duration::from_secs(320));
    assert_eq!(track.info.identifier, VIDEO_ID);
    assert_eq!(track.info.uri.as_deref(), Some(VIDEO_URL));
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://nicovideo.cdn.example/large.jpg")
    );
    assert!(track.playback_available);

    let playback = manager
        .resolve_track_playback(&track, &MediaCancellation::new())
        .unwrap()
        .expect("current access right");
    assert!(playback.as_str().contains("media-secret"));
    let diagnostics = format!("{manager:?} {track:?} {playback:?}");
    for secret in [
        SESSION,
        "access-right-secret",
        "watch-track-secret",
        "media-secret",
    ] {
        assert!(!diagnostics.contains(secret));
    }
    assert_eq!(delivery.requests().len(), 0);
}

#[test]
fn encrypted_cmaf_audio_replay_opens_and_decodes_pcm() {
    let fragmented = fs::read(media_fixture("tone-aac-lc-fragmented.m4a")).unwrap();
    let (init, media) = split_fragmented_mp4(&fragmented);
    let key = *b"0123456789abcdef";
    let iv = [0x22_u8; 16];
    let encrypted_init = encrypt(init, &key, &iv);
    let encrypted_media = encrypt(media, &key, &iv);
    let delivery = ReplayServer::start(move |request, _| {
        match request.target.as_str() {
        "/delivery/master.m3u8?signature=media-secret" => ReplayResponse::json(
            b"#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio-aac-128kbps\",NAME=\"audio\",DEFAULT=YES,AUTOSELECT=YES,URI=\"audio.m3u8\"\n#EXT-X-STREAM-INF:BANDWIDTH=500000,AUDIO=\"audio-aac-128kbps\"\nvideo.m3u8\n"
                .to_vec(),
        ),
        "/delivery/audio.m3u8" => ReplayResponse::json(
            b"#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:2\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-KEY:METHOD=AES-128,URI=\"../asset/key.bin\",IV=0x22222222222222222222222222222222\n#EXT-X-MAP:URI=\"../asset/init.cmfa\"\n#EXTINF:1.0,\n../asset/segment.cmfa\n#EXT-X-ENDLIST\n"
                .to_vec(),
        ),
        "/asset/key.bin" => ReplayResponse::json(key.to_vec()),
        "/asset/init.cmfa" => ReplayResponse::json(encrypted_init.clone()),
        "/asset/segment.cmfa" => ReplayResponse::json(encrypted_media.clone()),
        target => panic!("unexpected delivery target {target}"),
    }
    });
    let content_url = delivery.url("delivery/master.m3u8?signature=media-secret");
    let control = ReplayServer::start(move |request, _| {
        if request.target.starts_with("/watch/v3_guest/") {
            assert_eq!(request.header("cookie"), None);
            ReplayResponse::json(watch_response())
        } else {
            assert_eq!(request.header("cookie"), None);
            ReplayResponse::json_status(
                201,
                serde_json::to_vec(&json!({
                    "meta": {"status": 201},
                    "data": {"contentUrl": content_url}
                }))
                .unwrap(),
            )
        }
    });
    let manager = public_manager(&control);
    let track = manager
        .load_route(&route(), &MediaCancellation::new())
        .unwrap()
        .unwrap();
    let mut session = manager
        .open_track_playback(&track, MediaLimits::default(), MediaCancellation::new())
        .unwrap()
        .expect("CMAF playback");
    assert_eq!(session.info().container, Container::Mp4);
    assert_eq!(session.info().codec, Codec::AacLc);
    let mut pcm = PcmFrame::with_capacity(256 * 1024);
    assert!(session.read_pcm(&mut pcm).unwrap());
    assert!(!pcm.samples().is_empty());

    for request in delivery.requests() {
        assert_eq!(request.header("cookie"), None);
        assert_eq!(request.header("x-access-right-key"), None);
    }
}

#[test]
fn malformed_watch_data_and_cancelled_load_are_stable_failures() {
    let malformed =
        ReplayServer::start(|_, _| ReplayResponse::json(b"{\"meta\":{\"status\":200}}".to_vec()));
    assert_eq!(
        public_manager(&malformed)
            .load_route(&route(), &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        NicoNicoErrorKind::InvalidResponse
    );

    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert_eq!(
        public_manager(&malformed)
            .load_route(&route(), &cancellation)
            .unwrap_err()
            .kind(),
        NicoNicoErrorKind::Cancelled
    );
}

#[test]
fn source_manager_uses_empty_details_and_lifecycle_guards() {
    let server = ReplayServer::start(|_, _| ReplayResponse::json(watch_response()));
    let manager = public_manager(&server);
    assert_eq!(manager.source_name(), "niconico");
    let loaded = manager
        .load(&SourceReference::new(Some(VIDEO_URL.to_owned()), false))
        .unwrap()
        .expect("claimed route");
    let SourceLoad::Item(loaded) = loaded else {
        panic!("expected single item")
    };
    assert_eq!(manager.encode(&loaded).unwrap(), Vec::<u8>::new());
    let decoded = manager.decode_with_info(&loaded.info, &[]).unwrap();
    assert!(!decoded.playback_available);
    assert!(manager.decode_with_info(&loaded.info, &[1]).is_err());
    manager.shutdown();
    assert_eq!(
        manager.load(&SourceReference::new(Some(VIDEO_URL.to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    );
}

#[test]
#[ignore = "scheduled live-service smoke; playback delivery is region conditional"]
fn scheduled_live_watch_and_access_rights_smoke() {
    let url = std::env::var("MANTLE_NICONICO_SMOKE_URL").unwrap_or_else(|_| VIDEO_URL.to_owned());
    let options = NicoNicoSourceOptions::default();
    let route = route_niconico_identifier(&url, &options).expect("NicoNico smoke URL");
    let manager = if let Ok(session) = std::env::var("MANTLE_NICONICO_USER_SESSION") {
        NicoNicoSourceManager::with_authentication(
            options,
            NicoNicoAuthentication::new_user_session(session).expect("NicoNico user_session"),
        )
        .unwrap()
    } else {
        NicoNicoSourceManager::new(options).unwrap()
    };
    let track = manager
        .load_route(&route, &MediaCancellation::new())
        .unwrap()
        .expect("live NicoNico metadata");
    assert_eq!(track.info.identifier, route.video_id);
    assert!(
        manager
            .resolve_track_playback(&track, &MediaCancellation::new())
            .unwrap()
            .is_some()
    );
}

fn route() -> NicoNicoRoute {
    NicoNicoRoute {
        video_id: VIDEO_ID.to_owned(),
    }
}

fn public_manager(server: &ReplayServer) -> NicoNicoSourceManager {
    NicoNicoSourceManager::new(options(server)).unwrap()
}

fn authenticated_manager(server: &ReplayServer) -> NicoNicoSourceManager {
    NicoNicoSourceManager::with_authentication(
        options(server),
        NicoNicoAuthentication::new_user_session(SESSION).unwrap(),
    )
    .unwrap()
}

fn options(server: &ReplayServer) -> NicoNicoSourceOptions {
    NicoNicoSourceOptions {
        watch_api_base_url: server.url("watch"),
        access_api_base_url: server.url("access"),
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            max_retries: 0,
            ..RemoteHttpOptions::default()
        },
        playback_scheme: NicoNicoPlaybackScheme::HttpForPrivateNetworks,
        ..NicoNicoSourceOptions::default()
    }
}

fn watch_response() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "meta": {"status": 200},
        "data": {
            "video": {
                "id": VIDEO_ID,
                "title": "Fixture Nico Video",
                "duration": 320,
                "thumbnail": {
                    "url": "https://nicovideo.cdn.example/small.jpg",
                    "largeUrl": "https://nicovideo.cdn.example/large.jpg"
                }
            },
            "owner": {"nickname": "Fixture Uploader"},
            "client": {"watchTrackId": "watch-track-secret"},
            "media": {
                "domand": {
                    "accessRightKey": "access-right-secret",
                    "videos": [
                        {"id": "video-h264-720p", "isAvailable": true, "bitRate": 1_500_000},
                        {"id": "video-h264-360p", "isAvailable": true, "bitRate": 300_000}
                    ],
                    "audios": [
                        {"id": "audio-aac-64kbps", "isAvailable": true, "bitRate": 64000},
                        {"id": "audio-aac-128kbps", "isAvailable": true, "bitRate": 128_000}
                    ]
                }
            }
        }
    }))
    .unwrap()
}

fn encrypt(bytes: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Vec<u8> {
    let mut buffer = vec![0_u8; bytes.len() + 16];
    buffer[..bytes.len()].copy_from_slice(bytes);
    let encrypted = cbc::Encryptor::<Aes128>::new(key.into(), iv.into())
        .encrypt_padded::<Pkcs7>(&mut buffer, bytes.len())
        .unwrap();
    encrypted.to_vec()
}

fn split_fragmented_mp4(bytes: &[u8]) -> (&[u8], &[u8]) {
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert!(size >= 8 && offset + size <= bytes.len());
        if &bytes[offset + 4..offset + 8] == b"moof" {
            return bytes.split_at(offset);
        }
        offset += size;
    }
    panic!("fragmented MP4 fixture has no moof box")
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}
