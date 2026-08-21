#[path = "support/http_replay.rs"]
mod http_replay;
#[path = "support/range_media.rs"]
mod range_media;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use http_replay::{ReplayResponse, ReplayServer};
use mantle_core::{SourceCancellation, SourceLoad, SourceManager, SourceReference, TrackInfo};
use mantle_media::{
    Codec, Container, HttpNetworkAccess, HttpRangeOptions, MediaCancellation, MediaLimits,
    PcmFrame, RemoteHttpOptions, YandexMusicAuthentication, YandexMusicErrorKind,
    YandexMusicPlaybackErrorKind, YandexMusicPlaybackScheme, YandexMusicPlaylistKind,
    YandexMusicRoute, YandexMusicSourceItem, YandexMusicSourceManager, YandexMusicSourceOptions,
    route_yandex_music_identifier,
};
use range_media::RangeMediaServer;
use serde_json::json;

#[test]
fn routes_current_track_album_playlist_search_and_recommendation_inputs_strictly() {
    let options = YandexMusicSourceOptions::default();
    assert_eq!(
        route_yandex_music_identifier("ymsearch:  animals architects ", &options),
        Some(YandexMusicRoute::Search("animals architects".to_owned()))
    );
    assert_eq!(
        route_yandex_music_identifier("ymrec:71663565", &options),
        Some(YandexMusicRoute::Recommendations("71663565".to_owned()))
    );
    assert_eq!(
        route_yandex_music_identifier(
            "https://music.yandex.ru/album/13886032/track/71663565?from=fixture",
            &options,
        ),
        Some(YandexMusicRoute::Track {
            track_id: "71663565".to_owned(),
            album_id: Some("13886032".to_owned()),
            domain: "ru".to_owned(),
        })
    );
    assert_eq!(
        route_yandex_music_identifier("music.yandex.com/track/71663565", &options),
        Some(YandexMusicRoute::Track {
            track_id: "71663565".to_owned(),
            album_id: None,
            domain: "com".to_owned(),
        })
    );
    assert_eq!(
        route_yandex_music_identifier("https://music.yandex.kz/album/13886032", &options),
        Some(YandexMusicRoute::Album {
            album_id: "13886032".to_owned(),
            domain: "kz".to_owned(),
        })
    );
    assert_eq!(
        route_yandex_music_identifier(
            "https://music.yandex.by/users/yamusic-bestsongs/playlists/701626",
            &options,
        ),
        Some(YandexMusicRoute::Playlist {
            owner: Some("yamusic-bestsongs".to_owned()),
            playlist_id: "701626".to_owned(),
            domain: "by".to_owned(),
        })
    );
    assert_eq!(
        route_yandex_music_identifier(
            "https://music.yandex.ru/playlists/e1bb61b5-360d-e3c5-124c-ef58d981ca7d",
            &options,
        ),
        Some(YandexMusicRoute::Playlist {
            owner: None,
            playlist_id: "e1bb61b5-360d-e3c5-124c-ef58d981ca7d".to_owned(),
            domain: "ru".to_owned(),
        })
    );
    for rejected in [
        "https://music.yandex.test/track/71663565",
        "https://token@music.yandex.ru/track/71663565",
        "https://music.yandex.ru/track/not-a-number",
        "https://music.yandex.ru/track/71663565/extra",
        "ymrec:not-a-number",
        "ymsearch:   ",
    ] {
        assert_eq!(route_yandex_music_identifier(rejected, &options), None);
    }
}

#[test]
fn authenticated_track_replay_builds_current_metadata_without_exposing_credentials() {
    let response = serde_json::to_vec(&json!({
        "result": [{
            "available": true,
            "id": "71663565",
            "title": "Animals",
            "durationMs": 244_321,
            "artists": [{"name": "Architects"}, {"name": "Fixture Artist"}],
            "albums": [{"id": "13886032", "title": "For Those That Wish to Exist"}],
            "coverUri": "avatars.yandex.net/get-music-content/fixture/%%"
        }]
    }))
    .unwrap();
    let server = ReplayServer::start(move |request, _| {
        assert_eq!(request.target, "/tracks/71663565");
        assert_eq!(request.header("accept"), Some("application/json"));
        assert_eq!(request.header("authorization"), Some("OAuth token-secret"));
        assert_eq!(request.header("user-agent"), Some("Yandex-Music-API"));
        assert_eq!(
            request.header("x-yandex-music-client"),
            Some("YandexMusicAndroid/24023621")
        );
        ReplayResponse::json(response.clone())
    });
    let manager = manager(&server, "token-secret");
    let track = manager
        .load_track_metadata("71663565", "ru", &MediaCancellation::new())
        .unwrap()
        .expect("available track");
    assert_eq!(track.info.title, "Animals");
    assert_eq!(track.info.author, "Architects, Fixture Artist");
    assert_eq!(track.info.duration, Duration::from_millis(244_321));
    assert_eq!(track.info.identifier, "71663565");
    assert_eq!(
        track.info.uri.as_deref(),
        Some("https://music.yandex.ru/track/71663565")
    );
    assert_eq!(
        track.info.artwork_url.as_deref(),
        Some("https://avatars.yandex.net/get-music-content/fixture/400x400")
    );
    for diagnostic in [
        format!("{manager:?}"),
        format!("{:?}", manager.authentication()),
    ] {
        assert!(!diagnostic.contains("token-secret"), "{diagnostic}");
        assert!(!diagnostic.contains(&server.url("")), "{diagnostic}");
    }

    let load = manager
        .load(&SourceReference::new(
            Some("https://music.yandex.ru/track/71663565".to_owned()),
            false,
        ))
        .unwrap()
        .expect("recognized source route");
    let SourceLoad::Item(YandexMusicSourceItem::Track(loaded)) = load else {
        panic!("expected native Yandex Music track");
    };
    assert_eq!(loaded, track);
    assert_eq!(manager.source_name(), "yandex-music");
}

#[test]
fn authentication_response_bounds_cancellation_and_source_details_fail_closed() {
    assert_eq!(
        YandexMusicAuthentication::new("").unwrap_err().kind(),
        YandexMusicErrorKind::InvalidAuthentication
    );
    assert_eq!(
        YandexMusicAuthentication::new("x".repeat(16 * 1024 + 1))
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidAuthentication
    );

    let policy_server = ReplayServer::start(|_, _| ReplayResponse::json(b"{}".to_vec()));
    let mut invalid_options = options(&policy_server);
    invalid_options.max_artists = 0;
    assert_eq!(
        YandexMusicSourceManager::new(
            invalid_options,
            YandexMusicAuthentication::new("token-secret").unwrap(),
        )
        .unwrap_err()
        .kind(),
        YandexMusicErrorKind::InvalidOptions
    );
    assert!(policy_server.requests().is_empty());

    let unauthorized = ReplayServer::start(|_, _| ReplayResponse::status(401));
    assert_eq!(
        manager(&unauthorized, "token-secret")
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::AuthenticationRequired
    );

    let malformed = ReplayServer::start(|_, _| ReplayResponse::json(br#"{"result":{}}"#.to_vec()));
    assert_eq!(
        manager(&malformed, "token-secret")
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let oversized = ReplayServer::start(|_, _| ReplayResponse::json(vec![b'x'; 65]));
    let mut options = options(&oversized);
    options.max_response_bytes = 64;
    let bounded = YandexMusicSourceManager::new(
        options,
        YandexMusicAuthentication::new("token-secret").unwrap(),
    )
    .unwrap();
    assert_eq!(
        bounded
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let cancelled_server = ReplayServer::start(|_, _| ReplayResponse::json(b"{}".to_vec()));
    let cancelled = MediaCancellation::new();
    cancelled.cancel();
    assert_eq!(
        manager(&cancelled_server, "token-secret")
            .load_track_metadata("71663565", "ru", &cancelled)
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::Cancelled
    );
    assert!(cancelled_server.requests().is_empty());

    let manager = manager(&cancelled_server, "token-secret");
    let info = TrackInfo {
        title: "Fixture".to_owned(),
        author: "Artist".to_owned(),
        duration: Duration::from_secs(1),
        identifier: "71663565".to_owned(),
        is_stream: false,
        uri: Some("https://music.yandex.ru/track/71663565".to_owned()),
        artwork_url: None,
        isrc: None,
    };
    let item =
        YandexMusicSourceItem::Track(mantle_media::YandexMusicSourceTrack { info: info.clone() });
    assert_eq!(manager.encode(&item).unwrap(), Vec::<u8>::new());
    assert!(matches!(
        manager.decode_with_info(&info, &[]).unwrap(),
        YandexMusicSourceItem::Track(track) if track.info == info
    ));
    assert!(manager.decode_with_info(&info, &[1]).is_err());

    let source_cancelled = SourceCancellation::new();
    source_cancelled.cancel();
    assert!(
        manager
            .load_with_cancellation(
                &SourceReference::new(
                    Some("https://music.yandex.ru/track/71663565".to_owned()),
                    false,
                ),
                &source_cancelled,
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn unavailable_mismatched_and_oversized_metadata_do_not_produce_partial_tracks() {
    let unavailable = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"result":[{"available":false,"id":"71663565","title":"Hidden","durationMs":1}]}"#
                .to_vec(),
        )
    });
    assert!(
        manager(&unavailable, "token-secret")
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap()
            .is_none()
    );

    let mismatched = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [{
                    "available": true,
                    "id": "999",
                    "title": "Wrong track",
                    "durationMs": 1,
                    "artists": [{"name": "Fixture"}]
                }]
            }))
            .unwrap(),
        )
    });
    assert_eq!(
        manager(&mismatched, "token-secret")
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let oversized_title = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [{
                    "available": true,
                    "id": "71663565",
                    "title": "title-too-long",
                    "durationMs": 1,
                    "artists": [{"name": "Fixture"}]
                }]
            }))
            .unwrap(),
        )
    });
    let mut bounded_options = options(&oversized_title);
    bounded_options.max_metadata_string_bytes = 8;
    let bounded = YandexMusicSourceManager::new(
        bounded_options,
        YandexMusicAuthentication::new("token-secret").unwrap(),
    )
    .unwrap();
    assert_eq!(
        bounded
            .load_track_metadata("71663565", "ru", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let empty_album = ReplayServer::start(|_, _| ReplayResponse::json(b"{}".to_vec()));
    let empty_album_manager = manager(&empty_album, "token-secret");
    assert!(matches!(
        empty_album_manager
            .load(&SourceReference::new(
                Some("https://music.yandex.ru/album/13886032".to_owned()),
                false,
            ))
            .unwrap(),
        Some(SourceLoad::Referral(_))
    ));
    assert_eq!(empty_album.requests().len(), 1);
}

#[test]
fn album_and_artist_replays_preserve_bounded_collection_metadata_and_order() {
    let server = ReplayServer::start(|request, _| {
        match request.target.as_str() {
        "/albums/13886032/with-tracks?page-size=300" => ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {
                    "title": "For Those That Wish to Exist",
                    "artists": [{"name": "Architects"}],
                    "coverUri": "avatars.yandex.net/album/%%",
                    "volumes": [[track_json("1", "First")], [track_json("2", "Second")]]
                }
            }))
            .unwrap(),
        ),
        "/artists/42/tracks?page-size=60" => ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {"tracks": [track_json("3", "Artist first"), track_json("4", "Artist second")]}
            }))
            .unwrap(),
        ),
        "/artists/42" => ReplayResponse::json(
            br#"{"result":{"artist":{"name":"Architects","coverUri":"avatars.yandex.net/artist/%%"}}}"#.to_vec(),
        ),
        target => panic!("unexpected target {target}"),
    }
    });
    let manager = manager(&server, "token-secret");

    let album = manager
        .load(&SourceReference::new(
            Some("https://music.yandex.kz/album/13886032".to_owned()),
            false,
        ))
        .unwrap()
        .expect("album result");
    let SourceLoad::Item(YandexMusicSourceItem::Playlist(album)) = album else {
        panic!("expected album playlist");
    };
    assert_eq!(album.kind, YandexMusicPlaylistKind::Album);
    assert_eq!(album.name, "For Those That Wish to Exist");
    assert_eq!(album.author.as_deref(), Some("Architects"));
    assert_eq!(album.tracks.len(), 2);
    assert_eq!(album.tracks[0].info.identifier, "1");
    assert_eq!(album.tracks[1].info.identifier, "2");
    assert_eq!(
        album.uri.as_deref(),
        Some("https://music.yandex.kz/album/13886032")
    );

    let artist = manager
        .load_route(
            &YandexMusicRoute::Artist {
                artist_id: "42".to_owned(),
                domain: "ru".to_owned(),
            },
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("artist result");
    let YandexMusicSourceItem::Playlist(artist) = artist else {
        panic!("expected artist playlist");
    };
    assert_eq!(artist.kind, YandexMusicPlaylistKind::Artist);
    assert_eq!(artist.name, "Architects's Top Tracks");
    assert_eq!(artist.author.as_deref(), Some("Architects"));
    assert_eq!(artist.tracks.len(), 2);
    assert_eq!(server.requests().len(), 3);
}

#[test]
fn user_and_uuid_playlist_replays_accept_wrapped_tracks_and_liked_song_names() {
    let server = ReplayServer::start(|request, _| match request.target.as_str() {
        "/users/fixture-user/playlists/7?page-size=600&rich-tracks=true" => ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {
                    "kind": "3",
                    "title": "Ignored liked title",
                    "owner": {"name": "Fixture Owner", "login": "fixture-user"},
                    "tracks": [{"track": track_json("5", "Liked")}]
                }
            }))
            .unwrap(),
        ),
        "/playlist/e1bb61b5-360d-e3c5-124c-ef58d981ca7d?page-size=600&rich-tracks=true" => {
            ReplayResponse::json(
                serde_json::to_vec(&json!({
                    "result": {
                        "kind": 9,
                        "title": "UUID fixture",
                        "owner": {"login": "uuid-owner"},
                        "tracks": [track_json("6", "UUID track")]
                    }
                }))
                .unwrap(),
            )
        }
        target => panic!("unexpected target {target}"),
    });
    let manager = manager(&server, "token-secret");
    let user = manager
        .load_route(
            &YandexMusicRoute::Playlist {
                owner: Some("fixture-user".to_owned()),
                playlist_id: "7".to_owned(),
                domain: "by".to_owned(),
            },
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("user playlist");
    let YandexMusicSourceItem::Playlist(user) = user else {
        panic!("expected user playlist");
    };
    assert_eq!(user.name, "Fixture Owner's liked songs");
    assert_eq!(user.author.as_deref(), Some("Fixture Owner"));
    assert_eq!(user.tracks[0].info.identifier, "5");
    assert_eq!(user.kind, YandexMusicPlaylistKind::Playlist);

    let uuid = manager
        .load_route(
            &YandexMusicRoute::Playlist {
                owner: None,
                playlist_id: "e1bb61b5-360d-e3c5-124c-ef58d981ca7d".to_owned(),
                domain: "com".to_owned(),
            },
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("UUID playlist");
    let YandexMusicSourceItem::Playlist(uuid) = uuid else {
        panic!("expected UUID playlist");
    };
    assert_eq!(uuid.name, "UUID fixture");
    assert_eq!(uuid.author.as_deref(), Some("uuid-owner"));
    assert_eq!(
        uuid.uri.as_deref(),
        Some("https://music.yandex.com/playlists/e1bb61b5-360d-e3c5-124c-ef58d981ca7d")
    );
}

#[test]
fn search_and_recommendation_replays_use_form_encoding_and_search_classification() {
    let server = ReplayServer::start(|request, _| match request.target.as_str() {
        "/search?text=animals+%26+architects&type=track&page=0" => ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {"tracks": {"results": [track_json("7", "Search result")]}}
            }))
            .unwrap(),
        ),
        "/tracks/71663565/similar" => ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {"similarTracks": [track_json("8", "Recommendation")]}
            }))
            .unwrap(),
        ),
        target => panic!("unexpected target {target}"),
    });
    let manager = manager(&server, "token-secret");
    let search = manager
        .load_route(
            &YandexMusicRoute::Search("animals & architects".to_owned()),
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("search result");
    let YandexMusicSourceItem::Playlist(search) = search else {
        panic!("expected search playlist");
    };
    assert_eq!(search.kind, YandexMusicPlaylistKind::Search);
    assert!(search.is_search_result);
    assert_eq!(search.name, "Yandex Music Search: animals & architects");
    assert_eq!(search.tracks[0].info.identifier, "7");

    let recommendations = manager
        .load_route(
            &YandexMusicRoute::Recommendations("71663565".to_owned()),
            &MediaCancellation::new(),
        )
        .unwrap()
        .expect("recommendations");
    let YandexMusicSourceItem::Playlist(recommendations) = recommendations else {
        panic!("expected recommendations playlist");
    };
    assert_eq!(
        recommendations.kind,
        YandexMusicPlaylistKind::Recommendations
    );
    assert!(!recommendations.is_search_result);
    assert_eq!(recommendations.name, "Yandex Music Recommendations");
    assert_eq!(recommendations.tracks[0].info.identifier, "8");
}

#[test]
fn collection_limits_and_empty_region_results_fail_without_partial_playlists() {
    let server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": {"tracks": {"results": [track_json("9", "One"), track_json("10", "Two")]}}
            }))
            .unwrap(),
        )
    });
    let mut limited_options = options(&server);
    limited_options.max_collection_tracks = 1;
    let limited = YandexMusicSourceManager::new(
        limited_options,
        YandexMusicAuthentication::new("token-secret").unwrap(),
    )
    .unwrap();
    assert_eq!(
        limited
            .load_route(
                &YandexMusicRoute::Search("fixture".to_owned()),
                &MediaCancellation::new(),
            )
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let empty = ReplayServer::start(|_, _| {
        ReplayResponse::json(br#"{"result":{"similarTracks":[]}}"#.to_vec())
    });
    assert!(
        manager(&empty, "token-secret")
            .load_route(
                &YandexMusicRoute::Recommendations("71663565".to_owned()),
                &MediaCancellation::new(),
            )
            .unwrap()
            .is_none()
    );

    let mut invalid_options = options(&empty);
    invalid_options.max_collection_pages = 0;
    assert_eq!(
        YandexMusicSourceManager::new(
            invalid_options,
            YandexMusicAuthentication::new("token-secret").unwrap(),
        )
        .unwrap_err()
        .kind(),
        YandexMusicErrorKind::InvalidOptions
    );
}

#[test]
fn playback_replay_selects_the_best_mp3_and_builds_the_pinned_md5_url() {
    let media = RangeMediaServer::start(Vec::new());
    let xml = format!(
        "<download-info><host>{}</host><path>/fixture/audio.mp3</path><ts>1700000000</ts><s>fixture-secret</s></download-info>",
        media.authority()
    );
    let xml_server = ReplayServer::start(move |request, _| {
        assert_eq!(request.target, "/download-info.xml?secret=xml-secret");
        assert_eq!(request.header("authorization"), Some("OAuth token-secret"));
        ReplayResponse::json(xml.clone().into_bytes())
    });
    let xml_url = xml_server.url("download-info.xml?secret=xml-secret");
    let api = ReplayServer::start(move |request, _| {
        assert_eq!(request.target, "/tracks/71663565/download-info");
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [
                    {"codec": "aac", "bitrateInKbps": 999, "downloadInfoUrl": "https://ignored.example/aac"},
                    {"codec": "mp3", "bitrateInKbps": 128, "downloadInfoUrl": xml_url},
                    {"codec": "mp3", "bitrateInKbps": 320, "downloadInfoUrl": xml_url}
                ]
            }))
            .unwrap(),
        )
    });
    let manager = playback_manager(&api, "token-secret");
    let resolved = manager
        .resolve_track_playback("71663565", &MediaCancellation::new())
        .unwrap()
        .expect("MP3 playback URL");
    assert_eq!(resolved.bitrate_kbps(), 320);
    assert_eq!(
        resolved.as_str(),
        format!(
            "http://{}/get-mp3/2283497a37ecd218cc61b7bd0d335004/1700000000/fixture/audio.mp3",
            media.authority()
        )
    );
    let diagnostic = format!("{resolved:?}");
    assert!(!diagnostic.contains("fixture-secret"));
    assert!(!diagnostic.contains("xml-secret"));
}

#[test]
fn resolved_mp3_opens_through_bounded_range_input_and_decodes_pcm() {
    let bytes = fs::read(media_fixture("tone-mp3-vbr-id3.mp3")).unwrap();
    let media = RangeMediaServer::start(bytes);
    let xml = format!(
        "<download-info><host>{}</host><path>/fixture/audio.mp3</path><ts>1700000000</ts><s>fixture-secret</s></download-info>",
        media.authority()
    );
    let xml_server =
        ReplayServer::start(move |_, _| ReplayResponse::json(xml.clone().into_bytes()));
    let xml_url = xml_server.url("download-info.xml");
    let api = ReplayServer::start(move |_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [{"codec": "mp3", "bitrateInKbps": 192, "downloadInfoUrl": xml_url}]
            }))
            .unwrap(),
        )
    });
    let manager = playback_manager(&api, "token-secret");
    let mut session = manager
        .open_track_playback(
            "71663565",
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap()
        .expect("available MP3 playback");
    assert_eq!(session.info().container, Container::Mp3);
    assert_eq!(session.info().codec, Codec::Mp3);
    let mut pcm = PcmFrame::with_capacity(256 * 1024);
    assert!(session.read_pcm(&mut pcm).unwrap());
    assert!(!pcm.samples().is_empty());
    assert!(!media.requests().is_empty());
}

#[test]
fn playback_resolution_rejects_candidate_xml_origin_and_cancellation_failures() {
    let no_mp3 = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"result":[{"codec":"aac","bitrateInKbps":192,"downloadInfoUrl":"https://ignored.example/aac"}]}"#.to_vec(),
        )
    });
    assert!(
        playback_manager(&no_mp3, "token-secret")
            .resolve_track_playback("71663565", &MediaCancellation::new())
            .unwrap()
            .is_none()
    );

    let bad_origin = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            br#"{"result":[{"codec":"mp3","bitrateInKbps":192,"downloadInfoUrl":"https://credentials.example.test/secret"}]}"#.to_vec(),
        )
    });
    assert_eq!(
        playback_manager(&bad_origin, "token-secret")
            .resolve_track_playback("71663565", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    for malformed in [
        "<download-info><host>media.example</host><path>/a.mp3</path><ts>1</ts></download-info>",
        "<download-info><host>media.example</host><host>duplicate.example</host><path>/a.mp3</path><ts>1</ts><s>x</s></download-info>",
        "<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///etc/passwd'>]><download-info><host>&e;</host><path>/a.mp3</path><ts>1</ts><s>x</s></download-info>",
    ] {
        let xml = malformed.as_bytes().to_vec();
        let xml_server = ReplayServer::start(move |_, _| ReplayResponse::json(xml.clone()));
        let xml_url = xml_server.url("bad.xml");
        let api = ReplayServer::start(move |_, _| {
            ReplayResponse::json(
                serde_json::to_vec(&json!({
                    "result": [{"codec": "mp3", "bitrateInKbps": 192, "downloadInfoUrl": xml_url}]
                }))
                .unwrap(),
            )
        });
        assert_eq!(
            playback_manager(&api, "token-secret")
                .resolve_track_playback("71663565", &MediaCancellation::new())
                .unwrap_err()
                .kind(),
            YandexMusicErrorKind::InvalidResponse
        );
    }

    let cancelled_server = ReplayServer::start(|_, _| ReplayResponse::json(b"{}".to_vec()));
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert_eq!(
        playback_manager(&cancelled_server, "token-secret")
            .resolve_track_playback("71663565", &cancellation)
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::Cancelled
    );
    assert!(cancelled_server.requests().is_empty());
}

#[test]
fn playback_bounds_and_media_mismatch_fail_without_exposing_urls() {
    let xml_server = ReplayServer::start(|_, _| {
        ReplayResponse::json(
            b"<download-info><host>127.0.0.1:9</host><path>/a.mp3</path><ts>1</ts><s>x</s></download-info>"
                .to_vec(),
        )
    });
    let xml_url = xml_server.url("bounded.xml");
    let api = ReplayServer::start(move |_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [
                    {"codec": "mp3", "bitrateInKbps": 128, "downloadInfoUrl": xml_url},
                    {"codec": "mp3", "bitrateInKbps": 192, "downloadInfoUrl": xml_url}
                ]
            }))
            .unwrap(),
        )
    });
    let mut limited_options = playback_options(&api);
    limited_options.max_download_candidates = 1;
    let limited = YandexMusicSourceManager::new(
        limited_options,
        YandexMusicAuthentication::new("token-secret").unwrap(),
    )
    .unwrap();
    assert_eq!(
        limited
            .resolve_track_playback("71663565", &MediaCancellation::new())
            .unwrap_err()
            .kind(),
        YandexMusicErrorKind::InvalidResponse
    );

    let not_mp3 = RangeMediaServer::start(fs::read(media_fixture("tone-aac-lc.m4a")).unwrap());
    let xml = format!(
        "<download-info><host>{}</host><path>/fixture/not-mp3</path><ts>1</ts><s>x</s></download-info>",
        not_mp3.authority()
    );
    let xml_server =
        ReplayServer::start(move |_, _| ReplayResponse::json(xml.clone().into_bytes()));
    let xml_url = xml_server.url("mismatch.xml");
    let api = ReplayServer::start(move |_, _| {
        ReplayResponse::json(
            serde_json::to_vec(&json!({
                "result": [{"codec": "mp3", "bitrateInKbps": 192, "downloadInfoUrl": xml_url}]
            }))
            .unwrap(),
        )
    });
    let error = playback_manager(&api, "token-secret")
        .open_track_playback(
            "71663565",
            private_range_options(),
            MediaLimits::default(),
            MediaCancellation::new(),
        )
        .unwrap_err();
    assert_eq!(
        error.kind(),
        YandexMusicPlaybackErrorKind::IncompatibleFormat
    );
    assert!(!format!("{error:?}").contains(&not_mp3.authority()));
}

fn options(server: &ReplayServer) -> YandexMusicSourceOptions {
    YandexMusicSourceOptions {
        api_base_url: server.url(""),
        http: RemoteHttpOptions {
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            max_retries: 0,
            ..RemoteHttpOptions::default()
        },
        ..YandexMusicSourceOptions::default()
    }
}

fn manager(server: &ReplayServer, token: &str) -> YandexMusicSourceManager {
    YandexMusicSourceManager::new(
        options(server),
        YandexMusicAuthentication::new(token).unwrap(),
    )
    .unwrap()
}

fn playback_options(server: &ReplayServer) -> YandexMusicSourceOptions {
    YandexMusicSourceOptions {
        playback_scheme: YandexMusicPlaybackScheme::HttpForPrivateNetworks,
        ..options(server)
    }
}

fn playback_manager(server: &ReplayServer, token: &str) -> YandexMusicSourceManager {
    YandexMusicSourceManager::new(
        playback_options(server),
        YandexMusicAuthentication::new(token).unwrap(),
    )
    .unwrap()
}

fn private_range_options() -> HttpRangeOptions {
    HttpRangeOptions {
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        max_retries: 0,
        ..HttpRangeOptions::default()
    }
}

fn media_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

fn track_json(id: &str, title: &str) -> serde_json::Value {
    json!({
        "available": true,
        "id": id,
        "title": title,
        "durationMs": 123_000,
        "artists": [{"name": "Fixture Artist"}],
        "coverUri": "avatars.yandex.net/track/%%"
    })
}
