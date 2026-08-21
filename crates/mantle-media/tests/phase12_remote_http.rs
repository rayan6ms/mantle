use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mantle_media::{
    HttpNetworkAccess, MediaCancellation, RemoteHttpClient, RemoteHttpErrorKind, RemoteHttpOptions,
    RemoteHttpRequest, RemoteRetryMode,
};

#[test]
fn enforces_ssrf_policy_and_redacts_request_credentials() {
    let server = ReplayServer::start(|_, _| ReplayResponse::ok(b"unreachable"));
    let url = server.url("lookup?api_key=query-secret");
    let request = RemoteHttpRequest::get(&url)
        .unwrap()
        .header("Authorization", "Bearer header-secret")
        .unwrap();

    let diagnostic = format!("{request:?}");
    assert!(diagnostic.contains("GET"), "{diagnostic}");
    assert!(diagnostic.contains("header_count: 1"), "{diagnostic}");
    assert!(!diagnostic.contains("authorization"), "{diagnostic}");
    assert!(!diagnostic.contains("query-secret"), "{diagnostic}");
    assert!(!diagnostic.contains("header-secret"), "{diagnostic}");
    assert!(!diagnostic.contains("lookup"), "{diagnostic}");

    let error = RemoteHttpClient::new(RemoteHttpOptions::default())
        .unwrap()
        .execute(&request)
        .unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::DestinationDenied);
    assert_eq!(
        error.to_string(),
        "remote destination rejected by network access policy"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn sends_bounded_get_and_post_requests_without_exposing_response_secrets() {
    let server = ReplayServer::start(|request, _| {
        assert_eq!(
            request.header("authorization"),
            Some("Bearer header-secret")
        );
        assert_eq!(request.header("content-type"), Some("application/json"));
        assert_eq!(request.body, br#"{"videoId":"abc"}"#);
        ReplayResponse::ok(br#"{"stream":"ready"}"#)
            .header("X-RateLimit-Remaining", "17")
            .header("Set-Cookie", "session=response-secret")
    });
    let request = RemoteHttpRequest::post(
        server.url("player?key=query-secret"),
        br#"{"videoId":"abc"}"#,
    )
    .unwrap()
    .header("Authorization", "Bearer header-secret")
    .unwrap()
    .header("Content-Type", "application/json")
    .unwrap();
    let response = private_client(RemoteHttpOptions::default())
        .execute(&request)
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), br#"{"stream":"ready"}"#);
    assert_eq!(response.header("x-ratelimit-remaining"), Some("17"));
    let diagnostic = format!("{response:?}");
    assert!(diagnostic.contains("body_len: 18"), "{diagnostic}");
    assert!(!diagnostic.contains("response-secret"), "{diagnostic}");
    assert!(!diagnostic.contains("ready"), "{diagnostic}");
}

#[test]
fn follows_headerless_redirects_but_never_forwards_caller_headers() {
    let server = ReplayServer::start(|_, count| {
        if count == 0 {
            ReplayResponse::redirect("/final")
        } else {
            ReplayResponse::ok(b"redirected")
        }
    });
    let request = RemoteHttpRequest::get(server.url("start")).unwrap();
    let response = private_client(RemoteHttpOptions::default())
        .execute(&request)
        .unwrap();
    assert_eq!(response.body(), b"redirected");
    assert_eq!(server.requests().len(), 2);

    let target = ReplayServer::start(|request, _| {
        assert_eq!(request.header("x-source-token"), None);
        ReplayResponse::ok(b"must not be reached")
    });
    let target_url = target.url("credential-target");
    let origin = ReplayServer::start(move |_, _| ReplayResponse::redirect(&target_url));
    let request = RemoteHttpRequest::get(origin.url("credential-origin"))
        .unwrap()
        .header("X-Source-Token", "redirect-secret")
        .unwrap();
    let error = private_client(RemoteHttpOptions::default())
        .execute(&request)
        .unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::HttpStatus);
    assert_eq!(error.status_code(), Some(302));
    assert_eq!(origin.requests().len(), 1);
    assert!(target.requests().is_empty());
}

#[test]
fn retries_only_explicitly_idempotent_posts_with_bounded_backoff() {
    let server = ReplayServer::start(|_, count| {
        if count == 0 {
            ReplayResponse::status("503 Service Unavailable")
        } else if count == 1 {
            ReplayResponse::status("429 Too Many Requests").header("Retry-After", "0")
        } else {
            ReplayResponse::ok(b"ok")
        }
    });
    let options = RemoteHttpOptions {
        max_retries: 2,
        retry_base_delay: Duration::from_millis(1),
        retry_max_delay: Duration::from_millis(2),
        ..RemoteHttpOptions::default()
    };
    let request = RemoteHttpRequest::post(server.url("idempotent"), b"body")
        .unwrap()
        .retry_mode(RemoteRetryMode::Idempotent);
    assert_eq!(
        private_client(options).execute(&request).unwrap().body(),
        b"ok"
    );
    assert_eq!(server.requests().len(), 3);

    let server = ReplayServer::start(|_, _| ReplayResponse::status("503 Service Unavailable"));
    let request = RemoteHttpRequest::post(server.url("unsafe"), b"body").unwrap();
    let error = private_client(options).execute(&request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::ServerUnavailable);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn classifies_statuses_and_retry_after_without_returning_error_bodies() {
    for (status, kind) in [
        ("401 Unauthorized", RemoteHttpErrorKind::Unauthorized),
        ("403 Forbidden", RemoteHttpErrorKind::Forbidden),
        ("404 Not Found", RemoteHttpErrorKind::NotFound),
        ("418 I'm a teapot", RemoteHttpErrorKind::HttpStatus),
        (
            "503 Service Unavailable",
            RemoteHttpErrorKind::ServerUnavailable,
        ),
    ] {
        let server = ReplayServer::start(move |_, _| {
            ReplayResponse::status(status).body(b"credential=error-body-secret")
        });
        let options = RemoteHttpOptions {
            max_retries: 0,
            ..RemoteHttpOptions::default()
        };
        let request = RemoteHttpRequest::get(server.url("status?token=url-secret")).unwrap();
        let error = private_client(options).execute(&request).unwrap_err();
        assert_eq!(error.kind(), kind);
        assert!(!error.to_string().contains("secret"));
        assert!(!format!("{error:?}").contains("secret"));
    }

    let server = ReplayServer::start(|_, _| {
        ReplayResponse::status("429 Too Many Requests").header("Retry-After", "7")
    });
    let options = RemoteHttpOptions {
        max_retries: 0,
        retry_after_max_delay: Duration::from_secs(3),
        ..RemoteHttpOptions::default()
    };
    let request = RemoteHttpRequest::get(server.url("limited")).unwrap();
    let error = private_client(options).execute(&request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::RateLimited);
    assert_eq!(error.retry_after(), Some(Duration::from_secs(3)));
}

#[test]
fn bounds_request_and_response_bodies_before_allocation_growth() {
    let server = ReplayServer::start(|_, _| ReplayResponse::ok(b"12345"));
    let options = RemoteHttpOptions {
        max_request_bytes: 4,
        max_response_bytes: 4,
        ..RemoteHttpOptions::default()
    };
    let client = private_client(options);
    let oversized_request = RemoteHttpRequest::post(server.url("post"), b"12345").unwrap();
    let error = client.execute(&oversized_request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::RequestTooLarge);
    assert!(server.requests().is_empty());

    let request = RemoteHttpRequest::get(server.url("get")).unwrap();
    let error = client.execute(&request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::ResponseTooLarge);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn request_specific_response_ceiling_is_validated_and_enforced() {
    let server = ReplayServer::start(|_, _| ReplayResponse::ok(b"12345"));
    let client = private_client(RemoteHttpOptions::default());
    let request = RemoteHttpRequest::get(server.url("small"))
        .unwrap()
        .max_response_bytes(4)
        .unwrap();
    let error = client.execute(&request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::ResponseTooLarge);

    let error = RemoteHttpRequest::get(server.url("invalid"))
        .unwrap()
        .max_response_bytes(0)
        .unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::InvalidRequest);

    let constrained_client = private_client(RemoteHttpOptions {
        max_response_bytes: 4,
        ..RemoteHttpOptions::default()
    });
    let request = RemoteHttpRequest::get(server.url("policy"))
        .unwrap()
        .max_response_bytes(5)
        .unwrap();
    let error = constrained_client.execute(&request).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::InvalidRequest);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn cancellation_interrupts_retry_backoff_promptly() {
    let server = ReplayServer::start(|_, _| ReplayResponse::status("503 Service Unavailable"));
    let options = RemoteHttpOptions {
        max_retries: 3,
        retry_base_delay: Duration::from_secs(5),
        retry_max_delay: Duration::from_secs(5),
        ..RemoteHttpOptions::default()
    };
    let client = private_client(options);
    let request = RemoteHttpRequest::get(server.url("cancel")).unwrap();
    let cancellation = MediaCancellation::new();
    let trigger = cancellation.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        trigger.cancel();
    });

    let started = Instant::now();
    let error = client
        .execute_with_cancellation(&request, &cancellation)
        .unwrap_err();
    canceller.join().unwrap();
    assert_eq!(error.kind(), RemoteHttpErrorKind::Cancelled);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn rejects_unbounded_policy_and_credential_bearing_request_syntax() {
    let options = RemoteHttpOptions {
        max_response_bytes: 64 * 1024 * 1024 + 1,
        ..RemoteHttpOptions::default()
    };
    let error = RemoteHttpClient::new(options).unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::InvalidOptions);

    let error = RemoteHttpRequest::get("https://user:url-secret@example.com/player").unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::InvalidRequest);
    assert!(!format!("{error:?}").contains("url-secret"));

    let error = RemoteHttpRequest::get("https://example.com")
        .unwrap()
        .header("Host", "attacker.example")
        .unwrap_err();
    assert_eq!(error.kind(), RemoteHttpErrorKind::InvalidRequest);
}

fn private_client(mut options: RemoteHttpOptions) -> RemoteHttpClient {
    options.network_access = HttpNetworkAccess::AllowPrivateNetworks;
    options.connect_timeout = Duration::from_secs(2);
    options.request_timeout = Duration::from_secs(2);
    RemoteHttpClient::new(options).unwrap()
}

#[derive(Clone, Debug)]
struct ReplayRequest {
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
    status: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl ReplayResponse {
    fn status(status: &'static str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn ok(body: &[u8]) -> Self {
        Self::status("200 OK").body(body)
    }

    fn redirect(location: &str) -> Self {
        Self::status("302 Found").header("Location", location)
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    fn body(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
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
        headers,
        body: raw[header_end..header_end + content_len].to_vec(),
    };
    let count = requests.lock().unwrap().len();
    requests.lock().unwrap().push(request.clone());
    let response = responder(request, count);
    let _ = write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.body.len()
    );
    for (name, value) in response.headers {
        let _ = write!(stream, "{name}: {value}\r\n");
    }
    let _ = stream.write_all(b"\r\n");
    let _ = stream.write_all(&response.body);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}
