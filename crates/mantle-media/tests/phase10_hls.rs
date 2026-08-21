use std::time::Duration;

use mantle_media::{
    HlsError, HlsLimits, HlsLiveLimits, HlsLivePoll, HlsLiveSequence, HlsMediaPlaylist,
    HlsPlaylist, HlsSegment, HlsVodSequence, PlaylistLimits, parse_hls_playlist,
};

const BASE: &str = "https://media.example/live/lists/master.m3u8?token=old";

#[test]
fn master_playlist_preserves_variant_order_and_resolves_uris() {
    let playlist = parse_hls_playlist(
        b"#EXTM3U\n\
          #EXT-X-STREAM-INF:BANDWIDTH=64000,CODECS=\"mp4a.40.2\"\nlow/index.m3u8\n\
          #EXT-X-STREAM-INF:BANDWIDTH=128000\n//cdn.example/high.m3u8\n",
        BASE,
        PlaylistLimits::default(),
        HlsLimits::default(),
    )
    .unwrap();
    let HlsPlaylist::Master(master) = playlist else {
        panic!("expected master playlist");
    };
    assert_eq!(master.variants.len(), 2);
    assert_eq!(master.variants[0].bandwidth, Some(64_000));
    assert_eq!(
        master.variants[0].uri,
        "https://media.example/live/lists/low/index.m3u8"
    );
    assert_eq!(master.variants[1].bandwidth, Some(128_000));
    assert_eq!(master.variants[1].uri, "https://cdn.example/high.m3u8");
    assert_eq!(master.selected_variant(), Some(&master.variants[0]));

    for directive in [
        "#EXT-X-STREAM-INF:CODECS=\"mp4a.40.2\"",
        "#EXT-X-STREAM-INF:BANDWIDTH=nope",
    ] {
        let playlist = format!("#EXTM3U\n{directive}\na.m3u8\n");
        let HlsPlaylist::Master(master) = parse_hls_playlist(
            playlist.as_bytes(),
            BASE,
            PlaylistLimits::default(),
            HlsLimits::default(),
        )
        .unwrap() else {
            panic!("expected master playlist");
        };
        assert_eq!(master.variants[0].bandwidth, None);
    }
}

#[test]
fn media_playlist_models_sequences_durations_titles_and_discontinuities() {
    let playlist = parse_hls_playlist(
        b"#EXTM3U\n#EXT-X-TARGETDURATION:7\n#EXT-X-MEDIA-SEQUENCE:42\n\
          #EXTINF:6.006,First segment\nsegments/42.ts\n\
          #EXT-X-DISCONTINUITY\n#EXTINF:5.5,Second segment\n../43.ts?key=one#ignored\n\
          #EXT-X-ENDLIST\n",
        BASE,
        PlaylistLimits::default(),
        HlsLimits::default(),
    )
    .unwrap();
    let HlsPlaylist::Media(media) = playlist else {
        panic!("expected media playlist");
    };
    assert_eq!(media.media_sequence, 42);
    assert_eq!(media.target_duration, Some(Duration::from_secs(7)));
    assert!(media.end_list);
    assert_eq!(media.segments.len(), 2);
    assert_eq!(media.segments[0].sequence, 42);
    assert_eq!(
        media.segments[0].duration,
        Some(Duration::from_millis(6_006))
    );
    assert_eq!(media.segments[0].title.as_deref(), Some("First segment"));
    assert!(!media.segments[0].discontinuity);
    assert_eq!(
        media.segments[0].uri,
        "https://media.example/live/lists/segments/42.ts"
    );
    assert_eq!(media.segments[1].sequence, 43);
    assert_eq!(
        media.segments[1].duration,
        Some(Duration::from_millis(5_500))
    );
    assert!(media.segments[1].discontinuity);
    assert_eq!(
        media.segments[1].uri,
        "https://media.example/live/43.ts?key=one"
    );
}

#[test]
fn vod_sequence_delivers_each_segment_once_and_has_stable_eof() {
    let HlsPlaylist::Media(media) = parse_hls_playlist(
        b"#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:7\n#EXTINF:1,One\n7.ts\n\
          #EXTINF:1,Two\n8.ts\n#EXT-X-ENDLIST\n",
        BASE,
        PlaylistLimits::default(),
        HlsLimits::default(),
    )
    .unwrap() else {
        panic!("expected media playlist");
    };
    let mut sequence = HlsVodSequence::new(media).unwrap();
    assert_eq!(sequence.remaining(), 2);
    assert_eq!(sequence.next_segment().unwrap().sequence, 7);
    assert_eq!(sequence.remaining(), 1);
    assert_eq!(sequence.next_segment().unwrap().sequence, 8);
    assert_eq!(sequence.remaining(), 0);
    assert!(sequence.next_segment().is_none());
    assert!(sequence.next_segment().is_none());
}

#[test]
fn hls_collections_and_durations_are_hard_bounded() {
    let variants = b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\na.m3u8\n\
        #EXT-X-STREAM-INF:BANDWIDTH=2\nb.m3u8\n";
    assert!(matches!(
        parse_hls_playlist(
            variants,
            BASE,
            PlaylistLimits::default(),
            HlsLimits {
                max_variants: 1,
                ..HlsLimits::default()
            },
        ),
        Err(HlsError::TooManyVariants { limit: 1 })
    ));

    let segments = b"#EXTM3U\n#EXTINF:2,One\na.ts\n#EXTINF:2,Two\nb.ts\n#EXT-X-ENDLIST\n";
    assert!(matches!(
        parse_hls_playlist(
            segments,
            BASE,
            PlaylistLimits::default(),
            HlsLimits {
                max_segments: 1,
                ..HlsLimits::default()
            },
        ),
        Err(HlsError::TooManySegments { limit: 1 })
    ));
    assert!(matches!(
        parse_hls_playlist(
            segments,
            BASE,
            PlaylistLimits::default(),
            HlsLimits {
                max_segment_duration: Duration::from_secs(1),
                ..HlsLimits::default()
            },
        ),
        Err(HlsError::SegmentDurationExceeded { .. })
    ));
    assert!(matches!(
        parse_hls_playlist(
            segments,
            BASE,
            PlaylistLimits::default(),
            HlsLimits {
                max_playlist_duration: Duration::from_secs(3),
                ..HlsLimits::default()
            },
        ),
        Err(HlsError::PlaylistDurationExceeded { .. })
    ));
}

#[test]
fn unsupported_or_malformed_hls_features_fail_explicitly() {
    for (tag, feature) in [
        ("#EXT-X-KEY:METHOD=AES-128,URI=\"key\"", "encryption"),
        ("#EXT-X-MAP:URI=\"init.mp4\"", "initialization maps"),
        ("#EXT-X-BYTERANGE:100@0", "byte ranges"),
    ] {
        let playlist = format!("#EXTM3U\n{tag}\n#EXTINF:1,One\na.ts\n#EXT-X-ENDLIST\n");
        assert!(matches!(
            parse_hls_playlist(
                playlist.as_bytes(),
                BASE,
                PlaylistLimits::default(),
                HlsLimits::default(),
            ),
            Err(HlsError::UnsupportedFeature(actual)) if actual == feature
        ));
    }

    for playlist in [
        "#EXTM3U\n#EXT-X-TARGETDURATION:nope\n#EXTINF:1,One\na.ts\n",
        "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:18446744073709551615\n#EXTINF:1,A\na.ts\n#EXTINF:1,B\nb.ts\n",
    ] {
        assert!(matches!(
            parse_hls_playlist(
                playlist.as_bytes(),
                BASE,
                PlaylistLimits::default(),
                HlsLimits::default(),
            ),
            Err(HlsError::InvalidPlaylist(_))
        ));
    }
}

#[test]
fn live_media_playlist_cannot_be_misused_as_vod() {
    let HlsPlaylist::Media(media) = parse_hls_playlist(
        b"#EXTM3U\n#EXTINF:1,One\na.ts\n",
        BASE,
        PlaylistLimits::default(),
        HlsLimits::default(),
    )
    .unwrap() else {
        panic!("expected media playlist");
    };
    assert!(matches!(HlsVodSequence::new(media), Err(HlsError::NotVod)));
}

#[test]
fn every_hls_limit_must_be_nonzero() {
    for limits in [
        HlsLimits {
            max_variants: 0,
            ..HlsLimits::default()
        },
        HlsLimits {
            max_segments: 0,
            ..HlsLimits::default()
        },
        HlsLimits {
            max_segment_duration: Duration::ZERO,
            ..HlsLimits::default()
        },
        HlsLimits {
            max_playlist_duration: Duration::ZERO,
            ..HlsLimits::default()
        },
    ] {
        assert!(matches!(
            parse_hls_playlist(b"#EXTM3U\n", BASE, PlaylistLimits::default(), limits,),
            Err(HlsError::InvalidLimits(_))
        ));
    }
}

#[test]
fn live_sequence_emits_oldest_unseen_segments_then_polls_to_the_duration_deadline() {
    let mut live = HlsLiveSequence::new(HlsLiveLimits {
        reload_interval: Duration::from_millis(200),
        max_segment_wait: Duration::from_secs(2),
        ..HlsLiveLimits::default()
    })
    .unwrap();
    let playlist = live_playlist(10, 3, false, Some(Duration::from_secs(1)));
    for sequence in 10..13 {
        let HlsLivePoll::Segment(segment) = live.poll(&playlist, Duration::ZERO).unwrap() else {
            panic!("expected the next retained segment");
        };
        assert_eq!(segment.sequence, sequence);
    }

    assert_eq!(
        live.poll(&playlist, Duration::ZERO).unwrap(),
        HlsLivePoll::WaitUntil(Duration::from_millis(200))
    );
    assert_eq!(
        live.poll(&playlist, Duration::from_millis(199)).unwrap(),
        HlsLivePoll::WaitUntil(Duration::from_millis(200))
    );
    assert_eq!(
        live.poll(&playlist, Duration::from_millis(200)).unwrap(),
        HlsLivePoll::WaitUntil(Duration::from_millis(400))
    );
    assert_eq!(
        live.poll(&playlist, Duration::from_secs(1)).unwrap(),
        HlsLivePoll::Exhausted
    );
}

#[test]
fn live_history_remains_bounded_during_a_long_sliding_window_simulation() {
    let limits = HlsLiveLimits {
        max_retained_segments: 64,
        ..HlsLiveLimits::default()
    };
    let mut live = HlsLiveSequence::new(limits).unwrap();
    let mut stable_capacity = None;
    let mut stable_identity_bytes = None;
    for sequence in 0..100_000_u64 {
        let playlist = live_playlist(sequence, 6, false, Some(Duration::from_secs(1)));
        let HlsLivePoll::Segment(segment) = live.poll(&playlist, Duration::ZERO).unwrap() else {
            panic!("sliding window should always expose one unseen segment");
        };
        assert_eq!(segment.sequence, sequence);
        assert!(live.retained_segments() <= limits.max_retained_segments);
        if sequence == 1_000 {
            stable_capacity = Some(live.retained_segment_capacity());
            stable_identity_bytes = Some(live.retained_identity_capacity_bytes());
        } else if sequence > 1_000 {
            assert_eq!(
                live.retained_segment_capacity(),
                stable_capacity.unwrap(),
                "history allocation grew at sequence {sequence}"
            );
            assert_eq!(
                live.retained_identity_capacity_bytes(),
                stable_identity_bytes.unwrap(),
                "retained URI allocation grew at sequence {sequence}"
            );
        }
    }
    assert_eq!(live.retained_segments(), limits.max_retained_segments);
}

#[test]
fn live_completion_reload_limit_and_configuration_fail_explicitly() {
    let finished = live_playlist(1, 1, true, Some(Duration::from_secs(1)));
    let mut live = HlsLiveSequence::new(HlsLiveLimits::default()).unwrap();
    assert!(matches!(
        live.poll(&finished, Duration::ZERO).unwrap(),
        HlsLivePoll::Segment(_)
    ));
    assert_eq!(
        live.poll(&finished, Duration::ZERO).unwrap(),
        HlsLivePoll::Ended
    );

    let playlist = live_playlist(1, 1, false, Some(Duration::from_secs(10)));
    let mut limited = HlsLiveSequence::new(HlsLiveLimits {
        max_no_progress_reloads: 1,
        reload_interval: Duration::from_millis(1),
        ..HlsLiveLimits::default()
    })
    .unwrap();
    assert!(matches!(
        limited.poll(&playlist, Duration::ZERO).unwrap(),
        HlsLivePoll::Segment(_)
    ));
    assert!(matches!(
        limited.poll(&playlist, Duration::ZERO).unwrap(),
        HlsLivePoll::WaitUntil(_)
    ));
    assert!(matches!(
        limited.poll(&playlist, Duration::from_millis(1)),
        Err(HlsError::LiveReloadLimitExceeded { limit: 1 })
    ));

    for limits in [
        HlsLiveLimits {
            max_retained_segments: 0,
            ..HlsLiveLimits::default()
        },
        HlsLiveLimits {
            reload_interval: Duration::ZERO,
            ..HlsLiveLimits::default()
        },
        HlsLiveLimits {
            max_segment_wait: Duration::ZERO,
            ..HlsLiveLimits::default()
        },
        HlsLiveLimits {
            max_no_progress_reloads: 0,
            ..HlsLiveLimits::default()
        },
    ] {
        assert!(matches!(
            HlsLiveSequence::new(limits),
            Err(HlsError::InvalidLimits(_))
        ));
    }
}

fn live_playlist(
    media_sequence: u64,
    count: usize,
    end_list: bool,
    duration: Option<Duration>,
) -> HlsMediaPlaylist {
    HlsMediaPlaylist {
        media_sequence,
        target_duration: duration,
        end_list,
        segments: (0..count)
            .map(|offset| {
                let sequence = media_sequence + u64::try_from(offset).unwrap();
                HlsSegment {
                    sequence,
                    uri: format!("https://media.example/{sequence:020}.ts"),
                    duration,
                    title: None,
                    discontinuity: false,
                }
            })
            .collect(),
    }
}
