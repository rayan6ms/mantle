use mantle_media::{
    PlaylistError, PlaylistFormat, PlaylistLimits, PlaylistMatch, PlaylistReference,
    probe_playlist, resolve_http_reference,
};

fn parse(bytes: &[u8], include_plain: bool) -> Option<PlaylistMatch> {
    probe_playlist(bytes, include_plain, PlaylistLimits::default()).unwrap()
}

fn reference(identifier: &str, title: Option<&str>) -> PlaylistReference {
    PlaylistReference {
        identifier: identifier.to_owned(),
        title: title.map(str::to_owned),
    }
}

#[test]
fn m3u_preserves_referral_order_and_extinf_stream_classification() {
    assert_eq!(
        parse(b"#EXTM3U\n# a comment\nhttps://one.example/audio\n", false),
        Some(PlaylistMatch {
            format: PlaylistFormat::M3u,
            reference: reference("https://one.example/audio", None),
        })
    );

    assert_eq!(
        parse(
            b"#EXTM3U\nignored-relative-item\n#EXTINF:-1,Station title\nicy://radio.example/live\n",
            false,
        ),
        Some(PlaylistMatch {
            format: PlaylistFormat::Hls,
            reference: reference("http://radio.example/live", None),
        })
    );
}

#[test]
fn hls_classification_precedes_an_ordinary_m3u_referral() {
    let playlist = b"#EXTM3U\nhttps://fallback.example/audio\n\
        #EXT-X-STREAM-INF:BANDWIDTH=64000\nvariants/low.m3u8\n";
    assert_eq!(
        parse(playlist, false),
        Some(PlaylistMatch {
            format: PlaylistFormat::Hls,
            reference: reference("variants/low.m3u8", None),
        })
    );
}

#[test]
fn pls_pairs_entries_by_numeric_index_deterministically() {
    let playlist = b"[Playlist]\n\
        Title2=Second\nFile2=https://two.example/audio\n\
        File1=icy://one.example/live\nTitle1=First\nNumberOfEntries=2\n";
    assert_eq!(
        parse(playlist, false),
        Some(PlaylistMatch {
            format: PlaylistFormat::Pls,
            reference: reference("http://one.example/live", Some("First")),
        })
    );
}

#[test]
fn plain_lists_are_explicit_and_keep_first_supported_line() {
    let playlist = b"https://one.example/audio\nhttp://two.example/audio\n";
    assert_eq!(parse(playlist, false), None);
    assert_eq!(
        parse(playlist, true),
        Some(PlaylistMatch {
            format: PlaylistFormat::Plain,
            reference: reference("https://one.example/audio", None),
        })
    );
    assert_eq!(parse(b"not a URL\nhttps://one.example/audio\n", true), None);
    assert_eq!(parse(b" HTTPS://one.example/audio\n", true), None);
}

#[test]
fn malformed_utf8_outside_a_reference_is_replaced_without_failure() {
    let playlist = b"#EXTM3U\n# bad text: \xff\nhttps://one.example/audio\n";
    assert_eq!(
        parse(playlist, false).unwrap().reference,
        reference("https://one.example/audio", None)
    );
}

#[test]
fn non_playlist_bytes_and_empty_playlists_do_not_match() {
    assert_eq!(parse(b"ID3\0\0\0\0", true), None);
    assert_eq!(parse(b"#EXTM3U\n# comments only\n", false), None);
    assert_eq!(parse(b"[playlist]\nVersion=2\n", false), None);
}

#[test]
fn playlist_limits_fail_before_returning_a_partial_result() {
    let playlist = b"#EXTM3U\nhttps://one.example/audio\nhttps://two.example/audio\n";
    let too_many = PlaylistLimits {
        max_entries: 1,
        ..PlaylistLimits::default()
    };
    assert_eq!(
        probe_playlist(playlist, false, too_many),
        Err(PlaylistError::TooManyEntries { limit: 1 })
    );

    let too_few_bytes = PlaylistLimits {
        max_playlist_bytes: playlist.len() - 1,
        ..PlaylistLimits::default()
    };
    assert_eq!(
        probe_playlist(playlist, false, too_few_bytes),
        Err(PlaylistError::TooLarge {
            actual: playlist.len(),
            limit: playlist.len() - 1,
        })
    );

    let short_lines = PlaylistLimits {
        max_line_bytes: 12,
        ..PlaylistLimits::default()
    };
    assert_eq!(
        probe_playlist(playlist, false, short_lines),
        Err(PlaylistError::LineTooLong {
            actual: "https://one.example/audio".len(),
            limit: 12,
        })
    );
}

#[test]
fn every_playlist_limit_must_be_nonzero() {
    for limits in [
        PlaylistLimits {
            max_playlist_bytes: 0,
            ..PlaylistLimits::default()
        },
        PlaylistLimits {
            max_line_bytes: 0,
            ..PlaylistLimits::default()
        },
        PlaylistLimits {
            max_entries: 0,
            ..PlaylistLimits::default()
        },
    ] {
        assert!(matches!(
            probe_playlist(b"", false, limits),
            Err(PlaylistError::InvalidLimits(_))
        ));
    }
}

#[test]
fn http_references_resolve_relative_paths_queries_and_authorities() {
    let base = "https://media.example/lists/live/master.m3u8?old=1";
    assert_eq!(
        resolve_http_reference(base, "../audio/./low.m3u8?token=2#ignored").unwrap(),
        "https://media.example/lists/audio/low.m3u8?token=2"
    );
    assert_eq!(
        resolve_http_reference(base, "/root/../stream.m3u8").unwrap(),
        "https://media.example/stream.m3u8"
    );
    assert_eq!(
        resolve_http_reference(base, "?token=next").unwrap(),
        "https://media.example/lists/live/master.m3u8?token=next"
    );
    assert_eq!(
        resolve_http_reference(base, "//cdn.example/audio.m3u8").unwrap(),
        "https://cdn.example/audio.m3u8"
    );
}

#[test]
fn http_reference_resolution_rejects_credentials_and_other_schemes() {
    let base = "https://media.example/list.m3u8";
    for reference in [
        "ftp://media.example/audio",
        "https://user:secret@media.example/audio",
        "javascript:ignored",
    ] {
        assert!(matches!(
            resolve_http_reference(base, reference),
            Err(PlaylistError::InvalidReference(_))
        ));
    }
    assert!(matches!(
        resolve_http_reference("relative/base", "next.m3u8"),
        Err(PlaylistError::InvalidReference(_))
    ));
}
