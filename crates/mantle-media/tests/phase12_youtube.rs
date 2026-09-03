use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mantle_audio::{
    AudioFrameError, COMPATIBLE_PCM_SAMPLES, EncodedFrameSlot, FilterChainBuilder,
    OpusEncodingQuality, PcmFilter, PcmFilterFactory, PcmFormat, PcmFrame, PcmOpusEncoder,
    StreamingPcmProcessor, StreamingPcmProgress, VolumeLevel,
};
use mantle_core::{
    SourceCancellation, SourceLoad, SourceManager, SourceReference, SourceRegistryError, TrackInfo,
};
use mantle_media::{
    Codec, EncodedPacket, HlsLimits, HlsLiveLimits, HttpNetworkAccess, HttpPlaylistOptions,
    HttpRangeOptions, HttpStreamOptions, MediaCancellation, MediaLimits, MediaSession,
    MpegTsLimits, OutboundRoute, OutboundRouteContext, OutboundRouteOutcome, OutboundRoutePolicy,
    PlaylistLimits, YoutubeAudioSourceManager, YoutubeAuthentication, YoutubeCipherChallenge,
    YoutubeCipherResolver, YoutubeCipherResolverError, YoutubeCipherSolution, YoutubeClientKind,
    YoutubeErrorKind, YoutubeLivePlaybackOptions, YoutubeLivePlaybackPoll, YoutubeOAuthClock,
    YoutubeOAuthOptions, YoutubePlaybackErrorKind, YoutubePlaybackFormatKind, YoutubePlaybackMode,
    YoutubeProcessCipherOptions, YoutubeProcessCipherResolver, YoutubeRoute, YoutubeSourceItem,
    YoutubeSourceOptions, route_youtube_identifier,
};
use serde_json::Value;

struct StaticCipherResolver {
    solution: YoutubeCipherSolution,
    calls: AtomicUsize,
}

struct SilenceFilter;

impl PcmFilter for SilenceFilter {
    fn process(&mut self, frame: &mut PcmFrame) -> Result<(), AudioFrameError> {
        frame.samples_mut().fill(0.0);
        Ok(())
    }

    fn reset(&mut self) {}
}

struct SilenceFactory;

impl PcmFilterFactory for SilenceFactory {
    fn build(
        &self,
        _format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), AudioFrameError> {
        builder.push(SilenceFilter)
    }
}

struct OversizedFilterFactory;

impl PcmFilterFactory for OversizedFilterFactory {
    fn build(
        &self,
        _format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), AudioFrameError> {
        for _ in 0..=mantle_audio::MAX_FILTERS_PER_CHAIN {
            builder.push(SilenceFilter)?;
        }
        Ok(())
    }
}

struct DelayedProcessor {
    buffered: [f32; COMPATIBLE_PCM_SAMPLES],
    buffered_valid: bool,
    nonempty_calls: Arc<AtomicUsize>,
    resets: Arc<AtomicUsize>,
}

impl StreamingPcmProcessor for DelayedProcessor {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<StreamingPcmProgress, AudioFrameError> {
        if input.is_empty() {
            return Ok(StreamingPcmProgress::default());
        }
        self.nonempty_calls.fetch_add(1, Ordering::AcqRel);
        if self.buffered_valid {
            output[..COMPATIBLE_PCM_SAMPLES].copy_from_slice(&self.buffered);
            self.buffered_valid = false;
            return Ok(StreamingPcmProgress::new(0, COMPATIBLE_PCM_SAMPLES));
        }
        if input.len() != COMPATIBLE_PCM_SAMPLES {
            return Err(AudioFrameError::StreamingProcessorCapacityExceeded {
                required: COMPATIBLE_PCM_SAMPLES,
                capacity: input.len(),
            });
        }
        self.buffered.copy_from_slice(input);
        self.buffered_valid = true;
        Ok(StreamingPcmProgress::new(input.len(), 0))
    }

    fn finish(&mut self, output: &mut [f32]) -> Result<usize, AudioFrameError> {
        if !self.buffered_valid {
            return Ok(0);
        }
        output[..COMPATIBLE_PCM_SAMPLES].copy_from_slice(&self.buffered);
        self.buffered_valid = false;
        Ok(COMPATIBLE_PCM_SAMPLES)
    }

    fn reset(&mut self) {
        self.buffered.fill(0.0);
        self.buffered_valid = false;
        self.resets.fetch_add(1, Ordering::AcqRel);
    }
}

struct DelayedFactory {
    nonempty_calls: Arc<AtomicUsize>,
    resets: Arc<AtomicUsize>,
}

impl PcmFilterFactory for DelayedFactory {
    fn build(
        &self,
        _format: PcmFormat,
        builder: &mut FilterChainBuilder,
    ) -> Result<(), AudioFrameError> {
        builder.push_streaming(DelayedProcessor {
            buffered: [0.0; COMPATIBLE_PCM_SAMPLES],
            buffered_valid: false,
            nonempty_calls: Arc::clone(&self.nonempty_calls),
            resets: Arc::clone(&self.resets),
        })
    }
}

#[derive(Debug)]
struct LoopbackRoutePolicy {
    selections: AtomicUsize,
    outcomes: AtomicUsize,
}

impl OutboundRoutePolicy for LoopbackRoutePolicy {
    fn select_route(&self, context: OutboundRouteContext<'_>) -> Option<OutboundRoute> {
        assert_eq!(context.scheme, "http");
        self.selections.fetch_add(1, Ordering::AcqRel);
        Some(OutboundRoute {
            local_ip: "127.0.0.2".parse().unwrap(),
            identity: 7,
        })
    }

    fn report_outcome(&self, route: OutboundRoute, outcome: OutboundRouteOutcome) {
        assert_eq!(route.identity, 7);
        assert_eq!(outcome, OutboundRouteOutcome::ConnectionEstablished);
        self.outcomes.fetch_add(1, Ordering::AcqRel);
    }
}

impl StaticCipherResolver {
    fn new(solution: YoutubeCipherSolution) -> Self {
        Self {
            solution,
            calls: AtomicUsize::new(0),
        }
    }
}

impl YoutubeCipherResolver for StaticCipherResolver {
    fn resolve(
        &self,
        challenge: &YoutubeCipherChallenge<'_>,
    ) -> Result<YoutubeCipherSolution, YoutubeCipherResolverError> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        assert!(
            challenge
                .player_script_url()
                .ends_with("/s/player/current.js")
        );
        assert!(
            std::str::from_utf8(challenge.player_script())
                .unwrap()
                .contains("signatureTimestamp:20676")
        );
        assert_eq!(challenge.signature(), Some("cipher-secret"));
        assert_eq!(challenge.n_parameter(), Some("throttle-secret"));
        assert!(challenge.max_output_bytes() >= 16);
        assert!(!challenge.cancellation().is_cancelled());
        let diagnostic = format!("{challenge:?}");
        for secret in ["current.js", "cipher-secret", "throttle-secret"] {
            assert!(!diagnostic.contains(secret), "{diagnostic}");
        }
        Ok(self.solution.clone())
    }
}

#[test]
fn routes_current_video_playlist_mix_and_search_identifiers() {
    let options = YoutubeSourceOptions::default();
    let video_id = "dQw4w9WgXcQ";
    assert_eq!(
        route_youtube_identifier("ytsearch:  synthwave mix ", &options),
        Some(YoutubeRoute::Search("synthwave mix".to_owned()))
    );
    assert_eq!(
        route_youtube_identifier("ytmsearch:  ambient ", &options),
        Some(YoutubeRoute::MusicSearch("ambient".to_owned()))
    );
    assert_eq!(
        route_youtube_identifier(video_id, &options),
        Some(YoutubeRoute::Video(video_id.to_owned()))
    );
    assert_eq!(
        route_youtube_identifier(
            "https://www.youtube.com/watch?list=PLabc_123&v=dQw4w9WgXcQ&t=3",
            &options,
        ),
        Some(YoutubeRoute::Playlist {
            playlist_id: "PLabc_123".to_owned(),
            selected_video_id: Some(video_id.to_owned()),
        })
    );
    assert_eq!(
        route_youtube_identifier(
            "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RDdQw4w9WgXcQ",
            &options,
        ),
        Some(YoutubeRoute::Mix {
            playlist_id: "RDdQw4w9WgXcQ".to_owned(),
            selected_video_id: video_id.to_owned(),
        })
    );
    assert_eq!(
        route_youtube_identifier("https://youtu.be/dQw4w9WgXcQ?si=ignored", &options),
        Some(YoutubeRoute::Video(video_id.to_owned()))
    );
    assert_eq!(
        route_youtube_identifier("youtube.com/shorts/dQw4w9WgXcQ-extra-path-data", &options,),
        Some(YoutubeRoute::Video(video_id.to_owned()))
    );
    assert_eq!(
        route_youtube_identifier("https://youtube.com/playlist?list=UUchannel_1", &options),
        Some(YoutubeRoute::Playlist {
            playlist_id: "UUchannel_1".to_owned(),
            selected_video_id: None,
        })
    );
    assert_eq!(
        route_youtube_identifier("ytsearch:   ", &options),
        Some(YoutubeRoute::NoTrack)
    );
    assert_eq!(
        route_youtube_identifier("https://youtube.com/watch?v=bad", &options),
        Some(YoutubeRoute::NoTrack)
    );
    assert_eq!(
        route_youtube_identifier("https://example.com/watch?v=dQw4w9WgXcQ", &options),
        None
    );

    let disabled = YoutubeSourceOptions {
        allow_search: false,
        allow_direct_video_ids: false,
        allow_direct_playlist_ids: false,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(route_youtube_identifier("ytsearch:test", &disabled), None);
    assert_eq!(route_youtube_identifier(video_id, &disabled), None);
    assert_eq!(route_youtube_identifier("PLabc_123", &disabled), None);
}

#[test]
fn default_clients_preserve_the_pinned_order_and_capabilities() {
    let options = YoutubeSourceOptions::default();
    assert_eq!(
        options.clients,
        [
            YoutubeClientKind::Music,
            YoutubeClientKind::AndroidVr,
            YoutubeClientKind::Web,
            YoutubeClientKind::WebEmbedded,
        ]
    );
    assert!(!YoutubeClientKind::Music.supports_video_loading());
    assert!(YoutubeClientKind::Music.supports_music_search());
    assert!(YoutubeClientKind::AndroidVr.supports_video_loading());
    assert!(YoutubeClientKind::AndroidVr.supports_playback());
    assert!(YoutubeClientKind::Web.supports_search());
    assert!(YoutubeClientKind::Web.supports_playlist_loading());
    assert!(!YoutubeClientKind::WebEmbedded.supports_search());
    assert!(!YoutubeClientKind::WebEmbedded.supports_playlist_loading());
    assert!(YoutubeClientKind::Tv.supports_playback());
    assert!(YoutubeClientKind::Tv.supports_oauth());
    assert!(!YoutubeClientKind::Tv.supports_video_loading());
    assert!(!YoutubeClientKind::Tv.supports_search());
    assert!(!YoutubeClientKind::Tv.supports_playlist_loading());
}

#[derive(Clone, Default)]
struct ManualOAuthClock {
    now_millis: Arc<AtomicU64>,
}

impl ManualOAuthClock {
    fn advance(&self, duration: Duration) {
        self.now_millis.fetch_add(
            u64::try_from(duration.as_millis()).unwrap(),
            Ordering::AcqRel,
        );
    }
}

impl YoutubeOAuthClock for ManualOAuthClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.now_millis.load(Ordering::Acquire))
    }
}

#[test]
fn tv_oauth_refresh_is_cached_rotated_and_scoped_to_player_requests() {
    let server = ReplayServer::start(|request, count| oauth_refresh_replay(&request, count));
    let clock = ManualOAuthClock::default();
    let manager = YoutubeAudioSourceManager::with_oauth_clock(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            oauth: YoutubeOAuthOptions {
                token_url: server.url("oauth/token"),
                ..YoutubeOAuthOptions::default()
            },
            clients: vec![YoutubeClientKind::Tv],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::with_refresh_token("refresh-secret-one".to_owned(), None, None)
            .unwrap(),
        Arc::new(clock.clone()),
    )
    .unwrap();

    for _ in 0..2 {
        assert_eq!(
            manager
                .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
                .unwrap()
                .client(),
            YoutubeClientKind::Tv
        );
    }
    assert_eq!(
        manager.oauth_refresh_token().as_deref(),
        Some("refresh-secret-two")
    );
    clock.advance(Duration::from_secs(61));
    manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();

    let requests = server.requests();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.target == "/oauth/token")
            .count(),
        2
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.target == "/youtubei/v1/player?prettyPrint=false")
            .count(),
        3
    );
    assert_eq!(
        manager.oauth_refresh_token().as_deref(),
        Some("refresh-secret-two")
    );
    let diagnostic = format!("{manager:?}");
    for secret in [
        "refresh-secret-one",
        "refresh-secret-two",
        "access-secret-one",
        "access-secret-two",
        "SboVhoG9s0rNafixCSGGKXAT",
    ] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
}

fn oauth_refresh_replay(request: &ReplayRequest, count: usize) -> ReplayResponse {
    match request.target.as_str() {
        "/oauth/token" => {
            assert_eq!(request.header("authorization"), None);
            assert_eq!(request.header("content-type"), Some("application/json"));
            let payload: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["grant_type"], "refresh_token");
            assert_eq!(
                payload["client_id"],
                "861556708454-d6dlm3lh05idd8npek18k6be8ba3oc68.apps.googleusercontent.com"
            );
            assert_eq!(payload["client_secret"], "SboVhoG9s0rNafixCSGGKXAT");
            if count == 0 {
                assert_eq!(payload["refresh_token"], "refresh-secret-one");
                ReplayResponse::json(
                    br#"{"access_token":"access-secret-one","token_type":"Bearer","expires_in":120,"refresh_token":"refresh-secret-two"}"#,
                )
            } else {
                assert_eq!(count, 3);
                assert_eq!(payload["refresh_token"], "refresh-secret-two");
                ReplayResponse::json(
                    br#"{"access_token":"access-secret-two","token_type":"Bearer","expires_in":3600}"#,
                )
            }
        }
        "/youtubei/v1/player?prettyPrint=false" => {
            let expected = if count < 4 {
                "Bearer access-secret-one"
            } else {
                "Bearer access-secret-two"
            };
            assert_eq!(request.header("authorization"), Some(expected));
            assert_eq!(
                request.header("user-agent"),
                Some("Mozilla/5.0 (ChromiumStylePlatform) Cobalt/Version")
            );
            let payload: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["context"]["client"]["clientName"], "TVHTML5");
            assert_eq!(
                payload["context"]["client"]["clientVersion"],
                "7.20250319.10.00"
            );
            assert!(payload["context"]["client"].get("clientScreen").is_none());
            assert!(payload["context"].get("thirdParty").is_none());
            assert_eq!(payload["params"], "2AMB");
            ReplayResponse::json(&playback_response(
                "https://media.example.test/audio.webm",
                "audio/webm; codecs=\"opus\"",
                100,
            ))
        }
        target => panic!("unexpected target {target}"),
    }
}

#[test]
fn oauth_device_exchange_is_caller_polled_bounded_and_redacted() {
    let server = ReplayServer::start(|request, count| {
        match count {
        0 => {
            assert_eq!(request.target, "/oauth/device");
            let payload: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["device_id"], "fixture-device-id");
            assert_eq!(payload["device_model"], "ytlr::");
            assert!(payload["scope"].as_str().unwrap().contains("youtube"));
            ReplayResponse::json(
                br#"{"verification_url":"https://example.test/device","user_code":"user-secret","device_code":"device-secret","interval":5,"expires_in":600}"#,
            )
        }
        1 => {
            assert_eq!(request.target, "/oauth/token");
            let payload: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(payload["code"], "device-secret");
            assert_eq!(
                payload["grant_type"],
                "http://oauth.net/grant_type/device/1.0"
            );
            ReplayResponse::json(br#"{"error":"authorization_pending"}"#)
        }
        2 => ReplayResponse::json(
            br#"{"access_token":"access-secret","token_type":"Bearer","expires_in":3600,"refresh_token":"refresh-secret"}"#,
        ),
        _ => panic!("unexpected OAuth request"),
    }
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            oauth: YoutubeOAuthOptions {
                device_code_url: server.url("oauth/device"),
                token_url: server.url("oauth/token"),
                ..YoutubeOAuthOptions::default()
            },
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let cancellation = MediaCancellation::new();
    let code = manager
        .request_oauth_device_code("fixture-device-id", &cancellation)
        .unwrap();
    assert_eq!(code.verification_url(), "https://example.test/device");
    assert_eq!(code.user_code(), "user-secret");
    assert_eq!(code.device_code(), "device-secret");
    assert_eq!(code.poll_interval(), Duration::from_secs(5));
    assert_eq!(code.expires_in(), Duration::from_mins(10));
    let diagnostic = format!("{code:?}");
    for secret in ["example.test", "user-secret", "device-secret"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }

    assert_eq!(
        manager
            .exchange_oauth_device_code(code.device_code(), &cancellation)
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::AuthorizationPending
    );
    let status = manager
        .exchange_oauth_device_code(code.device_code(), &cancellation)
        .unwrap();
    assert_eq!(status.expires_in(), Duration::from_hours(1));
    assert!(status.refresh_token_rotated());
    assert_eq!(
        manager.oauth_refresh_token().as_deref(),
        Some("refresh-secret")
    );

    let cancelled = MediaCancellation::new();
    cancelled.cancel();
    assert_eq!(
        manager
            .request_oauth_device_code("another-device", &cancelled)
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::Cancelled
    );
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn oauth_polling_outcomes_and_response_ceiling_fail_closed() {
    let outcomes = [
        ("slow_down", YoutubeErrorKind::OAuthSlowDown),
        ("access_denied", YoutubeErrorKind::AccessDenied),
        ("expired_token", YoutubeErrorKind::ExpiredDeviceCode),
        ("unknown", YoutubeErrorKind::InvalidResponse),
    ];
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/oauth/token");
        let error = ["slow_down", "access_denied", "expired_token", "unknown"][count];
        ReplayResponse::json(format!(r#"{{"error":"{error}"}}"#).as_bytes())
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            oauth: YoutubeOAuthOptions {
                token_url: server.url("oauth/token"),
                ..YoutubeOAuthOptions::default()
            },
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    for (error, expected) in outcomes {
        assert_eq!(
            manager
                .exchange_oauth_device_code("device-code", &MediaCancellation::new())
                .unwrap_err()
                .kind(),
            expected,
            "{error}"
        );
    }

    let oversized = ReplayServer::start(|_, _| ReplayResponse::json(&[b'x'; 33]));
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            oauth: YoutubeOAuthOptions {
                device_code_url: oversized.url("oauth/device"),
                max_response_bytes: 32,
                ..YoutubeOAuthOptions::default()
            },
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        manager
            .request_oauth_device_code("device-id", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidResponse
    );
}

#[test]
fn video_loading_falls_through_clients_and_builds_bounded_track_metadata() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.header("content-type"), Some("application/json"));
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        let client_name = payload["context"]["client"]["clientName"].as_str().unwrap();
        if count == 0 {
            assert_eq!(client_name, "ANDROID_VR");
            return ReplayResponse::json(
                br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"wrong-video","title":"Wrong","author":"Wrong","lengthSeconds":"1"}}"#,
            );
        }
        assert_eq!(client_name, "WEB");
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","title":"Fixture title","author":"Fixture author","lengthSeconds":"213","isLive":false,"thumbnail":{"thumbnails":[{"url":"https://i.ytimg.com/first.jpg"},{"url":"https://i.ytimg.com/best.jpg"}]}}}"#,
        )
    });
    let options = YoutubeSourceOptions {
        api_base_url: server.url("youtubei/v1"),
        clients: vec![YoutubeClientKind::AndroidVr, YoutubeClientKind::Web],
        http: private_http_options(),
        ..YoutubeSourceOptions::default()
    };
    let manager =
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default()).unwrap();
    let loaded = manager
        .load(&SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false))
        .unwrap()
        .unwrap();
    let SourceLoad::Item(YoutubeSourceItem::Track(track)) = loaded else {
        panic!("expected a YouTube track");
    };
    assert_eq!(track.info.title, "Fixture title");
    assert_eq!(track.info.author, "Fixture author");
    assert_eq!(track.info.duration, Duration::from_secs(213));
    assert_eq!(track.info.identifier, "dQw4w9WgXcQ");
    assert!(!track.info.is_stream);
    assert_eq!(
        track.info.uri.as_deref(),
        Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
    );
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://i.ytimg.com/best.jpg")
    );
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn ordinary_search_falls_through_clients_and_skips_non_tracks() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/youtubei/v1/search?prettyPrint=false");
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(payload["query"], "fixture query");
        assert_eq!(payload["params"], "EgIQAfABAQ==");
        if count == 0 {
            assert_eq!(payload["context"]["client"]["clientName"], "ANDROID_VR");
            return ReplayResponse::json(br#"{"unexpected":true}"#);
        }
        assert_eq!(payload["context"]["client"]["clientName"], "WEB");
        ReplayResponse::json(
            br#"{"contents":{"twoColumnSearchResultsRenderer":{"primaryContents":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"videoRenderer":{"videoId":"aaaaabbbbbb","title":{"runs":[{"text":"Search result"}]},"longBylineText":{"runs":[{"text":"Search author"}]},"lengthText":{"simpleText":"3:33"},"thumbnail":{"thumbnails":[{"url":"https://i.ytimg.com/search.jpg"}]}}},{"videoRenderer":{"videoId":"ccccccddddd","title":{"runs":[{"text":"Live is skipped"}]},"longBylineText":{"runs":[{"text":"Live author"}]}}}]}}]}}}}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr, YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let Some(SourceLoad::Item(YoutubeSourceItem::Playlist(results))) = manager
        .load(&SourceReference::new(
            Some("ytsearch: fixture query".to_owned()),
            false,
        ))
        .unwrap()
    else {
        panic!("expected search results");
    };
    assert_eq!(results.name, "Search results for: fixture query");
    assert!(results.is_search_result);
    assert_eq!(results.selected_track, None);
    assert_eq!(results.tracks.len(), 1);
    assert_eq!(results.tracks[0].info.identifier, "aaaaabbbbbb");
    assert_eq!(results.tracks[0].info.duration, Duration::from_secs(213));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn music_search_uses_the_music_endpoint_and_bounded_result_shape() {
    let server = ReplayServer::start(|request, _| {
        assert_eq!(
            request.target,
            "/music/youtubei/v1/search?prettyPrint=false"
        );
        assert_eq!(request.header("referer"), Some("music.youtube.com"));
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(payload["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(payload["query"], "music fixture");
        assert_eq!(
            payload["params"],
            "Eg-KAQwIARAAGAAgACgAMABqChADEAQQCRAFEAo="
        );
        ReplayResponse::json(
            br#"{"contents":{"tabbedSearchResultsRenderer":{"tabs":[{"tabRenderer":{"content":{"sectionListRenderer":{"contents":[{"musicShelfRenderer":{"contents":[{"musicResponsiveListItemRenderer":{"flexColumns":[{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Music result","navigationEndpoint":{"watchEndpoint":{"videoId":"mmmmmmnnnnn"}}}]}}},{"musicResponsiveListItemFlexColumnRenderer":{"text":{"runs":[{"text":"Music author","navigationEndpoint":{"browseEndpoint":{"browseId":"artist"}}},{"text":" - "},{"text":"4:05"}]}}}]}}]}}]}}}}]}}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            music_api_base_url: server.url("music/youtubei/v1"),
            clients: vec![YoutubeClientKind::Music],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let Some(SourceLoad::Item(YoutubeSourceItem::Playlist(results))) = manager
        .load(&SourceReference::new(
            Some("ytmsearch: music fixture".to_owned()),
            false,
        ))
        .unwrap()
    else {
        panic!("expected music search results");
    };
    assert_eq!(results.name, "Search music results for: music fixture");
    assert!(results.is_search_result);
    assert_eq!(results.tracks.len(), 1);
    assert_eq!(results.tracks[0].info.identifier, "mmmmmmnnnnn");
    assert_eq!(results.tracks[0].info.duration, Duration::from_secs(245));
}

#[test]
fn playlist_loading_follows_bounded_continuations_and_selects_the_requested_track() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/youtubei/v1/browse?prettyPrint=false");
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        if count == 0 {
            assert_eq!(payload["browseId"], "VLPLfixture");
            return ReplayResponse::json(
                br#"{"metadata":{"playlistMetadataRenderer":{"title":"Fixture playlist"}},"contents":{"twoColumnBrowseResultsRenderer":{"tabs":[{"tabRenderer":{"content":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"playlistVideoListRenderer":{"contents":[{"playlistVideoRenderer":{"videoId":"aaaaabbbbbb","title":{"runs":[{"text":"First"}]},"shortBylineText":{"runs":[{"text":"First author"}]},"lengthSeconds":"10","isPlayable":true}},{"continuationItemRenderer":{"continuationEndpoint":{"continuationCommand":{"token":"next-page"}}}}]}}]}}]}}}}]}}}"#,
            );
        }
        assert_eq!(payload["continuation"], "next-page");
        ReplayResponse::json(
            br#"{"onResponseReceivedActions":[{"appendContinuationItemsAction":{"continuationItems":[{"playlistVideoRenderer":{"videoId":"ccccccddddd","title":{"simpleText":"Second"},"shortBylineText":{"runs":[{"text":"Second author"}]},"lengthSeconds":"20","isPlayable":true}}]}}]}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::Web],
            max_playlist_pages: 6,
            max_playlist_tracks: 8,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let Some(SourceLoad::Item(YoutubeSourceItem::Playlist(playlist))) = manager
        .load(&SourceReference::new(
            Some("https://youtube.com/watch?v=ccccccddddd&list=PLfixture".to_owned()),
            false,
        ))
        .unwrap()
    else {
        panic!("expected playlist");
    };
    assert_eq!(playlist.name, "Fixture playlist");
    assert!(!playlist.is_search_result);
    assert_eq!(playlist.tracks.len(), 2);
    assert_eq!(playlist.selected_track, Some(1));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn mix_loading_falls_through_clients_and_selects_the_requested_track() {
    let server = ReplayServer::start(|request, count| {
        assert_eq!(request.target, "/youtubei/v1/next?prettyPrint=false");
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(payload["playlistId"], "RDfixture");
        assert_eq!(payload["videoId"], "ccccccddddd");
        if count == 0 {
            assert_eq!(payload["context"]["client"]["clientName"], "ANDROID_VR");
            return ReplayResponse::json(br#"{"unexpected":true}"#);
        }
        assert_eq!(payload["context"]["client"]["clientName"], "WEB");
        ReplayResponse::json(
            br#"{"contents":{"twoColumnWatchNextResults":{"playlist":{"playlist":{"title":"Fixture mix","contents":[{"playlistPanelVideoRenderer":{"videoId":"aaaaabbbbbb","title":{"simpleText":"First mix track"},"shortBylineText":{"runs":[{"text":"First author"}]},"lengthText":{"simpleText":"0:10"}}},{"playlistPanelVideoRenderer":{"videoId":"ccccccddddd","title":{"runs":[{"text":"Selected mix track"}]},"longBylineText":{"runs":[{"text":"Second author"}]},"lengthText":{"runs":[{"text":"0:20"}]},"thumbnail":{"thumbnails":[{"url":"https://i.ytimg.com/mix.jpg"}]}}}]}}}}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr, YoutubeClientKind::Web],
            max_mix_tracks: 8,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let Some(SourceLoad::Item(YoutubeSourceItem::Playlist(mix))) = manager
        .load(&SourceReference::new(
            Some("https://youtube.com/watch?v=ccccccddddd&list=RDfixture".to_owned()),
            false,
        ))
        .unwrap()
    else {
        panic!("expected a mix");
    };
    assert_eq!(mix.name, "Fixture mix");
    assert!(!mix.is_search_result);
    assert_eq!(mix.tracks.len(), 2);
    assert_eq!(mix.selected_track, Some(1));
    assert_eq!(mix.tracks[1].info.duration, Duration::from_secs(20));
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn mix_track_limit_fails_closed() {
    let server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"contents":{"singleColumnWatchNextResults":{"playlist":{"playlist":{"title":"Too many","contents":[{"playlistPanelVideoRenderer":{"videoId":"aaaaabbbbbb","title":{"simpleText":"First"},"shortBylineText":{"runs":[{"text":"Author"}]},"lengthText":{"simpleText":"0:01"}}},{"playlistPanelVideoRenderer":{"videoId":"ccccccddddd","title":{"simpleText":"Second"},"shortBylineText":{"runs":[{"text":"Author"}]},"lengthText":{"simpleText":"0:02"}}}]}}}}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            max_mix_tracks: 1,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        manager.load(&SourceReference::new(
            Some("https://youtube.com/watch?v=ccccccddddd&list=RDfixture".to_owned(),),
            false,
        )),
        Err(SourceRegistryError::SourceFailure)
    );
}

#[test]
fn playback_discovery_falls_through_clients_and_prefers_stereo_non_drc_opus() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"<html>{"jsUrl":"/s/player/base.js"}</html>"#);
        }
        if request.target == "/s/player/base.js" {
            return ReplayResponse::json(b"var config={signatureTimestamp:20433};");
        }
        assert_eq!(request.target, "/youtubei/v1/player?prettyPrint=false");
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        if payload["context"]["client"]["clientName"] == "ANDROID_VR" {
            assert!(payload.get("playbackContext").is_none());
            assert_eq!(payload["context"]["client"]["clientName"], "ANDROID_VR");
            return ReplayResponse::json(
                br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":160000,"contentLength":"1000","audioChannels":2,"url":"https://media.example.test/not-default?token=hidden","audioTrack":{"audioIsDefault":false}}]}}"#,
            );
        }
        assert_eq!(payload["context"]["client"]["clientName"], "WEB");
        assert_eq!(
            payload["playbackContext"]["contentPlaybackContext"]["signatureTimestamp"],
            "20433"
        );
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"formats":[{"itag":18,"mimeType":"video/mp4; codecs=\"avc1.42001E, mp4a.40.2\"","bitrate":96000,"contentLength":"900","audioChannels":2,"url":"https://media.example.test/legacy?token=legacy-secret"}],"adaptiveFormats":[{"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":192000,"contentLength":"1200","audioChannels":2,"url":"https://media.example.test/aac?token=aac-secret"},{"itag":250,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":160000,"contentLength":"1100","audioChannels":2,"url":"https://media.example.test/drc?token=drc-secret","isDrc":true},{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"url":"https://media.example.test/opus?token=opus-secret"},{"itag":258,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":256000,"contentLength":"1400","audioChannels":6,"url":"https://media.example.test/surround?token=surround-secret"}]}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::AndroidVr, YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let discovery = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    assert_eq!(discovery.client(), YoutubeClientKind::Web);
    assert!(discovery.requires_player_script());
    assert_eq!(discovery.formats().len(), 5);
    let selected = discovery.selected();
    assert_eq!(selected.kind(), Some(YoutubePlaybackFormatKind::WebmOpus));
    assert_eq!(selected.itag(), 251);
    assert!(!selected.is_drc());
    assert_eq!(selected.audio_channels(), 2);
    assert_eq!(selected.content_length(), Some(1000));
    assert_eq!(
        selected.playback_url(),
        "https://media.example.test/opus?token=opus-secret"
    );
    let diagnostic = format!("{discovery:?} {selected:?}");
    for secret in ["legacy-secret", "aac-secret", "drc-secret", "opus-secret"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
    assert_eq!(server.requests().len(), 4);
}

#[test]
fn playback_discovery_preserves_bounded_cipher_inputs_without_logging_them() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/base.js"}"#);
        }
        if request.target == "/s/player/base.js" {
            return ReplayResponse::json(b"var config={sts:20434};");
        }
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            payload["playbackContext"]["contentPlaybackContext"]["signatureTimestamp"],
            "20434"
        );
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dthrottle-secret%26expire%3D999&sp=sig&s=cipher-secret"}]}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let discovery = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let selected = discovery.selected();
    assert_eq!(selected.signature(), Some("cipher-secret"));
    assert_eq!(selected.signature_key(), "sig");
    assert_eq!(selected.n_parameter(), Some("throttle-secret"));
    assert!(selected.requires_cipher());
    let diagnostic = format!("{discovery:?} {selected:?}");
    for secret in ["cipher-secret", "throttle-secret", "media.example.test"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }

    let second = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    assert_eq!(second.selected().signature(), Some("cipher-secret"));
    let requests = server.requests();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.target == "/embed/")
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.target == "/s/player/base.js")
            .count(),
        1
    );
    assert_eq!(requests.len(), 4);
}

#[test]
fn native_cipher_program_resolves_signature_and_n_without_executing_javascript() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/cipher.js"}"#);
        }
        if request.target == "/s/player/cipher.js" {
            return ReplayResponse::json(
                br#"var H={r:function(a){a.reverse()},s:function(a,b){a.splice(0,b)},w:function(a,b){var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c}};function S(a){a=a.split("");H.w(a,2);H.r(a);H.s(a,1);return a.join("")}function N(a){a=a.split("");H.r(a);H.w(a,1);return a.join("")}player.set("signature",S(token));player.get("n")&&(token=N(token));var config={signatureTimestamp:20436};"#,
            );
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dwxyz%26expire%3D999&sp=sig&s=ab%2Bc%2Ff"}]}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let resolved = manager
        .resolve_selected_playback_url(&formats, &MediaCancellation::new())
        .unwrap();
    assert!(resolved.as_str().contains("expire=999"));
    assert!(resolved.as_str().contains("sig=%2Fcab%2B"));
    assert!(resolved.as_str().contains("n=yzxw"));
    assert!(!resolved.as_str().contains("ab+c/f"));
    assert!(!resolved.as_str().contains("n=wxyz"));
    let diagnostic = format!("{resolved:?}");
    for secret in ["media.example.test", "%2Fcab%2B", "yzxw"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn native_cipher_program_fails_closed_at_the_operation_limit_and_caches_failure() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/too-many.js"}"#);
        }
        if request.target == "/s/player/too-many.js" {
            return ReplayResponse::json(
                br#"var H={r:function(a){a.reverse()},s:function(a,b){a.splice(0,b)},w:function(a,b){var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c}};function S(a){a=a.split("");H.w(a,2);H.r(a);H.s(a,1);return a.join("")}function N(a){a=a.split("");H.r(a);return a.join("")}player.set("signature",S(token));player.get("n")&&(token=N(token));var config={signatureTimestamp:20437};"#,
            );
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dwxyz&sp=sig&s=abcdef"}]}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            max_cipher_operations: 2,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            manager
                .resolve_selected_playback_url(&formats, &MediaCancellation::new())
                .unwrap_err()
                .kind(),
            YoutubeErrorKind::InvalidResponse
        );
    }
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn bounded_cipher_provider_handles_current_script_shapes_and_validates_output() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/current.js"}"#);
        }
        if request.target == "/s/player/current.js" {
            return ReplayResponse::json(
                b"var config={signatureTimestamp:20676};(()=>{return dynamicChallengeGraph})()",
            );
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dthrottle-secret%26expire%3D999&sp=sig&s=cipher-secret"}]}}"#,
        )
    });
    let resolver = Arc::new(StaticCipherResolver::new(YoutubeCipherSolution::new(
        Some("provider-signature".to_owned()),
        Some("provider-n".to_owned()),
    )));
    let manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        resolver.clone(),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let resolved_url = manager
        .resolve_selected_playback_url(&formats, &MediaCancellation::new())
        .unwrap();
    assert!(resolved_url.as_str().contains("sig=provider-signature"));
    assert!(resolved_url.as_str().contains("n=provider-n"));
    assert_eq!(resolver.calls.load(Ordering::Acquire), 1);

    let invalid_resolver = Arc::new(StaticCipherResolver::new(YoutubeCipherSolution::new(
        Some("provider-signature".to_owned()),
        Some("throttle-secret".to_owned()),
    )));
    let invalid_manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        invalid_resolver.clone(),
    )
    .unwrap();
    let formats = invalid_manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    assert_eq!(
        invalid_manager
            .resolve_selected_playback_url(&formats, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidResponse
    );
    assert_eq!(invalid_resolver.calls.load(Ordering::Acquire), 1);
    assert_eq!(server.requests().len(), 6);
}

#[test]
fn isolated_process_cipher_provider_uses_the_bounded_protocol_without_a_shell() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/process.js"}"#);
        }
        if request.target == "/s/player/process.js" {
            return ReplayResponse::json(
                b"var config={signatureTimestamp:20676};(()=>dynamicChallengeGraph)()",
            );
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dthrottle-secret&sp=sig&s=cipher-secret"}]}}"#,
        )
    });
    let executable = std::env::current_exe().unwrap();
    let resolver = YoutubeProcessCipherResolver::new(
        &executable,
        [
            "--exact",
            "process_cipher_fixture_child",
            "--ignored",
            "--nocapture",
        ],
        YoutubeProcessCipherOptions::default(),
    )
    .unwrap();
    let diagnostic = format!("{resolver:?}");
    assert!(!diagnostic.contains(executable.to_string_lossy().as_ref()));
    assert!(!diagnostic.contains("process_cipher_fixture_child"));
    let manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        Arc::new(resolver),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let resolved_url = manager
        .resolve_selected_playback_url(&formats, &MediaCancellation::new())
        .unwrap();
    assert!(resolved_url.as_str().contains("sig=process-cipher-secret"));
    assert!(resolved_url.as_str().contains("n=process-throttle-secret"));
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn isolated_process_cipher_provider_enforces_timeout_and_cancellation() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/process.js"}"#);
        }
        if request.target == "/s/player/process.js" {
            return ReplayResponse::json(b"var config={signatureTimestamp:20676};slowFixture");
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dslow-n&sp=sig&s=slow-signature"}]}}"#,
        )
    });
    let executable = std::env::current_exe().unwrap();
    let arguments = [
        "--exact",
        "process_cipher_fixture_child",
        "--ignored",
        "--nocapture",
    ];
    let timeout_resolver = YoutubeProcessCipherResolver::new(
        &executable,
        arguments,
        YoutubeProcessCipherOptions {
            timeout: Duration::from_millis(30),
            poll_interval: Duration::from_millis(5),
            ..YoutubeProcessCipherOptions::default()
        },
    )
    .unwrap();
    let timeout_manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        Arc::new(timeout_resolver),
    )
    .unwrap();
    let formats = timeout_manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(
        timeout_manager
            .resolve_selected_playback_url(&formats, &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidResponse
    );
    assert!(started.elapsed() < Duration::from_secs(1));

    let cancellation_resolver = YoutubeProcessCipherResolver::new(
        &executable,
        arguments,
        YoutubeProcessCipherOptions {
            timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(5),
            ..YoutubeProcessCipherOptions::default()
        },
    )
    .unwrap();
    let cancellation_manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        Arc::new(cancellation_resolver),
    )
    .unwrap();
    let formats = cancellation_manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let cancellation = MediaCancellation::new();
    let trigger = cancellation.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        trigger.cancel();
    });
    let error = cancellation_manager
        .resolve_selected_playback_url(&formats, &cancellation)
        .unwrap_err();
    canceller.join().unwrap();
    assert_eq!(error.kind(), YoutubeErrorKind::Cancelled);
    assert_eq!(server.requests().len(), 6);
}

#[test]
fn finite_webm_opus_handoff_preserves_the_exact_passthrough_packet() {
    let bytes = fs::read(media_fixture("tone-opus.webm")).unwrap();
    let media = RangeMediaServer::start(bytes.clone());
    let media_url = media.url("audio.webm?signature=media-secret");
    let response = playback_response(&media_url, "audio/webm; codecs=\"opus\"", bytes.len());
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let route_policy = Arc::new(LoopbackRoutePolicy {
        selections: AtomicUsize::new(0),
        outcomes: AtomicUsize::new(0),
    });
    let manager = YoutubeAudioSourceManager::with_route_policy(
        YoutubeSourceOptions {
            api_base_url: api.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        route_policy.clone(),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let mut playback = manager
        .open_selected_playback_routed(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
            route_policy.clone(),
        )
        .unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::OpusPassthrough);
    assert_eq!(playback.info().codec, Codec::Opus);

    let mut expected_session =
        MediaSession::open_file(media_fixture("tone-opus.webm"), MediaLimits::default()).unwrap();
    let mut expected = EncodedPacket::with_capacity(expected_session.limits().max_packet_bytes);
    assert!(expected_session.read_encoded(&mut expected).unwrap());
    let mut actual = EncodedFrameSlot::new();
    assert!(playback.read_frame(&mut actual).unwrap());
    assert_eq!(actual.data(), expected.data());
    assert_eq!(actual.timestamp(), expected.timestamp());
    assert_eq!(actual.duration(), Duration::from_millis(20));
    assert!(!media.requests().is_empty());
    assert!(route_policy.selections.load(Ordering::Acquire) >= 2);
    assert_eq!(
        route_policy.selections.load(Ordering::Acquire),
        route_policy.outcomes.load(Ordering::Acquire)
    );
}

#[test]
fn finite_opus_session_seeks_filters_and_returns_safely_to_passthrough() {
    let bytes = fs::read(media_fixture("tone-opus.webm")).unwrap();
    let media = RangeMediaServer::start(bytes.clone());
    let response = playback_response(
        &media.url("audio.webm"),
        "audio/webm; codecs=\"opus\"",
        bytes.len(),
    );
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let mut playback = manager
        .open_selected_playback(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap();
    let mut output = EncodedFrameSlot::new();
    assert!(playback.read_frame(&mut output).unwrap());
    let direct = output.data().to_vec();

    playback.seek(Duration::ZERO).unwrap();
    playback.set_filter_factory(Some(&SilenceFactory)).unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);
    assert!(playback.read_frame(&mut output).unwrap());
    let filtered = output.data().to_vec();
    assert_ne!(filtered, direct);

    playback.seek(Duration::ZERO).unwrap();
    assert!(playback.read_frame(&mut output).unwrap());
    assert_eq!(output.data(), filtered);

    assert!(
        playback
            .set_filter_factory(Some(&OversizedFilterFactory))
            .is_err()
    );
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);

    playback.set_filter_factory(None).unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::OpusPassthrough);
    playback.seek(Duration::ZERO).unwrap();
    assert!(playback.read_frame(&mut output).unwrap());
    let mut expected =
        MediaSession::open_file(media_fixture("tone-opus.webm"), MediaLimits::default()).unwrap();
    expected.seek(Duration::ZERO).unwrap();
    let mut packet = EncodedPacket::with_capacity(expected.limits().max_packet_bytes);
    assert!(expected.read_encoded(&mut packet).unwrap());
    assert_eq!(output.data(), packet.data());
}

#[test]
fn finite_streaming_filter_pulls_resets_replaces_and_restores_passthrough() {
    let bytes = fs::read(media_fixture("tone-opus.webm")).unwrap();
    let media = RangeMediaServer::start(bytes.clone());
    let response = playback_response(
        &media.url("audio.webm"),
        "audio/webm; codecs=\"opus\"",
        bytes.len(),
    );
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let nonempty_calls = Arc::new(AtomicUsize::new(0));
    let resets = Arc::new(AtomicUsize::new(0));
    let factory = DelayedFactory {
        nonempty_calls: Arc::clone(&nonempty_calls),
        resets: Arc::clone(&resets),
    };
    let mut playback = manager
        .open_selected_playback(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap();
    playback.set_filter_factory(Some(&factory)).unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);

    let mut output = EncodedFrameSlot::new();
    assert!(playback.read_frame(&mut output).unwrap());
    assert!(!output.data().is_empty());
    assert!(nonempty_calls.load(Ordering::Acquire) >= 2);
    let first_timestamp = output.timestamp().unwrap();
    let first_source_position = playback.source_media_position().unwrap();
    assert!(first_source_position >= first_timestamp + output.duration());

    assert!(
        playback
            .set_filter_factory(Some(&OversizedFilterFactory))
            .is_err()
    );
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);
    assert_eq!(resets.load(Ordering::Acquire), 0);

    let seek = playback.seek(Duration::ZERO).unwrap();
    assert_eq!(resets.load(Ordering::Acquire), 1);
    assert!(playback.read_frame(&mut output).unwrap());
    let mut clean_after_seek = manager
        .open_selected_playback(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap();
    let clean_factory = DelayedFactory {
        nonempty_calls: Arc::new(AtomicUsize::new(0)),
        resets: Arc::new(AtomicUsize::new(0)),
    };
    clean_after_seek
        .set_filter_factory(Some(&clean_factory))
        .unwrap();
    clean_after_seek.seek(Duration::ZERO).unwrap();
    let mut clean_output = EncodedFrameSlot::new();
    assert!(clean_after_seek.read_frame(&mut clean_output).unwrap());
    assert_eq!(output.data(), clean_output.data());
    assert_eq!(output.timestamp(), clean_output.timestamp());
    assert!(playback.source_media_position().unwrap() >= seek.actual.unwrap_or_default());

    playback.set_filter_factory(Some(&factory)).unwrap();
    assert_eq!(resets.load(Ordering::Acquire), 2);
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);
    playback.set_filter_factory(None).unwrap();
    assert_eq!(resets.load(Ordering::Acquire), 3);
    assert_eq!(playback.mode(), YoutubePlaybackMode::OpusPassthrough);

    assert!(playback.read_frame(&mut output).unwrap());
    let mut expected =
        MediaSession::open_file(media_fixture("tone-opus.webm"), MediaLimits::default()).unwrap();
    expected.seek(Duration::ZERO).unwrap();
    let mut packet = EncodedPacket::with_capacity(expected.limits().max_packet_bytes);
    for _ in 0..3 {
        assert!(expected.read_encoded(&mut packet).unwrap());
    }
    assert_eq!(output.data(), packet.data());

    playback.seek(Duration::ZERO).unwrap();
    assert!(playback.read_frame(&mut output).unwrap());
    expected.seek(Duration::ZERO).unwrap();
    assert!(expected.read_encoded(&mut packet).unwrap());
    assert_eq!(output.data(), packet.data());

    playback.set_filter_factory(Some(&factory)).unwrap();
    drop(playback);
    assert_eq!(resets.load(Ordering::Acquire), 4);
}

#[test]
fn finite_aac_handoff_uses_the_normal_pcm_to_opus_fallback() {
    let bytes = fs::read(media_fixture("tone-aac-lc.m4a")).unwrap();
    let media = RangeMediaServer::start(bytes.clone());
    let response = playback_response(
        &media.url("audio.m4a?signature=media-secret"),
        "audio/mp4; codecs=\"mp4a.40.2\"",
        bytes.len(),
    );
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let mut playback = manager
        .open_selected_playback(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);
    assert_eq!(playback.info().codec, Codec::AacLc);

    let mut local =
        MediaSession::open_file(media_fixture("tone-aac-lc.m4a"), MediaLimits::default()).unwrap();
    let mut decoded = PcmFrame::with_capacity(local.limits().max_pcm_samples_per_frame);
    assert!(local.read_pcm(&mut decoded).unwrap());
    assert!(decoded.samples().len() >= COMPATIBLE_PCM_SAMPLES);
    let format = PcmFormat::new(48_000, 2).unwrap();
    let mut encoder_input = PcmFrame::with_capacity(COMPATIBLE_PCM_SAMPLES);
    encoder_input
        .copy_from_interleaved(
            &decoded.samples()[..COMPATIBLE_PCM_SAMPLES],
            format,
            decoded.timestamp(),
        )
        .unwrap();
    let mut encoder = PcmOpusEncoder::new(OpusEncodingQuality::MAXIMUM).unwrap();
    let mut expected = EncodedFrameSlot::new();
    encoder
        .encode(&encoder_input, &mut expected, VolumeLevel::NORMAL)
        .unwrap();

    let mut actual = EncodedFrameSlot::new();
    assert!(playback.read_frame(&mut actual).unwrap());
    assert_eq!(actual.data(), expected.data());
    assert_eq!(actual.timestamp(), expected.timestamp());
    assert!(!media.requests().is_empty());
}

#[test]
fn finite_media_handoff_rejects_mismatch_bounds_and_preflight_cancellation() {
    let bytes = fs::read(media_fixture("tone-aac-lc.m4a")).unwrap();
    let media = RangeMediaServer::start(bytes.clone());
    let response = playback_response(
        &media.url("mismatch?signature=do-not-log"),
        "audio/webm; codecs=\"opus\"",
        bytes.len(),
    );
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let error = manager
        .open_selected_playback(
            &formats,
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), YoutubePlaybackErrorKind::IncompatibleFormat);
    assert!(!format!("{error:?}").contains("do-not-log"));

    let too_small = HttpRangeOptions {
        max_source_bytes: u64::try_from(bytes.len() - 1).unwrap(),
        ..private_range_options()
    };
    assert_eq!(
        manager
            .open_selected_playback(
                &formats,
                too_small,
                MediaLimits::default(),
                MediaCancellation::new(),
            )
            .unwrap_err()
            .kind(),
        YoutubePlaybackErrorKind::InvalidOptions
    );

    let cancelled = MediaCancellation::new();
    cancelled.cancel();
    let request_count = media.requests().len();
    assert_eq!(
        manager
            .open_selected_playback(
                &formats,
                private_range_options(),
                MediaLimits::default(),
                cancelled,
            )
            .unwrap_err()
            .kind(),
        YoutubePlaybackErrorKind::Cancelled
    );
    assert_eq!(media.requests().len(), request_count);
}

#[test]
#[allow(clippy::too_many_lines)]
fn live_hls_handoff_reloads_at_deadlines_and_preserves_continuous_output() {
    let transport: Arc<[u8]> = fs::read(media_fixture("tone-aac-lc.ts")).unwrap().into();
    let media_reloads = Arc::new(AtomicUsize::new(0));
    let responder_reloads = Arc::clone(&media_reloads);
    let hls = ReplayServer::start(move |request, _| match request.target.as_str() {
        "/master.m3u8" => {
            ReplayResponse::json(b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=128000\nmedia.m3u8\n")
        }
        "/media.m3u8" => {
            let reload = responder_reloads.fetch_add(1, Ordering::AcqRel);
            if reload < 2 {
                ReplayResponse::json(
                    b"#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:10\n#EXTINF:1,First\na.ts\n",
                )
            } else {
                ReplayResponse::json(
                    b"#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-MEDIA-SEQUENCE:11\n#EXTINF:1,Second\nb.ts\n#EXT-X-ENDLIST\n",
                )
            }
        }
        "/a.ts" | "/b.ts" => ReplayResponse::json(&transport),
        _ => ReplayResponse::json(b"missing"),
    });
    let response = live_playback_response(&hls.url("master.m3u8"));
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let route_policy = Arc::new(LoopbackRoutePolicy {
        selections: AtomicUsize::new(0),
        outcomes: AtomicUsize::new(0),
    });
    let mut playback = manager
        .open_selected_live_playback_routed(
            &formats,
            private_live_options(),
            MediaCancellation::new(),
            route_policy.clone(),
        )
        .unwrap();
    assert_eq!(playback.mode(), YoutubePlaybackMode::Transcode);
    playback.set_filter_factory(Some(&SilenceFactory)).unwrap();

    let mut output = EncodedFrameSlot::new();
    let mut previous = None;
    let mut first_segment_frames = 0_usize;
    loop {
        match playback.poll_frame(Duration::ZERO, &mut output).unwrap() {
            YoutubeLivePlaybackPoll::Frame => {
                let timestamp = output.timestamp().unwrap();
                if let Some(previous) = previous {
                    assert_eq!(timestamp, previous + output.duration());
                } else {
                    assert_eq!(timestamp, Duration::ZERO);
                }
                previous = Some(timestamp);
                first_segment_frames += 1;
            }
            YoutubeLivePlaybackPoll::WaitUntil(deadline) => {
                assert_eq!(deadline, Duration::from_millis(200));
                break;
            }
            other => panic!("unexpected first-window outcome: {other:?}"),
        }
        assert!(first_segment_frames < 1_000);
    }
    assert!(first_segment_frames > 0);
    let requests_before_wait = hls.requests().len();
    assert_eq!(
        playback
            .poll_frame(Duration::from_millis(100), &mut output)
            .unwrap(),
        YoutubeLivePlaybackPoll::WaitUntil(Duration::from_millis(200))
    );
    assert_eq!(hls.requests().len(), requests_before_wait);

    let mut second_segment_frames = 0_usize;
    loop {
        match playback
            .poll_frame(Duration::from_millis(200), &mut output)
            .unwrap()
        {
            YoutubeLivePlaybackPoll::Frame => {
                let timestamp = output.timestamp().unwrap();
                assert_eq!(timestamp, previous.unwrap() + output.duration());
                previous = Some(timestamp);
                second_segment_frames += 1;
            }
            YoutubeLivePlaybackPoll::Ended => break,
            other => panic!("unexpected final-window outcome: {other:?}"),
        }
        assert!(second_segment_frames < 1_000);
    }
    assert!(second_segment_frames > 0);
    assert_eq!(media_reloads.load(Ordering::Acquire), 4);
    assert_eq!(
        playback
            .poll_frame(Duration::from_millis(200), &mut output)
            .unwrap(),
        YoutubeLivePlaybackPoll::Ended
    );
    assert!(route_policy.selections.load(Ordering::Acquire) >= 4);
    assert_eq!(
        route_policy.selections.load(Ordering::Acquire),
        route_policy.outcomes.load(Ordering::Acquire)
    );
}

#[test]
fn live_hls_handoff_rejects_finite_selection_and_preflight_cancellation() {
    let hls = ReplayServer::start(|_, _| ReplayResponse::json(b"unexpected"));
    let response = live_playback_response(&hls.url("master.m3u8?token=live-secret"));
    let api = ReplayServer::start(move |_, _| ReplayResponse::json(&response));
    let manager = playback_manager(&api);
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    let error = manager
        .open_selected_live_playback(&formats, private_live_options(), cancellation)
        .unwrap_err();
    assert_eq!(error.kind(), YoutubePlaybackErrorKind::Cancelled);
    assert!(!format!("{error:?}").contains("live-secret"));
    assert!(hls.requests().is_empty());

    let finite_bytes = fs::read(media_fixture("tone-aac-lc.m4a")).unwrap();
    let finite_media = RangeMediaServer::start(finite_bytes.clone());
    let finite_response = playback_response(
        &finite_media.url("audio.m4a"),
        "audio/mp4; codecs=\"mp4a.40.2\"",
        finite_bytes.len(),
    );
    let finite_api = ReplayServer::start(move |_, _| ReplayResponse::json(&finite_response));
    let finite_manager = playback_manager(&finite_api);
    let finite_formats = finite_manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    assert_eq!(
        finite_manager
            .open_selected_live_playback(
                &finite_formats,
                private_live_options(),
                MediaCancellation::new(),
            )
            .unwrap_err()
            .kind(),
        YoutubePlaybackErrorKind::IncompatibleFormat
    );
}

#[test]
#[ignore = "scheduled live-service metadata/playback-discovery smoke; not a normal PR gate"]
fn scheduled_live_video_and_playback_discovery_smoke() {
    let url = std::env::var("MANTLE_YOUTUBE_SMOKE_URL")
        .unwrap_or_else(|_| "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_owned());
    let options = YoutubeSourceOptions::default();
    let YoutubeRoute::Video(video_id) =
        route_youtube_identifier(&url, &options).expect("YouTube smoke URL")
    else {
        panic!("YouTube smoke URL must identify one video")
    };
    let manager =
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default()).unwrap();
    let loaded = manager
        .load(&SourceReference::new(Some(url), false))
        .unwrap()
        .expect("live YouTube video");
    let SourceLoad::Item(YoutubeSourceItem::Track(track)) = loaded else {
        panic!("expected one live YouTube track")
    };
    assert_eq!(track.info.identifier, video_id);
    assert!(!track.info.title.is_empty());
    assert!(
        !manager
            .discover_playback_formats(&video_id, &MediaCancellation::new())
            .unwrap()
            .formats()
            .is_empty()
    );
}

#[test]
#[ignore = "Deno-equipped CI executes the packaged no-permission EJS provider"]
fn deno_process_cipher_provider_executes_packaged_adapter_without_permissions() {
    let deno = std::env::var_os("MANTLE_DENO_BIN")
        .map(std::path::PathBuf::from)
        .expect("MANTLE_DENO_BIN must name the CI-installed Deno executable");
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/youtube-ejs/0.8.0/mantle-youtube-ejs.js");
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/synthetic.js"}"#);
        }
        if request.target == "/s/player/synthetic.js" {
            return ReplayResponse::json(include_bytes!("support/youtube_ejs_synthetic_player.js"));
        }
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"1000","audioChannels":2,"signatureCipher":"url=https%3A%2F%2Fmedia.example.test%2Faudio%3Fn%3Dthrottle-secret&sp=sig&s=cipher-secret"}]}}"#,
        )
    });
    let resolver =
        YoutubeProcessCipherResolver::deno(deno, adapter, YoutubeProcessCipherOptions::default())
            .unwrap();
    let manager = YoutubeAudioSourceManager::with_cipher_resolver(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            player_embed_url: server.url("embed/"),
            clients: vec![YoutubeClientKind::Web],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
        Arc::new(resolver),
    )
    .unwrap();
    let formats = manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    let resolved_url = manager
        .resolve_selected_playback_url(&formats, &MediaCancellation::new())
        .unwrap();

    assert!(resolved_url.as_str().contains("sig=terces-rehpic"));
    assert!(resolved_url.as_str().contains("n=n-throttle-secret"));
    assert_eq!(server.requests().len(), 3);
}

#[test]
#[ignore = "subprocess fixture invoked only by isolated process-provider tests"]
fn process_cipher_fixture_child() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let request: Value = serde_json::from_str(&input).unwrap();
    assert_eq!(request["version"], 1);
    assert!(request["maxOutputBytes"].as_u64().unwrap() >= 16);
    if request["playerScript"]
        .as_str()
        .unwrap()
        .contains("slowFixture")
    {
        thread::sleep(Duration::from_secs(2));
    }
    let signature = request["signature"]
        .as_str()
        .map(|value| format!("process-{value}"));
    let n_parameter = request["nParameter"]
        .as_str()
        .map(|value| format!("process-{value}"));
    println!(
        "MANTLE_YOUTUBE_CIPHER_V1\t{}",
        serde_json::json!({
            "version": 1,
            "signature": signature,
            "nParameter": n_parameter,
        })
    );
}

#[test]
fn player_script_acquisition_is_bounded_and_diagnostics_redact_its_url() {
    let server = ReplayServer::start(|request, _| {
        if request.target == "/embed/" {
            return ReplayResponse::json(br#"{"jsUrl":"/s/player/bounded.js"}"#);
        }
        ReplayResponse::json(b"var config={signatureTimestamp:20435};")
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            player_embed_url: server.url("embed/"),
            max_player_script_bytes: 16,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let error = manager
        .acquire_player_script(&MediaCancellation::new())
        .unwrap_err();
    assert_eq!(error.kind(), YoutubeErrorKind::InvalidResponse);

    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            player_embed_url: server.url("embed/"),
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let script = manager
        .acquire_player_script(&MediaCancellation::new())
        .unwrap();
    assert_eq!(script.signature_timestamp(), 20435);
    assert_eq!(script.byte_len(), 38);
    assert!(script.url().ends_with("/s/player/bounded.js"));
    let diagnostic = format!("{script:?}");
    assert!(!diagnostic.contains("bounded.js"));
    assert!(diagnostic.contains("<redacted>"));
}

#[test]
fn playback_format_and_live_content_length_limits_fail_closed() {
    let limit_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":250,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":64000,"contentLength":"100","url":"https://media.example.test/one"},{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"200","url":"https://media.example.test/two"}]}}"#,
        )
    });
    let limit_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: limit_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            max_playback_formats: 1,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        limit_manager
            .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidResponse
    );

    let url_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":false},"streamingData":{"adaptiveFormats":[{"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":128000,"contentLength":"200","url":"https://media.example.test/audio?token=too-long-for-policy"}]}}"#,
        )
    });
    let url_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: url_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            max_playback_url_bytes: 32,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        url_manager
            .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidResponse
    );

    let live_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","isLive":true},"streamingData":{"hlsManifestUrl":"https://media.example.test/live/master.m3u8","adaptiveFormats":[{"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":128000,"audioChannels":2,"url":"https://media.example.test/live"}]}}"#,
        )
    });
    let live_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: live_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let live = live_manager
        .discover_playback_formats("dQw4w9WgXcQ", &MediaCancellation::new())
        .unwrap();
    assert_eq!(
        live.selected().kind(),
        Some(YoutubePlaybackFormatKind::HlsMpegTsAac)
    );
    assert_eq!(live.selected().content_length(), None);
    assert!(!live.requires_player_script());
}

#[test]
fn playback_discovery_obeys_preflight_cancellation() {
    let server = ReplayServer::start(|_, _| ReplayResponse::json(br#"{"unexpected":"request"}"#));
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    let error = manager
        .discover_playback_formats("dQw4w9WgXcQ", &cancellation)
        .unwrap_err();
    assert_eq!(error.kind(), YoutubeErrorKind::Cancelled);
    assert_eq!(error.attempts(), 0);
    assert!(server.requests().is_empty());
}

#[test]
fn result_and_page_limits_fail_closed_without_overfetching() {
    let search_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"contents":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"compactVideoRenderer":{"videoId":"aaaaabbbbbb","title":{"simpleText":"First"},"shortBylineText":{"runs":[{"text":"Author"}]},"lengthText":{"simpleText":"0:01"}}},{"compactVideoRenderer":{"videoId":"ccccccddddd","title":{"simpleText":"Second"},"shortBylineText":{"runs":[{"text":"Author"}]},"lengthText":{"simpleText":"0:02"}}}]}}]}}}"#,
        )
    });
    let search_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: search_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            max_search_results: 1,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        search_manager.load(&SourceReference::new(
            Some("ytsearch:bounded".to_owned()),
            false,
        )),
        Err(SourceRegistryError::SourceFailure)
    );

    let playlist_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"header":{"playlistHeaderRenderer":{"title":{"runs":[{"text":"One page"}]}}},"contents":{"singleColumnBrowseResultsRenderer":{"tabs":[{"tabRenderer":{"content":{"sectionListRenderer":{"contents":[{"playlistVideoListRenderer":{"contents":[{"playlistVideoRenderer":{"videoId":"aaaaabbbbbb","title":{"simpleText":"First"},"shortBylineText":{"runs":[{"text":"Author"}]},"lengthSeconds":"1","isPlayable":true}},{"continuationItemRenderer":{"continuationEndpoint":{"continuationCommand":{"token":"must-not-fetch"}}}}]}}]}}}}]}}}"#,
        )
    });
    let playlist_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: playlist_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            max_playlist_pages: 1,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let Some(SourceLoad::Item(YoutubeSourceItem::Playlist(playlist))) = playlist_manager
        .load(&SourceReference::new(Some("PLbounded".to_owned()), false))
        .unwrap()
    else {
        panic!("expected the bounded first page");
    };
    assert_eq!(playlist.tracks.len(), 1);
    assert_eq!(playlist_server.requests().len(), 1);
}

#[test]
fn authentication_is_applied_but_never_appears_in_diagnostics() {
    let server = ReplayServer::start(|request, _| {
        // WEB is not an OAuth client. Its player metadata request must not receive the token.
        assert_eq!(request.header("authorization"), None);
        assert_eq!(request.header("x-goog-visitor-id"), Some("visitor-secret"));
        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            payload["serviceIntegrityDimensions"]["poToken"],
            "po-secret"
        );
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","title":"Authenticated","author":"Fixture","lengthSeconds":"1"}}"#,
        )
    });
    let authentication = YoutubeAuthentication::new(
        Some("oauth-secret".to_owned()),
        Some("po-secret".to_owned()),
        Some("visitor-secret".to_owned()),
    )
    .unwrap();
    let diagnostic = format!("{authentication:?}");
    assert!(diagnostic.contains("oauth: true"), "{diagnostic}");
    for secret in ["oauth-secret", "po-secret", "visitor-secret"] {
        assert!(!diagnostic.contains(secret), "{diagnostic}");
    }
    let options = YoutubeSourceOptions {
        api_base_url: server.url("youtubei/v1"),
        clients: vec![YoutubeClientKind::Web],
        http: private_http_options(),
        ..YoutubeSourceOptions::default()
    };
    let manager = YoutubeAudioSourceManager::new(options, authentication).unwrap();
    assert!(
        manager
            .load(&SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false))
            .unwrap()
            .is_some()
    );
}

#[test]
fn authentication_and_source_policy_reject_invalid_bounds() {
    let error = YoutubeAuthentication::new(None, Some("po-secret".to_owned()), None).unwrap_err();
    assert_eq!(error.kind(), YoutubeErrorKind::InvalidAuthentication);
    assert!(!format!("{error:?}").contains("po-secret"));

    let oversized = "s".repeat(16 * 1024 + 1);
    assert_eq!(
        YoutubeAuthentication::new(Some(oversized), None, None)
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidAuthentication
    );
    assert_eq!(
        YoutubeAuthentication::with_refresh_token(String::new(), None, None)
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidAuthentication
    );

    let options = YoutubeSourceOptions {
        oauth: YoutubeOAuthOptions {
            max_response_bytes: 0,
            ..YoutubeOAuthOptions::default()
        },
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_clients: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_playlist_pages: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_mix_tracks: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_playback_formats: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_playback_url_bytes: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );

    let options = YoutubeSourceOptions {
        max_cipher_operations: 0,
        ..YoutubeSourceOptions::default()
    };
    assert_eq!(
        YoutubeAudioSourceManager::new(options, YoutubeAuthentication::default())
            .unwrap_err()
            .kind(),
        YoutubeErrorKind::InvalidOptions
    );
}

#[test]
fn player_metadata_enforces_string_and_thumbnail_bounds() {
    let title_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","title":"too-long-title","author":"A","lengthSeconds":"1"}}"#,
        )
    });
    let title_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: title_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::Web],
            max_metadata_string_bytes: 8,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        title_manager.load(&SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false)),
        Err(SourceRegistryError::SourceFailure)
    );

    let thumbnail_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","title":"T","author":"A","lengthSeconds":"1","thumbnail":{"thumbnails":[{"url":"https://one"},{"url":"https://two"}]}}}"#,
        )
    });
    let thumbnail_manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: thumbnail_server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::Web],
            max_thumbnails: 1,
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    assert_eq!(
        thumbnail_manager.load(&SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false)),
        Err(SourceRegistryError::SourceFailure)
    );
}

#[test]
fn live_metadata_and_preflight_cancellation_are_deterministic() {
    let server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ","title":"Live fixture","author":"Fixture","isLive":true}}"#,
        )
    });
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: server.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let cancellation = SourceCancellation::new();
    cancellation.cancel();
    assert_eq!(
        manager
            .load_with_cancellation(
                &SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false),
                &cancellation,
            )
            .unwrap(),
        None
    );
    assert!(server.requests().is_empty());

    let Some(SourceLoad::Item(YoutubeSourceItem::Track(track))) = manager
        .load(&SourceReference::new(Some("dQw4w9WgXcQ".to_owned()), false))
        .unwrap()
    else {
        panic!("expected live track metadata");
    };
    assert!(track.info.is_stream);
    assert_eq!(track.info.duration, Duration::ZERO);
}

#[test]
fn youtube_track_details_are_empty_and_reconstruct_from_outer_info() {
    let manager = YoutubeAudioSourceManager::new(
        YoutubeSourceOptions::default(),
        YoutubeAuthentication::default(),
    )
    .unwrap();
    let info = TrackInfo {
        title: "Decoded".to_owned(),
        author: "Author".to_owned(),
        duration: Duration::from_secs(9),
        identifier: "dQw4w9WgXcQ".to_owned(),
        is_stream: false,
        uri: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_owned()),
        artwork_url: None,
        isrc: None,
    };
    let item = YoutubeSourceItem::Track(mantle_media::YoutubeSourceTrack { info: info.clone() });
    assert_eq!(manager.encode(&item).unwrap(), Vec::<u8>::new());
    assert_eq!(
        manager.decode_with_info(&info, &[]).unwrap(),
        YoutubeSourceItem::Track(mantle_media::YoutubeSourceTrack { info })
    );
    assert!(
        manager
            .decode_with_info(
                &TrackInfo {
                    identifier: "bad".to_owned(),
                    ..match item {
                        YoutubeSourceItem::Track(track) => track.info,
                        YoutubeSourceItem::Playlist(_) => unreachable!(),
                    }
                },
                b"unexpected",
            )
            .is_err()
    );
}

fn private_http_options() -> mantle_media::RemoteHttpOptions {
    mantle_media::RemoteHttpOptions {
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        retry_base_delay: Duration::from_millis(1),
        retry_max_delay: Duration::from_millis(2),
        ..mantle_media::RemoteHttpOptions::default()
    }
}

fn private_range_options() -> HttpRangeOptions {
    HttpRangeOptions {
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        ..HttpRangeOptions::default()
    }
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

fn playback_manager(api: &ReplayServer) -> YoutubeAudioSourceManager {
    YoutubeAudioSourceManager::new(
        YoutubeSourceOptions {
            api_base_url: api.url("youtubei/v1"),
            clients: vec![YoutubeClientKind::AndroidVr],
            http: private_http_options(),
            ..YoutubeSourceOptions::default()
        },
        YoutubeAuthentication::default(),
    )
    .unwrap()
}

fn playback_response(url: &str, mime_type: &str, content_length: usize) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "playabilityStatus": {"status": "OK"},
        "videoDetails": {"videoId": "dQw4w9WgXcQ", "isLive": false},
        "streamingData": {
            "adaptiveFormats": [{
                "itag": 251,
                "mimeType": mime_type,
                "bitrate": 128_000,
                "contentLength": content_length.to_string(),
                "audioChannels": 2,
                "url": url,
            }]
        }
    }))
    .unwrap()
}

fn live_playback_response(manifest_url: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "playabilityStatus": {"status": "OK"},
        "videoDetails": {"videoId": "dQw4w9WgXcQ", "isLive": true},
        "streamingData": {
            "hlsManifestUrl": manifest_url,
        }
    }))
    .unwrap()
}

fn private_live_options() -> YoutubeLivePlaybackOptions {
    let segment = HttpStreamOptions {
        max_response_bytes: 128 * 1_024,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpStreamOptions::default()
    };
    let playlist_http = HttpStreamOptions {
        max_response_bytes: 4 * 1_024,
        ..segment
    };
    YoutubeLivePlaybackOptions {
        playlist: HttpPlaylistOptions {
            http: playlist_http,
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

#[derive(Clone)]
struct ReplayRequest {
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ReplayRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find_map(|(candidate, value)| {
            candidate
                .eq_ignore_ascii_case(name)
                .then_some(value.as_str())
        })
    }
}

struct ReplayResponse {
    body: Vec<u8>,
}

impl ReplayResponse {
    fn json(body: &[u8]) -> Self {
        Self {
            body: body.to_vec(),
        }
    }
}

struct ReplayServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<ReplayRequest>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ReplayServer {
    fn start(
        responder: impl Fn(ReplayRequest, usize) -> ReplayResponse + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let responder = Arc::new(responder);
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve(stream, &thread_requests, responder.as_ref()),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}/{path}", self.address)
    }

    fn requests(&self) -> Vec<ReplayRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for ReplayServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    requests: &Mutex<Vec<ReplayRequest>>,
    responder: &(dyn Fn(ReplayRequest, usize) -> ReplayResponse + Send + Sync),
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut raw = Vec::with_capacity(4096);
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let Ok(count) = stream.read(&mut buffer) else {
            return;
        };
        if count == 0 {
            return;
        }
        raw.extend_from_slice(&buffer[..count]);
        if let Some(position) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
        if raw.len() > 16 * 1024 {
            return;
        }
    };
    let header_text = String::from_utf8_lossy(&raw[..header_end]);
    let target = header_text
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .unwrap()
        .to_owned();
    let headers: Vec<_> = header_text
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let content_len = headers
        .iter()
        .find_map(|(name, value)| {
            (name == "content-length")
                .then(|| value.parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    while raw.len() - header_end < content_len {
        let Ok(count) = stream.read(&mut buffer) else {
            return;
        };
        if count == 0 {
            return;
        }
        raw.extend_from_slice(&buffer[..count]);
    }
    let request = ReplayRequest {
        target,
        headers,
        body: raw[header_end..header_end + content_len].to_vec(),
    };
    let count = requests.lock().unwrap().len();
    requests.lock().unwrap().push(request.clone());
    let response = responder(request, count);
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.body.len()
    );
    let _ = stream.write_all(&response.body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

struct RangeMediaServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<(u64, u64)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RangeMediaServer {
    fn start(bytes: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let bytes: Arc<[u8]> = bytes.into();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve_media_range(stream, &bytes, &thread_requests),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}/{path}", self.address)
    }

    fn requests(&self) -> Vec<(u64, u64)> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for RangeMediaServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn serve_media_range(mut stream: TcpStream, bytes: &[u8], requests: &Mutex<Vec<(u64, u64)>>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request = [0_u8; 16 * 1024];
    let mut used = 0_usize;
    while used < request.len() {
        let Ok(count) = stream.read(&mut request[used..]) else {
            return;
        };
        if count == 0 {
            return;
        }
        used += count;
        if request[..used]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            break;
        }
    }
    let request = String::from_utf8_lossy(&request[..used]);
    let Some((start, requested_end)) = request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| name.eq_ignore_ascii_case("range").then_some(value.trim()))
        .and_then(|value| value.strip_prefix("bytes="))
        .and_then(|value| value.split_once('-'))
        .and_then(|(start, end)| Some((start.parse().ok()?, end.parse().ok()?)))
    else {
        return;
    };
    requests.lock().unwrap().push((start, requested_end));
    if start >= bytes.len() as u64 {
        return;
    }
    let end = requested_end.min(bytes.len() as u64 - 1);
    let (Ok(start_index), Ok(end_index)) = (usize::try_from(start), usize::try_from(end)) else {
        return;
    };
    let body = &bytes[start_index..=end_index];
    let _ = write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
        body.len(),
        bytes.len()
    );
    let _ = stream.write_all(body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}
