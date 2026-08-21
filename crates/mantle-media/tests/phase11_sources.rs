use mantle_core::{
    SerializationLimits, SourceCancellation, SourceLoad, SourceManager, SourceReference,
    SourceRegistry, SourceRegistryError, SourceRegistryLimits, decode_source_track,
    encode_source_track,
};
use mantle_media::{
    HttpMediaSourceManager, HttpMediaSourceOptions, HttpNetworkAccess, HttpPlaylistOptions,
    HttpRangeOptions, HttpStreamOptions, LocalMediaSourceManager, MediaProbe, MediaSourceTrack,
    PlaylistLimits,
};
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/media/fixtures")
        .join(name)
}

#[test]
fn local_source_probes_metadata_and_round_trips_registry_owned_wire_details() {
    let mut registry = SourceRegistry::<MediaSourceTrack>::new(SourceRegistryLimits::default());
    registry
        .register(Box::new(LocalMediaSourceManager::default()))
        .unwrap();
    let identifier = fixture("tone-metadata.flac").to_string_lossy().into_owned();
    let loaded = registry
        .load(&SourceReference::new(Some(identifier.clone()), false))
        .unwrap()
        .unwrap();

    assert_eq!(loaded.item.probe, MediaProbe::Flac);
    assert_eq!(loaded.item.info.identifier, identifier);
    assert_eq!(loaded.item.info.title, "Mantle Fixture Title");
    assert_eq!(loaded.item.info.author, "Mantle Fixture Artist");
    assert!(!loaded.item.info.is_stream);

    let encoded = encode_source_track(
        &loaded.item.info,
        Duration::from_millis(123),
        &loaded,
        &registry,
        SerializationLimits::default(),
    )
    .unwrap();
    let decoded = decode_source_track(&encoded, &registry, SerializationLimits::default())
        .unwrap()
        .unwrap();
    assert_eq!(decoded.position, Duration::from_millis(123));
    assert_eq!(decoded.item.item, loaded.item);

    assert_eq!(
        registry
            .load(&SourceReference::new(
                Some(fixture("missing.flac").to_string_lossy().into_owned()),
                false,
            ))
            .unwrap(),
        None
    );
}

#[test]
fn source_cancellation_stops_local_selection_before_probe() {
    let mut registry = SourceRegistry::<MediaSourceTrack>::new(SourceRegistryLimits::default());
    registry
        .register(Box::new(LocalMediaSourceManager::default()))
        .unwrap();
    let cancellation = SourceCancellation::new();
    cancellation.cancel();
    assert_eq!(
        registry
            .load_with_cancellation(
                &SourceReference::new(
                    Some(fixture("tone-mp3.mp3").to_string_lossy().into_owned()),
                    false,
                ),
                &cancellation,
            )
            .unwrap(),
        None
    );
}

#[test]
fn http_source_probes_bounded_ranges_and_reports_invalid_media_as_failure() {
    let bytes = fs::read(fixture("tone-mp3.mp3")).unwrap();
    let server = RangeServer::start(bytes);
    let options = HttpMediaSourceOptions {
        range: HttpRangeOptions {
            range_window_bytes: 32 * 1024,
            connect_timeout: Duration::from_secs(2),
            request_timeout: Duration::from_secs(2),
            network_access: HttpNetworkAccess::AllowPrivateNetworks,
            ..HttpRangeOptions::default()
        },
        ..HttpMediaSourceOptions::default()
    };
    let mut registry = SourceRegistry::<MediaSourceTrack>::new(SourceRegistryLimits::default());
    registry
        .register(Box::new(HttpMediaSourceManager::new(options)))
        .unwrap();
    let identifier = server.url("track.mp3");
    let loaded = registry
        .load(&SourceReference::new(Some(identifier.clone()), false))
        .unwrap()
        .unwrap();
    assert_eq!(loaded.item.probe, MediaProbe::Mp3);
    assert_eq!(loaded.item.info.identifier, identifier);
    assert!(!server.requests().is_empty());
    assert_eq!(
        registry
            .load(&SourceReference::new(
                Some("ftp://example.test/track.mp3".to_owned()),
                false,
            ))
            .unwrap(),
        None
    );

    let bad_server = RangeServer::start(vec![0_u8; 128]);
    assert_eq!(
        registry.load(&SourceReference::new(
            Some(bad_server.url("bad.mp3")),
            false,
        )),
        Err(SourceRegistryError::SourceFailure)
    );
}

#[test]
fn http_source_turns_ordinary_playlists_into_referrals_and_hls_into_tracks() {
    let manager = HttpMediaSourceManager::new(HttpMediaSourceOptions {
        playlist: HttpPlaylistOptions {
            http: HttpStreamOptions {
                max_response_bytes: 1024,
                connect_timeout: Duration::from_secs(2),
                request_timeout: Duration::from_secs(2),
                network_access: HttpNetworkAccess::AllowPrivateNetworks,
                ..HttpStreamOptions::default()
            },
            playlist: PlaylistLimits {
                max_playlist_bytes: 1024,
                ..PlaylistLimits::default()
            },
            include_plain: false,
        },
        ..HttpMediaSourceOptions::default()
    });
    let ordinary = RangeServer::start(b"#EXTM3U\nhttps://media.example.test/track.mp3\n".to_vec());
    assert_eq!(
        manager
            .load(&SourceReference::new(Some(ordinary.url("list.m3u")), false,))
            .unwrap(),
        Some(SourceLoad::Referral(SourceReference::new(
            Some("https://media.example.test/track.mp3".to_owned()),
            false,
        )))
    );

    let hls = RangeServer::start(b"#EXTM3U\n#EXTINF:2,Live\nsegment.ts\n".to_vec());
    let Some(SourceLoad::Item(track)) = manager
        .load(&SourceReference::new(Some(hls.url("live.m3u8")), false))
        .unwrap()
    else {
        panic!("HLS playlist should create a track");
    };
    assert_eq!(track.probe, MediaProbe::HlsOuter);
    assert!(track.info.is_stream);
}

struct RangeServer {
    address: std::net::SocketAddr,
    requests: Arc<std::sync::Mutex<Vec<(u64, u64)>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RangeServer {
    fn start(bytes: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let bytes: Arc<[u8]> = bytes.into();
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => serve_request(stream, &bytes, &thread_requests),
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
    requests: &std::sync::Mutex<Vec<(u64, u64)>>,
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
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        let _ = stream.write_all(bytes);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Both);
        return;
    };
    requests.lock().unwrap().push((start, requested_end));
    if start >= bytes.len() as u64 {
        return;
    }
    let end = requested_end.min(bytes.len() as u64 - 1);
    let body = &bytes[usize::try_from(start).unwrap()..=usize::try_from(end).unwrap()];
    let content_range = format!("bytes {start}-{end}/{}", bytes.len());
    let _ = write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: {content_range}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

fn parse_range(request: &str) -> Option<(u64, u64)> {
    let value = request
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| name.eq_ignore_ascii_case("range").then_some(value.trim()))?
        .strip_prefix("bytes=")?;
    let (start, end) = value.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}
