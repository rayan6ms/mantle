use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mantle_media::{
    HlsLimits, HlsLiveLimits, HlsLivePoll, HlsLiveSequence, HlsPlaylist, HlsSegment,
    HlsVodAdtsInput, HlsVodSequence, HttpNetworkAccess, HttpPlaylistOptions, HttpRangeInput,
    HttpRangeOptions, HttpStreamInput, HttpStreamOptions, MediaCancellation, MediaInput,
    MediaLimits, MediaSession, MpegTsLimits, PcmFrame, PlaylistFormat, PlaylistLimits,
    extract_mpeg_ts_adts, load_http_hls_playlist, load_http_hls_segment,
    load_http_hls_segment_with_cancellation, load_http_playlist,
    load_http_playlist_with_cancellation,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn decodes_and_seeks_over_bounded_http_ranges() {
    let bytes = fs::read(fixture("tone-mp3.mp3")).unwrap();
    let server = RangeServer::start(bytes.clone(), ResponseMode::Partial);
    let options = private_test_options();
    let input = HttpRangeInput::open(server.url("tone.mp3?signature=do-not-log"), options)
        .expect("range source should open");
    assert_eq!(input.byte_len(), Some(bytes.len() as u64));

    let mut session = MediaSession::open(Box::new(input), Some("mp3"), MediaLimits::default())
        .expect("HTTP MP3 should probe");
    let mut frame = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    assert!(session.read_pcm(&mut frame).unwrap());
    let seek = session.seek(Duration::from_secs(3)).unwrap();
    assert!(seek.actual.is_some());
    assert!(session.read_pcm(&mut frame).unwrap());
    assert!(
        frame
            .timestamp()
            .is_some_and(|timestamp| timestamp >= Duration::from_secs(2))
    );

    let requests = server.requests();
    assert!(requests.len() >= 2, "requests: {requests:?}");
    assert_eq!(requests[0], (0, options.range_window_bytes as u64 - 1));
    assert!(
        requests.iter().any(|(start, _)| *start > 0),
        "requests: {requests:?}"
    );
}

#[test]
fn denies_loopback_by_default_without_making_a_request() {
    let server = RangeServer::start(vec![0_u8; 64], ResponseMode::Partial);
    let error = HttpRangeInput::open(server.url("private"), HttpRangeOptions::default())
        .err()
        .expect("loopback must require an explicit opt-in");
    assert_eq!(
        error.to_string(),
        "media I/O failed: HTTP destination rejected by network access policy"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn rejects_non_range_encoded_and_oversized_responses_without_leaking_urls() {
    let secret = "signed/path.mp3?token=super-secret";
    for (mode, expected) in [
        (ResponseMode::Full, "status 200"),
        (ResponseMode::Encoded, "content encoding"),
        (ResponseMode::WrongRange, "Content-Range"),
    ] {
        let server = RangeServer::start(vec![7_u8; 128], mode);
        let error = HttpRangeInput::open(server.url(secret), private_test_options())
            .err()
            .expect("malformed response must fail");
        let message = error.to_string();
        assert!(message.contains(expected), "{message}");
        assert!(!message.contains("super-secret"), "{message}");
        assert!(!message.contains("signed/path"), "{message}");
    }

    let server = RangeServer::start(vec![3_u8; 128], ResponseMode::Partial);
    let options = HttpRangeOptions {
        max_source_bytes: 64,
        ..private_test_options()
    };
    let error = HttpRangeInput::open(server.url(secret), options)
        .err()
        .expect("oversized source must fail");
    let message = error.to_string();
    assert!(message.contains("source length"), "{message}");
    assert!(!message.contains("super-secret"), "{message}");
}

#[test]
fn follows_bounded_redirects_and_reuses_the_final_uri() {
    let bytes = vec![3_u8; 96];
    let server = ReplayServer::start(move |request, _| {
        if request.path == "/start" {
            ReplayResponse::redirect("/media")
        } else {
            partial_response(&request, &bytes, Some("\"stable\""))
        }
    });
    let options = HttpRangeOptions {
        range_window_bytes: 32,
        max_redirects: 1,
        ..private_test_options()
    };
    let mut input = HttpRangeInput::open(server.url("start?token=do-not-log"), options).unwrap();
    let mut body = Vec::new();
    input.read_to_end(&mut body).unwrap();
    assert_eq!(body, vec![3_u8; 96]);

    let paths: Vec<_> = server
        .requests()
        .into_iter()
        .map(|request| request.path)
        .collect();
    assert_eq!(paths, ["/start", "/media", "/media", "/media"]);
}

#[test]
fn rejects_redirect_overflow_and_changed_range_validators_without_leaking_urls() {
    let redirect_server = ReplayServer::start(|request, _| ReplayResponse::redirect(&request.path));
    let redirect_options = HttpRangeOptions {
        max_redirects: 1,
        ..private_test_options()
    };
    let error = HttpRangeInput::open(
        redirect_server.url("loop?token=redirect-secret"),
        redirect_options,
    )
    .err()
    .unwrap();
    assert!(!error.to_string().contains("redirect-secret"));

    let bytes = vec![4_u8; 64];
    let changed_server = ReplayServer::start(move |request, count| {
        let validator = if count == 0 {
            "\"first\""
        } else {
            "\"second\""
        };
        partial_response(&request, &bytes, Some(validator))
    });
    let options = HttpRangeOptions {
        range_window_bytes: 32,
        ..private_test_options()
    };
    let mut input = HttpRangeInput::open(changed_server.url("changed"), options).unwrap();
    let mut body = Vec::new();
    let error = input.read_to_end(&mut body).unwrap_err();
    assert!(error.to_string().contains("validator changed"));
}

#[test]
fn retries_one_transient_range_response_and_obeys_cancellation() {
    let bytes = vec![5_u8; 32];
    let server = ReplayServer::start(move |request, count| {
        if count == 0 {
            ReplayResponse::status("503 Service Unavailable")
        } else {
            partial_response(&request, &bytes, None)
        }
    });
    let options = HttpRangeOptions {
        max_retries: 1,
        ..private_test_options()
    };
    let mut input = HttpRangeInput::open(server.url("retry"), options).unwrap();
    let mut body = Vec::new();
    input.read_to_end(&mut body).unwrap();
    assert_eq!(body, vec![5_u8; 32]);
    assert_eq!(server.requests().len(), 2);

    let cancelled_server =
        ReplayServer::start(|_, _| ReplayResponse::status("500 Internal Server Error"));
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    let error = HttpRangeInput::open_with_cancellation(
        cancelled_server.url("cancelled"),
        private_test_options(),
        cancellation,
    )
    .err()
    .unwrap();
    assert_eq!(error.to_string(), "media operation cancelled");
    assert!(cancelled_server.requests().is_empty());

    let bytes = vec![6_u8; 32];
    let read_server =
        ReplayServer::start(move |request, _| partial_response(&request, &bytes, None));
    let cancellation = MediaCancellation::new();
    let mut input = HttpRangeInput::open_with_cancellation(
        read_server.url("cancel-read"),
        private_test_options(),
        cancellation.clone(),
    )
    .unwrap();
    cancellation.cancel();
    assert_eq!(
        input.read(&mut [0_u8; 1]).unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
}

#[test]
fn reads_bounded_finite_and_chunked_non_range_bodies() {
    let finite = ReplayServer::start(|_, _| ReplayResponse::ok(b"finite body"));
    let mut input = HttpStreamInput::open(finite.url("finite"), private_stream_options()).unwrap();
    assert!(!input.is_seekable());
    assert_eq!(input.byte_len(), Some(11));
    let mut body = Vec::new();
    input.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"finite body");

    let chunked = ReplayServer::start(|_, _| ReplayResponse::chunked(&[b"chunked ", b"body"]));
    let mut input =
        HttpStreamInput::open(chunked.url("chunked"), private_stream_options()).unwrap();
    assert_eq!(input.byte_len(), None);
    let mut body = Vec::new();
    input.read_to_end(&mut body).unwrap();
    assert_eq!(body, b"chunked body");

    let too_many_chunks =
        ReplayServer::start(|_, _| ReplayResponse::chunked(&[&[7_u8; 32], &[8_u8; 33]]));
    let mut input =
        HttpStreamInput::open(too_many_chunks.url("too-many"), private_stream_options()).unwrap();
    let error = input.read_to_end(&mut Vec::new()).unwrap_err();
    assert!(error.to_string().contains("exceeded its 64-byte limit"));

    let oversized = ReplayServer::start(|_, _| ReplayResponse::ok(&[0_u8; 65]));
    let error = HttpStreamInput::open(oversized.url("oversized"), private_stream_options())
        .err()
        .unwrap();
    assert!(error.to_string().contains("source length"));

    let truncated = ReplayServer::start(|_, _| ReplayResponse::truncated(b"short", 10));
    let mut input =
        HttpStreamInput::open(truncated.url("truncated"), private_stream_options()).unwrap();
    assert_eq!(
        input.read_to_end(&mut Vec::new()).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );

    let cancelled = ReplayServer::start(|_, _| ReplayResponse::ok(b"cancel me"));
    let cancellation = MediaCancellation::new();
    let mut input = HttpStreamInput::open_with_cancellation(
        cancelled.url("cancelled"),
        private_stream_options(),
        cancellation.clone(),
    )
    .unwrap();
    cancellation.cancel();
    assert_eq!(
        input.read(&mut [0_u8; 1]).unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
}

#[test]
fn loads_a_redirected_playlist_and_resolves_against_the_final_uri() {
    let server = ReplayServer::start(|request, _| match request.path.as_str() {
        "/start" => ReplayResponse::redirect("/lists/master.m3u8?signature=hidden"),
        "/lists/master.m3u8" => ReplayResponse::ok(
            b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=64000\n../media/low.m3u8?key=one#fragment\n",
        ),
        _ => ReplayResponse::status("404 Not Found"),
    });
    let matched = load_http_playlist(server.url("start"), private_playlist_options())
        .unwrap()
        .unwrap();
    assert_eq!(matched.format, PlaylistFormat::Hls);
    assert_eq!(
        matched.reference.identifier,
        server.url("media/low.m3u8?key=one")
    );
    assert_eq!(
        server
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect::<Vec<_>>(),
        ["/start", "/lists/master.m3u8"]
    );
}

#[test]
fn playlist_loading_shares_parser_bounds_and_cancellation() {
    let oversized = ReplayServer::start(|_, _| ReplayResponse::ok(&[b'x'; 129]));
    let error = load_http_playlist(oversized.url("large"), private_playlist_options()).unwrap_err();
    assert!(error.to_string().contains("source length"));

    let cancelled = ReplayServer::start(|_, _| ReplayResponse::ok(b"#EXTM3U\n"));
    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    let error = load_http_playlist_with_cancellation(
        cancelled.url("cancelled"),
        private_playlist_options(),
        cancellation,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "media operation cancelled");
    assert!(cancelled.requests().is_empty());
}

#[test]
fn loads_an_hls_master_then_sequences_its_vod_media_playlist() {
    let server = ReplayServer::start(|request, _| match request.path.as_str() {
        "/root/master.m3u8" => {
            ReplayResponse::ok(b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=64000\nvod/media.m3u8\n")
        }
        "/root/vod/media.m3u8" => ReplayResponse::ok(
            b"#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:9\n#EXTINF:1,First\n9.ts\n\
              #EXTINF:1,Second\n../10.ts\n#EXT-X-ENDLIST\n",
        ),
        _ => ReplayResponse::status("404 Not Found"),
    });

    let HlsPlaylist::Master(master) = load_http_hls_playlist(
        server.url("root/master.m3u8"),
        private_playlist_options(),
        HlsLimits::default(),
    )
    .unwrap() else {
        panic!("expected HLS master playlist");
    };
    let selected = master.selected_variant().unwrap();
    assert_eq!(selected.uri, server.url("root/vod/media.m3u8"));

    let HlsPlaylist::Media(media) = load_http_hls_playlist(
        &selected.uri,
        private_playlist_options(),
        HlsLimits::default(),
    )
    .unwrap() else {
        panic!("expected HLS media playlist");
    };
    let mut sequence = HlsVodSequence::new(media).unwrap();
    assert_eq!(
        sequence.next_segment().unwrap().uri,
        server.url("root/vod/9.ts")
    );
    assert_eq!(
        sequence.next_segment().unwrap().uri,
        server.url("root/10.ts")
    );
    assert!(sequence.next_segment().is_none());
}

#[test]
fn loads_bounded_hls_segment_bytes_and_observes_cancellation() {
    let body = fs::read(fixture("tone-aac-lc.ts")).unwrap();
    let expected = body.clone();
    let server = ReplayServer::start(move |request, _| match request.path.as_str() {
        "/segment.ts" => ReplayResponse::ok(&body),
        _ => ReplayResponse::status("404 Not Found"),
    });
    let segment = HlsSegment {
        sequence: 1,
        uri: server.url("segment.ts"),
        duration: Some(Duration::from_secs(1)),
        title: None,
        discontinuity: false,
    };
    let segment_options = HttpStreamOptions {
        max_response_bytes: 64 * 1024,
        ..private_stream_options()
    };
    let loaded = load_http_hls_segment(&segment, segment_options).unwrap();
    assert_eq!(loaded, expected);
    let extracted = extract_mpeg_ts_adts(&loaded, MpegTsLimits::default()).unwrap();
    let mut session = extracted
        .into_media_session(MediaLimits::default())
        .unwrap();
    let mut pcm = PcmFrame::with_capacity(session.limits().max_pcm_samples_per_frame);
    assert!(session.read_pcm(&mut pcm).unwrap());

    let error = load_http_hls_segment(
        &segment,
        HttpStreamOptions {
            max_response_bytes: 4,
            ..private_stream_options()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("4-byte limit"));

    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    let request_count = server.requests().len();
    assert!(matches!(
        load_http_hls_segment_with_cancellation(&segment, private_stream_options(), cancellation,),
        Err(mantle_media::MediaError::Cancelled)
    ));
    assert_eq!(server.requests().len(), request_count);
}

#[test]
fn joins_multiple_http_mpeg_ts_segments_into_one_continuous_adts_session() {
    let transport = Arc::new(fs::read(fixture("tone-aac-lc.ts")).unwrap());
    let server_transport = Arc::clone(&transport);
    let server = ReplayServer::start(move |request, _| match request.path.as_str() {
        "/live/media.m3u8" => ReplayResponse::ok(
            b"#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:20\n#EXTINF:2,First\na.ts\n\
              #EXTINF:2,Second\nb.ts\n#EXT-X-ENDLIST\n",
        ),
        "/live/a.ts" | "/live/b.ts" => ReplayResponse::ok(&server_transport),
        _ => ReplayResponse::status("404 Not Found"),
    });
    let HlsPlaylist::Media(media) = load_http_hls_playlist(
        server.url("live/media.m3u8"),
        private_playlist_options(),
        HlsLimits::default(),
    )
    .unwrap() else {
        panic!("expected a media playlist");
    };
    let input = HlsVodAdtsInput::new(
        media,
        HttpStreamOptions {
            max_response_bytes: 64 * 1024,
            ..private_stream_options()
        },
        MpegTsLimits::default(),
    )
    .unwrap();
    let mut joined =
        MediaSession::open(Box::new(input), Some("aac"), MediaLimits::default()).unwrap();
    let mut pcm = PcmFrame::with_capacity(joined.limits().max_pcm_samples_per_frame);
    let mut frames = 0;
    let mut previous = None;
    while joined.read_pcm(&mut pcm).unwrap() {
        let timestamp = pcm.timestamp().unwrap();
        assert!(previous.is_none_or(|value| timestamp >= value));
        previous = Some(timestamp);
        frames += 1;
    }

    let single_adts = extract_mpeg_ts_adts(&transport, MpegTsLimits::default()).unwrap();
    let mut single = single_adts
        .into_media_session(MediaLimits::default())
        .unwrap();
    let mut single_pcm = PcmFrame::with_capacity(single.limits().max_pcm_samples_per_frame);
    let mut single_frames = 0;
    while single.read_pcm(&mut single_pcm).unwrap() {
        single_frames += 1;
    }
    assert_eq!(frames, single_frames * 2);
    assert!(!joined.read_pcm(&mut pcm).unwrap());
    assert_eq!(
        server
            .requests()
            .into_iter()
            .map(|request| request.path)
            .collect::<Vec<_>>(),
        ["/live/media.m3u8", "/live/a.ts", "/live/b.ts"]
    );
}

#[test]
fn live_http_reload_obeys_manual_deadlines_and_cancellation_without_early_requests() {
    let server = ReplayServer::start(|request, count| match request.path.as_str() {
        "/live.m3u8" if count < 2 => ReplayResponse::ok(b"#EXTM3U\n#EXTINF:1,First\na.ts\n"),
        "/live.m3u8" => {
            ReplayResponse::ok(b"#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:2\n#EXTINF:1,Second\nb.ts\n")
        }
        _ => ReplayResponse::status("404 Not Found"),
    });
    let mut live = HlsLiveSequence::new(HlsLiveLimits::default()).unwrap();
    let HlsLivePoll::Segment(first) = live
        .poll_http(
            server.url("live.m3u8"),
            private_playlist_options(),
            HlsLimits::default(),
            Duration::ZERO,
        )
        .unwrap()
    else {
        panic!("expected first live segment");
    };
    assert_eq!(first.uri, server.url("a.ts"));
    assert_eq!(
        live.poll_http(
            server.url("live.m3u8"),
            private_playlist_options(),
            HlsLimits::default(),
            Duration::ZERO,
        )
        .unwrap(),
        HlsLivePoll::WaitUntil(Duration::from_millis(200))
    );
    assert_eq!(server.requests().len(), 2);
    assert_eq!(
        live.poll_http(
            server.url("live.m3u8"),
            private_playlist_options(),
            HlsLimits::default(),
            Duration::from_millis(100),
        )
        .unwrap(),
        HlsLivePoll::WaitUntil(Duration::from_millis(200))
    );
    assert_eq!(server.requests().len(), 2);

    let cancellation = MediaCancellation::new();
    cancellation.cancel();
    assert!(matches!(
        live.poll_http_with_cancellation(
            server.url("live.m3u8"),
            private_playlist_options(),
            HlsLimits::default(),
            Duration::from_millis(150),
            cancellation,
        ),
        Err(mantle_media::HlsError::Media(
            mantle_media::MediaError::Cancelled
        ))
    ));
    assert_eq!(server.requests().len(), 2);

    let HlsLivePoll::Segment(second) = live
        .poll_http(
            server.url("live.m3u8"),
            private_playlist_options(),
            HlsLimits::default(),
            Duration::from_millis(200),
        )
        .unwrap()
    else {
        panic!("expected reloaded live segment");
    };
    assert_eq!(second.uri, server.url("b.ts"));
    assert_eq!(server.requests().len(), 3);
}

fn private_test_options() -> HttpRangeOptions {
    HttpRangeOptions {
        range_window_bytes: 32 * 1024,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpRangeOptions::default()
    }
}

fn private_stream_options() -> HttpStreamOptions {
    HttpStreamOptions {
        max_response_bytes: 64,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpStreamOptions::default()
    }
}

fn private_playlist_options() -> HttpPlaylistOptions {
    let http = HttpStreamOptions {
        max_response_bytes: 128,
        ..private_stream_options()
    };
    HttpPlaylistOptions {
        http,
        playlist: PlaylistLimits {
            max_playlist_bytes: 128,
            ..PlaylistLimits::default()
        },
        include_plain: false,
    }
}

#[derive(Clone, Copy)]
enum ResponseMode {
    Partial,
    Full,
    Encoded,
    WrongRange,
}

struct RangeServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<(u64, u64)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RangeServer {
    fn start(bytes: Vec<u8>, mode: ResponseMode) -> Self {
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
                    Ok((stream, _)) => {
                        serve_request(stream, &bytes, mode, &thread_requests);
                    }
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

impl Drop for RangeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn serve_request(
    mut stream: TcpStream,
    bytes: &[u8],
    mode: ResponseMode,
    requests: &Mutex<Vec<(u64, u64)>>,
) {
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
    let Some((start, requested_end)) = parse_range(&request) else {
        let _ = write_response(&mut stream, "400 Bad Request", &[], &[]);
        return;
    };
    requests.lock().unwrap().push((start, requested_end));

    if matches!(mode, ResponseMode::Full) {
        let _ = write_response(&mut stream, "200 OK", &[], bytes);
        return;
    }
    if start >= bytes.len() as u64 {
        let content_range = format!("bytes */{}", bytes.len());
        let _ = write_response(
            &mut stream,
            "416 Range Not Satisfiable",
            &[("Content-Range", &content_range)],
            &[],
        );
        return;
    }
    let end = requested_end.min(bytes.len() as u64 - 1);
    let (Ok(start_index), Ok(end_index)) = (usize::try_from(start), usize::try_from(end)) else {
        return;
    };
    let body = &bytes[start_index..=end_index];
    let content_range = if matches!(mode, ResponseMode::WrongRange) {
        format!("bytes {}-{}/{}", start.saturating_add(1), end, bytes.len())
    } else {
        format!("bytes {start}-{end}/{}", bytes.len())
    };
    let mut headers = vec![("Content-Range", content_range.as_str())];
    if matches!(mode, ResponseMode::Encoded) {
        headers.push(("Content-Encoding", "gzip"));
    }
    let _ = write_response(&mut stream, "206 Partial Content", &headers, body);
}

fn parse_range(request: &str) -> Option<(u64, u64)> {
    let value = request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| name.eq_ignore_ascii_case("range").then_some(value.trim()))?
        .strip_prefix("bytes=")?;
    let (start, end) = value.trim().split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    )?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Both)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReplayRequest {
    path: String,
    range: Option<(u64, u64)>,
}

struct ReplayResponse {
    status: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    chunks: Option<Vec<Vec<u8>>>,
    declared_length: Option<usize>,
}

impl ReplayResponse {
    fn status(status: &'static str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            chunks: None,
            declared_length: None,
        }
    }

    fn redirect(location: &str) -> Self {
        Self {
            status: "302 Found",
            headers: vec![("Location".to_owned(), location.to_owned())],
            body: Vec::new(),
            chunks: None,
            declared_length: None,
        }
    }

    fn ok(body: &[u8]) -> Self {
        Self {
            status: "200 OK",
            headers: Vec::new(),
            body: body.to_vec(),
            chunks: None,
            declared_length: None,
        }
    }

    fn truncated(body: &[u8], declared_length: usize) -> Self {
        Self {
            status: "200 OK",
            headers: Vec::new(),
            body: body.to_vec(),
            chunks: None,
            declared_length: Some(declared_length),
        }
    }

    fn chunked(chunks: &[&[u8]]) -> Self {
        Self {
            status: "200 OK",
            headers: Vec::new(),
            body: Vec::new(),
            chunks: Some(chunks.iter().map(|chunk| chunk.to_vec()).collect()),
            declared_length: None,
        }
    }
}

fn partial_response(
    request: &ReplayRequest,
    bytes: &[u8],
    validator: Option<&str>,
) -> ReplayResponse {
    let (start, requested_end) = request.range.unwrap();
    let end = requested_end.min(bytes.len() as u64 - 1);
    let mut headers = vec![(
        "Content-Range".to_owned(),
        format!("bytes {start}-{end}/{}", bytes.len()),
    )];
    if let Some(validator) = validator {
        headers.push(("ETag".to_owned(), validator.to_owned()));
    }
    let start = usize::try_from(start).unwrap();
    let end = usize::try_from(end).unwrap();
    ReplayResponse {
        status: "206 Partial Content",
        headers,
        body: bytes[start..=end].to_vec(),
        chunks: None,
        declared_length: None,
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
                    Ok((stream, _)) => {
                        serve_replay_request(stream, &thread_requests, responder.as_ref());
                    }
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

fn serve_replay_request(
    mut stream: TcpStream,
    requests: &Mutex<Vec<ReplayRequest>>,
    responder: &(dyn Fn(ReplayRequest, usize) -> ReplayResponse + Send + Sync),
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut raw = [0_u8; 16 * 1024];
    let mut used = 0;
    while used < raw.len() {
        let Ok(count) = stream.read(&mut raw[used..]) else {
            return;
        };
        if count == 0 {
            return;
        }
        used += count;
        if raw[..used].windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let raw = String::from_utf8_lossy(&raw[..used]);
    let target = raw
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let request = ReplayRequest {
        path: target.split('?').next().unwrap_or(target).to_owned(),
        range: parse_range(&raw),
    };
    let count = {
        let mut requests = requests.lock().unwrap();
        let count = requests.len();
        requests.push(request.clone());
        count
    };
    let response = responder(request, count);
    let _ = write_replay_response(&mut stream, response);
}

fn write_replay_response(stream: &mut TcpStream, response: ReplayResponse) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nConnection: close\r\n",
        response.status
    )?;
    for (name, value) in response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    if let Some(chunks) = response.chunks {
        stream.write_all(b"Transfer-Encoding: chunked\r\n\r\n")?;
        for chunk in chunks {
            write!(stream, "{:x}\r\n", chunk.len())?;
            stream.write_all(&chunk)?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"0\r\n\r\n")?;
    } else {
        write!(
            stream,
            "Content-Length: {}\r\n\r\n",
            response.declared_length.unwrap_or(response.body.len())
        )?;
        stream.write_all(&response.body)?;
    }
    stream.flush()?;
    stream.shutdown(Shutdown::Both)
}
