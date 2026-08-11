use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mantle_media::{
    HttpNetworkAccess, HttpRangeInput, HttpRangeOptions, MediaInput, MediaLimits, MediaSession,
    PcmFrame,
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

fn private_test_options() -> HttpRangeOptions {
    HttpRangeOptions {
        range_window_bytes: 32 * 1024,
        connect_timeout: Duration::from_secs(2),
        request_timeout: Duration::from_secs(2),
        network_access: HttpNetworkAccess::AllowPrivateNetworks,
        ..HttpRangeOptions::default()
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
